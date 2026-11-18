use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter,
    QueryOrder, Set,
};
use serde_json::{Value, json};
use std::time::Instant;

use crate::entity::{
    b_models::{self as models},
    b_providers::{self as providers},
    b_request_logs, r_model_permissions, r_model_providers, r_provider_permissions,
};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::api_ok;
use crate::schema::gateway::{
    GatewayModel, GatewayProvider, ResponsesRequest, ResponsesResponse, UsageResponse,
};
use crate::service::sensitive::mask_json;
use crate::state::AppState;
use crate::utils::crypto::decrypt_secret;

/// 模型网关协议错误，按 OpenAI 兼容状态码返回。
#[derive(Debug)]
pub struct GatewayError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl GatewayError {
    fn forbidden(message: &str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "3",
            message: message.into(),
        }
    }

    fn not_found(message: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "4",
            message: message.into(),
        }
    }

    fn limit(message: &str) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "5",
            message: message.into(),
        }
    }

    fn upstream(message: String) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            code: "6",
            message,
        }
    }

    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "7",
            message,
        }
    }
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl ResponseError for GatewayError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(json!({
            "error": { "message": self.message, "code": self.code, "type": "agentsecret_error" }
        }))
    }
}

/// 可用模型列表；不暴露上游地址与令牌。
pub async fn models(
    user: AuthUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, GatewayError> {
    let rows = models::Entity::find()
        .filter(models::Column::DisabledAt.is_null())
        .order_by_asc(models::Column::Id)
        .all(&state.conn)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?;
    let allowed = allowed_model_ids(&state.conn, &user)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?;

    let mut result = Vec::new();
    for model in rows {
        if !user.is_admin && !allowed.as_ref().is_none_or(|ids| ids.contains(&model.id)) {
            continue;
        }
        let relations = r_model_providers::Entity::find()
            .filter(r_model_providers::Column::ModelId.eq(model.id))
            .all(&state.conn)
            .await
            .map_err(|e| GatewayError::internal(e.to_string()))?;
        result.push(GatewayModel {
            id: model.id,
            name: model.name.clone(),
            max_output_tokens: relations
                .first()
                .map(|relation| relation.max_output_tokens as u64)
                .unwrap_or(0),
            max_context_tokens: relations
                .first()
                .map(|relation| relation.max_context_tokens as u64)
                .unwrap_or(0),
            providers: relations
                .iter()
                .map(|relation| GatewayProvider {
                    provider_id: relation.provider_id,
                    model_name: relation.model_name.clone(),
                    enabled: relation.enabled != 0 && relation.disabled_at.is_none(),
                })
                .collect(),
        });
    }

    Ok(api_ok(result))
}

/// Responses 格式非流式模型调用。
pub async fn responses(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<ResponsesRequest>,
) -> Result<HttpResponse, GatewayError> {
    if body.stream == Some(true) {
        return responses_stream(user, state, body.into_inner()).await;
    }
    if body.model.trim().is_empty() {
        return Err(GatewayError::not_found("model does not exist"));
    }

    // 1. 查找启用模型并校验用户模型权限。
    let model = models::Entity::find()
        .filter(models::Column::Name.eq(&body.model))
        .filter(models::Column::DisabledAt.is_null())
        .one(&state.conn)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?
        .ok_or_else(|| GatewayError::not_found("model does not exist"))?;
    if !model_allowed(&state.conn, &user, model.id)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?
    {
        return Err(GatewayError::forbidden("model permission is required"));
    }

    // 2. 用户日 / 周 / 月配额在候选供应商循环内校验。

    // 3. 选择可用供应商，失败自动尝试下一个。
    let mut candidates = route_providers(&state, model.id, body.provider)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?;
    if !user.is_admin {
        filter_provider_permissions(&state.conn, &user, &mut candidates)
            .await
            .map_err(|e| GatewayError::internal(e.to_string()))?;
    }
    if candidates.is_empty() {
        return Err(GatewayError::upstream(
            "no available upstream provider".into(),
        ));
    }

    // 4. 请求先做敏感信息遮罩，再转发上游。
    let mut payload = serde_json::to_value(body.into_inner())
        .map_err(|e| GatewayError::internal(e.to_string()))?;
    let (masked_payload, request_sensitive) = mask_json(&payload);
    payload = masked_payload;
    let started = Instant::now();
    let mut first_byte_ms = 0_u64;
    let mut upstream_error = String::from("all upstream providers failed");
    let mut success = None;
    for (relation, provider) in candidates.iter().take(2) {
        check_user_quota(&state.conn, &user, model.id, provider.id)
            .await
            .map_err(|e| GatewayError::internal(e.to_string()))?;
        payload["model"] = Value::String(relation.model_name.clone());
        let call_start = Instant::now();
        match call_upstream(&state, provider, &payload).await {
            Ok(response) => {
                first_byte_ms = call_start.elapsed().as_millis() as u64;
                success = Some((relation.clone(), provider.clone(), response));
                break;
            }
            Err(error) => upstream_error = error,
        }
    }
    let Some((relation, provider, upstream_response)) = success else {
        let _ = insert_request_failure_log(
            &state.conn,
            user.id,
            model.id,
            candidates[0].1.id,
            started.elapsed().as_millis() as u64,
            &payload,
        )
        .await;
        return Err(GatewayError::upstream(upstream_error));
    };

    // 5. 响应遮罩、统计 usage 并写请求日志。
    let (masked_response, response_sensitive) = mask_json(&upstream_response);
    let contains_sensitive = request_sensitive || response_sensitive;
    let usage = UsageResponse {
        input_tokens: masked_response
            .get("usage")
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output_tokens: masked_response
            .get("usage")
            .and_then(|usage| usage.get("output_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_tokens: masked_response
            .get("usage")
            .and_then(|usage| usage.get("cache_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    insert_request_log(
        &state.conn,
        user.id,
        model.id,
        provider.id,
        usage.input_tokens,
        usage.output_tokens,
        usage.cache_tokens,
        first_byte_ms,
        duration_ms,
        contains_sensitive,
        &payload,
        &masked_response,
    )
    .await
    .map_err(|e| GatewayError::internal(e.to_string()))?;
    accrue_user_quota(
        &state.conn,
        &user,
        model.id,
        relation.provider_id,
        usage.input_tokens + usage.output_tokens,
    )
    .await
    .map_err(|e| GatewayError::internal(e.to_string()))?;
    accrue_provider_quota(
        &state.conn,
        provider.id,
        usage.input_tokens + usage.output_tokens,
    )
    .await
    .map_err(|e| GatewayError::internal(e.to_string()))?;

    Ok(HttpResponse::Ok().json(ResponsesResponse {
        id: format!("resp_{}", crate::utils::crypto::random_id()),
        object: "response",
        model: body_model_name(&masked_response, &model.name),
        output: masked_response
            .get("output")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
        usage,
        provider: provider.name.clone(),
        ttft_ms: first_byte_ms,
        duration_ms,
        contains_sensitive,
    }))
}

/// Responses 流式调用：当前采用上游完成后统一遮罩的 SSE 返回。
async fn responses_stream(
    user: AuthUser,
    state: web::Data<AppState>,
    body: ResponsesRequest,
) -> Result<HttpResponse, GatewayError> {
    let model = models::Entity::find()
        .filter(models::Column::Name.eq(&body.model))
        .filter(models::Column::DisabledAt.is_null())
        .one(&state.conn)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?
        .ok_or_else(|| GatewayError::not_found("model does not exist"))?;
    if !model_allowed(&state.conn, &user, model.id)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?
    {
        return Err(GatewayError::forbidden("model permission is required"));
    }
    let mut candidates = route_providers(&state, model.id, body.provider)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?;
    if !user.is_admin {
        filter_provider_permissions(&state.conn, &user, &mut candidates)
            .await
            .map_err(|e| GatewayError::internal(e.to_string()))?;
    }
    if candidates.is_empty() {
        return Err(GatewayError::upstream(
            "no available upstream provider".into(),
        ));
    }

    // 1. 流式请求同样先遮罩输入。
    let mut payload =
        serde_json::to_value(&body).map_err(|e| GatewayError::internal(e.to_string()))?;
    let (masked_payload, request_sensitive) = mask_json(&payload);
    payload = masked_payload;
    let started = Instant::now();
    let mut upstream_error = String::from("all upstream providers failed");
    let mut stream_result = None;
    for (relation, provider) in candidates.iter().take(2) {
        check_user_quota(&state.conn, &user, model.id, provider.id)
            .await
            .map_err(|e| GatewayError::internal(e.to_string()))?;
        payload["model"] = Value::String(relation.model_name.clone());
        match call_upstream_stream(&state, provider, &payload).await {
            Ok(bytes) => {
                stream_result = Some((relation.clone(), provider.clone(), bytes));
                break;
            }
            Err(error) => upstream_error = error,
        }
    }
    let Some((relation, provider, bytes)) = stream_result else {
        let _ = insert_request_failure_log(
            &state.conn,
            user.id,
            model.id,
            candidates[0].1.id,
            started.elapsed().as_millis() as u64,
            &payload,
        )
        .await;
        return Err(GatewayError::upstream(upstream_error));
    };

    // 2. 为保证跨块规则完整命中，先完整接收再统一遮罩并落日志。
    let raw = String::from_utf8_lossy(&bytes).into_owned();
    let (masked, response_sensitive) = crate::service::sensitive::mask_text(&raw);
    let contains_sensitive = request_sensitive || response_sensitive;
    let duration_ms = started.elapsed().as_millis() as u64;
    let response_detail = Value::String(masked.clone());
    insert_request_log(
        &state.conn,
        user.id,
        model.id,
        provider.id,
        0,
        0,
        0,
        duration_ms,
        duration_ms,
        contains_sensitive,
        &payload,
        &response_detail,
    )
    .await
    .map_err(|e| GatewayError::internal(e.to_string()))?;
    accrue_user_quota(&state.conn, &user, model.id, relation.provider_id, 0)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?;

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .body(masked))
}

async fn allowed_model_ids(
    conn: &DatabaseConnection,
    user: &AuthUser,
) -> Result<Option<Vec<u64>>, AppError> {
    let permissions = r_model_permissions::Entity::find()
        .filter(r_model_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await?;
    if permissions.is_empty()
        || permissions
            .iter()
            .any(|permission| permission.model_id.is_none())
    {
        return Ok(None);
    }

    Ok(Some(
        permissions
            .into_iter()
            .filter_map(|permission| permission.model_id)
            .collect(),
    ))
}

async fn model_allowed(
    conn: &DatabaseConnection,
    user: &AuthUser,
    model_id: u64,
) -> Result<bool, AppError> {
    if user.is_admin {
        return Ok(true);
    }
    let permissions = r_model_permissions::Entity::find()
        .filter(r_model_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await?;
    if permissions.is_empty() {
        return Ok(true);
    }

    Ok(permissions
        .iter()
        .any(|permission| permission.model_id.is_none_or(|id| id == model_id)))
}

async fn check_user_quota(
    conn: &DatabaseConnection,
    user: &AuthUser,
    model_id: u64,
    provider_id: u64,
) -> Result<(), GatewayError> {
    // 1. 模型维度命中时优先使用模型权限，不叠加供应商权限。
    let model_permissions = r_model_permissions::Entity::find()
        .filter(r_model_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?;
    let model_matches = model_permissions
        .into_iter()
        .filter(|permission| permission.model_id.is_none_or(|id| id == model_id))
        .collect::<Vec<_>>();
    if let Some(permission) = model_matches
        .into_iter()
        .min_by_key(|permission| permission.model_id.map_or(1, |_| 0))
    {
        let permission = refresh_model_permission_windows(conn, permission)
            .await
            .map_err(|e| GatewayError::internal(e.to_string()))?;
        if quota_exceeded(
            permission.daily_token_limit,
            permission.daily_token_used,
            permission.weekly_token_limit,
            permission.weekly_token_used,
            permission.monthly_token_limit,
            permission.monthly_token_used,
        ) {
            return Err(GatewayError::limit("user token quota exceeded"));
        }

        return Ok(());
    }

    // 2. 没有模型权限时再检查最具体的供应商权限窗口。
    let provider_permissions = r_provider_permissions::Entity::find()
        .filter(r_provider_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await
        .map_err(|e| GatewayError::internal(e.to_string()))?;
    let provider_matches: Vec<_> = provider_permissions
        .into_iter()
        .filter(|permission| permission.provider_id.is_none_or(|id| id == provider_id))
        .collect();
    if let Some(permission) = provider_matches
        .into_iter()
        .min_by_key(|permission| permission.provider_id.map_or(1, |_| 0))
    {
        let permission = refresh_provider_permission_windows(conn, permission)
            .await
            .map_err(|e| GatewayError::internal(e.to_string()))?;
        if quota_exceeded(
            permission.daily_token_limit,
            permission.daily_token_used,
            permission.weekly_token_limit,
            permission.weekly_token_used,
            permission.monthly_token_limit,
            permission.monthly_token_used,
        ) {
            return Err(GatewayError::limit("user token quota exceeded"));
        }
    }

    Ok(())
}

fn quota_exceeded(
    daily_limit: Option<i64>,
    daily_used: u64,
    weekly_limit: Option<i64>,
    weekly_used: u64,
    monthly_limit: Option<i64>,
    monthly_used: u64,
) -> bool {
    daily_limit.is_some_and(|limit| daily_used >= limit as u64)
        || weekly_limit.is_some_and(|limit| weekly_used >= limit as u64)
        || monthly_limit.is_some_and(|limit| monthly_used >= limit as u64)
}

/// 将权限表的日 / 周 / 月用量按窗口起点重置并持久化。
async fn refresh_model_permission_windows(
    conn: &DatabaseConnection,
    permission: r_model_permissions::Model,
) -> Result<r_model_permissions::Model, AppError> {
    use chrono::Datelike;

    let now = Utc::now().naive_utc();
    let today = now.date();
    let daily_start = today.and_hms_opt(0, 0, 0).unwrap_or(now);
    let weekly_day = today
        .checked_sub_signed(chrono::Duration::days(
            today.weekday().num_days_from_monday() as i64,
        ))
        .unwrap_or(today);
    let weekly_start = weekly_day.and_hms_opt(0, 0, 0).unwrap_or(daily_start);
    let monthly_day = today.with_day(1).unwrap_or(today);
    let monthly_start = monthly_day.and_hms_opt(0, 0, 0).unwrap_or(daily_start);

    let daily_refreshed = permission.daily_token_refreshed_at;
    let weekly_refreshed = permission.weekly_token_refreshed_at;
    let monthly_refreshed = permission.monthly_token_refreshed_at;
    let mut active: r_model_permissions::ActiveModel = permission.into();
    if daily_refreshed.is_none_or(|value| value < daily_start) {
        active.daily_token_used = Set(0);
        active.daily_token_refreshed_at = Set(Some(daily_start));
    }
    if weekly_refreshed.is_none_or(|value| value < weekly_start) {
        active.weekly_token_used = Set(0);
        active.weekly_token_refreshed_at = Set(Some(weekly_start));
    }
    if monthly_refreshed.is_none_or(|value| value < monthly_start) {
        active.monthly_token_used = Set(0);
        active.monthly_token_refreshed_at = Set(Some(monthly_start));
    }

    active.update(conn).await.map_err(AppError::from)
}

/// 将供应商权限的日 / 周 / 月用量按窗口起点重置并持久化。
async fn refresh_provider_permission_windows(
    conn: &DatabaseConnection,
    permission: r_provider_permissions::Model,
) -> Result<r_provider_permissions::Model, AppError> {
    use chrono::Datelike;

    let now = Utc::now().naive_utc();
    let today = now.date();
    let daily_start = today.and_hms_opt(0, 0, 0).unwrap_or(now);
    let weekly_day = today
        .checked_sub_signed(chrono::Duration::days(
            today.weekday().num_days_from_monday() as i64,
        ))
        .unwrap_or(today);
    let weekly_start = weekly_day.and_hms_opt(0, 0, 0).unwrap_or(daily_start);
    let monthly_day = today.with_day(1).unwrap_or(today);
    let monthly_start = monthly_day.and_hms_opt(0, 0, 0).unwrap_or(daily_start);

    let mut active: r_provider_permissions::ActiveModel = permission.into();
    if active
        .daily_token_refreshed_at
        .as_ref()
        .is_none_or(|value| value < daily_start)
    {
        active.daily_token_used = Set(0);
        active.daily_token_refreshed_at = Set(Some(daily_start));
    }
    if active
        .weekly_token_refreshed_at
        .as_ref()
        .is_none_or(|value| value < weekly_start)
    {
        active.weekly_token_used = Set(0);
        active.weekly_token_refreshed_at = Set(Some(weekly_start));
    }
    if active
        .monthly_token_refreshed_at
        .as_ref()
        .is_none_or(|value| value < monthly_start)
    {
        active.monthly_token_used = Set(0);
        active.monthly_token_refreshed_at = Set(Some(monthly_start));
    }

    active.update(conn).await.map_err(AppError::from)
}

/// 按供应商权限裁剪候选；无权限记录时按设计默认放行。
async fn filter_provider_permissions(
    conn: &DatabaseConnection,
    user: &AuthUser,
    candidates: &mut Vec<(r_model_providers::Model, providers::Model)>,
) -> Result<(), AppError> {
    let permissions = r_provider_permissions::Entity::find()
        .filter(r_provider_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await?;
    if permissions.is_empty() {
        return Ok(());
    }
    candidates.retain(|(_, provider)| {
        permissions
            .iter()
            .any(|permission| permission.provider_id.is_none_or(|id| id == provider.id))
    });

    Ok(())
}

async fn route_providers(
    state: &AppState,
    model_id: u64,
    provider_id: Option<u64>,
) -> Result<Vec<(r_model_providers::Model, providers::Model)>, AppError> {
    let conn = &state.conn;
    let relations = r_model_providers::Entity::find()
        .filter(r_model_providers::Column::ModelId.eq(model_id))
        .filter(r_model_providers::Column::Enabled.eq(1))
        .filter(r_model_providers::Column::DisabledAt.is_null())
        .all(conn)
        .await?;
    let mut result = Vec::new();
    for relation in relations {
        if provider_id.is_some_and(|id| id != relation.provider_id) {
            continue;
        }
        let Some(provider) = providers::Entity::find_by_id(relation.provider_id)
            .one(conn)
            .await?
        else {
            continue;
        };
        let mut provider = provider;

        // 1. 到达刷新时间的 0 额度先尝试同步刷新；失败则本轮不路由。
        if (quota_window_expired(provider.quota_5h_remaining, provider.quota_5h_refreshed_at)
            || quota_window_expired(
                provider.quota_weekly_remaining,
                provider.quota_weekly_refreshed_at,
            ))
            && let Ok(refreshed) =
                crate::service::provider_quota::refresh_provider_quota(state, &provider).await
        {
            provider = refreshed;
        }

        // 2. 只保留启用且未耗尽的供应商。
        let quota_available =
            quota_available(provider.quota_5h_remaining, provider.quota_5h_refreshed_at)
                && quota_available(
                    provider.quota_weekly_remaining,
                    provider.quota_weekly_refreshed_at,
                );
        if provider.disabled_at.is_none() && quota_available {
            result.push((relation, provider));
        }
    }

    // 3. 先按剩余额度与最近测试时间排序，再只在同分供应商内轮询。
    result.sort_by(|(left_relation, left), (right_relation, right)| {
        provider_score(right)
            .cmp(&provider_score(left))
            .then_with(|| {
                right_relation
                    .last_tested_at
                    .cmp(&left_relation.last_tested_at)
            })
    });
    if !result.is_empty() {
        let top_score = provider_score(&result[0].1);
        let same_score = result
            .iter()
            .take_while(|(_, provider)| provider_score(provider) == top_score)
            .count();
        let cursor = state
            .provider_route_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed) as usize;
        result.rotate_left(cursor % same_score.max(1));
    }

    Ok(result)
}

fn provider_score(provider: &providers::Model) -> i64 {
    let five_hours = provider.quota_5h_remaining.unwrap_or(i64::MAX);
    let weekly = provider.quota_weekly_remaining.unwrap_or(i64::MAX);

    five_hours.min(weekly)
}

/// 读取上游流式响应的完整字节；用于统一敏感信息过滤。
async fn call_upstream_stream(
    state: &AppState,
    provider: &providers::Model,
    payload: &Value,
) -> Result<Vec<u8>, String> {
    let access_token = decrypt_secret(state.config.master_key.as_bytes(), &provider.access_token)
        .map_err(|_| "upstream request failed".to_string())?;
    let request =
        crate::service::upstream::build(&provider.base_url, &provider.provider_type, payload)
            .map_err(|_| "upstream request failed".to_string())?;
    let response = state
        .http
        .post(request.url)
        .bearer_auth(access_token)
        .json(&request.body)
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
        .map_err(|_| "upstream request failed".to_string())?;
    if !response.status().is_success() {
        return Err(format!("upstream returned status {}", response.status()));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| error.to_string())
}

async fn call_upstream(
    state: &AppState,
    provider: &providers::Model,
    payload: &Value,
) -> Result<Value, String> {
    let access_token = decrypt_secret(state.config.master_key.as_bytes(), &provider.access_token)
        .map_err(|_| "upstream request failed".to_string())?;
    let request =
        crate::service::upstream::build(&provider.base_url, &provider.provider_type, payload)
            .map_err(|_| "upstream request failed".to_string())?;
    let response = state
        .http
        .post(request.url)
        .bearer_auth(access_token)
        .json(&request.body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .map_err(|_| "upstream request failed".to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("upstream returned {status}: {text}"));
    }

    let upstream_response = response
        .json::<Value>()
        .await
        .map_err(|_| "upstream request failed".to_string())?;
    crate::service::upstream::normalize(&provider.provider_type, upstream_response)
        .map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
async fn insert_request_log(
    conn: &DatabaseConnection,
    user_id: u64,
    model_id: u64,
    provider_id: u64,
    input_tokens: u64,
    output_tokens: u64,
    cache_tokens: u64,
    ttft_ms: u64,
    duration_ms: u64,
    contains_sensitive: bool,
    request_detail: &Value,
    response_detail: &Value,
) -> Result<(), AppError> {
    let active = b_request_logs::ActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        model_id: Set(model_id),
        provider_id: Set(provider_id),
        token_id: Set(None),
        input_tokens: Set(input_tokens as u32),
        output_tokens: Set(output_tokens as u32),
        cache_tokens: Set(cache_tokens as u32),
        ttft_ms: Set(ttft_ms as u32),
        duration_ms: Set(duration_ms as u32),
        contains_sensitive: Set(contains_sensitive as i8),
        request_detail: Set(Some(request_detail.clone())),
        response_detail: Set(Some(response_detail.clone())),
        request_status: Set("success".into()),
        failure_reason: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    };
    active.insert(conn).await?;

    Ok(())
}

async fn insert_request_failure_log(
    conn: &DatabaseConnection,
    user_id: u64,
    model_id: u64,
    provider_id: u64,
    duration_ms: u64,
    request_detail: &Value,
) -> Result<(), AppError> {
    let active = b_request_logs::ActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        model_id: Set(model_id),
        provider_id: Set(provider_id),
        token_id: Set(None),
        input_tokens: Set(0),
        output_tokens: Set(0),
        cache_tokens: Set(0),
        ttft_ms: Set(0),
        duration_ms: Set(duration_ms as u32),
        contains_sensitive: Set(0),
        request_detail: Set(Some(request_detail.clone())),
        response_detail: Set(Some(Value::Object(Default::default()))),
        request_status: Set("failure".into()),
        failure_reason: Set(Some("all upstream providers failed".into())),
        created_at: Set(Utc::now().naive_utc()),
    };
    active.insert(conn).await?;

    Ok(())
}

async fn accrue_user_quota(
    conn: &DatabaseConnection,
    user: &AuthUser,
    model_id: u64,
    provider_id: u64,
    tokens: u64,
) -> Result<(), AppError> {
    // 1. 命中模型权限时优先累加到最具体的一条；否则落到供应商权限。
    let model_permissions = r_model_permissions::Entity::find()
        .filter(r_model_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await?;
    let model_permission = model_permissions
        .into_iter()
        .filter(|permission| permission.model_id.is_none_or(|id| id == model_id))
        .min_by_key(|permission| permission.model_id.map_or(1, |_| 0));
    if let Some(permission) = model_permission {
        let mut active: r_model_permissions::ActiveModel = permission.into();
        active.daily_token_used = Set(active.daily_token_used.unwrap() + tokens);
        active.weekly_token_used = Set(active.weekly_token_used.unwrap() + tokens);
        active.monthly_token_used = Set(active.monthly_token_used.unwrap() + tokens);
        active.update(conn).await?;

        return Ok(());
    }

    let provider_permissions = r_provider_permissions::Entity::find()
        .filter(r_provider_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await?;
    let provider_permission = provider_permissions
        .into_iter()
        .filter(|permission| permission.provider_id.is_none_or(|id| id == provider_id))
        .min_by_key(|permission| permission.provider_id.map_or(1, |_| 0));
    if let Some(permission) = provider_permission {
        let mut active: r_provider_permissions::ActiveModel = permission.into();
        active.daily_token_used = Set(active.daily_token_used.unwrap() + tokens);
        active.weekly_token_used = Set(active.weekly_token_used.unwrap() + tokens);
        active.monthly_token_used = Set(active.monthly_token_used.unwrap() + tokens);
        active.update(conn).await?;
    }

    Ok(())
}

fn quota_available(remaining: Option<i64>, refreshed_at: Option<chrono::NaiveDateTime>) -> bool {
    remaining.is_none_or(|value| value > 0)
        || refreshed_at.is_some_and(|value| value <= Utc::now().naive_utc())
}

fn quota_window_expired(
    remaining: Option<i64>,
    refreshed_at: Option<chrono::NaiveDateTime>,
) -> bool {
    remaining.is_some_and(|value| value <= 0)
        && refreshed_at.is_some_and(|value| value <= Utc::now().naive_utc())
}

async fn accrue_provider_quota(
    conn: &DatabaseConnection,
    provider_id: u64,
    tokens: u64,
) -> Result<(), AppError> {
    let provider = providers::Entity::find_by_id(provider_id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("provider does not exist".into()))?;
    let mut active: providers::ActiveModel = provider.into();
    if let Some(used) = active.quota_5h_remaining.as_ref() {
        active.quota_5h_remaining = Set(Some((*used).saturating_sub(tokens as i64)));
    }
    if let Some(used) = active.quota_weekly_remaining.as_ref() {
        active.quota_weekly_remaining = Set(Some((*used).saturating_sub(tokens as i64)));
    }
    if let Some(used) = active.quota_monthly_remaining.as_ref() {
        active.quota_monthly_remaining = Set(Some((*used).saturating_sub(tokens as i64)));
    }
    active.update(conn).await?;

    Ok(())
}

fn body_model_name(response: &Value, fallback: &str) -> String {
    response
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}
