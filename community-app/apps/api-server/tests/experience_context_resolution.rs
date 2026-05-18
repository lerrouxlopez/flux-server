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

async fn get_context(app: &axum::Router, token: &str, org_id: &str, channel_id: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/experience/context?org_id={org_id}&channel_id={channel_id}"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn patch_pref(app: &axum::Router, token: &str, mode: Option<&str>) -> axum::http::StatusCode {
    let req = axum::http::Request::builder()
        .method("PATCH")
        .uri("/experience/preferences")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "mode_preference": mode
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    res.status()
}

#[tokio::test]
async fn resolution_order_user_over_channel_over_org() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let cfg = config::AppConfig::from_env().expect("config");
    let state = bootstrap_state(pool.clone()).await;
    let app = api_server::app::build_app(&cfg, state);

    let u1 = register(&app, &format!("u1+{}@example.com", Uuid::now_v7())).await;
    let t1 = u1.get("access_token").unwrap().as_str().unwrap();

    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = org.get("id").unwrap().as_str().unwrap();
    let channel = create_channel(&app, t1, org_id, "general").await;
    let channel_id = channel.get("id").unwrap().as_str().unwrap();

    // Set org default to play, channel hint to work.
    sqlx::query(r#"update organizations set experience_default_mode = 'play' where id = $1"#)
        .bind(Uuid::parse_str(org_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"update channels set experience_mode_hint = 'work' where id = $1"#)
        .bind(Uuid::parse_str(channel_id).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // No user pref -> channel hint wins.
    let ctx = get_context(&app, t1, org_id, channel_id).await;
    assert_eq!(ctx.get("mode").and_then(|v| v.as_str()), Some("work"));
    assert_eq!(ctx.get("source").and_then(|v| v.as_str()), Some("channel_hint"));

    // User pref -> wins over channel.
    assert_eq!(patch_pref(&app, t1, Some("play")).await, axum::http::StatusCode::OK);
    let ctx = get_context(&app, t1, org_id, channel_id).await;
    assert_eq!(ctx.get("mode").and_then(|v| v.as_str()), Some("play"));
    assert_eq!(ctx.get("source").and_then(|v| v.as_str()), Some("user_preference"));

    // Clear user pref -> channel hint wins again.
    assert_eq!(patch_pref(&app, t1, None).await, axum::http::StatusCode::OK);
    let ctx = get_context(&app, t1, org_id, channel_id).await;
    assert_eq!(ctx.get("mode").and_then(|v| v.as_str()), Some("work"));
    assert_eq!(ctx.get("source").and_then(|v| v.as_str()), Some("channel_hint"));
}

#[tokio::test]
async fn tenant_isolation_for_context() {
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

    let ch2 = create_channel(&app, t2, org2.get("id").unwrap().as_str().unwrap(), "private").await;
    let org2_id = org2.get("id").unwrap().as_str().unwrap();
    let ch2_id = ch2.get("id").unwrap().as_str().unwrap();

    // User1 tries to query org2 context -> forbidden.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/experience/context?org_id={org2_id}&channel_id={ch2_id}"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    let _ = org1;
}

