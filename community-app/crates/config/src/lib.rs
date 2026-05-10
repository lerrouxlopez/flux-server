use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app_env: String,
    pub http_addr: String,
    pub ws_addr: String,
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: String,
    pub jwt_access_secret: String,
    pub jwt_refresh_secret: String,
    pub access_token_ttl_seconds: u64,
    pub refresh_token_ttl_seconds: u64,
    pub livekit_url_internal: String,
    pub livekit_url_public: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let livekit_url_legacy = std::env::var("LIVEKIT_URL").ok();
        let livekit_url_internal = std::env::var("LIVEKIT_URL_INTERNAL")
            .ok()
            .or_else(|| livekit_url_legacy.clone())
            .with_context(|| "LIVEKIT_URL_INTERNAL (or LIVEKIT_URL) must be set")?;
        let livekit_url_public = std::env::var("LIVEKIT_URL_PUBLIC")
            .ok()
            .or_else(|| livekit_url_legacy)
            .with_context(|| "LIVEKIT_URL_PUBLIC (or LIVEKIT_URL) must be set")?;

        Ok(Self {
            app_env: env_string("APP_ENV").unwrap_or_else(|_| "local".to_string()),
            http_addr: env_string("HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            ws_addr: env_string("WS_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string()),
            database_url: env_string("DATABASE_URL")?,
            redis_url: env_string("REDIS_URL")?,
            nats_url: env_string("NATS_URL")?,
            jwt_access_secret: env_string("JWT_ACCESS_SECRET")?,
            jwt_refresh_secret: env_string("JWT_REFRESH_SECRET")?,
            access_token_ttl_seconds: env_u64("ACCESS_TOKEN_TTL_SECONDS", 900)?,
            refresh_token_ttl_seconds: env_u64("REFRESH_TOKEN_TTL_SECONDS", 2_592_000)?,
            livekit_url_internal,
            livekit_url_public,
            livekit_api_key: env_string("LIVEKIT_API_KEY")?,
            livekit_api_secret: env_string("LIVEKIT_API_SECRET")?,
        })
    }
}

fn env_string(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("{key} must be set"))
}

fn env_u64(key: &str, default: u64) -> anyhow::Result<u64> {
    match std::env::var(key) {
        Ok(v) => v
            .parse::<u64>()
            .with_context(|| format!("{key} must be a valid u64")),
        Err(_) => Ok(default),
    }
}
