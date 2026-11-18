//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_executions"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 脚本 ID
    pub script_id: u64,
    /// 执行时脚本版本
    pub script_revision: u64,
    /// 项目 ID
    pub project_id: u64,
    /// 环境 ID
    pub environment_id: Option<u64>,
    /// 脚本语言：bash / python
    pub r#type: String,
    /// 状态
    pub status: String,
    /// 过滤后的标准输出
    pub stdout: Option<String>,
    /// 过滤后的标准错误
    pub stderr: Option<String>,
    /// 固定错误原因码
    pub error_code: Option<String>,
    /// 沙盒配置 ID
    pub sandbox_id: u64,
    /// 沙盒实例 ID
    pub sandbox_instance_id: u64,
    /// 所属用户 ID
    pub user_id: u64,
    /// 开始执行时间
    pub started_at: Option<DateTime>,
    /// 执行结束时间
    pub finished_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    ScriptId,
    ScriptRevision,
    ProjectId,
    EnvironmentId,
    Type,
    Status,
    Stdout,
    Stderr,
    ErrorCode,
    SandboxId,
    SandboxInstanceId,
    UserId,
    StartedAt,
    FinishedAt,
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
    BSandboxInstances,
    BSandboxes,
    BScripts,
    BUsers,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::ScriptId => ColumnType::BigUnsigned.def(),
            Self::ScriptRevision => ColumnType::BigUnsigned.def(),
            Self::ProjectId => ColumnType::BigUnsigned.def(),
            Self::EnvironmentId => ColumnType::BigUnsigned.def().null(),
            Self::Type => ColumnType::String(StringLen::N(32u32)).def(),
            Self::Status => ColumnType::String(StringLen::N(16u32)).def(),
            Self::Stdout => ColumnType::custom("LONGTEXT").def().null(),
            Self::Stderr => ColumnType::custom("LONGTEXT").def().null(),
            Self::ErrorCode => ColumnType::String(StringLen::N(64u32)).def().null(),
            Self::SandboxId => ColumnType::BigUnsigned.def(),
            Self::SandboxInstanceId => ColumnType::BigUnsigned.def().unique(),
            Self::UserId => ColumnType::BigUnsigned.def(),
            Self::StartedAt => ColumnType::DateTime.def().null(),
            Self::FinishedAt => ColumnType::DateTime.def().null(),
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
            Self::BSandboxInstances => Entity::belongs_to(super::b_sandbox_instances::Entity)
                .from(Column::SandboxInstanceId)
                .to(super::b_sandbox_instances::Column::Id)
                .into(),
            Self::BSandboxes => Entity::belongs_to(super::b_sandboxes::Entity)
                .from(Column::SandboxId)
                .to(super::b_sandboxes::Column::Id)
                .into(),
            Self::BScripts => Entity::belongs_to(super::b_scripts::Entity)
                .from(Column::ScriptId)
                .to(super::b_scripts::Column::Id)
                .into(),
            Self::BUsers => Entity::belongs_to(super::b_users::Entity)
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

impl Related<super::b_projects::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BProjects.def()
    }
}

impl Related<super::b_sandbox_instances::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BSandboxInstances.def()
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

impl ActiveModelBehavior for ActiveModel {}
