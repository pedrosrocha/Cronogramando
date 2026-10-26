use std::cmp::Ordering;

use super::{
    classifier::normalize,
    query,
    types::{ConnectorKind, ResourceCandidate},
};

const MIN_TOKEN_MATCH_LEN: usize = 4;

fn trust(kind: ConnectorKind) -> i32 {
    match kind {
        ConnectorKind::Youtube => 10,
        ConnectorKind::Wikipedia => 9,
        ConnectorKind::OpenAlex => 7,
        ConnectorKind::Doaj => 6,
        ConnectorKind::Arxiv => 6,
        ConnectorKind::Searxng => 5,
        ConnectorKind::Gutenberg => 3,
        ConnectorKind::InternetArchive => 2,
        ConnectorKind::Curated => 1,
        ConnectorKind::GenericTask => 0,
    }
}

fn domain_score(candidate: &ResourceCandidate) -> i32 {
    let host = candidate
        .url
        .as_deref()
        .and_then(|url| url::Url::parse(url).ok())
        .and_then(|url| url.host_str().map(str::to_lowercase))
        .unwrap_or_default();
    if host.is_empty() {
        return 0;
    }
    const TRUSTED: [&str; 12] = [
        "gov.br",
        "todamateria.com.br",
        "brasilescola.uol.com.br",
        "mundoeducacao.uol.com.br",
        "descomplica.com.br",
        "khanacademy.org",
        "wikipedia.org",
        "youtube.com",
        "openstax.org",
        "educacao.uol.com.br",
        "infoescola.com",
        "mec.gov.br",
    ];
    const DENIED: [&str; 8] = [
        "dicio.com.br",
        "priberam.org",
        "significados.com.br",
        "conjugacao.com.br",
        "infopedia.pt",
        "assinei.digital",
        "dicionarioinformal.com.br",
        "sinonimos.com.br",
    ];
    if TRUSTED
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
    {
        return 15;
    }
    if DENIED
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
    {
        return -30;
    }
    0
}

fn token_matches(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if left.len() < MIN_TOKEN_MATCH_LEN || right.len() < MIN_TOKEN_MATCH_LEN {
        return false;
    }
    left.starts_with(right) || right.starts_with(left)
}

fn relevance(query_tokens: &[String], title: &str) -> (usize, i32) {
    if query_tokens.is_empty() {
        return (0, 0);
    }
    let title_tokens = query::tokens(title);
    let mut count = 0;
    let mut weighted = 0;
    for (index, query_token) in query_tokens.iter().enumerate() {
        if title_tokens
            .iter()
            .any(|title_token| token_matches(query_token, title_token))
        {
            count += 1;
            weighted += (query_tokens.len() - index).max(1) as i32;
        }
    }
    (count, weighted)
}

fn is_portuguese(language: &str) -> bool {
    language.to_lowercase().starts_with("pt")
}

fn is_english(language: &str) -> bool {
    let lower = language.to_lowercase();
    lower == "en" || lower.starts_with("eng")
}

fn normalized_contains(haystack: &str, needle: &str) -> bool {
    !needle.is_empty() && normalize(haystack).contains(&normalize(needle))
}

fn score(
    candidate: &ResourceCandidate,
    query_tokens: &[String],
    query_text: &str,
    exact_language: bool,
) -> i32 {
    let (_, weighted) = relevance(query_tokens, &candidate.title);
    let base = if candidate.score > 0 {
        candidate.score
    } else if exact_language {
        100
    } else {
        40
    };
    let mut score = base;
    score += weighted * 4;
    score += trust(candidate.connector);
    score += domain_score(candidate);
    if normalized_contains(&candidate.title, query_text) {
        score += 20;
    }
    score
}

fn compare(left: &(i32, ResourceCandidate), right: &(i32, ResourceCandidate)) -> Ordering {
    right
        .0
        .cmp(&left.0)
        .then_with(|| trust(right.1.connector).cmp(&trust(left.1.connector)))
        .then_with(|| left.1.title.cmp(&right.1.title))
}

pub fn rank(
    candidates: Vec<ResourceCandidate>,
    query_text: &str,
    language: &str,
) -> Vec<ResourceCandidate> {
    let query_tokens = query::tokens(query_text);
    let wants_portuguese = is_portuguese(language);

    let mut primary: Vec<(i32, ResourceCandidate)> = Vec::new();
    let mut secondary: Vec<(i32, ResourceCandidate)> = Vec::new();

    for candidate in candidates {
        if matches!(
            candidate.connector,
            ConnectorKind::Curated | ConnectorKind::GenericTask
        ) {
            primary.push((
                score(&candidate, &query_tokens, query_text, true),
                candidate,
            ));
            continue;
        }

        let candidate_portuguese = is_portuguese(&candidate.language);
        let candidate_english = is_english(&candidate.language);
        if !candidate_portuguese && !candidate_english {
            continue;
        }

        let exact = if wants_portuguese {
            candidate_portuguese
        } else {
            candidate_english
        };
        if !wants_portuguese && !candidate_english {
            continue;
        }

        let (overlap, _) = relevance(&query_tokens, &candidate.title);
        if overlap == 0 {
            continue;
        }

        let entry = (
            score(&candidate, &query_tokens, query_text, exact),
            candidate,
        );
        if exact {
            primary.push(entry);
        } else {
            secondary.push(entry);
        }
    }

    primary.sort_by(compare);
    secondary.sort_by(compare);

    let ordered = if primary.is_empty() {
        secondary
    } else {
        primary
    };
    ordered
        .into_iter()
        .map(|(_, candidate)| candidate)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::types::SourceCategory;

    fn candidate(title: &str, connector: ConnectorKind, language: &str) -> ResourceCandidate {
        ResourceCandidate {
            title: title.to_string(),
            url: Some(format!("https://example.org/{title}")),
            embed_id: None,
            connector,
            language: language.to_string(),
            activity_type: "article".to_string(),
            category: SourceCategory::Humanities,
            description: None,
            excerpt: None,
            duration_secs: None,
            page_count: None,
            score: 0,
        }
    }

    #[test]
    fn rejects_off_topic_top_hits() {
        let ranked = rank(
            vec![
                candidate("Anatocismo", ConnectorKind::Wikipedia, "pt-BR"),
                candidate(
                    "Rubrica de correção do ENEM",
                    ConnectorKind::Wikipedia,
                    "pt-BR",
                ),
            ],
            "rubrica correcao redacao enem",
            "pt-BR",
        );
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].title, "Rubrica de correção do ENEM");
    }

    #[test]
    fn prefers_portuguese_over_english() {
        let ranked = rank(
            vec![
                candidate("Redação do ENEM", ConnectorKind::Wikipedia, "pt-BR"),
                candidate("ENEM essay writing", ConnectorKind::Youtube, "en"),
            ],
            "redacao enem",
            "pt-BR",
        );
        assert_eq!(ranked[0].language, "pt-BR");
    }

    #[test]
    fn falls_back_to_english_when_portuguese_misses() {
        let ranked = rank(
            vec![
                candidate("Argumentation strategies", ConnectorKind::OpenAlex, "en"),
                candidate("Anatocismo", ConnectorKind::Wikipedia, "pt-BR"),
            ],
            "argumentation redacao enem",
            "pt-BR",
        );
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].language, "en");
    }

    #[test]
    fn matches_plural_stems() {
        let ranked = rank(
            vec![candidate(
                "Tipos de texto argumentativo",
                ConnectorKind::Wikipedia,
                "pt-BR",
            )],
            "tipos textos argumentativos redacao enem",
            "pt-BR",
        );
        assert_eq!(ranked.len(), 1);
    }
}
