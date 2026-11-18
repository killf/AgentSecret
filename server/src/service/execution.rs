#![allow(deprecated)] // Bollard 0.19 保留旧 options API 兼容层

use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, WaitContainerOptions,
};
use bollard::query_parameters::{AttachContainerOptionsBuilder, ListContainersOptionsBuilder};
use chrono::Utc;
use futures_util::StreamExt;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, PaginatorTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::entity::{b_executions, b_sandbox_instances, b_sandboxes, b_scripts};
use crate::error::AppError;
use crate::service::credential::{
    PreparedCredentialAction, execute_credential_action, prepare_credential_actions,
};
use crate::service::sensitive::mask_text;
use crate::state::AppState;
use crate::utils::crypto::random_id;

/// 内存中的有界输出捕获结果；ok=false 时调用方不得信任输出内容。
#[derive(Default)]
struct CapturedOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    ok: bool,
    action_ok: bool,
}

struct CaptureState {
    http: reqwest::Client,
    actions: Vec<PreparedCredentialAction>,
}

/// 执行一个已审核脚本：创建专属容器、等待结束、过滤输出并销毁容器。
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const ACTION_MARKER: &[u8] = b"\x1eAGENTSECRET_ACTION\x1f";
const MAX_BUFFER_BYTES: usize = MAX_OUTPUT_BYTES + 64 * 1024;

/// 服务启动时只清理本服务标记的遗留容器，并把未完成执行标记为失败。
pub async fn cleanup_interrupted_containers(state: &AppState) -> Result<usize, AppError> {
    let mut filters = std::collections::HashMap::new();
    filters.insert(
        "label".to_string(),
        vec!["agentsecret.service=sandbox-execution".to_string()],
    );
    let options = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();
    let containers = state
        .docker
        .list_containers(Some(options))
        .await
        .map_err(AppError::internal)?;

    let mut cleaned = 0;
    for container in containers {
        // 1. 只处理能解析出执行 ID 且数据库仍处于运行态的容器。
        let Some(labels) = container.labels else {
            continue;
        };
        let Some(execution_id) = labels
            .get("agentsecret.execution-id")
            .and_then(|value| value.parse::<u64>().ok())
        else {
            continue;
        };
        let Some(name) = container.names.as_ref().and_then(|names| names.first()) else {
            continue;
        };
        let instance_name = name.trim_start_matches('/').to_string();

        let Some(execution) = b_executions::Entity::find_by_id(execution_id)
            .one(&state.conn)
            .await?
        else {
            // 标签可被外部 Docker 调用方伪造；数据库无对应执行时不阻断启动。
            continue;
        };
        if execution.status != "running" {
            continue;
        }

        // 2. 先强制删除容器，再落终态，避免数据库先释放而清理失败。
        let removed = state
            .docker
            .remove_container(
                &instance_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .is_ok();
        let mut execution_active: b_executions::ActiveModel = execution.into();
        execution_active.status = Set("failed".into());
        execution_active.stdout = Set(None);
        execution_active.stderr = Set(None);
        execution_active.error_code = Set(Some("service_restarted".to_string()));
        execution_active.finished_at = Set(Some(Utc::now().naive_utc()));
        execution_active.update(&state.conn).await?;
        update_instance(&state.conn, instance_name, removed).await?;
        cleaned += 1;
    }

    Ok(cleaned)
}

pub async fn run_script(
    state: &AppState,
    script: b_scripts::Model,
    user_id: u64,
    params: Value,
) -> Result<u64, AppError> {
    if script.review_status != "approved" {
        return Err(AppError::InvalidArgument("script is not approved".into()));
    }
    let sandbox_id = script
        .sandbox_id
        .ok_or_else(|| AppError::InvalidArgument("script has no sandbox binding".into()))?;
    let conn = &state.conn;
    let docker = &state.docker;
    let instance_name = format!("agentsecret-exec-{}", random_id());

    // 2. 事务内检查并发并创建执行与专属实例记录。
    // 2. 锁定沙盒与脚本行后重新核对版本、绑定和密钥，避免与审核/绑定变更并发竞争。
    let txn = conn.begin().await?;
    let sandbox = b_sandboxes::Entity::find_by_id(sandbox_id)
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("sandbox does not exist".into()))?;
    let current_script = b_scripts::Entity::find_by_id(script.id)
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::NotFound("script does not exist".into()))?;
    if current_script.revision != script.revision
        || current_script.review_status != script.review_status
    {
        return Err(AppError::InvalidArgument(
            "script revision changed before execution".into(),
        ));
    }
    let prepared_actions = prepare_credential_actions(&txn, &state.config, &current_script).await?;
    let command = build_command(&current_script)?;
    let active_count = b_sandbox_instances::Entity::find()
        .filter(b_sandbox_instances::Column::SandboxId.eq(sandbox.id))
        .filter(b_sandbox_instances::Column::Status.ne("destroyed"))
        .count(&txn)
        .await?;
    if active_count >= sandbox.max_instances as u64 {
        return Err(AppError::LimitExceeded(
            "sandbox is at maximum concurrency".into(),
        ));
    }
    let instance = b_sandbox_instances::ActiveModel {
        id: NotSet,
        sandbox_id: Set(sandbox.id),
        name: Set(instance_name.clone()),
        status: Set("creating".into()),
        created_at: Set(Utc::now().naive_utc()),
        destroyed_at: Set(None),
    };
    let instance = instance.insert(&txn).await?;
    let execution = b_executions::ActiveModel {
        id: NotSet,
        script_id: Set(script.id),
        script_revision: Set(current_script.revision),
        project_id: Set(current_script.project_id),
        environment_id: Set(current_script.environment_id),
        r#type: Set(current_script.r#type.clone()),
        status: Set("running".into()),
        stdout: Set(None),
        stderr: Set(None),
        error_code: Set(None),
        sandbox_id: Set(sandbox.id),
        sandbox_instance_id: Set(instance.id),
        user_id: Set(user_id),
        started_at: Set(Some(Utc::now().naive_utc())),
        finished_at: Set(None),
    };
    let execution = execution.insert(&txn).await?;
    let mut instance_active: b_sandbox_instances::ActiveModel = instance.into();
    instance_active.status = Set("running".into());
    instance_active.update(&txn).await?;
    txn.commit().await?;

    // 4. 创建无网络、只读根文件系统、降权的一次性容器；标签用于恢复期清理。
    let host_config = bollard::models::HostConfig {
        network_mode: Some("none".to_string()),
        memory: Some(256 * 1024 * 1024),
        nano_cpus: Some(1_000_000_000),
        pids_limit: Some(128),
        cap_drop: Some(vec!["ALL".to_string()]),
        log_config: Some(bollard::models::HostConfigLogConfig {
            typ: Some("none".to_string()),
            config: Default::default(),
        }),
        readonly_rootfs: Some(true),
        tmpfs: Some({
            let mut tmpfs = std::collections::HashMap::new();
            tmpfs.insert("/tmp".to_string(), "rw,size=16m".to_string());
            tmpfs
        }),
        ..Default::default()
    };
    let mut labels = std::collections::HashMap::new();
    labels.insert(
        "agentsecret.service".to_string(),
        "sandbox-execution".to_string(),
    );
    labels.insert(
        "agentsecret.execution-id".to_string(),
        execution.id.to_string(),
    );
    let config = Config {
        image: Some(sandbox.image.clone()),
        cmd: Some(command),
        env: Some(vec![format!("AGENTSECRET_PARAMS={params}")]),
        user: Some("nobody".to_string()),
        working_dir: Some("/tmp".to_string()),
        open_stdin: Some(true),
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        labels: Some(labels),
        host_config: Some(host_config),
        ..Default::default()
    };
    let create_result = docker
        .create_container(
            Some(CreateContainerOptions {
                name: instance_name.clone(),
                platform: None,
            }),
            config,
        )
        .await;
    if let Err(error) = create_result {
        cleanup_failed(conn, execution.id, &instance_name, docker).await?;
        tracing::error!(error = %error, execution_id = execution.id, "container create failed");
        return Err(AppError::Internal("container create failed".into()));
    }

    // 5. 启动前附加双向流；stdin 只用于服务端返回已过滤动作结果。
    let attach_options = AttachContainerOptionsBuilder::default()
        .stdin(true)
        .stdout(true)
        .stderr(true)
        .stream(true)
        .build();
    let attached = match docker
        .attach_container(&instance_name, Some(attach_options))
        .await
    {
        Ok(attached) => attached,
        Err(error) => {
            cleanup_failed(conn, execution.id, &instance_name, docker).await?;
            tracing::error!(error = %error, execution_id = execution.id, "container attach failed");
            return Err(AppError::Internal("container attach failed".into()));
        }
    };
    if let Err(error) = docker
        .start_container(
            &instance_name,
            None::<bollard::container::StartContainerOptions<String>>,
        )
        .await
    {
        cleanup_failed(conn, execution.id, &instance_name, docker).await?;
        tracing::error!(error = %error, execution_id = execution.id, "container start failed");
        return Err(AppError::Internal("container start failed".into()));
    }

    // 6. 后台解析输出协议并调用凭据动作；原始输出不落库、不打印。
    let mut captured = CapturedOutput {
        ok: true,
        action_ok: true,
        ..Default::default()
    };
    let output_task = tokio::spawn(run_capture_stream(
        attached.output,
        attached.input,
        state.http.clone(),
        prepared_actions,
    ));
    let wait_result = tokio::time::timeout(
        std::time::Duration::from_secs(600),
        docker
            .wait_container(&instance_name, None::<WaitContainerOptions<String>>)
            .next(),
    )
    .await;
    let mut error_code = None;
    match wait_result {
        Ok(Some(Ok(_))) => {}
        Ok(Some(Err(error))) => {
            tracing::error!(error = %error, execution_id = execution.id, "container wait failed");
            error_code = Some("container_wait_failed");
        }
        Ok(None) => error_code = Some("container_wait_failed"),
        Err(_) => {
            let _ = docker
                .kill_container(
                    &instance_name,
                    None::<bollard::container::KillContainerOptions<String>>,
                )
                .await;
            error_code = Some("timeout");
        }
    }

    // 7. 附加流结束后统一过滤；读取、编码或动作失败时丢弃不可信输出。
    match tokio::time::timeout(std::time::Duration::from_secs(5), output_task).await {
        Ok(Ok(result)) => captured = result,
        _ => error_code = Some("log_capture_failed"),
    }
    if !captured.ok {
        error_code = Some("log_capture_failed");
    }
    if !captured.action_ok {
        error_code = Some("credential_action_failed");
    }
    let stdout = String::from_utf8(std::mem::take(&mut captured.stdout));
    let stderr = String::from_utf8(std::mem::take(&mut captured.stderr));
    let (stdout, stderr) = match (stdout, stderr) {
        (Ok(stdout), Ok(stderr)) => (stdout, stderr),
        _ => {
            error_code = Some("log_capture_failed");
            (String::new(), String::new())
        }
    };
    let (stdout, _) = mask_text(&stdout);
    let (stderr, _) = mask_text(&stderr);

    // 8. 强制删除容器，只有确认删除后才释放并发名额。
    let remove_result = docker
        .remove_container(
            &instance_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;
    let status = if error_code.is_none() {
        "success"
    } else {
        "failed"
    };
    let mut execution_active: b_executions::ActiveModel = execution.into();
    execution_active.status = Set(status.to_string());
    execution_active.stdout = Set(Some(stdout));
    execution_active.stderr = Set(Some(stderr));
    execution_active.error_code = Set(error_code.map(str::to_string));
    execution_active.finished_at = Set(Some(Utc::now().naive_utc()));
    let updated = execution_active.update(conn).await?;
    update_instance(conn, instance_name, remove_result.is_ok()).await?;
    if error_code.is_some() {
        return Err(AppError::Internal("script execution failed".into()));
    }

    Ok(updated.id)
}

/// 为 bash/python 注入同名动作入口；脚本只传动作 ID 和业务参数。
fn build_command(script: &b_scripts::Model) -> Result<Vec<String>, AppError> {
    match script.r#type.as_str() {
        "bash" => {
            let helper = r#"exec 9>&1
agentsecret_action() {
    local action_id params line
    action_id="$1"
    params="$2"
    printf '\036AGENTSECRET_ACTION\037%s %s\n' "$action_id" "$params" >&9
    if ! IFS= read -r line; then
        return 1
    fi
    printf '%s\n' "$line"
}
"#;
            let content = format!("{helper}\n{}", script.content);
            Ok(vec!["bash".to_string(), "-lc".to_string(), content])
        }
        "python" => {
            let encoded = serde_json::to_string(&script.content).map_err(AppError::internal)?;
            let helper = format!(
                r#"import json, os, sys

action_fd = os.dup(1)

def agentsecret_action(action_id, params):
    payload = json.dumps(params, separators=(",", ":"))
    request = "\x1eAGENTSECRET_ACTION\x1f%s %s\n" % (action_id, payload)
    os.write(action_fd, request.encode("utf-8"))
    line = sys.stdin.readline()
    if not line:
        raise RuntimeError("credential action channel closed")
    return json.loads(line)

exec(compile({encoded}, "<agentsecret-script>", "exec"), globals())
os.close(action_fd)
"#
            );
            Ok(vec!["python".to_string(), "-c".to_string(), helper])
        }
        _ => Err(AppError::InvalidArgument("unsupported script type".into())),
    }
}

/// 逐行处理 stdout 协议；协议行不进入输出，普通输出在完整收集后统一脱敏。
async fn run_capture_stream<S, W>(
    mut output: S,
    mut input: W,
    http: reqwest::Client,
    actions: Vec<PreparedCredentialAction>,
) -> CapturedOutput
where
    S: futures_util::Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>>
        + Unpin,
    W: tokio::io::AsyncWrite + Unpin + Send,
{
    let capture_state = CaptureState { http, actions };
    let mut result = CapturedOutput {
        ok: true,
        action_ok: true,
        ..Default::default()
    };
    let mut stdout_buffer = Vec::new();

    // 1. Docker 输出按帧到达，先按字节缓冲，避免多字节字符被截断。
    while let Some(message) = output.next().await {
        match message {
            Ok(bollard::container::LogOutput::StdOut { message }) => {
                stdout_buffer.extend_from_slice(&message);
                if stdout_buffer.len() > MAX_BUFFER_BYTES {
                    result.ok = false;
                    break;
                }
                while let Some(index) = stdout_buffer.iter().position(|byte| *byte == b'\n') {
                    let mut line: Vec<u8> = stdout_buffer.drain(..=index).collect();
                    line.pop();

                    // 2. 凭据动作协议行由服务端消费，普通行进入有界输出。
                    if line.starts_with(ACTION_MARKER) {
                        let action_result =
                            handle_action_line(&capture_state, &line[ACTION_MARKER.len()..]).await;
                        if action_result.is_err() {
                            result.action_ok = false;
                        }
                        let response = match action_result {
                            Ok(data) => serde_json::json!({ "ok": true, "data": data }),
                            Err(error) => serde_json::json!({ "ok": false, "error": error }),
                        };
                        let mut bytes = serde_json::to_vec(&response).unwrap_or_default();
                        bytes.push(b'\n');
                        if input.write_all(&bytes).await.is_err() {
                            result.action_ok = false;
                        }
                    } else {
                        append_output(&mut result.stdout, &line, &mut result.ok);
                        append_output(&mut result.stdout, b"\n", &mut result.ok);
                    }
                    if !result.ok {
                        break;
                    }
                }
                if !result.ok {
                    break;
                }
            }
            Ok(bollard::container::LogOutput::StdErr { message }) => {
                append_output(&mut result.stderr, &message, &mut result.ok);
            }
            Ok(_) => {}
            Err(_) => result.ok = false,
        }
    }

    // 3. 流结束时保留没有换行的普通输出；UTF-8 最终校验由调用方完成。
    if !stdout_buffer.is_empty() {
        append_output(&mut result.stdout, &stdout_buffer, &mut result.ok);
    }
    let _ = input.shutdown().await;

    result
}

async fn handle_action_line(state: &CaptureState, payload: &[u8]) -> Result<Value, String> {
    let split = payload
        .iter()
        .position(|byte| *byte == b' ')
        .ok_or("invalid credential action request")?;
    let action_id = std::str::from_utf8(&payload[..split])
        .map_err(|_| "invalid credential action request")?
        .to_string();
    let params: Value = serde_json::from_slice(&payload[split + 1..])
        .map_err(|_| "invalid credential action parameters")?;
    if !params.is_object() {
        return Err("credential action parameters must be an object".into());
    }

    // 4. 只允许已审核配置中的动作，参数和响应继续由执行端白名单约束。
    let action = state
        .actions
        .iter()
        .find(|item| item.action_id == action_id)
        .ok_or("credential action is not approved")?;

    execute_credential_action(&state.http, action, &params).await
}

fn append_output(target: &mut Vec<u8>, bytes: &[u8], ok: &mut bool) {
    if target.len() + bytes.len() > MAX_OUTPUT_BYTES {
        *ok = false;
        target.clear();
        return;
    }
    target.extend_from_slice(bytes);
}

async fn mark_failed(
    conn: &DatabaseConnection,
    execution_id: u64,
    error_code: &str,
) -> Result<(), AppError> {
    let execution = b_executions::Entity::find_by_id(execution_id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("execution does not exist".into()))?;
    let mut active: b_executions::ActiveModel = execution.into();
    active.status = Set("failed".into());
    active.error_code = Set(Some(error_code.to_string()));
    active.finished_at = Set(Some(Utc::now().naive_utc()));
    active.update(conn).await?;

    Ok(())
}

async fn cleanup_failed(
    conn: &DatabaseConnection,
    execution_id: u64,
    instance_name: &str,
    docker: &Docker,
) -> Result<(), AppError> {
    let removed = docker
        .remove_container(
            instance_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
        .is_ok();
    mark_failed(conn, execution_id, "container_create_failed").await?;
    update_instance(conn, instance_name.to_string(), removed).await?;

    Ok(())
}

async fn update_instance(
    conn: &DatabaseConnection,
    instance_name: String,
    destroyed: bool,
) -> Result<(), AppError> {
    let instance = b_sandbox_instances::Entity::find()
        .filter(b_sandbox_instances::Column::Name.eq(instance_name))
        .one(conn)
        .await?
        .ok_or_else(|| AppError::NotFound("sandbox instance does not exist".into()))?;
    let mut active: b_sandbox_instances::ActiveModel = instance.into();
    active.status = Set(if destroyed {
        "destroyed".into()
    } else {
        "cleanup_failed".into()
    });
    active.destroyed_at = Set(destroyed.then(|| Utc::now().naive_utc()));
    active.update(conn).await?;

    Ok(())
}
