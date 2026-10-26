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
    let language = if is_language_pt(&query.language) {
        "pt"
    } else {
        "en"
    };
    let url = Url::parse_with_params(
        "https://gutendex.com/books",
        &[("search", query.query.trim()), ("languages", language)],
    )?;

    let response: Value = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(parse_response(&response, category, query.limit))
}

fn parse_response(
    response: &Value,
    category: SourceCategory,
    limit: usize,
) -> Vec<ResourceCandidate> {
    let Some(results) = response.get("results").and_then(Value::as_array) else {
        return Vec::new();
    };

    results
        .iter()
        .take(limit.max(1))
        .filter_map(|book| {
            let title = book
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|title| !title.is_empty())?;
            let formats = book.get("formats");
            let pdf = formats
                .and_then(|formats| formats.get("application/pdf"))
                .and_then(Value::as_str);
            let url = pdf
                .or_else(|| {
                    formats
                        .and_then(|formats| formats.get("text/html"))
                        .and_then(Value::as_str)
                })
                .map(str::to_string)
                .or_else(|| {
                    book.get("id")
                        .and_then(Value::as_i64)
                        .map(|id| format!("https://www.gutenberg.org/ebooks/{id}"))
                });
            let activity_type = if pdf.is_some() { "pdf" } else { "article" };
            let language = language_from_book(book);

            Some(ResourceCandidate {
                title: title.to_string(),
                url,
                embed_id: None,
                connector: ConnectorKind::Gutenberg,
                language,
                activity_type: activity_type.to_string(),
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

fn language_from_book(book: &Value) -> String {
    let raw = match book.get("languages") {
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
    fn parses_books_with_pdf_format() {
        let response: Value = serde_json::from_str(
            r#"{"results":[
                {"id":11,"title":"Alice","languages":["en"],
                 "formats":{"application/pdf":"https://gutenberg.org/11.pdf"}},
                {"id":22,"title":"Dom Casmurro","languages":["pt"],
                 "formats":{"text/html":"https://gutenberg.org/22.html"}}
            ]}"#,
        )
        .unwrap();
        let candidates = parse_response(&response, SourceCategory::Humanities, 5);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].activity_type, "pdf");
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("https://gutenberg.org/11.pdf")
        );
        assert_eq!(candidates[1].language, "pt-BR");
        assert_eq!(candidates[1].activity_type, "article");
    }

    #[test]
    fn falls_back_to_ebook_url() {
        let response: Value =
            serde_json::from_str(r#"{"results":[{"id":33,"title":"Poems","languages":["en"]}]}"#)
                .unwrap();
        let candidates = parse_response(&response, SourceCategory::Arts, 5);
        assert_eq!(
            candidates[0].url.as_deref(),
            Some("https://www.gutenberg.org/ebooks/33")
        );
    }
}
