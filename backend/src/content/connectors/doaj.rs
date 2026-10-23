use anyhow::Result;
use serde_json::Value;
use url::Url;

use super::{is_language_pt, language_tag};
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
) -> Result<Vec<ResourceCandidate>> {
    let mut url = Url::parse("https://doaj.org/api/search/articles/").expect("valid doaj base url");
    {
        let mut segments = url
            .path_segments_mut()
            .expect("doaj url supports path segments");
        segments.pop_if_empty();
        segments.push(query.query.trim());
    }
    url.query_pairs_mut()
        .append_pair("pageSize", &query.limit.to_string());

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
        .filter_map(|result| {
            let bibjson = result.get("bibjson")?;
            let title = bibjson
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|title| !title.is_empty())?;
            let url = bibjson
                .get("link")
                .and_then(Value::as_array)
                .and_then(|links| links.first())
                .and_then(|link| link.get("url"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let language = language_from_bibjson(bibjson);

            Some(ResourceCandidate {
                title: title.to_string(),
                url,
                embed_id: None,
                connector: ConnectorKind::Doaj,
                language,
                activity_type: "article".to_string(),
                category,
            })
        })
        .collect()
}

fn language_from_bibjson(bibjson: &Value) -> String {
    let raw = match bibjson.get("language") {
        Some(Value::String(text)) => vec![text.as_str()],
        Some(Value::Array(values)) => values.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    let is_pt = raw.iter().any(|language| is_language_pt(language));
    language_tag(if is_pt { "pt" } else { "en" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_articles() {
        let response: Value = serde_json::from_str(
            r#"{"results":[
                {"bibjson":{"title":"Open access calculus","language":["EN"],
                 "link":[{"url":"https://example.org/article"}]}},
                {"bibjson":{"title":"Educação e saúde","language":["PT"]}}
            ]}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, SourceCategory::Health);
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("https://example.org/article")
        );
        assert_eq!(candidates[1].language, "pt-BR");
    }

    #[test]
    fn ignores_articles_without_title() {
        let response: Value =
            serde_json::from_str(r#"{"results":[{"bibjson":{"language":["EN"]}}]}"#).unwrap();
        assert!(parse_response(&response, SourceCategory::Stem).is_empty());
    }
}
