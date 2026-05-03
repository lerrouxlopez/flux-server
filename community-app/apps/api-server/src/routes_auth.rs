use crate::{AppState, AuthContext};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;
use validator::ValidateEmail;

use crate::util;
use api::ApiErrorCode;
use redis::AsyncCommands;
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub created_at: OffsetDateTime,
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    if !req.email.validate_email() {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    if req.password.len() < 8 {
        return util::api_error_msg(
            ApiErrorCode::ValidationError,
            "Password must be at least 8 characters.",
        );
    }

    let email = req.email.trim().to_lowercase();
    let display_name = req.display_name.trim().to_string();
    if display_name.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let user_id = Uuid::now_v7();
    let password_hash = match auth::hash_password(&req.password) {
        Ok(h) => h,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let inserted = sqlx::query(
        r#"
        insert into users (id, email, display_name, password_hash)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .execute(&state.pool)
    .await;

    match inserted {
        Ok(_) => {}
        Err(err) => {
            if is_unique_violation(&err) {
                return util::api_error(ApiErrorCode::Conflict);
            }
            return util::api_error(ApiErrorCode::InternalError);
        }
    }

    match issue_tokens(&state.pool, &state.auth_cfg, user_id).await {
        Ok(tokens) => (StatusCode::OK, Json(tokens)).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> impl IntoResponse {
    // Simple Redis-backed rate limit (do not log credentials).
    let email_norm = req.email.trim().to_lowercase();
    let mut redis = state.redis.clone();
    let rl_key = format!("rate:login:{email_norm}");
    let current: i64 = redis.incr(&rl_key, 1).await.unwrap_or(1);
    if current == 1 {
        let _: () = redis.expire(&rl_key, 60).await.unwrap_or(());
    }
    if current > 10 {
        return util::api_error(ApiErrorCode::RateLimited);
    }

    let email = email_norm;
    let row = sqlx::query_as::<_, (Uuid, Option<String>)>(
        r#"
        select id, password_hash
        from users
        where email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(&state.pool)
    .await;

    let Some((user_id, password_hash)) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::Unauthenticated);
    };

    let Some(password_hash) = password_hash else {
        return util::api_error(ApiErrorCode::Unauthenticated);
    };

    let verified = auth::verify_password(&req.password, &password_hash).unwrap_or(false);
    if !verified {
        return util::api_error(ApiErrorCode::Unauthenticated);
    }

    match issue_tokens(&state.pool, &state.auth_cfg, user_id).await {
        Ok(tokens) => (StatusCode::OK, Json(tokens)).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> impl IntoResponse {
    let now = OffsetDateTime::now_utc();
    let token_hash = auth::hash_refresh_token(&state.auth_cfg, &req.refresh_token);

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let existing = sqlx::query(
        r#"select id, user_id, expires_at, revoked_at from refresh_tokens where token_hash = $1"#,
    )
    .bind(token_hash)
    .fetch_optional(&mut *tx)
    .await;

    let Some(existing) = (match existing {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::Unauthenticated);
    };

    let id: Uuid = existing.get("id");
    let user_id: Uuid = existing.get("user_id");
    let expires_at: OffsetDateTime = existing.get("expires_at");
    let revoked_at: Option<OffsetDateTime> = existing.get("revoked_at");

    if revoked_at.is_some() || expires_at <= now {
        return util::api_error(ApiErrorCode::Unauthenticated);
    }

    let revoked = sqlx::query(
        r#"
        update refresh_tokens
        set revoked_at = now()
        where id = $1 and revoked_at is null
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await;
    if revoked.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let (new_refresh_token, new_refresh_hash, new_expires_at) =
        new_refresh_token_values(&state.auth_cfg, now);
    let refresh_id = Uuid::now_v7();

    let inserted = sqlx::query(
        r#"
        insert into refresh_tokens (id, user_id, token_hash, expires_at)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(refresh_id)
    .bind(user_id)
    .bind(new_refresh_hash)
    .bind(new_expires_at)
    .execute(&mut *tx)
    .await;
    if inserted.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let access_token = match auth::issue_access_token(&state.auth_cfg, user_id) {
        Ok(t) => t,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    (
        StatusCode::OK,
        Json(AuthResponse {
            access_token,
            refresh_token: new_refresh_token,
        }),
    )
        .into_response()
}

async fn logout(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> impl IntoResponse {
    let token_hash = auth::hash_refresh_token(&state.auth_cfg, &req.refresh_token);
    let res = sqlx::query(
        r#"
        update refresh_tokens
        set revoked_at = now()
        where token_hash = $1 and revoked_at is null
        "#,
    )
    .bind(token_hash)
    .execute(&state.pool)
    .await;

    match res {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn me(
    State(state): State<AppState>,
    auth: Option<axum::Extension<AuthContext>>,
) -> impl IntoResponse {
    let Some(axum::Extension(auth)) = auth else {
        return util::api_error(ApiErrorCode::Unauthenticated);
    };

    let user = sqlx::query(
        r#"
        select id, email, display_name, created_at
        from users
        where id = $1
        "#,
    )
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(user) = (match user {
        Ok(u) => u,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::Unauthenticated);
    };

    (
        StatusCode::OK,
        Json(MeResponse {
            id: user.get("id"),
            email: user.get("email"),
            display_name: user.get("display_name"),
            created_at: user.get("created_at"),
        }),
    )
        .into_response()
}

async fn issue_tokens(
    pool: &PgPool,
    cfg: &auth::AuthConfig,
    user_id: Uuid,
) -> anyhow::Result<AuthResponse> {
    let now = OffsetDateTime::now_utc();
    let (refresh_token, refresh_hash, expires_at) = new_refresh_token_values(cfg, now);
    let refresh_id = Uuid::now_v7();

    sqlx::query(
        r#"
        insert into refresh_tokens (id, user_id, token_hash, expires_at)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(refresh_id)
    .bind(user_id)
    .bind(refresh_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    let access_token = auth::issue_access_token(cfg, user_id)?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
    })
}

fn new_refresh_token_values(
    cfg: &auth::AuthConfig,
    now: OffsetDateTime,
) -> (String, String, OffsetDateTime) {
    let refresh_token = auth::new_refresh_token();
    let refresh_hash = auth::hash_refresh_token(cfg, &refresh_token);
    let expires_at = now + cfg.refresh_ttl;
    (refresh_token, refresh_hash, expires_at)
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => db_err.code().as_deref() == Some("23505"),
        _ => false,
    }
}
