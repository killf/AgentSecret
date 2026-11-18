use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Clone)]
pub struct TokenManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionClaims {
    pub sub: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub jti: String,
    pub iat: i64,
    pub exp: Option<i64>,
}

impl TokenManager {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// 签发单会话登录 JWT。
    pub fn issue_session(
        &self,
        user_id: u64,
        is_admin: bool,
        issued_at: DateTime<Utc>,
        ttl_days: i64,
    ) -> Result<(String, DateTime<Utc>), AppError> {
        let expires_at = issued_at + Duration::days(ttl_days);
        let claims = SessionClaims {
            sub: user_id.to_string(),
            role: if is_admin { "admin" } else { "user" }.to_string(),
            iat: issued_at.timestamp(),
            exp: expires_at.timestamp(),
        };
        let token =
            encode(&Header::default(), &claims, &self.encoding_key).map_err(AppError::internal)?;

        Ok((token, expires_at))
    }

    /// 验证登录 JWT。
    pub fn verify_session(&self, token: &str) -> Result<SessionClaims, AppError> {
        decode::<SessionClaims>(
            token,
            &self.decoding_key,
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        )
        .map(|data| data.claims)
        .map_err(|_| AppError::Unauthenticated("session token is invalid".into()))
    }

    /// 签发模型网关 / MCP 访问令牌；过期时间为空表示不过期。
    pub fn issue_access(
        &self,
        user_id: u64,
        jti: String,
        issued_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<String, AppError> {
        let claims = AccessClaims {
            sub: user_id.to_string(),
            jti,
            iat: issued_at.timestamp(),
            exp: expires_at.map(|expires_at| expires_at.timestamp()),
        };
        encode(&Header::default(), &claims, &self.encoding_key).map_err(AppError::internal)
    }

    /// 验证模型网关 / MCP 访问令牌。
    #[allow(dead_code)]
    pub fn verify_access(&self, token: &str) -> Result<AccessClaims, AppError> {
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.required_spec_claims = std::collections::HashSet::new();
        decode::<AccessClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|_| AppError::Unauthenticated("access token is invalid".into()))
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_token_round_trip() {
        let manager = TokenManager::new("secret");
        let issued_at = Utc::now();
        let (token, expires_at) = manager.issue_session(7, true, issued_at, 7).unwrap();
        let claims = manager.verify_session(&token).unwrap();

        assert_eq!(claims.sub, "7");
        assert_eq!(claims.role, "admin");
        assert_eq!(claims.exp, expires_at.timestamp());
    }

    #[test]
    fn access_token_can_have_no_expiry() {
        let manager = TokenManager::new("secret");
        let token = manager
            .issue_access(8, "jti".into(), Utc::now(), None)
            .unwrap();
        let claims = manager.verify_access(&token).unwrap();

        assert_eq!(claims.jti, "jti");
        assert!(claims.exp.is_none());
    }
}
