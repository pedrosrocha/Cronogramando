use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::config::Config;

use super::{
    client::{parse_json, ChatMessage, LlmClient},
    ActivityDraft, BookPlanner, PlanRequest, StepDraft,
};

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

fn assessment_step(req: &PlanRequest, references_index: Option<usize>) -> StepDraft {
    let pt = req.preferred_language.to_lowercase().starts_with("pt");
    let criteria = if req.book_assessment.trim().is_empty() {
        if pt {
            format!("Demonstrar o objetivo do módulo: {}", req.book_objective)
        } else {
            format!("Demonstrate the module objective: {}", req.book_objective)
        }
    } else {
        req.book_assessment.clone()
    };
    StepDraft {
        title: if pt {
            format!("Avaliação: {}", req.book_title)
        } else {
            format!("Assessment: {}", req.book_title)
        },
        learning_outcome: criteria.clone(),
        success_criteria: criteria,
        content_intent: "assessment".to_string(),
        activity_kind: "quiz".to_string(),
        estimated_duration_mins: 20,
        references_index,
        activity: ActivityDraft {
            activity_type: "quiz".to_string(),
            resource_url: None,
            embed_id: None,
            source_connector: Some("placeholder".to_string()),
            source_category: None,
            language: req.preferred_language.clone(),
        },
    }
}

#[async_trait]
impl BookPlanner for MockBookPlanner {
    async fn plan(&self, req: &PlanRequest) -> anyhow::Result<Vec<StepDraft>> {
        let pt = req.preferred_language.to_lowercase().starts_with("pt");
        let total = req.planned_duration_mins.max(30);
        let count = ((total / 30).clamp(3, 12)) as usize;
        let title = &req.book_title;
        let language = req.preferred_language.clone();
        let mut steps = Vec::with_capacity(count + 1);

        for i in 0..count {
            if i % 4 == 0 {
                steps.push(StepDraft {
                    title: if pt {
                        format!("Vitória rápida: {title}")
                    } else {
                        format!("Quick win: {title}")
                    },
                    learning_outcome: if pt {
                        format!(
                            "Ao final desta etapa, o aluno consegue dar o primeiro passo em {title}."
                        )
                    } else {
                        format!(
                            "By the end of this step, the learner can take the first step in {title}."
                        )
                    },
                    success_criteria: if pt {
                        format!("Concluir a tarefa curta de {title} e registrar uma dúvida.")
                    } else {
                        format!("Complete the short {title} task and record one question.")
                    },
                    content_intent: "practice".to_string(),
                    activity_kind: "resource".to_string(),
                    estimated_duration_mins: 10,
                    references_index: None,
                    activity: ActivityDraft {
                        activity_type: "video".to_string(),
                        resource_url: Some(placeholder_url(title, i)),
                        embed_id: None,
                        source_connector: Some("placeholder".to_string()),
                        source_category: None,
                        language: language.clone(),
                    },
                });
            } else if i % 3 == 0 {
                let references_index = if i >= 3 { i - 3 } else { 0 };
                steps.push(StepDraft {
                    title: if pt {
                        format!("Revisão espaçada de material anterior em {title}")
                    } else {
                        format!("Spaced review of earlier material in {title}")
                    },
                    learning_outcome: if pt {
                        format!(
                            "Ao final desta etapa, o aluno consegue recordar o conteúdo anterior de {title}."
                        )
                    } else {
                        format!(
                            "By the end of this step, the learner can recall earlier material from {title}."
                        )
                    },
                    success_criteria: if pt {
                        "Acertar a maioria das questões de revisão.".to_string()
                    } else {
                        "Answer most review questions correctly.".to_string()
                    },
                    content_intent: "review".to_string(),
                    activity_kind: "quiz".to_string(),
                    estimated_duration_mins: 15,
                    references_index: Some(references_index),
                    activity: ActivityDraft {
                        activity_type: "quiz".to_string(),
                        resource_url: None,
                        embed_id: None,
                        source_connector: Some("placeholder".to_string()),
                        source_category: None,
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
                let intent = if kind == "task" {
                    "practice"
                } else if i % 2 == 0 {
                    "explanation"
                } else {
                    "worked_example"
                };
                let prefix = match (pt, kind) {
                    (true, "task") => "Prática",
                    (true, _) => "Estudo",
                    (false, "task") => "Practice",
                    (false, _) => "Study",
                };
                steps.push(StepDraft {
                    title: format!("{prefix}: {title}"),
                    learning_outcome: if pt {
                        format!(
                            "Ao final desta etapa, o aluno consegue aplicar {title} em uma tarefa."
                        )
                    } else {
                        format!(
                            "By the end of this step, the learner can apply {title} in a task."
                        )
                    },
                    success_criteria: if pt {
                        format!(
                            "Produzir ou explicar um exemplo de {title} sem consultar o gabarito."
                        )
                    } else {
                        format!(
                            "Produce or explain an example of {title} without consulting the answer key."
                        )
                    },
                    content_intent: intent.to_string(),
                    activity_kind: kind.to_string(),
                    estimated_duration_mins: estimated,
                    references_index: None,
                    activity: ActivityDraft {
                        activity_type,
                        resource_url,
                        embed_id: None,
                        source_connector: Some("placeholder".to_string()),
                        source_category: None,
                        language: language.clone(),
                    },
                });
            }
        }

        if !steps
            .last()
            .map(|step| step.content_intent == "assessment")
            .unwrap_or(false)
        {
            let reference = steps.len().saturating_sub(1);
            steps.push(assessment_step(req, Some(reference)));
        }

        Ok(steps)
    }
}

pub struct LlmBookPlanner {
    client: LlmClient,
}

impl LlmBookPlanner {
    pub fn new(cfg: &Config) -> Result<Self> {
        Ok(Self {
            client: LlmClient::new(&cfg.llm_planner)?,
        })
    }
}

#[derive(Debug, Deserialize)]
struct PlanPayload {
    steps: Vec<PlanItem>,
}

#[derive(Debug, Deserialize)]
struct PlanItem {
    title: String,
    #[serde(default)]
    learning_outcome: String,
    #[serde(default)]
    success_criteria: String,
    #[serde(default)]
    content_intent: String,
    #[serde(default)]
    activity_kind: String,
    #[serde(default)]
    estimated_duration_mins: Option<i32>,
    #[serde(default)]
    references_index: Option<usize>,
    #[serde(default)]
    activity_type: Option<String>,
}

#[async_trait]
impl BookPlanner for LlmBookPlanner {
    async fn plan(&self, req: &PlanRequest) -> Result<Vec<StepDraft>> {
        let total_minutes = req.planned_duration_mins.max(30);
        let min_steps = ((total_minutes / 45).clamp(3, 6)) as usize;
        let max_steps = ((total_minutes / 15).clamp(6, 20)) as usize;

        let system = format!(
            "You are an expert instructional designer. Create a study plan for ONE \
             module that achieves its objective within the available time.\n\
             Return only JSON matching this schema: \
             {{\"steps\":[{{\"title\":\"string\",\"learning_outcome\":\"string\",\
             \"success_criteria\":\"string\",\
             \"content_intent\":\"explanation|worked_example|practice|assessment|review\",\
             \"activity_kind\":\"resource|quiz|task\",\
             \"estimated_duration_mins\":integer,\"references_index\":integer|null,\
             \"activity_type\":\"video|pdf|article|podcast|quiz|task\"}}]}}.\n\
             Rules:\n\
             - Write all user-facing text in {language}.\n\
             - Produce between {min_steps} and {max_steps} ordered steps.\n\
             - The sum of estimated_duration_mins must be close to {total_minutes} minutes \
               (within about 20%).\n\
             - Every step must state an observable learning_outcome and a measurable \
               success_criteria.\n\
             - The final step must have content_intent \"assessment\" and match the module \
               assessment.\n\
             - explanation and worked_example steps use resource content; practice uses \
               tasks or exercises; assessment uses a quiz or task.\n\
             - Use review steps to reference earlier steps for spaced retrieval.\n\
             - For a quiz or task, activity_type must match the kind.\n\
             - resource_url must not be included: the application supplies real links.\n\
             - references_index may point to an earlier step (0-based) for spaced review, \
               otherwise null. It must never point to the current or a later step.\n\
             - Do not include markdown, code fences, or commentary.",
            language = req.preferred_language,
            min_steps = min_steps,
            max_steps = max_steps,
            total_minutes = total_minutes,
        );

        let why = req
            .why_statement
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("\nLearner's reason: {value}"))
            .unwrap_or_default();
        let previous = req
            .previous_book_objective
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("\nPrevious module objective: {value}"))
            .unwrap_or_default();
        let next = req
            .next_book_objective
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("\nNext module objective: {value}"))
            .unwrap_or_default();

        let user = format!(
            "Goal: {goal_title}\nGoal outcome: {goal_outcome}{why}\n\
             Module: {book_title}\nModule description: {book_description}\n\
             Module objective: {book_objective}\n\
             Scope in: {scope_in}\nScope out: {scope_out}\n\
             Prerequisites: {prerequisites}\nModule assessment: {assessment}\n\
             Planned study time: {minutes} minutes{previous}{next}",
            goal_title = req.goal_title,
            goal_outcome = req.goal_outcome,
            book_title = req.book_title,
            book_description = req.book_description,
            book_objective = req.book_objective,
            scope_in = req.book_scope_in.join("; "),
            scope_out = req.book_scope_out.join("; "),
            prerequisites = req.book_prerequisites.join("; "),
            assessment = req.book_assessment,
            minutes = total_minutes,
        );

        let schema = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "module_plan",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "steps": {
                            "type": "array",
                            "minItems": min_steps,
                            "maxItems": max_steps,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": { "type": "string" },
                                    "learning_outcome": { "type": "string" },
                                    "success_criteria": { "type": "string" },
                                    "content_intent": {
                                        "type": "string",
                                        "enum": ["explanation", "worked_example", "practice", "assessment", "review"]
                                    },
                                    "activity_kind": {
                                        "type": "string",
                                        "enum": ["resource", "quiz", "task"]
                                    },
                                    "estimated_duration_mins": {
                                        "type": "integer",
                                        "minimum": 5,
                                        "maximum": 180
                                    },
                                    "references_index": {
                                        "anyOf": [
                                            { "type": "integer", "minimum": 0 },
                                            { "type": "null" }
                                        ]
                                    },
                                    "activity_type": {
                                        "type": "string",
                                        "enum": ["video", "pdf", "article", "podcast", "quiz", "task"]
                                    }
                                },
                                "required": [
                                    "title", "learning_outcome", "success_criteria",
                                    "content_intent", "activity_kind",
                                    "estimated_duration_mins", "activity_type"
                                ],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["steps"],
                    "additionalProperties": false
                }
            }
        });

        let reasoning = json!({ "effort": "low" });
        let raw = self
            .client
            .chat(
                &[ChatMessage::system(system), ChatMessage::user(user)],
                0.4,
                Some(4000),
                Some(&schema),
                Some(&reasoning),
            )
            .await?;

        let payload: PlanPayload = parse_json(&raw, "book plan")?;
        validate(payload.steps, req)
    }
}

fn validate(items: Vec<PlanItem>, req: &PlanRequest) -> Result<Vec<StepDraft>> {
    let mut steps = Vec::new();
    for item in items.into_iter().take(20) {
        let title = item.title.trim();
        if title.is_empty() {
            continue;
        }

        let kind = match item.activity_kind.trim().to_lowercase().as_str() {
            "quiz" => "quiz",
            "task" => "task",
            _ => "resource",
        };

        let activity_type = match (kind, item.activity_type.as_deref().unwrap_or("")) {
            ("quiz", _) => "quiz",
            ("task", _) => "task",
            (_, "video") => "video",
            (_, "pdf") => "pdf",
            (_, "podcast") => "podcast",
            (_, "article") => "article",
            _ => "article",
        };

        let content_intent = match item.content_intent.trim().to_lowercase().as_str() {
            "explanation" => "explanation",
            "worked_example" => "worked_example",
            "practice" => "practice",
            "assessment" => "assessment",
            "review" => "review",
            _ => match kind {
                "quiz" => "assessment",
                "task" => "practice",
                _ => "explanation",
            },
        };

        let current_index = steps.len();
        let references_index = item.references_index.filter(|index| *index < current_index);

        let learning_outcome = if item.learning_outcome.trim().is_empty() {
            format!("Ao final desta etapa, o aluno consegue aplicar {title}.")
        } else {
            item.learning_outcome.trim().to_string()
        };
        let success_criteria = if item.success_criteria.trim().is_empty() {
            format!("Concluir {title} e explicar o resultado.")
        } else {
            item.success_criteria.trim().to_string()
        };

        steps.push(StepDraft {
            title: title.to_string(),
            learning_outcome,
            success_criteria,
            content_intent: content_intent.to_string(),
            activity_kind: kind.to_string(),
            estimated_duration_mins: item.estimated_duration_mins.unwrap_or(30).clamp(5, 180),
            references_index,
            activity: ActivityDraft {
                activity_type: activity_type.to_string(),
                resource_url: None,
                embed_id: None,
                source_connector: None,
                source_category: None,
                language: req.preferred_language.clone(),
            },
        });
    }

    if steps.is_empty() {
        bail!("book plan returned no usable steps");
    }

    if steps
        .last()
        .map(|step| step.content_intent != "assessment")
        .unwrap_or(false)
    {
        let reference = steps.len().saturating_sub(1);
        let criteria = if req.book_assessment.trim().is_empty() {
            format!("Demonstrar o objetivo do módulo: {}", req.book_objective)
        } else {
            req.book_assessment.clone()
        };
        steps.push(StepDraft {
            title: format!("Assessment: {}", req.book_title),
            learning_outcome: criteria.clone(),
            success_criteria: criteria,
            content_intent: "assessment".to_string(),
            activity_kind: "quiz".to_string(),
            estimated_duration_mins: 20,
            references_index: Some(reference),
            activity: ActivityDraft {
                activity_type: "quiz".to_string(),
                resource_url: None,
                embed_id: None,
                source_connector: None,
                source_category: None,
                language: req.preferred_language.clone(),
            },
        });
    }

    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PlanRequest {
        PlanRequest {
            goal_title: "Redação ENEM".into(),
            goal_outcome: "Escrever uma redação nota 800+.".into(),
            why_statement: None,
            book_title: "Matriz de correção".into(),
            book_description: "Entender as competências.".into(),
            book_objective:
                "Ao final deste módulo, o aluno consegue explicar as cinco competências.".into(),
            book_scope_in: vec!["cinco competências".into()],
            book_scope_out: vec![],
            book_prerequisites: vec![],
            book_assessment: "Corrigir uma redação com justificativa.".into(),
            previous_book_objective: None,
            next_book_objective: None,
            planned_duration_mins: 120,
            preferred_language: "pt-BR".into(),
        }
    }

    #[test]
    fn appends_an_assessment_when_the_plan_ends_without_one() {
        let items = vec![PlanItem {
            title: "Explicar as competências".into(),
            learning_outcome: "Listar as cinco competências.".into(),
            success_criteria: "Acertar 4 de 5.".into(),
            content_intent: "explanation".into(),
            activity_kind: "resource".into(),
            estimated_duration_mins: Some(30),
            references_index: None,
            activity_type: Some("video".into()),
        }];
        let steps = validate(items, &request()).unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[1].content_intent, "assessment");
        assert_eq!(
            steps[1].success_criteria,
            "Corrigir uma redação com justificativa."
        );
    }

    #[test]
    fn fills_a_missing_learning_outcome() {
        let items = vec![PlanItem {
            title: "Ler o guia".into(),
            learning_outcome: String::new(),
            success_criteria: String::new(),
            content_intent: "explanation".into(),
            activity_kind: "resource".into(),
            estimated_duration_mins: Some(20),
            references_index: None,
            activity_type: Some("article".into()),
        }];
        let steps = validate(items, &request()).unwrap();
        assert!(steps[0].learning_outcome.contains("Ler o guia"));
        assert!(!steps[0].success_criteria.is_empty());
    }
}
