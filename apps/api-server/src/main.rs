mod app;
mod controllers;
mod models;
mod repositories;
mod services;
mod state;

use std::{env, net::SocketAddr};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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

