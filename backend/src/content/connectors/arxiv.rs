use anyhow::Result;
use regex::Regex;
use url::Url;

use super::clean_text;
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
) -> Result<Vec<ResourceCandidate>> {
    let limit = query.limit.to_string();
    let terms: Vec<String> = query
        .query
        .split_whitespace()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| format!("all:{token}"))
        .collect();
    let search = if terms.is_empty() {
        format!("all:\"{}\"", query.query.trim())
    } else {
        terms.join(" AND ")
    };
    let url = Url::parse_with_params(
        "https://export.arxiv.org/api/query",
        &[
            ("search_query", search.as_str()),
            ("start", "0"),
            ("max_results", limit.as_str()),
        ],
    )?;

    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(parse_response(&body, category))
}

fn parse_response(body: &str, category: SourceCategory) -> Vec<ResourceCandidate> {
    let entry_re = Regex::new(r"(?s)<entry>(.*?)</entry>").expect("valid entry regex");
    let title_re = Regex::new(r"(?s)<title>(.*?)</title>").expect("valid title regex");
    let id_re = Regex::new(r"(?s)<id>(.*?)</id>").expect("valid id regex");
    let summary_re = Regex::new(r"(?s)<summary>(.*?)</summary>").expect("valid summary regex");

    entry_re
        .captures_iter(body)
        .filter_map(|entry| {
            let entry = entry.get(0)?.as_str();
            let title = title_re
                .captures(entry)
                .and_then(|capture| capture.get(1))
                .map(|value| clean_text(value.as_str()))?;
            let id = id_re
                .captures(entry)
                .and_then(|capture| capture.get(1))
                .map(|value| clean_text(value.as_str()))?;

            if title.is_empty() || id.is_empty() {
                return None;
            }

            let description = summary_re
                .captures(entry)
                .and_then(|capture| capture.get(1))
                .map(|value| clean_text(value.as_str()))
                .filter(|text| !text.is_empty());

            Some(ResourceCandidate {
                title,
                url: Some(id),
                embed_id: None,
                connector: ConnectorKind::Arxiv,
                language: "en".to_string(),
                activity_type: "pdf".to_string(),
                category,
                description,
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
    fn parses_atom_entries() {
        let body = r#"<?xml version="1.0"?>
            <feed>
              <entry>
                <id>http://arxiv.org/abs/1234.5678v1</id>
                <title>Calculus
                  and Series</title>
              </entry>
            </feed>"#;
        let candidates = parse_response(body, SourceCategory::Stem);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].title, "Calculus and Series");
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("http://arxiv.org/abs/1234.5678v1")
        );
        assert_eq!(candidates[0].language, "en");
    }

    #[test]
    fn ignores_malformed_entries() {
        let body = "<feed><entry><title>No id</title></entry></feed>";
        assert!(parse_response(body, SourceCategory::Stem).is_empty());
    }
}
