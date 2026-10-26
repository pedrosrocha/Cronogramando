use anyhow::Result;
use serde_json::Value;
use url::Url;

use super::{is_language_pt, language_tag};
use crate::content::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

const MAX_PAGES: u64 = 40;
const MAX_METADATA_LOOKUPS: usize = 2;

struct ArchiveDoc {
    identifier: String,
    title: String,
    language: String,
    page_count: Option<u32>,
}

pub async fn search(
    client: &reqwest::Client,
    query: &ConnectorQuery,
    category: SourceCategory,
) -> Result<Vec<ResourceCandidate>> {
    let language = language_clause(&query.language);
    let keywords = keyword_clause(&query.query);
    let term = format!(
        "title:({keywords}) AND mediatype:texts AND language:({language}) \
         AND imagecount:[1 TO {MAX_PAGES}]"
    );
    let rows = (query.limit.max(1) * 2).to_string();
    let url = Url::parse_with_params(
        "https://archive.org/advancedsearch.php",
        &[
            ("q", term.as_str()),
            ("fl[]", "identifier"),
            ("fl[]", "title"),
            ("fl[]", "mediatype"),
            ("fl[]", "language"),
            ("fl[]", "imagecount"),
            ("fl[]", "downloads"),
            ("rows", rows.as_str()),
            ("page", "1"),
            ("sort[]", "downloads desc"),
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

    let docs = parse_docs(&response, &query.language);
    let mut candidates = Vec::new();
    for doc in docs.into_iter().take(MAX_METADATA_LOOKUPS) {
        if let Some(url) = direct_pdf_url(client, &doc.identifier).await {
            candidates.push(ResourceCandidate {
                title: doc.title,
                url: Some(url),
                embed_id: None,
                connector: ConnectorKind::InternetArchive,
                language: doc.language,
                activity_type: "pdf".to_string(),
                category,
                description: None,
                excerpt: None,
                duration_secs: None,
                page_count: doc.page_count,
                score: 0,
            });
        }
    }
    Ok(candidates)
}

fn keyword_clause(query: &str) -> String {
    let keywords: Vec<&str> = query
        .split_whitespace()
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    if keywords.is_empty() {
        return query.trim().to_string();
    }
    keywords.join(" OR ")
}

fn language_clause(language: &str) -> &'static str {
    if is_language_pt(language) {
        "por OR Portuguese OR português OR pt"
    } else {
        "eng OR English OR en"
    }
}

fn parse_docs(response: &Value, requested_language: &str) -> Vec<ArchiveDoc> {
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
            if !language_matches(doc, requested_language) {
                return None;
            }
            let pages = doc.get("imagecount").and_then(as_u64)?;
            if pages == 0 || pages > MAX_PAGES {
                return None;
            }
            let title = first_string(doc.get("title")).unwrap_or_else(|| identifier.to_string());
            Some(ArchiveDoc {
                identifier: identifier.to_string(),
                title,
                language: language_tag(requested_language),
                page_count: Some(pages as u32),
            })
        })
        .collect()
}

fn language_matches(doc: &Value, requested_language: &str) -> bool {
    let values: Vec<&str> = match doc.get("language") {
        Some(Value::String(text)) => vec![text.as_str()],
        Some(Value::Array(values)) => values.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    if values.is_empty() {
        return false;
    }
    if is_language_pt(requested_language) {
        values.iter().any(|language| {
            let language = language.to_lowercase();
            is_language_pt(&language)
                || language == "por"
                || language == "portuguese"
                || language == "portugues"
        })
    } else {
        values.iter().any(|language| {
            let language = language.to_lowercase();
            language == "eng" || language == "english" || language == "en"
        })
    }
}

fn as_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.trim().parse().ok())
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

async fn direct_pdf_url(client: &reqwest::Client, identifier: &str) -> Option<String> {
    let mut metadata_url = Url::parse("https://archive.org/metadata/").ok()?;
    {
        let mut segments = metadata_url.path_segments_mut().ok()?;
        segments.pop_if_empty().push(identifier);
    }
    let response: Value = client
        .get(metadata_url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let files = response.get("files")?.as_array()?;
    let name = select_pdf_file(files)?;

    let mut url = Url::parse("https://archive.org/download/").ok()?;
    let mut segments = url.path_segments_mut().ok()?;
    segments.pop_if_empty().push(identifier).push(&name);
    drop(segments);
    Some(url.to_string())
}

fn select_pdf_file(files: &[Value]) -> Option<String> {
    let mut pdfs: Vec<(u8, u64, String)> = files
        .iter()
        .filter_map(|file| {
            let name = file.get("name").and_then(Value::as_str)?.trim();
            if !name.to_lowercase().ends_with(".pdf") {
                return None;
            }
            let format = file
                .get("format")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            let text_pdf =
                format.contains("text pdf") || name.to_lowercase().ends_with("_text.pdf");
            let priority = if text_pdf { 0 } else { 1 };
            let size = file.get("size").and_then(as_u64).unwrap_or(u64::MAX);
            Some((priority, size, name.to_string()))
        })
        .collect();

    pdfs.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    pdfs.into_iter().next().map(|(_, _, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_short_documents_in_the_requested_language() {
        let response: Value = serde_json::from_str(
            r#"{"response":{"docs":[
                {"identifier":"short-pt","title":"Redação do ENEM","language":["por"],"imagecount":24},
                {"identifier":"spanish","title":"La argumentación","language":["spa"],"imagecount":20},
                {"identifier":"long","title":"Redação completa","language":["Portuguese"],"imagecount":220},
                {"identifier":"no-pages","title":"Sem páginas","language":["por"]}
            ]}}"#,
        )
        .unwrap();
        let docs = parse_docs(&response, "pt-BR");
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].identifier, "short-pt");
        assert_eq!(docs[0].language, "pt-BR");
    }

    #[test]
    fn english_request_rejects_portuguese_documents() {
        let response: Value = serde_json::from_str(
            r#"{"response":{"docs":[
                {"identifier":"pt","title":"Redação","language":["por"],"imagecount":10},
                {"identifier":"en","title":"Essay writing","language":["eng"],"imagecount":10}
            ]}}"#,
        )
        .unwrap();
        let docs = parse_docs(&response, "en");
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].identifier, "en");
    }

    #[test]
    fn selects_a_text_pdf_over_a_scanned_pdf() {
        let files: Value = serde_json::from_str(
            r#"[
                {"name":"book.pdf","format":"Image Container PDF","size":"9000000"},
                {"name":"book_text.pdf","format":"Text PDF","size":"400000"}
            ]"#,
        )
        .unwrap();
        assert_eq!(
            select_pdf_file(files.as_array().unwrap()).as_deref(),
            Some("book_text.pdf")
        );
    }

    #[test]
    fn builds_fielded_language_queries() {
        assert!(language_clause("pt-BR").contains("por"));
        assert!(language_clause("en").contains("eng"));
        assert_eq!(keyword_clause("redacao enem"), "redacao OR enem");
    }
}
