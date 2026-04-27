mod app;
mod controllers;
mod models;
mod repositories;
mod services;
mod state;

use std::net::SocketAddr;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    telemetry::init();

    let database_url = config::required("DATABASE_URL")?;
    let port: u16 = config::parse("PORT")?.unwrap_or(3000);

    let pool = db::connect(&database_url).await?;
    db::migrate(&pool).await?;

    let state = state::AppState::new(pool);
    let app = app::router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!(%addr, "api-server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    if let Err(err) = axum::serve(listener, app).await {
        error!(error = %err, "server exited with error");
    }

    Ok(())
}
