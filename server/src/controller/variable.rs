use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, NotSet, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

use crate::entity::{
    b_environments::{self as environments},
    b_projects, b_scripts, b_users, b_variables,
};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::admin::{CreatedResponse, IdRequest};
use crate::schema::api_ok;
use crate::schema::variable::{
    CreateVariableRequest, UpdateVariableRequest, VariableListRequest, VariableResponse,
};
use crate::state::AppState;

/// 查询项目环境下的凭据绑定，不读取或返回密钥值。
pub async fn list(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<VariableListRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    require_environment(&state.conn, body.project_id, body.environment_id).await?;
    let rows = b_variables::Entity::find()
        .filter(b_variables::Column::ProjectId.eq(body.project_id))
        .filter(b_variables::Column::EnvironmentId.eq(body.environment_id))
        .order_by_asc(b_variables::Column::Id)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let items: Vec<_> = rows
        .into_iter()
        .map(|row| VariableResponse {
            id: row.id,
            key: row.key,
            source_env: row.source_env,
            description: row.description,
            disabled_at: row.disabled_at.map(|value| value.and_utc()),
        })
        .collect();

    Ok(api_ok(items))
}

/// 新增凭据绑定。
pub async fn add(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<CreateVariableRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    require_environment(&state.conn, body.project_id, body.environment_id).await?;
    validate_key(&body.key)?;
    validate_source_env(&body.source_env)?;
    let exists = b_variables::Entity::find()
        .filter(b_variables::Column::ProjectId.eq(body.project_id))
        .filter(b_variables::Column::EnvironmentId.eq(body.environment_id))
        .filter(b_variables::Column::Key.eq(&body.key))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if exists > 0 {
        return Err(AppError::InvalidArgument(
            "variable key already exists".into(),
        ));
    }

    let active = b_variables::ActiveModel {
        id: NotSet,
        project_id: Set(body.project_id),
        environment_id: Set(body.environment_id),
        key: Set(body.key.clone()),
        source_env: Set(body.source_env.clone()),
        description: Set(body.description.clone()),
        disabled_at: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    };
    let variable = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(CreatedResponse { id: variable.id }))
}

/// 更新绑定；变更后相关脚本重新待审核。
pub async fn update(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<UpdateVariableRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    let body = body.into_inner();
    if let Some(key) = body.key.as_deref() {
        validate_key(key)?;
    }
    if let Some(source_env) = body.source_env.as_deref() {
        validate_source_env(source_env)?;
    }

    // 1. 事务内更新绑定并让依赖环境的脚本进入下一待审核版本。
    let txn = state.conn.begin().await.map_err(AppError::internal)?;
    let variable = b_variables::Entity::find_by_id(body.id)
        .one(&txn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("variable does not exist".into()))?;
    let mut active: b_variables::ActiveModel = variable.clone().into();
    if let Some(key) = body.key {
        active.key = Set(key);
    }
    if let Some(source_env) = body.source_env {
        active.source_env = Set(source_env);
    }
    if let Some(description) = body.description {
        active.description = Set(description);
    }
    if let Some(disabled_at) = body.disabled_at {
        active.disabled_at = Set(disabled_at.map(|value| value.naive_utc()));
    }
    active.update(&txn).await.map_err(AppError::internal)?;
    reset_environment_scripts(&txn, variable.project_id, variable.environment_id).await?;
    txn.commit().await.map_err(AppError::internal)?;
    b_users::insert_log(
        &state.conn,
        actor.id,
        "credential_binding_update",
        "success",
        Some(&body.id.to_string()),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 删除绑定；依赖脚本仍会重新待审核。
pub async fn delete(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    let txn = state.conn.begin().await.map_err(AppError::internal)?;
    let variable = b_variables::Entity::find_by_id(body.id)
        .one(&txn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("variable does not exist".into()))?;
    b_variables::Entity::delete_by_id(body.id)
        .exec(&txn)
        .await
        .map_err(AppError::internal)?;
    reset_environment_scripts(&txn, variable.project_id, variable.environment_id).await?;
    txn.commit().await.map_err(AppError::internal)?;
    b_users::insert_log(
        &state.conn,
        actor.id,
        "credential_binding_update",
        "success",
        Some(&body.id.to_string()),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 环境凭据变化后，所有绑定该环境的脚本递增版本并清空批准。
async fn reset_environment_scripts(
    txn: &DatabaseTransaction,
    project_id: u64,
    environment_id: u64,
) -> Result<(), AppError> {
    let scripts = b_scripts::Entity::find()
        .filter(b_scripts::Column::ProjectId.eq(project_id))
        .filter(b_scripts::Column::EnvironmentId.eq(environment_id))
        .lock_exclusive()
        .all(txn)
        .await
        .map_err(AppError::internal)?;
    for script in scripts {
        let mut active: b_scripts::ActiveModel = script.into();
        active.revision = Set(active.revision.unwrap() + 1);
        active.review_status = Set("pending".into());
        active.reviewed_by = Set(None);
        active.reviewed_at = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(txn).await.map_err(AppError::internal)?;
    }

    Ok(())
}

async fn require_environment(
    conn: &sea_orm::DatabaseConnection,
    project_id: u64,
    environment_id: u64,
) -> Result<(), AppError> {
    let project = b_projects::Entity::find_by_id(project_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("project does not exist".into()))?;
    if project.disabled_at.is_some() {
        return Err(AppError::NotFound("project does not exist".into()));
    }
    let environment = environments::Entity::find_by_id(environment_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("environment does not exist".into()))?;
    if environment.project_id != project_id {
        return Err(AppError::InvalidArgument(
            "environment does not belong to project".into(),
        ));
    }

    Ok(())
}

fn require_admin(actor: &AuthUser) -> Result<(), AppError> {
    if actor.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "administrator permission is required".into(),
        ))
    }
}

fn validate_key(key: &str) -> Result<(), AppError> {
    let valid = !key.is_empty() && key.len() <= 128;
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "key must contain 1 to 128 characters".into(),
        ))
    }
}

/// 只校验部署侧环境变量是否存在，不读取值。
fn validate_source_env(source_env: &str) -> Result<(), AppError> {
    let identifier_valid = !source_env.is_empty()
        && source_env.len() <= 128
        && source_env
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && source_env
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    let is_service_config = source_env.starts_with("AGENTSECRET_")
        || matches!(source_env, "DATABASE_URL" | "MYSQL_ROOT_PASSWORD");
    if identifier_valid && !is_service_config && std::env::var_os(source_env).is_some() {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "source_env is not a registered execution credential".into(),
        ))
    }
}
