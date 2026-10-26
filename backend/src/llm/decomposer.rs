use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::config::Config;

use super::{
    client::{parse_json, ChatMessage, LlmClient},
    BookDraft, DecomposeRequest, Decomposition, GoalDecomposer,
};

pub struct LlmGoalDecomposer {
    client: LlmClient,
}

impl LlmGoalDecomposer {
    pub fn new(cfg: &Config) -> Result<Self> {
        Ok(Self {
            client: LlmClient::new(&cfg.llm_decomposer)?,
        })
    }
}

#[derive(Debug, Deserialize)]
struct BooksPayload {
    #[serde(default)]
    goal_outcome: String,
    #[serde(default)]
    goal_success_criteria: Vec<String>,
    books: Vec<BookItem>,
}

#[derive(Debug, Deserialize)]
struct BookItem {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    objective: String,
    #[serde(default)]
    scope_in: Vec<String>,
    #[serde(default)]
    scope_out: Vec<String>,
    #[serde(default)]
    prerequisites: Vec<String>,
    #[serde(default)]
    assessment: String,
    #[serde(default)]
    weight: Option<f32>,
}

const OBSERVABLE_VERBS: [&str; 38] = [
    "explicar",
    "identificar",
    "escrever",
    "corrigir",
    "comparar",
    "aplicar",
    "analisar",
    "descrever",
    "criar",
    "resolver",
    "calcular",
    "avaliar",
    "planejar",
    "construir",
    "demonstrar",
    "listar",
    "classificar",
    "interpretar",
    "produzir",
    "argumentar",
    "justificar",
    "selecionar",
    "organizar",
    "implementar",
    "usar",
    "executar",
    "praticar",
    "revisar",
    "diagnosticar",
    "mapear",
    "relacionar",
    "sintetizar",
    "defender",
    "propor",
    "explain",
    "identify",
    "write",
    "apply",
];

fn has_observable_verb(objective: &str) -> bool {
    let lower = objective.to_lowercase();
    OBSERVABLE_VERBS.iter().any(|verb| lower.contains(verb))
}

fn clean_list(items: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in items {
        let item = item.trim().to_string();
        if !item.is_empty()
            && !out
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&item))
        {
            out.push(item);
        }
    }
    out
}

#[async_trait]
impl GoalDecomposer for LlmGoalDecomposer {
    async fn decompose(&self, req: &DecomposeRequest) -> Result<Decomposition> {
        let (min_books, max_books) = if req.horizon_days <= 30 {
            (1, 2)
        } else if req.horizon_days <= 90 {
            (2, 4)
        } else {
            (3, 6)
        };

        let system = format!(
            "You are an expert curriculum designer. Turn a learning goal into an \
             ordered sequence of modules. A module is a measurable objective, not a \
             topic bucket.\n\
             Return only JSON matching this schema: \
             {{\"goal_outcome\":\"string\",\"goal_success_criteria\":[\"string\"],\
             \"books\":[{{\"title\":\"string\",\"description\":\"string\",\
             \"objective\":\"string\",\"scope_in\":[\"string\"],\
             \"scope_out\":[\"string\"],\"prerequisites\":[\"string\"],\
             \"assessment\":\"string\",\"weight\":number}}]}}.\n\
             Rules:\n\
             - Write all user-facing text in {language}.\n\
             - Produce between {min_books} and {max_books} modules, ordered from \
               fundamentals to application to assessment.\n\
             - goal_outcome: one sentence, observable, with a standard.\n\
             - goal_success_criteria: 2 to 4 measurable criteria.\n\
             - objective: \"Ao final deste módulo, o aluno consegue <verb> <what> <standard>.\"\n\
             - description: one or two sentences for the UI.\n\
             - scope_in: 3 to 5 concrete topics or skills.\n\
             - scope_out: 1 to 3 things explicitly out of scope.\n\
             - prerequisites: earlier module titles or prior knowledge; [] for the first.\n\
             - assessment: one concrete task that proves the objective.\n\
             - weight: a number from 1 to 5; higher values for heavier modules.\n\
             - Avoid generic titles such as \"Fundamentos\", \"Introdução\" or \
               \"Prática\" unless the objective is specific.\n\
             - Do not include markdown, code fences, or commentary.",
            language = req.preferred_language,
        );

        let why = req
            .why_statement
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("\nThe learner's reason: {value}"))
            .unwrap_or_default();

        let user = format!(
            "Learning goal: {title}\nAvailable time: {days} days (~{capacity} minutes){why}",
            title = req.title,
            days = req.horizon_days,
            capacity = req.capacity_mins,
        );

        let schema = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "objective_decomposition",
                "strict": true,
                "schema": {
                    "type": "object",
                    "properties": {
                        "goal_outcome": { "type": "string" },
                        "goal_success_criteria": {
                            "type": "array",
                            "minItems": 2,
                            "maxItems": 4,
                            "items": { "type": "string" }
                        },
                        "books": {
                            "type": "array",
                            "minItems": min_books,
                            "maxItems": max_books,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": { "type": "string" },
                                    "description": { "type": "string" },
                                    "objective": { "type": "string" },
                                    "scope_in": {
                                        "type": "array",
                                        "minItems": 1,
                                        "maxItems": 6,
                                        "items": { "type": "string" }
                                    },
                                    "scope_out": {
                                        "type": "array",
                                        "maxItems": 4,
                                        "items": { "type": "string" }
                                    },
                                    "prerequisites": {
                                        "type": "array",
                                        "maxItems": 6,
                                        "items": { "type": "string" }
                                    },
                                    "assessment": { "type": "string" },
                                    "weight": { "type": "number", "minimum": 1, "maximum": 5 }
                                },
                                "required": [
                                    "title", "description", "objective", "scope_in",
                                    "scope_out", "prerequisites", "assessment", "weight"
                                ],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["goal_outcome", "goal_success_criteria", "books"],
                    "additionalProperties": false
                }
            }
        });

        let reasoning = json!({ "effort": "low" });
        let raw = self
            .client
            .chat(
                &[ChatMessage::system(system), ChatMessage::user(user)],
                0.3,
                Some(2400),
                Some(&schema),
                Some(&reasoning),
            )
            .await?;

        let payload: BooksPayload = parse_json(&raw, "goal decomposition")?;
        validate(payload, &req.title, max_books as usize)
    }
}

fn validate(payload: BooksPayload, goal_title: &str, max_books: usize) -> Result<Decomposition> {
    let mut goal_outcome = payload.goal_outcome.trim().to_string();
    if goal_outcome.is_empty() || goal_outcome.eq_ignore_ascii_case(goal_title.trim()) {
        goal_outcome = format!(
            "Ao final, o aluno consegue aplicar os conhecimentos de \"{}\" em uma tarefa prática.",
            goal_title.trim()
        );
    }

    let mut goal_success_criteria = clean_list(payload.goal_success_criteria);
    if goal_success_criteria.is_empty() {
        goal_success_criteria.push(format!(
            "Demonstrar \"{}\" em uma tarefa prática.",
            goal_title.trim()
        ));
    }

    let mut books: Vec<BookDraft> = Vec::new();
    for item in payload.books.into_iter().take(max_books) {
        let title = item.title.trim().to_string();
        if title.is_empty() {
            continue;
        }
        if books
            .iter()
            .any(|existing| existing.title.eq_ignore_ascii_case(&title))
        {
            continue;
        }

        let mut objective = item.objective.trim().to_string();
        if objective.len() < 40 || !has_observable_verb(&objective) {
            objective = format!(
                "Ao final deste módulo, o aluno consegue aplicar os conhecimentos de \"{}\" em uma tarefa prática.",
                title
            );
        }

        let mut assessment = item.assessment.trim().to_string();
        if assessment.is_empty() {
            assessment = format!("Demonstrar \"{}\" em uma tarefa prática.", title);
        }

        let mut scope_in = clean_list(item.scope_in);
        if scope_in.is_empty() {
            scope_in.push(title.clone());
        }

        let mut description = item.description.trim().to_string();
        if description.is_empty() {
            description = format!("Módulo focado em {}.", title);
        }

        let weight = item
            .weight
            .filter(|w| w.is_finite() && *w > 0.0)
            .unwrap_or(1.0);

        books.push(BookDraft {
            title,
            description,
            objective,
            scope_in,
            scope_out: clean_list(item.scope_out),
            prerequisites: clean_list(item.prerequisites),
            assessment,
            weight,
        });
    }

    if books.is_empty() {
        bail!("goal decomposition returned no usable modules");
    }

    Ok(Decomposition {
        goal_outcome,
        goal_success_criteria,
        books,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(objective: &str, assessment: &str) -> BooksPayload {
        BooksPayload {
            goal_outcome: String::new(),
            goal_success_criteria: vec![],
            books: vec![BookItem {
                title: "Matriz de correção".into(),
                description: String::new(),
                objective: objective.into(),
                scope_in: vec![],
                scope_out: vec![],
                prerequisites: vec![],
                assessment: assessment.into(),
                weight: Some(2.0),
            }],
        }
    }

    #[test]
    fn fills_a_missing_goal_outcome() {
        let decomposition = validate(payload("", ""), "Redação ENEM", 4).unwrap();
        assert!(decomposition.goal_outcome.contains("Redação ENEM"));
        assert!(!decomposition.goal_success_criteria.is_empty());
    }

    #[test]
    fn replaces_a_generic_objective() {
        let decomposition =
            validate(payload("Técnicas de redação", ""), "Redação ENEM", 4).unwrap();
        assert!(decomposition.books[0].objective.contains("consegue"));
        assert!(!decomposition.books[0].assessment.is_empty());
    }

    #[test]
    fn keeps_a_specific_objective() {
        let objective = "Ao final deste módulo, o aluno consegue explicar as cinco competências e classificar trechos de uma redação modelo.";
        let decomposition = validate(
            payload(objective, "Corrigir uma redação."),
            "Redação ENEM",
            4,
        )
        .unwrap();
        assert_eq!(decomposition.books[0].objective, objective);
        assert_eq!(decomposition.books[0].assessment, "Corrigir uma redação.");
    }
}
