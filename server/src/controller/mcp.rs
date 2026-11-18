use actix_web::{HttpResponse, web};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};

use crate::entity::{
    b_environments, b_executions, b_projects, b_sandboxes, b_scripts, b_users,
    r_environment_permissions, r_project_permissions,
};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::state::AppState;

/// 处理 MCP Streamable HTTP JSON-RPC 请求。
pub async fn handle(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<Value>,
) -> HttpResponse {
    let id = body.get("id").cloned();
    let method = body
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let Ok(_mcp_permit) = state.mcp_semaphore.clone().acquire_owned().await else {
        return mcp_error(id, 7, "MCP service is closed");
    };

    if method == "tools/call" {
        let name = body
            .get("params")
            .and_then(|params| params.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let execution = matches!(name, "script_run");
        if !state.acquire_mcp_rate_limit(actor.id, execution) {
            return mcp_error(id, 5, "MCP rate limit exceeded");
        }
    }

    let result = match method {
        "initialize" => Ok(initialize()),
        "notifications/initialized" => return HttpResponse::Accepted().finish(),
        "tools/list" => Ok(tools_list()),
        "tools/call" => call_tool(&actor, &state, body.get("params")).await,
        "resources/list" => Ok(resources_list()),
        "resources/read" => {
            read_resource(
                &state,
                &actor,
                body.get("params")
                    .and_then(|params| params.get("uri"))
                    .and_then(Value::as_str),
            )
            .await
        }
        _ => Err(AppError::InvalidArgument(format!(
            "unsupported MCP method: {method}"
        ))),
    };

    let (status, payload) = match result {
        Ok(result) => (
            actix_web::http::StatusCode::OK,
            json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        ),
        Err(error) => (
            actix_web::http::StatusCode::OK,
            json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": error.code(), "message": error.to_string() }
            }),
        ),
    };
    HttpResponse::build(status).json(payload)
}

fn mcp_error(id: Option<Value>, code: i32, message: &str) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }))
}

fn initialize() -> Value {
    json!({
        "protocolVersion": "2025-03-26",
        "capabilities": { "tools": { "listChanged": false }, "resources": { "listChanged": false } },
        "serverInfo": { "name": "agentsecret", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn tools_list() -> Value {
    json!({
        "tools": [
            { "name": "project_list", "description": "列出当前用户可访问的项目", "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false } },
            { "name": "script_list", "description": "列出项目中已审核的脚本", "inputSchema": { "type": "object", "required": ["project"], "properties": { "project": { "type": "string" } }, "additionalProperties": false } },
            { "name": "script_run", "description": "执行已审核脚本", "inputSchema": { "type": "object", "required": ["project", "script_name"], "properties": { "project": { "type": "string" }, "environment": { "type": "string" }, "script_name": { "type": "string" }, "params": { "type": "object" } }, "additionalProperties": false } },
            { "name": "execution_status", "description": "查询执行进度", "inputSchema": { "type": "object", "required": ["execution_id"], "properties": { "execution_id": { "type": "integer" } }, "additionalProperties": false } }
        ]
    })
}

fn resources_list() -> Value {
    json!({
        "resources": [
            { "uri": "agentsecret://projects/{project_id}/environments", "name": "项目环境", "mimeType": "application/json" },
            { "uri": "agentsecret://executions/{execution_id}", "name": "执行详情", "mimeType": "application/json" }
        ]
    })
}

/// 读取 MCP 只读资源：项目环境或执行详情。
async fn read_resource(
    state: &web::Data<AppState>,
    actor: &AuthUser,
    uri: Option<&str>,
) -> Result<Value, AppError> {
    let uri = uri.ok_or_else(|| AppError::InvalidArgument("resource uri is required".into()))?;
    if let Some(rest) = uri
        .strip_prefix("agentsecret://projects/")
        .and_then(|rest| rest.strip_suffix("/environments"))
    {
        let project_id = rest
            .parse::<u64>()
            .map_err(|_| AppError::InvalidArgument("resource uri is invalid".into()))?;
        let project = b_projects::Entity::find_by_id(project_id)
            .one(&state.conn)
            .await?
            .ok_or_else(|| AppError::NotFound("project does not exist".into()))?;
        let allowed = allowed_projects(&state.conn, actor).await?;
        if !actor.is_admin && !allowed.as_ref().is_none_or(|ids| ids.contains(&project.id)) {
            return Err(AppError::Forbidden("project permission is required".into()));
        }
        let environments = b_environments::Entity::find()
            .filter(b_environments::Column::ProjectId.eq(project.id))
            .all(&state.conn)
            .await?;
        let items = environments
            .iter()
            .map(|env| json!({ "id": env.id, "name": env.name }))
            .collect::<Vec<_>>();
        return Ok(
            json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": serde_json::to_string(&items).unwrap_or_default() }] }),
        );
    }

    if let Some(rest) = uri.strip_prefix("agentsecret://executions/") {
        let execution_id = rest
            .parse::<u64>()
            .map_err(|_| AppError::InvalidArgument("resource uri is invalid".into()))?;
        let arguments = json!({ "execution_id": execution_id });
        let result = execution_status(state, actor, &arguments).await?;
        return Ok(
            json!({ "contents": [{ "uri": uri, "mimeType": "application/json", "text": result.to_string() }] }),
        );
    }

    Err(AppError::NotFound("resource does not exist".into()))
}

async fn call_tool(
    actor: &AuthUser,
    state: &web::Data<AppState>,
    params: Option<&Value>,
) -> Result<Value, AppError> {
    let params =
        params.ok_or_else(|| AppError::InvalidArgument("MCP params is required".into()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("MCP tool name is required".into()))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let content = match name {
        "project_list" => project_list(state, actor).await?,
        "script_list" => script_list(state, actor, &arguments).await?,
        "script_run" => script_run(state, actor, &arguments).await?,
        "execution_status" => execution_status(state, actor, &arguments).await?,
        _ => {
            return Err(AppError::InvalidArgument(format!(
                "unknown MCP tool: {name}"
            )));
        }
    };
    b_users::insert_log(&state.conn, actor.id, "mcp_call", "success", Some(name)).await?;

    Ok(json!({
        "content": [{ "type": "text", "text": content.to_string() }],
        "isError": false
    }))
}

async fn project_list(state: &web::Data<AppState>, actor: &AuthUser) -> Result<Value, AppError> {
    let projects = b_projects::Entity::find()
        .filter(b_projects::Column::DisabledAt.is_null())
        .all(&state.conn)
        .await?;
    let allowed = allowed_projects(&state.conn, actor).await?;
    let mut items = Vec::new();
    for project in projects {
        if !actor.is_admin && !allowed.as_ref().is_none_or(|ids| ids.contains(&project.id)) {
            continue;
        }
        let repos = project
            .repos
            .clone()
            .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
            .unwrap_or_default();
        items.push(json!({
            "name": project.name,
            "description": project.description,
            "guidance": project.guidance,
            "repos": repos
        }));
    }

    Ok(json!({ "projects": items }))
}

async fn script_list(
    state: &web::Data<AppState>,
    actor: &AuthUser,
    arguments: &Value,
) -> Result<Value, AppError> {
    let project_name = arguments
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("project is required".into()))?;
    let project = b_projects::Entity::find()
        .filter(b_projects::Column::Name.eq(project_name))
        .filter(b_projects::Column::DisabledAt.is_null())
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("project does not exist".into()))?;
    let allowed = allowed_projects(&state.conn, actor).await?;
    if !actor.is_admin && !allowed.as_ref().is_none_or(|ids| ids.contains(&project.id)) {
        return Err(AppError::Forbidden("project permission is required".into()));
    }

    let scripts = b_scripts::Entity::find()
        .filter(b_scripts::Column::ProjectId.eq(project.id))
        .filter(b_scripts::Column::ReviewStatus.eq("approved"))
        .all(&state.conn)
        .await?;
    let mut items = Vec::new();
    for script in scripts {
        let environment_name = match script.environment_id {
            Some(environment_id) => b_environments::Entity::find_by_id(environment_id)
                .one(&state.conn)
                .await?
                .map(|environment| environment.name),
            None => None,
        };
        let sandbox_name = match script.sandbox_id {
            Some(sandbox_id) => b_sandboxes::Entity::find_by_id(sandbox_id)
                .one(&state.conn)
                .await?
                .map(|sandbox| sandbox.name),
            None => None,
        };
        items.push(json!({
            "name": script.name,
            "type": script.r#type,
            "description": script.description,
            "environment_name": environment_name,
            "sandbox_name": sandbox_name,
            "updated_at": script.updated_at.and_utc()
        }));
    }

    Ok(json!({ "scripts": items }))
}

/// 按项目名和脚本名执行已审核脚本。
async fn script_run(
    state: &web::Data<AppState>,
    actor: &AuthUser,
    arguments: &Value,
) -> Result<Value, AppError> {
    let project_name = arguments
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("project is required".into()))?;
    let script_name = arguments
        .get("script_name")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidArgument("script_name is required".into()))?;
    let environment = arguments.get("environment").and_then(Value::as_str);

    // 1. 查找项目、脚本并核对审核版本与环境绑定。
    let project = b_projects::Entity::find()
        .filter(b_projects::Column::Name.eq(project_name))
        .filter(b_projects::Column::DisabledAt.is_null())
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("project does not exist".into()))?;
    let allowed = allowed_projects(&state.conn, actor).await?;
    if !actor.is_admin && !allowed.as_ref().is_none_or(|ids| ids.contains(&project.id)) {
        return Err(AppError::Forbidden("project permission is required".into()));
    }
    let script = b_scripts::Entity::find()
        .filter(b_scripts::Column::ProjectId.eq(project.id))
        .filter(b_scripts::Column::Name.eq(script_name))
        .filter(b_scripts::Column::ReviewStatus.eq("approved"))
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("approved script does not exist".into()))?;
    let actual_environment = match script.environment_id {
        Some(environment_id) => b_environments::Entity::find_by_id(environment_id)
            .one(&state.conn)
            .await?
            .map(|row| row.name),
        None => None,
    };
    if actual_environment.as_deref() != environment {
        return Err(AppError::InvalidArgument(
            "environment does not match the approved script".into(),
        ));
    }

    // 2. 复用与 HTTP 相同的沙盒执行服务。
    let params = arguments
        .get("params")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    crate::service::script_params::validate_runtime(script.params.as_ref(), &params)?;
    let execution_id =
        crate::service::execution::run_script(state, script, actor.id, params).await?;

    Ok(json!({ "execution_id": execution_id, "status": "success" }))
}

async fn execution_status(
    state: &web::Data<AppState>,
    actor: &AuthUser,
    arguments: &Value,
) -> Result<Value, AppError> {
    let execution_id = arguments
        .get("execution_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| AppError::InvalidArgument("execution_id is required".into()))?;
    let execution = b_executions::Entity::find_by_id(execution_id)
        .one(&state.conn)
        .await?
        .ok_or_else(|| AppError::NotFound("execution does not exist".into()))?;
    if !actor.is_admin && execution.user_id != actor.id {
        return Err(AppError::Forbidden(
            "execution does not belong to current user".into(),
        ));
    }
    let duration_ms = match (execution.started_at, execution.finished_at) {
        (Some(started_at), Some(finished_at)) => {
            Some((finished_at.and_utc() - started_at.and_utc()).num_milliseconds())
        }
        _ => None,
    };

    Ok(json!({
        "execution_id": execution.id,
        "status": execution.status,
        "started_at": execution.started_at.map(|value| value.and_utc()),
        "finished_at": execution.finished_at.map(|value| value.and_utc()),
        "duration_ms": duration_ms,
        "stdout": execution.stdout,
        "stderr": execution.stderr
    }))
}

async fn allowed_projects(
    conn: &sea_orm::DatabaseConnection,
    actor: &AuthUser,
) -> Result<Option<Vec<u64>>, AppError> {
    let permissions = r_project_permissions::Entity::find()
        .filter(r_project_permissions::Column::UserId.eq(actor.id))
        .all(conn)
        .await?;
    if permissions
        .iter()
        .any(|permission| permission.project_id.is_none())
    {
        return Ok(None);
    }
    // 环境通配授权不直接代表项目可见，这里仍要求项目授权。
    let _ = r_environment_permissions::Entity::find()
        .filter(r_environment_permissions::Column::UserId.eq(actor.id))
        .all(conn)
        .await?;

    Ok(Some(
        permissions
            .into_iter()
            .filter_map(|permission| permission.project_id)
            .collect(),
    ))
}
