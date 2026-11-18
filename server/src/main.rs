use actix_web::{App, HttpServer, web};
use clap::Parser;
use sea_orm::Database;
use sqlx::mysql::MySqlPoolOptions;
use tracing_actix_web::TracingLogger;

mod args;
mod config;
mod controller;
mod entity;
mod error;
mod extractor;
mod middleware;
mod router;
#[cfg(all(test, feature = "integration"))]
mod router_test;
mod schema;
mod service;
mod state;
mod utils;

use args::{Args, Command};
use config::{Config, LogFormat};
use state::AppState;
use utils::jwt::TokenManager;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let config = match Config::from_env(args.database_url.clone()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("configuration error: {error}");
            std::process::exit(1);
        }
    };
    init_tracing(config.log_format);

    if let Some(Command::SeedAdmin { email, name }) = args.command {
        return seed_admin(&config, &email, &name).await;
    }

    // 1. 使用独立连接执行唯一 init 迁移。
    let migration_conn = match MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database_url)
        .await
    {
        Ok(conn) => conn,
        Err(error) => {
            tracing::error!(error = %error, "migration connection failed");
            std::process::exit(1);
        }
    };
    if let Err(error) = MIGRATOR.run(&migration_conn).await {
        tracing::error!(error = %error, "database migration failed");
        std::process::exit(1);
    }
    migration_conn.close().await;

    // 2. 建立 SeaORM 业务连接并注入应用状态。
    let conn = match Database::connect(&config.database_url).await {
        Ok(conn) => conn,
        Err(error) => {
            tracing::error!(error = %error, "database connection failed");
            std::process::exit(1);
        }
    };
    let addr = config.addr.clone();
    let state = web::Data::new(AppState {
        http: reqwest::Client::new(),
        docker: match bollard::Docker::connect_with_socket_defaults() {
            Ok(docker) => docker,
            Err(error) => {
                eprintln!("Docker connection failed: {error}");
                std::process::exit(1);
            }
        },
        conn,
        tokens: TokenManager::new(&config.jwt_secret),
        mcp_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(4)),
        mcp_rate_limits: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
        provider_route_counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        config,
    });
    // 3. 清理上次服务中断遗留的一次性沙盒，再开放新请求。
    match crate::service::execution::cleanup_interrupted_containers(&state).await {
        Ok(count) => {
            tracing::info!(count, "interrupted sandbox containers cleaned");
        }
        Err(error) => {
            tracing::error!(error = %error, "interrupted sandbox cleanup failed");
            std::process::exit(1);
        }
    }

    // 4. 启动 HTTP 服务。
    tracing::info!(%addr, "agentsecret server listening");
    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(actix_web::middleware::from_fn(middleware::request_id))
            .app_data(state.clone())
            .configure(router::configure)
    })
    .bind(addr)?
    .run()
    .await
}

fn init_tracing(format: LogFormat) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    match format {
        LogFormat::Json => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init(),
        LogFormat::Pretty => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }
}

async fn seed_admin(config: &Config, email: &str, name: &str) -> std::io::Result<()> {
    let password = std::env::var("AGENTSECRET_ADMIN_PASSWORD_SHA1").unwrap_or_default();
    if password.len() != 40 || !password.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        eprintln!("AGENTSECRET_ADMIN_PASSWORD_SHA1 must be a 40-character SHA-1 value");
        std::process::exit(1);
    }

    let migration_conn = match MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&config.database_url)
        .await
    {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!("migration connection failed: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = MIGRATOR.run(&migration_conn).await {
        eprintln!("database migration failed: {error}");
        std::process::exit(1);
    }
    migration_conn.close().await;

    let conn = match Database::connect(&config.database_url).await {
        Ok(conn) => conn,
        Err(error) => {
            eprintln!("database connection failed: {error}");
            std::process::exit(1);
        }
    };
    match entity::b_users::create_initial_admin(&conn, email, name, &password).await {
        Ok(user) => {
            println!("created initial admin {} (id={})", user.email, user.id);
            Ok(())
        }
        Err(error) => {
            eprintln!("seed admin failed: {error}");
            std::process::exit(1);
        }
    }
}
