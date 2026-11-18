use actix_web::web;

use crate::controller::{
    admin, auth, frontend, gateway, log as log_controller, mcp,
    permission as permission_controller, project as project_controller, provider_usage,
    sandbox as sandbox_controller, script as script_controller, system as system_controller, token,
    user as user_controller, variable as variable_controller,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .wrap(actix_web::middleware::from_fn(
                crate::middleware::access_token::access_token,
            ))
            .route("/models", web::get().to(gateway::models))
            .route("/responses", web::post().to(gateway::responses)),
    )
    .service(
        web::scope("/mcp")
            .wrap(actix_web::middleware::from_fn(
                crate::middleware::access_token::access_token,
            ))
            .route("", web::post().to(mcp::handle)),
    )
    .route("/", web::get().to(frontend::index))
    .route("/health", web::get().to(crate::controller::health))
    .service(
        web::scope("/api/v1/providers")
            .wrap(actix_web::middleware::from_fn(
                crate::middleware::dual_auth::session_or_access,
            ))
            .route("/usage", web::post().to(provider_usage::provider_usage)),
    )
    .route("/api/v1/auth/login", web::post().to(auth::login))
    .service(
        web::scope("/api/v1")
            .wrap(actix_web::middleware::from_fn(crate::middleware::auth))
            .service(
                web::scope("/auth")
                    .route("/logout", web::post().to(auth::logout))
                    .route("/password", web::post().to(auth::change_password))
                    .route("/me", web::post().to(auth::me)),
            )
            .service(
                web::scope("/projects")
                    .route("/list", web::post().to(project_controller::list_projects))
                    .route("/add", web::post().to(project_controller::add_project))
                    .route(
                        "/detail",
                        web::post().to(project_controller::project_detail),
                    )
                    .route(
                        "/update",
                        web::post().to(project_controller::update_project),
                    )
                    .route(
                        "/delete",
                        web::post().to(project_controller::delete_project),
                    ),
            )
            .service(
                web::scope("/environments")
                    .route(
                        "/list",
                        web::post().to(project_controller::list_environments),
                    )
                    .route("/add", web::post().to(project_controller::add_environment))
                    .route(
                        "/update",
                        web::post().to(project_controller::update_environment),
                    )
                    .route(
                        "/delete",
                        web::post().to(project_controller::delete_environment),
                    ),
            )
            .service(
                web::scope("/variables")
                    .route("/list", web::post().to(variable_controller::list))
                    .route("/add", web::post().to(variable_controller::add))
                    .route("/update", web::post().to(variable_controller::update))
                    .route("/delete", web::post().to(variable_controller::delete)),
            )
            .service(
                web::scope("/scripts")
                    .route("/list", web::post().to(script_controller::list))
                    .route("/detail", web::post().to(script_controller::detail))
                    .route("/add", web::post().to(script_controller::add))
                    .route("/update", web::post().to(script_controller::update))
                    .route("/delete", web::post().to(script_controller::delete))
                    .route("/review", web::post().to(script_controller::review))
                    .route("/run", web::post().to(script_controller::run)),
            )
            .service(
                web::scope("/sandboxes")
                    .route("/list", web::post().to(sandbox_controller::list))
                    .route("/instances", web::post().to(sandbox_controller::instances)),
            )
            .service(
                web::scope("/logs")
                    .route(
                        "/request/list",
                        web::post().to(log_controller::request_list),
                    )
                    .route(
                        "/request/detail",
                        web::post().to(log_controller::request_detail),
                    )
                    .route(
                        "/executions/list",
                        web::post().to(log_controller::execution_list),
                    )
                    .route(
                        "/executions/detail",
                        web::post().to(log_controller::execution_detail),
                    ),
            )
            .service(
                web::scope("/tokens")
                    .route("/list", web::post().to(token::list))
                    .route("/detail", web::post().to(token::detail))
                    .route("/add", web::post().to(token::add))
                    .route("/disable", web::post().to(token::disable))
                    .route("/enable", web::post().to(token::enable))
                    .route("/delete", web::post().to(token::delete))
                    .route("/reset", web::post().to(token::reset)),
            )
            .service(
                web::scope("/admin")
                    .wrap(actix_web::middleware::from_fn(
                        crate::middleware::require_admin,
                    ))
                    .service(
                        web::scope("/models")
                            .route("/list", web::post().to(admin::list_models))
                            .route("/add", web::post().to(admin::add_model))
                            .route("/update", web::post().to(admin::update_model))
                            .route("/delete", web::post().to(admin::delete_model)),
                    )
                    .service(
                        web::scope("/model-providers")
                            .route("/list", web::post().to(admin::list_model_providers))
                            .route("/add", web::post().to(admin::add_model_provider))
                            .route("/update", web::post().to(admin::update_model_provider))
                            .route("/delete", web::post().to(admin::delete_model_provider))
                            .route("/test", web::post().to(provider_usage::test_model_provider)),
                    )
                    .service(
                        web::scope("/providers")
                            .route("/list", web::post().to(admin::list_providers))
                            .route("/add", web::post().to(admin::add_provider))
                            .route("/update", web::post().to(admin::update_provider))
                            .route("/delete", web::post().to(admin::delete_provider))
                            .route(
                                "/refresh-quota",
                                web::post().to(provider_usage::refresh_provider_quota),
                            ),
                    )
                    .service(
                        web::scope("/users")
                            .route("/list", web::post().to(user_controller::list))
                            .route("/add", web::post().to(user_controller::add))
                            .route("/update", web::post().to(user_controller::update))
                            .route("/delete", web::post().to(user_controller::delete))
                            .route("/ban", web::post().to(user_controller::ban))
                            .route("/unban", web::post().to(user_controller::unban))
                            .route("/password", web::post().to(user_controller::reset_password)),
                    )
                    .service(
                        web::scope("/permissions")
                            .route("/list", web::post().to(permission_controller::list))
                            .route("", web::post().to(permission_controller::upsert))
                            .route("/delete", web::post().to(permission_controller::delete)),
                    )
                    .service(
                        web::scope("/settings")
                            .route("/list", web::post().to(system_controller::list))
                            .route("/update", web::post().to(system_controller::update)),
                    ),
            ),
    )
    .default_service(web::to(frontend::fallback));
}
