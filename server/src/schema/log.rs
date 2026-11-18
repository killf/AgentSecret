use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct RequestLogListRequest {
    pub user_id: Option<u64>,
    pub model_id: Option<u64>,
    pub provider_id: Option<u64>,
    pub status: Option<String>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct RequestLogItem {
    pub id: u64,
    pub created_at: DateTime<Utc>,
    pub request_status: String,
    pub failure_reason: Option<String>,
    pub user_id: u64,
    pub user_email: String,
    pub model_id: u64,
    pub model_name: String,
    pub provider_id: u64,
    pub provider_name: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    pub ttft_ms: u64,
    pub duration_ms: u64,
    pub contains_sensitive: bool,
}

#[derive(Debug, Serialize)]
pub struct RequestLogDetail {
    #[serde(flatten)]
    pub item: RequestLogItem,
    pub request_detail: Option<Value>,
    pub response_detail: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionLogListRequest {
    pub user_id: Option<u64>,
    pub project_id: Option<u64>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionItem {
    pub id: u64,
    pub script_id: u64,
    pub script_revision: u64,
    pub project_id: u64,
    pub environment_id: Option<u64>,
    pub r#type: String,
    pub status: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error_code: Option<String>,
    pub sandbox_id: u64,
    pub user_id: u64,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}
