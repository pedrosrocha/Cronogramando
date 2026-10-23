use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthUser,
        jwt::{self, TokenKind},
        password,
    },
    error::AppError,
    models::user::{User, UserResponse},
    state::AppState,
};

const USER_COLUMNS: &str =
    "id, email, password_hash, role, preferred_language, created_at, updated_at";

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub preferred_language: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

async fn fetch_by_email(state: &AppState, email: &str) -> Result<Option<User>, AppError> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE lower(email) = $1");
    let user = sqlx::query_as::<_, User>(&sql)
        .bind(email)
        .fetch_optional(&state.pool)
        .await?;
    Ok(user)
}

async fn fetch_by_id(state: &AppState, id: Uuid) -> Result<Option<User>, AppError> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1");
    let user = sqlx::query_as::<_, User>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    Ok(user)
}

fn build_auth_response(state: &AppState, user: User) -> Result<AuthResponse, AppError> {
    let (access_token, refresh_token) = jwt::issue_pair(&state.cfg, user.id, &user.role)?;
    Ok(AuthResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: state.cfg.access_token_ttl_secs,
        user: user.into(),
    })
}

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 409, description = "Email already registered", body = ErrorResponse),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let email = payload.email.trim().to_lowercase();
    if !email.contains('@') || email.len() < 3 {
        return Err(AppError::BadRequest("invalid email address".into()));
    }
    if payload.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }

    if fetch_by_email(&state, &email).await?.is_some() {
        return Err(AppError::Conflict("email already registered".into()));
    }

    let password_hash = password::hash_password(&payload.password)?;
    let language = payload
        .preferred_language
        .unwrap_or_else(|| "pt-BR".to_string());

    let sql = format!(
        "INSERT INTO users (email, password_hash, role, preferred_language) \
         VALUES ($1, $2, 'user', $3) RETURNING {USER_COLUMNS}"
    );
    let user = sqlx::query_as::<_, User>(&sql)
        .bind(&email)
        .bind(&password_hash)
        .bind(&language)
        .fetch_one(&state.pool)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(build_auth_response(&state, user)?),
    ))
}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = payload.email.trim().to_lowercase();
    let user = fetch_by_email(&state, &email)
        .await?
        .ok_or_else(|| AppError::Unauthorized("invalid credentials".into()))?;

    if !password::verify_password(&payload.password, &user.password_hash) {
        return Err(AppError::Unauthorized("invalid credentials".into()));
    }

    Ok(Json(build_auth_response(&state, user)?))
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "New token pair", body = AuthResponse),
        (status = 401, description = "Invalid refresh token", body = ErrorResponse),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let claims = jwt::verify(&state.cfg, &payload.refresh_token, TokenKind::Refresh)?;
    let user = fetch_by_id(&state, claims.sub)
        .await?
        .ok_or_else(|| AppError::Unauthorized("user no longer exists".into()))?;

    Ok(Json(build_auth_response(&state, user)?))
}

#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    )
)]
pub async fn me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<UserResponse>, AppError> {
    let user = fetch_by_id(&state, auth_user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;
    Ok(Json(user.into()))
}
