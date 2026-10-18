use std::sync::Arc;

use async_trait::async_trait;

use crate::config::Config;

pub mod mock;
pub mod plan;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DecomposeRequest {
    pub title: String,
    pub why_statement: Option<String>,
    pub horizon_days: i64,
    pub preferred_language: String,
}

#[derive(Debug, Clone)]
pub struct BookDraft {
    pub title: String,
    pub description: String,
    pub weight: f32,
}

#[async_trait]
pub trait GoalDecomposer: Send + Sync {
    async fn decompose(&self, req: &DecomposeRequest) -> anyhow::Result<Vec<BookDraft>>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PlanRequest {
    pub book_title: String,
    pub planned_duration_mins: i32,
    pub preferred_language: String,
}

#[derive(Debug, Clone)]
pub struct ActivityDraft {
    pub activity_type: String,
    pub resource_url: Option<String>,
    pub embed_id: Option<String>,
    pub source_connector: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct StepDraft {
    pub title: String,
    pub activity_kind: String,
    pub estimated_duration_mins: i32,
    pub references_index: Option<usize>,
    pub activity: ActivityDraft,
}

#[async_trait]
pub trait BookPlanner: Send + Sync {
    async fn plan(&self, req: &PlanRequest) -> anyhow::Result<Vec<StepDraft>>;
}

pub fn build(cfg: &Config) -> Arc<dyn GoalDecomposer> {
    if cfg.nim_api_key.as_deref().unwrap_or("").is_empty() {
        tracing::info!("NIM_API_KEY not set; using mock goal decomposer");
    }
    Arc::new(mock::MockDecomposer::new())
}

pub fn build_planner(cfg: &Config) -> Arc<dyn BookPlanner> {
    if cfg.nim_api_key.as_deref().unwrap_or("").is_empty() {
        tracing::info!("NIM_API_KEY not set; using mock book planner");
    }
    Arc::new(plan::MockBookPlanner::new())
}
