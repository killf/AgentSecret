pub mod codex;
pub mod glm;
pub mod minimax;

use serde_json::{Map, Value};

use crate::error::AppError;

/// 上游 HTTP 请求规格；适配器只输出目标、请求体和响应解析方式。
#[derive(Debug, Clone)]
pub struct UpstreamRequest {
    pub url: String,
    pub body: Value,
}

/// 构建 Responses 入口到厂商接口的请求。
pub fn build(
    base_url: &str,
    provider_type: &str,
    payload: &Value,
) -> Result<UpstreamRequest, AppError> {
    let base = base_url.trim_end_matches('/');
    match provider_type {
        codex::PROVIDER_TYPE => codex::build(base, payload),
        glm::PROVIDER_TYPE => glm::build(base, payload),
        minimax::PROVIDER_TYPE => minimax::build(base, payload),
        _ => Err(AppError::InvalidArgument(format!(
            "unsupported provider type: {provider_type}"
        ))),
    }
}

/// 将厂商响应规范化为 Responses JSON。
pub fn normalize(provider_type: &str, response: Value) -> Result<Value, AppError> {
    match provider_type {
        codex::PROVIDER_TYPE => codex::normalize(response),
        glm::PROVIDER_TYPE => glm::normalize(response),
        minimax::PROVIDER_TYPE => minimax::normalize(response),
        _ => Err(AppError::InvalidArgument(format!(
            "unsupported provider type: {provider_type}"
        ))),
    }
}

/// 提取 Responses input 中的文本；仅用于转换，不改变调用方 payload。
pub fn input_text(input: &Value) -> String {
    match input {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(input_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(object) => object
            .get("text")
            .or_else(|| object.get("content"))
            .map(input_text)
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// 复制可选数值字段并按厂商字段名写入。
pub fn copy_number(
    source: &Map<String, Value>,
    target: &mut Map<String, Value>,
    from: &str,
    to: &str,
) {
    if let Some(value) = source.get(from).filter(|value| value.is_number()) {
        target.insert(to.to_string(), value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_glm_chat_completions_request() {
        let request = build(
            "https://glm.example.com",
            "glm",
            &json!({
                "model": "glm-test",
                "input": "你好",
                "instructions": "简洁回答",
                "max_output_tokens": 128,
                "temperature": 0.2
            }),
        )
        .unwrap();

        assert_eq!(request.url, "https://glm.example.com/v1/chat/completions");
        assert_eq!(request.body["model"], "glm-test");
        assert_eq!(request.body["messages"][1]["content"], "你好");
        assert_eq!(request.body["max_tokens"], 128);
    }

    #[test]
    fn normalizes_glm_response_to_responses_format() {
        let response = json!({
            "id": "chatcmpl-1",
            "model": "glm-test",
            "choices": [{ "message": { "content": "好的" } }],
            "usage": { "prompt_tokens": 3, "completion_tokens": 4 }
        });
        let normalized = normalize("glm", response).unwrap();

        assert_eq!(normalized["output"][0]["text"], "好的");
        assert_eq!(normalized["usage"]["input_tokens"], 3);
        assert_eq!(normalized["usage"]["output_tokens"], 4);
    }

    #[test]
    fn extracts_nested_responses_input() {
        let text = input_text(&json!([{ "text": "第一段" }, { "content": "第二段" }]));

        assert_eq!(text, "第一段\n第二段");
    }
}
