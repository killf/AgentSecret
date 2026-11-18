use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::web::Data;
use actix_web::{Error, HttpMessage, HttpResponse};

use crate::entity::{b_access_tokens, b_users};
use crate::error::AppError;
use crate::schema::ApiResponse;
use crate::state::AppState;

/// 辅助接口鉴权：优先登录会话，失败时尝试访问令牌。
pub async fn session_or_access(
    req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let Some(state) = req.app_data::<Data<AppState>>().cloned() else {
        return Ok(error_response(
            req,
            AppError::internal("app state is missing"),
        ));
    };
    let Some(token) = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return Ok(error_response(
            req,
            AppError::Unauthenticated("bearer token is required".into()),
        ));
    };

    // 1. 先按登录会话验证；JWT role 字段能快速区分两类令牌。
    let actor = if let Ok(claims) = state.tokens.verify_session(token) {
        let user_id = claims
            .sub
            .parse::<u64>()
            .map_err(|_| AppError::Unauthenticated("token is invalid".into()))?;
        let user = b_users::find_by_id(&state.conn, user_id)
            .await?
            .ok_or_else(|| AppError::Unauthenticated("token is invalid".into()))?;
        if user.session_token.as_deref() == Some(token) {
            super::AuthUser {
                id: user.id,
                email: user.email,
                name: user.name,
                is_admin: user.is_admin != 0,
            }
        } else {
            return Ok(error_response(
                req,
                AppError::Unauthenticated("token is invalid".into()),
            ));
        }
    } else {
        // 2. 再按模型网关 / MCP 访问令牌验证。
        let user = b_access_tokens::validate(&state.conn, token, &state.tokens).await?;
        super::AuthUser {
            id: user.id,
            email: user.email,
            name: user.name,
            is_admin: user.is_admin != 0,
        }
    };
    req.extensions_mut().insert(actor);

    next.call(req)
        .await
        .map(ServiceResponse::map_into_boxed_body)
}

fn error_response(req: ServiceRequest, error: AppError) -> ServiceResponse<BoxBody> {
    let response = HttpResponse::Ok().json(ApiResponse::<Option<()>> {
        code: error.code(),
        message: error.to_string(),
        data: None,
    });

    ServiceResponse::new(req.into_parts().0, response).map_into_boxed_body()
}
