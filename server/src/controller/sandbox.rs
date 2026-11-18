use actix_web::{HttpResponse, web};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::entity::{b_sandbox_instances, b_sandboxes, r_project_permissions};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::api_ok;
use crate::schema::sandbox::{SandboxIdRequest, SandboxInstanceResponse, SandboxResponse};
use crate::state::AppState;

/// 沙盒配置列表；按项目权限裁剪。
pub async fn list(actor: AuthUser, state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let rows = b_sandboxes::Entity::find()
        .order_by_asc(b_sandboxes::Column::Id)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let allowed = allowed_project_ids(&state.conn, &actor).await?;
    let items = rows
        .iter()
        .filter(|sandbox| {
            actor.is_admin
                || allowed
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&sandbox.project_id))
        })
        .map(|sandbox| SandboxResponse {
            id: sandbox.id,
            project_id: sandbox.project_id,
            name: sandbox.name.clone(),
            image: sandbox.image.clone(),
            max_instances: sandbox.max_instances,
        })
        .collect::<Vec<_>>();

    Ok(api_ok(items))
}

/// 沙盒实例历史列表。
pub async fn instances(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<SandboxIdRequest>,
) -> Result<HttpResponse, AppError> {
    let sandbox = b_sandboxes::Entity::find_by_id(body.sandbox_id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("sandbox does not exist".into()))?;
    if !actor.is_admin {
        let allowed = allowed_project_ids(&state.conn, &actor).await?;
        if !allowed
            .as_ref()
            .is_none_or(|ids| ids.contains(&sandbox.project_id))
        {
            return Err(AppError::Forbidden("project permission is required".into()));
        }
    }

    let rows = b_sandbox_instances::Entity::find()
        .filter(b_sandbox_instances::Column::SandboxId.eq(sandbox.id))
        .order_by_desc(b_sandbox_instances::Column::Id)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let items = rows
        .into_iter()
        .map(|row| SandboxInstanceResponse {
            id: row.id,
            name: row.name,
            status: row.status,
            created_at: row.created_at.and_utc(),
            destroyed_at: row.destroyed_at.map(|value| value.and_utc()),
        })
        .collect::<Vec<_>>();

    Ok(api_ok(items))
}

async fn allowed_project_ids(
    conn: &sea_orm::DatabaseConnection,
    actor: &AuthUser,
) -> Result<Option<Vec<u64>>, AppError> {
    let permissions = r_project_permissions::Entity::find()
        .filter(r_project_permissions::Column::UserId.eq(actor.id))
        .all(conn)
        .await
        .map_err(AppError::internal)?;
    if permissions
        .iter()
        .any(|permission| permission.project_id.is_none())
    {
        return Ok(None);
    }

    Ok(Some(
        permissions
            .into_iter()
            .filter_map(|permission| permission.project_id)
            .collect(),
    ))
}
