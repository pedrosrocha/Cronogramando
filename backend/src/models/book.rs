use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{types::Json, FromRow};
use utoipa::ToSchema;
use uuid::Uuid;

pub const BOOK_COLUMNS: &str = "id, goal_id, order_index, title, description, objective, \
     scope_in, scope_out, prerequisites, assessment, status, \
     planned_duration_mins, min_duration_mins, actual_duration_mins, completed_at, \
     reconciliation_status, reconciliation_warning, created_at, updated_at";

#[derive(Debug, Clone, FromRow)]
pub struct Book {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub order_index: i32,
    pub title: String,
    pub description: String,
    pub objective: String,
    pub scope_in: Json<Vec<String>>,
    pub scope_out: Json<Vec<String>>,
    pub prerequisites: Json<Vec<String>>,
    pub assessment: String,
    pub status: String,
    pub planned_duration_mins: i32,
    pub min_duration_mins: i32,
    pub actual_duration_mins: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub reconciliation_status: Option<String>,
    pub reconciliation_warning: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BookResponse {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub order_index: i32,
    pub title: String,
    pub description: String,
    pub objective: String,
    pub scope_in: Vec<String>,
    pub scope_out: Vec<String>,
    pub prerequisites: Vec<String>,
    pub assessment: String,
    pub status: String,
    pub planned_duration_mins: i32,
    pub min_duration_mins: i32,
    pub actual_duration_mins: Option<i32>,
    pub completed_at: Option<DateTime<Utc>>,
    pub reconciliation_status: Option<String>,
    pub reconciliation_warning: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Book> for BookResponse {
    fn from(b: Book) -> Self {
        Self {
            id: b.id,
            goal_id: b.goal_id,
            order_index: b.order_index,
            title: b.title,
            description: b.description,
            objective: b.objective,
            scope_in: b.scope_in.0,
            scope_out: b.scope_out.0,
            prerequisites: b.prerequisites.0,
            assessment: b.assessment,
            status: b.status,
            planned_duration_mins: b.planned_duration_mins,
            min_duration_mins: b.min_duration_mins,
            actual_duration_mins: b.actual_duration_mins,
            completed_at: b.completed_at,
            reconciliation_status: b.reconciliation_status,
            reconciliation_warning: b.reconciliation_warning,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }
    }
}
