use crate::Hub;
use axum::extract::ws::{Message, WebSocket};
use futures_util::stream::StreamExt;
use tracing::info;

pub async fn handle_socket(hub: Hub, mut socket: WebSocket) {
    info!("ws connected");
    let mut rx = hub.sender().subscribe();

    loop {
        tokio::select! {
            inbound = socket.next() => {
                match inbound {
                    Some(Ok(Message::Text(txt))) => {
                        // For now, echo to the client (placeholder for authenticated org-scoped events).
                        if socket.send(Message::Text(txt)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            outbound = rx.recv() => {
                if let Ok(evt) = outbound {
                    let payload = evt.to_string();
                    if socket.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}
