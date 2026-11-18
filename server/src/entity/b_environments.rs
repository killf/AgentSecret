//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_environments"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 项目 ID
    pub project_id: u64,
    /// 名称或备注
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 创建人 ID
    pub created_by: u64,
    /// 创建时间
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    ProjectId,
    Name,
    Description,
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
    BExecutions,
    BProjects,
    BScripts,
    BUsers,
    BVariables,
    REnvironmentPermissions,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::ProjectId => ColumnType::BigUnsigned.def(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def(),
            Self::Description => ColumnType::Text.def().null(),
            Self::CreatedBy => ColumnType::BigUnsigned.def(),
            Self::CreatedAt => ColumnType::DateTime.def(),
        }
    }
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::BExecutions => Entity::has_many(super::b_executions::Entity).into(),
            Self::BProjects => Entity::belongs_to(super::b_projects::Entity)
                .from(Column::ProjectId)
                .to(super::b_projects::Column::Id)
                .into(),
            Self::BScripts => Entity::has_many(super::b_scripts::Entity).into(),
            Self::BUsers => Entity::belongs_to(super::b_users::Entity)
                .from(Column::CreatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BVariables => Entity::has_many(super::b_variables::Entity).into(),
            Self::REnvironmentPermissions => {
                Entity::has_many(super::r_environment_permissions::Entity).into()
            }
        }
    }
}

impl Related<super::b_executions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BExecutions.def()
    }
}

impl Related<super::b_projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BProjects.def()
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

impl Related<super::r_environment_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::REnvironmentPermissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
