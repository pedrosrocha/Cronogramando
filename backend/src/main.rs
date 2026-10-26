mod auth;
mod config;
mod content;
mod db;
mod error;
mod llm;
mod models;
mod openapi;
mod pdf;
mod reconciliation;
mod routes;
mod scheduler;
mod state;

use std::sync::Arc;

use axum::{
    routing::{get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{config::Config, openapi::ApiDoc, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    let cfg = Arc::new(Config::from_env());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;
    db::run_migrations(&pool).await?;

    let resolver = content::build_resolver(&cfg, pool.clone())?;
    let pdf = pdf::build(&cfg)?;
    let state = AppState {
        pool,
        cfg: cfg.clone(),
        llm: llm::build(&cfg),
        planner: llm::build_planner(&cfg),
        resolver,
        pdf,
        translator: llm::build_translator(&cfg),
    };
    db::seed_admin(&state).await?;

    let app = Router::new()
        .route("/health", get(routes::health::health))
        .route("/auth/register", post(auth::handlers::register))
        .route("/auth/login", post(auth::handlers::login))
        .route("/auth/refresh", post(auth::handlers::refresh))
        .route("/auth/me", get(auth::handlers::me))
        .route("/admin/users", get(routes::admin::list_users))
        .route("/admin/users/:id", patch(routes::admin::update_user))
        .route(
            "/goals",
            post(routes::goals::create_goal).get(routes::goals::list_goals),
        )
        .route(
            "/goals/:id",
            get(routes::goals::get_goal)
                .patch(routes::goals::update_goal)
                .delete(routes::goals::delete_goal),
        )
        .route("/goals/:id/books", get(routes::goals::list_goal_books))
        .route(
            "/goals/:id/redecompose",
            post(routes::goals::redecompose_goal),
        )
        .route(
            "/books/:id",
            get(routes::goals::get_book).patch(routes::goals::update_book),
        )
        .route(
            "/books/:id/complete",
            post(routes::reconciliation::complete_book),
        )
        .route(
            "/books/:id/reconcile-deadline",
            post(routes::reconciliation::reconcile_deadline),
        )
        .route("/books/:id/plan", get(routes::plan::get_plan))
        .route(
            "/books/:id/generate-schedule",
            post(routes::plan::generate_plan),
        )
        .route("/books/:id/steps", post(routes::plan::create_step))
        .route(
            "/steps/:id",
            patch(routes::plan::update_step).delete(routes::plan::delete_step),
        )
        .route(
            "/steps/:id/regenerate-resource",
            post(routes::plan::regenerate_resource),
        )
        .route(
            "/steps/:id/generate-lesson",
            post(routes::plan::generate_lesson),
        )
        .route("/activities/:id", get(routes::plan::get_activity))
        .route(
            "/activities/:id/complete",
            patch(routes::plan::complete_activity),
        )
        .route(
            "/activities/:id/feedback",
            post(routes::plan::activity_feedback),
        )
        .route("/activities/:id/trace", get(routes::plan::activity_trace))
        .route("/activities/:id/content", get(routes::pdf::get_content))
        .route(
            "/activities/:id/translation",
            get(routes::pdf::get_translation),
        )
        .route("/schedule", get(routes::schedule::list_schedule))
        .route("/schedule/:slot_id", patch(routes::schedule::move_slot))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    tracing::info!("listening on {}", cfg.bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
