use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let ws_addr = config::parse_socket_addr("WS_ADDR")?
        .unwrap_or_else(|| "0.0.0.0:8081".parse().expect("valid default addr"));

    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws))
        .layer(TraceLayer::new_for_http());

    info!(%ws_addr, "realtime-gateway listening");

    let listener = tokio::net::TcpListener::bind(ws_addr).await?;
    if let Err(err) = axum::serve(listener, app).await {
        error!(error = %err, "server exited with error");
    }

    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn ws(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let _ = socket.send(Message::Text("connected".into())).await;
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(t) if t == "ping" => {
                let _ = socket.send(Message::Text("pong".into())).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
