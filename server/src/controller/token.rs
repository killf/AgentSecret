use actix_web::{HttpResponse, web};
use chrono::Utc;

use crate::entity::b_access_tokens::{self as tokens};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::api_ok;
use crate::schema::token::{
    CreateTokenRequest, ResetTokenResponse, TokenDetail, TokenIdRequest, TokenInfo,
};
use crate::state::AppState;

/// 当前用户令牌列表。
pub async fn list(user: AuthUser, state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    let rows = tokens::list_by_user(&state.conn, user.id).await?;
    let items = rows
        .into_iter()
        .map(|token| TokenInfo {
            id: token.id,
            name: token.name,
            expires_at: token.expires_at.map(|value| value.and_utc()),
            disabled_at: token.disabled_at.map(|value| value.and_utc()),
            created_at: token.created_at.and_utc(),
        })
        .collect::<Vec<_>>();

    Ok(api_ok(items))
}

/// 令牌详情，仅在本人访问时返回明文。
pub async fn detail(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<TokenIdRequest>,
) -> Result<HttpResponse, AppError> {
    let token = owned_token(&state.conn, user.id, body.id).await?;
    let detail = TokenDetail {
        info: TokenInfo {
            id: token.id,
            name: token.name,
            expires_at: token.expires_at.map(|value| value.and_utc()),
            disabled_at: token.disabled_at.map(|value| value.and_utc()),
            created_at: token.created_at.and_utc(),
        },
        token: token.token,
    };

    Ok(api_ok(detail))
}

/// 创建访问令牌。
pub async fn add(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<CreateTokenRequest>,
) -> Result<HttpResponse, AppError> {
    validate_name(body.name.as_deref())?;
    let expires_at = body.expires_at.map(|value| value.naive_utc());
    if expires_at.is_some_and(|expires_at| expires_at <= Utc::now().naive_utc()) {
        return Err(AppError::InvalidArgument(
            "expires_at must be in the future".into(),
        ));
    }

    let token = tokens::create(
        &state.conn,
        user.id,
        body.name.as_deref(),
        expires_at,
        &state.tokens,
    )
    .await?;
    users_log(&state, user.id, "token_create", token.id).await?;
    let detail = TokenDetail {
        info: TokenInfo {
            id: token.id,
            name: token.name,
            expires_at: token.expires_at.map(|value| value.and_utc()),
            disabled_at: token.disabled_at.map(|value| value.and_utc()),
            created_at: token.created_at.and_utc(),
        },
        token: token.token,
    };

    Ok(api_ok(detail))
}

/// 禁用令牌。
pub async fn disable(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<TokenIdRequest>,
) -> Result<HttpResponse, AppError> {
    let token = owned_token(&state.conn, user.id, body.id).await?;
    tokens::set_disabled(&state.conn, token.id, Some(Utc::now().naive_utc())).await?;

    Ok(api_ok(Option::<()>::None))
}

/// 启用未过期令牌。
pub async fn enable(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<TokenIdRequest>,
) -> Result<HttpResponse, AppError> {
    let token = owned_token(&state.conn, user.id, body.id).await?;
    if token
        .expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now().naive_utc())
    {
        return Err(AppError::InvalidArgument("token has expired".into()));
    }
    tokens::set_disabled(&state.conn, token.id, None).await?;

    Ok(api_ok(Option::<()>::None))
}

/// 逻辑删除令牌。
pub async fn delete(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<TokenIdRequest>,
) -> Result<HttpResponse, AppError> {
    let token = owned_token(&state.conn, user.id, body.id).await?;
    tokens::set_disabled(&state.conn, token.id, Some(Utc::now().naive_utc())).await?;
    users_log(&state, user.id, "token_delete", token.id).await?;

    Ok(api_ok(Option::<()>::None))
}

/// 重置令牌明文，旧令牌立即失效。
pub async fn reset(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<TokenIdRequest>,
) -> Result<HttpResponse, AppError> {
    let old = owned_token(&state.conn, user.id, body.id).await?;
    let token = tokens::reset(&state.conn, old.id, &state.tokens).await?;
    users_log(&state, user.id, "token_reset", token.id).await?;

    Ok(api_ok(ResetTokenResponse {
        token: token.token,
        expires_at: token.expires_at.map(|value| value.and_utc()),
    }))
}

async fn owned_token(
    conn: &sea_orm::DatabaseConnection,
    user_id: u64,
    token_id: u64,
) -> Result<tokens::Model, AppError> {
    if token_id == 0 {
        return Err(AppError::InvalidArgument(
            "id must be greater than zero".into(),
        ));
    }
    let token = tokens::find_by_id(conn, token_id)
        .await?
        .ok_or_else(|| AppError::NotFound("token does not exist".into()))?;
    if token.user_id != user_id {
        return Err(AppError::Forbidden(
            "token does not belong to current user".into(),
        ));
    }

    Ok(token)
}

async fn users_log(
    state: &web::Data<AppState>,
    user_id: u64,
    kind: &str,
    token_id: u64,
) -> Result<(), AppError> {
    crate::entity::b_users::insert_log(
        &state.conn,
        user_id,
        kind,
        "success",
        Some(&token_id.to_string()),
    )
    .await
}

fn validate_name(name: Option<&str>) -> Result<(), AppError> {
    if name.is_some_and(|name| name.len() > 128) {
        Err(AppError::InvalidArgument(
            "name must contain at most 128 characters".into(),
        ))
    } else {
        Ok(())
    }
}
