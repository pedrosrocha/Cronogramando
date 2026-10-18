use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::auth::handlers::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest};
use crate::error::ErrorResponse;
use crate::models::activity::ActivityResponse;
use crate::models::book::BookResponse;
use crate::models::goal::{Availability, GoalResponse};
use crate::models::step::StepResponse;
use crate::models::user::UserResponse;
use crate::routes::admin::UpdateUserRequest;
use crate::routes::goals::{CreateGoalRequest, UpdateGoalRequest};
use crate::routes::plan::{
    BookPlanResponse, CompleteActivityRequest, CreateActivityRequest, CreateStepRequest,
    UpdateStepRequest,
};

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
        crate::routes::goals::create_goal,
        crate::routes::goals::list_goals,
        crate::routes::goals::get_goal,
        crate::routes::goals::update_goal,
        crate::routes::goals::delete_goal,
        crate::routes::goals::list_goal_books,
        crate::routes::goals::get_book,
        crate::routes::plan::get_plan,
        crate::routes::plan::generate_plan,
        crate::routes::plan::create_step,
        crate::routes::plan::update_step,
        crate::routes::plan::delete_step,
        crate::routes::plan::regenerate_resource,
        crate::routes::plan::complete_activity,
    ),
    components(schemas(
        RegisterRequest,
        LoginRequest,
        RefreshRequest,
        AuthResponse,
        UserResponse,
        UpdateUserRequest,
        Availability,
        CreateGoalRequest,
        UpdateGoalRequest,
        GoalResponse,
        BookResponse,
        CreateStepRequest,
        UpdateStepRequest,
        CreateActivityRequest,
        CompleteActivityRequest,
        StepResponse,
        ActivityResponse,
        BookPlanResponse,
        ErrorResponse,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication and account"),
        (name = "admin", description = "Administrative operations"),
        (name = "goals", description = "Goals and their decomposed books"),
        (name = "plan", description = "Per-book plans: steps and activities"),
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
