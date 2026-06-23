use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};
use redis::AsyncCommands;
use sqlx::PgPool;
use std::sync::Arc;
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
pub mod routes_lorelei;
pub mod routes_media;
pub mod routes_messages;
pub mod routes_notifications;
pub mod routes_orgs;
pub mod routes_threads;
pub mod util;
pub mod readiness;

/// Lorelei integration state. `None` when `LORELEI_HARBOR_URL`/`LORELEI_CREDENTIALS_KEY`
/// aren't set — every Lorelei-aware route treats that as "feature unavailable" rather than
/// erroring, so dev/test environments that don't care about Lorelei are unaffected.
pub struct LoreleiRuntime {
    pub harbor: lorelei_bridge::HarborClient,
    pub credentials_key: secrets::CredentialsKey,
}

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
    pub(crate) lorelei: Option<Arc<LoreleiRuntime>>,
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
            lorelei: None,
        }
    }

    /// Enables the Lorelei integration on an already-constructed state. Additive — does not
    /// change `new()`'s signature, so existing callers (including every integration test)
    /// are unaffected.
    pub fn with_lorelei(
        mut self,
        harbor: lorelei_bridge::HarborClient,
        credentials_key: secrets::CredentialsKey,
    ) -> Self {
        self.lorelei = Some(Arc::new(LoreleiRuntime {
            harbor,
            credentials_key,
        }));
        self
    }
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub(crate) user_id: Uuid,
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = api::ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let v = parts.extensions.get::<AuthContext>().cloned();
        std::future::ready(v.ok_or_else(|| api::ApiError::new(api::ApiErrorCode::Unauthenticated)))
    }
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
