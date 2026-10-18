use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthUser,
    error::AppError,
    llm::PlanRequest,
    models::{
        activity::{Activity, ActivityResponse},
        book::{Book, BookResponse},
        step::{Step, StepResponse},
    },
    routes::goals,
    state::AppState,
};

const BOOK_COLUMNS: &str = "id, goal_id, order_index, title, description, status, \
     planned_duration_mins, min_duration_mins, actual_duration_mins, created_at, updated_at";
const STEP_COLUMNS: &str = "id, book_id, order_index, title, activity_kind, \
     estimated_duration_mins, references_step_id, created_at, updated_at";
const ACTIVITY_COLUMNS: &str = "id, step_id, type, resource_url, embed_id, \
     source_connector, language, completed, created_at, updated_at";

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateActivityRequest {
    #[serde(rename = "type")]
    pub activity_type: String,
    pub resource_url: Option<String>,
    pub embed_id: Option<String>,
    pub source_connector: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateStepRequest {
    pub title: String,
    pub activity_kind: String,
    pub estimated_duration_mins: i32,
    pub references_step_id: Option<Uuid>,
    pub activity: Option<CreateActivityRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStepRequest {
    pub title: Option<String>,
    pub activity_kind: Option<String>,
    pub estimated_duration_mins: Option<i32>,
    pub references_step_id: Option<Uuid>,
    pub order_index: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteActivityRequest {
    pub completed: bool,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct BookPlanResponse {
    pub book: BookResponse,
    pub steps: Vec<StepResponse>,
}

fn valid_kind(kind: &str) -> bool {
    matches!(kind, "resource" | "quiz" | "task")
}

fn valid_type(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "video" | "podcast" | "pdf" | "article" | "quiz" | "task"
    )
}

async fn load_book(state: &AppState, id: Uuid, auth: &AuthUser) -> Result<Book, AppError> {
    let sql = format!("SELECT {BOOK_COLUMNS} FROM books WHERE id = $1");
    let book = sqlx::query_as::<_, Book>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("book not found".into()))?;
    goals::load_goal(state, book.goal_id, auth).await?;
    Ok(book)
}

async fn load_step_owned(
    state: &AppState,
    id: Uuid,
    auth: &AuthUser,
) -> Result<(Step, Book), AppError> {
    let sql = format!("SELECT {STEP_COLUMNS} FROM steps WHERE id = $1");
    let step = sqlx::query_as::<_, Step>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("step not found".into()))?;
    let book = load_book(state, step.book_id, auth).await?;
    Ok((step, book))
}

async fn load_activity_owned(
    state: &AppState,
    id: Uuid,
    auth: &AuthUser,
) -> Result<Activity, AppError> {
    let sql = format!("SELECT {ACTIVITY_COLUMNS} FROM activities WHERE id = $1");
    let activity = sqlx::query_as::<_, Activity>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("activity not found".into()))?;
    load_step_owned(state, activity.step_id, auth).await?;
    Ok(activity)
}

async fn load_steps(state: &AppState, book_id: Uuid) -> Result<Vec<Step>, AppError> {
    let sql = format!("SELECT {STEP_COLUMNS} FROM steps WHERE book_id = $1 ORDER BY order_index");
    let steps = sqlx::query_as::<_, Step>(&sql)
        .bind(book_id)
        .fetch_all(&state.pool)
        .await?;
    Ok(steps)
}

async fn load_activities(state: &AppState, step_id: Uuid) -> Result<Vec<Activity>, AppError> {
    let sql = format!(
        "SELECT {ACTIVITY_COLUMNS} FROM activities WHERE step_id = $1 ORDER BY created_at"
    );
    let activities = sqlx::query_as::<_, Activity>(&sql)
        .bind(step_id)
        .fetch_all(&state.pool)
        .await?;
    Ok(activities)
}

fn step_response(step: Step, activities: Vec<Activity>) -> StepResponse {
    StepResponse {
        id: step.id,
        book_id: step.book_id,
        order_index: step.order_index,
        title: step.title,
        activity_kind: step.activity_kind,
        estimated_duration_mins: step.estimated_duration_mins,
        references_step_id: step.references_step_id,
        created_at: step.created_at,
        updated_at: step.updated_at,
        activities: activities.into_iter().map(ActivityResponse::from).collect(),
    }
}

async fn build_book_plan(state: &AppState, book_id: Uuid) -> Result<BookPlanResponse, AppError> {
    let sql = format!("SELECT {BOOK_COLUMNS} FROM books WHERE id = $1");
    let book = sqlx::query_as::<_, Book>(&sql)
        .bind(book_id)
        .fetch_one(&state.pool)
        .await?;

    let mut steps = Vec::new();
    for step in load_steps(state, book_id).await? {
        let activities = load_activities(state, step.id).await?;
        steps.push(step_response(step, activities));
    }

    Ok(BookPlanResponse {
        book: book.into(),
        steps,
    })
}

async fn renumber(
    conn: &mut sqlx::PgConnection,
    book_id: Uuid,
    ordered: &[Uuid],
) -> Result<(), AppError> {
    sqlx::query("UPDATE steps SET order_index = -order_index - 1 WHERE book_id = $1")
        .bind(book_id)
        .execute(&mut *conn)
        .await?;
    for (position, id) in ordered.iter().enumerate() {
        sqlx::query("UPDATE steps SET order_index = $2 WHERE id = $1")
            .bind(id)
            .bind(position as i32)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

async fn reorder_step(
    state: &AppState,
    book_id: Uuid,
    step_id: Uuid,
    new_position: i32,
) -> Result<(), AppError> {
    let mut tx = state.pool.begin().await?;
    let mut ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM steps WHERE book_id = $1 ORDER BY order_index")
            .bind(book_id)
            .fetch_all(&mut *tx)
            .await?;
    if let Some(current) = ids.iter().position(|id| *id == step_id) {
        ids.remove(current);
    }
    let position = (new_position.max(0) as usize).min(ids.len());
    ids.insert(position, step_id);
    renumber(&mut tx, book_id, &ids).await?;
    tx.commit().await?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/books/{id}/plan",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Book id")),
    responses(
        (status = 200, description = "Book plan with ordered steps and activities", body = BookPlanResponse),
        (status = 404, description = "Book not found", body = ErrorResponse),
    )
)]
pub async fn get_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(book_id): Path<Uuid>,
) -> Result<Json<BookPlanResponse>, AppError> {
    let book = load_book(&state, book_id, &auth).await?;
    Ok(Json(build_book_plan(&state, book.id).await?))
}

#[utoipa::path(
    post,
    path = "/books/{id}/generate-schedule",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Book id")),
    responses(
        (status = 201, description = "Plan generated for the book", body = BookPlanResponse),
        (status = 404, description = "Book not found", body = ErrorResponse),
        (status = 409, description = "Plan already generated", body = ErrorResponse),
    )
)]
pub async fn generate_plan(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(book_id): Path<Uuid>,
) -> Result<(StatusCode, Json<BookPlanResponse>), AppError> {
    let book = load_book(&state, book_id, &auth).await?;

    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM steps WHERE book_id = $1")
        .bind(book.id)
        .fetch_one(&state.pool)
        .await?;
    if existing > 0 {
        return Err(AppError::Conflict(
            "plan already generated for this book".into(),
        ));
    }

    let drafts = state
        .planner
        .plan(&PlanRequest {
            book_title: book.title.clone(),
            planned_duration_mins: book.planned_duration_mins,
            preferred_language: "pt-BR".to_string(),
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "plan generation failed");
            AppError::Internal("plan generation failed".into())
        })?;

    let mut tx = state.pool.begin().await?;
    let mut ids: Vec<Uuid> = Vec::with_capacity(drafts.len());
    for (index, draft) in drafts.iter().enumerate() {
        let references_step_id = draft
            .references_index
            .and_then(|ref_index| ids.get(ref_index).copied());

        let sql = format!(
            "INSERT INTO steps (book_id, order_index, title, activity_kind, \
             estimated_duration_mins, references_step_id) VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING {STEP_COLUMNS}"
        );
        let step = sqlx::query_as::<_, Step>(&sql)
            .bind(book.id)
            .bind(index as i32)
            .bind(&draft.title)
            .bind(&draft.activity_kind)
            .bind(draft.estimated_duration_mins)
            .bind(references_step_id)
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO activities (step_id, type, resource_url, embed_id, source_connector, language) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(step.id)
        .bind(&draft.activity.activity_type)
        .bind(&draft.activity.resource_url)
        .bind(&draft.activity.embed_id)
        .bind(&draft.activity.source_connector)
        .bind(&draft.activity.language)
        .execute(&mut *tx)
        .await?;

        ids.push(step.id);
    }

    sqlx::query("UPDATE books SET status = 'in_progress', updated_at = now() WHERE id = $1 AND status <> 'completed'")
        .bind(book.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(build_book_plan(&state, book.id).await?)))
}

#[utoipa::path(
    post,
    path = "/books/{id}/steps",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Book id")),
    request_body = CreateStepRequest,
    responses(
        (status = 201, description = "Step created", body = StepResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 404, description = "Book not found", body = ErrorResponse),
    )
)]
pub async fn create_step(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(book_id): Path<Uuid>,
    Json(payload): Json<CreateStepRequest>,
) -> Result<(StatusCode, Json<StepResponse>), AppError> {
    let book = load_book(&state, book_id, &auth).await?;

    if payload.title.trim().is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }
    if !valid_kind(&payload.activity_kind) {
        return Err(AppError::BadRequest(
            "activity_kind must be 'resource', 'quiz' or 'task'".into(),
        ));
    }
    if let Some(activity) = &payload.activity {
        if !valid_type(&activity.activity_type) {
            return Err(AppError::BadRequest("invalid activity type".into()));
        }
    }
    if let Some(references) = payload.references_step_id {
        let exists: Option<i32> =
            sqlx::query_scalar("SELECT order_index FROM steps WHERE id = $1 AND book_id = $2")
                .bind(references)
                .bind(book.id)
                .fetch_optional(&state.pool)
                .await?;
        if exists.is_none() {
            return Err(AppError::BadRequest(
                "references_step_id must point to a step in the same book".into(),
            ));
        }
    }

    let next_index: i32 =
        sqlx::query_scalar("SELECT COALESCE(MAX(order_index) + 1, 0) FROM steps WHERE book_id = $1")
            .bind(book.id)
            .fetch_one(&state.pool)
            .await?;

    let mut tx = state.pool.begin().await?;
    let sql = format!(
        "INSERT INTO steps (book_id, order_index, title, activity_kind, estimated_duration_mins, references_step_id) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING {STEP_COLUMNS}"
    );
    let step = sqlx::query_as::<_, Step>(&sql)
        .bind(book.id)
        .bind(next_index)
        .bind(payload.title.trim())
        .bind(&payload.activity_kind)
        .bind(payload.estimated_duration_mins)
        .bind(payload.references_step_id)
        .fetch_one(&mut *tx)
        .await?;

    if let Some(activity) = &payload.activity {
        sqlx::query(
            "INSERT INTO activities (step_id, type, resource_url, embed_id, source_connector, language) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(step.id)
        .bind(&activity.activity_type)
        .bind(&activity.resource_url)
        .bind(&activity.embed_id)
        .bind(&activity.source_connector)
        .bind(activity.language.clone().unwrap_or_else(|| "pt-BR".to_string()))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let activities = load_activities(&state, step.id).await?;
    Ok((StatusCode::CREATED, Json(step_response(step, activities))))
}

#[utoipa::path(
    patch,
    path = "/steps/{id}",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Step id")),
    request_body = UpdateStepRequest,
    responses(
        (status = 200, description = "Updated step", body = StepResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 404, description = "Step not found", body = ErrorResponse),
    )
)]
pub async fn update_step(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(step_id): Path<Uuid>,
    Json(payload): Json<UpdateStepRequest>,
) -> Result<Json<StepResponse>, AppError> {
    let (step, _book) = load_step_owned(&state, step_id, &auth).await?;

    if let Some(kind) = payload.activity_kind.as_deref() {
        if !valid_kind(kind) {
            return Err(AppError::BadRequest(
                "activity_kind must be 'resource', 'quiz' or 'task'".into(),
            ));
        }
    }
    if let Some(title) = payload.title.as_deref() {
        if title.trim().is_empty() {
            return Err(AppError::BadRequest("title cannot be empty".into()));
        }
    }
    if let Some(references) = payload.references_step_id {
        let exists: Option<i32> =
            sqlx::query_scalar("SELECT order_index FROM steps WHERE id = $1 AND book_id = $2")
                .bind(references)
                .bind(step.book_id)
                .fetch_optional(&state.pool)
                .await?;
        if exists.is_none() {
            return Err(AppError::BadRequest(
                "references_step_id must point to a step in the same book".into(),
            ));
        }
    }

    let sql = format!(
        "UPDATE steps SET title = COALESCE($2, title), \
         activity_kind = COALESCE($3, activity_kind), \
         estimated_duration_mins = COALESCE($4, estimated_duration_mins), \
         references_step_id = COALESCE($5, references_step_id), updated_at = now() \
         WHERE id = $1 RETURNING {STEP_COLUMNS}"
    );
    let step = sqlx::query_as::<_, Step>(&sql)
        .bind(step.id)
        .bind(&payload.title)
        .bind(&payload.activity_kind)
        .bind(payload.estimated_duration_mins)
        .bind(payload.references_step_id)
        .fetch_one(&state.pool)
        .await?;

    if let Some(position) = payload.order_index {
        reorder_step(&state, step.book_id, step.id, position).await?;
        let sql = format!("SELECT {STEP_COLUMNS} FROM steps WHERE id = $1");
        let step = sqlx::query_as::<_, Step>(&sql)
            .bind(step.id)
            .fetch_one(&state.pool)
            .await?;
        let activities = load_activities(&state, step.id).await?;
        return Ok(Json(step_response(step, activities)));
    }

    let activities = load_activities(&state, step.id).await?;
    Ok(Json(step_response(step, activities)))
}

#[utoipa::path(
    delete,
    path = "/steps/{id}",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Step id")),
    responses(
        (status = 204, description = "Step deleted"),
        (status = 404, description = "Step not found", body = ErrorResponse),
    )
)]
pub async fn delete_step(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(step_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let (step, book) = load_step_owned(&state, step_id, &auth).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM steps WHERE id = $1")
        .bind(step.id)
        .execute(&mut *tx)
        .await?;
    let remaining: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM steps WHERE book_id = $1 ORDER BY order_index")
            .bind(book.id)
            .fetch_all(&mut *tx)
            .await?;
    renumber(&mut tx, book.id, &remaining).await?;
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/steps/{id}/regenerate-resource",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Step id")),
    responses(
        (status = 200, description = "Step with a freshly picked resource", body = StepResponse),
        (status = 404, description = "Step not found", body = ErrorResponse),
    )
)]
pub async fn regenerate_resource(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(step_id): Path<Uuid>,
) -> Result<Json<StepResponse>, AppError> {
    let (step, _book) = load_step_owned(&state, step_id, &auth).await?;

    const CYCLE: [&str; 6] = ["video", "pdf", "article", "podcast", "quiz", "task"];
    let existing = load_activities(&state, step.id).await?;

    match existing.into_iter().next() {
        Some(activity) => {
            let current = CYCLE
                .iter()
                .position(|t| *t == activity.activity_type)
                .unwrap_or(0);
            let next_type = CYCLE[(current + 1) % CYCLE.len()];
            let url = if next_type == "quiz" || next_type == "task" {
                None
            } else {
                Some(format!(
                    "https://placeholder.cronogramando.local/regenerated-{}",
                    Uuid::new_v4()
                ))
            };
            sqlx::query(
                "UPDATE activities SET type = $2, resource_url = $3, \
                 source_connector = 'placeholder', updated_at = now() WHERE id = $1",
            )
            .bind(activity.id)
            .bind(next_type)
            .bind(&url)
            .execute(&state.pool)
            .await?;
        }
        None => {
            let url = format!(
                "https://placeholder.cronogramando.local/regenerated-{}",
                Uuid::new_v4()
            );
            sqlx::query(
                "INSERT INTO activities (step_id, type, resource_url, source_connector, language) \
                 VALUES ($1, 'video', $2, 'placeholder', 'pt-BR')",
            )
            .bind(step.id)
            .bind(&url)
            .execute(&state.pool)
            .await?;
        }
    }

    let activities = load_activities(&state, step.id).await?;
    Ok(Json(step_response(step, activities)))
}

#[utoipa::path(
    patch,
    path = "/activities/{id}/complete",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Activity id")),
    request_body = CompleteActivityRequest,
    responses(
        (status = 200, description = "Updated activity", body = ActivityResponse),
        (status = 404, description = "Activity not found", body = ErrorResponse),
    )
)]
pub async fn complete_activity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(activity_id): Path<Uuid>,
    Json(payload): Json<CompleteActivityRequest>,
) -> Result<Json<ActivityResponse>, AppError> {
    let activity = load_activity_owned(&state, activity_id, &auth).await?;

    let sql = format!(
        "UPDATE activities SET completed = $2, updated_at = now() WHERE id = $1 \
         RETURNING {ACTIVITY_COLUMNS}"
    );
    let updated = sqlx::query_as::<_, Activity>(&sql)
        .bind(activity.id)
        .bind(payload.completed)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(updated.into()))
}
