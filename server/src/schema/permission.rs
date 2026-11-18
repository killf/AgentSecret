use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionType {
    Projects,
    Environments,
    Models,
    Providers,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenLimits {
    pub daily_token_limit: Option<i64>,
    pub weekly_token_limit: Option<i64>,
    pub monthly_token_limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionListRequest {
    #[serde(rename = "type")]
    pub kind: PermissionType,
    pub user_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionUpsertRequest {
    #[serde(rename = "type")]
    pub kind: PermissionType,
    pub user_id: u64,
    pub target_id: Option<u64>,
    pub limits: Option<TokenLimits>,
}

#[derive(Debug, Deserialize)]
pub struct PermissionDeleteRequest {
    #[serde(rename = "type")]
    pub kind: PermissionType,
    pub id: u64,
}

#[derive(Debug, Serialize)]
pub struct PermissionResponse {
    pub id: u64,
    pub user_id: u64,
    pub target_id: Option<u64>,
    pub limits: Option<TokenLimits>,
}
