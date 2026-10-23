use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;

use crate::config::Config;
use quota::QuotaTracker;
use types::{
    ConnectorKind, ConnectorQuery, ContentResolver, ResolveRequest, ResolvedResource,
    ResourceCandidate, SourceCategory,
};

pub mod classifier;
pub mod connectors;
pub mod fallback;
pub mod quota;
pub mod types;

const PER_CONNECTOR_LIMIT: usize = 5;
const MAX_CANDIDATES: usize = 12;

pub struct HttpContentResolver {
    client: reqwest::Client,
    youtube_api_key: Option<String>,
    quota: QuotaTracker,
}

impl HttpContentResolver {
    pub fn new(cfg: &Config, pool: PgPool) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.connector_timeout_secs))
            .user_agent(concat!(
                "cronogramando-backend/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/anomalyco/cronogramando)"
            ))
            .build()?;

        Ok(Self {
            client,
            youtube_api_key: cfg.youtube_api_key.clone(),
            quota: QuotaTracker::new(pool, cfg.youtube_daily_units),
        })
    }

    async fn connector_candidates(
        &self,
        kind: ConnectorKind,
        query: &ConnectorQuery,
        category: SourceCategory,
    ) -> Result<Vec<ResourceCandidate>> {
        if kind.uses_paid_quota() {
            if self.youtube_api_key.as_deref().unwrap_or("").is_empty() {
                return Ok(Vec::new());
            }
            if !self.quota.consume_youtube_search().await {
                tracing::info!("YouTube quota exhausted or unavailable; falling back");
                return Ok(Vec::new());
            }
        }

        connectors::search(
            &self.client,
            kind,
            query,
            category,
            self.youtube_api_key.as_deref(),
        )
        .await
    }
}

#[async_trait]
impl ContentResolver for HttpContentResolver {
    async fn resolve(&self, req: &ResolveRequest) -> ResolvedResource {
        let category = classifier::classify(&req.query)
            .into_iter()
            .next()
            .unwrap_or(SourceCategory::Manual);
        let language = connectors::language_tag(&req.language);
        let query = ConnectorQuery {
            query: req.query.trim().to_string(),
            language,
            limit: PER_CONNECTOR_LIMIT,
        };

        let mut seen = HashSet::new();
        let mut candidates = Vec::new();

        for connector in ordered_connectors(category) {
            match self.connector_candidates(connector, &query, category).await {
                Ok(results) => {
                    for candidate in results {
                        if candidates.len() >= MAX_CANDIDATES {
                            break;
                        }
                        let key = candidate.url.clone().unwrap_or_else(|| {
                            format!("{}:{}", candidate.connector.as_str(), candidate.title)
                        });
                        if seen.insert(key) {
                            candidates.push(candidate);
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        connector = connector.as_str(),
                        %error,
                        "content connector failed"
                    );
                }
            }
        }

        if !candidates.is_empty() {
            return ResolvedResource {
                category,
                candidates,
                used_fallback: false,
            };
        }

        let fallback = fallback::fallback_candidates(category, &query.language);
        if fallback.is_empty() {
            tracing::info!(
                category = category.as_str(),
                query = query.query,
                "no resource found; using a generic task"
            );
            return ResolvedResource::generic_task(
                category,
                &format!("Study task: {}", req.query.trim()),
                &req.language,
            );
        }

        ResolvedResource {
            category,
            candidates: fallback,
            used_fallback: true,
        }
    }
}

pub fn build_resolver(cfg: &Config, pool: PgPool) -> Result<Arc<dyn ContentResolver>> {
    let resolver = HttpContentResolver::new(cfg, pool)?;
    if cfg.youtube_api_key.as_deref().unwrap_or("").is_empty() {
        tracing::info!("YOUTUBE_API_KEY not set; YouTube connector disabled");
    }
    Ok(Arc::new(resolver))
}

fn ordered_connectors(category: SourceCategory) -> Vec<ConnectorKind> {
    let mut connectors = category.connectors().to_vec();
    for fallback in [ConnectorKind::Wikipedia, ConnectorKind::Youtube] {
        if !connectors.contains(&fallback) {
            connectors.push(fallback);
        }
    }
    connectors
}

pub fn step_query(book_title: &str, step_title: &str) -> String {
    const PREFIXES: [&str; 4] = [
        "quick win:",
        "study:",
        "practice:",
        "spaced review of earlier material in",
    ];

    let step = step_title.trim();
    let lower = step.to_lowercase();
    for prefix in PREFIXES {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let rest = step[step.len() - rest.len()..].trim();
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }

    if step.is_empty() {
        book_title.to_string()
    } else {
        step.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_generic_planner_prefixes() {
        assert_eq!(
            step_query("Calculus II", "Quick win: Calculus II"),
            "Calculus II"
        );
        assert_eq!(
            step_query("Calculus II", "Study: Calculus II"),
            "Calculus II"
        );
        assert_eq!(
            step_query(
                "Calculus II",
                "Spaced review of earlier material in Calculus II"
            ),
            "Calculus II"
        );
    }

    #[test]
    fn keeps_specific_step_titles() {
        assert_eq!(
            step_query("Calculus II", "Integrals by substitution"),
            "Integrals by substitution"
        );
    }

    #[test]
    fn falls_back_to_book_title() {
        assert_eq!(step_query("Calculus II", "  "), "Calculus II");
    }

    #[test]
    fn every_category_gets_wikipedia_and_youtube_fallback() {
        for category in SourceCategory::ALL {
            let ordered = ordered_connectors(category);
            assert!(ordered.contains(&ConnectorKind::Wikipedia));
            assert!(ordered.contains(&ConnectorKind::Youtube));
        }
    }
}
