use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::Config, error::AppError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenKind {
    Access,
    Refresh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub kind: TokenKind,
    pub iat: i64,
    pub exp: i64,
}

fn ttl(cfg: &Config, kind: TokenKind) -> i64 {
    match kind {
        TokenKind::Access => cfg.access_token_ttl_secs,
        TokenKind::Refresh => cfg.refresh_token_ttl_secs,
    }
}

pub fn issue(cfg: &Config, user_id: Uuid, role: &str, kind: TokenKind) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user_id,
        role: role.to_string(),
        kind,
        iat: now,
        exp: now + ttl(cfg, kind),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify(cfg: &Config, token: &str, expected: TokenKind) -> Result<Claims, AppError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::Unauthorized("invalid or expired token".into()))?;

    if data.claims.kind != expected {
        return Err(AppError::Unauthorized("unexpected token type".into()));
    }

    Ok(data.claims)
}

pub fn issue_pair(cfg: &Config, user_id: Uuid, role: &str) -> Result<(String, String), AppError> {
    let access = issue(cfg, user_id, role, TokenKind::Access)?;
    let refresh = issue(cfg, user_id, role, TokenKind::Refresh)?;
    Ok((access, refresh))
}
