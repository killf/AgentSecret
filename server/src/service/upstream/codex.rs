use super::UpstreamRequest;
use crate::error::AppError;
use serde_json::Value;

pub const PROVIDER_TYPE: &str = "codex";

/// Codex 使用 Responses 兼容接口，直接透传并改写模型名。
pub fn build(base: &str, payload: &Value) -> Result<UpstreamRequest, AppError> {
    Ok(UpstreamRequest {
        url: format!("{base}/v1/responses"),
        body: payload.clone(),
    })
}

/// Responses 响应无需转换。
pub fn normalize(response: Value) -> Result<Value, AppError> {
    Ok(response)
}
