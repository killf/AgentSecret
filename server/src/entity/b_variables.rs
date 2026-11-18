//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_variables"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 项目 ID
    pub project_id: u64,
    /// 环境 ID
    pub environment_id: u64,
    /// 项目内变量别名
    pub key: String,
    /// 可信执行端环境变量名
    pub source_env: String,
    /// 描述
    pub description: Option<String>,
    /// 禁用时间；空表示启用
    pub disabled_at: Option<DateTime>,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    ProjectId,
    EnvironmentId,
    Key,
    SourceEnv,
    Description,
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
    BEnvironments,
    BProjects,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::ProjectId => ColumnType::BigUnsigned.def(),
            Self::EnvironmentId => ColumnType::BigUnsigned.def(),
            Self::Key => ColumnType::String(StringLen::N(128u32)).def(),
            Self::SourceEnv => ColumnType::String(StringLen::N(128u32)).def(),
            Self::Description => ColumnType::Text.def().null(),
            Self::DisabledAt => ColumnType::DateTime.def().null(),
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
            Self::BProjects => Entity::belongs_to(super::b_projects::Entity)
                .from(Column::ProjectId)
                .to(super::b_projects::Column::Id)
                .into(),
        }
    }
}

impl Related<super::b_environments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BEnvironments.def()
    }
}

impl Related<super::b_projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BProjects.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
