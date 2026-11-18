use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SandboxIdRequest {
    pub sandbox_id: u64,
}

#[derive(Debug, Serialize)]
pub struct SandboxResponse {
    pub id: u64,
    pub project_id: u64,
    pub name: String,
    pub image: String,
    pub max_instances: u32,
}

#[derive(Debug, Serialize)]
pub struct SandboxInstanceResponse {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub destroyed_at: Option<DateTime<Utc>>,
}
