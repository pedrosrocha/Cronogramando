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
    let limit = query.limit.to_string();
    let url = Url::parse_with_params(
        "https://archive.org/advancedsearch.php",
        &[
            ("q", query.query.as_str()),
            ("fl[]", "identifier"),
            ("fl[]", "title"),
            ("fl[]", "mediatype"),
            ("fl[]", "language"),
            ("rows", limit.as_str()),
            ("page", "1"),
            ("output", "json"),
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
    let Some(docs) = response
        .get("response")
        .and_then(|value| value.get("docs"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    docs.iter()
        .filter_map(|doc| {
            let identifier = doc.get("identifier").and_then(Value::as_str)?.trim();
            if identifier.is_empty() {
                return None;
            }
            let title = first_string(doc.get("title")).unwrap_or_else(|| identifier.to_string());
            let mediatype = doc
                .get("mediatype")
                .and_then(Value::as_str)
                .unwrap_or("texts");
            let language = language_from_doc(doc);

            Some(ResourceCandidate {
                title,
                url: Some(details_url(identifier)),
                embed_id: None,
                connector: ConnectorKind::InternetArchive,
                language,
                activity_type: activity_type(mediatype).to_string(),
                category,
            })
        })
        .collect()
}

fn first_string(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
        Value::Array(values) => values
            .iter()
            .find_map(Value::as_str)
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty()),
        _ => None,
    }
}

fn language_from_doc(doc: &Value) -> String {
    let raw = match doc.get("language") {
        Some(Value::String(text)) => vec![text.as_str()],
        Some(Value::Array(values)) => values.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };

    let is_pt = raw
        .iter()
        .any(|language| is_language_pt(language) || language.eq_ignore_ascii_case("por"));
    language_tag(if is_pt { "pt" } else { "en" })
}

fn activity_type(mediatype: &str) -> &'static str {
    match mediatype {
        "movies" | "video" => "video",
        "audio" => "podcast",
        _ => "article",
    }
}

fn details_url(identifier: &str) -> String {
    let mut url = Url::parse("https://archive.org/details/").expect("valid archive base url");
    url.path_segments_mut()
        .expect("archive url supports path segments")
        .pop_if_empty()
        .push(identifier);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_docs_and_maps_media_types() {
        let response: Value = serde_json::from_str(
            r#"{"response":{"docs":[
                {"identifier":"crafts-pt","title":"Artesanato","mediatype":"texts","language":["por"]},
                {"identifier":"lecture","title":"Physics lecture","mediatype":"movies","language":"eng"}
            ]}}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, SourceCategory::Manual);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].language, "pt-BR");
        assert_eq!(candidates[0].activity_type, "article");
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("https://archive.org/details/crafts-pt")
        );
        assert_eq!(candidates[1].activity_type, "video");
        assert_eq!(candidates[1].language, "en");
    }

    #[test]
    fn ignores_docs_without_identifier() {
        let response: Value =
            serde_json::from_str(r#"{"response":{"docs":[{"title":"No id"}]}}"#).unwrap();
        assert!(parse_response(&response, SourceCategory::Arts).is_empty());
    }
}
