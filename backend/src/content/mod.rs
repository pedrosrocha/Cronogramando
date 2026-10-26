use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use sqlx::PgPool;

use crate::config::{Config, ContentConfig, SearxngConfig};
use brief::ContentBrief;
use curator::{Curator, JudgeOutcome};
use quota::QuotaTracker;
use tutor::Tutor;
use types::{
    ConnectorKind, ConnectorQuery, ContentResolver, LessonDraft, ResolveRequest, ResolvedResource,
    ResourceCandidate, SourceCategory,
};
use verify::{Verification, Verifier};

pub mod brief;
pub mod classifier;
pub mod connectors;
pub mod curator;
pub mod fallback;
pub mod query;
pub mod quota;
pub mod rank;
pub mod tutor;
pub mod types;
pub mod verify;

const PER_CONNECTOR_LIMIT: usize = 5;
const MAX_CANDIDATES: usize = 40;

pub struct HttpContentResolver {
    client: reqwest::Client,
    youtube_api_key: Option<String>,
    quota: QuotaTracker,
    searxng: SearxngConfig,
    content: ContentConfig,
    verifier: Verifier,
    curator: Option<Arc<Curator>>,
    tutor: Option<Arc<Tutor>>,
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

        let content = cfg.content.clone();
        let verifier = Verifier::new(
            client.clone(),
            cfg.pdf_service_url.clone(),
            cfg.pdf_service_timeout_secs,
            content.clone(),
        );
        let curator = if content.judge_enabled() && cfg.llm_curator.enabled() {
            match Curator::new(&cfg.llm_curator, content.clone()) {
                Ok(curator) => Some(Arc::new(curator)),
                Err(error) => {
                    tracing::warn!(%error, "content curator unavailable; using deterministic gate");
                    None
                }
            }
        } else {
            None
        };
        let tutor = if cfg.llm_tutor.enabled() {
            match Tutor::new(&cfg.llm_tutor) {
                Ok(tutor) => Some(Arc::new(tutor)),
                Err(error) => {
                    tracing::warn!(%error, "content tutor unavailable; using curated fallback");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            client,
            youtube_api_key: cfg.youtube_api_key.clone(),
            quota: QuotaTracker::new(pool, cfg.youtube_daily_units),
            searxng: cfg.searxng.clone(),
            content,
            verifier,
            curator,
            tutor,
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

    async fn retrieve(
        &self,
        brief: &ContentBrief,
        category: SourceCategory,
        language: &str,
    ) -> (Vec<ResourceCandidate>, Map<String, Value>) {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        let mut queries = Map::new();

        for connector in ordered_connectors(category) {
            let query_text = query::build_for(connector, brief);
            queries.insert(
                connector.as_str().to_string(),
                Value::String(query_text.clone()),
            );
            let connector_query = ConnectorQuery {
                query: query_text,
                language: language.to_string(),
                limit: PER_CONNECTOR_LIMIT,
            };

            if connector == ConnectorKind::Searxng {
                if !self.searxng.enabled() {
                    continue;
                }
                let (general, videos) = tokio::join!(
                    connectors::searxng::search(
                        &self.client,
                        &connector_query,
                        category,
                        &self.searxng,
                        "general"
                    ),
                    connectors::searxng::search(
                        &self.client,
                        &connector_query,
                        category,
                        &self.searxng,
                        "videos"
                    ),
                );
                for result in [general, videos] {
                    match result {
                        Ok(results) => push_unique(&mut candidates, &mut seen, results),
                        Err(error) => {
                            tracing::warn!(%error, "SearXNG search failed");
                        }
                    }
                }
                continue;
            }

            match self
                .connector_candidates(connector, &connector_query, category)
                .await
            {
                Ok(results) => push_unique(&mut candidates, &mut seen, results),
                Err(error) => {
                    tracing::warn!(
                        connector = connector.as_str(),
                        %error,
                        "content connector failed"
                    );
                }
            }
        }

        (candidates, queries)
    }

    async fn fallback(
        &self,
        req: &ResolveRequest,
        category: SourceCategory,
        brief: &ContentBrief,
        verifications: &[Verification],
        queries: &Map<String, Value>,
        judge: Option<JudgeOutcome>,
        reason: &str,
    ) -> ResolvedResource {
        let mut trace = build_trace(
            brief,
            category,
            queries,
            verifications,
            &judge,
            None,
            Some(reason),
        );

        if self.content.fallback_lesson == "auto" {
            if let Some(tutor) = &self.tutor {
                let lesson = tokio::time::timeout(
                    Duration::from_secs(self.content.tutor_timeout_secs),
                    tutor.lesson(brief),
                )
                .await
                .ok()
                .flatten();
                if let Some(lesson) = lesson {
                    let selection_reason =
                        format!("generated micro-lesson '{}': {reason}", lesson.title);
                    trace["lesson"] = json!(true);
                    return ResolvedResource {
                        category,
                        candidates: Vec::new(),
                        lesson: Some(lesson),
                        used_fallback: true,
                        quality_status: "fallback".to_string(),
                        selection_reason,
                        trace,
                    };
                }
            }
        }

        let fallback = fallback::fallback_candidates(category, &brief.language);
        if fallback.is_empty() {
            return ResolvedResource::generic_task(
                category,
                &format!("Study task: {}", req.step_outcome.trim()),
                &req.language,
            );
        }

        ResolvedResource {
            category,
            candidates: fallback,
            lesson: None,
            used_fallback: true,
            quality_status: "fallback".to_string(),
            selection_reason: format!("curated fallback: {reason}"),
            trace,
        }
    }
}

#[async_trait]
impl ContentResolver for HttpContentResolver {
    async fn resolve(&self, req: &ResolveRequest) -> ResolvedResource {
        let brief = brief::build(req);
        let combined = format!(
            "{} {} {} {}",
            req.goal_outcome, req.module_title, req.module_objective, req.step_title
        );
        let category = classifier::classify(&combined)
            .into_iter()
            .next()
            .unwrap_or(SourceCategory::Manual);
        let language = connectors::language_tag(&req.language);

        let (candidates, queries) = self.retrieve(&brief, category, &language).await;
        let query_text = brief.search_text();
        let ranked = rank::rank(candidates, &query_text, &language);
        let top: Vec<ResourceCandidate> = ranked
            .into_iter()
            .take(self.content.verify_top_k.max(1))
            .collect();

        let mut verifications = Vec::new();
        for candidate in top {
            verifications.push(self.verifier.verify(candidate, &brief).await);
        }

        let mut verified: Vec<ResourceCandidate> = verifications
            .iter()
            .filter(|verification| verification.accepted)
            .map(|verification| verification.candidate.clone())
            .collect();
        verified = rank::rank(verified, &query_text, &language);

        let mut judge = None;
        if let Some(curator) = &self.curator {
            let judged = tokio::time::timeout(
                Duration::from_secs(self.content.judge_timeout_secs),
                curator.judge(&brief, &verified),
            )
            .await
            .ok()
            .flatten();
            if let Some(outcome) = judged {
                let chosen_url = outcome.chosen_url.clone();
                judge = Some(outcome);
                match chosen_url {
                    Some(url) => {
                        if let Some(position) = verified
                            .iter()
                            .position(|candidate| candidate.url.as_deref() == Some(url.as_str()))
                        {
                            let chosen = verified.remove(position);
                            verified.insert(0, chosen);
                        }
                    }
                    None => {
                        return self
                            .fallback(
                                req,
                                category,
                                &brief,
                                &verifications,
                                &queries,
                                judge,
                                "the relevance judge rejected every candidate",
                            )
                            .await;
                    }
                }
            }
        }

        if verified.is_empty() {
            return self
                .fallback(
                    req,
                    category,
                    &brief,
                    &verifications,
                    &queries,
                    judge,
                    "no candidate passed verification",
                )
                .await;
        }

        let chosen = verified[0].clone();
        let judge_reason = judge.as_ref().and_then(|outcome| {
            outcome
                .decisions
                .iter()
                .find(|decision| {
                    chosen.url.is_some() && Some(decision.url.as_str()) == chosen.url.as_deref()
                })
                .map(|decision| decision.reason.clone())
        });
        let selection_reason = format!(
            "{} | {} | score {} | {}",
            chosen.connector.as_str(),
            domain_of(chosen.url.as_deref()),
            chosen.score,
            judge_reason.unwrap_or_else(|| "passed deterministic verification".to_string())
        );
        let trace = build_trace(
            &brief,
            category,
            &queries,
            &verifications,
            &judge,
            Some(&chosen),
            None,
        );

        ResolvedResource {
            category,
            candidates: verified,
            lesson: None,
            used_fallback: false,
            quality_status: "ok".to_string(),
            selection_reason,
            trace,
        }
    }

    async fn lesson(&self, req: &ResolveRequest) -> Option<LessonDraft> {
        let brief = brief::build(req);
        self.tutor.as_ref()?.lesson(&brief).await
    }
}

pub fn build_resolver(cfg: &Config, pool: PgPool) -> Result<Arc<dyn ContentResolver>> {
    let resolver = HttpContentResolver::new(cfg, pool)?;
    if cfg.youtube_api_key.as_deref().unwrap_or("").is_empty() {
        tracing::info!("YOUTUBE_API_KEY not set; YouTube connector disabled");
    }
    if !cfg.searxng.enabled() {
        tracing::info!("SearXNG not configured; general web search disabled");
    }
    if !cfg.llm_curator.enabled() {
        tracing::info!("LLM curator not configured; deterministic relevance only");
    }
    if !cfg.llm_tutor.enabled() {
        tracing::info!("LLM tutor not configured; fallback lessons disabled");
    }
    Ok(Arc::new(resolver))
}

fn push_unique(
    candidates: &mut Vec<ResourceCandidate>,
    seen: &mut HashSet<String>,
    results: Vec<ResourceCandidate>,
) {
    for candidate in results {
        if candidates.len() >= MAX_CANDIDATES {
            break;
        }
        let key = candidate
            .url
            .clone()
            .unwrap_or_else(|| format!("{}:{}", candidate.connector.as_str(), candidate.title));
        if seen.insert(key) {
            candidates.push(candidate);
        }
    }
}

fn build_trace(
    brief: &ContentBrief,
    category: SourceCategory,
    queries: &Map<String, Value>,
    verifications: &[Verification],
    judge: &Option<JudgeOutcome>,
    chosen: Option<&ResourceCandidate>,
    note: Option<&str>,
) -> Value {
    let candidates: Vec<Value> = verifications
        .iter()
        .map(|verification| {
            json!({
                "title": &verification.candidate.title,
                "url": &verification.candidate.url,
                "connector": verification.candidate.connector.as_str(),
                "score": verification.candidate.score,
                "accepted": verification.accepted,
                "reason": &verification.reason,
                "excerpt": verification
                    .candidate
                    .excerpt
                    .as_deref()
                    .or(verification.candidate.description.as_deref())
                    .unwrap_or("")
                    .chars()
                    .take(240)
                    .collect::<String>(),
            })
        })
        .collect();

    json!({
        "brief": {
            "goal_outcome": &brief.goal_outcome,
            "module_objective": &brief.module_objective,
            "step_outcome": &brief.step_outcome,
            "success_criteria": &brief.success_criteria,
            "content_intent": &brief.content_intent,
            "preferred_type": &brief.activity_type,
            "must_terms": &brief.must_terms,
            "exclude_terms": &brief.exclude_terms,
            "max_minutes": brief.max_minutes,
        },
        "category": category.as_str(),
        "queries": queries,
        "candidates": candidates,
        "judge": judge,
        "chosen": chosen.and_then(|candidate| candidate.url.clone()),
        "note": note,
    })
}

fn domain_of(url: Option<&str>) -> String {
    url.and_then(|url| url::Url::parse(url).ok())
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_gets_wikipedia_and_youtube_fallback() {
        for category in SourceCategory::ALL {
            let ordered = ordered_connectors(category);
            assert!(ordered.contains(&ConnectorKind::Wikipedia));
            assert!(ordered.contains(&ConnectorKind::Youtube));
        }
    }

    #[test]
    fn every_category_includes_web_search() {
        for category in SourceCategory::ALL {
            assert!(ordered_connectors(category).contains(&ConnectorKind::Searxng));
        }
    }
}
