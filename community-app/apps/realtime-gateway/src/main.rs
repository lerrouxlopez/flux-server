use api::{ApiError, ApiErrorCode};
use axum::{
    extract::{Query, State, WebSocketUpgrade},
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
    Router,
};
use sqlx::PgPool;
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;

mod protocol;
mod runtime;

#[derive(Clone)]
struct AppState {
    app_env: String,
    pool: PgPool,
    redis: redis::aio::ConnectionManager,
    nats: async_nats::Client,
    auth_cfg: auth::AuthConfig,
    rt: Arc<runtime::Runtime>,
}

#[derive(serde::Deserialize)]
struct WsQuery {
    access_token: Option<String>,
    token: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let cfg = config::AppConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;

    let redis_client = redis::Client::open(cfg.redis_url)?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;

    let nats = events::connect(&cfg.nats_url).await?;

    let rt = Arc::new(runtime::Runtime::new(nats.clone(), pool.clone()));
    rt.spawn_nats_fanout();

    let state = AppState {
        app_env: cfg.app_env.clone(),
        pool,
        redis,
        nats,
        auth_cfg: auth::AuthConfig {
            jwt_access_secret: cfg.jwt_access_secret.clone(),
            jwt_refresh_secret: cfg.jwt_refresh_secret.clone(),
            access_ttl: time::Duration::seconds(cfg.access_token_ttl_seconds as i64),
            refresh_ttl: time::Duration::seconds(cfg.refresh_token_ttl_seconds as i64),
        },
        rt,
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/realtime/ws", get(ws_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = cfg.ws_addr.parse()?;
    info!(%addr, "realtime-gateway listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let token = bearer_token(&headers).map(|s| s.to_string()).or_else(|| {
        if state.app_env == "local" {
            q.access_token.clone().or(q.token.clone())
        } else {
            None
        }
    });

    let Some(token) = token else {
        return ApiError::new(ApiErrorCode::Unauthenticated).into_response();
    };

    let claims = match auth::decode_access_token(&state.auth_cfg, &token) {
        Ok(c) => c,
        Err(_) => {
            return ApiError::new(ApiErrorCode::Unauthenticated).into_response();
        }
    };
    let Ok(user_id) = Uuid::parse_str(&claims.sub) else {
        return ApiError::new(ApiErrorCode::Unauthenticated).into_response();
    };

    let org_ids = match load_memberships(&state.pool, user_id).await {
        Ok(o) => o,
        Err(res) => return res,
    };
    if org_ids.is_empty() {
        return ApiError::new(ApiErrorCode::PermissionDenied).into_response();
    }

    ws.on_upgrade(move |socket| async move {
        if let Err(err) = state
            .rt
            .handle_socket(
                runtime::SocketContext {
                    user_id,
                    org_ids,
                    redis: state.redis,
                    nats: state.nats,
                    pool: state.pool,
                },
                socket,
            )
            .await
        {
            warn!(?err, "ws session ended with error");
        }
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
}

async fn load_memberships(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Uuid>, axum::response::Response> {
    let rows = sqlx::query_scalar::<_, Uuid>(
        r#"
        select organization_id
        from organization_members
        where user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await;

    match rows {
        Ok(r) => Ok(r),
        Err(_) => Err(ApiError::new(ApiErrorCode::InternalError).into_response()),
    }
}
