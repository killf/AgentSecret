use actix_web::{HttpResponse, web};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde_json::{Value, json};

use crate::entity::{
    b_environments::{self as environments},
    b_executions,
    b_projects::{self as projects},
    b_scripts, b_variables, r_environment_permissions, r_project_permissions,
};
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::schema::admin::{CreatedResponse, IdRequest};
use crate::schema::api_ok;
use crate::schema::project::{
    CreateEnvironmentRequest, CreateProjectRequest, EnvironmentResponse, ProjectIdRequest,
    ProjectResponse, UpdateEnvironmentRequest, UpdateProjectRequest,
};
use crate::state::AppState;

/// 项目列表；管理员全量，普通用户按授权裁剪。
pub async fn list_projects(
    user: AuthUser,
    state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let rows = projects::Entity::find()
        .filter(projects::Column::DisabledAt.is_null())
        .order_by_asc(projects::Column::Id)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let allowed = allowed_project_ids(&state.conn, &user).await?;
    let items: Vec<_> = rows
        .iter()
        .filter(|project| {
            user.is_admin || allowed.as_ref().is_none_or(|ids| ids.contains(&project.id))
        })
        .map(project_response)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(api_ok(items))
}

/// 创建项目。
pub async fn add_project(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<CreateProjectRequest>,
) -> Result<HttpResponse, AppError> {
    validate_name(&body.name)?;
    validate_repos(body.repos.as_deref().unwrap_or(&[]))?;
    let exists = projects::Entity::find()
        .filter(projects::Column::Name.eq(&body.name))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if exists > 0 {
        return Err(AppError::InvalidArgument(
            "project name already exists".into(),
        ));
    }

    let active = projects::ActiveModel {
        id: NotSet,
        name: Set(body.name.clone()),
        description: Set(body.description.clone()),
        guidance: Set(body.guidance.clone()),
        repos: Set(Some(json!(body.repos.clone().unwrap_or_default()))),

        created_by: Set(user.id),
        created_at: Set(Utc::now().naive_utc()),
        disabled_at: Set(None),
    };
    let project = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(CreatedResponse { id: project.id }))
}

/// 项目详情。
pub async fn project_detail(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let project = require_project(&state.conn, &user, body.id).await?;

    Ok(api_ok(project_response(&project)?))
}

/// 更新项目字段。
pub async fn update_project(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<UpdateProjectRequest>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    validate_id(body.id)?;
    if let Some(name) = body.name.as_deref() {
        validate_name(name)?;
    }
    if let Some(repos) = body.repos.as_deref() {
        validate_repos(repos)?;
    }
    let project = require_project(&state.conn, &user, body.id).await?;
    let mut active: projects::ActiveModel = project.into();
    if let Some(name) = body.name {
        active.name = Set(name);
    }
    if let Some(description) = body.description {
        active.description = Set(description);
    }
    if let Some(guidance) = body.guidance {
        active.guidance = Set(guidance);
    }
    if let Some(repos) = body.repos {
        active.repos = Set(Some(json!(repos)));
    }
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 删除未被引用的项目。
pub async fn delete_project(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    require_project(&state.conn, &user, body.id).await?;
    if project_in_use(&state.conn, body.id).await? {
        return Err(AppError::NotFound("project is still referenced".into()));
    }

    projects::Entity::delete_by_id(body.id)
        .exec(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 环境列表。
pub async fn list_environments(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<ProjectIdRequest>,
) -> Result<HttpResponse, AppError> {
    require_project(&state.conn, &user, body.project_id).await?;
    let rows = environments::Entity::find()
        .filter(environments::Column::ProjectId.eq(body.project_id))
        .order_by_asc(environments::Column::Id)
        .all(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let items: Vec<_> = rows
        .iter()
        .map(|env| EnvironmentResponse {
            id: env.id,
            project_id: env.project_id,
            name: env.name.clone(),
            description: env.description.clone(),
            created_at: env.created_at.and_utc(),
        })
        .collect();

    Ok(api_ok(items))
}

/// 创建项目环境。
pub async fn add_environment(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<CreateEnvironmentRequest>,
) -> Result<HttpResponse, AppError> {
    require_project(&state.conn, &user, body.project_id).await?;
    if !matches!(body.name.as_str(), "dev" | "test" | "prod") {
        return Err(AppError::InvalidArgument(
            "environment name must be dev, test, or prod".into(),
        ));
    }
    let exists = environments::Entity::find()
        .filter(environments::Column::ProjectId.eq(body.project_id))
        .filter(environments::Column::Name.eq(&body.name))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if exists > 0 {
        return Err(AppError::InvalidArgument(
            "environment name already exists".into(),
        ));
    }

    let active = environments::ActiveModel {
        id: NotSet,
        project_id: Set(body.project_id),
        name: Set(body.name.clone()),
        description: Set(body.description.clone()),
        created_by: Set(user.id),
        created_at: Set(Utc::now().naive_utc()),
    };
    let environment = active
        .insert(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(CreatedResponse { id: environment.id }))
}

/// 更新环境。
pub async fn update_environment(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<UpdateEnvironmentRequest>,
) -> Result<HttpResponse, AppError> {
    let body = body.into_inner();
    let environment = require_environment(&state.conn, &user, body.id).await?;
    if let Some(name) = body.name.as_deref() {
        if !matches!(name, "dev" | "test" | "prod") {
            return Err(AppError::InvalidArgument(
                "environment name must be dev, test, or prod".into(),
            ));
        }
        let exists = environments::Entity::find()
            .filter(environments::Column::ProjectId.eq(environment.project_id))
            .filter(environments::Column::Name.eq(name))
            .filter(environments::Column::Id.ne(body.id))
            .count(&state.conn)
            .await
            .map_err(AppError::internal)?;
        if exists > 0 {
            return Err(AppError::InvalidArgument(
                "environment name already exists".into(),
            ));
        }
    }

    let mut active: environments::ActiveModel = environment.into();
    if let Some(name) = body.name {
        active.name = Set(name);
    }
    if let Some(description) = body.description {
        active.description = Set(description);
    }
    active
        .update(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

/// 删除未被引用的环境。
pub async fn delete_environment(
    user: AuthUser,
    state: web::Data<AppState>,
    body: web::Json<IdRequest>,
) -> Result<HttpResponse, AppError> {
    let environment = require_environment(&state.conn, &user, body.id).await?;
    let variable_count = b_variables::Entity::find()
        .filter(b_variables::Column::EnvironmentId.eq(environment.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let script_count = b_scripts::Entity::find()
        .filter(b_scripts::Column::EnvironmentId.eq(environment.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let execution_count = b_executions::Entity::find()
        .filter(b_executions::Column::EnvironmentId.eq(environment.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    let permission_count = r_environment_permissions::Entity::find()
        .filter(r_environment_permissions::Column::EnvironmentId.eq(environment.id))
        .count(&state.conn)
        .await
        .map_err(AppError::internal)?;
    if variable_count + script_count + execution_count + permission_count > 0 {
        return Err(AppError::NotFound("environment is still referenced".into()));
    }

    environments::Entity::delete_by_id(environment.id)
        .exec(&state.conn)
        .await
        .map_err(AppError::internal)?;

    Ok(api_ok(Option::<()>::None))
}

async fn require_project(
    conn: &sea_orm::DatabaseConnection,
    user: &AuthUser,
    project_id: u64,
) -> Result<projects::Model, AppError> {
    validate_id(project_id)?;
    let project = projects::Entity::find_by_id(project_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("project does not exist".into()))?;
    if project.disabled_at.is_some() {
        return Err(AppError::NotFound("project does not exist".into()));
    }
    if user.is_admin {
        return Ok(project);
    }

    let allowed = allowed_project_ids(conn, user).await?;
    if allowed.is_some_and(|ids| ids.contains(&project_id)) {
        Ok(project)
    } else {
        Err(AppError::Forbidden("project permission is required".into()))
    }
}

async fn require_environment(
    conn: &sea_orm::DatabaseConnection,
    user: &AuthUser,
    environment_id: u64,
) -> Result<environments::Model, AppError> {
    validate_id(environment_id)?;
    let environment = environments::Entity::find_by_id(environment_id)
        .one(conn)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::NotFound("environment does not exist".into()))?;
    require_project(conn, user, environment.project_id).await?;

    Ok(environment)
}

async fn allowed_project_ids(
    conn: &sea_orm::DatabaseConnection,
    user: &AuthUser,
) -> Result<Option<Vec<u64>>, AppError> {
    let permissions = r_project_permissions::Entity::find()
        .filter(r_project_permissions::Column::UserId.eq(user.id))
        .all(conn)
        .await
        .map_err(AppError::internal)?;
    if permissions
        .iter()
        .any(|permission| permission.project_id.is_none())
    {
        return Ok(None);
    }

    Ok(Some(
        permissions
            .into_iter()
            .filter_map(|permission| permission.project_id)
            .collect(),
    ))
}

async fn project_in_use(
    conn: &sea_orm::DatabaseConnection,
    project_id: u64,
) -> Result<bool, AppError> {
    let environment_count = environments::Entity::find()
        .filter(environments::Column::ProjectId.eq(project_id))
        .count(conn)
        .await
        .map_err(AppError::internal)?;
    let variable_count = b_variables::Entity::find()
        .filter(b_variables::Column::ProjectId.eq(project_id))
        .count(conn)
        .await
        .map_err(AppError::internal)?;
    let sandbox_count = crate::entity::b_sandboxes::Entity::find()
        .filter(crate::entity::b_sandboxes::Column::ProjectId.eq(project_id))
        .count(conn)
        .await
        .map_err(AppError::internal)?;
    let script_count = b_scripts::Entity::find()
        .filter(b_scripts::Column::ProjectId.eq(project_id))
        .count(conn)
        .await
        .map_err(AppError::internal)?;
    let execution_count = b_executions::Entity::find()
        .filter(b_executions::Column::ProjectId.eq(project_id))
        .count(conn)
        .await
        .map_err(AppError::internal)?;
    let permission_count = r_project_permissions::Entity::find()
        .filter(r_project_permissions::Column::ProjectId.eq(project_id))
        .count(conn)
        .await
        .map_err(AppError::internal)?;

    Ok(environment_count
        + variable_count
        + sandbox_count
        + script_count
        + execution_count
        + permission_count
        > 0)
}

fn project_response(project: &projects::Model) -> Result<ProjectResponse, AppError> {
    let repos = project
        .repos
        .clone()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let repos = serde_json::from_value::<Vec<String>>(repos).map_err(AppError::internal)?;

    Ok(ProjectResponse {
        id: project.id,
        name: project.name.clone(),
        description: project.description.clone(),
        guidance: project.guidance.clone(),
        repos,
        created_at: project.created_at.and_utc(),
        disabled_at: project.disabled_at.map(|value| value.and_utc()),
    })
}

fn validate_id(id: u64) -> Result<(), AppError> {
    if id == 0 {
        Err(AppError::InvalidArgument(
            "id must be greater than zero".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_name(name: &str) -> Result<(), AppError> {
    if !name.trim().is_empty() && name == name.trim() && name.len() <= 128 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "name must contain 1 to 128 characters without surrounding spaces".into(),
        ))
    }
}

fn validate_repos(repos: &[String]) -> Result<(), AppError> {
    let valid = repos.iter().all(|repo| {
        (repo.starts_with("https://") || repo.starts_with("http://")) && repo.len() <= 512
    });
    if valid {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "repos must contain HTTP(S) URLs with at most 512 characters".into(),
        ))
    }
}
