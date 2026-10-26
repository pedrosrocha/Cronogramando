use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceCategory {
    Manual,
    Stem,
    Business,
    Humanities,
    Arts,
    Health,
}

impl SourceCategory {
    #[cfg(test)]
    pub const ALL: [SourceCategory; 6] = [
        SourceCategory::Manual,
        SourceCategory::Stem,
        SourceCategory::Business,
        SourceCategory::Humanities,
        SourceCategory::Arts,
        SourceCategory::Health,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            SourceCategory::Manual => "manual",
            SourceCategory::Stem => "stem",
            SourceCategory::Business => "business",
            SourceCategory::Humanities => "humanities",
            SourceCategory::Arts => "arts",
            SourceCategory::Health => "health",
        }
    }

    pub fn connectors(self) -> &'static [ConnectorKind] {
        match self {
            SourceCategory::Manual => &[
                ConnectorKind::Searxng,
                ConnectorKind::Youtube,
                ConnectorKind::InternetArchive,
                ConnectorKind::Wikipedia,
            ],
            SourceCategory::Stem => &[
                ConnectorKind::Arxiv,
                ConnectorKind::OpenAlex,
                ConnectorKind::Searxng,
                ConnectorKind::Wikipedia,
                ConnectorKind::Youtube,
            ],
            SourceCategory::Business => &[
                ConnectorKind::Searxng,
                ConnectorKind::Youtube,
                ConnectorKind::OpenAlex,
                ConnectorKind::Doaj,
                ConnectorKind::Wikipedia,
            ],
            SourceCategory::Humanities => &[
                ConnectorKind::Searxng,
                ConnectorKind::Wikipedia,
                ConnectorKind::Youtube,
                ConnectorKind::Doaj,
                ConnectorKind::OpenAlex,
                ConnectorKind::Gutenberg,
            ],
            SourceCategory::Arts => &[
                ConnectorKind::Searxng,
                ConnectorKind::Youtube,
                ConnectorKind::Wikipedia,
                ConnectorKind::InternetArchive,
            ],
            SourceCategory::Health => &[
                ConnectorKind::Searxng,
                ConnectorKind::Youtube,
                ConnectorKind::OpenAlex,
                ConnectorKind::Wikipedia,
            ],
        }
    }
}

impl std::fmt::Display for SourceCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SourceCategory {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_lowercase().as_str() {
            "manual" => Ok(SourceCategory::Manual),
            "stem" => Ok(SourceCategory::Stem),
            "business" => Ok(SourceCategory::Business),
            "humanities" => Ok(SourceCategory::Humanities),
            "arts" => Ok(SourceCategory::Arts),
            "health" => Ok(SourceCategory::Health),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectorKind {
    Searxng,
    Youtube,
    Wikipedia,
    InternetArchive,
    Arxiv,
    OpenAlex,
    Doaj,
    Gutenberg,
    Curated,
    GenericTask,
}

impl ConnectorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectorKind::Searxng => "searxng",
            ConnectorKind::Youtube => "youtube",
            ConnectorKind::Wikipedia => "wikipedia",
            ConnectorKind::InternetArchive => "internet_archive",
            ConnectorKind::Arxiv => "arxiv",
            ConnectorKind::OpenAlex => "openalex",
            ConnectorKind::Doaj => "doaj",
            ConnectorKind::Gutenberg => "gutenberg",
            ConnectorKind::Curated => "curated",
            ConnectorKind::GenericTask => "generic_task",
        }
    }

    pub fn uses_paid_quota(self) -> bool {
        matches!(self, ConnectorKind::Youtube)
    }
}

#[derive(Debug, Clone)]
pub struct ConnectorQuery {
    pub query: String,
    pub language: String,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct ResourceCandidate {
    pub title: String,
    pub url: Option<String>,
    pub embed_id: Option<String>,
    pub connector: ConnectorKind,
    pub language: String,
    pub activity_type: String,
    pub category: SourceCategory,
    /// Snippet/description supplied by the connector.
    pub description: Option<String>,
    /// Bounded readable excerpt used by the relevance judge.
    pub excerpt: Option<String>,
    pub duration_secs: Option<u32>,
    pub page_count: Option<u32>,
    pub score: i32,
}

#[derive(Debug, Clone)]
pub struct ResolveRequest {
    pub goal_outcome: String,
    pub module_title: String,
    pub module_objective: String,
    pub module_scope_in: Vec<String>,
    pub module_scope_out: Vec<String>,
    pub step_title: String,
    pub step_outcome: String,
    pub success_criteria: String,
    pub content_intent: String,
    pub activity_type: String,
    pub estimated_duration_mins: i32,
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct LessonDraft {
    pub title: String,
    pub content_body: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedResource {
    pub category: SourceCategory,
    pub candidates: Vec<ResourceCandidate>,
    pub lesson: Option<LessonDraft>,
    pub used_fallback: bool,
    pub quality_status: String,
    pub selection_reason: String,
    pub trace: serde_json::Value,
}

impl ResolvedResource {
    pub fn generic_task(category: SourceCategory, title: &str, language: &str) -> Self {
        Self {
            category,
            used_fallback: true,
            quality_status: "fallback".to_string(),
            selection_reason: "no verified resource".to_string(),
            trace: serde_json::json!({ "reason": "generic_task" }),
            lesson: None,
            candidates: vec![ResourceCandidate {
                title: title.to_string(),
                url: None,
                embed_id: None,
                connector: ConnectorKind::GenericTask,
                language: language.to_string(),
                activity_type: "task".to_string(),
                category,
                description: None,
                excerpt: None,
                duration_secs: None,
                page_count: None,
                score: 0,
            }],
        }
    }
}

#[async_trait]
pub trait ContentResolver: Send + Sync {
    async fn resolve(&self, req: &ResolveRequest) -> ResolvedResource;

    /// Generate a short lesson for a step that has no suitable resource.
    async fn lesson(&self, req: &ResolveRequest) -> Option<LessonDraft>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_round_trips() {
        for category in SourceCategory::ALL {
            assert_eq!(category.as_str().parse::<SourceCategory>(), Ok(category));
        }
    }

    #[test]
    fn every_category_has_connectors() {
        for category in SourceCategory::ALL {
            assert!(!category.connectors().is_empty());
        }
    }
}
