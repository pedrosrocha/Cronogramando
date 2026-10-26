use anyhow::Result;
use serde::Deserialize;
use url::Url;

use super::{language_code, language_tag};
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

const MIN_VIDEO_SECS: u32 = 45;
const MAX_VIDEO_SECS: u32 = 20 * 60;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    items: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
struct SearchItem {
    id: VideoId,
    snippet: SearchSnippet,
}

#[derive(Debug, Deserialize)]
struct VideoId {
    #[serde(rename = "videoId")]
    video_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchSnippet {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "defaultAudioLanguage")]
    default_audio_language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoListResponse {
    #[serde(default)]
    items: Vec<VideoItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoItem {
    id: String,
    snippet: VideoSnippet,
    content_details: Option<ContentDetails>,
    status: Option<VideoStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoSnippet {
    title: String,
    #[serde(default)]
    description: Option<String>,
    default_audio_language: Option<String>,
    default_language: Option<String>,
    live_broadcast_content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentDetails {
    duration: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoStatus {
    embeddable: Option<bool>,
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
            ("videoEmbeddable", "true"),
            ("safeSearch", "moderate"),
            ("order", "relevance"),
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

    let ids: Vec<String> = response
        .items
        .iter()
        .filter_map(|item| item.id.video_id.as_deref())
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    match fetch_details(client, &ids, api_key).await {
        Ok(details) if !details.is_empty() => Ok(parse_details(details, &query.language, category)),
        Ok(_) => Ok(parse_response(&response, &query.language, category)),
        Err(error) => {
            tracing::warn!(%error, "YouTube video details failed; using search snippets");
            Ok(parse_response(&response, &query.language, category))
        }
    }
}

async fn fetch_details(
    client: &reqwest::Client,
    ids: &[String],
    api_key: &str,
) -> Result<Vec<VideoItem>> {
    let url = Url::parse_with_params(
        "https://www.googleapis.com/youtube/v3/videos",
        &[
            ("part", "contentDetails,snippet,status"),
            ("id", &ids.join(",")),
            ("key", api_key),
        ],
    )?;
    let response: VideoListResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response.items)
}

fn parse_details(
    details: Vec<VideoItem>,
    requested_language: &str,
    category: SourceCategory,
) -> Vec<ResourceCandidate> {
    details
        .into_iter()
        .filter_map(|video| {
            let video_id = video.id.trim();
            if video_id.is_empty() {
                return None;
            }
            if video.status.as_ref().and_then(|status| status.embeddable) == Some(false) {
                return None;
            }
            if matches!(
                video.snippet.live_broadcast_content.as_deref(),
                Some("live") | Some("upcoming")
            ) {
                return None;
            }
            let duration_secs = video
                .content_details
                .as_ref()
                .and_then(|details| parse_duration(&details.duration));
            if let Some(duration) = duration_secs {
                if !(MIN_VIDEO_SECS..=MAX_VIDEO_SECS).contains(&duration) {
                    return None;
                }
            }
            let language = video
                .snippet
                .default_audio_language
                .as_deref()
                .or(video.snippet.default_language.as_deref())
                .map(language_tag)
                .unwrap_or_else(|| language_tag(requested_language));

            Some(ResourceCandidate {
                title: video.snippet.title.trim().to_string(),
                url: Some(format!("https://www.youtube.com/watch?v={video_id}")),
                embed_id: Some(video_id.to_string()),
                connector: ConnectorKind::Youtube,
                language,
                activity_type: "video".to_string(),
                category,
                description: video.snippet.description.clone(),
                excerpt: None,
                duration_secs,
                page_count: None,
                score: 0,
            })
        })
        .collect()
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
                description: item.snippet.description.clone(),
                excerpt: None,
                duration_secs: None,
                page_count: None,
                score: 0,
            })
        })
        .collect()
}

fn parse_duration(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    if !raw.starts_with('P') {
        return None;
    }

    let mut total = 0u32;
    let mut number = String::new();
    for character in raw.chars().skip(1) {
        if character.is_ascii_digit() {
            number.push(character);
            continue;
        }
        let value: u32 = if number.is_empty() {
            0
        } else {
            number.parse().ok()?
        };
        number.clear();
        match character {
            'W' => total += value * 7 * 24 * 60 * 60,
            'D' => total += value * 24 * 60 * 60,
            'H' => total += value * 60 * 60,
            'M' => total += value * 60,
            'S' => total += value,
            'T' => {}
            _ => return None,
        }
    }
    Some(total)
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

    #[test]
    fn parses_iso_durations() {
        assert_eq!(parse_duration("PT1H2M3S"), Some(3723));
        assert_eq!(parse_duration("PT15M"), Some(900));
        assert_eq!(parse_duration("PT45S"), Some(45));
        assert_eq!(parse_duration("P1DT2H"), Some(93600));
        assert_eq!(parse_duration("nonsense"), None);
    }

    #[test]
    fn filters_long_videos_and_uses_declared_language() {
        let details: VideoListResponse = serde_json::from_str(
            r#"{"items":[
                {"id":"short","snippet":{"title":"Redação do ENEM em 5 minutos","defaultAudioLanguage":"pt-BR"},
                 "contentDetails":{"duration":"PT5M10S"},"status":{"embeddable":true}},
                {"id":"long","snippet":{"title":"Redação do ENEM completa"},
                 "contentDetails":{"duration":"PT1H2M"},"status":{"embeddable":true}},
                {"id":"foreign","snippet":{"title":"Redação do ENEM","defaultAudioLanguage":"en-US"},
                 "contentDetails":{"duration":"PT6M"},"status":{"embeddable":true}}
            ]}"#,
        )
        .unwrap();
        let candidates = parse_details(details.items, "pt-BR", SourceCategory::Humanities);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].embed_id.as_deref(), Some("short"));
        assert_eq!(candidates[0].language, "pt-BR");
        assert_eq!(candidates[1].language, "en");
    }
}
