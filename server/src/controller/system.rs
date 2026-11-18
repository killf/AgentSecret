use actix_web::{HttpResponse, web};
use sea_orm::{EntityTrait, QueryOrder};

use crate::entity::b_system_settings::{self as settings};
use crate::error::AppError;
use crate::schema::api_ok;
use crate::state::AppState;

/// 系统设置列表。
pub async fn list(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let rows = settings::Entity::find()
        .order_by_asc(settings::Column::Key)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(rows))
}

/// 当前版本预留写入口，固定返回无权限。
pub async fn update() -> Result<HttpResponse, AppError> {
    Err(AppError::Forbidden(
        "system settings are read-only in the current version".into(),
    ))
}
