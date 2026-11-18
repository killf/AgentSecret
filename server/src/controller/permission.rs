use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, QueryOrder, Set};

use crate::entity::{
    b_environments, b_models, b_projects, b_providers, b_users, r_environment_permissions,
    r_model_permissions, r_project_permissions, r_provider_permissions,
};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::admin::CreatedResponse;
use crate::schema::api_ok;
use crate::schema::permission::{
    PermissionDeleteRequest, PermissionListRequest, PermissionResponse, PermissionType,
    PermissionUpsertRequest, TokenLimits,
};
use crate::state::AppState;

/// 按类型查询权限。
pub async fn list(
    state: web::Data<AppState>,
    body: web::Json<PermissionListRequest>,
) -> Result<HttpResponse, AppError> {
    let items = match body.kind {
        PermissionType::Projects => {
            let mut query = r_project_permissions::Entity::find()
                .order_by_asc(r_project_permissions::Column::Id);
            if let Some(user_id) = body.user_id {
                query = query.filter(r_project_permissions::Column::UserId.eq(user_id));
            }
            query
                .all(&state.conn)
                .await
                .map_err(AppError::internal)?
                .into_iter()
                .map(|row| PermissionResponse {
                    id: row.id,
                    user_id: row.user_id,
                    target_id: row.project_id,
                    limits: None,
                })
                .collect::<Vec<_>>()
        }
        PermissionType::Environments => {
            let mut query = r_environment_permissions::Entity::find()
                .order_by_asc(r_environment_permissions::Column::Id);
            if let Some(user_id) = body.user_id {
                query = query.filter(r_environment_permissions::Column::UserId.eq(user_id));
            }
            query
                .all(&state.conn)
                .await
                .map_err(AppError::internal)?
                .into_iter()
                .map(|row| PermissionResponse {
                    id: row.id,
                    user_id: row.user_id,
                    target_id: row.environment_id,
                    limits: None,
                })
                .collect::<Vec<_>>()
        }
        PermissionType::Models => list_model_permissions(&state.conn, body.user_id).await?,
        PermissionType::Providers => list_provider_permissions(&state.conn, body.user_id).await?,
    };

    Ok(api_ok(items))
}

/// 创建或更新权限。
pub async fn upsert(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<PermissionUpsertRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.user_id)?;
    if let Some(target_id) = body.target_id {
        validate_id(target_id)?;
        if !target_exists(&state.conn, body.kind, target_id).await? {
            return Err(AppError::NotFound(
                "permission target does not exist".into(),
            ));
        }
    }
    if let Some(limits) = body.limits {
        validate_limits(limits)?;
    }
    let user_exists = b_users::Entity::find_by_id(body.user_id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .is_some();
    if !user_exists {
        return Err(AppError::NotFound("user does not exist".into()));
    }

    // 1. 按用户与目标查找已有记录，命中则更新限额。
    let existing_id = match body.kind {
        PermissionType::Projects => {
            let mut query = r_project_permissions::Entity::find()
                .filter(r_project_permissions::Column::UserId.eq(body.user_id));
            query = match body.target_id {
                Some(id) => query.filter(r_project_permissions::Column::ProjectId.eq(id)),
                None => query.filter(r_project_permissions::Column::ProjectId.is_null()),
            };
            query
                .one(&state.conn)
                .await
                .map_err(AppError::internal)?
                .map(|row| row.id)
        }
        PermissionType::Environments => {
            let mut query = r_environment_permissions::Entity::find()
                .filter(r_environment_permissions::Column::UserId.eq(body.user_id));
            query = match body.target_id {
                Some(id) => query.filter(r_environment_permissions::Column::EnvironmentId.eq(id)),
                None => query.filter(r_environment_permissions::Column::EnvironmentId.is_null()),
            };
            query
                .one(&state.conn)
                .await
                .map_err(AppError::internal)?
                .map(|row| row.id)
        }
        PermissionType::Models => {
            let mut query = r_model_permissions::Entity::find()
                .filter(r_model_permissions::Column::UserId.eq(body.user_id));
            query = match body.target_id {
                Some(id) => query.filter(r_model_permissions::Column::ModelId.eq(id)),
                None => query.filter(r_model_permissions::Column::ModelId.is_null()),
            };
            query
                .one(&state.conn)
                .await
                .map_err(AppError::internal)?
                .map(|row| row.id)
        }
        PermissionType::Providers => {
            let mut query = r_provider_permissions::Entity::find()
                .filter(r_provider_permissions::Column::UserId.eq(body.user_id));
            query = match body.target_id {
                Some(id) => query.filter(r_provider_permissions::Column::ProviderId.eq(id)),
                None => query.filter(r_provider_permissions::Column::ProviderId.is_null()),
            };
            query
                .one(&state.conn)
                .await
                .map_err(AppError::internal)?
                .map(|row| row.id)
        }
    };

    if let Some(id) = existing_id {
        update_limits(&state.conn, body.kind, id, body.limits).await?;
        return Ok(api_ok(CreatedResponse { id }));
    }

    // 2. 未命中则插入新权限。
    let now = Utc::now().naive_utc();
    let id = match body.kind {
        PermissionType::Projects => {
            r_project_permissions::ActiveModel {
                id: NotSet,
                user_id: Set(body.user_id),
                project_id: Set(body.target_id),
                created_by: Set(actor.id),
                created_at: Set(now),
            }
            .insert(&state.conn)
            .await
            .map_err(AppError::internal)?
            .id
        }
        PermissionType::Environments => {
            r_environment_permissions::ActiveModel {
                id: NotSet,
                user_id: Set(body.user_id),
                environment_id: Set(body.target_id),
                created_by: Set(actor.id),
                created_at: Set(now),
            }
            .insert(&state.conn)
            .await
            .map_err(AppError::internal)?
            .id
        }
        PermissionType::Models => {
            let limits = body.limits.unwrap_or_default();
            r_model_permissions::ActiveModel {
                id: NotSet,
                user_id: Set(body.user_id),
                model_id: Set(body.target_id),
                daily_token_limit: Set(limits.daily_token_limit),
                daily_token_used: Set(0),
                daily_token_refreshed_at: Set(None),
                weekly_token_limit: Set(limits.weekly_token_limit),
                weekly_token_used: Set(0),
                weekly_token_refreshed_at: Set(None),
                monthly_token_limit: Set(limits.monthly_token_limit),
                monthly_token_used: Set(0),
                monthly_token_refreshed_at: Set(None),
                created_by: Set(Some(actor.id)),
                created_at: Set(now),
                updated_by: Set(Some(actor.id)),
                updated_at: Set(now),
            }
            .insert(&state.conn)
            .await
            .map_err(AppError::internal)?
            .id
        }
        PermissionType::Providers => {
            let limits = body.limits.unwrap_or_default();
            r_provider_permissions::ActiveModel {
                id: NotSet,
                user_id: Set(body.user_id),
                provider_id: Set(body.target_id),
                daily_token_limit: Set(limits.daily_token_limit),
                daily_token_used: Set(0),
                daily_token_refreshed_at: Set(None),
                weekly_token_limit: Set(limits.weekly_token_limit),
                weekly_token_used: Set(0),
                weekly_token_refreshed_at: Set(None),
                monthly_token_limit: Set(limits.monthly_token_limit),
                monthly_token_used: Set(0),
                monthly_token_refreshed_at: Set(None),
                created_by: Set(Some(actor.id)),
                created_at: Set(now),
                updated_by: Set(Some(actor.id)),
                updated_at: Set(now),
            }
            .insert(&state.conn)
            .await
            .map_err(AppError::internal)?
            .id
        }
    };
    Ok(api_ok(CreatedResponse { id }))
}

/// 删除权限记录。
pub async fn delete(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<PermissionDeleteRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    let result = match body.kind {
        PermissionType::Projects => r_project_permissions::Entity::delete_by_id(body.id)
            .exec(&state.conn)
            .await
            .map_err(AppError::internal)?,
        PermissionType::Environments => r_environment_permissions::Entity::delete_by_id(body.id)
            .exec(&state.conn)
            .await
            .map_err(AppError::internal)?,
        PermissionType::Models => r_model_permissions::Entity::delete_by_id(body.id)
            .exec(&state.conn)
            .await
            .map_err(AppError::internal)?,
        PermissionType::Providers => r_provider_permissions::Entity::delete_by_id(body.id)
            .exec(&state.conn)
            .await
            .map_err(AppError::internal)?,
    };
    if result.rows_affected == 0 {
        return Err(AppError::NotFound("permission does not exist".into()));
    }
    b_users::insert_log(
        &state.conn,
        actor.id,
        "permission_delete",
        "success",
        Some(&body.id.to_string()),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 查询模型权限。
async fn list_model_permissions(
    conn: &sea_orm::DatabaseConnection,
    user_id: Option<u64>,
) -> Result<Vec<PermissionResponse>, AppError> {
    let mut query =
        r_model_permissions::Entity::find().order_by_asc(r_model_permissions::Column::Id);
    if let Some(user_id) = user_id {
        query = query.filter(r_model_permissions::Column::UserId.eq(user_id));
    }
    let rows = query.all(conn).await.map_err(AppError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| PermissionResponse {
            id: row.id,
            user_id: row.user_id,
            target_id: row.model_id,
            limits: Some(TokenLimits {
                daily_token_limit: row.daily_token_limit,
                weekly_token_limit: row.weekly_token_limit,
                monthly_token_limit: row.monthly_token_limit,
            }),
        })
        .collect())
}

/// 查询供应商权限。
async fn list_provider_permissions(
    conn: &sea_orm::DatabaseConnection,
    user_id: Option<u64>,
) -> Result<Vec<PermissionResponse>, AppError> {
    let mut query =
        r_provider_permissions::Entity::find().order_by_asc(r_provider_permissions::Column::Id);
    if let Some(user_id) = user_id {
        query = query.filter(r_provider_permissions::Column::UserId.eq(user_id));
    }
    let rows = query.all(conn).await.map_err(AppError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| PermissionResponse {
            id: row.id,
            user_id: row.user_id,
            target_id: row.provider_id,
            limits: Some(TokenLimits {
                daily_token_limit: row.daily_token_limit,
                weekly_token_limit: row.weekly_token_limit,
                monthly_token_limit: row.monthly_token_limit,
            }),
        })
        .collect())
}

/// 更新已有权限的限额。
async fn update_limits(
    conn: &sea_orm::DatabaseConnection,
    kind: PermissionType,
    id: u64,
    limits: Option<TokenLimits>,
) -> Result<(), AppError> {
    let limits = limits.unwrap_or_default();
    match kind {
        PermissionType::Models => {
            let model = r_model_permissions::Entity::find_by_id(id)
                .one(conn)
                .await
                .map_err(AppError::internal)?
                .ok_or_else(|| AppError::NotFound("permission does not exist".into()))?;
            let mut active: r_model_permissions::ActiveModel = model.into();
            active.daily_token_limit = Set(limits.daily_token_limit);
            active.weekly_token_limit = Set(limits.weekly_token_limit);
            active.monthly_token_limit = Set(limits.monthly_token_limit);
            active.update(conn).await.map_err(AppError::internal)?;
        }
        PermissionType::Providers => {
            let model = r_provider_permissions::Entity::find_by_id(id)
                .one(conn)
                .await
                .map_err(AppError::internal)?
                .ok_or_else(|| AppError::NotFound("permission does not exist".into()))?;
            let mut active: r_provider_permissions::ActiveModel = model.into();
            active.daily_token_limit = Set(limits.daily_token_limit);
            active.weekly_token_limit = Set(limits.weekly_token_limit);
            active.monthly_token_limit = Set(limits.monthly_token_limit);
            active.update(conn).await.map_err(AppError::internal)?;
        }
        PermissionType::Projects | PermissionType::Environments => {}
    }

    Ok(())
}
async fn target_exists(
    conn: &sea_orm::DatabaseConnection,
    kind: PermissionType,
    target_id: u64,
) -> Result<bool, AppError> {
    let exists = match kind {
        PermissionType::Projects => b_projects::Entity::find_by_id(target_id)
            .one(conn)
            .await
            .map_err(AppError::internal)?
            .is_some(),
        PermissionType::Environments => b_environments::Entity::find_by_id(target_id)
            .one(conn)
            .await
            .map_err(AppError::internal)?
            .is_some(),
        PermissionType::Models => b_models::Entity::find_by_id(target_id)
            .one(conn)
            .await
            .map_err(AppError::internal)?
            .is_some(),
        PermissionType::Providers => b_providers::Entity::find_by_id(target_id)
            .one(conn)
            .await
            .map_err(AppError::internal)?
            .is_some(),
    };

    Ok(exists)
}

fn validate_id(id: u64) -> Result<(), AppError> {
    if id == 0 {
        Err(AppError::InvalidArgument(
            "id must be greater than zero".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_limits(limits: TokenLimits) -> Result<(), AppError> {
    let valid = [
        limits.daily_token_limit,
        limits.weekly_token_limit,
        limits.monthly_token_limit,
    ]
    .iter()
    .all(|value| value.is_none_or(|value| value >= 0));
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "token limits cannot be negative".into(),
        ))
    }
}
