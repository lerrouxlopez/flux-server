use axum::{
    extract::State,
    http::{HeaderMap, HeaderName, StatusCode},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use redis::AsyncCommands;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info, Span};
use uuid::Uuid;
use api::{ApiError, ApiErrorCode};

mod routes_auth;
mod routes_orgs;
mod routes_channels;
mod routes_messages;
mod routes_media;
mod routes_branding;
mod util;

#[derive(Clone)]
pub(crate) struct AppState {
    pool: PgPool,
    redis: redis::aio::ConnectionManager,
    nats: async_nats::Client,
    auth_cfg: auth::AuthConfig,
    livekit_url: String,
    livekit_api_key: String,
    livekit_api_secret: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthContext {
    user_id: Uuid,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let cfg = config::AppConfig::from_env()?;
    let cors = cors_layer(&cfg);
    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;

    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;

    let nats = events::connect(&cfg.nats_url).await?;

    let state = AppState {
        pool,
        redis,
        nats,
        auth_cfg: auth::AuthConfig {
            jwt_access_secret: cfg.jwt_access_secret.clone(),
            jwt_refresh_secret: cfg.jwt_refresh_secret.clone(),
            access_ttl: time::Duration::seconds(cfg.access_token_ttl_seconds as i64),
            refresh_ttl: time::Duration::seconds(cfg.refresh_token_ttl_seconds as i64),
        },
        livekit_url: cfg.livekit_url.clone(),
        livekit_api_key: cfg.livekit_api_key.clone(),
        livekit_api_secret: cfg.livekit_api_secret.clone(),
    };
    let auth_state = state.clone();

    let request_id_header = HeaderName::from_static("x-request-id");

    let app = Router::new()
        .nest("/auth", routes_auth::router())
        .nest("/orgs", routes_orgs::router())
        .merge(routes_channels::router())
        .merge(routes_messages::router())
        .merge(routes_media::router())
        .merge(routes_branding::router())
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::new(request_id_header.clone(), MakeRequestUuid))
                .layer(PropagateRequestIdLayer::new(request_id_header))
                .layer(CatchPanicLayer::new())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|req: &axum::http::Request<_>| {
                            let request_id = req
                                .headers()
                                .get("x-request-id")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("");
                            tracing::info_span!(
                                "http.request",
                                request_id = %request_id,
                                method = %req.method(),
                                path = %req.uri().path(),
                                status = tracing::field::Empty,
                                latency_ms = tracing::field::Empty,
                                user_id = tracing::field::Empty,
                                organization_id = tracing::field::Empty,
                            )
                        })
                        .on_response(|res: &axum::http::Response<_>, latency: std::time::Duration, span: &Span| {
                            span.record("status", res.status().as_u16());
                            span.record("latency_ms", latency.as_millis() as u64);
                            tracing::info!(
                                parent: span,
                                status = %res.status().as_u16(),
                                latency_ms = %(latency.as_millis() as u64),
                                "request finished"
                            );
                        }),
                )
                .layer(cors)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024)) // 1 MiB default
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    std::time::Duration::from_secs(10),
                ))
                .into_inner(),
        )
        .layer(middleware::from_fn_with_state(auth_state, auth_middleware))
        .layer(middleware::from_fn(error_middleware))
        .with_state(state);

    let addr: SocketAddr = cfg.http_addr.parse()?;
    info!(%addr, "api-server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let mut problems: Vec<&'static str> = Vec::new();

    // Postgres
    if sqlx::query_scalar::<_, i64>("select 1")
        .fetch_one(&state.pool)
        .await
        .is_err()
    {
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

async fn auth_middleware(
    State(state): State<AppState>,
    mut req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Response {
    if let Some(auth) = bearer_token(req.headers()) {
        if let Ok(claims) = auth::decode_access_token(&state.auth_cfg, auth) {
            if let Ok(user_id) = Uuid::parse_str(&claims.sub) {
                req.extensions_mut().insert(AuthContext { user_id });
                Span::current().record("user_id", tracing::field::display(user_id));
            }
        }
    }
    next.run(req).await
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").or_else(|| value.strip_prefix("bearer "))
}

async fn error_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Response {
    let res = next.run(req).await;
    if res.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return ApiError::with_message(
            ApiErrorCode::ValidationError,
            "Request body too large.",
        )
        .into_response();
    }
    res
}

fn cors_layer(cfg: &config::AppConfig) -> CorsLayer {
    // Allowlist via env: CORS_ALLOW_ORIGINS="https://a.com,https://b.com"
    // If unset in local/dev, allow any.
    let raw = std::env::var("CORS_ALLOW_ORIGINS").unwrap_or_default();
    if raw.trim().is_empty() || cfg.app_env == "local" {
        return CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    }

    let mut origins = Vec::new();
    for part in raw.split(',') {
        let o = part.trim();
        if o.is_empty() {
            continue;
        }
        if let Ok(v) = o.parse::<axum::http::HeaderValue>() {
            origins.push(v);
        }
    }

    if origins.is_empty() {
        CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)
    } else {
        CorsLayer::new().allow_origin(origins).allow_methods(Any).allow_headers(Any)
    }
}
