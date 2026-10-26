use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, FromRow};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Availability {
    pub days: Vec<String>,
    pub minutes_per_day: u32,
}

impl Default for Availability {
    fn default() -> Self {
        Self {
            days: vec!["mon".into(), "wed".into(), "fri".into()],
            minutes_per_day: 60,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Goal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub why_statement: Option<String>,
    pub outcome: String,
    pub success_criteria: Json<Vec<String>>,
    pub deadline: NaiveDate,
    pub target_deadline: NaiveDate,
    pub availability: Json<Availability>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GoalResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub why_statement: Option<String>,
    pub outcome: String,
    pub success_criteria: Vec<String>,
    pub deadline: NaiveDate,
    pub target_deadline: NaiveDate,
    pub availability: Availability,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub books: Vec<super::book::BookResponse>,
}
