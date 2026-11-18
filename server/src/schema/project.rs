use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ProjectIdRequest {
    pub project_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub guidance: Option<String>,
    pub repos: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub id: u64,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub guidance: Option<Option<String>>,
    pub repos: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub guidance: Option<String>,
    pub repos: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub disabled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEnvironmentRequest {
    pub project_id: u64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEnvironmentRequest {
    pub id: u64,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct EnvironmentResponse {
    pub id: u64,
    pub project_id: u64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}
