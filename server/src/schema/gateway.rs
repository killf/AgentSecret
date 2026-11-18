use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct GatewayProvider {
    pub provider_id: u64,
    pub model_name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct GatewayModel {
    pub id: u64,
    pub name: String,
    pub max_output_tokens: u64,
    pub max_context_tokens: u64,
    pub providers: Vec<GatewayProvider>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponsesRequest {
    pub model: String,
    pub input: Value,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub max_output_tokens: Option<u64>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub provider: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
}

#[derive(Debug, Serialize)]
pub struct ResponsesResponse {
    pub id: String,
    pub object: &'static str,
    pub model: String,
    pub output: Value,
    pub usage: UsageResponse,
    pub provider: String,
    pub ttft_ms: u64,
    pub duration_ms: u64,
    pub contains_sensitive: bool,
}
