use anyhow::Result;
use serde::Deserialize;
use url::Url;

use super::{language_code, language_tag};
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

#[derive(Debug, Deserialize)]
struct SearchResponse {
    query: Option<SearchQuery>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    search: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    title: String,
}

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
) -> Result<Vec<ResourceCandidate>> {
    let lang = language_code(&query.language);
    let limit = query.limit.to_string();
    let base = format!("https://{lang}.wikipedia.org/w/api.php");
    let url = Url::parse_with_params(
        &base,
        &[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", query.query.as_str()),
            ("srlimit", limit.as_str()),
            ("format", "json"),
            ("utf8", "1"),
        ],
    )?;

    let response: SearchResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_response(&response, lang, category))
}

fn parse_response(
    response: &SearchResponse,
    lang: &str,
    category: SourceCategory,
) -> Vec<ResourceCandidate> {
    response
        .query
        .as_ref()
        .map(|query| query.search.as_slice())
        .unwrap_or_default()
        .iter()
        .filter_map(|result| {
            let title = result.title.trim();
            if title.is_empty() {
                return None;
            }
            Some(ResourceCandidate {
                title: title.to_string(),
                url: Some(article_url(lang, title)),
                embed_id: None,
                connector: ConnectorKind::Wikipedia,
                language: language_tag(lang),
                activity_type: "article".to_string(),
                category,
            })
        })
        .collect()
}

fn article_url(lang: &str, title: &str) -> String {
    let base = format!("https://{lang}.wikipedia.org/wiki/");
    let mut url = Url::parse(&base).expect("valid wikipedia base url");
    url.path_segments_mut()
        .expect("wikipedia url supports path segments")
        .pop_if_empty()
        .push(&title.replace(' ', "_"));
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_results() {
        let response: SearchResponse = serde_json::from_str(
            r#"{"query":{"search":[{"title":"Cálculo"},{"title":"Integral"}]}}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, "pt", SourceCategory::Stem);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].language, "pt-BR");
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("https://pt.wikipedia.org/wiki/C%C3%A1lculo")
        );
    }

    #[test]
    fn empty_response_is_fine() {
        let response: SearchResponse = serde_json::from_str("{}").unwrap();
        assert!(parse_response(&response, "en", SourceCategory::Arts).is_empty());
    }
}
