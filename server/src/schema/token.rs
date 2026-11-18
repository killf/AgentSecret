use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct TokenIdRequest {
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct TokenInfo {
    pub id: u64,
    pub name: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TokenDetail {
    #[serde(flatten)]
    pub info: TokenInfo,
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ResetTokenResponse {
    pub token: String,
    pub expires_at: Option<DateTime<Utc>>,
}
