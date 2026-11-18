use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct VariableListRequest {
    pub project_id: u64,
    pub environment_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateVariableRequest {
    pub project_id: u64,
    pub environment_id: u64,
    pub key: String,
    pub source_env: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVariableRequest {
    pub id: u64,
    pub key: Option<String>,
    pub source_env: Option<String>,
    pub description: Option<Option<String>>,
    pub disabled_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Serialize)]
pub struct VariableResponse {
    pub id: u64,
    pub key: String,
    pub source_env: String,
    pub description: Option<String>,
    pub disabled_at: Option<DateTime<Utc>>,
}
