use super::brief::ContentBrief;
use super::classifier::normalize;
use super::types::ConnectorKind;

const MAX_STEP_TOKENS: usize = 5;
const MAX_QUERY_TOKENS: usize = 8;
const MIN_TOKEN_LEN: usize = 2;

const PREFIXES: [&str; 8] = [
    "quick win:",
    "study:",
    "practice:",
    "spaced review of earlier material in",
    "vitoria rapida:",
    "estudo:",
    "pratica:",
    "revisao espacada de material anterior em",
];

const STOPWORDS: [&str; 76] = [
    "a", "o", "os", "as", "um", "uma", "uns", "umas", "de", "da", "do", "das", "dos", "e", "ou",
    "para", "por", "com", "sem", "em", "no", "na", "nos", "nas", "que", "se", "ao", "aos", "mais",
    "menos", "seu", "sua", "seus", "suas", "este", "esta", "isso", "isto", "sobre", "entre", "ate",
    "apos", "antes", "durante", "entao", "como", "the", "an", "of", "to", "and", "or", "for",
    "with", "without", "in", "on", "at", "from", "by", "is", "are", "be", "this", "that", "these",
    "those", "your", "you", "how", "what", "why", "when", "while", "into", "using",
];

const INSTRUCTION_WORDS: [&str; 48] = [
    "entenda",
    "aprenda",
    "estude",
    "pratique",
    "revise",
    "complete",
    "crie",
    "monte",
    "aplique",
    "analise",
    "compare",
    "identifique",
    "descreva",
    "explique",
    "escolha",
    "defina",
    "organize",
    "planeje",
    "prepare",
    "refaca",
    "avalie",
    "leia",
    "assista",
    "faca",
    "fazer",
    "use",
    "usar",
    "comece",
    "comecar",
    "learn",
    "study",
    "practice",
    "review",
    "complete",
    "create",
    "build",
    "apply",
    "analyze",
    "compare",
    "identify",
    "describe",
    "explain",
    "choose",
    "define",
    "organize",
    "plan",
    "prepare",
    "read",
];

fn strip_prefix<'a>(step: &'a str) -> &'a str {
    let trimmed = step.trim();
    let lower = trimmed.to_lowercase();
    for prefix in PREFIXES {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let rest = trimmed[trimmed.len() - rest.len()..].trim();
            if !rest.is_empty() {
                return rest;
            }
        }
    }
    trimmed
}

fn is_stopword(token: &str) -> bool {
    STOPWORDS.contains(&token) || INSTRUCTION_WORDS.contains(&token)
}

pub fn tokens(text: &str) -> Vec<String> {
    let normalized = normalize(text);
    let mut seen = std::collections::HashSet::new();
    normalized
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() >= MIN_TOKEN_LEN)
        .filter(|token| !token.chars().all(|character| character.is_numeric()))
        .filter(|token| !is_stopword(token))
        .filter(|token| seen.insert(token.to_string()))
        .map(str::to_string)
        .collect()
}

pub fn build(topic: &str, step: &str) -> String {
    let step = strip_prefix(step);
    let mut out = tokens(step);
    out.truncate(MAX_STEP_TOKENS);

    for token in tokens(topic) {
        if out.len() >= MAX_QUERY_TOKENS {
            break;
        }
        if !out.contains(&token) {
            out.push(token);
        }
    }

    if out.is_empty() {
        let fallback = if step.trim().is_empty() {
            topic.trim()
        } else {
            step.trim()
        };
        return fallback.to_string();
    }

    out.join(" ")
}

/// Build a connector-specific query from the content brief. Web queries are
/// topic-first (SearXNG/Bing returns dictionary noise when a step verb leads),
/// and the step's content intent adds the medium hint.
pub fn build_for(connector: ConnectorKind, brief: &ContentBrief) -> String {
    let mut terms = brief.topic_terms.clone();
    for term in &brief.step_terms {
        if !terms.contains(term) {
            terms.push(term.clone());
        }
    }

    let intent: &[&str] = match brief.content_intent.as_str() {
        "worked_example" => &["exemplo", "passo"],
        "practice" => &["exercicio", "simulado"],
        "assessment" => &["quiz", "prova"],
        "review" => &["revisao"],
        _ => &["explicacao"],
    };
    for term in intent {
        let term = (*term).to_string();
        if !terms.contains(&term) {
            terms.push(term);
        }
    }

    let pt = brief.language.to_lowercase().starts_with("pt");
    let hint = match connector {
        ConnectorKind::Youtube => Some(if pt { "aula" } else { "tutorial" }),
        ConnectorKind::InternetArchive | ConnectorKind::Arxiv => Some("pdf"),
        _ => None,
    };
    if let Some(hint) = hint {
        let hint = hint.to_string();
        if !terms.contains(&hint) {
            terms.push(hint);
        }
    }

    terms.truncate(MAX_QUERY_TOKENS);
    if terms.is_empty() {
        return brief.search_text();
    }
    terms.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_planner_prefixes() {
        assert_eq!(
            build("Calculus II", "Quick win: Calculus II"),
            "calculus ii"
        );
        assert_eq!(build("Calculus II", "Study: Calculus II"), "calculus ii");
    }

    #[test]
    fn drops_stopwords_and_instruction_verbs() {
        let query = build(
            "Introdução à Redação do ENEM",
            "Entenda a rubrica de correção",
        );
        assert_eq!(query, "rubrica correcao introducao redacao enem");
    }

    #[test]
    fn adds_topic_context_before_generic_steps() {
        let query = build("Introdução à Redação do ENEM", "Simulado completo");
        assert_eq!(query, "simulado completo introducao redacao enem");
    }

    #[test]
    fn caps_keyword_count() {
        let query = build(
            "one two three four five six seven eight nine",
            "alpha beta gamma delta epsilon zeta eta theta",
        );
        assert!(query.split(' ').count() <= MAX_QUERY_TOKENS);
    }

    #[test]
    fn falls_back_to_step_or_topic_when_everything_is_a_stopword() {
        assert_eq!(build("Redação", "de a o"), "redacao");
        assert_eq!(build("Redação ENEM", "  "), "redacao enem");
    }
}
