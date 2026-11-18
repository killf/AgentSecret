//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_user_operation_logs"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 所属用户 ID
    pub user_id: u64,
    /// 脚本语言：bash / python
    pub r#type: String,
    /// 状态
    pub status: String,
    /// 描述
    pub description: Option<String>,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    UserId,
    Type,
    Status,
    Description,
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
    BUsers,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::UserId => ColumnType::BigUnsigned.def(),
            Self::Type => ColumnType::String(StringLen::N(32u32)).def(),
            Self::Status => ColumnType::String(StringLen::N(16u32)).def(),
            Self::Description => ColumnType::Text.def().null(),
            Self::CreatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BUsers => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UserId)
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
