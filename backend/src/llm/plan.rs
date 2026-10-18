use async_trait::async_trait;

use super::{ActivityDraft, BookPlanner, PlanRequest, StepDraft};

pub struct MockBookPlanner;

impl MockBookPlanner {
    pub fn new() -> Self {
        MockBookPlanner
    }
}

impl Default for MockBookPlanner {
    fn default() -> Self {
        Self::new()
    }
}

const RESOURCE_TYPES: [&str; 4] = ["video", "pdf", "article", "podcast"];

fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn placeholder_url(book_title: &str, index: usize) -> String {
    format!(
        "https://placeholder.cronogramando.local/{}-{}",
        slug(book_title),
        index
    )
}

#[async_trait]
impl BookPlanner for MockBookPlanner {
    async fn plan(&self, req: &PlanRequest) -> anyhow::Result<Vec<StepDraft>> {
        let total = req.planned_duration_mins.max(30);
        let count = ((total / 30).clamp(3, 12)) as usize;
        let title = &req.book_title;
        let language = req.preferred_language.clone();
        let mut steps = Vec::with_capacity(count);

        for i in 0..count {
            if i % 4 == 0 {
                steps.push(StepDraft {
                    title: format!("Quick win: {title}"),
                    activity_kind: "resource".to_string(),
                    estimated_duration_mins: 10,
                    references_index: None,
                    activity: ActivityDraft {
                        activity_type: "video".to_string(),
                        resource_url: Some(placeholder_url(title, i)),
                        embed_id: None,
                        source_connector: Some("placeholder".to_string()),
                        language: language.clone(),
                    },
                });
            } else if i % 3 == 0 {
                let references_index = if i >= 3 { i - 3 } else { 0 };
                steps.push(StepDraft {
                    title: format!("Spaced review of earlier material in {title}"),
                    activity_kind: "quiz".to_string(),
                    estimated_duration_mins: 15,
                    references_index: Some(references_index),
                    activity: ActivityDraft {
                        activity_type: "quiz".to_string(),
                        resource_url: None,
                        embed_id: None,
                        source_connector: Some("placeholder".to_string()),
                        language: language.clone(),
                    },
                });
            } else {
                let kind = if i % 2 == 0 { "resource" } else { "task" };
                let estimated = 30 + (i % 3) as i32 * 15;
                let (activity_type, resource_url) = if kind == "task" {
                    ("task".to_string(), None)
                } else {
                    (
                        RESOURCE_TYPES[i % RESOURCE_TYPES.len()].to_string(),
                        Some(placeholder_url(title, i)),
                    )
                };
                steps.push(StepDraft {
                    title: format!(
                        "{}: {title}",
                        if kind == "task" { "Practice" } else { "Study" }
                    ),
                    activity_kind: kind.to_string(),
                    estimated_duration_mins: estimated,
                    references_index: None,
                    activity: ActivityDraft {
                        activity_type,
                        resource_url,
                        embed_id: None,
                        source_connector: Some("placeholder".to_string()),
                        language: language.clone(),
                    },
                });
            }
        }

        Ok(steps)
    }
}
