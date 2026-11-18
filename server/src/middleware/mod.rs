use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::web::Data;
pub mod access_token;
pub mod dual_auth;

use actix_web::{Error, HttpMessage, HttpResponse};
use chrono::Utc;

use crate::entity::b_users;
use crate::error::AppError;
use crate::schema::ApiResponse;
use crate::state::AppState;

/// 校验登录会话并写入 AuthUser。
pub async fn auth(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let Some(state) = req.app_data::<Data<AppState>>().cloned() else {
        return Ok(error_response(
            req,
            AppError::internal("app state is missing"),
        ));
    };
    let Some(token) = bearer_token(&req) else {
        return Ok(error_response(
            req,
            AppError::Unauthenticated("bearer session token is required".into()),
        ));
    };

    // 1. JWT 验签并取出用户 ID。
    let claims = state.tokens.verify_session(token)?;
    let user_id = claims
        .sub
        .parse::<u64>()
        .map_err(|_| AppError::Unauthenticated("session token is invalid".into()))?;

    // 2. 数据库确认用户存在、封禁结束、会话 token 一致且未过期。
    let Some(user) = b_users::find_by_id(&state.conn, user_id).await? else {
        return Ok(error_response(
            req,
            AppError::Unauthenticated("session token is invalid".into()),
        ));
    };
    let now = Utc::now();
    let banned = matches!(
        (user.banned_at, user.unban_at),
        (Some(banned_at), None) if now.naive_utc() >= banned_at
    ) || matches!(
        (user.banned_at, user.unban_at),
        (Some(banned_at), Some(unban_at)) if now.naive_utc() >= banned_at && now.naive_utc() < unban_at
    );
    if banned
        || user.session_token.as_deref() != Some(token)
        || user
            .session_expire_at
            .is_none_or(|expires_at| expires_at <= now.naive_utc())
    {
        return Ok(error_response(
            req,
            AppError::Unauthenticated("session token is invalid".into()),
        ));
    }

    // 3. 写入请求扩展，供控制器和管理员中间件复用。
    let auth_user = crate::middleware::AuthUser {
        id: user.id,
        email: user.email.clone(),
        name: user.name.clone(),
        is_admin: user.is_admin != 0,
    };
    req.extensions_mut().insert(auth_user);

    next.call(req)
        .await
        .map(ServiceResponse::map_into_boxed_body)
}

/// 校验当前登录用户是否为管理员。
pub async fn require_admin(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let Some(user) = req.extensions().get::<AuthUser>().cloned() else {
        return Ok(error_response(
            req,
            AppError::Unauthenticated("authenticated user is required".into()),
        ));
    };
    if !user.is_admin {
        return Ok(error_response(
            req,
            AppError::Forbidden("administrator permission is required".into()),
        ));
    }

    next.call(req)
        .await
        .map(ServiceResponse::map_into_boxed_body)
}

/// 生成或透传请求 ID。
pub async fn request_id(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(crate::utils::crypto::random_id);

    req.extensions_mut().insert(request_id.clone());
    let mut response = next.call(req).await?;
    if let Ok(value) = actix_web::http::header::HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(
            actix_web::http::header::HeaderName::from_static("x-request-id"),
            value,
        );
    }

    Ok(response.map_into_boxed_body())
}

fn bearer_token(req: &ServiceRequest) -> Option<&str> {
    req.headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

fn error_response(req: ServiceRequest, error: AppError) -> ServiceResponse<BoxBody> {
    let response = HttpResponse::Ok().json(ApiResponse::<Option<()>> {
        code: error.code(),
        message: error.to_string(),
        data: None,
    });

    ServiceResponse::new(req.into_parts().0, response).map_into_boxed_body()
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: u64,
    pub email: String,
    pub name: String,
    pub is_admin: bool,
}
