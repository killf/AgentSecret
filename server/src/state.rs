use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::AtomicU64};
use tokio::sync::Semaphore;

use crate::config::Config;
use crate::utils::jwt::TokenManager;

type McpRateLimits = HashMap<(u64, &'static str), Vec<std::time::Instant>>;

pub struct AppState {
    pub conn: DatabaseConnection,
    pub http: reqwest::Client,
    pub docker: bollard::Docker,
    pub config: Config,
    pub tokens: TokenManager,
    pub mcp_semaphore: Arc<Semaphore>,
    pub mcp_rate_limits: Arc<Mutex<McpRateLimits>>,
    pub provider_route_counter: Arc<AtomicU64>,
}

impl AppState {
    /// MCP 限速：列表 / 查询 30 次每分钟，执行 10 次每分钟。
    pub fn acquire_mcp_rate_limit(&self, user_id: u64, execution: bool) -> bool {
        let (category, limit) = if execution {
            ("execution", 10)
        } else {
            ("query", 30)
        };
        let Ok(mut limits) = self.mcp_rate_limits.lock() else {
            return false;
        };
        let entries = limits.entry((user_id, category)).or_default();
        let now = std::time::Instant::now();
        entries.retain(|instant| now.duration_since(*instant) < std::time::Duration::from_secs(60));
        if entries.len() >= limit {
            return false;
        }
        entries.push(now);

        true
    }
}
