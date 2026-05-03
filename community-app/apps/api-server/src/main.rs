use api_server::AppState;
use std::net::SocketAddr;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let cfg = config::AppConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;

    let redis_client = redis::Client::open(cfg.redis_url.clone())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;

    let nats = events::connect(&cfg.nats_url).await?;

    let state = AppState::new(
        pool,
        redis,
        nats,
        auth::AuthConfig {
            jwt_access_secret: cfg.jwt_access_secret.clone(),
            jwt_refresh_secret: cfg.jwt_refresh_secret.clone(),
            access_ttl: time::Duration::seconds(cfg.access_token_ttl_seconds as i64),
            refresh_ttl: time::Duration::seconds(cfg.refresh_token_ttl_seconds as i64),
        },
        cfg.livekit_url.clone(),
        cfg.livekit_api_key.clone(),
        cfg.livekit_api_secret.clone(),
    );
    let auth_state = state.clone();
    let _ = auth_state;

    let app = api_server::app::build_app(&cfg, state);

    let addr: SocketAddr = cfg.http_addr.parse()?;
    info!(%addr, "api-server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
