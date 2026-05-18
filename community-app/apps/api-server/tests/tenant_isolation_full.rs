use axum::body::Body;
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
}

async fn setup_pool() -> PgPool {
    let url =
        test_database_url().expect("set TEST_DATABASE_URL (or DATABASE_URL) for integration tests");
    let pool = db::connect(&url).await.expect("connect db");
    db::migrate(&pool).await.expect("migrate");
    pool
}

async fn bootstrap_state(pool: PgPool) -> api_server::AppState {
    let cfg = config::AppConfig::from_env().expect("config");
    let redis_client = redis::Client::open(cfg.redis_url.clone()).expect("redis");
    let redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("redis mgr");
    let nats = events::connect(&cfg.nats_url).await.expect("nats");

    api_server::AppState::new(
        pool,
        redis,
        nats,
        auth::AuthConfig {
            jwt_access_secret: cfg.jwt_access_secret.clone(),
            jwt_refresh_secret: cfg.jwt_refresh_secret.clone(),
            access_ttl: time::Duration::seconds(cfg.access_token_ttl_seconds as i64),
            refresh_ttl: time::Duration::seconds(cfg.refresh_token_ttl_seconds as i64),
        },
        cfg.livekit_url_internal.clone(),
        cfg.livekit_url_public.clone(),
        cfg.livekit_api_key.clone(),
        cfg.livekit_api_secret.clone(),
    )
}

async fn register(app: &axum::Router, email: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "email": email,
                "display_name": "Test",
                "password": "password123"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn create_org(app: &axum::Router, token: &str, slug: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/orgs")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "name": "Acme",
                "slug": slug
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn create_channel(app: &axum::Router, token: &str, org_id: &str, name: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/channels"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "name": name,
                "kind": "text"
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn send_message(app: &axum::Router, token: &str, channel_id: &str, body: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/channels/{channel_id}/messages"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "body": body
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn cross_tenant_denies_messages_branding_search_media() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let cfg = config::AppConfig::from_env().expect("config");
    let state = bootstrap_state(pool).await;
    let app = api_server::app::build_app(&cfg, state);

    let u1 = register(&app, &format!("u1+{}@example.com", Uuid::now_v7())).await;
    let u2 = register(&app, &format!("u2+{}@example.com", Uuid::now_v7())).await;
    let t1 = u1.get("access_token").unwrap().as_str().unwrap();
    let t2 = u2.get("access_token").unwrap().as_str().unwrap();

    let org1 = create_org(&app, t1, &format!("org{}", Uuid::now_v7().simple())).await;
    let org2 = create_org(&app, t2, &format!("org{}", Uuid::now_v7().simple())).await;
    let org2_id = org2.get("id").unwrap().as_str().unwrap();

    let ch2 = create_channel(&app, t2, org2_id, "general").await;
    let ch2_id = ch2.get("id").unwrap().as_str().unwrap();
    let msg = send_message(&app, t2, ch2_id, "secret from org2").await;
    let msg_id = msg.get("id").unwrap().as_str().unwrap();

    // Messages list (channel scoped)
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{ch2_id}/messages"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Search (channel scoped)
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{ch2_id}/search?q=secret"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Branding profile
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/orgs/{org2_id}/branding"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Message edit by id (should also be forbidden)
    let req = axum::http::Request::builder()
        .method("PATCH")
        .uri(format!("/messages/{msg_id}"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(serde_json::json!({ "body": "hacked" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Media room creation in org2
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org2_id}/media/rooms"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::from(
            serde_json::json!({
                "kind": "voice",
                "channel_id": ch2_id,
                "name": "Secret Room"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let room: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let room_id = room.get("id").unwrap().as_str().unwrap();

    // Media room read by user1 -> forbidden
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/media/rooms/{room_id}"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Media participants list by user1 -> forbidden (no LiveKit call should be reached)
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/media/rooms/{room_id}/participants"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Attempt cross-org channel_id injection when creating a media room in org1 (should be forbidden)
    let org1_id = org1.get("id").unwrap().as_str().unwrap();
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org1_id}/media/rooms"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(
            serde_json::json!({
                "kind": "voice",
                "channel_id": ch2_id,
                "name": "Injected Room"
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}

