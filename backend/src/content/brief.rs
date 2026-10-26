use super::{query, types::ResolveRequest};

const GENERIC_TOPIC_WORDS: &[&str] = &[
    "introducao",
    "fundamentos",
    "conceitos",
    "conceito",
    "pratica",
    "basico",
    "basica",
    "avancado",
    "avancada",
    "estudo",
    "aula",
    "curso",
    "modulo",
    "parte",
    "texto",
    "textos",
    "aluno",
    "aluna",
    "consegue",
    "conseguir",
    "deve",
    "pode",
    "atende",
    "todos",
    "toda",
    "alta",
    "alto",
    "qualidade",
    "potencializa",
    "pontuacao",
    "identifica",
    "identificar",
    "aplica",
    "aplicar",
    "explica",
    "explicar",
    "produz",
    "produzir",
    "estrutura",
    "estruturar",
    "utiliza",
    "utilizar",
    "mantem",
    "manter",
    "analisa",
    "analisar",
    "vitoria",
    "rapida",
    "rapido",
    "final",
    "tarefa",
    "etapa",
    "concluir",
    "registrar",
    "duvida",
    "recordar",
    "acertar",
    "maioria",
    "questoes",
    "revisao",
];

/// The curated target for one step. Every retrieval and relevance decision is
/// derived from this, not from the UI title alone.
#[derive(Debug, Clone)]
pub struct ContentBrief {
    pub goal_outcome: String,
    pub module_objective: String,
    pub step_outcome: String,
    pub success_criteria: String,
    pub content_intent: String,
    pub topic_terms: Vec<String>,
    pub step_terms: Vec<String>,
    pub must_terms: Vec<String>,
    pub exclude_terms: Vec<String>,
    pub activity_type: String,
    pub max_minutes: u32,
    pub language: String,
}

impl ContentBrief {
    pub fn objective(&self) -> String {
        format!("{} {}", self.step_outcome, self.success_criteria)
            .trim()
            .to_string()
    }

    pub fn search_text(&self) -> String {
        let mut terms = self.topic_terms.clone();
        for term in &self.step_terms {
            if !terms.contains(term) {
                terms.push(term.clone());
            }
        }
        terms.join(" ")
    }
}

pub fn build(req: &ResolveRequest) -> ContentBrief {
    let topic = format!(
        "{} {} {}",
        req.goal_outcome,
        req.module_title,
        req.module_scope_in.join(" ")
    );
    let step = format!(
        "{} {} {} {}",
        req.step_title, req.step_outcome, req.success_criteria, req.module_objective
    );

    let mut topic_terms = query::tokens(&topic);
    topic_terms.retain(|token| !is_generic(token));
    let mut step_terms = query::tokens(&step);
    step_terms.retain(|token| !is_generic(token));

    let mut must_terms = Vec::new();
    for term in topic_terms.iter().chain(step_terms.iter()) {
        if must_terms.len() >= 5 {
            break;
        }
        if !must_terms.iter().any(|existing| existing == term) {
            must_terms.push(term.clone());
        }
    }
    // If a very terse goal produced no distinctive topic terms, keep the raw
    // first tokens so verification still has something to require.
    if must_terms.is_empty() {
        for term in query::tokens(&topic).into_iter().take(3) {
            must_terms.push(term);
        }
    }

    let exclude_terms = query::tokens(&req.module_scope_out.join(" "));

    ContentBrief {
        goal_outcome: req.goal_outcome.clone(),
        module_objective: req.module_objective.clone(),
        step_outcome: req.step_outcome.clone(),
        success_criteria: req.success_criteria.clone(),
        content_intent: req.content_intent.clone(),
        topic_terms,
        step_terms,
        must_terms,
        exclude_terms,
        activity_type: req.activity_type.clone(),
        max_minutes: req.estimated_duration_mins.max(10) as u32,
        language: req.language.clone(),
    }
}

fn is_generic(token: &str) -> bool {
    GENERIC_TOPIC_WORDS.contains(&token)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ResolveRequest {
        ResolveRequest {
            goal_outcome: "O aluno consegue produzir uma redação argumentativa com nota 800+ no ENEM.".into(),
            module_title: "Fundamentos Textuais e Gramaticais".into(),
            module_objective: "Ao final, o aluno consegue identificar os gêneros textuais e aplicar normas formais.".into(),
            module_scope_in: vec!["gêneros textuais".into()],
            module_scope_out: vec!["literatura".into()],
            step_title: "Introdução aos gêneros textuais argumentativos".into(),
            step_outcome: "O aluno identifica os gêneros textuais argumentativos exigidos pela prova.".into(),
            success_criteria: "O aluno classifica corretamente artigo de opinião e editorial.".into(),
            content_intent: "explanation".into(),
            activity_type: "article".into(),
            estimated_duration_mins: 30,
            language: "pt-BR".into(),
        }
    }

    #[test]
    fn keeps_distinctive_terms_and_drops_generic_ones() {
        let brief = build(&request());
        assert!(brief.must_terms.iter().any(|term| term == "enem"));
        assert!(brief.must_terms.iter().any(|term| term == "redacao"));
        assert!(!brief.must_terms.iter().any(|term| term == "aluno"));
        assert!(!brief.must_terms.iter().any(|term| term == "fundamentos"));
    }

    #[test]
    fn includes_scope_out_as_exclusions() {
        let brief = build(&request());
        assert_eq!(brief.exclude_terms, vec!["literatura".to_string()]);
    }
}
