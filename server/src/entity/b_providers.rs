//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_providers"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 名称或备注
    pub name: String,
    /// 上游服务基础地址
    pub base_url: String,
    /// AES-GCM 加密后的上游令牌
    pub access_token: String,
    /// 供应商类型：minimax / glm / codex
    pub provider_type: String,
    /// 5 小时窗口剩余额度
    pub quota_5h_remaining: Option<i64>,
    /// 5 小时额度刷新时间
    pub quota_5h_refreshed_at: Option<DateTime>,
    /// 周额度剩余值
    pub quota_weekly_remaining: Option<i64>,
    /// 周额度刷新时间
    pub quota_weekly_refreshed_at: Option<DateTime>,
    /// 月额度剩余值（预留）
    pub quota_monthly_remaining: Option<i64>,
    /// 月额度刷新时间（预留）
    pub quota_monthly_refreshed_at: Option<DateTime>,
    /// 禁用时间；空表示启用
    pub disabled_at: Option<DateTime>,
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
    Name,
    BaseUrl,
    AccessToken,
    ProviderType,
    #[sea_orm(column_name = "quota_5h_remaining")]
    Quota5hRemaining,
    #[sea_orm(column_name = "quota_5h_refreshed_at")]
    Quota5hRefreshedAt,
    QuotaWeeklyRemaining,
    QuotaWeeklyRefreshedAt,
    QuotaMonthlyRemaining,
    QuotaMonthlyRefreshedAt,
    DisabledAt,
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
    BRequestLogs,
    BUsers2,
    BUsers1,
    RModelProviders,
    RProviderPermissions,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def().unique(),
            Self::BaseUrl => ColumnType::String(StringLen::N(512u32)).def(),
            Self::AccessToken => ColumnType::Text.def(),
            Self::ProviderType => ColumnType::String(StringLen::N(32u32)).def(),
            Self::Quota5hRemaining => ColumnType::BigInteger.def().null(),
            Self::Quota5hRefreshedAt => ColumnType::DateTime.def().null(),
            Self::QuotaWeeklyRemaining => ColumnType::BigInteger.def().null(),
            Self::QuotaWeeklyRefreshedAt => ColumnType::DateTime.def().null(),
            Self::QuotaMonthlyRemaining => ColumnType::BigInteger.def().null(),
            Self::QuotaMonthlyRefreshedAt => ColumnType::DateTime.def().null(),
            Self::DisabledAt => ColumnType::DateTime.def().null(),
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
            Self::BRequestLogs => Entity::has_many(super::b_request_logs::Entity).into(),
            Self::BUsers2 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::CreatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BUsers1 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UpdatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::RModelProviders => Entity::has_many(super::r_model_providers::Entity).into(),
            Self::RProviderPermissions => {
                Entity::has_many(super::r_provider_permissions::Entity).into()
            }
        }
    }
}

impl Related<super::b_request_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BRequestLogs.def()
    }
}

impl Related<super::r_model_providers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RModelProviders.def()
    }
}

impl Related<super::r_provider_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RProviderPermissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
