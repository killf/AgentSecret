use actix_web::{HttpResponse, web};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::entity::{b_executions, b_models, b_providers, b_request_logs, b_users};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::admin::IdRequest;
use crate::schema::api_ok;
use crate::schema::log::{
    ExecutionItem, ExecutionLogListRequest, RequestLogDetail, RequestLogItem, RequestLogListRequest,
};
use crate::state::AppState;

/// 分页查询模型网关请求日志。
pub async fn request_list(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<RequestLogListRequest>,
) -> Result<HttpResponse, AppError> {
    let page = body.page.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    validate_pagination(page, page_size)?;
    let user_id = if actor.is_admin {
        body.user_id
    } else {
        Some(actor.id)
    };

    // 1. 组装过滤条件；非管理员固定只能查看自己。
    let mut query = b_request_logs::Entity::find().order_by_desc(b_request_logs::Column::Id);
    if let Some(user_id) = user_id {
        query = query.filter(b_request_logs::Column::UserId.eq(user_id));
    }
    if let Some(model_id) = body.model_id {
        query = query.filter(b_request_logs::Column::ModelId.eq(model_id));
    }
    if let Some(provider_id) = body.provider_id {
        query = query.filter(b_request_logs::Column::ProviderId.eq(provider_id));
    }
    if let Some(status) = &body.status {
        query = query.filter(b_request_logs::Column::RequestStatus.eq(status));
    }
    if let Some(start) = body.start {
        query = query.filter(b_request_logs::Column::CreatedAt.gte(start.naive_utc()));
    }
    if let Some(end) = body.end {
        query = query.filter(b_request_logs::Column::CreatedAt.lt(end.naive_utc()));
    }
    let total = query
        .clone()
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let rows = query
        .limit(page_size)
        .offset((page - 1).saturating_mul(page_size))
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;

    // 2. 补齐用户、模型与供应商展示名。
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(request_item(&state.conn, row).await?);
    }

    Ok(api_ok(
        serde_json::json!({ "total": total, "items": items }),
    ))
}

/// 查询请求日志详情，敏感内容已在上游入口遮罩。
pub async fn request_detail(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let row = b_request_logs::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("request log does not exist".into()))?;
    if !actor.is_admin && row.user_id != actor.id {
        return Err(AppError::Forbidden(
            "request log does not belong to current user".into(),
        ));
    }
    let item = request_item(&state.conn, row.clone()).await?;

    Ok(api_ok(RequestLogDetail {
        item,
        request_detail: row.request_detail,
        response_detail: row.response_detail,
    }))
}

/// 分页查询沙盒执行记录。
pub async fn execution_list(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<ExecutionLogListRequest>,
) -> Result<HttpResponse, AppError> {
    let page = body.page.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    validate_pagination(page, page_size)?;
    let user_id = if actor.is_admin {
        body.user_id
    } else {
        Some(actor.id)
    };

    // 1. 组装执行记录过滤条件。
    let mut query = b_executions::Entity::find().order_by_desc(b_executions::Column::Id);
    if let Some(user_id) = user_id {
        query = query.filter(b_executions::Column::UserId.eq(user_id));
    }
    if let Some(project_id) = body.project_id {
        query = query.filter(b_executions::Column::ProjectId.eq(project_id));
    }
    if let Some(status) = &body.status {
        query = query.filter(b_executions::Column::Status.eq(status));
    }
    let total = query
        .clone()
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let rows = query
        .limit(page_size)
        .offset((page - 1).saturating_mul(page_size))
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let items = rows.into_iter().map(execution_item).collect::<Vec<_>>();

    Ok(api_ok(
        serde_json::json!({ "total": total, "items": items }),
    ))
}

/// 查询执行详情，仅执行发起人或管理员可见。
pub async fn execution_detail(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let row = b_executions::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("execution does not exist".into()))?;
    if !actor.is_admin && row.user_id != actor.id {
        return Err(AppError::Forbidden(
            "execution does not belong to current user".into(),
        ));
    }

    Ok(api_ok(execution_item(row)))
}

async fn request_item(
    conn: &DatabaseConnection,
    row: b_request_logs::Model,
) -> Result<RequestLogItem, AppError> {
    let user = b_users::Entity::find_by_id(row.user_id)
        .one(conn)
        .await?
        .map(|user| user.email)
        .unwrap_or_default();
    let model = b_models::Entity::find_by_id(row.model_id)
        .one(conn)
        .await?
        .map(|model| model.name)
        .unwrap_or_default();
    let provider = b_providers::Entity::find_by_id(row.provider_id)
        .one(conn)
        .await?
        .map(|provider| provider.name)
        .unwrap_or_default();

    Ok(RequestLogItem {
        id: row.id,
        created_at: row.created_at.and_utc(),
        request_status: row.request_status,
        failure_reason: row.failure_reason,
        user_id: row.user_id,
        user_email: user,
        model_id: row.model_id,
        model_name: model,
        provider_id: row.provider_id,
        provider_name: provider,
        input_tokens: row.input_tokens as u64,
        output_tokens: row.output_tokens as u64,
        cache_tokens: row.cache_tokens as u64,
        ttft_ms: row.ttft_ms as u64,
        duration_ms: row.duration_ms as u64,
        contains_sensitive: row.contains_sensitive != 0,
    })
}

fn execution_item(row: b_executions::Model) -> ExecutionItem {
    ExecutionItem {
        id: row.id,
        script_id: row.script_id,
        script_revision: row.script_revision,
        project_id: row.project_id,
        environment_id: row.environment_id,
        r#type: row.r#type,
        status: row.status,
        stdout: row.stdout,
        stderr: row.stderr,
        error_code: row.error_code,
        sandbox_id: row.sandbox_id,
        user_id: row.user_id,
        started_at: row.started_at.map(|value| value.and_utc()),
        finished_at: row.finished_at.map(|value| value.and_utc()),
    }
}

fn validate_pagination(page: u64, page_size: u64) -> Result<(), AppError> {
    if page == 0 || page_size == 0 || page_size > 100 {
        Err(AppError::InvalidArgument(
            "page must be >= 1 and page_size must be between 1 and 100".into(),
        ))
    } else {
        Ok(())
    }
}
