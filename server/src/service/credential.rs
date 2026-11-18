//! 预定义凭据动作：只暴露部署侧登记的固定 HTTP 操作，不把密钥传给脚本。

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sha1::{Digest, Sha1};
use std::time::Duration;

use crate::config::{
    Config, CredentialActionDefinition, CredentialParameterLocation, CredentialParameterType,
};
use crate::entity::{b_scripts, b_variables};
use crate::error::AppError;
use crate::service::sensitive::mask_json_with_secret;

/// 脚本提交时允许只引用动作与绑定；定义指纹由服务端写入。
#[derive(Debug, Clone, Deserialize)]
struct CredentialActionInput {
    action_id: String,
    binding_id: u64,
}

/// 已固定的审核配置；配置指纹变化后不能继续执行旧批准。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CredentialActionStored {
    action_id: String,
    binding_id: u64,
    definition_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CredentialActionsDocument {
    version: u8,
    actions: Vec<CredentialActionStored>,
}

/// 执行前解析出的动作；secret 只留在服务进程内存，不写日志或容器环境。
#[derive(Debug)]
pub struct PreparedCredentialAction {
    pub action_id: String,
    definition: CredentialActionDefinition,
    secret: String,
}

/// 校验并固定脚本的凭据动作引用；返回可审核、可持久化的规范化 JSON。
pub async fn normalize_credential_actions(
    conn: &impl ConnectionTrait,
    config: &Config,
    project_id: u64,
    environment_id: Option<u64>,
    value: Option<&Value>,
) -> Result<Option<Value>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(raw_actions) = value.get("actions").and_then(Value::as_array) else {
        return Err(AppError::InvalidArgument(
            "credential_actions.actions must be an array".into(),
        ));
    };

    let mut stored = Vec::with_capacity(raw_actions.len());
    let mut seen = std::collections::HashSet::new();
    for raw in raw_actions {
        let input: CredentialActionInput = serde_json::from_value(raw.clone())
            .map_err(|_| AppError::InvalidArgument("invalid credential action reference".into()))?;
        if !seen.insert(input.action_id.clone()) {
            return Err(AppError::InvalidArgument(
                "credential action references must be unique".into(),
            ));
        }

        // 1. 先核对部署侧动作与绑定归属，再读取但不保留密钥。
        let (definition, variable) = load_definition_and_variable(
            conn,
            config,
            project_id,
            environment_id,
            &input.action_id,
            input.binding_id,
        )
        .await?;
        let secret = read_secret(&variable.source_env)?;
        validate_secret_header(&definition, &secret)?;

        // 2. 写入配置指纹，审核时固定目的地和字段白名单。
        stored.push(CredentialActionStored {
            action_id: input.action_id,
            binding_id: input.binding_id,
            definition_hash: definition_hash(&definition)?,
        });
    }

    Ok(Some(json!({ "version": 1, "actions": stored })))
}

/// 审核通过前刷新配置指纹；调用方仍需按版本条件更新脚本。
pub async fn refresh_credential_actions(
    conn: &impl ConnectionTrait,
    config: &Config,
    script: &b_scripts::Model,
) -> Result<Option<Value>, AppError> {
    normalize_credential_actions(
        conn,
        config,
        script.project_id,
        script.environment_id,
        script.credential_actions.as_ref(),
    )
    .await
}

/// 启动容器前重新核对绑定、禁用状态、配置指纹和服务端密钥。
pub async fn prepare_credential_actions(
    conn: &impl ConnectionTrait,
    config: &Config,
    script: &b_scripts::Model,
) -> Result<Vec<PreparedCredentialAction>, AppError> {
    let Some(value) = script.credential_actions.as_ref() else {
        return Ok(Vec::new());
    };
    let document: CredentialActionsDocument = serde_json::from_value(value.clone())
        .map_err(|_| AppError::InvalidArgument("invalid credential action configuration".into()))?;
    if document.version != 1 {
        return Err(AppError::InvalidArgument(
            "unsupported credential action version".into(),
        ));
    }

    let mut prepared = Vec::with_capacity(document.actions.len());
    for action in document.actions {
        let (definition, variable) = load_definition_and_variable(
            conn,
            config,
            script.project_id,
            script.environment_id,
            &action.action_id,
            action.binding_id,
        )
        .await?;

        // 1. 目的地策略变化必须重新审核，不能沿用旧批准。
        if definition_hash(&definition)? != action.definition_hash {
            return Err(AppError::InvalidArgument(
                "credential action definition changed; review is required".into(),
            ));
        }

        // 2. 密钥只在本次执行内存中出现，容器不会收到该环境变量。
        let secret = read_secret(&variable.source_env)?;
        validate_secret_header(&definition, &secret)?;
        prepared.push(PreparedCredentialAction {
            action_id: action.action_id,
            definition,
            secret,
        });
    }

    Ok(prepared)
}

/// 使用固定目的地、参数白名单和响应字段白名单调用上游。
pub async fn execute_credential_action(
    client: &reqwest::Client,
    action: &PreparedCredentialAction,
    params: &Value,
) -> Result<Value, String> {
    let object = params.as_object().ok_or("invalid action parameters")?;
    if object.len() != action.definition.parameters.len() {
        return Err("action parameter set does not match".into());
    }

    // 1. 逐项校验类型并按配置拆分 query/body。
    let mut query = Vec::new();
    let mut body = Map::new();
    for (name, definition) in &action.definition.parameters {
        let value = object.get(name).ok_or("missing action parameter")?;
        match definition.r#type {
            CredentialParameterType::String => {
                let text = value.as_str().ok_or("invalid string parameter")?;
                push_parameter(
                    &mut query,
                    &mut body,
                    definition.location,
                    name,
                    json!(text),
                );
            }
            CredentialParameterType::Integer => {
                let number = value.as_i64().ok_or("invalid integer parameter")?;
                push_parameter(
                    &mut query,
                    &mut body,
                    definition.location,
                    name,
                    json!(number),
                );
            }
            CredentialParameterType::Boolean => {
                let flag = value.as_bool().ok_or("invalid boolean parameter")?;
                push_parameter(
                    &mut query,
                    &mut body,
                    definition.location,
                    name,
                    json!(flag),
                );
            }
        }
    }

    // 2. 固定 URL 与认证头，脚本不能提供任意目的地或请求头。
    let mut url = reqwest::Url::parse(&action.definition.url).map_err(|_| "invalid action url")?;
    for (key, value) in query {
        url.query_pairs_mut().append_pair(&key, &value);
    }
    let header_name =
        reqwest::header::HeaderName::from_bytes(action.definition.credential.header.as_bytes())
            .map_err(|_| "invalid credential header")?;
    let credential = match &action.definition.credential.scheme {
        Some(scheme) => format!("{scheme} {}", action.secret),
        None => action.secret.clone(),
    };
    let header_value = reqwest::header::HeaderValue::from_str(&credential)
        .map_err(|_| "invalid credential value")?;
    let method = reqwest::Method::from_bytes(action.definition.method.as_bytes())
        .map_err(|_| "invalid action method")?;
    let mut request = client
        .request(method, url)
        .header(header_name, header_value);
    if action.definition.method == "POST" {
        request = request.json(&Value::Object(body));
    }

    // 3. 只接受 JSON 对象，并先按字段白名单裁剪再遮罩已知密钥。
    let response = tokio::time::timeout(Duration::from_secs(30), request.send())
        .await
        .map_err(|_| "action timeout")?
        .map_err(|_| "action request failed")?;
    if !response.status().is_success() {
        return Err("action upstream failed".into());
    }
    let response_body: Value = response
        .json()
        .await
        .map_err(|_| "invalid action response")?;
    let response_object = response_body.as_object().ok_or("invalid action response")?;
    let mut result = Map::new();
    for field in &action.definition.response_fields {
        if let Some(value) = response_object.get(field) {
            let (value, _) = mask_json_with_secret(value, &action.secret);
            result.insert(field.clone(), value);
        }
    }

    Ok(Value::Object(result))
}

async fn load_definition_and_variable(
    conn: &impl ConnectionTrait,
    config: &Config,
    project_id: u64,
    environment_id: Option<u64>,
    action_id: &str,
    binding_id: u64,
) -> Result<(CredentialActionDefinition, b_variables::Model), AppError> {
    let definition = config
        .credential_actions
        .get(action_id)
        .ok_or_else(|| AppError::NotFound("credential action is not registered".into()))?
        .clone();
    let environment_id = environment_id.ok_or_else(|| {
        AppError::InvalidArgument("credential actions require a script environment".into())
    })?;
    let variable = b_variables::Entity::find()
        .filter(b_variables::Column::Id.eq(binding_id))
        .filter(b_variables::Column::ProjectId.eq(project_id))
        .filter(b_variables::Column::EnvironmentId.eq(environment_id))
        .filter(b_variables::Column::Key.eq(&definition.credential_key))
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("credential binding does not match the action".into()))?;
    if variable.disabled_at.is_some() {
        return Err(AppError::InvalidArgument(
            "credential binding is disabled".into(),
        ));
    }

    Ok((definition, variable))
}

fn push_parameter(
    query: &mut Vec<(String, String)>,
    body: &mut Map<String, Value>,
    location: CredentialParameterLocation,
    name: &str,
    value: Value,
) {
    match location {
        CredentialParameterLocation::Query => {
            let text = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            query.push((name.to_string(), text));
        }
        CredentialParameterLocation::Body => {
            body.insert(name.to_string(), value);
        }
    }
}

fn read_secret(source_env: &str) -> Result<String, AppError> {
    std::env::var(source_env)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("credential source environment is missing".into()))
}

fn validate_secret_header(
    definition: &CredentialActionDefinition,
    secret: &str,
) -> Result<(), AppError> {
    let credential = match &definition.credential.scheme {
        Some(scheme) => format!("{scheme} {secret}"),
        None => secret.to_string(),
    };
    reqwest::header::HeaderValue::from_str(&credential)
        .map(|_| ())
        .map_err(|_| {
            AppError::InvalidArgument("credential cannot be used in the configured header".into())
        })
}

fn definition_hash(definition: &CredentialActionDefinition) -> Result<String, AppError> {
    let raw = serde_json::to_vec(definition).map_err(AppError::internal)?;
    let digest = Sha1::digest(raw);
    let mut text = String::with_capacity(digest.len() * 2);
    for byte in digest {
        text.push_str(&format!("{byte:02x}"));
    }

    Ok(text)
}
