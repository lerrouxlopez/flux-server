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

async fn create_channel(app: &axum::Router, token: &str, org_id: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/channels"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "name": "private-test",
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

#[tokio::test]
async fn unauthorized_user_cannot_access_other_org_channel() {
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

    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = org.get("id").unwrap().as_str().unwrap();
    let ch = create_channel(&app, t1, org_id).await;
    let channel_id = ch.get("id").unwrap().as_str().unwrap();

    // u2 tries to list messages for org1's channel -> 403
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{channel_id}/messages?limit=1"))
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();

    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}
