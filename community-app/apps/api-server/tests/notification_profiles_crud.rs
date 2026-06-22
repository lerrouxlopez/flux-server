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

async fn list_profiles(app: &axum::Router, token: &str, org_id: &str, mode: &str) -> (axum::http::StatusCode, serde_json::Value) {
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/orgs/{org_id}/notification-profiles?mode={mode}"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn create_profile(
    app: &axum::Router,
    token: &str,
    org_id: &str,
    mode: &str,
    label: &str,
) -> (axum::http::StatusCode, serde_json::Value) {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/notification-profiles"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "mode": mode,
                "label": label,
                "description": "Custom profile",
                "rules": {
                    "message_all": { "in_app": true, "desktop": false, "sound": false },
                    "message_mentions": { "in_app": true, "desktop": true, "sound": true }
                }
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
}

async fn patch_profile(app: &axum::Router, token: &str, profile_id: &str, label: &str) -> axum::http::StatusCode {
    let req = axum::http::Request::builder()
        .method("PATCH")
        .uri(format!("/notification-profiles/{profile_id}"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::json!({ "label": label }).to_string()))
        .unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

async fn delete_profile(app: &axum::Router, token: &str, profile_id: &str) -> axum::http::StatusCode {
    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/notification-profiles/{profile_id}"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

async fn patch_user_override_full(
    app: &axum::Router,
    token: &str,
    org_id: &str,
    mode: &str,
    profile_id: Option<&str>,
    quiet_hours_enabled: bool,
    quiet_from: Option<&str>,
    quiet_to: Option<&str>,
) -> axum::http::StatusCode {
    let req = axum::http::Request::builder()
        .method("PATCH")
        .uri("/notifications/overrides/user")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "org_id": org_id,
                "mode": mode,
                "profile_id": profile_id,
                "quiet_hours_enabled": quiet_hours_enabled,
                "quiet_from": quiet_from,
                "quiet_to": quiet_to,
                "quiet_priority_override": true
            })
            .to_string(),
        ))
        .unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

async fn get_ctx(app: &axum::Router, token: &str, org_id: &str, mode: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/notifications/context?org_id={org_id}&mode={mode}"))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn list_includes_platform_defaults_and_create_then_list_shows_org_profile() {
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

    let (status, list) = list_profiles(&app, t1, org_id, "work").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let labels: Vec<&str> = list.as_array().unwrap().iter()
        .filter_map(|p| p.get("label").and_then(|v| v.as_str()))
        .collect();
    assert!(labels.contains(&"Work (Default)"));

    let (status, created) = create_profile(&app, t1, org_id, "work", "My Custom").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let profile_id = created.get("id").unwrap().as_str().unwrap().to_string();
    assert!(created.get("created_by").and_then(|v| v.as_str()).is_some());

    let (status, list2) = list_profiles(&app, t1, org_id, "work").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let ids: Vec<&str> = list2.as_array().unwrap().iter()
        .filter_map(|p| p.get("id").and_then(|v| v.as_str()))
        .collect();
    assert!(ids.contains(&profile_id.as_str()));
}

#[tokio::test]
async fn platform_profiles_are_protected_and_only_owners_can_mutate() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let cfg = config::AppConfig::from_env().expect("config");
    let state = bootstrap_state(pool.clone()).await;
    let app = api_server::app::build_app(&cfg, state);

    let u1 = register(&app, &format!("u1+{}@example.com", Uuid::now_v7())).await;
    let u2 = register(&app, &format!("u2+{}@example.com", Uuid::now_v7())).await;
    let t1 = u1.get("access_token").unwrap().as_str().unwrap();
    let t2 = u2.get("access_token").unwrap().as_str().unwrap();
    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = org.get("id").unwrap().as_str().unwrap();

    // Platform profile cannot be mutated by anyone.
    let platform_id = "11111111-1111-1111-1111-111111111111";
    assert_eq!(
        patch_profile(&app, t1, platform_id, "Hacked").await,
        axum::http::StatusCode::FORBIDDEN
    );
    assert_eq!(
        delete_profile(&app, t1, platform_id).await,
        axum::http::StatusCode::FORBIDDEN
    );

    // Owner (org creator) can create + edit + delete their own profile.
    let (_, created) = create_profile(&app, t1, org_id, "work", "Owner Custom").await;
    let profile_id = created.get("id").unwrap().as_str().unwrap();
    assert_eq!(
        patch_profile(&app, t1, profile_id, "Owner Custom Renamed").await,
        axum::http::StatusCode::OK
    );

    // u2 is not a member of this org and not the creator -> forbidden.
    assert_eq!(
        patch_profile(&app, t2, profile_id, "Hijacked").await,
        axum::http::StatusCode::FORBIDDEN
    );

    assert_eq!(
        delete_profile(&app, t1, profile_id).await,
        axum::http::StatusCode::OK
    );
}

#[tokio::test]
async fn quiet_hours_round_trip_through_override_and_context() {
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

    let ctx0 = get_ctx(&app, t1, org_id, "play").await;
    assert_eq!(
        ctx0.get("quiet_hours").and_then(|q| q.get("enabled")).and_then(|v| v.as_bool()),
        Some(false)
    );

    assert_eq!(
        patch_user_override_full(&app, t1, org_id, "play", None, true, Some("22:00"), Some("08:00")).await,
        axum::http::StatusCode::OK
    );

    let ctx1 = get_ctx(&app, t1, org_id, "play").await;
    let qh = ctx1.get("quiet_hours").unwrap();
    assert_eq!(qh.get("enabled").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(qh.get("from").and_then(|v| v.as_str()), Some("22:00"));
    assert_eq!(qh.get("to").and_then(|v| v.as_str()), Some("08:00"));
}
