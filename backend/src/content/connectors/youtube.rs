use anyhow::Result;
use serde::Deserialize;
use url::Url;

use super::{language_code, language_tag};
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    id: VideoId,
    snippet: Snippet,
}

#[derive(Debug, Deserialize)]
struct VideoId {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Snippet {
    title: String,
    #[serde(rename = "defaultAudioLanguage")]
    default_audio_language: Option<String>,
}

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
    api_key: Option<&str>,
) -> Result<Vec<ResourceCandidate>> {
    let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) else {
        return Ok(Vec::new());
    };

    let lang = language_code(&query.language);
    let region = if lang == "pt" { "BR" } else { "US" };
    let limit = query.limit.to_string();
    let url = Url::parse_with_params(
        "https://www.googleapis.com/youtube/v3/search",
        &[
            ("part", "snippet"),
            ("type", "video"),
            ("maxResults", limit.as_str()),
            ("q", query.query.as_str()),
            ("relevanceLanguage", lang),
            ("regionCode", region),
            ("key", api_key),
        ],
    )?;

    let response: SearchResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_response(&response, &query.language, category))
}

fn parse_response(
    response: &SearchResponse,
    requested_language: &str,
    category: SourceCategory,
) -> Vec<ResourceCandidate> {
    response
        .items
        .iter()
        .filter_map(|item| {
            let video_id = item.id.video_id.as_deref()?.trim();
            if video_id.is_empty() {
                return None;
            }
            let language = item
                .snippet
                .default_audio_language
                .as_deref()
                .map(language_tag)
                .unwrap_or_else(|| language_tag(requested_language));
            Some(ResourceCandidate {
                title: item.snippet.title.trim().to_string(),
                url: Some(format!("https://www.youtube.com/watch?v={video_id}")),
                embed_id: Some(video_id.to_string()),
                connector: ConnectorKind::Youtube,
                language,
                activity_type: "video".to_string(),
                category,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_video_results() {
        let response: SearchResponse = serde_json::from_str(
            r#"{"items":[{"id":{"videoId":"abc123"},"snippet":{"title":"Cálculo 1","defaultAudioLanguage":"pt-BR"}}]}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, "pt-BR", SourceCategory::Stem);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].embed_id.as_deref(), Some("abc123"));
        assert_eq!(candidates[0].language, "pt-BR");
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("https://www.youtube.com/watch?v=abc123")
        );
    }

    #[test]
    fn tags_missing_audio_language_with_query_language() {
        let response: SearchResponse = serde_json::from_str(
            r#"{"items":[{"id":{"videoId":"xyz"},"snippet":{"title":"Art history"}}]}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, "en", SourceCategory::Arts);
        assert_eq!(candidates[0].language, "en");
    }
}
