-- ==== 用户认证基础 ====
CREATE TABLE IF NOT EXISTS b_users (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(128) NOT NULL,
    password CHAR(40) NOT NULL,
    password_salt CHAR(32) NOT NULL,
    is_admin TINYINT(1) NOT NULL DEFAULT 0,
    banned_at DATETIME(3) NULL,
    unban_at DATETIME(3) NULL,
    session_token TEXT NULL,
    session_expire_at DATETIME(3) NULL,
    created_by BIGINT UNSIGNED NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_users_email UNIQUE (email),
    CONSTRAINT fk_users_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;


-- ==== 访问令牌 ====
CREATE TABLE IF NOT EXISTS b_access_tokens (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(128) NULL,
    token VARCHAR(512) NOT NULL,
    expires_at DATETIME(3) NULL,
    disabled_at DATETIME(3) NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_access_tokens_token UNIQUE (token),
    CONSTRAINT fk_access_tokens_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT,
    INDEX idx_access_tokens_user (user_id)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;


-- ==== 用户操作日志 ====
CREATE TABLE IF NOT EXISTS b_user_operation_logs (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    type VARCHAR(32) NOT NULL,
    status VARCHAR(16) NOT NULL,
    description TEXT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_user_operation_logs_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT,
    INDEX idx_user_operation_logs_user_time (user_id, created_at),
    INDEX idx_user_operation_logs_time (created_at)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;


-- ==== 模型网关 ====
CREATE TABLE IF NOT EXISTS b_providers (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    base_url VARCHAR(512) NOT NULL,
    access_token TEXT NOT NULL,
    provider_type VARCHAR(32) NOT NULL,
    quota_5h_remaining BIGINT NULL,
    quota_5h_refreshed_at DATETIME(3) NULL,
    quota_weekly_remaining BIGINT NULL,
    quota_weekly_refreshed_at DATETIME(3) NULL,
    quota_monthly_remaining BIGINT NULL,
    quota_monthly_refreshed_at DATETIME(3) NULL,
    disabled_at DATETIME(3) NULL,
    created_by BIGINT UNSIGNED NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_by BIGINT UNSIGNED NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_providers_name UNIQUE (name),
    CONSTRAINT fk_providers_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_providers_updater FOREIGN KEY (updated_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_models (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    disabled_at DATETIME(3) NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_models_name UNIQUE (name)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS r_model_providers (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    model_id BIGINT UNSIGNED NOT NULL,
    provider_id BIGINT UNSIGNED NOT NULL,
    model_name VARCHAR(128) NOT NULL,
    max_output_tokens INT UNSIGNED NOT NULL,
    max_context_tokens INT UNSIGNED NOT NULL,
    enabled TINYINT(1) NOT NULL DEFAULT 1,
    last_tested_at DATETIME(3) NULL,
    disabled_at DATETIME(3) NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_model_providers_model_provider UNIQUE (model_id, provider_id),
    CONSTRAINT fk_model_providers_model FOREIGN KEY (model_id) REFERENCES b_models (id) ON DELETE RESTRICT,
    CONSTRAINT fk_model_providers_provider FOREIGN KEY (provider_id) REFERENCES b_providers (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_request_logs (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    model_id BIGINT UNSIGNED NOT NULL,
    provider_id BIGINT UNSIGNED NOT NULL,
    token_id BIGINT UNSIGNED NULL,
    input_tokens INT UNSIGNED NOT NULL,
    output_tokens INT UNSIGNED NOT NULL,
    cache_tokens INT UNSIGNED NOT NULL,
    ttft_ms INT UNSIGNED NOT NULL,
    duration_ms INT UNSIGNED NOT NULL,
    contains_sensitive TINYINT(1) NOT NULL,
    request_detail JSON NULL,
    response_detail JSON NULL,
    request_status VARCHAR(16) NOT NULL,
    failure_reason TEXT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_request_logs_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_request_logs_model FOREIGN KEY (model_id) REFERENCES b_models (id) ON DELETE RESTRICT,
    CONSTRAINT fk_request_logs_provider FOREIGN KEY (provider_id) REFERENCES b_providers (id) ON DELETE RESTRICT,
    CONSTRAINT fk_request_logs_token FOREIGN KEY (token_id) REFERENCES b_access_tokens (id) ON DELETE RESTRICT,
    INDEX idx_request_logs_time (created_at),
    INDEX idx_request_logs_user_time (user_id, created_at),
    INDEX idx_request_logs_model_time (model_id, created_at)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS r_model_permissions (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    model_id BIGINT UNSIGNED NULL,
    daily_token_limit BIGINT NULL,
    daily_token_used BIGINT UNSIGNED NOT NULL DEFAULT 0,
    daily_token_refreshed_at DATETIME(3) NULL,
    weekly_token_limit BIGINT NULL,
    weekly_token_used BIGINT UNSIGNED NOT NULL DEFAULT 0,
    weekly_token_refreshed_at DATETIME(3) NULL,
    monthly_token_limit BIGINT NULL,
    monthly_token_used BIGINT UNSIGNED NOT NULL DEFAULT 0,
    monthly_token_refreshed_at DATETIME(3) NULL,
    created_by BIGINT UNSIGNED NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_by BIGINT UNSIGNED NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_model_permissions_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_model_permissions_model FOREIGN KEY (model_id) REFERENCES b_models (id) ON DELETE RESTRICT,
    CONSTRAINT fk_model_permissions_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_model_permissions_updater FOREIGN KEY (updated_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS r_provider_permissions (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    provider_id BIGINT UNSIGNED NULL,
    daily_token_limit BIGINT NULL,
    daily_token_used BIGINT UNSIGNED NOT NULL DEFAULT 0,
    daily_token_refreshed_at DATETIME(3) NULL,
    weekly_token_limit BIGINT NULL,
    weekly_token_used BIGINT UNSIGNED NOT NULL DEFAULT 0,
    weekly_token_refreshed_at DATETIME(3) NULL,
    monthly_token_limit BIGINT NULL,
    monthly_token_used BIGINT UNSIGNED NOT NULL DEFAULT 0,
    monthly_token_refreshed_at DATETIME(3) NULL,
    created_by BIGINT UNSIGNED NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_by BIGINT UNSIGNED NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_provider_permissions_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_provider_permissions_provider FOREIGN KEY (provider_id) REFERENCES b_providers (id) ON DELETE RESTRICT,
    CONSTRAINT fk_provider_permissions_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_provider_permissions_updater FOREIGN KEY (updated_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;


-- ==== 项目工作区 ====
CREATE TABLE IF NOT EXISTS b_projects (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    description TEXT NULL,
    guidance TEXT NULL,
    repos JSON NULL,
    created_by BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    disabled_at DATETIME(3) NULL,
    CONSTRAINT fk_projects_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_environments (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    project_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT NULL,
    created_by BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_environments_project_name UNIQUE (project_id, name),
    CONSTRAINT fk_environments_project FOREIGN KEY (project_id) REFERENCES b_projects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_environments_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_variables (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    project_id BIGINT UNSIGNED NOT NULL,
    environment_id BIGINT UNSIGNED NOT NULL,
    `key` VARCHAR(128) NOT NULL,
    source_env VARCHAR(128) NOT NULL,
    description TEXT NULL,
    disabled_at DATETIME(3) NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_variables_project_environment_key UNIQUE (project_id, environment_id, `key`),
    CONSTRAINT fk_variables_project FOREIGN KEY (project_id) REFERENCES b_projects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_variables_environment FOREIGN KEY (environment_id) REFERENCES b_environments (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_sandboxes (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    project_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT NULL,
    image VARCHAR(512) NOT NULL,
    max_instances INT UNSIGNED NOT NULL,
    updated_by BIGINT UNSIGNED NOT NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_sandboxes_project FOREIGN KEY (project_id) REFERENCES b_projects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_sandboxes_updater FOREIGN KEY (updated_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_scripts (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    project_id BIGINT UNSIGNED NOT NULL,
    environment_id BIGINT UNSIGNED NULL,
    sandbox_id BIGINT UNSIGNED NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT NULL,
    type VARCHAR(32) NOT NULL,
    content LONGTEXT NOT NULL,
    params JSON NULL,
    credential_actions JSON NULL,
    revision BIGINT UNSIGNED NOT NULL DEFAULT 1,
    review_status VARCHAR(16) NOT NULL DEFAULT 'pending',
    reviewed_by BIGINT UNSIGNED NULL,
    reviewed_at DATETIME(3) NULL,
    created_by BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_by BIGINT UNSIGNED NOT NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_scripts_project_environment_name UNIQUE (project_id, (COALESCE(environment_id, 0)), name),
    CONSTRAINT fk_scripts_project FOREIGN KEY (project_id) REFERENCES b_projects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_scripts_environment FOREIGN KEY (environment_id) REFERENCES b_environments (id) ON DELETE RESTRICT,
    CONSTRAINT fk_scripts_sandbox FOREIGN KEY (sandbox_id) REFERENCES b_sandboxes (id) ON DELETE RESTRICT,
    CONSTRAINT fk_scripts_reviewer FOREIGN KEY (reviewed_by) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_scripts_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_scripts_updater FOREIGN KEY (updated_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_sandbox_instances (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    sandbox_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(128) NOT NULL,
    status VARCHAR(16) NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    destroyed_at DATETIME(3) NULL,
    CONSTRAINT ux_sandbox_instances_name UNIQUE (name),
    CONSTRAINT fk_sandbox_instances_sandbox FOREIGN KEY (sandbox_id) REFERENCES b_sandboxes (id) ON DELETE RESTRICT,
    INDEX idx_sandbox_instances_sandbox_status (sandbox_id, status)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_executions (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    script_id BIGINT UNSIGNED NOT NULL,
    script_revision BIGINT UNSIGNED NOT NULL,
    project_id BIGINT UNSIGNED NOT NULL,
    environment_id BIGINT UNSIGNED NULL,
    type VARCHAR(32) NOT NULL,
    status VARCHAR(16) NOT NULL,
    stdout LONGTEXT NULL,
    stderr LONGTEXT NULL,
    error_code VARCHAR(64) NULL,
    sandbox_id BIGINT UNSIGNED NOT NULL,
    sandbox_instance_id BIGINT UNSIGNED NOT NULL,
    user_id BIGINT UNSIGNED NOT NULL,
    started_at DATETIME(3) NULL,
    finished_at DATETIME(3) NULL,
    CONSTRAINT ux_executions_instance UNIQUE (sandbox_instance_id),
    CONSTRAINT fk_executions_script FOREIGN KEY (script_id) REFERENCES b_scripts (id) ON DELETE RESTRICT,
    CONSTRAINT fk_executions_project FOREIGN KEY (project_id) REFERENCES b_projects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_executions_environment FOREIGN KEY (environment_id) REFERENCES b_environments (id) ON DELETE RESTRICT,
    CONSTRAINT fk_executions_sandbox FOREIGN KEY (sandbox_id) REFERENCES b_sandboxes (id) ON DELETE RESTRICT,
    CONSTRAINT fk_executions_instance FOREIGN KEY (sandbox_instance_id) REFERENCES b_sandbox_instances (id) ON DELETE RESTRICT,
    CONSTRAINT fk_executions_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;


-- ==== 权限与系统设置 ====
CREATE TABLE IF NOT EXISTS r_project_permissions (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    project_id BIGINT UNSIGNED NULL,
    created_by BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_project_permissions_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_project_permissions_project FOREIGN KEY (project_id) REFERENCES b_projects (id) ON DELETE RESTRICT,
    CONSTRAINT fk_project_permissions_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS r_environment_permissions (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    environment_id BIGINT UNSIGNED NULL,
    created_by BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_environment_permissions_user FOREIGN KEY (user_id) REFERENCES b_users (id) ON DELETE RESTRICT,
    CONSTRAINT fk_environment_permissions_environment FOREIGN KEY (environment_id) REFERENCES b_environments (id) ON DELETE RESTRICT,
    CONSTRAINT fk_environment_permissions_creator FOREIGN KEY (created_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;

CREATE TABLE IF NOT EXISTS b_system_settings (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    `key` VARCHAR(128) NOT NULL,
    value TEXT NOT NULL,
    description TEXT NULL,
    updated_by BIGINT UNSIGNED NOT NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    CONSTRAINT ux_system_settings_key UNIQUE (`key`),
    CONSTRAINT fk_system_settings_updater FOREIGN KEY (updated_by) REFERENCES b_users (id) ON DELETE RESTRICT
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4;


