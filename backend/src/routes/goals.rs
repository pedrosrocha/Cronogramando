use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use sqlx::types::Json as SqlJson;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthUser,
    error::AppError,
    llm::{BookDraft, DecomposeRequest},
    models::{
        book::{Book, BookResponse, BOOK_COLUMNS},
        goal::{Availability, Goal, GoalResponse},
    },
    state::AppState,
};

const GOAL_COLUMNS: &str = "id, user_id, title, why_statement, outcome, success_criteria, \
     deadline, target_deadline, availability, created_at, updated_at";

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGoalRequest {
    pub title: String,
    pub why_statement: Option<String>,
    pub deadline: NaiveDate,
    pub target_deadline: Option<NaiveDate>,
    pub availability: Option<Availability>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateGoalRequest {
    pub title: Option<String>,
    pub why_statement: Option<String>,
    pub outcome: Option<String>,
    pub success_criteria: Option<Vec<String>>,
    pub target_deadline: Option<NaiveDate>,
    pub availability: Option<Availability>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateBookRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub objective: Option<String>,
    pub scope_in: Option<Vec<String>>,
    pub scope_out: Option<Vec<String>>,
    pub prerequisites: Option<Vec<String>>,
    pub assessment: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RedecomposeGoalRequest {
    #[serde(default)]
    pub confirm: bool,
}

fn capacity_minutes(availability: &Availability, horizon_days: i64) -> i32 {
    let days_per_week = availability.days.len().clamp(1, 7) as i64;
    let weekly = days_per_week * availability.minutes_per_day as i64;
    let total = weekly * horizon_days / 7;
    total.max(30) as i32
}

fn fallback_book(title: &str) -> BookDraft {
    BookDraft {
        title: title.to_string(),
        description: format!("Complete \"{title}\"."),
        objective: format!(
            "Ao final, o aluno consegue aplicar os conhecimentos de \"{title}\" em uma tarefa prática."
        ),
        scope_in: vec![title.to_string()],
        scope_out: Vec::new(),
        prerequisites: Vec::new(),
        assessment: format!("Demonstrar \"{title}\" em uma tarefa prática."),
        weight: 1.0,
    }
}

async fn insert_books(
    conn: &mut sqlx::PgConnection,
    goal_id: Uuid,
    drafts: &[BookDraft],
    capacity: i32,
) -> Result<(), AppError> {
    let total_weight: f32 = drafts.iter().map(|d| d.weight).sum::<f32>().max(1.0);
    for (index, draft) in drafts.iter().enumerate() {
        let planned = (((draft.weight / total_weight) * capacity as f32).round() as i32).max(15);
        let min = (((planned as f32) * 0.6).round() as i32).max(10);
        let status = if index == 0 { "unlocked" } else { "locked" };

        sqlx::query(
            "INSERT INTO books (goal_id, order_index, title, description, objective, \
             scope_in, scope_out, prerequisites, assessment, status, \
             planned_duration_mins, min_duration_mins) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(goal_id)
        .bind(index as i32)
        .bind(&draft.title)
        .bind(&draft.description)
        .bind(&draft.objective)
        .bind(SqlJson(draft.scope_in.clone()))
        .bind(SqlJson(draft.scope_out.clone()))
        .bind(SqlJson(draft.prerequisites.clone()))
        .bind(&draft.assessment)
        .bind(status)
        .bind(planned)
        .bind(min)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

pub(crate) async fn load_goal(
    state: &AppState,
    id: Uuid,
    auth: &AuthUser,
) -> Result<Goal, AppError> {
    let sql = format!("SELECT {GOAL_COLUMNS} FROM goals WHERE id = $1");
    let goal = sqlx::query_as::<_, Goal>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("goal not found".into()))?;

    if goal.user_id != auth.id && !auth.is_admin() {
        return Err(AppError::Forbidden(
            "not allowed to access this goal".into(),
        ));
    }
    Ok(goal)
}

async fn load_books(state: &AppState, goal_id: Uuid) -> Result<Vec<Book>, AppError> {
    let sql = format!("SELECT {BOOK_COLUMNS} FROM books WHERE goal_id = $1 ORDER BY order_index");
    let books = sqlx::query_as::<_, Book>(&sql)
        .bind(goal_id)
        .fetch_all(&state.pool)
        .await?;
    Ok(books)
}

fn goal_response(goal: Goal, books: Vec<Book>) -> GoalResponse {
    GoalResponse {
        id: goal.id,
        user_id: goal.user_id,
        title: goal.title,
        why_statement: goal.why_statement,
        outcome: goal.outcome,
        success_criteria: goal.success_criteria.0,
        deadline: goal.deadline,
        target_deadline: goal.target_deadline,
        availability: goal.availability.0,
        created_at: goal.created_at,
        updated_at: goal.updated_at,
        books: books.into_iter().map(BookResponse::from).collect(),
    }
}

#[utoipa::path(
    post,
    path = "/goals",
    tag = "goals",
    security(("bearer_auth" = [])),
    request_body = CreateGoalRequest,
    responses(
        (status = 201, description = "Goal created and decomposed into books", body = GoalResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
    )
)]
pub async fn create_goal(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(payload): Json<CreateGoalRequest>,
) -> Result<(StatusCode, Json<GoalResponse>), AppError> {
    let title = payload.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::BadRequest("title is required".into()));
    }

    let today = Utc::now().date_naive();
    if payload.deadline <= today {
        return Err(AppError::BadRequest(
            "deadline must be in the future".into(),
        ));
    }

    let target_deadline = payload.target_deadline.unwrap_or(payload.deadline);
    let availability = payload.availability.unwrap_or_default();
    let horizon_days = (payload.deadline - today).num_days().max(1);

    let capacity = capacity_minutes(&availability, horizon_days);

    let decomposition = state
        .llm
        .decompose(&DecomposeRequest {
            title: title.clone(),
            why_statement: payload.why_statement.clone(),
            horizon_days,
            capacity_mins: capacity,
            preferred_language: "pt-BR".to_string(),
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "goal decomposition failed");
            AppError::Internal("goal decomposition failed".into())
        })?;

    let mut drafts = decomposition.books;
    if drafts.is_empty() {
        drafts.push(fallback_book(&title));
    }

    let mut tx = state.pool.begin().await?;
    let sql = format!(
        "INSERT INTO goals (user_id, title, why_statement, outcome, success_criteria, \
         deadline, target_deadline, availability) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING {GOAL_COLUMNS}"
    );
    let goal = sqlx::query_as::<_, Goal>(&sql)
        .bind(auth.id)
        .bind(&title)
        .bind(&payload.why_statement)
        .bind(&decomposition.goal_outcome)
        .bind(SqlJson(decomposition.goal_success_criteria))
        .bind(payload.deadline)
        .bind(target_deadline)
        .bind(SqlJson(availability.clone()))
        .fetch_one(&mut *tx)
        .await?;

    insert_books(&mut *tx, goal.id, &drafts, capacity).await?;
    tx.commit().await?;

    let books = load_books(&state, goal.id).await?;
    Ok((StatusCode::CREATED, Json(goal_response(goal, books))))
}

#[utoipa::path(
    get,
    path = "/goals",
    tag = "goals",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Current user's goals", body = [GoalResponse]))
)]
pub async fn list_goals(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<GoalResponse>>, AppError> {
    let sql =
        format!("SELECT {GOAL_COLUMNS} FROM goals WHERE user_id = $1 ORDER BY created_at DESC");
    let goals = sqlx::query_as::<_, Goal>(&sql)
        .bind(auth.id)
        .fetch_all(&state.pool)
        .await?;

    let mut out = Vec::with_capacity(goals.len());
    for goal in goals {
        let books = load_books(&state, goal.id).await?;
        out.push(goal_response(goal, books));
    }
    Ok(Json(out))
}

#[utoipa::path(
    get,
    path = "/goals/{id}",
    tag = "goals",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Goal id")),
    responses(
        (status = 200, description = "Goal with books", body = GoalResponse),
        (status = 404, description = "Goal not found", body = ErrorResponse),
    )
)]
pub async fn get_goal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<GoalResponse>, AppError> {
    let goal = load_goal(&state, id, &auth).await?;
    let books = load_books(&state, goal.id).await?;
    Ok(Json(goal_response(goal, books)))
}

#[utoipa::path(
    patch,
    path = "/goals/{id}",
    tag = "goals",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Goal id")),
    request_body = UpdateGoalRequest,
    responses(
        (status = 200, description = "Updated goal", body = GoalResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 404, description = "Goal not found", body = ErrorResponse),
    )
)]
pub async fn update_goal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateGoalRequest>,
) -> Result<Json<GoalResponse>, AppError> {
    load_goal(&state, id, &auth).await?;

    if let Some(title) = payload.title.as_deref() {
        if title.trim().is_empty() {
            return Err(AppError::BadRequest("title cannot be empty".into()));
        }
    }

    let sql = format!(
        "UPDATE goals SET title = COALESCE($2, title), \
         why_statement = COALESCE($3, why_statement), \
         target_deadline = COALESCE($4, target_deadline), \
         availability = COALESCE($5, availability), \
         outcome = COALESCE($6, outcome), \
         success_criteria = COALESCE($7, success_criteria), updated_at = now() \
         WHERE id = $1 RETURNING {GOAL_COLUMNS}"
    );
    let goal = sqlx::query_as::<_, Goal>(&sql)
        .bind(id)
        .bind(&payload.title)
        .bind(&payload.why_statement)
        .bind(payload.target_deadline)
        .bind(payload.availability.map(SqlJson))
        .bind(&payload.outcome)
        .bind(payload.success_criteria.map(SqlJson))
        .fetch_one(&state.pool)
        .await?;

    let books = load_books(&state, goal.id).await?;
    Ok(Json(goal_response(goal, books)))
}

#[utoipa::path(
    delete,
    path = "/goals/{id}",
    tag = "goals",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Goal id")),
    responses(
        (status = 204, description = "Goal deleted"),
        (status = 404, description = "Goal not found", body = ErrorResponse),
    )
)]
pub async fn delete_goal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    load_goal(&state, id, &auth).await?;
    sqlx::query("DELETE FROM goals WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/goals/{id}/books",
    tag = "goals",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Goal id")),
    responses(
        (status = 200, description = "Ordered books for the goal", body = [BookResponse]),
        (status = 404, description = "Goal not found", body = ErrorResponse),
    )
)]
pub async fn list_goal_books(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<BookResponse>>, AppError> {
    let goal = load_goal(&state, id, &auth).await?;
    let books = load_books(&state, goal.id).await?;
    Ok(Json(books.into_iter().map(BookResponse::from).collect()))
}

#[utoipa::path(
    get,
    path = "/books/{id}",
    tag = "goals",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Book id")),
    responses(
        (status = 200, description = "Book detail", body = BookResponse),
        (status = 404, description = "Book not found", body = ErrorResponse),
    )
)]
pub async fn get_book(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<BookResponse>, AppError> {
    let sql = format!("SELECT {BOOK_COLUMNS} FROM books WHERE id = $1");
    let book = sqlx::query_as::<_, Book>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("book not found".into()))?;

    load_goal(&state, book.goal_id, &auth).await?;
    Ok(Json(book.into()))
}

#[utoipa::path(
    patch,
    path = "/books/{id}",
    tag = "goals",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Book id")),
    request_body = UpdateBookRequest,
    responses(
        (status = 200, description = "Updated module contract", body = BookResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 404, description = "Book not found", body = ErrorResponse),
    )
)]
pub async fn update_book(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateBookRequest>,
) -> Result<Json<BookResponse>, AppError> {
    let sql = format!("SELECT {BOOK_COLUMNS} FROM books WHERE id = $1");
    let book = sqlx::query_as::<_, Book>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("book not found".into()))?;
    load_goal(&state, book.goal_id, &auth).await?;

    if let Some(title) = payload.title.as_deref() {
        if title.trim().is_empty() {
            return Err(AppError::BadRequest("title cannot be empty".into()));
        }
    }
    if let Some(objective) = payload.objective.as_deref() {
        if objective.trim().is_empty() {
            return Err(AppError::BadRequest("objective cannot be empty".into()));
        }
    }
    if let Some(assessment) = payload.assessment.as_deref() {
        if assessment.trim().is_empty() {
            return Err(AppError::BadRequest("assessment cannot be empty".into()));
        }
    }

    let sql = format!(
        "UPDATE books SET title = COALESCE($2, title), \
         description = COALESCE($3, description), \
         objective = COALESCE($4, objective), \
         scope_in = COALESCE($5, scope_in), \
         scope_out = COALESCE($6, scope_out), \
         prerequisites = COALESCE($7, prerequisites), \
         assessment = COALESCE($8, assessment), updated_at = now() \
         WHERE id = $1 RETURNING {BOOK_COLUMNS}"
    );
    let updated = sqlx::query_as::<_, Book>(&sql)
        .bind(book.id)
        .bind(&payload.title)
        .bind(&payload.description)
        .bind(&payload.objective)
        .bind(payload.scope_in.map(SqlJson))
        .bind(payload.scope_out.map(SqlJson))
        .bind(payload.prerequisites.map(SqlJson))
        .bind(&payload.assessment)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(updated.into()))
}

#[utoipa::path(
    post,
    path = "/goals/{id}/redecompose",
    tag = "goals",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Goal id")),
    request_body = RedecomposeGoalRequest,
    responses(
        (status = 200, description = "Goal decomposed again into objective-driven modules", body = GoalResponse),
        (status = 400, description = "Confirmation missing or validation error", body = ErrorResponse),
        (status = 404, description = "Goal not found", body = ErrorResponse),
        (status = 409, description = "Goal already has completed activities", body = ErrorResponse),
    )
)]
pub async fn redecompose_goal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<RedecomposeGoalRequest>,
) -> Result<Json<GoalResponse>, AppError> {
    if !payload.confirm {
        return Err(AppError::BadRequest(
            "confirm must be true to redecompose".into(),
        ));
    }
    let goal = load_goal(&state, id, &auth).await?;

    let completed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM steps s \
         JOIN books b ON b.id = s.book_id \
         JOIN activities a ON a.step_id = s.id \
         WHERE b.goal_id = $1 AND a.completed = true",
    )
    .bind(goal.id)
    .fetch_one(&state.pool)
    .await?;
    if completed > 0 {
        return Err(AppError::Conflict(
            "cannot redecompose a goal with completed activities".into(),
        ));
    }

    let today = Utc::now().date_naive();
    let horizon_days = (goal.target_deadline - today).num_days().max(1);
    let availability = goal.availability.0.clone();
    let capacity = capacity_minutes(&availability, horizon_days);

    let decomposition = state
        .llm
        .decompose(&DecomposeRequest {
            title: goal.title.clone(),
            why_statement: goal.why_statement.clone(),
            horizon_days,
            capacity_mins: capacity,
            preferred_language: "pt-BR".to_string(),
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "goal redecomposition failed");
            AppError::Internal("goal redecomposition failed".into())
        })?;

    let mut drafts = decomposition.books;
    if drafts.is_empty() {
        drafts.push(fallback_book(&goal.title));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM books WHERE goal_id = $1")
        .bind(goal.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE goals SET outcome = $2, success_criteria = $3, updated_at = now() WHERE id = $1",
    )
    .bind(goal.id)
    .bind(&decomposition.goal_outcome)
    .bind(SqlJson(decomposition.goal_success_criteria))
    .execute(&mut *tx)
    .await?;
    insert_books(&mut *tx, goal.id, &drafts, capacity).await?;
    tx.commit().await?;

    let refreshed = load_goal(&state, goal.id, &auth).await?;
    let books = load_books(&state, goal.id).await?;
    Ok(Json(goal_response(refreshed, books)))
}
