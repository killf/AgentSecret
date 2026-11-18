use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct IdRequest {
    pub id: u64,
}

#[derive(Debug, Serialize)]
pub struct CreatedResponse {
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRequest {
    pub id: u64,
    pub name: Option<String>,
    pub disabled_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Deserialize)]
pub struct ModelProviderListRequest {
    pub model_id: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ModelProviderItem {
    pub id: u64,
    pub model_id: u64,
    pub provider_id: u64,
    pub model_name: String,
    pub max_output_tokens: u64,
    pub max_context_tokens: u64,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelProviderRequest {
    pub model_id: u64,
    pub provider_id: u64,
    pub model_name: String,
    pub max_output_tokens: u64,
    pub max_context_tokens: u64,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelProviderRequest {
    pub id: u64,
    pub model_name: Option<String>,
    pub max_output_tokens: Option<u64>,
    pub max_context_tokens: Option<u64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub base_url: String,
    pub access_token: String,
    pub provider_type: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    pub id: u64,
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub access_token: Option<String>,
    pub disabled_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: u64,
    pub name: String,
    pub base_url: String,
    pub provider_type: String,
    pub quota_5h_remaining: Option<i64>,
    pub quota_5h_refreshed_at: Option<DateTime<Utc>>,
    pub quota_weekly_remaining: Option<i64>,
    pub quota_weekly_refreshed_at: Option<DateTime<Utc>>,
    pub quota_monthly_remaining: Option<i64>,
    pub quota_monthly_refreshed_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
}
