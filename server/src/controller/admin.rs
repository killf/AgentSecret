use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};

use crate::entity::{
    b_models::{self as models},
    b_providers::{self as providers},
    b_request_logs, r_model_permissions, r_model_providers, r_provider_permissions,
};
use crate::error::AppError;
use crate::schema::admin::{
    CreateModelProviderRequest, CreateModelRequest, CreateProviderRequest, CreatedResponse,
    IdRequest, ModelProviderItem, ModelProviderListRequest, ProviderInfo,
    UpdateModelProviderRequest, UpdateModelRequest, UpdateProviderRequest,
};
use crate::schema::api_ok;
use crate::state::AppState;
use crate::utils::crypto::encrypt_secret;

/// 模型列表。
pub async fn list_models(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let rows = models::Entity::find()
        .order_by_asc(models::Column::Id)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(rows))
}

/// 创建模型。
pub async fn add_model(
    state: web::Data<AppState>,
    body: web::Json<CreateModelRequest>,
) -> Result<HttpResponse, AppError> {
    validate_name(&body.name)?;
    if model_name_exists(&state.conn, &body.name, None).await? {
        return Err(AppError::InvalidArgument(
            "model name already exists".into(),
        ));
    }

    let now = Utc::now().naive_utc();
    let active = models::ActiveModel {
        id: NotSet,
        name: Set(body.name.clone()),
        disabled_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let model = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(CreatedResponse { id: model.id }))
}

/// 更新模型名称或启停状态。
pub async fn update_model(
    state: web::Data<AppState>,
    body: web::Json<UpdateModelRequest>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    if let Some(name) = body.name.as_deref() {
        validate_name(name)?;
        if model_name_exists(&state.conn, name, Some(body.id)).await? {
            return Err(AppError::InvalidArgument(
                "model name already exists".into(),
            ));
        }
    }
    let model = find_model(&state.conn, body.id).await?;
    let mut active: models::ActiveModel = model.into();
    if let Some(name) = body.name {
        active.name = Set(name);
    }
    if let Some(disabled_at) = body.disabled_at {
        active.disabled_at = Set(disabled_at.map(|value| value.naive_utc()));
    }
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 删除未被引用的模型。
pub async fn delete_model(
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    find_model(&state.conn, body.id).await?;

    // 1. 关系、权限和请求日志都保留历史引用，存在即拒绝删除。
    let relation_count = r_model_providers::Entity::find()
        .filter(r_model_providers::Column::ModelId.eq(body.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let permission_count = r_model_permissions::Entity::find()
        .filter(r_model_permissions::Column::ModelId.eq(body.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let log_count = b_request_logs::Entity::find()
        .filter(b_request_logs::Column::ModelId.eq(body.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if relation_count + permission_count + log_count > 0 {
        return Err(AppError::NotFound("model is still referenced".into()));
    }

    models::Entity::delete_by_id(body.id)
        .exec(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 查询模型-供应商关系；可按模型过滤。
pub async fn list_model_providers(
    state: web::Data<AppState>,
    body: web::Json<ModelProviderListRequest>,
) -> Result<HttpResponse, AppError> {
    let mut query = r_model_providers::Entity::find().order_by_asc(r_model_providers::Column::Id);
    if let Some(model_id) = body.model_id {
        query = query.filter(r_model_providers::Column::ModelId.eq(model_id));
    }
    let rows = query.all(&state.conn).await.map_err(AppError::internal)?;
    let items = rows
        .into_iter()
        .map(|row| ModelProviderItem {
            id: row.id,
            model_id: row.model_id,
            provider_id: row.provider_id,
            model_name: row.model_name,
            max_output_tokens: row.max_output_tokens as u64,
            max_context_tokens: row.max_context_tokens as u64,
            enabled: row.enabled != 0,
        })
        .collect::<Vec<_>>();

    Ok(api_ok(items))
}

/// 新增模型-供应商关系。
pub async fn add_model_provider(
    state: web::Data<AppState>,
    body: web::Json<CreateModelProviderRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.model_id)?;
    validate_id(body.provider_id)?;
    validate_name(&body.model_name)?;
    validate_tokens(body.max_output_tokens)?;
    validate_tokens(body.max_context_tokens)?;
    find_model(&state.conn, body.model_id).await?;
    find_provider(&state.conn, body.provider_id).await?;

    let exists = r_model_providers::Entity::find()
        .filter(r_model_providers::Column::ModelId.eq(body.model_id))
        .filter(r_model_providers::Column::ProviderId.eq(body.provider_id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if exists > 0 {
        return Err(AppError::InvalidArgument(
            "model provider relation already exists".into(),
        ));
    }

    let active = r_model_providers::ActiveModel {
        id: NotSet,
        model_id: Set(body.model_id),
        provider_id: Set(body.provider_id),
        model_name: Set(body.model_name.clone()),
        max_output_tokens: Set(body.max_output_tokens as u32),
        max_context_tokens: Set(body.max_context_tokens as u32),
        enabled: Set(body.enabled.unwrap_or(true) as i8),
        last_tested_at: Set(None),
        disabled_at: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    };
    let relation = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(CreatedResponse { id: relation.id }))
}

/// 更新模型-供应商关系。
pub async fn update_model_provider(
    state: web::Data<AppState>,
    body: web::Json<UpdateModelProviderRequest>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    if let Some(name) = body.model_name.as_deref() {
        validate_name(name)?;
    }
    if let Some(value) = body.max_output_tokens {
        validate_tokens(value)?;
    }
    if let Some(value) = body.max_context_tokens {
        validate_tokens(value)?;
    }
    let relation = r_model_providers::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("model provider relation does not exist".into()))?;
    let mut active: r_model_providers::ActiveModel = relation.into();
    if let Some(name) = body.model_name {
        active.model_name = Set(name);
    }
    if let Some(value) = body.max_output_tokens {
        active.max_output_tokens = Set(value as u32);
    }
    if let Some(value) = body.max_context_tokens {
        active.max_context_tokens = Set(value as u32);
    }
    if let Some(value) = body.enabled {
        active.enabled = Set(value as i8);
    }
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 删除模型-供应商关系。
pub async fn delete_model_provider(
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    let result = r_model_providers::Entity::delete_by_id(body.id)
        .exec(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if result.rows_affected == 0 {
        return Err(AppError::NotFound(
            "model provider relation does not exist".into(),
        ));
    }

    Ok(api_ok(Option::<()>::None))
}

/// 供应商列表；不返回上游令牌。
pub async fn list_providers(state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let rows = providers::Entity::find()
        .order_by_asc(providers::Column::Id)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let items = rows
        .into_iter()
        .map(|provider| ProviderInfo {
            id: provider.id,
            name: provider.name,
            base_url: provider.base_url,
            provider_type: provider.provider_type,
            quota_5h_remaining: provider.quota_5h_remaining,
            quota_5h_refreshed_at: provider.quota_5h_refreshed_at.map(|v| v.and_utc()),
            quota_weekly_remaining: provider.quota_weekly_remaining,
            quota_weekly_refreshed_at: provider.quota_weekly_refreshed_at.map(|v| v.and_utc()),
            quota_monthly_remaining: provider.quota_monthly_remaining,
            quota_monthly_refreshed_at: provider.quota_monthly_refreshed_at.map(|v| v.and_utc()),
            disabled_at: provider.disabled_at.map(|v| v.and_utc()),
        })
        .collect::<Vec<_>>();

    Ok(api_ok(items))
}

/// 创建供应商并加密上游令牌。
pub async fn add_provider(
    state: web::Data<AppState>,
    body: web::Json<CreateProviderRequest>,
) -> Result<HttpResponse, AppError> {
    validate_name(&body.name)?;
    validate_base_url(&body.base_url)?;
    if !matches!(body.provider_type.as_str(), "minimax" | "glm" | "codex") {
        return Err(AppError::InvalidArgument(
            "provider_type must be minimax, glm, or codex".into(),
        ));
    }
    if body.access_token.is_empty() {
        return Err(AppError::InvalidArgument("access_token is required".into()));
    }
    let exists = providers::Entity::find()
        .filter(providers::Column::Name.eq(&body.name))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if exists > 0 {
        return Err(AppError::InvalidArgument(
            "provider name already exists".into(),
        ));
    }

    let now = Utc::now().naive_utc();
    let access_token = encrypt_secret(state.config.master_key.as_bytes(), &body.access_token)?;
    let active = providers::ActiveModel {
        id: NotSet,
        name: Set(body.name.clone()),
        base_url: Set(body.base_url.clone()),
        access_token: Set(access_token),
        provider_type: Set(body.provider_type.clone()),
        quota_5h_remaining: Set(None),
        quota_5h_refreshed_at: Set(None),
        quota_weekly_remaining: Set(None),
        quota_weekly_refreshed_at: Set(None),
        quota_monthly_remaining: Set(None),
        quota_monthly_refreshed_at: Set(None),
        disabled_at: Set(None),
        created_by: Set(None),
        created_at: Set(now),
        updated_by: Set(None),
        updated_at: Set(now),
    };
    let provider = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(CreatedResponse { id: provider.id }))
}

/// 更新供应商基础信息和上游令牌。
pub async fn update_provider(
    state: web::Data<AppState>,
    body: web::Json<UpdateProviderRequest>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    if let Some(name) = body.name.as_deref() {
        validate_name(name)?;
    }
    if let Some(base_url) = body.base_url.as_deref() {
        validate_base_url(base_url)?;
    }
    let provider = find_provider(&state.conn, body.id).await?;
    let mut active: providers::ActiveModel = provider.into();
    if let Some(name) = body.name {
        active.name = Set(name);
    }
    if let Some(base_url) = body.base_url {
        active.base_url = Set(base_url);
    }
    if let Some(token) = body.access_token {
        if token.is_empty() {
            return Err(AppError::InvalidArgument(
                "access_token cannot be empty".into(),
            ));
        }
        active.access_token = Set(encrypt_secret(state.config.master_key.as_bytes(), &token)?);
    }
    if let Some(disabled_at) = body.disabled_at {
        active.disabled_at = Set(disabled_at.map(|value| value.naive_utc()));
    }
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 删除未被引用的供应商。
pub async fn delete_provider(
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    find_provider(&state.conn, body.id).await?;
    let relation_count = r_model_providers::Entity::find()
        .filter(r_model_providers::Column::ProviderId.eq(body.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let permission_count = r_provider_permissions::Entity::find()
        .filter(r_provider_permissions::Column::ProviderId.eq(body.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let log_count = b_request_logs::Entity::find()
        .filter(b_request_logs::Column::ProviderId.eq(body.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if relation_count + permission_count + log_count > 0 {
        return Err(AppError::NotFound("provider is still referenced".into()));
    }

    providers::Entity::delete_by_id(body.id)
        .exec(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

async fn find_model(
    conn: &sea_orm::DatabaseConnection,
    id: u64,
) -> Result<models::Model, AppError> {
    models::Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("model does not exist".into()))
}

async fn find_provider(
    conn: &sea_orm::DatabaseConnection,
    id: u64,
) -> Result<providers::Model, AppError> {
    providers::Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("provider does not exist".into()))
}

async fn model_name_exists(
    conn: &sea_orm::DatabaseConnection,
    name: &str,
    excluded_id: Option<u64>,
) -> Result<bool, AppError> {
    let mut query = models::Entity::find().filter(models::Column::Name.eq(name));
    if let Some(id) = excluded_id {
        query = query.filter(models::Column::Id.ne(id));
    }
    let count = query.count(conn).await.map_err(AppError::internal)?;

    Ok(count > 0)
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

fn validate_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() || name != name.trim() || name.len() > 128 {
        Err(AppError::InvalidArgument(
            "name must contain 1 to 128 characters without surrounding spaces".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_base_url(url: &str) -> Result<(), AppError> {
    if url.starts_with("https://") && url.len() <= 512 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "base_url must be an HTTPS URL".into(),
        ))
    }
}

fn validate_tokens(value: u64) -> Result<(), AppError> {
    if value == 0 || value > u32::MAX as u64 {
        Err(AppError::InvalidArgument(format!(
            "token limit must be between 1 and {}",
            u32::MAX
        )))
    } else {
        Ok(())
    }
}
