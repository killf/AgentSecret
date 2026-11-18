use actix_web::HttpResponse;
use serde::Serialize;

/// /api/v1 统一响应包裹。
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

pub fn api_ok<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(ApiResponse {
        code: 0,
        message: "success".into(),
        data,
    })
}
