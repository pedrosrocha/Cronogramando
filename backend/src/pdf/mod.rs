use std::time::Duration;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use url::Url;

use crate::{config::Config, models::pdf::PdfBlock};

#[derive(Debug, Clone)]
pub struct ExtractRequest {
    pub resource_url: String,
    pub connector: String,
}

#[derive(Debug, Clone)]
pub struct Extraction {
    pub connector: String,
    pub engine: String,
    pub page_count: i32,
    pub blocks: Vec<PdfBlock>,
    pub markdown: String,
}

#[async_trait]
pub trait PdfExtractor: Send + Sync {
    async fn extract(&self, req: &ExtractRequest) -> Result<Extraction>;
}

#[derive(Debug, Deserialize)]
struct SidecarExtraction {
    connector: String,
    engine: String,
    page_count: i32,
    blocks: Vec<PdfBlock>,
    markdown: String,
}

pub struct HttpPdfExtractor {
    client: reqwest::Client,
    service_url: String,
    max_bytes: usize,
}

impl HttpPdfExtractor {
    pub fn new(cfg: &Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(cfg.pdf_service_timeout_secs))
            .user_agent(concat!(
                "cronogramando-backend/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/anomalyco/cronogramando)"
            ))
            .build()?;

        Ok(Self {
            client,
            service_url: cfg.pdf_service_url.trim_end_matches('/').to_string(),
            max_bytes: cfg.pdf_max_bytes,
        })
    }

    async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("could not download {url}"))?
            .error_for_status()
            .with_context(|| format!("resource returned an error: {url}"))?;

        if let Some(length) = response.content_length() {
            if length as usize > self.max_bytes {
                bail!(
                    "PDF is {} bytes, above the {} byte limit",
                    length,
                    self.max_bytes
                );
            }
        }

        let bytes = response
            .bytes()
            .await
            .with_context(|| format!("could not read {url}"))?;
        if bytes.len() > self.max_bytes {
            bail!(
                "PDF is {} bytes, above the {} byte limit",
                bytes.len(),
                self.max_bytes
            );
        }
        if !bytes.starts_with(b"%PDF") {
            bail!("resource is not a direct PDF (raw-PDF toggle stays available)");
        }
        Ok(bytes.to_vec())
    }
}

#[async_trait]
impl PdfExtractor for HttpPdfExtractor {
    async fn extract(&self, req: &ExtractRequest) -> Result<Extraction> {
        let url = normalize_source_url(&req.resource_url);
        let bytes = self.download(&url).await?;

        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name("document.pdf")
            .mime_str("application/pdf")?;
        let form = reqwest::multipart::Form::new()
            .text("connector", req.connector.clone())
            .part("file", part);

        let response = self
            .client
            .post(format!("{}/extract", self.service_url))
            .multipart(form)
            .send()
            .await
            .context("PDF sidecar is unreachable")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("PDF sidecar returned {status}: {}", truncate(&body, 300));
        }

        let extraction: SidecarExtraction = response
            .json()
            .await
            .context("PDF sidecar returned an invalid extraction payload")?;
        if !matches!(extraction.engine.as_str(), "normal" | "academic") {
            bail!("PDF sidecar returned unknown engine {}", extraction.engine);
        }
        if extraction.blocks.is_empty() && extraction.markdown.trim().is_empty() {
            bail!("PDF sidecar extracted no content");
        }

        Ok(Extraction {
            connector: extraction.connector,
            engine: extraction.engine,
            page_count: extraction.page_count,
            blocks: extraction.blocks,
            markdown: extraction.markdown,
        })
    }
}

/// arXiv search results point at the abstract page; the extractor needs the
/// PDF variant. Other connectors already return direct PDF URLs.
pub fn normalize_source_url(raw: &str) -> String {
    let Ok(mut url) = Url::parse(raw) else {
        return raw.to_string();
    };
    url.set_fragment(None);

    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    if host == "arxiv.org" || host.ends_with(".arxiv.org") {
        let _ = url.set_scheme("https");
        let path = url.path().to_string();
        if let Some(id) = path.strip_prefix("/abs/") {
            let id = id.trim_end_matches('/');
            if !id.is_empty() {
                url.set_query(None);
                url.set_path(&format!("/pdf/{}.pdf", id.trim_end_matches(".pdf")));
            }
        } else if let Some(id) = path.strip_prefix("/pdf/") {
            let id = id.trim_end_matches('/');
            if !id.is_empty() && !id.ends_with(".pdf") {
                url.set_path(&format!("/pdf/{id}.pdf"));
            }
        }
    }

    url.to_string()
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

pub fn build(cfg: &Config) -> Result<std::sync::Arc<dyn PdfExtractor>> {
    Ok(std::sync::Arc::new(HttpPdfExtractor::new(cfg)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_arxiv_abstract_urls_to_pdf() {
        assert_eq!(
            normalize_source_url("http://arxiv.org/abs/1234.5678v1"),
            "https://arxiv.org/pdf/1234.5678v1.pdf"
        );
        assert_eq!(
            normalize_source_url("https://arxiv.org/abs/1234.5678?context=cs"),
            "https://arxiv.org/pdf/1234.5678.pdf"
        );
    }

    #[test]
    fn completes_arxiv_pdf_urls() {
        assert_eq!(
            normalize_source_url("https://arxiv.org/pdf/1234.5678"),
            "https://arxiv.org/pdf/1234.5678.pdf"
        );
        assert_eq!(
            normalize_source_url("https://arxiv.org/pdf/1234.5678v2.pdf"),
            "https://arxiv.org/pdf/1234.5678v2.pdf"
        );
    }

    #[test]
    fn leaves_other_resources_untouched() {
        assert_eq!(
            normalize_source_url("https://assets.openstax.org/book.pdf#page=2"),
            "https://assets.openstax.org/book.pdf"
        );
        assert_eq!(normalize_source_url("not a url"), "not a url");
    }
}
