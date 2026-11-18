//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_sandbox_instances"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 沙盒配置 ID
    pub sandbox_id: u64,
    /// 名称或备注
    pub name: String,
    /// 状态
    pub status: String,
    /// 创建时间
    pub created_at: DateTime,
    /// 确认销毁时间
    pub destroyed_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    SandboxId,
    Name,
    Status,
    CreatedAt,
    DestroyedAt,
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
    BExecutions,
    BSandboxes,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::SandboxId => ColumnType::BigUnsigned.def(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def().unique(),
            Self::Status => ColumnType::String(StringLen::N(16u32)).def(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::DestroyedAt => ColumnType::DateTime.def().null(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BExecutions => Entity::has_one(super::b_executions::Entity).into(),
            Self::BSandboxes => Entity::belongs_to(super::b_sandboxes::Entity)
                .from(Column::SandboxId)
                .to(super::b_sandboxes::Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BExecutions.def()
    }
}

impl Related<super::b_sandboxes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BSandboxes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
