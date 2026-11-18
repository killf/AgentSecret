use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// 单个预定义凭据动作的部署侧定义；目标、认证位置和字段白名单都固定在配置中。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialActionDefinition {
    pub method: String,
    pub url: String,
    pub credential_key: String,
    pub credential: CredentialLocation,
    pub parameters: BTreeMap<String, CredentialParameter>,
    pub response_fields: Vec<String>,
}

/// 凭据写入固定请求头；scheme 存在时生成 “scheme secret”。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialLocation {
    pub header: String,
    pub scheme: Option<String>,
}

/// 供应商额度查询接口映射；字段名使用 JSON Pointer 风格路径。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderQuotaDefinition {
    pub path: String,
    pub field_5h: String,
    pub field_weekly: String,
    pub field_monthly: Option<String>,
}

/// 允许脚本传入的业务参数及其位置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialParameter {
    pub r#type: CredentialParameterType,
    pub location: CredentialParameterLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialParameterType {
    String,
    Integer,
    Boolean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialParameterLocation {
    Query,
    Body,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub master_key: String,
    pub addr: String,
    pub token_ttl_days: i64,
    pub log_format: LogFormat,
    pub credential_actions: HashMap<String, CredentialActionDefinition>,
    pub provider_quotas: HashMap<String, ProviderQuotaDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

impl Config {
    /// 从环境变量加载配置；命令行连接串优先。
    pub fn from_env(database_url: Option<String>) -> Result<Self, AppError> {
        let database_url = required(database_url, "DATABASE_URL", "--database-url")?;
        let jwt_secret = required(None, "AGENTSECRET_JWT_SECRET", "")?;
        let master_key = required(None, "AGENTSECRET_MASTER_KEY", "")?;

        if master_key.len() != 32 || !master_key.is_ascii() {
            return Err(AppError::InvalidArgument(
                "AGENTSECRET_MASTER_KEY must contain exactly 32 ASCII bytes".into(),
            ));
        }

        let token_ttl_days = optional("AGENTSECRET_TOKEN_TTL_DAYS", "7")?
            .parse::<i64>()
            .map_err(|_| {
                AppError::InvalidArgument("AGENTSECRET_TOKEN_TTL_DAYS must be an integer".into())
            })?;
        if token_ttl_days <= 0 {
            return Err(AppError::InvalidArgument(
                "AGENTSECRET_TOKEN_TTL_DAYS must be greater than zero".into(),
            ));
        }

        let log_format = match optional("AGENTSECRET_LOG", "json")?.as_str() {
            "json" => LogFormat::Json,
            "pretty" => LogFormat::Pretty,
            value => {
                return Err(AppError::InvalidArgument(format!(
                    "unsupported AGENTSECRET_LOG: {value}"
                )));
            }
        };
        let credential_actions =
            parse_credential_actions(&optional("AGENTSECRET_CREDENTIAL_ACTIONS", "")?)?;
        let provider_quotas = parse_provider_quotas(&optional("AGENTSECRET_PROVIDER_QUOTAS", "")?)?;

        Ok(Self {
            database_url,
            jwt_secret,
            master_key,
            addr: optional("AGENTSECRET_ADDR", "0.0.0.0:8080")?,
            token_ttl_days,
            log_format,
            credential_actions,
            provider_quotas,
        })
    }
}

/// 解析并校验动作目录；空配置表示第一版没有登记任何凭据动作。
pub fn parse_credential_actions(
    raw: &str,
) -> Result<HashMap<String, CredentialActionDefinition>, AppError> {
    if raw.trim().is_empty() {
        return Ok(HashMap::new());
    }

    let actions: HashMap<String, CredentialActionDefinition> =
        serde_json::from_str(raw).map_err(|_| {
            AppError::InvalidArgument("AGENTSECRET_CREDENTIAL_ACTIONS must be a JSON object".into())
        })?;
    for (id, definition) in &actions {
        validate_action_id(id)?;
        validate_definition(definition)?;
    }

    Ok(actions)
}

/// 解析供应商类型到额度接口的映射；空配置表示未启用额度刷新。
pub fn parse_provider_quotas(
    raw: &str,
) -> Result<HashMap<String, ProviderQuotaDefinition>, AppError> {
    if raw.trim().is_empty() {
        return Ok(HashMap::new());
    }

    let quotas: HashMap<String, ProviderQuotaDefinition> =
        serde_json::from_str(raw).map_err(|_| {
            AppError::InvalidArgument("AGENTSECRET_PROVIDER_QUOTAS must be a JSON object".into())
        })?;
    for (provider_type, definition) in &quotas {
        validate_action_id(provider_type)?;
        if !definition.path.starts_with('/') || definition.path.len() > 512 {
            return Err(AppError::InvalidArgument(
                "provider quota path must start with / and contain at most 512 characters".into(),
            ));
        }

        for field in [
            Some(definition.field_5h.as_str()),
            Some(definition.field_weekly.as_str()),
            definition.field_monthly.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if field.is_empty() || field.len() > 256 {
                return Err(AppError::InvalidArgument(
                    "provider quota fields must contain 1 to 256 characters".into(),
                ));
            }
        }
    }

    Ok(quotas)
}

fn validate_definition(definition: &CredentialActionDefinition) -> Result<(), AppError> {
    if !matches!(definition.method.as_str(), "GET" | "POST") {
        return Err(AppError::InvalidArgument(
            "credential action method must be GET or POST".into(),
        ));
    }
    if !definition.url.starts_with("http://") && !definition.url.starts_with("https://") {
        return Err(AppError::InvalidArgument(
            "credential action url must use http or https".into(),
        ));
    }
    validate_action_id(&definition.credential_key)?;
    validate_header_name(&definition.credential.header)?;
    if let Some(scheme) = &definition.credential.scheme
        && (scheme.is_empty() || scheme.len() > 32)
    {
        return Err(AppError::InvalidArgument(
            "credential scheme must contain 1 to 32 characters".into(),
        ));
    }

    let body_allowed = definition.method == "POST";
    for (name, parameter) in &definition.parameters {
        validate_action_id(name)?;
        if parameter.location == CredentialParameterLocation::Body && !body_allowed {
            return Err(AppError::InvalidArgument(
                "GET credential actions only support query parameters".into(),
            ));
        }
    }
    if definition.response_fields.iter().any(String::is_empty) {
        return Err(AppError::InvalidArgument(
            "credential response fields cannot be empty".into(),
        ));
    }

    Ok(())
}

fn validate_action_id(value: &str) -> Result<(), AppError> {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "credential action identifiers must contain 1 to 128 safe characters".into(),
        ))
    }
}

fn validate_header_name(name: &str) -> Result<(), AppError> {
    let valid = !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-');
    let forbidden = name.eq_ignore_ascii_case("host")
        || name.eq_ignore_ascii_case("content-length")
        || name.eq_ignore_ascii_case("connection");
    if valid && !forbidden {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "credential header is invalid or forbidden".into(),
        ))
    }
}

fn required(value: Option<String>, name: &str, argument: &str) -> Result<String, AppError> {
    value
        .or_else(|| std::env::var(name).ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            let source = if argument.is_empty() {
                name.to_string()
            } else {
                format!("{name} or {argument}")
            };
            AppError::InvalidArgument(format!("missing required configuration: {source}"))
        })
}

fn optional(name: &str, default: &str) -> Result<String, AppError> {
    Ok(std::env::var(name).unwrap_or_else(|_| default.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_rejects_credential_action_catalog() {
        let actions = parse_credential_actions(
            r#"{
                "release.status": {
                    "method": "GET",
                    "url": "https://release.example.test/status",
                    "credential_key": "release_token",
                    "credential": {"header": "Authorization", "scheme": "Bearer"},
                    "parameters": {"release": {"type": "string", "location": "query"}},
                    "response_fields": ["status", "count"]
                }
            }"#,
        )
        .unwrap();
        assert!(actions.contains_key("release.status"));

        let quotas = parse_provider_quotas(
            r#"{"codex":{"path":"/v1/usage","field_5h":"five_hours","field_weekly":"weekly","field_monthly":null}}"#,
        )
        .unwrap();
        assert!(quotas.contains_key("codex"));

        let error = parse_credential_actions(
            r#"{"bad":{"method":"GET","url":"ftp://example","credential_key":"x","credential":{"header":"Host"},"parameters":{},"response_fields":[]}}"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("credential action"));
    }
}
