use super::types::{ResourceCandidate, SourceCategory, SourceCategory::*};

pub struct CuratedResource {
    pub title: &'static str,
    pub url: &'static str,
    pub language: &'static str,
    pub activity_type: &'static str,
    pub category: SourceCategory,
}

const CURATED: &[CuratedResource] = &[
    CuratedResource {
        title: "WikiHow em português",
        url: "https://pt.wikihow.com/",
        language: "pt-BR",
        activity_type: "article",
        category: Manual,
    },
    CuratedResource {
        title: "Internet Archive — artesanato e ofícios",
        url: "https://archive.org/details/crafts",
        language: "pt-BR",
        activity_type: "article",
        category: Manual,
    },
    CuratedResource {
        title: "iFixit Repair Guides",
        url: "https://www.ifixit.com/Guide",
        language: "en",
        activity_type: "article",
        category: Manual,
    },
    CuratedResource {
        title: "WikiHow",
        url: "https://www.wikihow.com/",
        language: "en",
        activity_type: "article",
        category: Manual,
    },
    CuratedResource {
        title: "Khan Academy em português",
        url: "https://pt.khanacademy.org/",
        language: "pt-BR",
        activity_type: "video",
        category: Stem,
    },
    CuratedResource {
        title: "Matemática — Wikipédia",
        url: "https://pt.wikipedia.org/wiki/Matem%C3%A1tica",
        language: "pt-BR",
        activity_type: "article",
        category: Stem,
    },
    CuratedResource {
        title: "OpenStax — Mathematics",
        url: "https://openstax.org/subjects/math",
        language: "en",
        activity_type: "pdf",
        category: Stem,
    },
    CuratedResource {
        title: "arXiv — open access preprints",
        url: "https://arxiv.org/",
        language: "en",
        activity_type: "pdf",
        category: Stem,
    },
    CuratedResource {
        title: "Sebrae — empreendedorismo",
        url: "https://sebrae.com.br/",
        language: "pt-BR",
        activity_type: "article",
        category: Business,
    },
    CuratedResource {
        title: "Administração — Wikipédia",
        url: "https://pt.wikipedia.org/wiki/Administra%C3%A7%C3%A3o",
        language: "pt-BR",
        activity_type: "article",
        category: Business,
    },
    CuratedResource {
        title: "OpenStax — Business",
        url: "https://openstax.org/subjects/business",
        language: "en",
        activity_type: "pdf",
        category: Business,
    },
    CuratedResource {
        title: "Business — Wikipedia",
        url: "https://en.wikipedia.org/wiki/Business",
        language: "en",
        activity_type: "article",
        category: Business,
    },
    CuratedResource {
        title: "Domínio Público",
        url: "https://www.dominiopublico.gov.br/",
        language: "pt-BR",
        activity_type: "article",
        category: Humanities,
    },
    CuratedResource {
        title: "História — Wikipédia",
        url: "https://pt.wikipedia.org/wiki/Hist%C3%B3ria",
        language: "pt-BR",
        activity_type: "article",
        category: Humanities,
    },
    CuratedResource {
        title: "Project Gutenberg",
        url: "https://www.gutenberg.org/",
        language: "en",
        activity_type: "article",
        category: Humanities,
    },
    CuratedResource {
        title: "Stanford Encyclopedia of Philosophy",
        url: "https://plato.stanford.edu/",
        language: "en",
        activity_type: "article",
        category: Humanities,
    },
    CuratedResource {
        title: "Enciclopédia Itaú Cultural",
        url: "https://enciclopedia.itaucultural.org.br/",
        language: "pt-BR",
        activity_type: "article",
        category: Arts,
    },
    CuratedResource {
        title: "Arte — Wikipédia",
        url: "https://pt.wikipedia.org/wiki/Arte",
        language: "pt-BR",
        activity_type: "article",
        category: Arts,
    },
    CuratedResource {
        title: "The Metropolitan Museum of Art — collection",
        url: "https://www.metmuseum.org/art/collection",
        language: "en",
        activity_type: "article",
        category: Arts,
    },
    CuratedResource {
        title: "Art — Wikipedia",
        url: "https://en.wikipedia.org/wiki/Art",
        language: "en",
        activity_type: "article",
        category: Arts,
    },
    CuratedResource {
        title: "Ministério da Saúde",
        url: "https://www.gov.br/saude/",
        language: "pt-BR",
        activity_type: "article",
        category: Health,
    },
    CuratedResource {
        title: "Saúde — Wikipédia",
        url: "https://pt.wikipedia.org/wiki/Sa%C3%BAde",
        language: "pt-BR",
        activity_type: "article",
        category: Health,
    },
    CuratedResource {
        title: "WHO — Health topics",
        url: "https://www.who.int/health-topics",
        language: "en",
        activity_type: "article",
        category: Health,
    },
    CuratedResource {
        title: "OpenStax — Science",
        url: "https://openstax.org/subjects/science",
        language: "en",
        activity_type: "pdf",
        category: Health,
    },
];

pub fn for_category(category: SourceCategory) -> impl Iterator<Item = &'static CuratedResource> {
    CURATED.iter().filter(move |r| r.category == category)
}

pub fn fallback_candidates(category: SourceCategory, language: &str) -> Vec<ResourceCandidate> {
    let want_pt = language.to_lowercase().starts_with("pt");
    let mut matching: Vec<&CuratedResource> = for_category(category)
        .filter(|r| {
            if want_pt {
                r.language == "pt-BR" || r.language == "en"
            } else {
                r.language == "en"
            }
        })
        .collect();

    matching.sort_by_key(|r| {
        let exact = r.language.eq_ignore_ascii_case(language) || (want_pt && r.language == "pt-BR");
        if exact {
            0
        } else {
            1
        }
    });

    matching
        .into_iter()
        .map(|r| ResourceCandidate {
            title: r.title.to_string(),
            url: Some(r.url.to_string()),
            embed_id: None,
            connector: super::types::ConnectorKind::Curated,
            language: r.language.to_string(),
            activity_type: r.activity_type.to_string(),
            category,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_category_has_fallback() {
        for category in SourceCategory::ALL {
            assert!(!fallback_candidates(category, "pt-BR").is_empty());
            assert!(!fallback_candidates(category, "en").is_empty());
        }
    }

    #[test]
    fn portuguese_prefers_portuguese_fallback() {
        let candidates = fallback_candidates(Stem, "pt-BR");
        assert_eq!(candidates[0].language, "pt-BR");
        assert!(candidates.iter().any(|c| c.language == "en"));
    }

    #[test]
    fn english_does_not_lead_with_portuguese() {
        let candidates = fallback_candidates(Stem, "en");
        assert_eq!(candidates[0].language, "en");
        assert!(candidates.iter().all(|c| c.language == "en"));
    }
}
