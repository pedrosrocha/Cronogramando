use anyhow::Result;
use serde_json::Value;
use url::Url;

use super::language_tag;
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
) -> Result<Vec<ResourceCandidate>> {
    let limit = query.limit.to_string();
    let url = Url::parse_with_params(
        "https://api.openalex.org/works",
        &[
            ("search", query.query.as_str()),
            ("per-page", limit.as_str()),
        ],
    )?;

    let response: Value = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_response(&response, category))
}

fn parse_response(response: &Value, category: SourceCategory) -> Vec<ResourceCandidate> {
    let Some(results) = response.get("results").and_then(Value::as_array) else {
        return Vec::new();
    };

    results
        .iter()
        .filter_map(|work| {
            let title = work
                .get("display_name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|title| !title.is_empty())?;
            let url = work
                .get("primary_location")
                .and_then(|location| location.get("landing_page_url"))
                .and_then(Value::as_str)
                .or_else(|| {
                    work.get("best_oa_location")
                        .and_then(|location| location.get("landing_page_url"))
                        .and_then(Value::as_str)
                })
                .or_else(|| work.get("id").and_then(Value::as_str))
                .map(str::to_string);
            let language = work
                .get("language")
                .and_then(Value::as_str)
                .map(language_tag)
                .unwrap_or_else(|| "en".to_string());

            Some(ResourceCandidate {
                title: title.to_string(),
                url,
                embed_id: None,
                connector: ConnectorKind::OpenAlex,
                language,
                activity_type: "article".to_string(),
                category,
                description: None,
                excerpt: None,
                duration_secs: None,
                page_count: None,
                score: 0,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_works() {
        let response: Value = serde_json::from_str(
            r#"{"results":[
                {"id":"https://openalex.org/W1","display_name":"Learning calculus","language":"en",
                 "primary_location":{"landing_page_url":"https://example.org/paper"}},
                {"id":"https://openalex.org/W2","display_name":"Educação financeira","language":"pt"}
            ]}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, SourceCategory::Business);
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("https://example.org/paper")
        );
        assert_eq!(
            candidates[1].url.as_deref(),
            Some("https://openalex.org/W2")
        );
        assert_eq!(candidates[1].language, "pt-BR");
    }

    #[test]
    fn ignores_works_without_title() {
        let response: Value = serde_json::from_str(r#"{"results":[{"id":"x"}]}"#).unwrap();
        assert!(parse_response(&response, SourceCategory::Stem).is_empty());
    }
}
