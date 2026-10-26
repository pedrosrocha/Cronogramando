use std::ops::Range;

use anyhow::{bail, Result};
use async_trait::async_trait;
use serde_json::json;

use crate::config::Config;

use super::client::{ChatMessage, LlmClient};

const SEGMENT_SEPARATOR: &str = "<<<SEG>>>";

/// Translate a batch of independent text segments. The caller batches text
/// blocks so one PDF view does not fire hundreds of completion requests.
#[async_trait]
pub trait Translator: Send + Sync {
    fn provider_name(&self) -> String;
    async fn translate_batch(
        &self,
        texts: &[String],
        source_language: &str,
        target_language: &str,
    ) -> Result<Vec<String>>;
}

pub struct MockTranslator;

#[async_trait]
impl Translator for MockTranslator {
    fn provider_name(&self) -> String {
        "mock".to_string()
    }

    async fn translate_batch(
        &self,
        texts: &[String],
        _source_language: &str,
        _target_language: &str,
    ) -> Result<Vec<String>> {
        Ok(texts.to_vec())
    }
}

pub struct LlmTranslator {
    client: LlmClient,
}

impl LlmTranslator {
    pub fn new(cfg: &Config) -> Result<Self> {
        Ok(Self {
            client: LlmClient::new(&cfg.llm_translator)?,
        })
    }
}

#[async_trait]
impl Translator for LlmTranslator {
    fn provider_name(&self) -> String {
        self.client.provider_name()
    }

    async fn translate_batch(
        &self,
        texts: &[String],
        source_language: &str,
        target_language: &str,
    ) -> Result<Vec<String>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        if source_language.eq_ignore_ascii_case(target_language) {
            return Ok(texts.to_vec());
        }

        let system = format!(
            "You are a precise translator. Translate each segment from {source_language} \
             into {target_language}. Preserve Markdown, URLs, code, numbers and inline math \
             exactly. Return only the translated segments, separated by the exact token \
             {SEGMENT_SEPARATOR} with no commentary."
        );
        let user = texts.join(&format!("\n{SEGMENT_SEPARATOR}\n"));

        let source_chars: usize = texts.iter().map(|text| text.chars().count()).sum();
        let max_tokens = ((source_chars / 3) as u32 + 512).clamp(1024, 16_000);

        let reasoning = json!({ "enabled": false });
        let content = self
            .client
            .chat(
                &[ChatMessage::system(system), ChatMessage::user(user)],
                0.0,
                Some(max_tokens),
                None,
                Some(&reasoning),
            )
            .await?;

        let translated: Vec<String> = content
            .split(SEGMENT_SEPARATOR)
            .map(|segment| segment.trim().to_string())
            .collect();
        if translated.len() != texts.len() {
            bail!(
                "translation provider returned {} segments, expected {}",
                translated.len(),
                texts.len()
            );
        }

        Ok(translated)
    }
}

/// Split translated text blocks into bounded batches. Keeps a single huge
/// paragraph in its own batch and caps every request by character count.
pub fn batch_ranges(texts: &[String], max_chars: usize, max_items: usize) -> Vec<Range<usize>> {
    let max_chars = max_chars.max(1);
    let max_items = max_items.max(1);
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut chars = 0;

    for (index, text) in texts.iter().enumerate() {
        let length = text.chars().count();
        if index > start && (chars + length > max_chars || index - start >= max_items) {
            ranges.push(start..index);
            start = index;
            chars = 0;
        }
        chars += length;
    }
    if start < texts.len() {
        ranges.push(start..texts.len());
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn batches_by_character_budget() {
        let ranges = batch_ranges(&texts(&["aaaa", "bbbb", "cccc"]), 8, 10);
        assert_eq!(ranges, vec![0..2, 2..3]);
    }

    #[test]
    fn batches_by_item_count() {
        let ranges = batch_ranges(&texts(&["a", "b", "c", "d", "e"]), 10_000, 2);
        assert_eq!(ranges, vec![0..2, 2..4, 4..5]);
    }

    #[test]
    fn huge_segment_gets_its_own_batch() {
        let ranges = batch_ranges(&texts(&["a", "abcdefghij", "b"]), 8, 10);
        assert_eq!(ranges, vec![0..1, 1..2, 2..3]);
    }

    #[test]
    fn empty_input_has_no_batches() {
        assert!(batch_ranges(&[], 8, 10).is_empty());
    }
}
