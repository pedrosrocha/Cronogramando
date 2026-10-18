use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Step {
    pub id: Uuid,
    pub book_id: Uuid,
    pub order_index: i32,
    pub title: String,
    pub activity_kind: String,
    pub estimated_duration_mins: i32,
    pub references_step_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StepResponse {
    pub id: Uuid,
    pub book_id: Uuid,
    pub order_index: i32,
    pub title: String,
    pub activity_kind: String,
    pub estimated_duration_mins: i32,
    pub references_step_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub activities: Vec<super::activity::ActivityResponse>,
}
