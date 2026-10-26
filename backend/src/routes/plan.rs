use std::collections::HashMap;

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
    content::{query, types::ResolveRequest},
    error::AppError,
    llm::PlanRequest,
    models::{
        activity::{Activity, ActivityResponse},
        book::{Book, BookResponse, BOOK_COLUMNS},
        step::{Step, StepResponse},
    },
    routes::goals,
    state::AppState,
};

const STEP_COLUMNS: &str = "id, book_id, order_index, title, learning_outcome, success_criteria, \
     content_intent, activity_kind, estimated_duration_mins, references_step_id, \
     created_at, updated_at";
const ACTIVITY_COLUMNS: &str = "id, step_id, type, resource_url, embed_id, \
     source_connector, source_category, language, content_body, content_meta, \
     quality_status, selection_reason, completed, created_at, updated_at";

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateActivityRequest {
    #[serde(rename = "type")]
    pub activity_type: String,
    pub resource_url: Option<String>,
    pub embed_id: Option<String>,
    pub source_connector: Option<String>,
    pub source_category: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateStepRequest {
    pub title: String,
    pub activity_kind: String,
    pub estimated_duration_mins: i32,
    pub references_step_id: Option<Uuid>,
    pub learning_outcome: Option<String>,
    pub success_criteria: Option<String>,
    pub content_intent: Option<String>,
    pub activity: Option<CreateActivityRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStepRequest {
    pub title: Option<String>,
    pub activity_kind: Option<String>,
    pub estimated_duration_mins: Option<i32>,
    pub references_step_id: Option<Uuid>,
    pub order_index: Option<i32>,
    pub learning_outcome: Option<String>,
    pub success_criteria: Option<String>,
    pub content_intent: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteActivityRequest {
    pub completed: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FeedbackRequest {
    pub rating: String,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub comment: Option<String>,
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
        "video" | "podcast" | "pdf" | "article" | "quiz" | "task" | "lesson"
    )
}

fn valid_content_intent(intent: &str) -> bool {
    matches!(
        intent,
        "explanation" | "worked_example" | "practice" | "assessment" | "review"
    )
}

pub(crate) async fn preferred_language(
    state: &AppState,
    user_id: Uuid,
) -> Result<String, AppError> {
    let language: Option<String> =
        sqlx::query_scalar("SELECT preferred_language FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    Ok(language.unwrap_or_else(|| "pt-BR".to_string()))
}

pub(crate) async fn load_book(
    state: &AppState,
    id: Uuid,
    auth: &AuthUser,
) -> Result<Book, AppError> {
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

pub(crate) async fn load_activity_owned(
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
    let sql =
        format!("SELECT {ACTIVITY_COLUMNS} FROM activities WHERE step_id = $1 ORDER BY created_at");
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
        learning_outcome: step.learning_outcome,
        success_criteria: step.success_criteria,
        content_intent: step.content_intent,
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

    let pending_reconciliation: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM books done \
         JOIN books b ON b.goal_id = done.goal_id \
         WHERE b.id = $1 AND done.order_index < b.order_index \
           AND done.status = 'completed' AND done.reconciliation_status = 'pending'",
    )
    .bind(book.id)
    .fetch_one(&state.pool)
    .await?;
    if pending_reconciliation > 0 {
        return Err(AppError::Conflict(
            "reconcile the previous book's deadline before generating this plan".into(),
        ));
    }

    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM steps WHERE book_id = $1")
        .bind(book.id)
        .fetch_one(&state.pool)
        .await?;
    if existing > 0 {
        return Err(AppError::Conflict(
            "plan already generated for this book".into(),
        ));
    }

    let language = preferred_language(&state, auth.id).await?;
    let goal = goals::load_goal(&state, book.goal_id, &auth).await?;
    let previous_book_objective: Option<String> = sqlx::query_scalar(
        "SELECT objective FROM books WHERE goal_id = $1 AND order_index < $2 \
         ORDER BY order_index DESC LIMIT 1",
    )
    .bind(book.goal_id)
    .bind(book.order_index)
    .fetch_optional(&state.pool)
    .await?;
    let next_book_objective: Option<String> = sqlx::query_scalar(
        "SELECT objective FROM books WHERE goal_id = $1 AND order_index > $2 \
         ORDER BY order_index ASC LIMIT 1",
    )
    .bind(book.goal_id)
    .bind(book.order_index)
    .fetch_optional(&state.pool)
    .await?;

    let mut drafts = state
        .planner
        .plan(&PlanRequest {
            goal_title: goal.title.clone(),
            goal_outcome: goal.outcome.clone(),
            why_statement: goal.why_statement.clone(),
            book_title: book.title.clone(),
            book_description: book.description.clone(),
            book_objective: book.objective.clone(),
            book_scope_in: book.scope_in.0.clone(),
            book_scope_out: book.scope_out.0.clone(),
            book_prerequisites: book.prerequisites.0.clone(),
            book_assessment: book.assessment.clone(),
            previous_book_objective,
            next_book_objective,
            planned_duration_mins: book.planned_duration_mins,
            preferred_language: language.clone(),
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "plan generation failed");
            AppError::Internal("plan generation failed".into())
        })?;

    let mut resolved_cache: HashMap<String, crate::content::types::ResolvedResource> =
        HashMap::new();
    let mut resolved_results: Vec<Option<crate::content::types::ResolvedResource>> =
        vec![None; drafts.len()];
    for (index, draft) in drafts.iter_mut().enumerate() {
        draft.activity.language = language.clone();
        if !matches!(
            draft.activity.activity_type.as_str(),
            "video" | "pdf" | "article" | "podcast"
        ) {
            continue;
        }

        let cache_key = format!(
            "{}|{}",
            query::build(&book.title, &draft.title),
            draft.learning_outcome
        );
        let resolved = if let Some(cached) = resolved_cache.get(&cache_key) {
            cached.clone()
        } else {
            let resolved = state
                .resolver
                .resolve(&ResolveRequest {
                    goal_outcome: goal.outcome.clone(),
                    module_title: book.title.clone(),
                    module_objective: book.objective.clone(),
                    module_scope_in: book.scope_in.0.clone(),
                    module_scope_out: book.scope_out.0.clone(),
                    step_title: draft.title.clone(),
                    step_outcome: draft.learning_outcome.clone(),
                    success_criteria: draft.success_criteria.clone(),
                    content_intent: draft.content_intent.clone(),
                    activity_type: draft.activity.activity_type.clone(),
                    estimated_duration_mins: draft.estimated_duration_mins,
                    language: language.clone(),
                })
                .await;
            if resolved.used_fallback {
                tracing::info!(
                    category = resolved.category.as_str(),
                    query = %cache_key,
                    "resource resolution used a fallback"
                );
            }
            resolved_cache.insert(cache_key, resolved.clone());
            resolved
        };

        if resolved.lesson.is_some() {
            draft.activity.activity_type = "lesson".to_string();
            draft.activity.resource_url = None;
            draft.activity.embed_id = None;
            draft.activity.source_connector = Some("llm_tutor".to_string());
            draft.activity.source_category = Some(resolved.category.as_str().to_string());
            draft.activity.language = language.clone();
            draft.activity_kind = "resource".to_string();
        } else if let Some(candidate) = resolved
            .candidates
            .iter()
            .find(|candidate| candidate.activity_type == draft.activity.activity_type)
            .or_else(|| resolved.candidates.first())
        {
            draft.activity.activity_type = candidate.activity_type.clone();
            draft.activity.resource_url = candidate.url.clone();
            draft.activity.embed_id = candidate.embed_id.clone();
            draft.activity.source_connector = Some(candidate.connector.as_str().to_string());
            draft.activity.source_category = Some(candidate.category.as_str().to_string());
            draft.activity.language = candidate.language.clone();
            if candidate.activity_type == "task" {
                draft.activity_kind = "task".to_string();
            }
        }
        resolved_results[index] = Some(resolved);
    }

    let mut tx = state.pool.begin().await?;
    let mut ids: Vec<Uuid> = Vec::with_capacity(drafts.len());
    for (index, draft) in drafts.iter().enumerate() {
        let references_step_id = draft
            .references_index
            .and_then(|ref_index| ids.get(ref_index).copied());

        let sql = format!(
            "INSERT INTO steps (book_id, order_index, title, learning_outcome, success_criteria, \
             content_intent, activity_kind, estimated_duration_mins, references_step_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING {STEP_COLUMNS}"
        );
        let step = sqlx::query_as::<_, Step>(&sql)
            .bind(book.id)
            .bind(index as i32)
            .bind(&draft.title)
            .bind(&draft.learning_outcome)
            .bind(&draft.success_criteria)
            .bind(&draft.content_intent)
            .bind(&draft.activity_kind)
            .bind(draft.estimated_duration_mins)
            .bind(references_step_id)
            .fetch_one(&mut *tx)
            .await?;

        let (content_body, content_meta, quality_status, selection_reason) =
            match resolved_results[index].as_ref() {
                Some(resolved) => (
                    resolved
                        .lesson
                        .as_ref()
                        .map(|lesson| lesson.content_body.clone()),
                    Some(resolved.trace.clone()),
                    resolved.quality_status.clone(),
                    Some(resolved.selection_reason.clone()),
                ),
                None => (None, None, "ok".to_string(), None),
            };

        sqlx::query(
            "INSERT INTO activities (step_id, type, resource_url, embed_id, source_connector, \
             source_category, language, content_body, content_meta, quality_status, \
             selection_reason) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(step.id)
        .bind(&draft.activity.activity_type)
        .bind(&draft.activity.resource_url)
        .bind(&draft.activity.embed_id)
        .bind(&draft.activity.source_connector)
        .bind(&draft.activity.source_category)
        .bind(&draft.activity.language)
        .bind(&content_body)
        .bind(&content_meta)
        .bind(&quality_status)
        .bind(&selection_reason)
        .execute(&mut *tx)
        .await?;

        ids.push(step.id);
    }

    sqlx::query("UPDATE books SET status = 'in_progress', updated_at = now() WHERE id = $1 AND status <> 'completed'")
        .bind(book.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    crate::routes::schedule::schedule_pending_goal(&state, book.goal_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(build_book_plan(&state, book.id).await?),
    ))
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
    if let Some(intent) = payload.content_intent.as_deref() {
        if !valid_content_intent(intent) {
            return Err(AppError::BadRequest(
                "content_intent must be explanation, worked_example, practice, assessment or review"
                    .into(),
            ));
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

    let next_index: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(order_index) + 1, 0) FROM steps WHERE book_id = $1",
    )
    .bind(book.id)
    .fetch_one(&state.pool)
    .await?;

    let mut tx = state.pool.begin().await?;
    let learning_outcome = payload
        .learning_outcome
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "Ao final desta etapa, o aluno consegue aplicar {}.",
                payload.title.trim()
            )
        });
    let success_criteria = payload
        .success_criteria
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("Concluir {} e explicar o resultado.", payload.title.trim()));
    let content_intent = payload.content_intent.clone().unwrap_or_else(|| {
        match payload.activity_kind.as_str() {
            "quiz" => "assessment",
            "task" => "practice",
            _ => "explanation",
        }
        .to_string()
    });
    let sql = format!(
        "INSERT INTO steps (book_id, order_index, title, learning_outcome, success_criteria, \
         content_intent, activity_kind, estimated_duration_mins, references_step_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING {STEP_COLUMNS}"
    );
    let step = sqlx::query_as::<_, Step>(&sql)
        .bind(book.id)
        .bind(next_index)
        .bind(payload.title.trim())
        .bind(learning_outcome)
        .bind(success_criteria)
        .bind(content_intent)
        .bind(&payload.activity_kind)
        .bind(payload.estimated_duration_mins)
        .bind(payload.references_step_id)
        .fetch_one(&mut *tx)
        .await?;

    if let Some(activity) = &payload.activity {
        sqlx::query(
            "INSERT INTO activities (step_id, type, resource_url, embed_id, source_connector, \
             source_category, language, content_body, content_meta, quality_status, \
             selection_reason) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(step.id)
        .bind(&activity.activity_type)
        .bind(&activity.resource_url)
        .bind(&activity.embed_id)
        .bind(&activity.source_connector)
        .bind(&activity.source_category)
        .bind(
            activity
                .language
                .clone()
                .unwrap_or_else(|| "pt-BR".to_string()),
        )
        .bind(Option::<String>::None)
        .bind(Option::<serde_json::Value>::None)
        .bind("ok")
        .bind(Option::<String>::None)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    crate::routes::schedule::schedule_pending_goal(&state, book.goal_id).await?;

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
    if let Some(intent) = payload.content_intent.as_deref() {
        if !valid_content_intent(intent) {
            return Err(AppError::BadRequest(
                "content_intent must be explanation, worked_example, practice, assessment or review"
                    .into(),
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
         references_step_id = COALESCE($5, references_step_id), \
         learning_outcome = COALESCE($6, learning_outcome), \
         success_criteria = COALESCE($7, success_criteria), \
         content_intent = COALESCE($8, content_intent), updated_at = now() \
         WHERE id = $1 RETURNING {STEP_COLUMNS}"
    );
    let step = sqlx::query_as::<_, Step>(&sql)
        .bind(step.id)
        .bind(&payload.title)
        .bind(&payload.activity_kind)
        .bind(payload.estimated_duration_mins)
        .bind(payload.references_step_id)
        .bind(&payload.learning_outcome)
        .bind(&payload.success_criteria)
        .bind(&payload.content_intent)
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
    let (step, book) = load_step_owned(&state, step_id, &auth).await?;
    let goal = goals::load_goal(&state, book.goal_id, &auth).await?;
    let language = preferred_language(&state, auth.id).await?;
    let existing = load_activities(&state, step.id).await?;
    let current_url = existing
        .first()
        .and_then(|activity| activity.resource_url.clone());
    let current_type = existing
        .first()
        .map(|activity| activity.activity_type.clone())
        .unwrap_or_else(|| "article".to_string());

    let rejected: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT a.resource_url FROM activity_feedback f \
         JOIN activities a ON a.id = f.activity_id \
         JOIN steps s ON s.id = a.step_id \
         WHERE f.user_id = $1 AND f.rating = -1 AND s.book_id = $2 \
           AND a.resource_url IS NOT NULL",
    )
    .bind(auth.id)
    .bind(book.id)
    .fetch_all(&state.pool)
    .await?;

    let resolved = state
        .resolver
        .resolve(&ResolveRequest {
            goal_outcome: goal.outcome.clone(),
            module_title: book.title.clone(),
            module_objective: book.objective.clone(),
            module_scope_in: book.scope_in.0.clone(),
            module_scope_out: book.scope_out.0.clone(),
            step_title: step.title.clone(),
            step_outcome: step.learning_outcome.clone(),
            success_criteria: step.success_criteria.clone(),
            content_intent: step.content_intent.clone(),
            activity_type: current_type,
            estimated_duration_mins: step.estimated_duration_mins,
            language: language.clone(),
        })
        .await;

    let mut activity_kind = step.activity_kind.clone();
    let (
        activity_type,
        resource_url,
        embed_id,
        source_connector,
        source_category,
        activity_language,
        content_body,
    ) = if let Some(lesson) = &resolved.lesson {
        activity_kind = "resource".to_string();
        (
            "lesson".to_string(),
            None,
            None,
            Some("llm_tutor".to_string()),
            Some(resolved.category.as_str().to_string()),
            language.clone(),
            Some(lesson.content_body.clone()),
        )
    } else {
        let candidate = resolved
            .candidates
            .iter()
            .find(|candidate| {
                let url = candidate.url.as_deref();
                url != current_url.as_deref()
                    && !rejected
                        .iter()
                        .any(|rejected| Some(rejected.as_str()) == url)
            })
            .or_else(|| resolved.candidates.first())
            .ok_or_else(|| AppError::Internal("no resource candidate available".into()))?;
        if candidate.activity_type == "task" {
            activity_kind = "task".to_string();
        }
        (
            candidate.activity_type.clone(),
            candidate.url.clone(),
            candidate.embed_id.clone(),
            Some(candidate.connector.as_str().to_string()),
            Some(candidate.category.as_str().to_string()),
            candidate.language.clone(),
            None,
        )
    };

    if activity_kind != step.activity_kind {
        sqlx::query("UPDATE steps SET activity_kind = $2, updated_at = now() WHERE id = $1")
            .bind(step.id)
            .bind(&activity_kind)
            .execute(&state.pool)
            .await?;
    }

    let content_meta = Some(resolved.trace.clone());
    let quality_status = resolved.quality_status.clone();
    let selection_reason = Some(resolved.selection_reason.clone());

    match existing.first() {
        Some(activity) => {
            sqlx::query(
                "UPDATE activities SET type = $2, resource_url = $3, embed_id = $4, \
                 source_connector = $5, source_category = $6, language = $7, \
                 content_body = $8, content_meta = $9, quality_status = $10, \
                 selection_reason = $11, updated_at = now() \
                 WHERE id = $1",
            )
            .bind(activity.id)
            .bind(&activity_type)
            .bind(&resource_url)
            .bind(&embed_id)
            .bind(&source_connector)
            .bind(&source_category)
            .bind(&activity_language)
            .bind(&content_body)
            .bind(&content_meta)
            .bind(&quality_status)
            .bind(&selection_reason)
            .execute(&state.pool)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO activities (step_id, type, resource_url, embed_id, source_connector, \
                 source_category, language, content_body, content_meta, quality_status, \
                 selection_reason) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(step.id)
            .bind(&activity_type)
            .bind(&resource_url)
            .bind(&embed_id)
            .bind(&source_connector)
            .bind(&source_category)
            .bind(&activity_language)
            .bind(&content_body)
            .bind(&content_meta)
            .bind(&quality_status)
            .bind(&selection_reason)
            .execute(&state.pool)
            .await?;
        }
    }

    let sql = format!("SELECT {STEP_COLUMNS} FROM steps WHERE id = $1");
    let step = sqlx::query_as::<_, Step>(&sql)
        .bind(step.id)
        .fetch_one(&state.pool)
        .await?;
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

fn activity_domain(url: Option<&str>) -> String {
    url.and_then(|url| url::Url::parse(url).ok())
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_default()
}

#[utoipa::path(
    post,
    path = "/steps/{id}/generate-lesson",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Step id")),
    responses(
        (status = 200, description = "Step with a generated micro-lesson", body = StepResponse),
        (status = 404, description = "Step not found", body = ErrorResponse),
        (status = 500, description = "Content tutor is not configured", body = ErrorResponse),
    )
)]
pub async fn generate_lesson(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(step_id): Path<Uuid>,
) -> Result<Json<StepResponse>, AppError> {
    let (step, book) = load_step_owned(&state, step_id, &auth).await?;
    let goal = goals::load_goal(&state, book.goal_id, &auth).await?;
    let language = preferred_language(&state, auth.id).await?;
    let lesson = state
        .resolver
        .lesson(&ResolveRequest {
            goal_outcome: goal.outcome.clone(),
            module_title: book.title.clone(),
            module_objective: book.objective.clone(),
            module_scope_in: book.scope_in.0.clone(),
            module_scope_out: book.scope_out.0.clone(),
            step_title: step.title.clone(),
            step_outcome: step.learning_outcome.clone(),
            success_criteria: step.success_criteria.clone(),
            content_intent: step.content_intent.clone(),
            activity_type: "article".to_string(),
            estimated_duration_mins: step.estimated_duration_mins,
            language: language.clone(),
        })
        .await
        .ok_or_else(|| {
            AppError::Internal("content tutor is unavailable or the LLM returned no lesson".into())
        })?;

    let existing = load_activities(&state, step.id).await?;
    let content_meta = serde_json::json!({ "lesson": true, "on_demand": true });
    let reason = format!("generated on demand: {}", lesson.title);
    match existing.first() {
        Some(activity) => {
            sqlx::query(
                "UPDATE activities SET type = 'lesson', resource_url = NULL, embed_id = NULL, \
                 source_connector = 'llm_tutor', language = $2, content_body = $3, \
                 content_meta = $4, quality_status = 'fallback', selection_reason = $5, \
                 updated_at = now() WHERE id = $1",
            )
            .bind(activity.id)
            .bind(&language)
            .bind(&lesson.content_body)
            .bind(&content_meta)
            .bind(&reason)
            .execute(&state.pool)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO activities (step_id, type, resource_url, embed_id, source_connector, \
                 source_category, language, content_body, content_meta, quality_status, \
                 selection_reason) VALUES ($1, 'lesson', NULL, NULL, 'llm_tutor', NULL, $2, $3, $4, \
                 'fallback', $5)",
            )
            .bind(step.id)
            .bind(&language)
            .bind(&lesson.content_body)
            .bind(&content_meta)
            .bind(&reason)
            .execute(&state.pool)
            .await?;
        }
    }

    if step.activity_kind != "resource" {
        sqlx::query(
            "UPDATE steps SET activity_kind = 'resource', updated_at = now() WHERE id = $1",
        )
        .bind(step.id)
        .execute(&state.pool)
        .await?;
    }

    let sql = format!("SELECT {STEP_COLUMNS} FROM steps WHERE id = $1");
    let step = sqlx::query_as::<_, Step>(&sql)
        .bind(step.id)
        .fetch_one(&state.pool)
        .await?;
    let activities = load_activities(&state, step.id).await?;
    Ok(Json(step_response(step, activities)))
}

#[utoipa::path(
    post,
    path = "/activities/{id}/feedback",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Activity id")),
    request_body = FeedbackRequest,
    responses(
        (status = 200, description = "Updated activity", body = ActivityResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 404, description = "Activity not found", body = ErrorResponse),
    )
)]
pub async fn activity_feedback(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(activity_id): Path<Uuid>,
    Json(payload): Json<FeedbackRequest>,
) -> Result<Json<ActivityResponse>, AppError> {
    let activity = load_activity_owned(&state, activity_id, &auth).await?;
    let rating: i16 = match payload.rating.trim().to_lowercase().as_str() {
        "helpful" => 1,
        "not_helpful" | "not-helpful" | "unhelpful" => -1,
        _ => {
            return Err(AppError::BadRequest(
                "rating must be 'helpful' or 'not_helpful'".into(),
            ))
        }
    };
    const REASONS: [&str; 7] = [
        "too_long",
        "unrelated",
        "not_digitized",
        "broken_link",
        "wrong_language",
        "too_advanced",
        "other",
    ];
    if let Some(reason) = payload
        .reasons
        .iter()
        .find(|reason| !REASONS.contains(&reason.as_str()))
    {
        return Err(AppError::BadRequest(format!(
            "unknown feedback reason: {reason}"
        )));
    }

    sqlx::query(
        "INSERT INTO activity_feedback (activity_id, user_id, rating, reasons, comment) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (activity_id, user_id) DO UPDATE SET rating = EXCLUDED.rating, \
         reasons = EXCLUDED.reasons, comment = EXCLUDED.comment, created_at = now()",
    )
    .bind(activity.id)
    .bind(auth.id)
    .bind(rating)
    .bind(&payload.reasons)
    .bind(&payload.comment)
    .execute(&state.pool)
    .await?;

    if rating < 0 {
        sqlx::query(
            "UPDATE activities SET quality_status = 'rejected', updated_at = now() WHERE id = $1",
        )
        .bind(activity.id)
        .execute(&state.pool)
        .await?;
    }

    let (helpful, not_helpful) = if rating > 0 { (1, 0) } else { (0, 1) };
    sqlx::query(
        "INSERT INTO source_stats (connector, domain, category, language, helpful, not_helpful) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (connector, domain, category, language) DO UPDATE SET \
         helpful = source_stats.helpful + EXCLUDED.helpful, \
         not_helpful = source_stats.not_helpful + EXCLUDED.not_helpful, updated_at = now()",
    )
    .bind(activity.source_connector.as_deref().unwrap_or("unknown"))
    .bind(activity_domain(activity.resource_url.as_deref()))
    .bind(activity.source_category.as_deref().unwrap_or("unknown"))
    .bind(&activity.language)
    .bind(helpful)
    .bind(not_helpful)
    .execute(&state.pool)
    .await?;

    let updated = load_activity_owned(&state, activity.id, &auth).await?;
    Ok(Json(updated.into()))
}

#[utoipa::path(
    get,
    path = "/activities/{id}/trace",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Activity id")),
    responses(
        (status = 200, description = "Resolution trace for the activity"),
        (status = 404, description = "Activity not found", body = ErrorResponse),
    )
)]
pub async fn activity_trace(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(activity_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let activity = load_activity_owned(&state, activity_id, &auth).await?;
    Ok(Json(
        activity
            .content_meta
            .unwrap_or_else(|| serde_json::json!({ "trace": null })),
    ))
}

#[utoipa::path(
    get,
    path = "/activities/{id}",
    tag = "plan",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Activity id")),
    responses(
        (status = 200, description = "Activity detail", body = ActivityResponse),
        (status = 404, description = "Activity not found", body = ErrorResponse),
    )
)]
pub async fn get_activity(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(activity_id): Path<Uuid>,
) -> Result<Json<ActivityResponse>, AppError> {
    let activity = load_activity_owned(&state, activity_id, &auth).await?;
    Ok(Json(activity.into()))
}
