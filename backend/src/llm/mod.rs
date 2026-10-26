use std::sync::Arc;

use async_trait::async_trait;

use crate::config::Config;

pub mod client;
pub mod decomposer;
pub mod mock;
pub mod plan;
pub mod translator;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DecomposeRequest {
    pub title: String,
    pub why_statement: Option<String>,
    pub horizon_days: i64,
    pub capacity_mins: i32,
    pub preferred_language: String,
}

#[derive(Debug, Clone)]
pub struct Decomposition {
    pub goal_outcome: String,
    pub goal_success_criteria: Vec<String>,
    pub books: Vec<BookDraft>,
}

#[derive(Debug, Clone)]
pub struct BookDraft {
    pub title: String,
    pub description: String,
    pub objective: String,
    pub scope_in: Vec<String>,
    pub scope_out: Vec<String>,
    pub prerequisites: Vec<String>,
    pub assessment: String,
    pub weight: f32,
}

#[async_trait]
pub trait GoalDecomposer: Send + Sync {
    async fn decompose(&self, req: &DecomposeRequest) -> anyhow::Result<Decomposition>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PlanRequest {
    pub goal_title: String,
    pub goal_outcome: String,
    pub why_statement: Option<String>,
    pub book_title: String,
    pub book_description: String,
    pub book_objective: String,
    pub book_scope_in: Vec<String>,
    pub book_scope_out: Vec<String>,
    pub book_prerequisites: Vec<String>,
    pub book_assessment: String,
    pub previous_book_objective: Option<String>,
    pub next_book_objective: Option<String>,
    pub planned_duration_mins: i32,
    pub preferred_language: String,
}

#[derive(Debug, Clone)]
pub struct ActivityDraft {
    pub activity_type: String,
    pub resource_url: Option<String>,
    pub embed_id: Option<String>,
    pub source_connector: Option<String>,
    pub source_category: Option<String>,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct StepDraft {
    pub title: String,
    pub learning_outcome: String,
    pub success_criteria: String,
    pub content_intent: String,
    pub activity_kind: String,
    pub estimated_duration_mins: i32,
    pub references_index: Option<usize>,
    pub activity: ActivityDraft,
}

#[async_trait]
pub trait BookPlanner: Send + Sync {
    async fn plan(&self, req: &PlanRequest) -> anyhow::Result<Vec<StepDraft>>;
}

struct FallbackDecomposer {
    primary: Arc<dyn GoalDecomposer>,
    fallback: Arc<dyn GoalDecomposer>,
}

#[async_trait]
impl GoalDecomposer for FallbackDecomposer {
    async fn decompose(&self, req: &DecomposeRequest) -> anyhow::Result<Decomposition> {
        match self.primary.decompose(req).await {
            Ok(decomposition) if !decomposition.books.is_empty() => Ok(decomposition),
            Ok(_) => {
                tracing::warn!("LLM goal decomposition returned no books; using mock");
                self.fallback.decompose(req).await
            }
            Err(error) => {
                tracing::warn!(%error, "LLM goal decomposition failed; using mock");
                self.fallback.decompose(req).await
            }
        }
    }
}

struct FallbackPlanner {
    primary: Arc<dyn BookPlanner>,
    fallback: Arc<dyn BookPlanner>,
}

#[async_trait]
impl BookPlanner for FallbackPlanner {
    async fn plan(&self, req: &PlanRequest) -> anyhow::Result<Vec<StepDraft>> {
        match self.primary.plan(req).await {
            Ok(steps) if !steps.is_empty() => Ok(steps),
            Ok(_) => {
                tracing::warn!("LLM book planning returned no steps; using mock");
                self.fallback.plan(req).await
            }
            Err(error) => {
                tracing::warn!(%error, "LLM book planning failed; using mock");
                self.fallback.plan(req).await
            }
        }
    }
}

pub fn build(cfg: &Config) -> Arc<dyn GoalDecomposer> {
    if !cfg.llm_decomposer.enabled() {
        tracing::info!("LLM decomposer not configured; using mock goal decomposer");
        return Arc::new(mock::MockDecomposer::new());
    }

    tracing::info!(
        models = %cfg.llm_decomposer.models.join(", "),
        "using LLM goal decomposer"
    );
    match decomposer::LlmGoalDecomposer::new(cfg) {
        Ok(llm) => Arc::new(FallbackDecomposer {
            primary: Arc::new(llm),
            fallback: Arc::new(mock::MockDecomposer::new()),
        }),
        Err(error) => {
            tracing::warn!(%error, "LLM decomposer unavailable; using mock");
            Arc::new(mock::MockDecomposer::new())
        }
    }
}

pub fn build_planner(cfg: &Config) -> Arc<dyn BookPlanner> {
    if !cfg.llm_planner.enabled() {
        tracing::info!("LLM planner not configured; using mock book planner");
        return Arc::new(plan::MockBookPlanner::new());
    }

    tracing::info!(
        models = %cfg.llm_planner.models.join(", "),
        "using LLM book planner"
    );
    match plan::LlmBookPlanner::new(cfg) {
        Ok(llm) => Arc::new(FallbackPlanner {
            primary: Arc::new(llm),
            fallback: Arc::new(plan::MockBookPlanner::new()),
        }),
        Err(error) => {
            tracing::warn!(%error, "LLM planner unavailable; using mock");
            Arc::new(plan::MockBookPlanner::new())
        }
    }
}

pub fn build_translator(cfg: &Config) -> Arc<dyn translator::Translator> {
    if !cfg.llm_translator.enabled() {
        tracing::info!("LLM translator not configured; using identity translation mock");
        return Arc::new(translator::MockTranslator);
    }

    tracing::info!(
        models = %cfg.llm_translator.models.join(", "),
        "using LLM translator"
    );
    match translator::LlmTranslator::new(cfg) {
        Ok(llm) => Arc::new(llm),
        Err(error) => {
            tracing::warn!(%error, "LLM translator unavailable; using identity mock");
            Arc::new(translator::MockTranslator)
        }
    }
}
