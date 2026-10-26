use anyhow::Result;

use super::brief::ContentBrief;
use super::types::LessonDraft;
use crate::config::LlmConfig;
use crate::llm::client::{ChatMessage, LlmClient};

pub struct Tutor {
    client: LlmClient,
}

impl Tutor {
    pub fn new(cfg: &LlmConfig) -> Result<Self> {
        Ok(Self {
            client: LlmClient::new(cfg)?,
        })
    }

    pub async fn lesson(&self, brief: &ContentBrief) -> Option<LessonDraft> {
        let pt = brief.language.to_lowercase().starts_with("pt");
        let language = if pt {
            "português do Brasil"
        } else {
            "English"
        };
        let system = format!(
            "You are a concise tutor. Write a micro-lesson of 150 to 250 words in \
             {language}. Structure it as: one sentence stating the objective; 3 to 5 \
             bullet key points; one short worked example; one quick exercise. Do not \
             invent links, citations or facts. Return plain markdown, without code \
             fences or commentary outside the lesson."
        );
        let user = format!(
            "Goal outcome: {goal}\nModule: {module}\nStep: {step}\n\
             Learning outcome: {outcome}\nSuccess criteria: {criteria}\n\
             Scope: {scope}",
            goal = brief.goal_outcome,
            module = brief.module_objective,
            step = brief.step_outcome,
            outcome = brief.step_outcome,
            criteria = brief.success_criteria,
            scope = brief.topic_terms.join("; "),
        );

        let raw = self
            .client
            .chat(
                &[ChatMessage::system(system), ChatMessage::user(user)],
                0.4,
                Some(700),
                None,
                None,
            )
            .await
            .ok()?;

        let body = raw.trim();
        if body.is_empty() {
            return None;
        }
        let title = if pt {
            format!("Lição: {}", truncate(brief.step_outcome.trim(), 70))
        } else {
            format!("Lesson: {}", truncate(brief.step_outcome.trim(), 70))
        };
        Some(LessonDraft {
            title,
            content_body: body.to_string(),
        })
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect::<String>() + "…"
}
