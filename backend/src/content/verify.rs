use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;

use super::brief::ContentBrief;
use super::query;
use super::types::ResourceCandidate;
use crate::config::ContentConfig;

const MAX_HTML_BYTES: usize = 2 * 1024 * 1024;

pub struct Verifier {
    client: reqwest::Client,
    pdf_service_url: String,
    pdf_timeout_secs: u64,
    cfg: ContentConfig,
}

#[derive(Debug)]
pub struct Verification {
    pub candidate: ResourceCandidate,
    pub accepted: bool,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
struct ProbeResponse {
    page_count: u32,
    text_chars: u32,
    #[serde(default)]
    has_text_layer: bool,
    #[serde(default)]
    excerpt: String,
}

impl Verifier {
    pub fn new(
        client: reqwest::Client,
        pdf_service_url: String,
        pdf_timeout_secs: u64,
        cfg: ContentConfig,
    ) -> Self {
        Self {
            client,
            pdf_service_url,
            pdf_timeout_secs,
            cfg,
        }
    }

    pub async fn verify(&self, candidate: ResourceCandidate, brief: &ContentBrief) -> Verification {
        match candidate.activity_type.as_str() {
            "video" => self.verify_video(candidate, brief),
            "pdf" => self.verify_pdf(candidate, brief).await,
            _ => self.verify_article(candidate, brief).await,
        }
    }

    fn verify_video(&self, mut candidate: ResourceCandidate, brief: &ContentBrief) -> Verification {
        let max_duration = self
            .cfg
            .max_video_secs
            .min(brief.max_minutes.saturating_mul(60));
        if let Some(duration) = candidate.duration_secs {
            if duration < self.cfg.min_video_secs || duration > max_duration {
                return reject(candidate, "video duration outside the step budget");
            }
        }

        let text = format!(
            "{} {}",
            candidate.title,
            candidate.description.clone().unwrap_or_default()
        );
        let (must_hits, hits) = term_hits(brief, &text);
        if must_hits == 0 {
            return reject(
                candidate,
                "video title/description misses the required terms",
            );
        }
        if hits == 0 {
            return reject(candidate, "video has no topical overlap");
        }
        if has_excluded_term(brief, &text) {
            return reject(candidate, "video matches an excluded term");
        }

        candidate.excerpt = candidate
            .description
            .as_deref()
            .map(|text| truncate(text, 1200));
        candidate.score = 50 + hits.min(8) as i32 * 5 + must_hits.min(4) as i32 * 5;
        Verification {
            candidate,
            accepted: true,
            reason: "video metadata passed".to_string(),
        }
    }

    async fn verify_article(
        &self,
        mut candidate: ResourceCandidate,
        brief: &ContentBrief,
    ) -> Verification {
        let Some(url) = candidate.url.clone() else {
            return reject(candidate, "missing url");
        };
        if url.to_lowercase().contains(".pdf") {
            return self.verify_pdf(candidate, brief).await;
        }

        let response = match self
            .client
            .get(&url)
            .timeout(Duration::from_secs(8))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => return reject(candidate, &format!("fetch failed: {error}")),
        };
        if !response.status().is_success() {
            return reject(candidate, &format!("http {}", response.status()));
        }
        if response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_lowercase().contains("pdf"))
            .unwrap_or(false)
        {
            return self.verify_pdf(candidate, brief).await;
        }

        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => return reject(candidate, &format!("body read failed: {error}")),
        };
        if bytes.len() > MAX_HTML_BYTES {
            return reject(candidate, "page is too large to verify");
        }
        let html = String::from_utf8_lossy(&bytes);
        let text = extract_text(&html);
        let words = text.split_whitespace().count();
        if words < self.cfg.min_article_words {
            return reject(candidate, &format!("article too short ({words} words)"));
        }
        if words > self.cfg.max_article_words {
            return reject(candidate, &format!("article too long ({words} words)"));
        }
        if looks_wrong_language(&text, &brief.language) {
            return reject(candidate, "body language does not match the request");
        }
        if has_excluded_term(brief, &text) {
            return reject(candidate, "body matches an excluded term");
        }

        let (must_hits, hits) = term_hits(brief, &text);
        if must_hits == 0 {
            return reject(candidate, "body misses the required terms");
        }
        if hits < 2 {
            return reject(candidate, "body overlap is too weak");
        }

        candidate.excerpt = Some(truncate(&text, 1500));
        candidate.score = 50 + hits.min(8) as i32 * 5 + must_hits.min(4) as i32 * 5;
        Verification {
            candidate,
            accepted: true,
            reason: format!("article body passed ({words} words)"),
        }
    }

    async fn verify_pdf(
        &self,
        mut candidate: ResourceCandidate,
        brief: &ContentBrief,
    ) -> Verification {
        if let Some(pages) = candidate.page_count {
            if pages > self.cfg.max_pdf_pages {
                return reject(
                    candidate,
                    &format!("pdf has {pages} pages (limit {})", self.cfg.max_pdf_pages),
                );
            }
        }
        let Some(url) = candidate.url.clone() else {
            return reject(candidate, "missing url");
        };

        let response = match self
            .client
            .get(&url)
            .timeout(Duration::from_secs(20))
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => return reject(candidate, &format!("pdf fetch failed: {error}")),
        };
        if !response.status().is_success() {
            return reject(candidate, &format!("pdf http {}", response.status()));
        }
        if let Some(length) = response.content_length() {
            if length as usize > self.cfg.max_pdf_bytes {
                return reject(candidate, "pdf exceeds the size limit");
            }
        }
        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(error) => return reject(candidate, &format!("pdf read failed: {error}")),
        };
        if bytes.len() > self.cfg.max_pdf_bytes {
            return reject(candidate, "pdf exceeds the size limit");
        }
        if !bytes.starts_with(b"%PDF") {
            return reject(candidate, "resource is not a PDF");
        }

        let part = reqwest::multipart::Part::bytes(bytes.to_vec())
            .file_name("resource.pdf")
            .mime_str("application/pdf")
            .expect("valid pdf mime");
        let form = reqwest::multipart::Form::new()
            .text("max_pages", "2")
            .text("connector", "probe")
            .part("file", part);
        let probe = self
            .client
            .post(format!(
                "{}/probe",
                self.pdf_service_url.trim_end_matches('/')
            ))
            .multipart(form)
            .timeout(Duration::from_secs(self.pdf_timeout_secs.max(5)))
            .send()
            .await;
        let probe: ProbeResponse = match probe {
            Ok(response) if response.status().is_success() => match response.json().await {
                Ok(value) => value,
                Err(error) => {
                    return reject(candidate, &format!("pdf probe decode failed: {error}"))
                }
            },
            Ok(response) => {
                return reject(candidate, &format!("pdf probe http {}", response.status()))
            }
            Err(error) => return reject(candidate, &format!("pdf probe failed: {error}")),
        };

        if probe.page_count > self.cfg.max_pdf_pages {
            return reject(
                candidate,
                &format!(
                    "pdf has {} pages (limit {})",
                    probe.page_count, self.cfg.max_pdf_pages
                ),
            );
        }
        if !probe.has_text_layer || probe.text_chars < 100 {
            return reject(candidate, "pdf has no usable text layer");
        }

        let text = format!("{} {}", candidate.title, probe.excerpt);
        if has_excluded_term(brief, &text) {
            return reject(candidate, "pdf matches an excluded term");
        }
        let (must_hits, hits) = term_hits(brief, &text);
        if must_hits == 0 {
            return reject(candidate, "pdf misses the required terms");
        }

        candidate.page_count = Some(probe.page_count);
        candidate.excerpt = Some(truncate(&probe.excerpt, 1200));
        candidate.score = 50 + hits.min(8) as i32 * 5 + must_hits.min(4) as i32 * 5;
        Verification {
            candidate,
            accepted: true,
            reason: format!(
                "pdf passed ({} pages, {} text chars)",
                probe.page_count, probe.text_chars
            ),
        }
    }
}

fn reject(mut candidate: ResourceCandidate, reason: &str) -> Verification {
    candidate.score = 0;
    Verification {
        candidate,
        accepted: false,
        reason: reason.to_string(),
    }
}

fn term_hits(brief: &ContentBrief, text: &str) -> (usize, usize) {
    let tokens = query::tokens(text);
    let mut must_hits = 0;
    for term in &brief.must_terms {
        if tokens.iter().any(|token| token_matches(term, token)) {
            must_hits += 1;
        }
    }
    let mut hits = 0;
    for term in brief.topic_terms.iter().chain(brief.step_terms.iter()) {
        if tokens.iter().any(|token| token_matches(term, token)) {
            hits += 1;
        }
    }
    (must_hits, hits)
}

fn token_matches(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if left.len() < 4 || right.len() < 4 {
        return false;
    }
    left.starts_with(right) || right.starts_with(left)
}

fn has_excluded_term(brief: &ContentBrief, text: &str) -> bool {
    let tokens = query::tokens(text);
    brief
        .exclude_terms
        .iter()
        .any(|term| tokens.iter().any(|token| token_matches(term, token)))
}

fn looks_wrong_language(text: &str, requested: &str) -> bool {
    if !requested.to_lowercase().starts_with("pt") {
        return false;
    }
    const PT: [&str; 18] = [
        "que", "para", "com", "uma", "dos", "das", "por", "mais", "como", "mas", "seu", "sua",
        "sobre", "também", "esta", "este", "são", "pela",
    ];
    const EN: [&str; 18] = [
        "the", "and", "with", "that", "this", "from", "your", "have", "will", "are", "was", "not",
        "you", "for", "can", "how", "what", "about",
    ];
    let mut pt = 0;
    let mut en = 0;
    for token in text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        if PT.contains(&token) {
            pt += 1;
        }
        if EN.contains(&token) {
            en += 1;
        }
    }
    en > 8 && pt < 3
}

fn extract_text(html: &str) -> String {
    static STRIP: OnceLock<Regex> = OnceLock::new();
    static TAGS: OnceLock<Regex> = OnceLock::new();
    let strip = STRIP.get_or_init(|| {
        Regex::new(
            r"(?is)<(script|style|nav|footer|header|noscript|svg|form)[^>]*>.*?</(script|style|nav|footer|header|noscript|svg|form)>",
        )
        .expect("valid strip regex")
    });
    let tags = TAGS.get_or_init(|| Regex::new(r"(?s)<[^>]+>").expect("valid tag regex"));
    let without_blocks = strip.replace_all(html, " ");
    let without_tags = tags.replace_all(&without_blocks, " ");
    let decoded = without_tags
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_scripts_and_tags() {
        let html = "<html><script>x()</script><body><h1>Redação ENEM</h1><p>Texto &amp; mais</p></body></html>";
        let text = extract_text(html);
        assert_eq!(text, "Redação ENEM Texto & mais");
    }

    #[test]
    fn detects_english_body_for_portuguese_request() {
        let english = "The essay is about the problem and the solution. You can write with clarity and the reader will understand what this is about.";
        assert!(looks_wrong_language(english, "pt-BR"));
        let portuguese = "A redação é sobre o problema e a solução. Você pode escrever com clareza para que o leitor entenda o que está sendo dito.";
        assert!(!looks_wrong_language(portuguese, "pt-BR"));
    }
}
