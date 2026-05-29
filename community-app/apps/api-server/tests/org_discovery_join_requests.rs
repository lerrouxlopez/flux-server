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

async fn create_org(app: &axum::Router, token: &str, name: &str, slug: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/orgs")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "name": name,
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

async fn patch_discovery(app: &axum::Router, token: &str, org_id: &str, discoverable: bool, join_policy: &str) {
    let req = axum::http::Request::builder()
        .method("PATCH")
        .uri(format!("/orgs/{org_id}/discovery-settings"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "discoverable": discoverable,
                "join_policy": join_policy,
                "member_count_visible": true,
                "online_count_visible": false,
                "tags": ["test"]
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
}

async fn discover(app: &axum::Router, token: &str, q: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/orgs/discover?q={q}&limit=50"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn join_open(app: &axum::Router, token: &str, org_id: &str) {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/join"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
}

async fn request_join(app: &axum::Router, token: &str, org_id: &str, msg: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/join-requests"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::json!({ "message": msg }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn list_requests(app: &axum::Router, token: &str, org_id: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/orgs/{org_id}/join-requests"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn approve_request(app: &axum::Router, token: &str, org_id: &str, request_id: &str) {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/join-requests/{request_id}/approve"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
}

#[tokio::test]
async fn discover_open_join_and_request_access_flow() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let cfg = config::AppConfig::from_env().expect("config");
    let state = bootstrap_state(pool).await;
    let app = api_server::app::build_app(&cfg, state);

    let u1 = register(&app, &format!("owner+{}@example.com", Uuid::now_v7())).await;
    let u2 = register(&app, &format!("member+{}@example.com", Uuid::now_v7())).await;
    let t1 = u1.get("access_token").unwrap().as_str().unwrap();
    let t2 = u2.get("access_token").unwrap().as_str().unwrap();

    // Open org: discover + join
    let slug_open = format!("nova{}", Uuid::now_v7().simple());
    let org_open = create_org(&app, t1, "Nova Labs", &slug_open).await;
    let org_open_id = org_open.get("id").unwrap().as_str().unwrap();
    patch_discovery(&app, t1, org_open_id, true, "open").await;

    let d = discover(&app, t2, "Nova").await;
    let orgs = d.get("organizations").unwrap().as_array().unwrap();
    assert!(orgs.iter().any(|o| o.get("slug").unwrap().as_str().unwrap() == slug_open));

    join_open(&app, t2, org_open_id).await;
    let d2 = discover(&app, t2, "Nova").await;
    let orgs2 = d2.get("organizations").unwrap().as_array().unwrap();
    let status = orgs2
        .iter()
        .find(|o| o.get("slug").unwrap().as_str().unwrap() == slug_open)
        .unwrap()
        .get("current_user_status")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(status, "member");

    // Request org: discover + request + approve
    let slug_req = format!("arcade{}", Uuid::now_v7().simple());
    let org_req = create_org(&app, t1, "Arcade Ops", &slug_req).await;
    let org_req_id = org_req.get("id").unwrap().as_str().unwrap();
    patch_discovery(&app, t1, org_req_id, true, "request").await;

    let _req = request_join(&app, t2, org_req_id, "please").await;
    let d3 = discover(&app, t2, "Arcade").await;
    let orgs3 = d3.get("organizations").unwrap().as_array().unwrap();
    let status3 = orgs3
        .iter()
        .find(|o| o.get("slug").unwrap().as_str().unwrap() == slug_req)
        .unwrap()
        .get("current_user_status")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(status3, "pending_request");

    let list = list_requests(&app, t1, org_req_id).await;
    let requests = list.get("requests").unwrap().as_array().unwrap();
    let rid = requests
        .iter()
        .find(|r| r.get("status").unwrap().as_str().unwrap() == "pending")
        .unwrap()
        .get("id")
        .unwrap()
        .as_str()
        .unwrap();
    approve_request(&app, t1, org_req_id, rid).await;

    let d4 = discover(&app, t2, "Arcade").await;
    let orgs4 = d4.get("organizations").unwrap().as_array().unwrap();
    let status4 = orgs4
        .iter()
        .find(|o| o.get("slug").unwrap().as_str().unwrap() == slug_req)
        .unwrap()
        .get("current_user_status")
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(status4, "member");

    // Closed org should not leak via discovery (non-member)
    let slug_closed = format!("secret{}", Uuid::now_v7().simple());
    let org_closed = create_org(&app, t1, "Secret Org", &slug_closed).await;
    let org_closed_id = org_closed.get("id").unwrap().as_str().unwrap();
    patch_discovery(&app, t1, org_closed_id, true, "closed").await;
    let d5 = discover(&app, t2, "Secret").await;
    let orgs5 = d5.get("organizations").unwrap().as_array().unwrap();
    assert!(!orgs5.iter().any(|o| o.get("slug").unwrap().as_str().unwrap() == slug_closed));
}

