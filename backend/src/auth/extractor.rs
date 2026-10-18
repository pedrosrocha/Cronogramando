use axum::{extract::FromRequestParts, http::header, http::request::Parts};
use uuid::Uuid;

use crate::{
    auth::jwt::{self, TokenKind},
    error::AppError,
    state::AppState,
};

pub struct AuthUser {
    pub id: Uuid,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("missing Authorization header".into()))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("expected 'Bearer <token>'".into()))?;

        let claims = jwt::verify(&state.cfg, token, TokenKind::Access)?;

        Ok(AuthUser {
            id: claims.sub,
            role: claims.role,
        })
    }
}

pub struct AdminUser(#[allow(dead_code)] pub AuthUser);

#[axum::async_trait]
impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if !user.is_admin() {
            return Err(AppError::Forbidden("admin role required".into()));
        }
        Ok(AdminUser(user))
    }
}
