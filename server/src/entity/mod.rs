#![allow(unused, dead_code)]
#![allow(clippy::enum_variant_names)] // SeaORM 依据外键表名生成关系枚举

//! 数据库实体定义

pub mod prelude;

pub mod b_access_tokens;
pub mod b_environments;
pub mod b_executions;
pub mod b_models;
pub mod b_projects;
pub mod b_providers;
pub mod b_request_logs;
pub mod b_sandbox_instances;
pub mod b_sandboxes;
pub mod b_scripts;
pub mod b_system_settings;
pub mod b_user_operation_logs;
pub mod b_users;
pub mod b_variables;
pub mod r_environment_permissions;
pub mod r_model_permissions;
pub mod r_model_providers;
pub mod r_project_permissions;
pub mod r_provider_permissions;
