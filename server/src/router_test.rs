#![cfg(feature = "integration")]
#![allow(deprecated)] // 测试直接使用 Bollard 旧容器 API 与服务实现保持一致

use actix_web::{App, http::StatusCode, test, web};
use serde_json::{Value, json};

use crate::config::{Config, LogFormat, parse_credential_actions};
use crate::entity::b_users;
use crate::middleware;
use crate::router;
use crate::state::AppState;
use crate::utils::jwt::TokenManager;

fn config(database_url: String, credential_actions: &str, provider_quotas: &str) -> Config {
    Config {
        database_url,
        jwt_secret: "integration-secret".into(),
        master_key: "0123456789abcdef0123456789abcdef".into(),
        addr: "127.0.0.1:0".into(),
        token_ttl_days: 7,
        log_format: LogFormat::Json,
        credential_actions: parse_credential_actions(credential_actions).unwrap(),
        provider_quotas: crate::config::parse_provider_quotas(provider_quotas).unwrap(),
    }
}

async fn reset_database(database_url: &str) {
    use sqlx::Connection;

    let mut conn = sqlx::MySqlConnection::connect(database_url).await.unwrap();
    sqlx::query("SET FOREIGN_KEY_CHECKS = 0")
        .execute(&mut conn)
        .await
        .unwrap();
    for table in [
        "b_user_operation_logs",
        "b_request_logs",
        "b_executions",
        "b_scripts",
        "b_variables",
        "b_sandbox_instances",
        "b_sandboxes",
        "b_environments",
        "b_projects",
        "r_environment_permissions",
        "r_model_permissions",
        "r_model_providers",
        "r_project_permissions",
        "r_provider_permissions",
        "b_system_settings",
        "b_access_tokens",
        "b_users",
        "b_models",
        "b_providers",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!("TRUNCATE TABLE {table}")))
            .execute(&mut conn)
            .await
            .unwrap();
    }
    sqlx::query("SET FOREIGN_KEY_CHECKS = 1")
        .execute(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
}

#[actix_rt::test]
async fn api_mcp_and_sandbox_complete_integration_flow() {
    let database_url = std::env::var("AGENTSECRET_TEST_DATABASE_URL")
        .expect("AGENTSECRET_TEST_DATABASE_URL is required for integration tests");
    let conn = sea_orm::Database::connect(&database_url).await.unwrap();
    reset_database(&database_url).await;
    b_users::create_initial_admin(
        &conn,
        "admin@example.com",
        "Administrator",
        "0123456789abcdef0123456789abcdef01234567",
    )
    .await
    .unwrap();

    // 预先绑定假发布系统，动作配置中的目的地必须固定指向该端口。
    let quota_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let quota_port = quota_listener.local_addr().unwrap().port();
    let provider_quotas =
        r#"{"glm":{"path":"/quota","field_5h":"five_hours","field_weekly":"weekly","field_monthly":null}}"#
            .to_string();
    let action_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let action_port = action_listener.local_addr().unwrap().port();
    let credential_actions = format!(
        r#"{{
            "release.status": {{
                "method": "GET",
                "url": "http://127.0.0.1:{action_port}/status",
                "credential_key": "release_token",
                "credential": {{"header": "Authorization", "scheme": "Bearer"}},
                "parameters": {{"release": {{"type": "string", "location": "query"}}}},
                "response_fields": ["status", "count", "echo"]
            }}
        }}"#
    );
    let state = web::Data::new(AppState {
        http: reqwest::Client::new(),
        docker: bollard::Docker::connect_with_socket_defaults().unwrap(),
        conn: conn.clone(),
        tokens: TokenManager::new("integration-secret"),
        mcp_semaphore: std::sync::Arc::new(tokio::sync::Semaphore::new(4)),
        mcp_rate_limits: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
        provider_route_counter: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        config: config(database_url, &credential_actions, &provider_quotas),
    });
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // 同一个假服务先响应 GLM 聊天，再响应统一额度查询。
        for _ in 0..2 {
            let (mut socket, _) = quota_listener.accept().await.unwrap();
            let mut buffer = [0_u8; 4096];
            let size = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..size]).to_string();
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer glm-token")
            );
            let body = if request.starts_with("POST /v1/chat/completions ") {
                br#"{"id":"chatcmpl-glm","model":"upstream-glm","choices":[{"message":{"content":"glm response"}}],"usage":{"prompt_tokens":5,"completion_tokens":7}}"#.to_vec()
            } else {
                assert!(request.starts_with("GET /quota "));
                br#"{"five_hours":100,"weekly":200}"#.to_vec()
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.write_all(&body).await.unwrap();
        }
    });
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut socket, _) = action_listener.accept().await.unwrap();
        let mut buffer = [0_u8; 4096];
        let size = socket.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..size]).to_string();
        assert!(request.starts_with("GET /status?release=R-100 "));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer fake-release-token")
        );
        let body = br#"{"status":"running","count":2,"echo":"fake-release-token"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
    });
    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(middleware::request_id))
            .app_data(state.clone())
            .configure(router::configure),
    )
    .await;

    // 1. 登录并创建访问令牌。
    let login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({
            "email": "admin@example.com",
            "password": "0123456789abcdef0123456789abcdef01234567"
        }))
        .send_request(&app)
        .await;
    assert_eq!(login.status(), StatusCode::OK);
    let login_body: Value = test::read_body_json(login).await;
    assert_eq!(login_body["code"], 0);
    let bearer = login_body["data"]["token"].as_str().unwrap().to_string();

    let token = test::TestRequest::post()
        .uri("/api/v1/tokens/add")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "name": "integration" }))
        .send_request(&app)
        .await;
    let token_body: Value = test::read_body_json(token).await;
    let access_token = token_body["data"]["token"].as_str().unwrap().to_string();

    // 2. 创建项目、环境、审核人与脚本，并执行真实沙盒。
    let project = test::TestRequest::post()
        .uri("/api/v1/projects/add")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "name": "integration-project", "repos": ["https://github.com/example/integration"] }))
        .send_request(&app).await;
    let project_body: Value = test::read_body_json(project).await;
    assert_eq!(project_body["code"], 0);
    let project_id = project_body["data"]["id"].as_u64().unwrap();

    let environment = test::TestRequest::post()
        .uri("/api/v1/environments/add")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "project_id": project_id, "name": "dev" }))
        .send_request(&app)
        .await;
    let environment_body: Value = test::read_body_json(environment).await;
    assert_eq!(environment_body["code"], 0);
    let environment_id = environment_body["data"]["id"].as_u64().unwrap();

    let reviewer = test::TestRequest::post()
        .uri("/api/v1/admin/users/add")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "email": "reviewer@example.com", "name": "Reviewer", "password": "abcdef1234567890abcdef1234567890abcdef12" }))
        .send_request(&app).await;
    let reviewer_body: Value = test::read_body_json(reviewer).await;
    assert_eq!(reviewer_body["code"], 0);
    let reviewer_id = reviewer_body["data"]["id"].as_u64().unwrap();

    let promote = test::TestRequest::post()
        .uri("/api/v1/admin/users/update")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "id": reviewer_id, "is_admin": true }))
        .send_request(&app)
        .await;
    assert_eq!(promote.status(), StatusCode::OK);

    let reviewer_login = test::TestRequest::post()
        .uri("/api/v1/auth/login")
        .set_json(json!({ "email": "reviewer@example.com", "password": "abcdef1234567890abcdef1234567890abcdef12" }))
        .send_request(&app).await;
    let reviewer_login_body: Value = test::read_body_json(reviewer_login).await;
    let reviewer_bearer = reviewer_login_body["data"]["token"].as_str().unwrap();

    use sea_orm::{ActiveModelTrait, NotSet, Set};
    let sandbox = crate::entity::b_sandboxes::ActiveModel {
        id: NotSet,
        project_id: Set(project_id),
        name: Set("bash-sandbox".into()),
        description: Set(Some("integration sandbox".into())),
        image: Set("bash:5.3".into()),
        max_instances: Set(1),
        updated_by: Set(1),
        updated_at: Set(chrono::Utc::now().naive_utc()),
    };
    let sandbox = sandbox.insert(&state.conn).await.unwrap();

    let script = test::TestRequest::post()
        .uri("/api/v1/scripts/add")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({
            "project_id": project_id,
            "environment_id": environment_id,
            "sandbox_id": sandbox.id,
            "name": "hello",
            "type": "bash",
            "content": "echo integration-sandbox",
            "params": {"greeting": {"type": "string", "required": true}}
        }))
        .send_request(&app)
        .await;
    let script_body: Value = test::read_body_json(script).await;
    assert_eq!(script_body["code"], 0);
    let script_id = script_body["data"]["id"].as_u64().unwrap();
    let revision = script_body["data"]["revision"].as_u64().unwrap();

    let review = test::TestRequest::post()
        .uri("/api/v1/scripts/review")
        .insert_header(("Authorization", format!("Bearer {reviewer_bearer}")))
        .set_json(json!({ "id": script_id, "revision": revision, "decision": "approve" }))
        .send_request(&app)
        .await;
    let review_body: Value = test::read_body_json(review).await;
    assert_eq!(review_body["code"], 0);

    let execution = test::TestRequest::post()
        .uri("/api/v1/scripts/run")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "id": script_id, "params": {"greeting": "hello"} }))
        .send_request(&app)
        .await;
    let execution_body: Value = test::read_body_json(execution).await;
    assert_eq!(execution_body["code"], 0, "{execution_body}");

    // 2b. 使用假发布系统验证预定义凭据动作和已知密钥遮罩。
    let variable = test::TestRequest::post()
        .uri("/api/v1/variables/add")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({
            "project_id": project_id,
            "environment_id": environment_id,
            "key": "release_token",
            "source_env": "TEST_RELEASE_CREDENTIAL"
        }))
        .send_request(&app)
        .await;
    let variable_body: Value = test::read_body_json(variable).await;
    assert_eq!(variable_body["code"], 0, "{variable_body}");
    let binding_id = variable_body["data"]["id"].as_u64().unwrap();

    let credential_script = test::TestRequest::post()
        .uri("/api/v1/scripts/add")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({
            "project_id": project_id,
            "environment_id": environment_id,
            "sandbox_id": sandbox.id,
            "name": "release-status",
            "type": "bash",
            "content": "result=\"$(agentsecret_action release.status '{\"release\":\"R-100\"}')\"\necho \"$result\"",
            "credential_actions": {
                "actions": [{ "action_id": "release.status", "binding_id": binding_id }]
            }
        }))
        .send_request(&app)
        .await;
    let credential_script_body: Value = test::read_body_json(credential_script).await;
    assert_eq!(
        credential_script_body["code"], 0,
        "{credential_script_body}"
    );
    let credential_script_id = credential_script_body["data"]["id"].as_u64().unwrap();
    let credential_revision = credential_script_body["data"]["revision"].as_u64().unwrap();

    let credential_review = test::TestRequest::post()
        .uri("/api/v1/scripts/review")
        .insert_header(("Authorization", format!("Bearer {reviewer_bearer}")))
        .set_json(json!({
            "id": credential_script_id,
            "revision": credential_revision,
            "decision": "approve"
        }))
        .send_request(&app)
        .await;
    let credential_review_body: Value = test::read_body_json(credential_review).await;
    assert_eq!(
        credential_review_body["code"], 0,
        "{credential_review_body}"
    );

    let credential_execution = test::TestRequest::post()
        .uri("/api/v1/scripts/run")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "id": credential_script_id, "params": {} }))
        .send_request(&app)
        .await;
    let credential_execution_body: Value = test::read_body_json(credential_execution).await;
    assert_eq!(
        credential_execution_body["code"], 0,
        "{credential_execution_body}"
    );
    let credential_execution_id = credential_execution_body["data"]["execution_id"]
        .as_u64()
        .unwrap();
    use sea_orm::EntityTrait;
    let execution_row = crate::entity::b_executions::Entity::find_by_id(credential_execution_id)
        .one(&state.conn)
        .await
        .unwrap()
        .unwrap();
    let credential_stdout = execution_row.stdout.unwrap_or_default();
    assert!(
        credential_stdout.contains("\"status\":\"running\""),
        "{credential_stdout}"
    );
    assert!(
        credential_stdout.contains("\"count\":2"),
        "{credential_stdout}"
    );
    assert!(
        !credential_stdout.contains("fake-release-token"),
        "{credential_stdout}"
    );

    // 2c. 构造遗留容器，验证服务重启清理只处理本服务标记。
    let leftover_name = format!("agentsecret-exec-{}", crate::utils::crypto::random_id());
    let leftover_instance = crate::entity::b_sandbox_instances::ActiveModel {
        id: NotSet,
        sandbox_id: Set(sandbox.id),
        name: Set(leftover_name.clone()),
        status: Set("running".into()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        destroyed_at: Set(None),
    }
    .insert(&state.conn)
    .await
    .unwrap();
    let leftover_execution = crate::entity::b_executions::ActiveModel {
        id: NotSet,
        script_id: Set(credential_script_id),
        script_revision: Set(credential_revision),
        project_id: Set(project_id),
        environment_id: Set(Some(environment_id)),
        r#type: Set("bash".into()),
        status: Set("running".into()),
        stdout: Set(None),
        stderr: Set(None),
        error_code: Set(None),
        sandbox_id: Set(sandbox.id),
        sandbox_instance_id: Set(leftover_instance.id),
        user_id: Set(1),
        started_at: Set(Some(chrono::Utc::now().naive_utc())),
        finished_at: Set(None),
    }
    .insert(&state.conn)
    .await
    .unwrap();
    let mut leftover_labels = std::collections::HashMap::new();
    leftover_labels.insert(
        "agentsecret.service".to_string(),
        "sandbox-execution".to_string(),
    );
    leftover_labels.insert(
        "agentsecret.execution-id".to_string(),
        leftover_execution.id.to_string(),
    );
    let leftover_config = bollard::container::Config {
        image: Some("bash:5.3".into()),
        cmd: Some(vec!["sleep".into(), "60".into()]),
        labels: Some(leftover_labels),
        ..Default::default()
    };
    state
        .docker
        .create_container(
            Some(bollard::container::CreateContainerOptions {
                name: leftover_name.clone(),
                platform: None,
            }),
            leftover_config,
        )
        .await
        .unwrap();
    state
        .docker
        .start_container(
            &leftover_name,
            None::<bollard::container::StartContainerOptions<String>>,
        )
        .await
        .unwrap();
    let cleaned = crate::service::execution::cleanup_interrupted_containers(&state)
        .await
        .unwrap();
    assert_eq!(cleaned, 1);
    let cleaned_execution = crate::entity::b_executions::Entity::find_by_id(leftover_execution.id)
        .one(&state.conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cleaned_execution.status, "failed");
    assert_eq!(
        cleaned_execution.error_code.as_deref(),
        Some("service_restarted")
    );
    assert!(
        state
            .docker
            .inspect_container(
                &leftover_name,
                Some(bollard::query_parameters::InspectContainerOptionsBuilder::default().build())
            )
            .await
            .is_err()
    );

    // 3. MCP initialize 与 project_list。
    let initialize = test::TestRequest::post()
        .uri("/mcp")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .set_json(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }))
        .send_request(&app)
        .await;
    assert_eq!(initialize.status(), StatusCode::OK);

    let project_list = test::TestRequest::post()
        .uri("/mcp")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .set_json(json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "project_list", "arguments": {} }
        }))
        .send_request(&app)
        .await;
    let project_list_body: Value = test::read_body_json(project_list).await;
    assert_eq!(project_list_body["result"]["isError"], false);

    let mcp_run = test::TestRequest::post()
        .uri("/mcp")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .set_json(json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {
                "name": "script_run",
                "arguments": { "project": "integration-project", "environment": "dev", "script_name": "hello", "params": {"greeting": "hello"} }
            }
        }))
        .send_request(&app).await;
    let mcp_run_body: Value = test::read_body_json(mcp_run).await;
    assert_eq!(mcp_run_body["result"]["isError"], false);

    let mcp_resource = test::TestRequest::post()
        .uri("/mcp")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .set_json(json!({
            "jsonrpc": "2.0", "id": 4, "method": "resources/read",
            "params": { "uri": format!("agentsecret://projects/{project_id}/environments") }
        }))
        .send_request(&app)
        .await;
    let mcp_resource_body: Value = test::read_body_json(mcp_resource).await;
    assert!(mcp_resource_body["result"]["contents"].is_array());

    let gateway_models = test::TestRequest::get()
        .uri("/v1/models")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .send_request(&app)
        .await;
    let gateway_models_body: Value = test::read_body_json(gateway_models).await;
    assert_eq!(gateway_models_body["code"], 0);

    // 4. 启动本地 SSE 假上游，验证流式响应也会脱敏。
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0_u8; 4096];
        let _ = socket.read(&mut buffer).await;
        let body = b"data: hello 13812345678\n\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
    });

    use sea_orm::{NotSet as ModelNotSet, Set as ModelSet};
    let stream_model = crate::entity::b_models::ActiveModel {
        id: ModelNotSet,
        name: ModelSet("stream-model".into()),
        disabled_at: ModelSet(None),
        created_at: ModelSet(chrono::Utc::now().naive_utc()),
        updated_at: ModelSet(chrono::Utc::now().naive_utc()),
    };
    let stream_model = stream_model.insert(&state.conn).await.unwrap();
    let encrypted_token =
        crate::utils::crypto::encrypt_secret(state.config.master_key.as_bytes(), "upstream-token")
            .unwrap();
    let stream_provider = crate::entity::b_providers::ActiveModel {
        id: ModelNotSet,
        name: ModelSet("stream-provider".into()),
        base_url: ModelSet(format!("http://127.0.0.1:{upstream_port}")),
        access_token: ModelSet(encrypted_token),
        provider_type: ModelSet("codex".into()),
        quota_5h_remaining: ModelSet(None),
        quota_5h_refreshed_at: ModelSet(None),
        quota_weekly_remaining: ModelSet(None),
        quota_weekly_refreshed_at: ModelSet(None),
        quota_monthly_remaining: ModelSet(None),
        quota_monthly_refreshed_at: ModelSet(None),
        disabled_at: ModelSet(None),
        created_by: ModelSet(None),
        created_at: ModelSet(chrono::Utc::now().naive_utc()),
        updated_by: ModelSet(None),
        updated_at: ModelSet(chrono::Utc::now().naive_utc()),
    };
    let stream_provider = stream_provider.insert(&state.conn).await.unwrap();
    let stream_relation = crate::entity::r_model_providers::ActiveModel {
        id: ModelNotSet,
        model_id: ModelSet(stream_model.id),
        provider_id: ModelSet(stream_provider.id),
        model_name: ModelSet("upstream-stream-model".into()),
        max_output_tokens: ModelSet(4096),
        max_context_tokens: ModelSet(200000),
        enabled: ModelSet(1),
        last_tested_at: ModelSet(None),
        disabled_at: ModelSet(None),
        created_at: ModelSet(chrono::Utc::now().naive_utc()),
    };
    stream_relation.insert(&state.conn).await.unwrap();

    let stream_response = test::TestRequest::post()
        .uri("/v1/responses")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .set_json(json!({ "model": "stream-model", "input": "hello", "stream": true }))
        .send_request(&app)
        .await;
    assert_eq!(stream_response.status(), StatusCode::OK);
    assert!(
        stream_response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
    let stream_text = String::from_utf8(test::read_body(stream_response).await.to_vec()).unwrap();
    assert!(stream_text.contains("***"));
    assert!(!stream_text.contains("13812345678"));

    // 5. 使用 GLM 假上游验证 Chat Completions 适配和额度刷新。

    let glm_model = crate::entity::b_models::ActiveModel {
        id: ModelNotSet,
        name: ModelSet("glm-model".into()),
        disabled_at: ModelSet(None),
        created_at: ModelSet(chrono::Utc::now().naive_utc()),
        updated_at: ModelSet(chrono::Utc::now().naive_utc()),
    };
    let glm_model = glm_model.insert(&state.conn).await.unwrap();
    let glm_provider = crate::entity::b_providers::ActiveModel {
        id: ModelNotSet,
        name: ModelSet("glm-provider".into()),
        base_url: ModelSet(format!("http://127.0.0.1:{quota_port}")),
        access_token: ModelSet(
            crate::utils::crypto::encrypt_secret(state.config.master_key.as_bytes(), "glm-token")
                .unwrap(),
        ),
        provider_type: ModelSet("glm".into()),
        quota_5h_remaining: ModelSet(None),
        quota_5h_refreshed_at: ModelSet(None),
        quota_weekly_remaining: ModelSet(None),
        quota_weekly_refreshed_at: ModelSet(None),
        quota_monthly_remaining: ModelSet(None),
        quota_monthly_refreshed_at: ModelSet(None),
        disabled_at: ModelSet(None),
        created_by: ModelSet(None),
        created_at: ModelSet(chrono::Utc::now().naive_utc()),
        updated_by: ModelSet(None),
        updated_at: ModelSet(chrono::Utc::now().naive_utc()),
    };
    let glm_provider = glm_provider.insert(&state.conn).await.unwrap();
    crate::entity::r_model_providers::ActiveModel {
        id: ModelNotSet,
        model_id: ModelSet(glm_model.id),
        provider_id: ModelSet(glm_provider.id),
        model_name: ModelSet("upstream-glm".into()),
        max_output_tokens: ModelSet(4096),
        max_context_tokens: ModelSet(200000),
        enabled: ModelSet(1),
        last_tested_at: ModelSet(None),
        disabled_at: ModelSet(None),
        created_at: ModelSet(chrono::Utc::now().naive_utc()),
    }
    .insert(&state.conn)
    .await
    .unwrap();

    let relations = test::TestRequest::post()
        .uri("/api/v1/admin/model-providers/list")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "model_id": glm_model.id }))
        .send_request(&app)
        .await;
    let relations_body: Value = test::read_body_json(relations).await;
    assert_eq!(relations_body["code"], 0, "{relations_body}");
    assert_eq!(relations_body["data"][0]["model_name"], "upstream-glm");

    let glm_response = test::TestRequest::post()
        .uri("/v1/responses")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .set_json(json!({ "model": "glm-model", "input": "hello" }))
        .send_request(&app)
        .await;
    let glm_response_body: Value = test::read_body_json(glm_response).await;
    assert_eq!(glm_response_body["output"][0]["text"], "glm response");
    assert_eq!(glm_response_body["usage"]["input_tokens"], 5);
    assert_eq!(glm_response_body["provider"], "glm-provider");

    let quota_refresh = test::TestRequest::post()
        .uri("/api/v1/admin/providers/refresh-quota")
        .insert_header(("Authorization", format!("Bearer {bearer}")))
        .set_json(json!({ "id": glm_provider.id }))
        .send_request(&app)
        .await;
    let quota_refresh_body: Value = test::read_body_json(quota_refresh).await;
    assert_eq!(quota_refresh_body["code"], 0, "{quota_refresh_body}");
    assert_eq!(quota_refresh_body["data"]["quota_5h_remaining"], 100);
    assert_eq!(quota_refresh_body["data"]["quota_weekly_remaining"], 200);

    let usage = test::TestRequest::post()
        .uri("/api/v1/providers/usage")
        .insert_header(("Authorization", format!("Bearer {access_token}")))
        .set_json(json!({ "id": stream_provider.id }))
        .send_request(&app)
        .await;
    let usage_body: Value = test::read_body_json(usage).await;
    assert_eq!(usage_body["code"], 0);

    // 3. 前端 SPA 与健康检查。
    let frontend = test::TestRequest::get()
        .uri("/tokens")
        .send_request(&app)
        .await;
    assert_eq!(frontend.status(), StatusCode::OK);
    let health = test::TestRequest::get()
        .uri("/health")
        .send_request(&app)
        .await;
    assert_eq!(health.status(), StatusCode::OK);
}
