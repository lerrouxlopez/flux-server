use argon2::{
    password_hash::SaltString, Algorithm, Argon2, Params, PasswordHash, PasswordHasher,
    PasswordVerifier, Version,
};
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
    pub jwt_access_secret: String,
    pub jwt_refresh_secret: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub typ: String,
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::default();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
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

pub fn issue_access_token(cfg: &AuthConfig, user_id: Uuid) -> anyhow::Result<String> {
    let now = OffsetDateTime::now_utc();
    let exp = now + cfg.access_ttl;
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.unix_timestamp() as usize,
        exp: exp.unix_timestamp() as usize,
        typ: "access".to_string(),
    };
    Ok(jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(cfg.jwt_access_secret.as_bytes()),
    )?)
}

pub fn decode_access_token(cfg: &AuthConfig, token: &str) -> anyhow::Result<Claims> {
    let claims = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(cfg.jwt_access_secret.as_bytes()),
        &Validation::default(),
    )?
    .claims;
    if claims.typ != "access" {
        anyhow::bail!("invalid token type");
    }
    Ok(claims)
}

pub fn new_refresh_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_refresh_token(cfg: &AuthConfig, refresh_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(cfg.jwt_refresh_secret.as_bytes());
    hasher.update(b":");
    hasher.update(refresh_token.as_bytes());
    format!("{:x}", hasher.finalize())
}
