use anyhow::Result;
use serde::Deserialize;
use url::Url;

use super::{language_code, language_tag};
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

const MAX_WORDS: u64 = 3_500;

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
    #[serde(default)]
    wordcount: Option<u64>,
    #[serde(default)]
    snippet: Option<String>,
}

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
) -> Result<Vec<ResourceCandidate>> {
    let candidates = fetch(client, &query.query, query, category).await?;
    if !candidates.is_empty() {
        return Ok(candidates);
    }

    let terms: Vec<&str> = query.query.split_whitespace().collect();
    if terms.len() < 2 {
        return Ok(candidates);
    }
    let broad = terms.join(" OR ");
    fetch(client, &broad, query, category).await
}

async fn fetch(
    client: &reqwest::Client,
    search: &str,
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
            ("srsearch", search),
            ("srlimit", limit.as_str()),
            ("srprop", "snippet|wordcount"),
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
            if result.wordcount.is_some_and(|words| words > MAX_WORDS) {
                return None;
            }
            if is_disambiguation(title, result.snippet.as_deref()) {
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
                description: result.snippet.clone(),
                excerpt: None,
                duration_secs: None,
                page_count: None,
                score: 0,
            })
        })
        .collect()
}

fn is_disambiguation(title: &str, snippet: Option<&str>) -> bool {
    if title.to_lowercase().contains("(disambiguation)")
        || title.to_lowercase().contains("(desambiguação)")
    {
        return true;
    }
    let Some(snippet) = snippet else {
        return false;
    };
    let snippet = snippet.to_lowercase();
    snippet.contains("may refer to")
        || snippet.contains("pode referir-se a")
        || snippet.contains("pode se referir a")
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
            r#"{"query":{"search":[
                {"title":"Cálculo","wordcount":900,"snippet":"Cálculo é..."},
                {"title":"Integral","wordcount":1200}
            ]}}"#,
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
    fn drops_long_articles_and_disambiguation_pages() {
        let response: SearchResponse = serde_json::from_str(
            r#"{"query":{"search":[
                {"title":"Livro enorme","wordcount":90000},
                {"title":"Redação (desambiguação)","wordcount":100},
                {"title":"Redação","wordcount":1200,"snippet":"Redação pode referir-se a..."},
                {"title":"Redação do ENEM","wordcount":1100,"snippet":"A redação do ENEM..."}
            ]}}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, "pt", SourceCategory::Humanities);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "Redação do ENEM");
    }

    #[test]
    fn empty_response_is_fine() {
        let response: SearchResponse = serde_json::from_str("{}").unwrap();
        assert!(parse_response(&response, "en", SourceCategory::Arts).is_empty());
    }
}
