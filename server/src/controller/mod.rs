pub mod admin;
pub mod auth;
pub mod frontend;
pub mod gateway;
pub mod log;
pub mod mcp;
pub mod permission;
pub mod project;
pub mod provider_usage;
pub mod sandbox;
pub mod script;
pub mod system;
pub mod token;
pub mod user;
pub mod variable;

use actix_web::HttpResponse;
use serde_json::json;

/// 健康检查。
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "status": "ok" }))
}
