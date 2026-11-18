//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_access_tokens"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 所属用户 ID
    pub user_id: u64,
    /// 名称或备注
    pub name: Option<String>,
    /// JWT 明文
    pub token: String,
    /// 过期时间；空表示不过期
    pub expires_at: Option<DateTime>,
    /// 禁用时间；空表示启用
    pub disabled_at: Option<DateTime>,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    UserId,
    Name,
    Token,
    ExpiresAt,
    DisabledAt,
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
    BRequestLogs,
    BUsers,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::UserId => ColumnType::BigUnsigned.def(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def().null(),
            Self::Token => ColumnType::String(StringLen::N(512u32)).def().unique(),
            Self::ExpiresAt => ColumnType::DateTime.def().null(),
            Self::DisabledAt => ColumnType::DateTime.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BRequestLogs => Entity::has_many(super::b_request_logs::Entity).into(),
            Self::BUsers => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UserId)
                .to(super::b_users::Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_request_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BRequestLogs.def()
    }
}

impl Related<super::b_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BUsers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter,
    QueryOrder, Set,
};

use super::b_users;
use crate::error::AppError;
use crate::utils::{crypto::random_id, jwt::TokenManager};

/// 列出用户访问令牌，不返回明文额外处理，由 schema 决定是否输出。
pub async fn list_by_user(conn: &DatabaseConnection, user_id: u64) -> Result<Vec<Model>, AppError> {
    Entity::find()
        .filter(Column::UserId.eq(user_id))
        .order_by_desc(Column::CreatedAt)
        .all(conn)
        .await
        .map_err(AppError::internal)
}

/// 按 ID 查找访问令牌。
pub async fn find_by_id(conn: &DatabaseConnection, id: u64) -> Result<Option<Model>, AppError> {
    Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::internal)
}

/// 创建访问令牌，JWT 只在创建/重置结果中返回。
pub async fn create(
    conn: &DatabaseConnection,
    user_id: u64,
    name: Option<&str>,
    expires_at: Option<chrono::NaiveDateTime>,
    manager: &TokenManager,
) -> Result<Model, AppError> {
    let token = manager.issue_access(
        user_id,
        random_id(),
        Utc::now(),
        expires_at.map(|value| value.and_utc()),
    )?;
    let active = ActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        name: Set(name.map(str::to_string)),
        token: Set(token),
        expires_at: Set(expires_at),
        disabled_at: Set(None),
        created_at: Set(Utc::now().naive_utc()),
    };
    active.insert(conn).await.map_err(AppError::internal)
}

/// 更新启停状态；disabled_at 为空表示启用。
pub async fn set_disabled(
    conn: &DatabaseConnection,
    id: u64,
    disabled_at: Option<chrono::NaiveDateTime>,
) -> Result<(), AppError> {
    let mut active: ActiveModel = Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("token does not exist".into()))?
        .into();
    active.disabled_at = Set(disabled_at);
    active.update(conn).await.map_err(AppError::internal)?;

    Ok(())
}

/// 重置访问令牌并保留原过期时间。
pub async fn reset(
    conn: &DatabaseConnection,
    id: u64,
    manager: &TokenManager,
) -> Result<Model, AppError> {
    let model = Entity::find_by_id(id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("token does not exist".into()))?;
    let token = manager.issue_access(
        model.user_id,
        random_id(),
        Utc::now(),
        model.expires_at.map(|value| value.and_utc()),
    )?;
    let mut active: ActiveModel = model.into();
    active.token = Set(token);
    active.update(conn).await.map_err(AppError::internal)
}

/// 确认访问令牌所属用户，并校验未禁用、未过期。
pub async fn validate(
    conn: &DatabaseConnection,
    token: &str,
    manager: &TokenManager,
) -> Result<b_users::Model, AppError> {
    let claims = manager.verify_access(token)?;
    let user_id = claims
        .sub
        .parse::<u64>()
        .map_err(|_| AppError::Unauthenticated("access token is invalid".into()))?;
    let model = Entity::find()
        .filter(Column::Token.eq(token))
        .filter(Column::DisabledAt.is_null())
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::Unauthenticated("access token is invalid".into()))?;
    let user = b_users::find_by_id(conn, user_id)
        .await?
        .ok_or_else(|| AppError::Unauthenticated("access token is invalid".into()))?;
    let now = Utc::now().naive_utc();
    if model.expires_at.is_some_and(|expires_at| expires_at <= now) || user.banned_at.is_some() {
        return Err(AppError::Unauthenticated("access token is invalid".into()));
    }

    Ok(user)
}
