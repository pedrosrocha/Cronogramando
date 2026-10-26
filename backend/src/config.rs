use std::collections::HashSet;
use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub access_token_ttl_secs: i64,
    pub refresh_token_ttl_secs: i64,
    pub admin_email: String,
    pub admin_password: String,
    pub bind_addr: String,
    pub llm_decomposer: LlmConfig,
    pub llm_planner: LlmConfig,
    pub llm_translator: LlmConfig,
    pub llm_curator: LlmConfig,
    pub llm_tutor: LlmConfig,
    pub searxng: SearxngConfig,
    pub content: ContentConfig,
    pub youtube_api_key: Option<String>,
    pub youtube_daily_units: i64,
    pub connector_timeout_secs: u64,
    pub pdf_service_url: String,
    pub pdf_service_timeout_secs: u64,
    pub pdf_max_bytes: usize,
}

/// Self-hosted SearXNG endpoint used as the general web-search connector.
#[derive(Clone, Debug)]
pub struct SearxngConfig {
    pub base_url: String,
    pub engines: String,
    pub language: String,
    pub max_results: usize,
    pub timeout_secs: u64,
}

impl SearxngConfig {
    pub fn enabled(&self) -> bool {
        !self.base_url.trim().is_empty()
    }
}

/// Content-quality knobs used by the resolver, verifier and judge.
#[derive(Clone, Debug)]
pub struct ContentConfig {
    pub verify_top_k: usize,
    pub max_pdf_pages: u32,
    pub max_pdf_bytes: usize,
    pub min_article_words: usize,
    pub max_article_words: usize,
    pub min_video_secs: u32,
    pub max_video_secs: u32,
    pub curator_mode: String,
    pub fallback_lesson: String,
    pub judge_threshold: i32,
    pub judge_margin: i32,
    pub judge_timeout_secs: u64,
    pub tutor_timeout_secs: u64,
}

impl ContentConfig {
    pub fn judge_enabled(&self) -> bool {
        self.curator_mode != "off"
    }
}

/// One OpenAI-compatible chat-completions provider configuration.
///
/// `models` is ordered: the first entry is the primary model and the rest are
/// fallbacks used when the primary is rate-limited or unavailable.
#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub models: Vec<String>,
    pub timeout_secs: u64,
    pub max_retries: u32,
}

impl LlmConfig {
    pub fn enabled(&self) -> bool {
        self.models.iter().any(|model| !model.trim().is_empty())
            && self
                .api_key
                .as_deref()
                .map(|key| !key.trim().is_empty())
                .unwrap_or(false)
    }
}

impl Config {
    pub fn from_env() -> Self {
        let global_base_url = optional_env("LLM_BASE_URL")
            .or_else(|| optional_env("NIM_BASE_URL"))
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".into());
        let global_api_key = optional_env("LLM_API_KEY").or_else(|| optional_env("NIM_API_KEY"));
        let global_timeout_secs =
            parse_i64("LLM_TIMEOUT_SECS", parse_i64("NIM_TIMEOUT_SECS", 90)).max(5) as u64;
        let global_max_retries = parse_i64("LLM_MAX_RETRIES", 2).clamp(0, 8) as u32;
        let legacy_model = optional_env("NIM_MODEL");

        let llm_decomposer = role_llm(
            "DECOMPOSER",
            &global_base_url,
            global_api_key.clone(),
            global_timeout_secs,
            global_max_retries,
            legacy_model.as_deref(),
        );
        let llm_planner = role_llm(
            "PLANNER",
            &global_base_url,
            global_api_key.clone(),
            global_timeout_secs,
            global_max_retries,
            legacy_model.as_deref(),
        );
        let llm_translator = role_llm(
            "TRANSLATOR",
            &global_base_url,
            global_api_key.clone(),
            global_timeout_secs,
            global_max_retries,
            legacy_model.as_deref(),
        );
        let mut llm_curator = role_llm(
            "CURATOR",
            &global_base_url,
            global_api_key.clone(),
            global_timeout_secs,
            global_max_retries,
            legacy_model.as_deref(),
        );
        if llm_curator.models.is_empty() {
            llm_curator = llm_planner.clone();
        }
        let mut llm_tutor = role_llm(
            "TUTOR",
            &global_base_url,
            global_api_key,
            global_timeout_secs,
            global_max_retries,
            legacy_model.as_deref(),
        );
        if llm_tutor.models.is_empty() {
            llm_tutor = llm_planner.clone();
        }

        let web_provider =
            optional_env("CONTENT_WEB_SEARCH_PROVIDER").unwrap_or_else(|| "searxng".into());
        let searxng = SearxngConfig {
            base_url: if web_provider.eq_ignore_ascii_case("none") {
                String::new()
            } else {
                optional_env("SEARXNG_URL").unwrap_or_else(|| "http://127.0.0.1:8082".into())
            },
            engines: optional_env("SEARXNG_ENGINES").unwrap_or_else(|| "bing".into()),
            language: optional_env("SEARXNG_LANGUAGE").unwrap_or_else(|| "pt-BR".into()),
            max_results: parse_i64("SEARXNG_MAX_RESULTS", 10).clamp(1, 20) as usize,
            timeout_secs: parse_i64("SEARXNG_TIMEOUT_SECS", 10).max(2) as u64,
        };
        let content = ContentConfig {
            verify_top_k: parse_i64("CONTENT_VERIFY_TOP_K", 3).clamp(1, 10) as usize,
            max_pdf_pages: parse_i64("CONTENT_MAX_PDF_PAGES", 30).clamp(1, 500) as u32,
            max_pdf_bytes: parse_i64("CONTENT_MAX_PDF_BYTES", 15 * 1024 * 1024).max(1024) as usize,
            min_article_words: parse_i64("CONTENT_MIN_ARTICLE_WORDS", 300).max(50) as usize,
            max_article_words: parse_i64("CONTENT_MAX_ARTICLE_WORDS", 3000).max(100) as usize,
            min_video_secs: parse_i64("CONTENT_MIN_VIDEO_SECS", 180).max(1) as u32,
            max_video_secs: parse_i64("CONTENT_MAX_VIDEO_SECS", 1200).max(60) as u32,
            curator_mode: optional_env("CONTENT_CURATOR_MODE")
                .unwrap_or_else(|| "ambiguous".into()),
            fallback_lesson: optional_env("CONTENT_FALLBACK_LESSON")
                .unwrap_or_else(|| "auto".into()),
            judge_threshold: parse_i64("CONTENT_JUDGE_THRESHOLD", 7).clamp(0, 10) as i32,
            judge_margin: parse_i64("CONTENT_JUDGE_MARGIN_PCT", 15).clamp(1, 100) as i32,
            judge_timeout_secs: parse_i64("CONTENT_JUDGE_TIMEOUT_SECS", 45).max(5) as u64,
            tutor_timeout_secs: parse_i64("CONTENT_TUTOR_TIMEOUT_SECS", 45).max(5) as u64,
        };

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            access_token_ttl_secs: parse_i64("ACCESS_TOKEN_TTL_SECS", 900),
            refresh_token_ttl_secs: parse_i64("REFRESH_TOKEN_TTL_SECS", 60 * 60 * 24 * 30),
            admin_email: env::var("ADMIN_EMAIL")
                .unwrap_or_else(|_| "admin@cronogramando.local".into()),
            admin_password: env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "changeme123".into()),
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".into()),
            llm_decomposer,
            llm_planner,
            llm_translator,
            llm_curator,
            llm_tutor,
            searxng,
            content,
            youtube_api_key: optional_env("YOUTUBE_API_KEY"),
            youtube_daily_units: parse_i64("YOUTUBE_DAILY_UNITS", 10_000),
            connector_timeout_secs: parse_i64("CONNECTOR_TIMEOUT_SECS", 5).max(1) as u64,
            pdf_service_url: optional_env("PDF_SERVICE_URL")
                .unwrap_or_else(|| "http://localhost:8001".into()),
            pdf_service_timeout_secs: parse_i64("PDF_SERVICE_TIMEOUT_SECS", 180).max(5) as u64,
            pdf_max_bytes: parse_i64("PDF_MAX_BYTES", 64 * 1024 * 1024).max(1) as usize,
        }
    }
}

fn role_llm(
    role: &str,
    global_base_url: &str,
    global_api_key: Option<String>,
    global_timeout_secs: u64,
    global_max_retries: u32,
    legacy_model: Option<&str>,
) -> LlmConfig {
    let prefix = format!("LLM_{role}");

    let base_url =
        optional_env(&format!("{prefix}_BASE_URL")).unwrap_or_else(|| global_base_url.to_string());
    let api_key = optional_env(&format!("{prefix}_API_KEY")).or(global_api_key);
    let timeout_secs = parse_i64_opt(&format!("{prefix}_TIMEOUT_SECS"))
        .map(|value| value.max(5) as u64)
        .unwrap_or(global_timeout_secs);
    let max_retries = parse_i64_opt(&format!("{prefix}_MAX_RETRIES"))
        .map(|value| value.clamp(0, 8) as u32)
        .unwrap_or(global_max_retries);

    let mut models = Vec::new();
    if let Some(primary) = optional_env(&format!("{prefix}_MODEL")) {
        models.push(primary);
    } else if let Some(legacy) = legacy_model {
        models.push(legacy.to_string());
    }
    if let Some(fallbacks) = optional_env(&format!("{prefix}_FALLBACK_MODELS")) {
        models.extend(split_list(&fallbacks));
    }

    let mut seen = HashSet::new();
    models.retain(|model| seen.insert(model.clone()));

    LlmConfig {
        base_url,
        api_key,
        models,
        timeout_secs,
        max_retries,
    }
}

fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn parse_i64(key: &str, default: i64) -> i64 {
    parse_i64_opt(key).unwrap_or(default)
}

fn parse_i64_opt(key: &str) -> Option<i64> {
    env::var(key).ok().and_then(|v| v.parse::<i64>().ok())
}
