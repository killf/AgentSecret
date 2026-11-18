use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListUsersRequest {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub id: u64,
    pub name: Option<String>,
    pub is_admin: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BanUserRequest {
    pub id: u64,
    pub permanent: Option<bool>,
    pub minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ResetUserPasswordRequest {
    pub id: u64,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct AdminUserInfo {
    pub id: u64,
    pub email: String,
    pub name: String,
    pub is_admin: bool,
    pub banned_at: Option<DateTime<Utc>>,
    pub unban_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserPageResponse {
    pub total: u64,
    pub items: Vec<AdminUserInfo>,
}
