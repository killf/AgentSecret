use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// 返回默认敏感信息规则；后续可由系统设置热更新替换。
pub fn patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            r"\bAKIA[0-9A-Z]{16}\b",
            r"\bSK-[A-Za-z0-9_-]{16,}\b",
            r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
            r"\b(?:mysql|postgres(?:ql)?|mongodb(?:\+srv)?)://[^\s]+",
            r"\bBearer\s+[A-Za-z0-9._~+/=-]{16,}",
            r"\b(?:password|passwd|secret|token)\s*[:=]\s*[^\s,;}]{6,}",
            r"\b1[3-9]\d{9}\b",
            r"\b\d{17}[0-9Xx]\b",
            r"\b\d{15,19}\b",
        ]
        .iter()
        .map(|pattern| Regex::new(pattern).expect("valid sensitive regex"))
        .collect()
    })
}

/// 遮罩一段文本，返回遮罩结果与是否命中。
pub fn mask_text(text: &str) -> (String, bool) {
    mask_text_with_secret(text, "")
}

/// 在通用规则之外同时遮罩本次执行使用的密钥明文和常见 Base64 编码。
pub fn mask_text_with_secret(text: &str, secret: &str) -> (String, bool) {
    let mut masked = text.to_string();
    let mut contains_sensitive = false;
    for pattern in patterns() {
        if pattern.is_match(&masked) {
            contains_sensitive = true;
        }
        masked = pattern.replace_all(&masked, "***").into_owned();
    }
    if !secret.is_empty() {
        for variant in [
            secret.to_string(),
            STANDARD.encode(secret),
            URL_SAFE_NO_PAD.encode(secret),
        ] {
            if masked.contains(&variant) {
                contains_sensitive = true;
                masked = masked.replace(&variant, "***");
            }
        }
    }

    (masked, contains_sensitive)
}

/// 递归遮罩 JSON 字符串字段。
pub fn mask_json(value: &Value) -> (Value, bool) {
    mask_json_with_secret(value, "")
}

/// 递归遮罩 JSON 值，并额外处理本次凭据动作使用的密钥。
pub fn mask_json_with_secret(value: &Value, secret: &str) -> (Value, bool) {
    match value {
        Value::String(text) => {
            let (masked, hit) = mask_text_with_secret(text, secret);
            (Value::String(masked), hit)
        }
        Value::Array(items) => {
            let mut hit = false;
            let items = items
                .iter()
                .map(|item| {
                    let (item, item_hit) = mask_json_with_secret(item, secret);
                    hit |= item_hit;
                    item
                })
                .collect();

            (Value::Array(items), hit)
        }
        Value::Object(object) => mask_map(object, secret),
        _ => (value.clone(), false),
    }
}

fn mask_map(object: &Map<String, Value>, secret: &str) -> (Value, bool) {
    let mut hit = false;
    let mut result = Map::new();
    for (key, value) in object {
        let (value, value_hit) = mask_json_with_secret(value, secret);
        hit |= value_hit;
        result.insert(key.clone(), value);
    }

    (Value::Object(result), hit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn masks_credentials_and_phone() {
        let (value, hit) = mask_json(&json!({
            "token": "token=abcdef123456",
            "phone": "13812345678",
            "safe": "hello"
        }));

        assert!(hit);
        assert_eq!(value["safe"], json!("hello"));
        assert_ne!(value["token"], json!("token=abcdef123456"));
        assert_ne!(value["phone"], json!("13812345678"));
    }

    #[test]
    fn masks_known_secret_and_base64_variants() {
        let secret = "fake-secret-123456";
        let encoded = STANDARD.encode(secret);
        let text = format!("{secret} {encoded}");

        let (masked, hit) = mask_text_with_secret(&text, secret);
        assert!(hit);
        assert!(!masked.contains(secret));
        assert!(!masked.contains(&encoded));
    }
}
