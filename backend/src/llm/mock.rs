use async_trait::async_trait;

use super::{BookDraft, DecomposeRequest, Decomposition, GoalDecomposer};

pub struct MockDecomposer;

impl MockDecomposer {
    pub fn new() -> Self {
        MockDecomposer
    }
}

impl Default for MockDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

const BROAD_KEYWORDS: [&str; 10] = [
    "learn ",
    "master",
    "become",
    "from scratch",
    "fundamentals",
    "introduction to",
    "complete",
    "linux",
    "programming",
    "language",
];

fn is_broad(title: &str) -> bool {
    let lower = title.to_lowercase();
    BROAD_KEYWORDS.iter().any(|k| lower.contains(k))
}

fn book_count(title: &str, horizon_days: i64) -> usize {
    let broad = is_broad(title);
    if broad && horizon_days > 120 {
        4
    } else if broad || horizon_days > 45 {
        3
    } else {
        1
    }
}

#[async_trait]
impl GoalDecomposer for MockDecomposer {
    async fn decompose(&self, req: &DecomposeRequest) -> anyhow::Result<Decomposition> {
        let pt = req.preferred_language.to_lowercase().starts_with("pt");
        let count = book_count(&req.title, req.horizon_days);
        let templates = if pt {
            [
                "Fundamentos de {g}",
                "Conceitos centrais de {g}",
                "Prática aplicada de {g}",
                "Avançado em {g}",
            ]
        } else {
            [
                "Fundamentals of {g}",
                "Core Concepts: {g}",
                "Applied Practice: {g}",
                "Advanced {g}",
            ]
        };

        let books = (0..count)
            .map(|i| {
                let title = templates[i].replace("{g}", &req.title);
                let description = if pt {
                    format!("Parte {} de {} sobre \"{}\".", i + 1, count, req.title)
                } else {
                    format!("Part {} of {} covering \"{}\".", i + 1, count, req.title)
                };
                let prerequisites = if i == 0 {
                    Vec::new()
                } else {
                    vec![templates[i - 1].replace("{g}", &req.title)]
                };
                let weight = (count - i) as f32;
                let (objective, assessment) = if pt {
                    (
                        format!(
                            "Ao final deste módulo, o aluno consegue explicar os fundamentos de \"{}\" e aplicá-los em uma tarefa prática.",
                            title
                        ),
                        format!("Demonstrar \"{}\" em uma tarefa prática.", title),
                    )
                } else {
                    (
                        format!(
                            "By the end of this module, the learner can explain the fundamentals of \"{}\" and apply them in a practical task.",
                            title
                        ),
                        format!("Demonstrate \"{}\" in a practical task.", title),
                    )
                };
                BookDraft {
                    objective,
                    assessment,
                    scope_in: vec![title.clone()],
                    scope_out: Vec::new(),
                    prerequisites,
                    title,
                    description,
                    weight,
                }
            })
            .collect();

        let (goal_outcome, goal_success_criteria) = if pt {
            (
                format!(
                    "Ao final, o aluno consegue aplicar \"{}\" em uma tarefa prática.",
                    req.title
                ),
                vec![
                    format!("Explicar os conceitos centrais de {}.", req.title),
                    format!("Resolver uma tarefa prática de {}.", req.title),
                ],
            )
        } else {
            (
                format!(
                    "By the end, the learner can apply \"{}\" in a practical task.",
                    req.title
                ),
                vec![
                    format!("Explain the core ideas of {}.", req.title),
                    format!("Complete a practical task using {}.", req.title),
                ],
            )
        };

        Ok(Decomposition {
            goal_outcome,
            goal_success_criteria,
            books,
        })
    }
}
