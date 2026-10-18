use async_trait::async_trait;

use super::{BookDraft, DecomposeRequest, GoalDecomposer};

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
    async fn decompose(&self, req: &DecomposeRequest) -> anyhow::Result<Vec<BookDraft>> {
        let count = book_count(&req.title, req.horizon_days);
        let templates = [
            "Fundamentals of {g}",
            "Core Concepts: {g}",
            "Applied Practice: {g}",
            "Advanced {g}",
        ];

        let drafts = (0..count)
            .map(|i| {
                let title = templates[i].replace("{g}", &req.title);
                let description =
                    format!("Part {} of {} covering \"{}\".", i + 1, count, req.title);
                let weight = (count - i) as f32;
                BookDraft {
                    title,
                    description,
                    weight,
                }
            })
            .collect();

        Ok(drafts)
    }
}
