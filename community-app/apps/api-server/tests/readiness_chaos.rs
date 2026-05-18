use std::net::SocketAddr;
use std::time::Duration;

#[tokio::test]
async fn chaos_redis_unreachable_fails_fast() {
    let ok = api_server::readiness::check_redis_url("redis://127.0.0.1:1", Duration::from_millis(200)).await;
    assert!(!ok);
}

#[tokio::test]
async fn chaos_nats_unreachable_fails_fast() {
    let ok = api_server::readiness::check_nats_url("nats://127.0.0.1:1", Duration::from_millis(200)).await;
    assert!(!ok);
}

async fn serve_livekit_stub(status: axum::http::StatusCode) -> SocketAddr {
    let app = axum::Router::new().route(
        "/twirp/livekit.RoomService/ListRooms",
        axum::routing::post(move || async move { status }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

#[tokio::test]
async fn chaos_livekit_roomservice_failure_detected() {
    let addr = serve_livekit_stub(axum::http::StatusCode::INTERNAL_SERVER_ERROR).await;
    let cfg = media::LiveKitConfig {
        internal_url: format!("http://{addr}"),
        public_url: format!("http://{addr}"),
        api_key: "test".to_string(),
        api_secret: "test".to_string(),
    };
    let ok = api_server::readiness::check_livekit_roomservice(&cfg, Duration::from_millis(300)).await;
    assert!(!ok);
}

#[tokio::test]
async fn chaos_livekit_roomservice_success_detected() {
    let addr = serve_livekit_stub(axum::http::StatusCode::OK).await;
    let cfg = media::LiveKitConfig {
        internal_url: format!("http://{addr}"),
        public_url: format!("http://{addr}"),
        api_key: "test".to_string(),
        api_secret: "test".to_string(),
    };
    let ok = api_server::readiness::check_livekit_roomservice(&cfg, Duration::from_millis(300)).await;
    assert!(ok);
}

