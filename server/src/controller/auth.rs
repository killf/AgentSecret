use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};

use crate::entity::b_users::{self as users};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::auth::{ChangePasswordRequest, LoginRequest, LoginResponse, UserInfo};
use crate::schema::{ApiResponse, api_ok};
use crate::state::AppState;
use crate::utils::crypto::{generate_salt, hash_password, verify_password};

/// 用户登录并签发单会话 JWT。
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    validate_email(&body.email)?;
    validate_password(&body.password)?;

    // 1. 查找用户；不存在与密码错误使用同一错误，避免账号枚举。
    let Some(user) = users::find_by_email(&state.conn, &body.email).await? else {
        return Err(AppError::Unauthenticated(
            "email or password is incorrect".into(),
        ));
    };
    let now = Utc::now();
    if is_banned(&user, now) {
        return Err(AppError::LimitExceeded("user is banned".into()));
    }

    // 2. 口令错误则审计并统计近 5 分钟失败数，达到 5 次封禁 30 分钟。
    if !verify_password(&user.password_salt, &body.password, &user.password) {
        users::insert_log(
            &state.conn,
            user.id,
            "login",
            "failed",
            Some("email or password mismatch"),
        )
        .await?;
        let failures = users::count_recent_failed_logins(&state.conn, user.id).await?;
        if failures >= 5 {
            let mut active: users::ActiveModel = user.into();
            active.banned_at = Set(Some(now.naive_utc()));
            active.unban_at = Set(Some((now + chrono::Duration::minutes(30)).naive_utc()));
            active.session_token = Set(None);
            active.session_expire_at = Set(None);
            active
                .update(&state.conn)
                .await
                .map_err(AppError::internal)?;
            return Err(AppError::LimitExceeded("user is banned".into()));
        }

        return Err(AppError::Unauthenticated(
            "email or password is incorrect".into(),
        ));
    }

    // 3. 签发新会话并覆盖旧 token。
    let (token, expires_at) = state.tokens.issue_session(
        user.id,
        user.is_admin != 0,
        now,
        state.config.token_ttl_days,
    )?;
    users::update_session(
        &state.conn,
        user.id,
        Some(&token),
        Some(expires_at.naive_utc()),
    )
    .await?;
    users::insert_log(&state.conn, user.id, "login", "success", None).await?;

    Ok(api_ok(LoginResponse {
        token,
        expires_at,
        user: UserInfo {
            id: user.id,
            email: user.email,
            name: user.name,
            is_admin: user.is_admin != 0,
        },
    }))
}

/// 退出当前会话。
pub async fn logout(user: AuthUser, state: web::Data<AppState>) -> Result<HttpResponse, AppError> {
    users::update_session(&state.conn, user.id, None, None).await?;
    users::insert_log(&state.conn, user.id, "logout", "success", None).await?;

    Ok(api_ok(Option::<()>::None))
}

/// 修改自己的密码并强制重新登录。
pub async fn change_password(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, AppError> {
    validate_password(&body.old_password)?;
    validate_password(&body.new_password)?;

    // 1. 校验旧口令。
    let model = users::find_by_id(&state.conn, user.id)
        .await?
        .ok_or_else(|| AppError::Unauthenticated("session token is invalid".into()))?;
    if !verify_password(&model.password_salt, &body.old_password, &model.password) {
        return Err(AppError::InvalidArgument(
            "old password is incorrect".into(),
        ));
    }

    // 2. 换盐写入口令并清空会话。
    let salt = generate_salt();
    let mut active: users::ActiveModel = model.into();
    active.password_salt = Set(salt.clone());
    active.password = Set(hash_password(&salt, &body.new_password));
    active.session_token = Set(None);
    active.session_expire_at = Set(None);
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;
    users::insert_log(&state.conn, user.id, "password_change", "success", None).await?;

    Ok(api_ok(Option::<()>::None))
}

/// 返回当前登录用户。
pub async fn me(user: AuthUser) -> HttpResponse {
    api_ok(UserInfo {
        id: user.id,
        email: user.email,
        name: user.name,
        is_admin: user.is_admin,
    })
}

fn is_banned(user: &users::Model, now: chrono::DateTime<Utc>) -> bool {
    match (user.banned_at, user.unban_at) {
        (Some(banned_at), None) => now.naive_utc() >= banned_at,
        (Some(banned_at), Some(unban_at)) => {
            now.naive_utc() >= banned_at && now.naive_utc() < unban_at
        }
        _ => false,
    }
}

fn validate_email(email: &str) -> Result<(), AppError> {
    let valid = email.len() <= 255
        && email.matches('@').count() == 1
        && !email.starts_with('@')
        && !email.ends_with('@');
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidArgument("email is invalid".into()))
    }
}

fn validate_password(password: &str) -> Result<(), AppError> {
    let valid = password.len() == 40 && password.bytes().all(|byte| byte.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "password must be a 40-character SHA-1 value".into(),
        ))
    }
}

#[allow(dead_code)]
fn _response_type_is_used(_: ApiResponse<()>) {}
