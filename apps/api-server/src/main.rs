use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use std::{env, net::SocketAddr};
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set (see .env.example)");
    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    let pool = db::connect(&database_url).await?;
    db::migrate(&pool).await?;

    let app = Router::new()
        .route("/health", get(health))
        .with_state(AppState { pool })
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!(%addr, "api-server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    if let Err(err) = axum::serve(listener, app).await {
        error!(error = %err, "server exited with error");
    }

    Ok(())
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    db::ping(&state.pool).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    Ok(Json(HealthResponse { status: "ok" }))
}
