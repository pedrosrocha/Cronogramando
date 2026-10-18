use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::auth::handlers::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest};
use crate::error::ErrorResponse;
use crate::models::user::UserResponse;
use crate::routes::admin::UpdateUserRequest;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health,
        crate::auth::handlers::register,
        crate::auth::handlers::login,
        crate::auth::handlers::refresh,
        crate::auth::handlers::me,
        crate::routes::admin::list_users,
        crate::routes::admin::update_user,
    ),
    components(schemas(
        RegisterRequest,
        LoginRequest,
        RefreshRequest,
        AuthResponse,
        UserResponse,
        UpdateUserRequest,
        ErrorResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication and account"),
        (name = "admin", description = "Administrative operations"),
        (name = "system", description = "Service health"),
    )
)]
pub struct ApiDoc;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
