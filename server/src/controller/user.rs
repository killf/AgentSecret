use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, NotSet, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use crate::entity::b_users::{self as users};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::admin::{CreatedResponse, IdRequest};
use crate::schema::api_ok;
use crate::schema::user::{
    AdminUserInfo, BanUserRequest, CreateUserRequest, ListUsersRequest, ResetUserPasswordRequest,
    UpdateUserRequest, UserPageResponse,
};
use crate::state::AppState;
use crate::utils::crypto::{generate_salt, hash_password};

/// 分页查询用户，并补充最近成功登录时间。
pub async fn list(
    state: web::Data<AppState>,
    body: web::Json<ListUsersRequest>,
) -> Result<HttpResponse, AppError> {
    let page = body.page.unwrap_or(1);
    let page_size = body.page_size.unwrap_or(20);
    if page == 0 || page_size == 0 || page_size > 100 {
        return Err(AppError::InvalidArgument(
            "page must be >= 1 and page_size must be between 1 and 100".into(),
        ));
    }
    let keyword = body.keyword.as_deref();
    if keyword.is_some_and(|keyword| keyword.len() > 255) {
        return Err(AppError::InvalidArgument("keyword is too long".into()));
    }

    // 1. 构造邮箱 / 名称模糊查询。
    let mut query = users::Entity::find().order_by_asc(users::Column::Id);
    if let Some(keyword) = keyword {
        let pattern = format!("%{}%", keyword);
        query = query.filter(
            Condition::any()
                .add(users::Column::Email.like(pattern.clone()))
                .add(users::Column::Name.like(pattern)),
        );
    }
    let total = query
        .clone()
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let rows = query
        .limit(page_size)
        .offset((page - 1).saturating_mul(page_size))
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;

    // 2. 逐个补充最近登录时间；当前用户量下保持查询简单清晰。
    let mut items = Vec::with_capacity(rows.len());
    for user in rows {
        let last_login_at = users::last_login_at(&state.conn, user.id).await?;
        items.push(AdminUserInfo {
            id: user.id,
            email: user.email,
            name: user.name,
            is_admin: user.is_admin != 0,
            banned_at: user.banned_at.map(|value| value.and_utc()),
            unban_at: user.unban_at.map(|value| value.and_utc()),
            last_login_at: last_login_at.map(|value| value.and_utc()),
            created_at: user.created_at.and_utc(),
        });
    }

    Ok(api_ok(UserPageResponse { total, items }))
}

/// 创建普通用户。
pub async fn add(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, AppError> {
    validate_email(&body.email)?;
    validate_name(&body.name)?;
    validate_password(&body.password)?;
    let exists = users::Entity::find()
        .filter(users::Column::Email.eq(&body.email))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if exists > 0 {
        return Err(AppError::InvalidArgument("email already exists".into()));
    }

    // 1. 随机盐 + 前端 SHA-1 生成存储摘要。
    let salt = generate_salt();
    let active = users::ActiveModel {
        id: NotSet,
        email: Set(body.email.clone()),
        name: Set(body.name.clone()),
        password: Set(hash_password(&salt, &body.password)),
        password_salt: Set(salt),
        is_admin: Set(0),
        banned_at: Set(None),
        unban_at: Set(None),
        session_token: Set(None),
        session_expire_at: Set(None),
        created_by: Set(Some(actor.id)),
        created_at: Set(Utc::now().naive_utc()),
    };
    let user = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;
    users::insert_log(
        &state.conn,
        actor.id,
        "user_create",
        "success",
        Some(&user.id.to_string()),
    )
    .await?;

    Ok(api_ok(CreatedResponse { id: user.id }))
}

/// 更新用户显示名或管理员标记。
pub async fn update(
    state: web::Data<AppState>,
    body: web::Json<UpdateUserRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    if let Some(name) = body.name.as_deref() {
        validate_name(name)?;
    }
    let user = require_user(&state.conn, body.id).await?;

    // 1. 最后一个管理员不能降级。
    if user.is_admin != 0 && body.is_admin == Some(false) {
        let other_admins = users::Entity::find()
            .filter(users::Column::IsAdmin.eq(1))
            .filter(users::Column::Id.ne(body.id))
            .count(&state.conn)
            .await
            .map_err(AppError::internal)?;
        if other_admins == 0 {
            return Err(AppError::InvalidArgument(
                "the last administrator cannot be downgraded".into(),
            ));
        }
    }

    let mut active: users::ActiveModel = user.into();
    if let Some(name) = &body.name {
        active.name = Set(name.clone());
    }
    if let Some(is_admin) = body.is_admin {
        active.is_admin = Set(is_admin as i8);
    }
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 删除未被业务引用的用户。
pub async fn delete(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    require_user(&state.conn, body.id).await?;
    if user_in_use(&state.conn, body.id).await? {
        return Err(AppError::NotFound("user is still referenced".into()));
    }

    users::Entity::delete_by_id(body.id)
        .exec(&state.conn)
        .await
        .map_err(AppError::internal)?;
    users::insert_log(
        &state.conn,
        actor.id,
        "user_delete",
        "success",
        Some(&body.id.to_string()),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 封禁用户并立即下线。
pub async fn ban(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<BanUserRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    let user = require_user(&state.conn, body.id).await?;
    let now = Utc::now().naive_utc();
    let unban_at = if body.permanent.unwrap_or(false) {
        None
    } else {
        let minutes = body.minutes.unwrap_or(30);
        if minutes <= 0 || minutes > 525_600 {
            return Err(AppError::InvalidArgument(
                "minutes must be between 1 and 525600".into(),
            ));
        }
        Some((Utc::now() + chrono::Duration::minutes(minutes)).naive_utc())
    };

    let mut active: users::ActiveModel = user.into();
    active.banned_at = Set(Some(now));
    active.unban_at = Set(unban_at);
    active.session_token = Set(None);
    active.session_expire_at = Set(None);
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;
    users::insert_log(
        &state.conn,
        actor.id,
        "user_ban",
        "success",
        Some(&body.id.to_string()),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 解禁用户。
pub async fn unban(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    let user = require_user(&state.conn, body.id).await?;
    let mut active: users::ActiveModel = user.into();
    active.banned_at = Set(None);
    active.unban_at = Set(None);
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;
    users::insert_log(
        &state.conn,
        actor.id,
        "user_unban",
        "success",
        Some(&body.id.to_string()),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

/// 管理员重置用户密码并强制重新登录。
pub async fn reset_password(
    actor: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<ResetUserPasswordRequest>,
) -> Result<HttpResponse, AppError> {
    validate_id(body.id)?;
    validate_password(&body.new_password)?;
    let user = require_user(&state.conn, body.id).await?;
    let salt = generate_salt();
    let mut active: users::ActiveModel = user.into();
    active.password_salt = Set(salt.clone());
    active.password = Set(hash_password(&salt, &body.new_password));
    active.session_token = Set(None);
    active.session_expire_at = Set(None);
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;
    users::insert_log(
        &state.conn,
        actor.id,
        "password_reset",
        "success",
        Some(&body.id.to_string()),
    )
    .await?;

    Ok(api_ok(Option::<()>::None))
}

async fn require_user(
    conn: &sea_orm::DatabaseConnection,
    id: u64,
) -> Result<users::Model, AppError> {
    users::Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("user does not exist".into()))
}

async fn user_in_use(conn: &sea_orm::DatabaseConnection, id: u64) -> Result<bool, AppError> {
    use crate::entity::{
        b_access_tokens, b_executions, b_projects, b_request_logs, b_scripts,
        r_environment_permissions, r_model_permissions, r_project_permissions,
        r_provider_permissions,
    };

    // 1. 任一业务引用存在即禁止物理删除。
    let checks = [
        b_access_tokens::Entity::find()
            .filter(b_access_tokens::Column::UserId.eq(id))
            .count(conn),
        crate::entity::b_user_operation_logs::Entity::find()
            .filter(crate::entity::b_user_operation_logs::Column::UserId.eq(id))
            .count(conn),
        r_model_permissions::Entity::find()
            .filter(r_model_permissions::Column::UserId.eq(id))
            .count(conn),
        r_provider_permissions::Entity::find()
            .filter(r_provider_permissions::Column::UserId.eq(id))
            .count(conn),
        r_project_permissions::Entity::find()
            .filter(r_project_permissions::Column::UserId.eq(id))
            .count(conn),
        r_environment_permissions::Entity::find()
            .filter(r_environment_permissions::Column::UserId.eq(id))
            .count(conn),
        b_projects::Entity::find()
            .filter(b_projects::Column::CreatedBy.eq(id))
            .count(conn),
        b_scripts::Entity::find()
            .filter(b_scripts::Column::CreatedBy.eq(id))
            .count(conn),
        b_executions::Entity::find()
            .filter(b_executions::Column::UserId.eq(id))
            .count(conn),
        b_request_logs::Entity::find()
            .filter(b_request_logs::Column::UserId.eq(id))
            .count(conn),
    ];
    for check in checks {
        if check.await.map_err(AppError::internal)? > 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

fn validate_id(id: u64) -> Result<(), AppError> {
    if id == 0 {
        Err(AppError::InvalidArgument(
            "id must be greater than zero".into(),
        ))
    } else {
        Ok(())
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

fn validate_name(name: &str) -> Result<(), AppError> {
    if !name.trim().is_empty() && name == name.trim() && name.len() <= 128 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "name must contain 1 to 128 characters without surrounding spaces".into(),
        ))
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
