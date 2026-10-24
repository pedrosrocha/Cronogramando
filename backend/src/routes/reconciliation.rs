use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthUser,
    error::AppError,
    models::book::{Book, BookResponse, BOOK_COLUMNS},
    reconciliation::{self, ReconcileChoice, RemainingBook},
    routes::{goals, plan},
    state::AppState,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct ReconciliationState {
    /// `on_time` when the book finished within its plan and the remaining
    /// books still fit before the target deadline, `behind` otherwise.
    pub status: String,
    pub planned_duration_mins: i32,
    pub actual_duration_mins: i32,
    /// Actual minus planned; positive means the book overran its plan.
    pub drift_mins: i32,
    pub remaining_demand_mins: i32,
    pub available_mins: i32,
    pub target_deadline: NaiveDate,
    pub requires_decision: bool,
    pub options: Vec<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompleteBookResponse {
    pub book: BookResponse,
    pub next_book: Option<BookResponse>,
    pub reconciliation: ReconciliationState,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReconcileDeadlineRequest {
    /// `stay_on_deadline` (compress remaining books) or `push_deadline`.
    pub choice: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReconcileDeadlineResponse {
    pub book: BookResponse,
    /// `stay_on_deadline`, `push_deadline`, or `forced_push` when the floor was
    /// breached.
    pub applied_choice: String,
    pub forced: bool,
    pub target_deadline: NaiveDate,
    pub warning: Option<String>,
    pub remaining_books: Vec<BookResponse>,
}

impl From<&Book> for RemainingBook {
    fn from(book: &Book) -> Self {
        Self {
            id: book.id,
            title: book.title.clone(),
            planned_duration_mins: book.planned_duration_mins,
            min_duration_mins: book.min_duration_mins,
        }
    }
}

async fn load_remaining_books(
    state: &AppState,
    goal_id: Uuid,
    after_order_index: i32,
) -> Result<Vec<Book>, AppError> {
    let sql = format!(
        "SELECT {BOOK_COLUMNS} FROM books \
         WHERE goal_id = $1 AND order_index > $2 AND status <> 'completed' \
         ORDER BY order_index"
    );
    let books = sqlx::query_as::<_, Book>(&sql)
        .bind(goal_id)
        .bind(after_order_index)
        .fetch_all(&state.pool)
        .await?;
    Ok(books)
}

async fn reload_book(state: &AppState, id: Uuid) -> Result<Book, AppError> {
    let sql = format!("SELECT {BOOK_COLUMNS} FROM books WHERE id = $1");
    let book = sqlx::query_as::<_, Book>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("book not found".into()))?;
    Ok(book)
}

fn behind_warning(
    actual: i32,
    planned: i32,
    remaining_demand: i32,
    available: i32,
    target_deadline: NaiveDate,
) -> String {
    let mut parts = Vec::new();
    if actual > planned {
        parts.push(format!(
            "Book finished {actual} min against a {planned} min plan (+{} min).",
            actual - planned
        ));
    }
    if remaining_demand > available {
        parts.push(format!(
            "Remaining books need {remaining_demand} min but only {available} min are left \
             before {target_deadline}."
        ));
    }
    parts.push(
        "Stay on the deadline to compress the remaining books, or push the deadline.".to_string(),
    );
    parts.join(" ")
}

#[utoipa::path(
    post,
    path = "/books/{id}/complete",
    tag = "reconciliation",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Book id")),
    responses(
        (status = 200, description = "Book completed; next book unlocked", body = CompleteBookResponse),
        (status = 404, description = "Book not found", body = ErrorResponse),
        (status = 409, description = "Book cannot be completed", body = ErrorResponse),
    )
)]
pub async fn complete_book(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(book_id): Path<Uuid>,
) -> Result<(StatusCode, Json<CompleteBookResponse>), AppError> {
    let book = plan::load_book(&state, book_id, &auth).await?;

    if book.status == "completed" {
        return Err(AppError::Conflict("book is already completed".into()));
    }
    if !matches!(book.status.as_str(), "unlocked" | "in_progress") {
        return Err(AppError::Conflict(
            "only an unlocked or in-progress book can be completed".into(),
        ));
    }

    let earlier_pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM books done \
         WHERE done.goal_id = $1 AND done.order_index < $2 \
           AND done.status = 'completed' AND done.reconciliation_status = 'pending'",
    )
    .bind(book.goal_id)
    .bind(book.order_index)
    .fetch_one(&state.pool)
    .await?;
    if earlier_pending > 0 {
        return Err(AppError::Conflict(
            "reconcile the previous book's deadline before completing this book".into(),
        ));
    }

    let actual: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(estimated_duration_mins), 0)::bigint FROM steps WHERE book_id = $1",
    )
    .bind(book.id)
    .fetch_one(&state.pool)
    .await?;
    let actual_mins = actual.clamp(0, i32::MAX as i64) as i32;

    let goal = goals::load_goal(&state, book.goal_id, &auth).await?;
    let remaining = load_remaining_books(&state, goal.id, book.order_index).await?;
    let remaining_specs: Vec<RemainingBook> = remaining.iter().map(Into::into).collect();
    let remaining_demand_mins = reconciliation::remaining_demand(&remaining_specs);
    let today = Utc::now().date_naive();
    let available_mins =
        reconciliation::available_minutes(&goal.availability.0, today, goal.target_deadline);

    let drift_mins = actual_mins - book.planned_duration_mins;
    let on_time = drift_mins <= 0 && remaining_demand_mins <= available_mins;
    let reconciliation_status = if on_time { "on_time" } else { "pending" };
    let warning = if on_time {
        None
    } else {
        Some(behind_warning(
            actual_mins,
            book.planned_duration_mins,
            remaining_demand_mins,
            available_mins,
            goal.target_deadline,
        ))
    };

    let mut tx = state.pool.begin().await?;

    let sql = format!(
        "UPDATE books SET status = 'completed', actual_duration_mins = $2, \
         completed_at = now(), reconciliation_status = $3, reconciliation_warning = $4, \
         updated_at = now() WHERE id = $1 RETURNING {BOOK_COLUMNS}"
    );
    let completed = sqlx::query_as::<_, Book>(&sql)
        .bind(book.id)
        .bind(actual_mins)
        .bind(reconciliation_status)
        .bind(&warning)
        .fetch_one(&mut *tx)
        .await?;

    let next_sql = format!(
        "UPDATE books SET status = 'unlocked', updated_at = now() \
         WHERE goal_id = $1 AND order_index = $2 AND status = 'locked' \
         RETURNING {BOOK_COLUMNS}"
    );
    let next_book = sqlx::query_as::<_, Book>(&next_sql)
        .bind(book.goal_id)
        .bind(book.order_index + 1)
        .fetch_optional(&mut *tx)
        .await?;

    tx.commit().await?;

    let reconciliation = ReconciliationState {
        status: if on_time { "on_time" } else { "behind" }.to_string(),
        planned_duration_mins: completed.planned_duration_mins,
        actual_duration_mins: actual_mins,
        drift_mins,
        remaining_demand_mins,
        available_mins,
        target_deadline: goal.target_deadline,
        requires_decision: !on_time,
        options: if on_time {
            Vec::new()
        } else {
            vec![
                ReconcileChoice::StayOnDeadline.as_str().to_string(),
                ReconcileChoice::PushDeadline.as_str().to_string(),
            ]
        },
        warning,
    };

    Ok((
        StatusCode::OK,
        Json(CompleteBookResponse {
            book: completed.into(),
            next_book: next_book.map(Into::into),
            reconciliation,
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/books/{id}/reconcile-deadline",
    tag = "reconciliation",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Book id")),
    request_body = ReconcileDeadlineRequest,
    responses(
        (status = 200, description = "Deadline reconciled", body = ReconcileDeadlineResponse),
        (status = 400, description = "Invalid choice", body = ErrorResponse),
        (status = 404, description = "Book not found", body = ErrorResponse),
        (status = 409, description = "Book is not awaiting reconciliation", body = ErrorResponse),
    )
)]
pub async fn reconcile_deadline(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(book_id): Path<Uuid>,
    Json(payload): Json<ReconcileDeadlineRequest>,
) -> Result<Json<ReconcileDeadlineResponse>, AppError> {
    let book = plan::load_book(&state, book_id, &auth).await?;
    let goal = goals::load_goal(&state, book.goal_id, &auth).await?;

    if book.status != "completed" {
        return Err(AppError::Conflict(
            "book must be completed before reconciling its deadline".into(),
        ));
    }

    match book.reconciliation_status.as_deref() {
        Some("pending") | None => {}
        Some("on_time") => {
            return Err(AppError::Conflict(
                "book finished on time; no reconciliation is needed".into(),
            ))
        }
        Some(_) => {
            return Err(AppError::Conflict(
                "deadline has already been reconciled for this book".into(),
            ))
        }
    }

    let choice = ReconcileChoice::parse(payload.choice.trim()).ok_or_else(|| {
        AppError::BadRequest("choice must be 'stay_on_deadline' or 'push_deadline'".into())
    })?;

    let remaining = load_remaining_books(&state, goal.id, book.order_index).await?;
    let remaining_specs: Vec<RemainingBook> = remaining.iter().map(Into::into).collect();
    let demand = reconciliation::remaining_demand(&remaining_specs);
    let today = Utc::now().date_naive();
    // The completed book's overrun already consumed part of the horizon, so it
    // is deducted from the capacity left for the remaining books.
    let drift = book
        .actual_duration_mins
        .unwrap_or(0)
        .saturating_sub(book.planned_duration_mins)
        .max(0);

    let (applied_choice, forced, warning, target_deadline, compressed) = match choice {
        ReconcileChoice::StayOnDeadline => {
            let available = reconciliation::available_minutes(
                &goal.availability.0,
                today,
                goal.target_deadline,
            );
            let budget = available.saturating_sub(drift);
            match reconciliation::compress(&remaining_specs, budget) {
                reconciliation::CompressionOutcome::Fits => (
                    choice.as_str().to_string(),
                    false,
                    None,
                    goal.target_deadline,
                    Vec::new(),
                ),
                reconciliation::CompressionOutcome::Compressed(list) => (
                    choice.as_str().to_string(),
                    false,
                    None,
                    goal.target_deadline,
                    list,
                ),
                reconciliation::CompressionOutcome::FloorExceeded {
                    book_title,
                    compressed_mins,
                    floor_mins,
                    ..
                } => {
                    let pushed = reconciliation::deadline_for(&goal.availability.0, today, demand)
                        .max(goal.target_deadline);
                    let warning = format!(
                        "Staying on the {} deadline would compress \"{book_title}\" to \
                         {compressed_mins} min, below its {floor_mins} min minimum. \
                         The deadline was pushed to {pushed} instead.",
                        goal.target_deadline
                    );
                    (
                        "forced_push".to_string(),
                        true,
                        Some(warning),
                        pushed,
                        Vec::new(),
                    )
                }
            }
        }
        ReconcileChoice::PushDeadline => {
            let pushed = reconciliation::deadline_for(&goal.availability.0, today, demand)
                .max(goal.target_deadline);
            (choice.as_str().to_string(), false, None, pushed, Vec::new())
        }
    };

    let mut tx = state.pool.begin().await?;

    if target_deadline != goal.target_deadline {
        sqlx::query("UPDATE goals SET target_deadline = $2, updated_at = now() WHERE id = $1")
            .bind(goal.id)
            .bind(target_deadline)
            .execute(&mut *tx)
            .await?;
    }

    for item in &compressed {
        sqlx::query(
            "UPDATE books SET planned_duration_mins = $2, updated_at = now() WHERE id = $1",
        )
        .bind(item.book_id)
        .bind(item.planned_duration_mins)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE books SET reconciliation_status = $2, reconciliation_warning = $3, \
         updated_at = now() WHERE id = $1",
    )
    .bind(book.id)
    .bind(&applied_choice)
    .bind(&warning)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let updated = reload_book(&state, book.id).await?;
    let remaining_books = load_remaining_books(&state, goal.id, book.order_index).await?;

    Ok(Json(ReconcileDeadlineResponse {
        book: updated.into(),
        applied_choice,
        forced,
        target_deadline,
        warning,
        remaining_books: remaining_books.into_iter().map(Into::into).collect(),
    }))
}
