use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64::Engine;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub refresh_token_pepper: String,
    pub access_ttl: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub org: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hashed = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(hashed.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> anyhow::Result<bool> {
    let parsed =
        PasswordHash::new(password_hash).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn issue_access_token(cfg: &AuthConfig, user_id: Uuid, org_id: Option<Uuid>) -> anyhow::Result<String> {
    let now = OffsetDateTime::now_utc();
    let exp = now + cfg.access_ttl;
    let claims = Claims {
        sub: user_id.to_string(),
        org: org_id.map(|o| o.to_string()),
        iat: now.unix_timestamp() as usize,
        exp: exp.unix_timestamp() as usize,
    };
    Ok(jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_secret.as_bytes()),
    )?)
}

pub fn decode_access_token(cfg: &AuthConfig, token: &str) -> anyhow::Result<Claims> {
    Ok(jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_secret.as_bytes()),
        &Validation::default(),
    )?
    .claims)
}

pub fn new_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_refresh_token(cfg: &AuthConfig, refresh_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cfg.refresh_token_pepper.as_bytes());
    hasher.update(b":");
    hasher.update(refresh_token.as_bytes());
    format!("{:x}", hasher.finalize())
}
