//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_projects"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 名称或备注
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 项目开发指导
    pub guidance: Option<String>,
    /// 仓库地址数组
    pub repos: Option<Json>,
    /// 创建人 ID
    pub created_by: u64,
    /// 创建时间
    pub created_at: DateTime,
    /// 禁用时间；空表示启用
    pub disabled_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    Name,
    Description,
    Guidance,
    Repos,
    CreatedBy,
    CreatedAt,
    DisabledAt,
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
    BExecutions,
    BSandboxes,
    BScripts,
    BUsers,
    BVariables,
    RProjectPermissions,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def(),
            Self::Description => ColumnType::Text.def().null(),
            Self::Guidance => ColumnType::Text.def().null(),
            Self::Repos => ColumnType::Json.def().null(),
            Self::CreatedBy => ColumnType::BigUnsigned.def(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::DisabledAt => ColumnType::DateTime.def().null(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BEnvironments => Entity::has_many(super::b_environments::Entity).into(),
            Self::BExecutions => Entity::has_many(super::b_executions::Entity).into(),
            Self::BSandboxes => Entity::has_many(super::b_sandboxes::Entity).into(),
            Self::BScripts => Entity::has_many(super::b_scripts::Entity).into(),
            Self::BUsers => Entity::belongs_to(super::b_users::Entity)
                .from(Column::CreatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BVariables => Entity::has_many(super::b_variables::Entity).into(),
            Self::RProjectPermissions => {
                Entity::has_many(super::r_project_permissions::Entity).into()
            }
        }
    }
}

impl Related<super::b_environments::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BEnvironments.def()
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

impl Related<super::b_scripts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BScripts.def()
    }
}

impl Related<super::b_users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BUsers.def()
    }
}

impl Related<super::b_variables::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BVariables.def()
    }
}

impl Related<super::r_project_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RProjectPermissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
