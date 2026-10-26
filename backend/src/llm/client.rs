use std::time::Duration;

use anyhow::{bail, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::config::LlmConfig;

/// A shared OpenAI-compatible chat-completions client.
///
/// It owns the model fallback chain and retry/backoff policy so every LLM role
/// (decomposer, planner, translator) behaves the same way.
pub struct LlmClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    models: Vec<String>,
    max_retries: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

impl LlmClient {
    pub fn new(cfg: &LlmConfig) -> Result<Self> {
        let api_key = cfg
            .api_key
            .clone()
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("LLM API key is not configured"))?;
        let models: Vec<String> = cfg
            .models
            .iter()
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty())
            .collect();
        if models.is_empty() {
            bail!("no LLM models are configured");
        }

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.timeout_secs.max(5)))
            .build()?;

        Ok(Self {
            http,
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key,
            models,
            max_retries: cfg.max_retries,
        })
    }

    pub fn provider_name(&self) -> String {
        let host = self
            .base_url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(&self.base_url);
        format!("{host}:{}", self.models[0])
    }

    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        temperature: f32,
        max_tokens: Option<u32>,
        response_format: Option<&Value>,
        reasoning: Option<&Value>,
    ) -> Result<String> {
        let mut errors = Vec::new();

        for model in &self.models {
            let mut use_format = response_format;
            let mut use_reasoning = reasoning;
            let mut can_drop_format = use_format.is_some();
            let mut can_drop_reasoning = use_reasoning.is_some();
            let mut attempt = 0u32;

            loop {
                match self
                    .request_once(
                        model,
                        messages,
                        temperature,
                        max_tokens,
                        use_format,
                        use_reasoning,
                    )
                    .await
                {
                    Ok(content) => return Ok(content),
                    Err(error) => {
                        if matches!(error.status, Some(400 | 422)) {
                            if can_drop_format && use_format.is_some() {
                                can_drop_format = false;
                                use_format = None;
                                tracing::debug!(
                                    model = %model,
                                    "provider rejected response_format; retrying without it"
                                );
                                continue;
                            }
                            if can_drop_reasoning && use_reasoning.is_some() {
                                can_drop_reasoning = false;
                                use_reasoning = None;
                                tracing::debug!(
                                    model = %model,
                                    "provider rejected reasoning options; retrying without them"
                                );
                                continue;
                            }
                        }

                        if error.retryable && attempt < self.max_retries {
                            attempt += 1;
                            let delay = error
                                .retry_after
                                .unwrap_or_else(|| Duration::from_millis(500 * 2u64.pow(attempt)));
                            tracing::warn!(
                                model = %model,
                                attempt,
                                error = %error.message,
                                "LLM request failed; retrying"
                            );
                            tokio::time::sleep(delay).await;
                            continue;
                        }

                        tracing::warn!(
                            model = %model,
                            error = %error.message,
                            "LLM model failed; trying next fallback"
                        );
                        errors.push(format!("{model}: {}", error.message));
                        break;
                    }
                }
            }
        }

        bail!("all configured LLM models failed: {}", errors.join(" | "))
    }

    async fn request_once(
        &self,
        model: &str,
        messages: &[ChatMessage],
        temperature: f32,
        max_tokens: Option<u32>,
        response_format: Option<&Value>,
        reasoning: Option<&Value>,
    ) -> std::result::Result<String, ChatError> {
        let request = ChatRequest {
            model,
            messages,
            temperature,
            max_tokens,
            response_format,
            reasoning,
        };

        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|error| ChatError {
                message: format!("network error: {error}"),
                status: None,
                retryable: true,
                retry_after: None,
            })?;

        let status = response.status();
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(Duration::from_secs);
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ChatError {
                message: format!("HTTP {status}: {}", truncate(&body, 600)),
                status: Some(status.as_u16()),
                retryable: status == reqwest::StatusCode::TOO_MANY_REQUESTS
                    || status == reqwest::StatusCode::REQUEST_TIMEOUT
                    || status.is_server_error(),
                retry_after,
            });
        }

        let payload: ChatResponse = serde_json::from_str(&body).map_err(|error| ChatError {
            message: format!(
                "invalid chat-completions payload: {error}: {}",
                truncate(&body, 600)
            ),
            status: Some(status.as_u16()),
            retryable: false,
            retry_after: None,
        })?;

        if let Some(error) = payload.error {
            return Err(ChatError {
                message: format!("provider error: {}", truncate(&error.to_string(), 600)),
                status: Some(status.as_u16()),
                retryable: false,
                retry_after: None,
            });
        }

        let content = payload
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .unwrap_or_default();
        if content.trim().is_empty() {
            return Err(ChatError {
                message: "provider returned an empty completion".to_string(),
                status: Some(status.as_u16()),
                // An empty completion is usually the model refusing or being
                // overloaded; retrying the same model rarely helps and can add
                // minutes of latency. Move on to the next fallback model.
                retryable: false,
                retry_after: None,
            });
        }

        Ok(content)
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<&'a Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<&'a Value>,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatContent,
}

#[derive(Deserialize)]
struct ChatContent {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug)]
struct ChatError {
    message: String,
    status: Option<u16>,
    retryable: bool,
    retry_after: Option<Duration>,
}

/// Parse a JSON value from a model response, tolerating markdown fences and
/// short leading/trailing commentary that some free models add.
pub fn parse_json<T: DeserializeOwned>(raw: &str, context: &str) -> Result<T> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }
    if let Some(fenced) = strip_code_fence(trimmed) {
        if let Ok(value) = serde_json::from_str(&fenced) {
            return Ok(value);
        }
    }
    if let Some(embedded) = extract_braced(trimmed) {
        if let Ok(value) = serde_json::from_str(embedded) {
            return Ok(value);
        }
    }
    bail!(
        "{context}: model did not return valid JSON: {}",
        truncate(raw, 500)
    )
}

fn strip_code_fence(raw: &str) -> Option<String> {
    let start = raw.find("```")?;
    let body = &raw[start + 3..];
    let body = body.trim_start();
    let body = body.strip_prefix("json").unwrap_or(body).trim_start();
    let end = body.rfind("```")?;
    Some(body[..end].trim().to_string())
}

fn extract_braced(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        None
    } else {
        Some(&raw[start..=end])
    }
}

fn truncate(raw: &str, max_chars: usize) -> String {
    if raw.chars().count() <= max_chars {
        return raw.to_string();
    }
    let mut out: String = raw.chars().take(max_chars).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Sample {
        value: i32,
    }

    #[test]
    fn parses_plain_json() {
        let parsed: Sample = parse_json(r#"{"value": 7}"#, "test").unwrap();
        assert_eq!(parsed, Sample { value: 7 });
    }

    #[test]
    fn parses_fenced_json() {
        let raw = "```json\n{\"value\": 3}\n```";
        let parsed: Sample = parse_json(raw, "test").unwrap();
        assert_eq!(parsed, Sample { value: 3 });
    }

    #[test]
    fn parses_json_with_surrounding_commentary() {
        let raw = "Here is the result:\n{\"value\": 9}\nHope this helps.";
        let parsed: Sample = parse_json(raw, "test").unwrap();
        assert_eq!(parsed, Sample { value: 9 });
    }

    #[test]
    fn rejects_non_json() {
        let error = parse_json::<Sample>("not json", "test").unwrap_err();
        assert!(error.to_string().contains("did not return valid JSON"));
    }
}
