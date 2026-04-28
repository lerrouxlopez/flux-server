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
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level};
use uuid::Uuid;

mod routes_auth;
mod routes_orgs;
mod routes_channels;
mod routes_messages;
mod util;

#[derive(Clone)]
pub(crate) struct AppState {
    pool: PgPool,
    redis: redis::aio::ConnectionManager,
    nats: async_nats::Client,
    auth_cfg: auth::AuthConfig,
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
    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;

    let redis_client = redis::Client::open(cfg.redis_url)?;
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
    };
    let auth_state = state.clone();

    let request_id_header = HeaderName::from_static("x-request-id");

    let app = Router::new()
        .nest("/auth", routes_auth::router())
        .nest("/orgs", routes_orgs::router())
        .merge(routes_channels::router())
        .merge(routes_messages::router())
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::new(request_id_header.clone(), MakeRequestUuid))
                .layer(PropagateRequestIdLayer::new(request_id_header))
                .layer(CatchPanicLayer::new())
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                        .on_response(DefaultOnResponse::new().level(Level::INFO)),
                )
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                )
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
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": "payload_too_large",
                "message": "Request body too large",
            })),
        )
            .into_response();
    }
    res
}
