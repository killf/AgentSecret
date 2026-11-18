use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::web::Data;
use actix_web::{Error, HttpMessage, HttpResponse};

use crate::entity::b_access_tokens;
use crate::state::AppState;

/// 校验模型网关 / MCP 共用的访问令牌。
pub async fn access_token(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let Some(state) = req.app_data::<Data<AppState>>().cloned() else {
        return Ok(error_response(req, 500, "7", "server error"));
    };
    let Some(token) = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return Ok(error_response(req, 401, "2", "missing bearer access token"));
    };

    // 1. 验证 JWT、数据库撤销状态和用户封禁状态。
    let user = match b_access_tokens::validate(&state.conn, token, &state.tokens).await {
        Ok(user) => user,
        Err(_) => return Ok(error_response(req, 401, "2", "invalid access token")),
    };

    // 2. 写入与登录会话一致的 AuthUser。
    req.extensions_mut().insert(super::AuthUser {
        id: user.id,
        email: user.email,
        name: user.name,
        is_admin: user.is_admin != 0,
    });

    next.call(req)
        .await
        .map(ServiceResponse::map_into_boxed_body)
}

fn error_response(
    req: ServiceRequest,
    status: u16,
    code: &str,
    message: &str,
) -> ServiceResponse<BoxBody> {
    let response = HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap())
        .json(serde_json::json!({
            "error": { "message": message, "type": "authentication_error", "code": code }
        }));

    ServiceResponse::new(req.into_parts().0, response).map_into_boxed_body()
}
