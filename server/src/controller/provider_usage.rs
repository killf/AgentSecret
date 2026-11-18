use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Serialize;

use crate::entity::{b_providers, r_model_providers, r_provider_permissions};
use crate::error::AppError;
use crate::schema::admin::IdRequest;
use crate::schema::api_ok;
use crate::state::AppState;
use crate::utils::crypto::decrypt_secret;

#[derive(Serialize)]
pub struct ConnectivityResponse {
    pub ok: bool,
    pub latency_ms: Option<u128>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ProviderUsageResponse {
    pub quota_5h_remaining: Option<i64>,
    pub quota_5h_refreshed_at: Option<String>,
    pub quota_weekly_remaining: Option<i64>,
    pub quota_weekly_refreshed_at: Option<String>,
    pub quota_monthly_remaining: Option<i64>,
    pub quota_monthly_refreshed_at: Option<String>,
}

/// 测试模型-供应商连通性；错误只返回固定摘要，不透出上游原文。
pub async fn test_model_provider(
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let relation = r_model_providers::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("model provider relation does not exist".into()))?;
    let provider = b_providers::Entity::find_by_id(relation.provider_id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("provider does not exist".into()))?;

    // 1. 发送最小 Responses 请求并计时；失败只保留固定错误摘要。
    let started = std::time::Instant::now();
    let token = decrypt_secret(state.config.master_key.as_bytes(), &provider.access_token)?;
    let base = provider.base_url.trim_end_matches('/');
    let url = if base.ends_with("/responses") {
        base.to_string()
    } else {
        format!("{base}/v1/responses")
    };
    let result = state
        .http
        .post(url)
        .bearer_auth(token)
        .json(&serde_json::json!({
            "model": relation.model_name,
            "input": "connectivity test",
            "max_output_tokens": 1
        }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;
    let response = match result {
        Ok(response) if response.status().is_success() => ConnectivityResponse {
            ok: true,
            latency_ms: Some(started.elapsed().as_millis()),
            error: None,
        },
        Ok(response) => ConnectivityResponse {
            ok: false,
            latency_ms: Some(started.elapsed().as_millis()),
            error: Some(format!("upstream returned status {}", response.status())),
        },
        Err(_) => ConnectivityResponse {
            ok: false,
            latency_ms: Some(started.elapsed().as_millis()),
            error: Some("upstream request failed".into()),
        },
    };

    // 2. 无论成败都记录最近测试时间。
    let mut active: r_model_providers::ActiveModel = relation.into();
    active.last_tested_at = Set(Some(Utc::now().naive_utc()));
    active.update(&state.conn).await?;

    Ok(api_ok(response))
}

/// 查询供应商当前额度；不实时访问上游。
pub async fn provider_usage(
    actor: crate::middleware::AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let provider = b_providers::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("provider does not exist".into()))?;
    if !actor.is_admin {
        let permissions = r_provider_permissions::Entity::find()
            .filter(r_provider_permissions::Column::UserId.eq(actor.id))
            .all(&state.conn)
            .await?;
        let allowed = permissions
            .iter()
            .any(|permission| permission.provider_id.is_none_or(|id| id == provider.id));
        if !allowed {
            return Err(AppError::Forbidden(
                "provider permission is required".into(),
            ));
        }
    }

    Ok(api_ok(usage_response(provider)))
}

/// 刷新供应商额度；失败时保留并返回当前持久化值。
pub async fn refresh_provider_quota(
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let provider = b_providers::Entity::find_by_id(body.id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("provider does not exist".into()))?;

    // 1. 按供应商类型读取部署侧接口映射并回写统一字段。
    let provider =
        match crate::service::provider_quota::refresh_provider_quota(&state, &provider).await {
            Ok(provider) => provider,
            Err(error) => {
                tracing::warn!(
                    provider_id = provider.id,
                    error,
                    "provider quota refresh failed"
                );
                provider
            }
        };

    // 2. 刷新失败不阻塞管理页，仍返回当前持久化值供审计。
    Ok(api_ok(usage_response(provider)))
}

fn usage_response(provider: b_providers::Model) -> ProviderUsageResponse {
    ProviderUsageResponse {
        quota_5h_remaining: provider.quota_5h_remaining,
        quota_5h_refreshed_at: provider
            .quota_5h_refreshed_at
            .map(|value| value.and_utc().to_rfc3339()),
        quota_weekly_remaining: provider.quota_weekly_remaining,
        quota_weekly_refreshed_at: provider
            .quota_weekly_refreshed_at
            .map(|value| value.and_utc().to_rfc3339()),
        quota_monthly_remaining: provider.quota_monthly_remaining,
        quota_monthly_refreshed_at: provider
            .quota_monthly_refreshed_at
            .map(|value| value.and_utc().to_rfc3339()),
    }
}
