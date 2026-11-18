//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_models"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 名称或备注
    pub name: String,
    /// 禁用时间；空表示启用
    pub disabled_at: Option<DateTime>,
    /// 创建时间
    pub created_at: DateTime,
    /// 更新时间
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Name,
    DisabledAt,
    CreatedAt,
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
    RModelPermissions,
    RModelProviders,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def().unique(),
            Self::DisabledAt => ColumnType::DateTime.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BRequestLogs => Entity::has_many(super::b_request_logs::Entity).into(),
            Self::RModelPermissions => Entity::has_many(super::r_model_permissions::Entity).into(),
            Self::RModelProviders => Entity::has_many(super::r_model_providers::Entity).into(),
        }
    }
}

impl Related<super::b_request_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BRequestLogs.def()
    }
}

impl Related<super::r_model_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RModelPermissions.def()
    }
}

impl Related<super::r_model_providers::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RModelProviders.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
