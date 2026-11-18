//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_system_settings"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 项目内变量别名
    pub key: String,
    /// 配置值 JSON 或文本
    pub value: String,
    /// 描述
    pub description: Option<String>,
    /// 更新人 ID
    pub updated_by: u64,
    /// 更新时间
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Key,
    Value,
    Description,
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
    BUsers,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::Key => ColumnType::String(StringLen::N(128u32)).def().unique(),
            Self::Value => ColumnType::Text.def(),
            Self::Description => ColumnType::Text.def().null(),
            Self::UpdatedBy => ColumnType::BigUnsigned.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BUsers => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UpdatedBy)
                .to(super::b_users::Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BUsers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
