//! 供应商额度查询：按部署配置把各厂商响应映射到统一额度字段。

use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;

use crate::entity::b_providers;
use crate::state::AppState;
use crate::utils::crypto::decrypt_secret;

/// 调用固定额度接口并回写供应商表；错误只向上返回固定摘要。
pub async fn refresh_provider_quota(
    state: &AppState,
    provider: &b_providers::Model,
) -> Result<b_providers::Model, String> {
    let definition = state
        .config
        .provider_quotas
        .get(&provider.provider_type)
        .ok_or_else(|| "provider quota endpoint is not configured".to_string())?;
    let token = decrypt_secret(state.config.master_key.as_bytes(), &provider.access_token)
        .map_err(|_| "provider token cannot be decrypted".to_string())?;
    let base = provider.base_url.trim_end_matches('/');
    let url = format!("{base}{}", definition.path);

    // 1. 使用供应商令牌访问固定 GET 接口，不透出原始错误和响应。
    let response = state
        .http
        .get(url)
        .bearer_auth(token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| "provider quota request failed".to_string())?;
    if !response.status().is_success() {
        return Err("provider quota upstream failed".into());
    }
    let body: Value = response
        .json()
        .await
        .map_err(|_| "provider quota response is invalid".to_string())?;

    // 2. 按配置读取可空数值；缺失或非法数值视为刷新失败。
    let quota_5h = read_quota(&body, &definition.field_5h)?;
    let quota_weekly = read_quota(&body, &definition.field_weekly)?;
    let quota_monthly = match &definition.field_monthly {
        Some(field) => read_quota(&body, field)?,
        None => provider.quota_monthly_remaining,
    };

    // 3. 全部字段使用同一刷新时间，便于路由判断当前窗口。
    let now = Utc::now().naive_utc();
    let mut active: b_providers::ActiveModel = provider.clone().into();
    active.quota_5h_remaining = Set(quota_5h);
    active.quota_5h_refreshed_at = Set(Some(now));
    active.quota_weekly_remaining = Set(quota_weekly);
    active.quota_weekly_refreshed_at = Set(Some(now));
    active.quota_monthly_remaining = Set(quota_monthly);
    active.quota_monthly_refreshed_at = if quota_monthly.is_none() {
        Set(None)
    } else {
        Set(Some(now))
    };
    active
        .update(&state.conn)
        .await
        .map_err(|_| "provider quota cannot be saved".to_string())
}

fn read_quota(body: &Value, field: &str) -> Result<Option<i64>, String> {
    let pointer = if field.starts_with('/') {
        field.to_string()
    } else {
        format!("/{field}")
    };
    match body.pointer(&pointer) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| "provider quota value is invalid".to_string()),
    }
}
