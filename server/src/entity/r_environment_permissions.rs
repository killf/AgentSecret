//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "r_environment_permissions"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 所属用户 ID
    pub user_id: u64,
    /// 环境 ID
    pub environment_id: Option<u64>,
    /// 创建人 ID
    pub created_by: u64,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    UserId,
    EnvironmentId,
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
    BEnvironments,
    BUsers2,
    BUsers1,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::UserId => ColumnType::BigUnsigned.def(),
            Self::EnvironmentId => ColumnType::BigUnsigned.def().null(),
            Self::CreatedBy => ColumnType::BigUnsigned.def(),
            Self::CreatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BEnvironments => Entity::belongs_to(super::b_environments::Entity)
                .from(Column::EnvironmentId)
                .to(super::b_environments::Column::Id)
                .into(),
            Self::BUsers2 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::CreatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BUsers1 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UserId)
                .to(super::b_users::Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_environments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BEnvironments.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
