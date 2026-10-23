use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    config::Config,
    content::types::ContentResolver,
    llm::{BookPlanner, GoalDecomposer},
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub cfg: Arc<Config>,
    pub llm: Arc<dyn GoalDecomposer>,
    pub planner: Arc<dyn BookPlanner>,
    pub resolver: Arc<dyn ContentResolver>,
}
