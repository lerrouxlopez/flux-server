use axum::{extract::State, response::IntoResponse, routing::get, Router};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Clone)]
struct AppState {
    hub: realtime::Hub,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let cfg = config::AppConfig::from_env()?;
    let state = AppState {
        hub: realtime::Hub::new(),
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = cfg.ws_addr.parse()?;
    info!(%addr, "realtime-gateway listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(State(state): State<AppState>, ws: axum::extract::WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| realtime::ws::handle_socket(state.hub, socket))
}
