use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;

pub const INVALID_ARGUMENT: i32 = 1;
pub const UNAUTHENTICATED: i32 = 2;
pub const FORBIDDEN: i32 = 3;
pub const NOT_FOUND: i32 = 4;
pub const LIMIT_EXCEEDED: i32 = 5;
pub const UPSTREAM_FAILED: i32 = 6;
pub const INTERNAL_ERROR: i32 = 7;

impl From<sea_orm::DbErr> for AppError {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::Internal(error.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    InvalidArgument(String),

    #[error("{0}")]
    Unauthenticated(String),

    #[error("{0}")]
    Forbidden(String),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    LimitExceeded(String),

    #[error("{0}")]
    #[allow(dead_code)] // 模型网关错误码映射保留
    UpstreamFailed(String),

    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn code(&self) -> i32 {
        match self {
            Self::InvalidArgument(_) => INVALID_ARGUMENT,
            Self::Unauthenticated(_) => UNAUTHENTICATED,
            Self::Forbidden(_) => FORBIDDEN,
            Self::NotFound(_) => NOT_FOUND,
            Self::LimitExceeded(_) => LIMIT_EXCEEDED,
            Self::UpstreamFailed(_) => UPSTREAM_FAILED,
            Self::Internal(_) => INTERNAL_ERROR,
        }
    }

    pub fn internal(source: impl std::fmt::Display) -> Self {
        Self::Internal(source.to_string())
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: i32,
    message: String,
    data: Option<()>,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::Ok().json(ErrorBody {
            code: self.code(),
            message: self.to_string(),
            data: None,
        })
    }
}
