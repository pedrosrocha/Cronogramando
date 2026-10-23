use anyhow::Result;

use super::types::{ConnectorKind, ConnectorQuery, ResourceCandidate, SourceCategory};

mod arxiv;
mod doaj;
mod gutenberg;
mod internet_archive;
mod openalex;
mod wikipedia;
mod youtube;

pub async fn search(
    client: &reqwest::Client,
    kind: ConnectorKind,
    query: &ConnectorQuery,
    category: SourceCategory,
    youtube_api_key: Option<&str>,
) -> Result<Vec<ResourceCandidate>> {
    match kind {
        ConnectorKind::Youtube => youtube::search(client, query, category, youtube_api_key).await,
        ConnectorKind::Wikipedia => wikipedia::search(client, query, category).await,
        ConnectorKind::InternetArchive => internet_archive::search(client, query, category).await,
        ConnectorKind::Arxiv => arxiv::search(client, query, category).await,
        ConnectorKind::OpenAlex => openalex::search(client, query, category).await,
        ConnectorKind::Doaj => doaj::search(client, query, category).await,
        ConnectorKind::Gutenberg => gutenberg::search(client, query, category).await,
        ConnectorKind::Curated | ConnectorKind::GenericTask => Ok(Vec::new()),
    }
}

pub fn is_language_pt(language: &str) -> bool {
    language.to_lowercase().starts_with("pt")
}

pub fn language_tag(language: &str) -> String {
    if is_language_pt(language) {
        "pt-BR".to_string()
    } else {
        "en".to_string()
    }
}

pub fn language_code(language: &str) -> &'static str {
    if is_language_pt(language) {
        "pt"
    } else {
        "en"
    }
}

pub(super) fn clean_text(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_helpers_follow_portuguese() {
        assert_eq!(language_tag("pt-BR"), "pt-BR");
        assert_eq!(language_tag("PT"), "pt-BR");
        assert_eq!(language_tag("en"), "en");
        assert_eq!(language_code("pt-BR"), "pt");
        assert_eq!(language_code("en"), "en");
    }

    #[test]
    fn clean_text_collapses_whitespace_and_entities() {
        assert_eq!(clean_text("  A &amp;\n B  "), "A & B");
    }
}
