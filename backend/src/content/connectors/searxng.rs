use anyhow::Result;
use serde::Deserialize;
use url::Url;

use super::language_tag;
use crate::config::SearxngConfig;
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

#[derive(Debug, Deserialize)]
struct SearxResponse {
    #[serde(default)]
    results: Vec<SearxResult>,
}

#[derive(Debug, Deserialize)]
struct SearxResult {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    length: Option<String>,
    #[serde(default)]
    iframe_src: Option<String>,
    #[serde(default)]
    metadata: Option<String>,
}

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
    cfg: &SearxngConfig,
    media: &str,
) -> Result<Vec<ResourceCandidate>> {
    if !cfg.enabled() {
        return Ok(Vec::new());
    }

    let endpoint = format!("{}/search", cfg.base_url.trim_end_matches('/'));
    let language = if cfg.language.trim().is_empty() {
        query.language.as_str()
    } else {
        cfg.language.as_str()
    };
    let mut form = vec![
        ("q", query.query.as_str()),
        ("format", "json"),
        ("language", language),
        (
            "categories",
            if media == "videos" {
                "videos"
            } else {
                "general"
            },
        ),
        ("safesearch", "0"),
        ("pageno", "1"),
    ];
    if !cfg.engines.trim().is_empty() {
        form.push(("engines", cfg.engines.as_str()));
    }

    let response: SearxResponse = client
        .post(endpoint)
        .form(&form)
        .timeout(std::time::Duration::from_secs(cfg.timeout_secs.max(2)))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(parse_results(
        response,
        query,
        category,
        cfg.max_results,
        media,
    ))
}

fn parse_results(
    response: SearxResponse,
    query: &ConnectorQuery,
    category: SourceCategory,
    max_results: usize,
    media: &str,
) -> Vec<ResourceCandidate> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for result in response.results.into_iter().take(max_results * 2) {
        let Some(raw_url) = result.url.as_deref() else {
            continue;
        };
        let Ok(url) = Url::parse(raw_url) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https") {
            continue;
        }
        let host = url.host_str().unwrap_or("").to_lowercase();
        let is_video_host = host.contains("youtube.")
            || host == "youtu.be"
            || host.contains("dailymotion.")
            || host.contains("vimeo.");
        if media == "videos" && !is_video_host {
            continue;
        }
        if media != "videos" && is_video_host {
            continue;
        }
        let title = result.title.trim();
        if title.is_empty() {
            continue;
        }
        let key = url.to_string();
        if !seen.insert(key.clone()) {
            continue;
        }

        let activity_type = if media == "videos" {
            "video".to_string()
        } else if url.path().to_lowercase().ends_with(".pdf") {
            "pdf".to_string()
        } else {
            "article".to_string()
        };
        let embed_id = if activity_type == "video" {
            result
                .iframe_src
                .as_deref()
                .and_then(youtube_id)
                .or_else(|| youtube_id(&key))
        } else {
            None
        };
        let duration_secs = result.length.as_deref().and_then(parse_length);
        let description = result
            .content
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| {
                if let Some(channel) = result.metadata.as_deref().filter(|m| !m.trim().is_empty()) {
                    format!("{channel}: {text}")
                } else {
                    text.to_string()
                }
            });

        out.push(ResourceCandidate {
            title: title.to_string(),
            url: Some(key),
            embed_id,
            connector: ConnectorKind::Searxng,
            language: language_tag(&query.language),
            activity_type,
            category,
            description,
            excerpt: None,
            duration_secs,
            page_count: None,
            score: 0,
        });
        if out.len() >= max_results {
            break;
        }
    }
    out
}

fn youtube_id(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let id = if let Some(rest) = raw.split("v=").nth(1) {
        rest.split(['&', '#']).next()
    } else if let Some(rest) = raw.split("youtu.be/").nth(1) {
        rest.split(['?', '&', '#']).next()
    } else if let Some(rest) = raw.split("/embed/").nth(1) {
        rest.split(['?', '&', '#']).next()
    } else {
        None
    }?;
    let id = id.trim();
    if id.len() >= 6
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Some(id.to_string())
    } else {
        None
    }
}

fn parse_length(raw: &str) -> Option<u32> {
    let parts: Vec<&str> = raw.trim().split(':').collect();
    let nums: Vec<u32> = parts
        .iter()
        .map(|part| part.trim().parse::<u32>().ok())
        .collect::<Option<Vec<u32>>>()?;
    match nums.as_slice() {
        [seconds] => Some(*seconds),
        [minutes, seconds] => Some(minutes * 60 + seconds),
        [hours, minutes, seconds] => Some(hours * 3600 + minutes * 60 + seconds),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lengths() {
        assert_eq!(parse_length("18:49"), Some(1129));
        assert_eq!(parse_length("1:02:03"), Some(3723));
        assert_eq!(parse_length("bad"), None);
    }

    #[test]
    fn extracts_youtube_ids() {
        assert_eq!(
            youtube_id("https://www.youtube.com/watch?v=abc123").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            youtube_id("https://www.youtube-nocookie.com/embed/xyz789").as_deref(),
            Some("xyz789")
        );
        assert_eq!(
            youtube_id("https://youtu.be/abc123?t=4").as_deref(),
            Some("abc123")
        );
    }
}
