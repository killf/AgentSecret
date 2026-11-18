use serde_json::{Map, Value, json};

use super::{UpstreamRequest, copy_number, input_text};
use crate::error::AppError;

pub const PROVIDER_TYPE: &str = "glm";

/// GLM 使用 OpenAI 兼容 Chat Completions 接口。
pub fn build(base: &str, payload: &Value) -> Result<UpstreamRequest, AppError> {
    let object = payload
        .as_object()
        .ok_or_else(|| AppError::internal("invalid upstream payload"))?;
    let input = input_text(payload.get("input").unwrap_or(&Value::Null));
    let mut messages = Vec::new();
    if let Some(instructions) = payload.get("instructions").and_then(Value::as_str)
        && !instructions.is_empty()
    {
        messages.push(json!({ "role": "system", "content": instructions }));
    }
    messages.push(json!({ "role": "user", "content": input }));

    let mut body = Map::new();
    body.insert(
        "model".into(),
        payload.get("model").cloned().unwrap_or_default(),
    );
    body.insert("messages".into(), Value::Array(messages));
    copy_number(object, &mut body, "max_output_tokens", "max_tokens");
    if let Some(temperature) = payload.get("temperature").filter(|value| value.is_number()) {
        body.insert("temperature".into(), temperature.clone());
    }
    if payload.get("stream").and_then(Value::as_bool) == Some(true) {
        body.insert("stream".into(), Value::Bool(true));
    }

    Ok(UpstreamRequest {
        url: format!("{base}/v1/chat/completions"),
        body: Value::Object(body),
    })
}

/// Chat Completions 响应转换为 Responses 结构。
pub fn normalize(response: Value) -> Result<Value, AppError> {
    let text = response
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let usage = response.get("usage").cloned().unwrap_or_else(|| json!({}));
    Ok(json!({
        "id": response.get("id").cloned().unwrap_or(Value::Null),
        "model": response.get("model").cloned().unwrap_or(Value::Null),
        "output": [{ "type": "output_text", "text": text }],
        "usage": {
            "input_tokens": usage.get("prompt_tokens").cloned().unwrap_or(json!(0)),
            "output_tokens": usage.get("completion_tokens").cloned().unwrap_or(json!(0)),
            "cache_tokens": 0
        }
    }))
}
