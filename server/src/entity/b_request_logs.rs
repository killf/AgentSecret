//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_request_logs"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 所属用户 ID
    pub user_id: u64,
    /// 模型 ID
    pub model_id: u64,
    /// 供应商 ID
    pub provider_id: u64,
    /// token_id 字段
    pub token_id: Option<u64>,
    /// 输入 token 数
    pub input_tokens: u32,
    /// 输出 token 数
    pub output_tokens: u32,
    /// 缓存 token 数
    pub cache_tokens: u32,
    /// 首字响应耗时（毫秒）
    pub ttft_ms: u32,
    /// 请求总耗时（毫秒）
    pub duration_ms: u32,
    /// 是否命中敏感信息规则
    pub contains_sensitive: i8,
    /// 脱敏后的请求详情
    pub request_detail: Option<Json>,
    /// 脱敏后的响应详情
    pub response_detail: Option<Json>,
    /// 请求状态：success / failed / timeout / rejected
    pub request_status: String,
    /// 失败原因
    pub failure_reason: Option<String>,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    UserId,
    ModelId,
    ProviderId,
    TokenId,
    InputTokens,
    OutputTokens,
    CacheTokens,
    TtftMs,
    DurationMs,
    ContainsSensitive,
    RequestDetail,
    ResponseDetail,
    RequestStatus,
    FailureReason,
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
    BModels,
    BProviders,
    BUsers,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::UserId => ColumnType::BigUnsigned.def(),
            Self::ModelId => ColumnType::BigUnsigned.def(),
            Self::ProviderId => ColumnType::BigUnsigned.def(),
            Self::TokenId => ColumnType::BigUnsigned.def().null(),
            Self::InputTokens => ColumnType::Unsigned.def(),
            Self::OutputTokens => ColumnType::Unsigned.def(),
            Self::CacheTokens => ColumnType::Unsigned.def(),
            Self::TtftMs => ColumnType::Unsigned.def(),
            Self::DurationMs => ColumnType::Unsigned.def(),
            Self::ContainsSensitive => ColumnType::TinyInteger.def(),
            Self::RequestDetail => ColumnType::Json.def().null(),
            Self::ResponseDetail => ColumnType::Json.def().null(),
            Self::RequestStatus => ColumnType::String(StringLen::N(16u32)).def(),
            Self::FailureReason => ColumnType::Text.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BAccessTokens => Entity::belongs_to(super::b_access_tokens::Entity)
                .from(Column::TokenId)
                .to(super::b_access_tokens::Column::Id)
                .into(),
            Self::BModels => Entity::belongs_to(super::b_models::Entity)
                .from(Column::ModelId)
                .to(super::b_models::Column::Id)
                .into(),
            Self::BProviders => Entity::belongs_to(super::b_providers::Entity)
                .from(Column::ProviderId)
                .to(super::b_providers::Column::Id)
                .into(),
            Self::BUsers => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UserId)
                .to(super::b_users::Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_access_tokens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BAccessTokens.def()
    }
}

impl Related<super::b_models::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BModels.def()
    }
}

impl Related<super::b_providers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BProviders.def()
    }
}

impl Related<super::b_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BUsers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
