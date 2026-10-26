use std::collections::HashSet;

use super::types::SourceCategory;

struct CategoryKeywords {
    category: SourceCategory,
    phrases: &'static [&'static str],
}

const KEYWORDS: &[CategoryKeywords] = &[
    CategoryKeywords {
        category: SourceCategory::Manual,
        phrases: &[
            "diy",
            "repair",
            "conserto",
            "reparo",
            "handyman",
            "woodworking",
            "marcenaria",
            "sewing",
            "costura",
            "knitting",
            "tricô",
            "cooking",
            "culinaria",
            "cozinha",
            "gardening",
            "jardinagem",
            "driving",
            "dirigir",
            "craft",
            "artesanato",
            "survival",
            "sobrevivencia",
            "hands on",
            "pratica",
            "oficina",
            "workshop",
            "home improvement",
            "encanamento",
            "eletricista",
            "bike repair",
        ],
    },
    CategoryKeywords {
        category: SourceCategory::Stem,
        phrases: &[
            "math",
            "matematica",
            "calculus",
            "calculo",
            "algebra",
            "geometry",
            "geometria",
            "physics",
            "fisica",
            "chemistry",
            "quimica",
            "biology",
            "biologia",
            "science",
            "ciencia",
            "programming",
            "programacao",
            "coding",
            "software",
            "computer",
            "computacao",
            "algorithm",
            "algoritmo",
            "data",
            "dados",
            "statistics",
            "estatistica",
            "machine learning",
            "artificial intelligence",
            "ai",
            "linux",
            "python",
            "javascript",
            "typescript",
            "rust",
            "java",
            "sql",
            "database",
            "banco de dados",
            "network",
            "redes",
            "electronics",
            "eletronica",
            "engineering",
            "engenharia",
            "robotics",
            "robotica",
            "astronomy",
            "astronomia",
            "git",
            "devops",
            "cloud",
            "docker",
        ],
    },
    CategoryKeywords {
        category: SourceCategory::Business,
        phrases: &[
            "business",
            "negocios",
            "marketing",
            "finance",
            "financas",
            "financial",
            "accounting",
            "contabilidade",
            "entrepreneurship",
            "empreendedorismo",
            "startup",
            "management",
            "gestao",
            "sales",
            "vendas",
            "ecommerce",
            "economics",
            "economia",
            "investing",
            "investimento",
            "leadership",
            "lideranca",
            "product management",
            "gestao de produtos",
            "project management",
            "gestao de projetos",
            "human resources",
            "recursos humanos",
        ],
    },
    CategoryKeywords {
        category: SourceCategory::Humanities,
        phrases: &[
            "history",
            "historia",
            "philosophy",
            "filosofia",
            "law",
            "legal",
            "direito",
            "juridico",
            "literature",
            "literatura",
            "psychology",
            "psicologia",
            "sociology",
            "sociologia",
            "politics",
            "political",
            "politica",
            "linguistics",
            "linguistica",
            "language",
            "idioma",
            "writing",
            "escrita",
            "redacao",
            "redator",
            "enem",
            "vestibular",
            "concurso",
            "argumentacao",
            "argumentativo",
            "dissertacao",
            "tese",
            "prova",
            "exame",
            "essay",
            "ethics",
            "etica",
            "religion",
            "religiao",
            "geography",
            "geografia",
            "anthropology",
            "antropologia",
            "journalism",
            "jornalismo",
        ],
    },
    CategoryKeywords {
        category: SourceCategory::Arts,
        phrases: &[
            "art",
            "arte",
            "painting",
            "pintura",
            "drawing",
            "desenho",
            "design",
            "graphic design",
            "music",
            "musica",
            "music theory",
            "teoria musical",
            "photography",
            "fotografia",
            "film",
            "cinema",
            "theater",
            "theatre",
            "teatro",
            "dance",
            "danca",
            "sculpture",
            "escultura",
            "illustration",
            "ilustracao",
            "animation",
            "animacao",
            "guitar",
            "violao",
            "piano",
            "singing",
            "canto",
            "acting",
            "atuacao",
        ],
    },
    CategoryKeywords {
        category: SourceCategory::Health,
        phrases: &[
            "health",
            "saude",
            "fitness",
            "nutrition",
            "nutricao",
            "medicine",
            "medicina",
            "medical",
            "anatomy",
            "anatomia",
            "nursing",
            "enfermagem",
            "exercise",
            "exercicio",
            "workout",
            "treino",
            "yoga",
            "ioga",
            "mental health",
            "saude mental",
            "first aid",
            "primeiros socorros",
            "diet",
            "dieta",
            "wellness",
            "bem estar",
            "physiology",
            "fisiologia",
            "meditation",
            "meditacao",
        ],
    },
];

pub(crate) fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

fn tokenize(text: &str) -> HashSet<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect()
}

pub fn classify(title: &str) -> Vec<SourceCategory> {
    let normalized = normalize(title);
    let tokens = tokenize(&normalized);

    let mut scores: Vec<(SourceCategory, u32)> = KEYWORDS
        .iter()
        .map(|entry| {
            let score = entry
                .phrases
                .iter()
                .map(|phrase| {
                    if phrase.contains(' ') {
                        if normalized.contains(phrase) {
                            3
                        } else {
                            0
                        }
                    } else if tokens.contains(phrase) {
                        2
                    } else {
                        0
                    }
                })
                .sum();
            (entry.category, score)
        })
        .collect();

    scores.sort_by(|a, b| b.1.cmp(&a.1));

    if scores[0].1 == 0 {
        return vec![SourceCategory::Manual];
    }

    let mut categories = vec![scores[0].0];
    if scores.len() > 1 && scores[1].1 > 0 {
        categories.push(scores[1].0);
    }
    categories
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primary(title: &str) -> SourceCategory {
        classify(title)[0]
    }

    #[test]
    fn classifies_stem() {
        assert_eq!(
            primary("Calculus II: integrals and series"),
            SourceCategory::Stem
        );
        assert_eq!(
            primary("Aprender programação em Rust"),
            SourceCategory::Stem
        );
        assert_eq!(primary("Introdução a banco de dados"), SourceCategory::Stem);
    }

    #[test]
    fn classifies_manual() {
        assert_eq!(primary("Learn to repair a bicycle"), SourceCategory::Manual);
        assert_eq!(
            primary("Curso de marcenaria para iniciantes"),
            SourceCategory::Manual
        );
    }

    #[test]
    fn classifies_business() {
        assert_eq!(
            primary("Marketing for early-stage startups"),
            SourceCategory::Business
        );
        assert_eq!(
            primary("Fundamentos de finanças pessoais"),
            SourceCategory::Business
        );
    }

    #[test]
    fn classifies_humanities() {
        assert_eq!(
            primary("History of the Roman Empire"),
            SourceCategory::Humanities
        );
        assert_eq!(
            primary("Filosofia política moderna"),
            SourceCategory::Humanities
        );
        assert_eq!(
            primary("Introdução à Redação do ENEM"),
            SourceCategory::Humanities
        );
        assert_eq!(
            primary("Como formular uma tese para o vestibular"),
            SourceCategory::Humanities
        );
    }

    #[test]
    fn classifies_arts() {
        assert_eq!(primary("Music theory for guitar"), SourceCategory::Arts);
        assert_eq!(primary("Curso de fotografia digital"), SourceCategory::Arts);
    }

    #[test]
    fn classifies_health() {
        assert_eq!(
            primary("Yoga and mobility for beginners"),
            SourceCategory::Health
        );
        assert_eq!(
            primary("Nutrição e dieta equilibrada"),
            SourceCategory::Health
        );
    }

    #[test]
    fn defaults_to_manual_when_unknown() {
        assert_eq!(
            primary("Observação de aves migratórias"),
            SourceCategory::Manual
        );
    }

    #[test]
    fn returns_at_most_two_categories() {
        assert!(classify("physics and marketing").len() <= 2);
    }
}
