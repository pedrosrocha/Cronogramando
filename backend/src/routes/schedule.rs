use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{Duration, NaiveDate, Utc};
use serde::Deserialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::extractor::AuthUser,
    error::AppError,
    models::{
        goal::Goal,
        schedule::{ScheduleSlotResponse, ScheduleSlotRow},
    },
    routes::goals,
    scheduler::{self, ScheduledStep, StepAllocation},
    state::AppState,
};

const SLOT_SELECT: &str = "SELECT ss.id, ss.step_id, s.title AS step_title, \
     b.id AS book_id, b.title AS book_title, g.id AS goal_id, g.title AS goal_title, \
     ss.slot_date, ss.start_minute, ss.end_minute, \
     (EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id) \
      AND NOT EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id AND a.completed = false)) \
      AS completed \
     FROM schedule_slots ss \
     JOIN steps s ON s.id = ss.step_id \
     JOIN books b ON b.id = s.book_id \
     JOIN goals g ON g.id = b.goal_id";

const GOAL_SELECT: &str = "SELECT id, user_id, title, why_statement, deadline, \
     target_deadline, availability, created_at, updated_at FROM goals WHERE id = $1";

const MAX_RANGE_DAYS: i64 = 366;

#[derive(Debug, Deserialize, ToSchema)]
pub struct ScheduleQuery {
    pub date: Option<NaiveDate>,
    pub goal_id: Option<Uuid>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveSlotRequest {
    pub date: NaiveDate,
}

#[derive(Debug, FromRow)]
struct SlotOwnership {
    step_id: Uuid,
    goal_id: Uuid,
    completed: bool,
}

#[derive(Debug, FromRow)]
struct BusyRow {
    slot_date: NaiveDate,
    minutes: i64,
}

#[derive(Debug, FromRow)]
struct PendingStepRow {
    step_id: Uuid,
    duration_mins: i32,
}

#[utoipa::path(
    get,
    path = "/schedule",
    tag = "schedule",
    security(("bearer_auth" = [])),
    params(
        ("date" = Option<NaiveDate>, Query, description = "Single day (YYYY-MM-DD)"),
        ("goal_id" = Option<Uuid>, Query, description = "Filter by goal"),
        ("from" = Option<NaiveDate>, Query, description = "Range start (defaults to today)"),
        ("to" = Option<NaiveDate>, Query, description = "Range end (defaults to from + 30 days)"),
    ),
    responses(
        (status = 200, description = "Schedule slots ordered by date and time", body = Vec<ScheduleSlotResponse>),
        (status = 400, description = "Invalid date range", body = ErrorResponse),
        (status = 404, description = "Goal not found", body = ErrorResponse),
    )
)]
pub async fn list_schedule(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ScheduleQuery>,
) -> Result<Json<Vec<ScheduleSlotResponse>>, AppError> {
    let today = Utc::now().date_naive();
    let (from, to) = match query.date {
        Some(date) => {
            if query.from.is_some() || query.to.is_some() {
                return Err(AppError::BadRequest(
                    "use either date or from/to, not both".into(),
                ));
            }
            (date, date)
        }
        None => {
            let from = query.from.unwrap_or(today);
            let to = query.to.unwrap_or(from + Duration::days(30));
            if to < from {
                return Err(AppError::BadRequest("to must be on or after from".into()));
            }
            if (to - from).num_days() > MAX_RANGE_DAYS {
                return Err(AppError::BadRequest(
                    "date range cannot exceed 366 days".into(),
                ));
            }
            (from, to)
        }
    };

    if let Some(goal_id) = query.goal_id {
        goals::load_goal(&state, goal_id, &auth).await?;
    }

    let rows = if let Some(goal_id) = query.goal_id {
        let sql = format!(
            "{SLOT_SELECT} WHERE g.id = $1 AND ss.slot_date BETWEEN $2 AND $3 \
             ORDER BY ss.slot_date, ss.start_minute"
        );
        sqlx::query_as::<_, ScheduleSlotRow>(&sql)
            .bind(goal_id)
            .bind(from)
            .bind(to)
            .fetch_all(&state.pool)
            .await?
    } else if auth.is_admin() {
        let sql = format!(
            "{SLOT_SELECT} WHERE ss.slot_date BETWEEN $1 AND $2 \
             ORDER BY ss.slot_date, ss.start_minute"
        );
        sqlx::query_as::<_, ScheduleSlotRow>(&sql)
            .bind(from)
            .bind(to)
            .fetch_all(&state.pool)
            .await?
    } else {
        let sql = format!(
            "{SLOT_SELECT} WHERE g.user_id = $1 AND ss.slot_date BETWEEN $2 AND $3 \
             ORDER BY ss.slot_date, ss.start_minute"
        );
        sqlx::query_as::<_, ScheduleSlotRow>(&sql)
            .bind(auth.id)
            .bind(from)
            .bind(to)
            .fetch_all(&state.pool)
            .await?
    };

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    patch,
    path = "/schedule/{slot_id}",
    tag = "schedule",
    security(("bearer_auth" = [])),
    params(("slot_id" = Uuid, Path, description = "Schedule slot id")),
    request_body = MoveSlotRequest,
    responses(
        (status = 200, description = "Step moved; source and destination days re-fitted", body = Vec<ScheduleSlotResponse>),
        (status = 400, description = "Invalid move", body = ErrorResponse),
        (status = 404, description = "Slot not found", body = ErrorResponse),
        (status = 409, description = "Completed step cannot be moved", body = ErrorResponse),
    )
)]
pub async fn move_slot(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(slot_id): Path<Uuid>,
    Json(payload): Json<MoveSlotRequest>,
) -> Result<Json<Vec<ScheduleSlotResponse>>, AppError> {
    let ownership = sqlx::query_as::<_, SlotOwnership>(
        "SELECT ss.step_id, b.goal_id, \
         (EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id) \
          AND NOT EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id AND a.completed = false)) \
          AS completed \
         FROM schedule_slots ss \
         JOIN steps s ON s.id = ss.step_id \
         JOIN books b ON b.id = s.book_id \
         WHERE ss.id = $1",
    )
    .bind(slot_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("schedule slot not found".into()))?;

    let goal = goals::load_goal(&state, ownership.goal_id, &auth).await?;

    if ownership.completed {
        return Err(AppError::Conflict("completed steps cannot be moved".into()));
    }

    let today = Utc::now().date_naive();
    if payload.date < today {
        return Err(AppError::BadRequest(
            "cannot move a step into the past".into(),
        ));
    }

    let earliest: NaiveDate =
        sqlx::query_scalar("SELECT MIN(slot_date) FROM schedule_slots WHERE step_id = $1")
            .bind(ownership.step_id)
            .fetch_one(&state.pool)
            .await?;
    let from_date = earliest.min(payload.date);

    refit_goal_from(
        &state,
        &goal,
        from_date,
        Some((ownership.step_id, payload.date)),
    )
    .await?;

    let sql = format!("{SLOT_SELECT} WHERE ss.step_id = $1 ORDER BY ss.slot_date, ss.start_minute");
    let rows = sqlx::query_as::<_, ScheduleSlotRow>(&sql)
        .bind(ownership.step_id)
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Schedule every pending step of a goal that has no slots yet. Existing slots
/// stay fixed and count as busy minutes, so new steps fill the gaps without
/// ever overflowing a day.
pub(crate) async fn schedule_pending_goal(state: &AppState, goal_id: Uuid) -> Result<(), AppError> {
    let goal = sqlx::query_as::<_, Goal>(GOAL_SELECT)
        .bind(goal_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("goal not found".into()))?;

    let today = Utc::now().date_naive();
    let pending = pending_steps_without_slots(state, goal.id).await?;
    if pending.is_empty() {
        return Ok(());
    }

    let busy = busy_minutes(state, goal.id, today, &[]).await?;
    let steps: Vec<StepAllocation> = pending
        .iter()
        .map(|row| StepAllocation {
            step_id: row.step_id,
            duration_mins: row.duration_mins.max(1),
        })
        .collect();
    let placements = scheduler::allocate_with_busy(&steps, &goal.availability.0, today, &busy);
    insert_placements(state, &placements).await
}

/// Re-pack pending slots from `from_date` onward. The moved step is pinned to
/// `pinned` (step id, target date); every other step keeps its relative order.
/// Completed slots are never moved and are counted as busy minutes.
async fn refit_goal_from(
    state: &AppState,
    goal: &Goal,
    from_date: NaiveDate,
    pinned: Option<(Uuid, NaiveDate)>,
) -> Result<(), AppError> {
    let rows = sqlx::query_as::<_, ScheduledStepRow>(
        "SELECT s.id AS step_id, s.estimated_duration_mins AS duration_mins, \
         MIN(ss.slot_date) AS slot_date, MIN(ss.start_minute) AS start_minute \
         FROM steps s \
         JOIN books b ON b.id = s.book_id \
         JOIN schedule_slots ss ON ss.step_id = s.id \
         WHERE b.goal_id = $1 \
           AND (EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id AND a.completed = false) \
                OR NOT EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id)) \
         GROUP BY s.id, s.estimated_duration_mins \
         HAVING MAX(ss.slot_date) >= $2",
    )
    .bind(goal.id)
    .bind(from_date)
    .fetch_all(&state.pool)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    if let Some((pinned_id, _)) = pinned {
        if !rows.iter().any(|row| row.step_id == pinned_id) {
            return Err(AppError::Conflict(
                "step has no pending schedule to move".into(),
            ));
        }
    }

    let actual_from = rows
        .iter()
        .map(|row| row.slot_date)
        .min()
        .unwrap_or(from_date)
        .min(from_date);

    let capacity = goal.availability.0.minutes_per_day as i32;
    let step_ids: Vec<Uuid> = rows.iter().map(|row| row.step_id).collect();
    // Completed slots and slots outside this re-fit stay fixed. Slots owned by
    // the steps being re-packed are excluded because they are about to move.
    let mut busy = busy_minutes(state, goal.id, actual_from, &step_ids).await?;
    let mut placements = Vec::new();

    let mut remaining: Vec<ScheduledStep> = rows
        .iter()
        .map(|row| ScheduledStep {
            step_id: row.step_id,
            duration_mins: row.duration_mins.max(1),
            date: row.slot_date,
            start_minute: row.start_minute,
        })
        .collect();

    if let Some((pinned_id, target)) = pinned {
        if !scheduler::is_available_day(&goal.availability.0, target) {
            return Err(AppError::BadRequest(
                "target date is not one of the goal's available days".into(),
            ));
        }

        let pinned_step = remaining
            .iter()
            .find(|step| step.step_id == pinned_id)
            .cloned()
            .ok_or_else(|| AppError::Conflict("step has no pending schedule to move".into()))?;

        let used = busy.get(&target).copied().unwrap_or(0);
        if used >= capacity {
            return Err(AppError::Conflict(
                "target day has no remaining availability".into(),
            ));
        }

        let pinned_alloc = scheduler::allocate_with_busy(
            &[StepAllocation {
                step_id: pinned_id,
                duration_mins: pinned_step.duration_mins,
            }],
            &goal.availability.0,
            target,
            &busy,
        );

        if pinned_alloc.first().map(|slot| slot.date) != Some(target) {
            return Err(AppError::Conflict("target day cannot fit the step".into()));
        }

        for slot in &pinned_alloc {
            *busy.entry(slot.date).or_insert(0) += slot.end_minute - slot.start_minute;
        }
        placements.extend(pinned_alloc);
        remaining.retain(|step| step.step_id != pinned_id);
    }

    let ordered = scheduler::sort_by_time(remaining);
    let allocations: Vec<StepAllocation> = ordered
        .iter()
        .map(|step| StepAllocation {
            step_id: step.step_id,
            duration_mins: step.duration_mins,
        })
        .collect();
    let others =
        scheduler::allocate_with_busy(&allocations, &goal.availability.0, actual_from, &busy);
    placements.extend(others);

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM schedule_slots WHERE step_id = ANY($1) AND slot_date >= $2")
        .bind(&step_ids)
        .bind(actual_from)
        .execute(&mut *tx)
        .await?;

    for placement in &placements {
        sqlx::query(
            "INSERT INTO schedule_slots (step_id, slot_date, start_minute, end_minute) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(placement.step_id)
        .bind(placement.date)
        .bind(placement.start_minute)
        .bind(placement.end_minute)
        .execute(&mut *tx)
        .await?;
    }

    let capacity = capacity as i64;
    let overflow: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT ss.slot_date FROM schedule_slots ss \
         JOIN steps s ON s.id = ss.step_id \
         JOIN books b ON b.id = s.book_id \
         WHERE b.goal_id = $1 AND ss.slot_date >= $2 \
         GROUP BY ss.slot_date \
         HAVING SUM(ss.end_minute - ss.start_minute) > $3 \
         LIMIT 1",
    )
    .bind(goal.id)
    .bind(actual_from)
    .bind(capacity)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(date) = overflow {
        tx.rollback().await?;
        tracing::error!(%date, "scheduler produced an overflowing day");
        return Err(AppError::Internal(
            "could not re-fit the schedule without overflowing a day".into(),
        ));
    }

    tx.commit().await?;
    Ok(())
}

async fn pending_steps_without_slots(
    state: &AppState,
    goal_id: Uuid,
) -> Result<Vec<PendingStepRow>, AppError> {
    let rows = sqlx::query_as::<_, PendingStepRow>(
        "SELECT s.id AS step_id, s.estimated_duration_mins AS duration_mins \
         FROM steps s \
         JOIN books b ON b.id = s.book_id \
         WHERE b.goal_id = $1 \
           AND NOT EXISTS (SELECT 1 FROM schedule_slots ss WHERE ss.step_id = s.id) \
           AND (EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id AND a.completed = false) \
                OR NOT EXISTS (SELECT 1 FROM activities a WHERE a.step_id = s.id)) \
         ORDER BY b.order_index, s.order_index",
    )
    .bind(goal_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(rows)
}

async fn busy_minutes(
    state: &AppState,
    goal_id: Uuid,
    from: NaiveDate,
    excluded_steps: &[Uuid],
) -> Result<HashMap<NaiveDate, i32>, AppError> {
    let rows = sqlx::query_as::<_, BusyRow>(
        "SELECT ss.slot_date, SUM(ss.end_minute - ss.start_minute) AS minutes \
         FROM schedule_slots ss \
         JOIN steps s ON s.id = ss.step_id \
         JOIN books b ON b.id = s.book_id \
         WHERE b.goal_id = $1 AND ss.slot_date >= $2 \
           AND NOT (ss.step_id = ANY($3)) \
         GROUP BY ss.slot_date",
    )
    .bind(goal_id)
    .bind(from)
    .bind(excluded_steps)
    .fetch_all(&state.pool)
    .await?;

    let mut busy = HashMap::new();
    for row in rows {
        busy.insert(row.slot_date, row.minutes.clamp(0, i32::MAX as i64) as i32);
    }
    Ok(busy)
}

async fn insert_placements(
    state: &AppState,
    placements: &[scheduler::SlotPlacement],
) -> Result<(), AppError> {
    if placements.is_empty() {
        return Ok(());
    }

    let mut tx = state.pool.begin().await?;
    for placement in placements {
        sqlx::query(
            "INSERT INTO schedule_slots (step_id, slot_date, start_minute, end_minute) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (step_id, slot_date) DO UPDATE \
             SET start_minute = EXCLUDED.start_minute, end_minute = EXCLUDED.end_minute, \
                 updated_at = now()",
        )
        .bind(placement.step_id)
        .bind(placement.date)
        .bind(placement.start_minute)
        .bind(placement.end_minute)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[derive(Debug, FromRow)]
struct ScheduledStepRow {
    step_id: Uuid,
    duration_mins: i32,
    slot_date: NaiveDate,
    start_minute: i32,
}
