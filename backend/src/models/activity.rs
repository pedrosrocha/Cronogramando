use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Activity {
    pub id: Uuid,
    pub step_id: Uuid,
    #[sqlx(rename = "type")]
    pub activity_type: String,
    pub resource_url: Option<String>,
    pub embed_id: Option<String>,
    pub source_connector: Option<String>,
    pub source_category: Option<String>,
    pub language: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ActivityResponse {
    pub id: Uuid,
    pub step_id: Uuid,
    #[serde(rename = "type")]
    pub activity_type: String,
    pub resource_url: Option<String>,
    pub embed_id: Option<String>,
    pub source_connector: Option<String>,
    pub source_category: Option<String>,
    pub language: String,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Activity> for ActivityResponse {
    fn from(a: Activity) -> Self {
        Self {
            id: a.id,
            step_id: a.step_id,
            activity_type: a.activity_type,
            resource_url: a.resource_url,
            embed_id: a.embed_id,
            source_connector: a.source_connector,
            source_category: a.source_category,
            language: a.language,
            completed: a.completed,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}
