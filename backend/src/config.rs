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
    pub nim_api_key: Option<String>,
    pub nim_model: Option<String>,
    pub nim_base_url: String,
    pub nim_timeout_secs: u64,
    pub youtube_api_key: Option<String>,
    pub youtube_daily_units: i64,
    pub connector_timeout_secs: u64,
    pub pdf_service_url: String,
    pub pdf_service_timeout_secs: u64,
    pub pdf_max_bytes: usize,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            access_token_ttl_secs: parse_i64("ACCESS_TOKEN_TTL_SECS", 900),
            refresh_token_ttl_secs: parse_i64("REFRESH_TOKEN_TTL_SECS", 60 * 60 * 24 * 30),
            admin_email: env::var("ADMIN_EMAIL")
                .unwrap_or_else(|_| "admin@cronogramando.local".into()),
            admin_password: env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "changeme123".into()),
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".into()),
            nim_api_key: optional_env("NIM_API_KEY"),
            nim_model: optional_env("NIM_MODEL"),
            nim_base_url: optional_env("NIM_BASE_URL")
                .unwrap_or_else(|| "https://integrate.api.nvidia.com/v1".into()),
            nim_timeout_secs: parse_i64("NIM_TIMEOUT_SECS", 60).max(5) as u64,
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

fn optional_env(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

fn parse_i64(key: &str, default: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default)
}
