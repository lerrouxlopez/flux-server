mod app;
mod controllers;
mod models;
mod repositories;
mod services;
mod state;

use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    telemetry::init();

    let database_url = config::required("DATABASE_URL")?;
    let http_addr = config::parse_socket_addr("HTTP_ADDR")?
        .unwrap_or_else(|| "0.0.0.0:8080".parse().expect("valid default addr"));

    let pool = db::connect(&database_url).await?;
    db::migrate(&pool).await?;

    let state = state::AppState::new(pool);
    let app = app::router(state);

    info!(%http_addr, "api-server listening");

    let listener = tokio::net::TcpListener::bind(http_addr).await?;
    if let Err(err) = axum::serve(listener, app).await {
        error!(error = %err, "server exited with error");
    }

    Ok(())
}
