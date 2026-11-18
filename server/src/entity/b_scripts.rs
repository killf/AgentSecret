//! 数据库实体定义

use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
pub struct Entity;

impl EntityName for Entity {
    fn table_name(&self) -> &'static str {
        "b_scripts"
    }
}

#[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, Eq, Serialize)]
pub struct Model {
    /// 自增主键
    pub id: u64,
    /// 项目 ID
    pub project_id: u64,
    /// 环境 ID
    pub environment_id: Option<u64>,
    /// 沙盒配置 ID
    pub sandbox_id: Option<u64>,
    /// 名称或备注
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 脚本语言：bash / python
    pub r#type: String,
    /// 脚本内容
    pub content: String,
    /// 参数定义 JSON
    pub params: Option<Json>,
    /// 审核固定的凭据操作 JSON
    pub credential_actions: Option<Json>,
    /// 当前审核版本号
    pub revision: u64,
    /// 审核状态：pending / approved / rejected
    pub review_status: String,
    /// 当前版本审核人 ID
    pub reviewed_by: Option<u64>,
    /// 当前版本审核时间
    pub reviewed_at: Option<DateTime>,
    /// 创建人 ID
    pub created_by: u64,
    /// 创建时间
    pub created_at: DateTime,
    /// 更新人 ID
    pub updated_by: u64,
    /// 更新时间
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
pub enum Column {
    Id,
    ProjectId,
    EnvironmentId,
    SandboxId,
    Name,
    Description,
    Type,
    Content,
    Params,
    CredentialActions,
    Revision,
    ReviewStatus,
    ReviewedBy,
    ReviewedAt,
    CreatedBy,
    CreatedAt,
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
    BEnvironments,
    BExecutions,
    BProjects,
    BSandboxes,
    BUsers3,
    BUsers2,
    BUsers1,
}

impl ColumnTrait for Column {
    type EntityName = Entity;
    fn def(&self) -> ColumnDef {
        match self {
            Self::Id => ColumnType::BigUnsigned.def(),
            Self::ProjectId => ColumnType::BigUnsigned.def(),
            Self::EnvironmentId => ColumnType::BigUnsigned.def().null(),
            Self::SandboxId => ColumnType::BigUnsigned.def().null(),
            Self::Name => ColumnType::String(StringLen::N(128u32)).def(),
            Self::Description => ColumnType::Text.def().null(),
            Self::Type => ColumnType::String(StringLen::N(32u32)).def(),
            Self::Content => ColumnType::custom("LONGTEXT").def(),
            Self::Params => ColumnType::Json.def().null(),
            Self::CredentialActions => ColumnType::Json.def().null(),
            Self::Revision => ColumnType::BigUnsigned.def(),
            Self::ReviewStatus => ColumnType::String(StringLen::N(16u32)).def(),
            Self::ReviewedBy => ColumnType::BigUnsigned.def().null(),
            Self::ReviewedAt => ColumnType::DateTime.def().null(),
            Self::CreatedBy => ColumnType::BigUnsigned.def(),
            Self::CreatedAt => ColumnType::DateTime.def(),
            Self::UpdatedBy => ColumnType::BigUnsigned.def(),
            Self::UpdatedAt => ColumnType::DateTime.def(),
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
            Self::BExecutions => Entity::has_many(super::b_executions::Entity).into(),
            Self::BProjects => Entity::belongs_to(super::b_projects::Entity)
                .from(Column::ProjectId)
                .to(super::b_projects::Column::Id)
                .into(),
            Self::BSandboxes => Entity::belongs_to(super::b_sandboxes::Entity)
                .from(Column::SandboxId)
                .to(super::b_sandboxes::Column::Id)
                .into(),
            Self::BUsers3 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::CreatedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BUsers2 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::ReviewedBy)
                .to(super::b_users::Column::Id)
                .into(),
            Self::BUsers1 => Entity::belongs_to(super::b_users::Entity)
                .from(Column::UpdatedBy)
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

impl Related<super::b_sandboxes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BSandboxes.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
