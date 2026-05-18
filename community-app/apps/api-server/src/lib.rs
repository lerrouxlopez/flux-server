use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;

pub mod app;
pub mod attachments_storage;
pub mod routes_audit;
pub mod routes_experience;
pub mod routes_attachments;
pub mod routes_auth;
pub mod routes_branding;
pub mod routes_channels;
pub mod routes_dms;
pub mod routes_friends;
pub mod routes_media;
pub mod routes_messages;
pub mod routes_notifications;
pub mod routes_orgs;
pub mod routes_threads;
pub mod util;
pub mod readiness;

#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) redis: redis::aio::ConnectionManager,
    pub(crate) nats: async_nats::Client,
    pub(crate) auth_cfg: auth::AuthConfig,
    pub(crate) livekit_url_internal: String,
    pub(crate) livekit_url_public: String,
    pub(crate) livekit_api_key: String,
    pub(crate) livekit_api_secret: String,
}

impl AppState {
    pub fn new(
        pool: PgPool,
        redis: redis::aio::ConnectionManager,
        nats: async_nats::Client,
        auth_cfg: auth::AuthConfig,
        livekit_url_internal: String,
        livekit_url_public: String,
        livekit_api_key: String,
        livekit_api_secret: String,
    ) -> Self {
        Self {
            pool,
            redis,
            nats,
            auth_cfg,
            livekit_url_internal,
            livekit_url_public,
            livekit_api_key,
            livekit_api_secret,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub(crate) user_id: Uuid,
}

pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let mut problems: Vec<&'static str> = Vec::new();

    // Postgres
    if !crate::readiness::check_postgres(&state.pool).await {
        problems.push("postgres");
    }

    // Redis
    let mut redis = state.redis.clone();
    let redis_ok: Result<String, _> = redis.ping().await;
    if redis_ok.is_err() {
        problems.push("redis");
    }

    // NATS
    if state.nats.flush().await.is_err() {
        problems.push("nats");
    }

    // LiveKit RoomService (Twirp)
    let lk = media::LiveKitConfig {
        internal_url: state.livekit_url_internal.clone(),
        public_url: state.livekit_url_public.clone(),
        api_key: state.livekit_api_key.clone(),
        api_secret: state.livekit_api_secret.clone(),
    };
    if !crate::readiness::check_livekit_roomservice(&lk, std::time::Duration::from_secs(2)).await {
        problems.push("livekit");
    }

    if problems.is_empty() {
        (StatusCode::OK, "ready").into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not_ready",
                "problems": problems,
            })),
        )
            .into_response()
    }
}
