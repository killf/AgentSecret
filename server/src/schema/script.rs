use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct ScriptListRequest {
    pub project_id: u64,
}

#[derive(Debug, Serialize)]
pub struct ScriptItem {
    pub id: u64,
    pub project_id: u64,
    pub environment_id: Option<u64>,
    pub sandbox_id: Option<u64>,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub review_status: String,
    pub revision: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ScriptDetail {
    #[serde(flatten)]
    pub item: ScriptItem,
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_actions: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateScriptRequest {
    pub project_id: u64,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub content: String,
    pub description: Option<String>,
    pub environment_id: Option<u64>,
    pub sandbox_id: Option<u64>,
    pub params: Option<Value>,
    pub credential_actions: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScriptRequest {
    pub id: u64,
    pub revision: u64,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub content: Option<String>,
    pub description: Option<Option<String>>,
    pub environment_id: Option<Option<u64>>,
    pub sandbox_id: Option<Option<u64>>,
    pub params: Option<Value>,
    pub credential_actions: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewScriptRequest {
    pub id: u64,
    pub revision: u64,
    pub decision: String,
}

#[derive(Debug, Serialize)]
pub struct ScriptVersionResponse {
    pub id: u64,
    pub revision: u64,
}
