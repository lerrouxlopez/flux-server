use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub port: u16,
    pub realtime_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: String,
    pub jwt_secret: String,
    pub refresh_token_pepper: String,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            port: env_u16("PORT", 3000)?,
            realtime_port: env_u16("REALTIME_PORT", 3001)?,
            database_url: env_string("DATABASE_URL")?,
            redis_url: env_string("REDIS_URL")?,
            nats_url: env_string("NATS_URL")?,
            jwt_secret: env_string("JWT_SECRET")?,
            refresh_token_pepper: env_string("REFRESH_TOKEN_PEPPER")?,
            livekit_url: env_string("LIVEKIT_URL")?,
            livekit_api_key: env_string("LIVEKIT_API_KEY")?,
            livekit_api_secret: env_string("LIVEKIT_API_SECRET")?,
        })
    }
}

fn env_string(key: &str) -> anyhow::Result<String> {
    std::env::var(key).with_context(|| format!("{key} must be set"))
}

fn env_u16(key: &str, default: u16) -> anyhow::Result<u16> {
    match std::env::var(key) {
        Ok(v) => v
            .parse::<u16>()
            .with_context(|| format!("{key} must be a valid u16")),
        Err(_) => Ok(default),
    }
}

