use actix_web::{Error, FromRequest, HttpMessage, HttpRequest};
use std::future::{Ready, ready};

use crate::middleware::AuthUser;

impl FromRequest for AuthUser {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let result = req.extensions().get::<AuthUser>().cloned().ok_or_else(|| {
            actix_web::error::ErrorInternalServerError("authenticated user is missing")
        });
        ready(result)
    }
}
