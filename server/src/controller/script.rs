use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::entity::{
    b_environments, b_executions, b_projects, b_sandboxes, b_scripts, b_users,
    r_environment_permissions, r_project_permissions,
};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::admin::IdRequest;
use crate::schema::api_ok;
use crate::schema::script::{
    CreateScriptRequest, ReviewScriptRequest, ScriptDetail, ScriptItem, ScriptListRequest,
    ScriptVersionResponse, UpdateScriptRequest,
};
use crate::service::credential::{normalize_credential_actions, refresh_credential_actions};
use crate::service::script_params::{validate_definition, validate_runtime};
use crate::state::AppState;

/// 脚本列表；管理员看全部，普通用户仅看已审核且授权范围内的脚本。
pub async fn list(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<ScriptListRequest>,
) -> Result<HttpResponse, AppError> {
    require_project(&state.conn, &actor, body.project_id).await?;
    let mut query = b_scripts::Entity::find()
        .filter(b_scripts::Column::ProjectId.eq(body.project_id))
        .order_by_asc(b_scripts::Column::Id);
    if !actor.is_admin {
        query = query.filter(b_scripts::Column::ReviewStatus.eq("approved"));
    }
    let rows = query.all(&state.conn).await.map_err(AppError::internal)?;
    let mut items = Vec::new();
    for script in rows {
        if !actor.is_admin
            && !environment_allowed(&state.conn, &actor, script.environment_id).await?
        {
            continue;
        }
        items.push(script_item(script));
    }

    Ok(api_ok(items))
}

/// 脚本详情；普通用户不返回源码与凭据配置。
pub async fn detail(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let script = require_script(&state.conn, &actor, body.id).await?;
    let include_secret_fields = actor.is_admin;
    let detail = ScriptDetail {
        item: script_item(script.clone()),
        params: script.params.clone(),
        content: include_secret_fields.then_some(script.content.clone()),
        credential_actions: if include_secret_fields {
            script.credential_actions.clone()
        } else {
            None
        },
    };

    Ok(api_ok(detail))
}

/// 新增脚本，初始版本为待审核。
pub async fn add(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<CreateScriptRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    require_project(&state.conn, &actor, body.project_id).await?;
    validate_script(&body.name, &body.kind, &body.content)?;
    validate_definition(body.params.as_ref())?;
    validate_json_object("credential_actions", body.credential_actions.as_ref())?;
    if let Some(environment_id) = body.environment_id {
        require_environment_project(&state.conn, body.project_id, environment_id).await?;
    }
    if let Some(sandbox_id) = body.sandbox_id {
        require_sandbox_project(&state.conn, body.project_id, sandbox_id).await?;
    }
    let credential_actions = normalize_credential_actions(
        &state.conn,
        &state.config,
        body.project_id,
        body.environment_id,
        body.credential_actions.as_ref(),
    )
    .await?;

    let now = Utc::now().naive_utc();
    let active = b_scripts::ActiveModel {
        id: NotSet,
        project_id: Set(body.project_id),
        environment_id: Set(body.environment_id),
        sandbox_id: Set(body.sandbox_id),
        name: Set(body.name.clone()),
        description: Set(body.description.clone()),
        r#type: Set(body.kind.clone()),
        content: Set(body.content.clone()),
        params: Set(body.params.clone()),
        credential_actions: Set(credential_actions),
        revision: Set(1),
        review_status: Set("pending".into()),
        reviewed_by: Set(None),
        reviewed_at: Set(None),
        created_by: Set(actor.id),
        created_at: Set(now),
        updated_by: Set(actor.id),
        updated_at: Set(now),
    };
    let script = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(ScriptVersionResponse {
        id: script.id,
        revision: script.revision,
    }))
}

/// 按版本条件更新脚本；成功后进入待审核。
pub async fn update(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<UpdateScriptRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    let body = body.into_inner();
    if let Some(name) = body.name.as_deref() {
        validate_script_name(name)?;
    }
    if let Some(kind) = body.kind.as_deref() {
        validate_script_kind(kind)?;
    }
    if let Some(content) = body.content.as_deref() {
        validate_content(content)?;
    }
    validate_definition(body.params.as_ref())?;
    validate_json_object("credential_actions", body.credential_actions.as_ref())?;

    // 1. 读取当前版本并校验乐观版本号。
    let script = b_scripts::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("script does not exist".into()))?;
    if script.revision != body.revision {
        return Err(AppError::InvalidArgument("script revision mismatch".into()));
    }
    let next_revision = script.revision + 1;
    let project_id = script.project_id;
    let environment_id = body.environment_id.unwrap_or(script.environment_id);
    let sandbox_id = body.sandbox_id.unwrap_or(script.sandbox_id);
    if let Some(environment_id) = environment_id {
        require_environment_project(&state.conn, project_id, environment_id).await?;
    }
    if let Some(sandbox_id) = sandbox_id {
        require_sandbox_project(&state.conn, project_id, sandbox_id).await?;
    }
    let credential_actions = normalize_credential_actions(
        &state.conn,
        &state.config,
        project_id,
        environment_id,
        body.credential_actions.as_ref(),
    )
    .await?;

    // 2. 更新字段并清空当前版本批准。
    let mut active: b_scripts::ActiveModel = script.into();
    if let Some(name) = body.name {
        active.name = Set(name);
    }
    if let Some(kind) = body.kind {
        active.r#type = Set(kind);
    }
    if let Some(content) = body.content {
        active.content = Set(content);
    }
    if let Some(description) = body.description {
        active.description = Set(description);
    }
    active.environment_id = Set(environment_id);
    active.sandbox_id = Set(sandbox_id);
    if let Some(params) = body.params {
        active.params = Set(Some(params));
    }
    if body.credential_actions.is_some() {
        active.credential_actions = Set(credential_actions);
    }
    active.revision = Set(next_revision);
    active.review_status = Set("pending".into());
    active.reviewed_by = Set(None);
    active.reviewed_at = Set(None);
    active.updated_by = Set(actor.id);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(ScriptVersionResponse {
        id: updated.id,
        revision: updated.revision,
    }))
}

/// 删除没有执行记录的脚本。
pub async fn delete(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    let script = b_scripts::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("script does not exist".into()))?;
    let executions = b_executions::Entity::find()
        .filter(b_executions::Column::ScriptId.eq(script.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if executions > 0 {
        return Err(AppError::NotFound("script is still referenced".into()));
    }

    b_scripts::Entity::delete_by_id(script.id)
        .exec(&state.conn)
        .await
        .map_err(AppError::internal)?;
    b_users::insert_log(
        &state.conn,
        actor.id,
        "script_edit",
        "success",
        Some(&format!("{}:deleted", script.id)),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 审核脚本；最后编辑者不能审核当前版本。
pub async fn review(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<ReviewScriptRequest>,
) -> Result<HttpResponse, AppError> {
    require_admin(&actor)?;
    let decision = match body.decision.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        _ => {
            return Err(AppError::InvalidArgument(
                "decision must be approve or reject".into(),
            ));
        }
    };
    let script = b_scripts::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("script does not exist".into()))?;
    if script.revision != body.revision || script.review_status != "pending" {
        return Err(AppError::InvalidArgument(
            "only the pending matching revision can be reviewed".into(),
        ));
    }
    if script.updated_by == actor.id {
        return Err(AppError::Forbidden(
            "the last editor cannot review this revision".into(),
        ));
    }

    // 1. 审核通过时重新读取部署侧动作定义，并固定配置指纹。
    let canonical_actions = if decision == "approved" {
        refresh_credential_actions(&state.conn, &state.config, &script).await?
    } else {
        script.credential_actions.clone()
    };

    // 2. 按版本条件写入审核结果。
    let mut active: b_scripts::ActiveModel = script.into();
    if decision == "approved" {
        active.credential_actions = Set(canonical_actions);
    }
    active.review_status = Set(decision.to_string());
    active.reviewed_by = Set(Some(actor.id));
    active.reviewed_at = Set(Some(Utc::now().naive_utc()));
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;
    b_users::insert_log(
        &state.conn,
        actor.id,
        "script_review",
        "success",
        Some(&format!("{}:{}:{}", body.id, body.revision, decision)),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 执行当前已审核脚本，并返回执行记录 ID。
pub async fn run(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    let script_id = body
        .get("id")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| AppError::InvalidArgument("id is required".into()))?;
    let params = body
        .get("params")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if !params.is_object() {
        return Err(AppError::InvalidArgument("params must be an object".into()));
    }
    let script = require_script(&state.conn, &actor, script_id).await?;
    validate_runtime(script.params.as_ref(), &params)?;
    let execution_id =
        crate::service::execution::run_script(&state, script, actor.id, params).await?;

    Ok(api_ok(serde_json::json!({ "execution_id": execution_id })))
}
async fn require_script(
    conn: &sea_orm::DatabaseConnection,
    actor: &AuthUser,
    script_id: u64,
) -> Result<b_scripts::Model, AppError> {
    let script = b_scripts::Entity::find_by_id(script_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("script does not exist".into()))?;
    require_project(conn, actor, script.project_id).await?;
    if !actor.is_admin && script.review_status != "approved" {
        return Err(AppError::NotFound("script does not exist".into()));
    }
    if !actor.is_admin && !environment_allowed(conn, actor, script.environment_id).await? {
        return Err(AppError::Forbidden(
            "environment permission is required".into(),
        ));
    }

    Ok(script)
}

async fn require_project(
    conn: &sea_orm::DatabaseConnection,
    actor: &AuthUser,
    project_id: u64,
) -> Result<(), AppError> {
    let project = b_projects::Entity::find_by_id(project_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("project does not exist".into()))?;
    if project.disabled_at.is_some() {
        return Err(AppError::NotFound("project does not exist".into()));
    }
    if actor.is_admin {
        return Ok(());
    }
    let permissions = r_project_permissions::Entity::find()
        .filter(r_project_permissions::Column::UserId.eq(actor.id))
        .all(conn)
        .await
        .map_err(AppError::internal)?;
    let allowed = permissions
        .iter()
        .any(|permission| permission.project_id.is_none_or(|id| id == project_id));
    if allowed {
        Ok(())
    } else {
        Err(AppError::Forbidden("project permission is required".into()))
    }
}

async fn environment_allowed(
    conn: &sea_orm::DatabaseConnection,
    actor: &AuthUser,
    environment_id: Option<u64>,
) -> Result<bool, AppError> {
    let permissions = r_environment_permissions::Entity::find()
        .filter(r_environment_permissions::Column::UserId.eq(actor.id))
        .all(conn)
        .await
        .map_err(AppError::internal)?;

    Ok(permissions.iter().any(|permission| {
        permission
            .environment_id
            .is_none_or(|allowed| Some(allowed) == environment_id)
    }))
}

async fn require_environment_project(
    conn: &sea_orm::DatabaseConnection,
    project_id: u64,
    environment_id: u64,
) -> Result<(), AppError> {
    let environment = b_environments::Entity::find_by_id(environment_id)
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

async fn require_sandbox_project(
    conn: &sea_orm::DatabaseConnection,
    project_id: u64,
    sandbox_id: u64,
) -> Result<(), AppError> {
    let sandbox = b_sandboxes::Entity::find_by_id(sandbox_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("sandbox does not exist".into()))?;
    if sandbox.project_id != project_id {
        return Err(AppError::InvalidArgument(
            "sandbox does not belong to project".into(),
        ));
    }

    Ok(())
}

fn script_item(script: b_scripts::Model) -> ScriptItem {
    ScriptItem {
        id: script.id,
        project_id: script.project_id,
        environment_id: script.environment_id,
        sandbox_id: script.sandbox_id,
        name: script.name,
        description: script.description,
        kind: script.r#type,
        review_status: script.review_status,
        revision: script.revision,
        updated_at: script.updated_at.and_utc(),
    }
}

fn validate_script(name: &str, kind: &str, content: &str) -> Result<(), AppError> {
    validate_script_name(name)?;
    validate_script_kind(kind)?;
    validate_content(content)
}

fn validate_script_name(name: &str) -> Result<(), AppError> {
    if !name.trim().is_empty() && name == name.trim() && name.len() <= 128 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "script name must contain 1 to 128 characters without surrounding spaces".into(),
        ))
    }
}

fn validate_script_kind(kind: &str) -> Result<(), AppError> {
    if matches!(kind, "bash" | "python") {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "script type must be bash or python".into(),
        ))
    }
}

fn validate_content(content: &str) -> Result<(), AppError> {
    if content.is_empty() {
        Err(AppError::InvalidArgument(
            "script content is required".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_json_object(field: &str, value: Option<&serde_json::Value>) -> Result<(), AppError> {
    if value.is_none_or(|value| value.is_object()) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{field} must be an object"
        )))
    }
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
