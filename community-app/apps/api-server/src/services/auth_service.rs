use crate::{
    models::auth::{self, ApiError, AuthContext, AuthResponse, LoginRequest, RegisterRequest, UserView},
    repositories::{SessionRepository, UserRepository},
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use validator::ValidateEmail;

#[derive(Clone)]
pub struct AuthService {
    users: UserRepository,
    sessions: SessionRepository,
    jwt: JwtConfig,
}

#[derive(Clone)]
pub struct JwtConfig {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_ttl: Duration,
    refresh_ttl: Duration,
    refresh_pepper: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessClaims {
    sub: String,
    sid: String,
    iat: i64,
    exp: i64,
    typ: String,
}

impl AuthService {
    pub fn new(users: UserRepository, sessions: SessionRepository, jwt: JwtConfig) -> Self {
        Self { users, sessions, jwt }
    }

    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse, ApiError> {
        let email = normalize_email(&req.email).ok_or_else(ApiError::bad_request)?;
        if !email.validate_email() {
            return Err(ApiError::bad_request());
        }
        if req.display_name.trim().is_empty() {
            return Err(ApiError::bad_request());
        }
        if req.password.len() < 8 {
            return Err(ApiError::bad_request());
        }

        if let Some(_) = self.users.find_by_email(&email).await.map_err(|_| ApiError::internal())? {
            return Err(ApiError::conflict());
        }

        let password_hash = hash_password(&req.password).map_err(|_| ApiError::internal())?;
        let user_id = Uuid::now_v7();
        let user = self
            .users
            .insert_user(user_id, &email, req.display_name.trim(), &password_hash)
            .await
            .map_err(map_unique_violation)?;

        let (access_token, refresh_token) = self.issue_session_and_tokens(user.id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user: UserView {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                created_at: user.created_at,
            },
        })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse, ApiError> {
        let email = normalize_email(&req.email).ok_or_else(ApiError::bad_request)?;
        if !email.validate_email() {
            return Err(ApiError::bad_request());
        }
        let user = self
            .users
            .find_by_email(&email)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        let Some(ref password_hash) = user.password_hash else {
            return Err(ApiError::unauthorized());
        };
        verify_password(password_hash, &req.password).map_err(|_| ApiError::unauthorized())?;

        let (access_token, refresh_token) = self.issue_session_and_tokens(user.id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user: UserView {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                created_at: user.created_at,
            },
        })
    }

    pub async fn refresh(&self, req: auth::RefreshRequest) -> Result<AuthResponse, ApiError> {
        if req.refresh_token.trim().is_empty() {
            return Err(ApiError::bad_request());
        }

        let now = OffsetDateTime::now_utc();
        let refresh_hash = hash_refresh_token(&self.jwt.refresh_pepper, &req.refresh_token);
        let session = self
            .sessions
            .find_by_refresh_hash(&refresh_hash)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        if session.revoked_at.is_some() || session.expires_at <= now {
            return Err(ApiError::unauthorized());
        }

        let user = self
            .users
            .find_by_id(session.user_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        let (new_refresh_token, new_refresh_hash) = generate_refresh_token_and_hash(&self.jwt.refresh_pepper);
        let new_expires = now + self.jwt.refresh_ttl;
        self.sessions
            .rotate_refresh_token_if_matches(session.id, &refresh_hash, &new_refresh_hash, now, new_expires)
            .await
            .map_err(|_| ApiError::internal())?
            .then_some(())
            .ok_or_else(ApiError::unauthorized)?;

        let access_token = self
            .sign_access_token(session.user_id, session.id, now)
            .map_err(|_| ApiError::internal())?;

        Ok(AuthResponse {
            access_token,
            refresh_token: new_refresh_token,
            user: UserView {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                created_at: user.created_at,
            },
        })
    }

    pub async fn logout(&self, session_id: Uuid) -> Result<(), ApiError> {
        let now = OffsetDateTime::now_utc();
        self.sessions
            .revoke(session_id, now)
            .await
            .map_err(|_| ApiError::internal())?;
        Ok(())
    }

    pub async fn authenticate(&self, access_token: &str) -> Result<AuthContext, ApiError> {
        let token_data = jsonwebtoken::decode::<AccessClaims>(
            access_token,
            &self.jwt.decoding_key,
            &Validation::default(),
        )
        .map_err(|_| ApiError::unauthorized())?;

        if token_data.claims.typ != "access" {
            return Err(ApiError::unauthorized());
        }

        let user_id = Uuid::parse_str(&token_data.claims.sub).map_err(|_| ApiError::unauthorized())?;
        let session_id =
            Uuid::parse_str(&token_data.claims.sid).map_err(|_| ApiError::unauthorized())?;

        let now = OffsetDateTime::now_utc();
        let session = self
            .sessions
            .find_active_by_id(session_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        if session.user_id != user_id || session.revoked_at.is_some() || session.expires_at <= now {
            return Err(ApiError::unauthorized());
        }

        self.sessions
            .mark_used(session_id, now)
            .await
            .map_err(|_| ApiError::internal())?;

        Ok(AuthContext { user_id, session_id })
    }

    pub async fn me(&self, user_id: Uuid) -> Result<UserView, ApiError> {
        let user = self
            .users
            .find_by_id(user_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        Ok(UserView {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            created_at: user.created_at,
        })
    }

    async fn issue_session_and_tokens(&self, user_id: Uuid) -> Result<(String, String), ApiError> {
        let now = OffsetDateTime::now_utc();
        let session_id = Uuid::now_v7();
        let (refresh_token, refresh_hash) = generate_refresh_token_and_hash(&self.jwt.refresh_pepper);
        let refresh_expires = now + self.jwt.refresh_ttl;

        self.sessions
            .insert_session(session_id, user_id, &refresh_hash, refresh_expires, None, None)
            .await
            .map_err(|_| ApiError::internal())?;

        let access_token = self
            .sign_access_token(user_id, session_id, now)
            .map_err(|_| ApiError::internal())?;

        Ok((access_token, refresh_token))
    }

    fn sign_access_token(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        now: OffsetDateTime,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let iat = now.unix_timestamp();
        let exp = (now + self.jwt.access_ttl).unix_timestamp();
        let claims = AccessClaims {
            sub: user_id.to_string(),
            sid: session_id.to_string(),
            iat,
            exp,
            typ: "access".to_string(),
        };

        jsonwebtoken::encode(&Header::default(), &claims, &self.jwt.encoding_key)
    }
}

pub fn jwt_config_from_env() -> Result<JwtConfig, ApiError> {
    let access_secret = std::env::var("JWT_ACCESS_SECRET").map_err(|_| ApiError::internal())?;
    let refresh_secret = std::env::var("JWT_REFRESH_SECRET").map_err(|_| ApiError::internal())?;
    if access_secret.trim().len() < 16 || refresh_secret.trim().len() < 16 {
        return Err(ApiError::internal());
    }

    let access_ttl_seconds: i64 = std::env::var("ACCESS_TOKEN_TTL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(900);
    let refresh_ttl_seconds: i64 = std::env::var("REFRESH_TOKEN_TTL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(2_592_000);

    Ok(JwtConfig {
        encoding_key: EncodingKey::from_secret(access_secret.as_bytes()),
        decoding_key: DecodingKey::from_secret(access_secret.as_bytes()),
        access_ttl: Duration::seconds(access_ttl_seconds.max(60)),
        refresh_ttl: Duration::seconds(refresh_ttl_seconds.clamp(60, 31_536_000)),
        refresh_pepper: refresh_secret,
    })
}

fn normalize_email(email: &str) -> Option<String> {
    let e = email.trim().to_lowercase();
    if e.is_empty() || !e.contains('@') {
        return None;
    }
    Some(e)
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::default(),
    );
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

fn verify_password(hash: &str, password: &str) -> Result<(), argon2::password_hash::Error> {
    let parsed = PasswordHash::new(hash)?;
    Argon2::default().verify_password(password.as_bytes(), &parsed)
}

fn generate_refresh_token_and_hash(refresh_pepper: &str) -> (String, String) {
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    let token = URL_SAFE_NO_PAD.encode(buf);
    let hash = hash_refresh_token(refresh_pepper, &token);
    (token, hash)
}

fn hash_refresh_token(refresh_pepper: &str, token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(refresh_pepper.as_bytes());
    hasher.update(b":");
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

fn map_unique_violation(err: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.is_unique_violation() {
            return ApiError::conflict();
        }
    }
    ApiError::internal()
}
