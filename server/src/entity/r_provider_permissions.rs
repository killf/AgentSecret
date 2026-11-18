//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "r_provider_permissions"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 所属用户 ID
    pub user_id: u64,
    /// 供应商 ID
    pub provider_id: Option<u64>,
    /// 日 token 限额；空表示不限
    pub daily_token_limit: Option<i64>,
    /// 当前窗口日 token 已用量
    pub daily_token_used: u64,
    /// 日额度窗口刷新时间
    pub daily_token_refreshed_at: Option<DateTime>,
    /// 周 token 限额；空表示不限
    pub weekly_token_limit: Option<i64>,
    /// 当前窗口周 token 已用量
    pub weekly_token_used: u64,
    /// 周额度窗口刷新时间
    pub weekly_token_refreshed_at: Option<DateTime>,
    /// 月 token 限额；空表示不限
    pub monthly_token_limit: Option<i64>,
    /// 当前窗口月 token 已用量
    pub monthly_token_used: u64,
    /// 月额度窗口刷新时间
    pub monthly_token_refreshed_at: Option<DateTime>,
    /// 创建人 ID
    pub created_by: Option<u64>,
    /// 创建时间
    pub created_at: DateTime,
    /// 更新人 ID
    pub updated_by: Option<u64>,
    /// 更新时间
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    UserId,
    ProviderId,
    DailyTokenLimit,
    DailyTokenUsed,
    DailyTokenRefreshedAt,
    WeeklyTokenLimit,
    WeeklyTokenUsed,
    WeeklyTokenRefreshedAt,
    MonthlyTokenLimit,
    MonthlyTokenUsed,
    MonthlyTokenRefreshedAt,
    CreatedBy,
    CreatedAt,
    UpdatedBy,
    UpdatedAt,
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
    BProviders,
    BUsers3,
    BUsers2,
    BUsers1,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::UserId => ColumnType::BigUnsigned.def(),
            Self::ProviderId => ColumnType::BigUnsigned.def().null(),
            Self::DailyTokenLimit => ColumnType::BigInteger.def().null(),
            Self::DailyTokenUsed => ColumnType::BigUnsigned.def(),
            Self::DailyTokenRefreshedAt => ColumnType::DateTime.def().null(),
            Self::WeeklyTokenLimit => ColumnType::BigInteger.def().null(),
            Self::WeeklyTokenUsed => ColumnType::BigUnsigned.def(),
            Self::WeeklyTokenRefreshedAt => ColumnType::DateTime.def().null(),
            Self::MonthlyTokenLimit => ColumnType::BigInteger.def().null(),
            Self::MonthlyTokenUsed => ColumnType::BigUnsigned.def(),
            Self::MonthlyTokenRefreshedAt => ColumnType::DateTime.def().null(),
            Self::CreatedBy => ColumnType::BigUnsigned.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedBy => ColumnType::BigUnsigned.def().null(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BProviders => Entity::belongs_to(super::b_providers::Entity)
                .from(Column::ProviderId)
                .to(super::b_providers::Column::Id)
                .into(),
            Self::BUsers3 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::CreatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BUsers2 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UpdatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BUsers1 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UserId)
                .to(super::b_users::Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_providers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BProviders.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
