//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "r_model_providers"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 模型 ID
    pub model_id: u64,
    /// 供应商 ID
    pub provider_id: u64,
    /// 上游真实模型名
    pub model_name: String,
    /// 最长输出 token 数
    pub max_output_tokens: u32,
    /// 最长上下文 token 数
    pub max_context_tokens: u32,
    /// 是否启用：0 否，1 是
    pub enabled: i8,
    /// 最近连通性测试时间
    pub last_tested_at: Option<DateTime>,
    /// 禁用时间；空表示启用
    pub disabled_at: Option<DateTime>,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    ModelId,
    ProviderId,
    ModelName,
    MaxOutputTokens,
    MaxContextTokens,
    Enabled,
    LastTestedAt,
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
    BModels,
    BProviders,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::ModelId => ColumnType::BigUnsigned.def(),
            Self::ProviderId => ColumnType::BigUnsigned.def(),
            Self::ModelName => ColumnType::String(StringLen::N(128u32)).def(),
            Self::MaxOutputTokens => ColumnType::Unsigned.def(),
            Self::MaxContextTokens => ColumnType::Unsigned.def(),
            Self::Enabled => ColumnType::TinyInteger.def(),
            Self::LastTestedAt => ColumnType::DateTime.def().null(),
            Self::DisabledAt => ColumnType::DateTime.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BModels => Entity::belongs_to(super::b_models::Entity)
                .from(Column::ModelId)
                .to(super::b_models::Column::Id)
                .into(),
            Self::BProviders => Entity::belongs_to(super::b_providers::Entity)
                .from(Column::ProviderId)
                .to(super::b_providers::Column::Id)
                .into(),
        }
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

impl ActiveModelBehavior for ActiveModel {}
