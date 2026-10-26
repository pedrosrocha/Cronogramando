use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::brief::ContentBrief;
use super::types::ResourceCandidate;
use crate::config::{ContentConfig, LlmConfig};
use crate::llm::client::{parse_json, ChatMessage, LlmClient};

pub struct Curator {
    client: LlmClient,
    cfg: ContentConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct JudgeDecision {
    pub url: String,
    pub relevant: bool,
    pub score: i32,
    pub evidence: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JudgeOutcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chosen_url: Option<String>,
    pub decisions: Vec<JudgeDecision>,
    pub mode: String,
}

#[derive(Debug, Deserialize)]
struct JudgePayload {
    #[serde(default)]
    decisions: Vec<JudgeDecisionRaw>,
}

#[derive(Debug, Deserialize)]
struct JudgeDecisionRaw {
    #[serde(default)]
    url: String,
    #[serde(default)]
    relevant: bool,
    #[serde(default)]
    score: i32,
    #[serde(default)]
    evidence: String,
    #[serde(default)]
    reason: String,
}

impl Curator {
    pub fn new(cfg: &LlmConfig, content: ContentConfig) -> Result<Self> {
        Ok(Self {
            client: LlmClient::new(cfg)?,
            cfg: content,
        })
    }

    pub async fn judge(
        &self,
        brief: &ContentBrief,
        candidates: &[ResourceCandidate],
    ) -> Option<JudgeOutcome> {
        if candidates.is_empty() || self.cfg.curator_mode == "off" {
            return None;
        }
        if !self.is_ambiguous(candidates) {
            return None;
        }

        let top: Vec<&ResourceCandidate> = candidates.iter().take(5).collect();
        let cards: Vec<serde_json::Value> = top
            .iter()
            .map(|candidate| {
                let details = match (candidate.duration_secs, candidate.page_count) {
                    (Some(seconds), _) => format!("{}:{:02}", seconds / 60, seconds % 60),
                    (_, Some(pages)) => format!("{pages} pages"),
                    _ => "unknown".to_string(),
                };
                json!({
                    "url": &candidate.url,
                    "title": &candidate.title,
                    "type": &candidate.activity_type,
                    "details": details,
                    "excerpt": candidate
                        .excerpt
                        .as_deref()
                        .or(candidate.description.as_deref())
                        .unwrap_or("")
                        .chars()
                        .take(900)
                        .collect::<String>(),
                })
            })
            .collect();

        let language = if brief.language.to_lowercase().starts_with("pt") {
            "Portuguese"
        } else {
            "English"
        };
        let system = format!(
            "You are a strict content relevance judge. Decide whether each candidate \
             genuinely teaches the step objective. You MUST quote exact text from the \
             candidate excerpt that supports your decision. If the excerpt is about a \
             different topic, or only shares keywords, mark relevant=false. Never invent \
             a quote. Write reasons in {language}.\n\
             Return only JSON: {{\"decisions\":[{{\"url\":\"string\",\
             \"relevant\":true|false,\"score\":0-10,\"evidence\":\"string\",\
             \"reason\":\"string\"}}]}}"
        );
        let user = format!(
            "Goal outcome: {goal}\nModule objective: {module}\nStep: {step}\n\
             Step outcome: {outcome}\nSuccess criteria: {criteria}\n\
             Scope in: {scope}\n\nCandidates:\n{candidates}\n\n\
             Evidence must be a verbatim substring of the candidate excerpt.",
            goal = brief.goal_outcome,
            module = brief.module_objective,
            step = brief.objective(),
            outcome = brief.step_outcome,
            criteria = brief.success_criteria,
            scope = brief.topic_terms.join("; "),
            candidates = serde_json::to_string(&cards).unwrap_or_else(|_| "[]".to_string()),
        );

        let schema = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "relevance_judgement",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "decisions": {
                            "type": "array",
                            "minItems": 1,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "url": { "type": "string" },
                                    "relevant": { "type": "boolean" },
                                    "score": { "type": "integer", "minimum": 0, "maximum": 10 },
                                    "evidence": { "type": "string" },
                                    "reason": { "type": "string" }
                                },
                                "required": ["url", "relevant", "score", "evidence", "reason"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["decisions"],
                    "additionalProperties": false
                }
            }
        });

        let raw = self
            .client
            .chat(
                &[ChatMessage::system(system), ChatMessage::user(user)],
                0.2,
                Some(1400),
                Some(&schema),
                Some(&json!({ "effort": "low" })),
            )
            .await
            .ok()?;
        let payload: JudgePayload = parse_json(&raw, "relevance judgement").ok()?;

        let mut decisions = Vec::new();
        let mut best: Option<(i32, String)> = None;
        for raw in payload.decisions {
            let Some(candidate) = top
                .iter()
                .find(|candidate| candidate.url.as_deref() == Some(raw.url.as_str()))
            else {
                continue;
            };
            let excerpt = candidate
                .excerpt
                .as_deref()
                .or(candidate.description.as_deref())
                .unwrap_or("");
            let evidence_ok = !raw.evidence.trim().is_empty()
                && normalize_text(excerpt).contains(&normalize_text(&raw.evidence));
            let accepted = raw.relevant && raw.score >= self.cfg.judge_threshold && evidence_ok;
            if accepted {
                if best
                    .as_ref()
                    .map(|(score, _)| raw.score > *score)
                    .unwrap_or(true)
                {
                    best = Some((raw.score, raw.url.clone()));
                }
            }
            decisions.push(JudgeDecision {
                url: raw.url,
                relevant: raw.relevant,
                score: raw.score,
                evidence: raw.evidence,
                reason: raw.reason,
                accepted: Some(accepted),
            });
        }

        Some(JudgeOutcome {
            chosen_url: best.map(|(_, url)| url),
            decisions,
            mode: self.cfg.curator_mode.clone(),
        })
    }

    fn is_ambiguous(&self, candidates: &[ResourceCandidate]) -> bool {
        if self.cfg.curator_mode == "always" {
            return true;
        }
        if candidates.len() <= 1 {
            return true;
        }
        let top = candidates[0].score.max(1);
        let second = candidates
            .get(1)
            .map(|candidate| candidate.score)
            .unwrap_or(0);
        if top < 60 {
            return true;
        }
        let gap = ((top - second) * 100) / top;
        gap < self.cfg.judge_margin
    }
}

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_whitespace_for_evidence_checks() {
        assert_eq!(normalize_text("A  redação\n do ENEM"), "a redação do enem");
    }
}
