use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AdminUser,
    error::AppError,
    models::user::{User, UserResponse},
    state::AppState,
};

const USER_COLUMNS: &str =
    "id, email, password_hash, role, preferred_language, created_at, updated_at";

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub preferred_language: Option<String>,
}

#[utoipa::path(
    get,
    path = "/admin/users",
    tag = "admin",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "All users", body = [UserResponse]),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Admin role required", body = ErrorResponse),
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users ORDER BY created_at");
    let users = sqlx::query_as::<_, User>(&sql)
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

#[utoipa::path(
    patch,
    path = "/admin/users/{id}",
    tag = "admin",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "User id")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Updated user", body = UserResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 403, description = "Admin role required", body = ErrorResponse),
    )
)]
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _admin: AdminUser,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    if let Some(role) = payload.role.as_deref() {
        if role != "user" && role != "admin" {
            return Err(AppError::BadRequest(
                "role must be either 'user' or 'admin'".into(),
            ));
        }
    }

    let sql = format!(
        "UPDATE users SET role = COALESCE($2, role), \
         preferred_language = COALESCE($3, preferred_language), updated_at = now() \
         WHERE id = $1 RETURNING {USER_COLUMNS}"
    );
    let user = sqlx::query_as::<_, User>(&sql)
        .bind(id)
        .bind(&payload.role)
        .bind(&payload.preferred_language)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    Ok(Json(user.into()))
}
