//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_users"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 登录邮箱
    pub email: String,
    /// 名称或备注
    pub name: String,
    /// 加盐口令哈希
    pub password: String,
    /// 口令随机盐
    pub password_salt: String,
    /// 是否管理员：0 否，1 是
    pub is_admin: i8,
    /// 封禁开始时间
    pub banned_at: Option<DateTime>,
    /// 封禁结束时间；空表示永久封禁
    pub unban_at: Option<DateTime>,
    /// 当前登录会话 JWT
    pub session_token: Option<String>,
    /// 当前登录会话过期时间
    pub session_expire_at: Option<DateTime>,
    /// 创建人 ID
    pub created_by: Option<u64>,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Email,
    Name,
    Password,
    PasswordSalt,
    IsAdmin,
    BannedAt,
    UnbanAt,
    SessionToken,
    SessionExpireAt,
    CreatedBy,
    CreatedAt,
}

#[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
pub enum PrimaryKey {
    Id,
}

impl PrimaryKeyTrait for PrimaryKey {
    type ValueType = u64;
    fn auto_increment() -> bool {
        true
    }
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    BAccessTokens,
    BEnvironments,
    BExecutions,
    BProjects,
    BRequestLogs,
    BSandboxes,
    BSystemSettings,
    BUserOperationLogs,
    SelfRef,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::Email => ColumnType::String(StringLen::N(255u32)).def().unique(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def(),
            Self::Password => ColumnType::Char(Some(40u32)).def(),
            Self::PasswordSalt => ColumnType::Char(Some(32u32)).def(),
            Self::IsAdmin => ColumnType::TinyInteger.def(),
            Self::BannedAt => ColumnType::DateTime.def().null(),
            Self::UnbanAt => ColumnType::DateTime.def().null(),
            Self::SessionToken => ColumnType::Text.def().null(),
            Self::SessionExpireAt => ColumnType::DateTime.def().null(),
            Self::CreatedBy => ColumnType::BigUnsigned.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BAccessTokens => Entity::has_many(super::b_access_tokens::Entity).into(),
            Self::BEnvironments => Entity::has_many(super::b_environments::Entity).into(),
            Self::BExecutions => Entity::has_many(super::b_executions::Entity).into(),
            Self::BProjects => Entity::has_many(super::b_projects::Entity).into(),
            Self::BRequestLogs => Entity::has_many(super::b_request_logs::Entity).into(),
            Self::BSandboxes => Entity::has_many(super::b_sandboxes::Entity).into(),
            Self::BSystemSettings => Entity::has_many(super::b_system_settings::Entity).into(),
            Self::BUserOperationLogs => {
                Entity::has_many(super::b_user_operation_logs::Entity).into()
            }
            Self::SelfRef => Entity::belongs_to(Entity)
                .from(Column::CreatedBy)
                .to(Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_access_tokens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BAccessTokens.def()
    }
}

impl Related<super::b_environments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BEnvironments.def()
    }
}

impl Related<super::b_executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BExecutions.def()
    }
}

impl Related<super::b_projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BProjects.def()
    }
}

impl Related<super::b_request_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BRequestLogs.def()
    }
}

impl Related<super::b_sandboxes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BSandboxes.def()
    }
}

impl Related<super::b_system_settings::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BSystemSettings.def()
    }
}

impl Related<super::b_user_operation_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BUserOperationLogs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter,
    QueryOrder, Set,
};

use super::b_user_operation_logs::{self as logs};
use crate::error::AppError;
use crate::utils::crypto::{generate_salt, hash_password};

/// 按邮箱查找用户。
pub async fn find_by_email(
    conn: &DatabaseConnection,
    email: &str,
) -> Result<Option<Model>, AppError> {
    Entity::find()
        .filter(Column::Email.eq(email))
        .one(conn)
        .await
        .map_err(AppError::internal)
}

/// 按用户 ID 查找用户。
pub async fn find_by_id(conn: &DatabaseConnection, id: u64) -> Result<Option<Model>, AppError> {
    Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::internal)
}

/// 覆盖单会话令牌；token 为空表示退出登录。
pub async fn update_session(
    conn: &DatabaseConnection,
    user_id: u64,
    token: Option<&str>,
    expires_at: Option<chrono::NaiveDateTime>,
) -> Result<(), AppError> {
    let mut active: ActiveModel = Entity::find_by_id(user_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("user does not exist".into()))?
        .into();
    active.session_token = Set(token.map(str::to_string));
    active.session_expire_at = Set(expires_at);
    active.update(conn).await.map_err(AppError::internal)?;

    Ok(())
}

/// 写入用户操作审计日志。
pub async fn insert_log(
    conn: &DatabaseConnection,
    user_id: u64,
    kind: &str,
    status: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    let log = logs::ActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        r#type: Set(kind.to_string()),
        status: Set(status.to_string()),
        description: Set(description.map(str::to_string)),
        created_at: Set(Utc::now().naive_utc()),
    };
    log.insert(conn).await.map_err(AppError::internal)?;

    Ok(())
}

/// 统计 5 分钟窗口内的登录失败次数。
pub async fn count_recent_failed_logins(
    conn: &DatabaseConnection,
    user_id: u64,
) -> Result<u64, AppError> {
    let since = Utc::now().naive_utc() - chrono::Duration::minutes(5);
    let count = logs::Entity::find()
        .filter(logs::Column::UserId.eq(user_id))
        .filter(logs::Column::Type.eq("login"))
        .filter(logs::Column::Status.eq("failed"))
        .filter(logs::Column::CreatedAt.gte(since))
        .count(conn)
        .await
        .map_err(AppError::internal)?;

    Ok(count)
}

/// 查询用户最近成功登录时间。
pub async fn last_login_at(
    conn: &DatabaseConnection,
    user_id: u64,
) -> Result<Option<chrono::NaiveDateTime>, AppError> {
    let log = logs::Entity::find()
        .filter(logs::Column::UserId.eq(user_id))
        .filter(logs::Column::Type.eq("login"))
        .filter(logs::Column::Status.eq("success"))
        .order_by_desc(logs::Column::CreatedAt)
        .one(conn)
        .await
        .map_err(AppError::internal)?;

    Ok(log.map(|log| log.created_at))
}

/// 仅在空库创建初始管理员。
pub async fn create_initial_admin(
    conn: &DatabaseConnection,
    email: &str,
    name: &str,
    password_sha1: &str,
) -> Result<Model, AppError> {
    if Entity::find()
        .count(conn)
        .await
        .map_err(AppError::internal)?
        != 0
    {
        return Err(AppError::InvalidArgument(
            "initial admin can only be created when no users exist".into(),
        ));
    }

    let salt = generate_salt();
    let active = ActiveModel {
        id: NotSet,
        email: Set(email.to_string()),
        name: Set(name.to_string()),
        password: Set(hash_password(&salt, password_sha1)),
        password_salt: Set(salt),
        is_admin: Set(1),
        banned_at: Set(None),
        unban_at: Set(None),
        session_token: Set(None),
        session_expire_at: Set(None),
        created_by: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    };
    active.insert(conn).await.map_err(AppError::internal)
}
