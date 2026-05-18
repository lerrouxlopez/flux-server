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

async fn create_channel(
    app: &axum::Router,
    token: &str,
    org_id: &str,
    name: &str,
) -> serde_json::Value {
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

async fn get_ctx(
    app: &axum::Router,
    token: &str,
    org_id: &str,
    channel_id: &str,
) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!(
            "/notifications/context?org_id={org_id}&channel_id={channel_id}"
        ))
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn patch_user_override(
    app: &axum::Router,
    token: &str,
    org_id: &str,
    mode: &str,
    profile_id: Option<&str>,
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
                "profile_id": profile_id
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    res.status()
}

async fn patch_channel_override(
    app: &axum::Router,
    token: &str,
    org_id: &str,
    channel_id: &str,
    profile_id: Option<&str>,
) -> axum::http::StatusCode {
    let req = axum::http::Request::builder()
        .method("PATCH")
        .uri("/notifications/overrides/channel")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "org_id": org_id,
                "channel_id": channel_id,
                "profile_id": profile_id
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    res.status()
}

async fn insert_profile(
    pool: &PgPool,
    org_id: Uuid,
    mode: &str,
    label: &str,
    rules: &[(&str, bool)],
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        r#"
        insert into notification_profiles (id, organization_id, scope, mode, label)
        values ($1, $2, 'org', $3, $4)
        "#,
    )
    .bind(id)
    .bind(org_id)
    .bind(mode)
    .bind(label)
    .execute(pool)
    .await
    .unwrap();

    for (rule, enabled) in rules {
        sqlx::query(
            r#"
            insert into notification_profile_rules (profile_id, rule, enabled)
            values ($1, $2, $3)
            on conflict (profile_id, rule) do update set enabled = excluded.enabled
            "#,
        )
        .bind(id)
        .bind(*rule)
        .bind(*enabled)
        .execute(pool)
        .await
        .unwrap();
    }

    id
}

#[tokio::test]
async fn resolution_order_user_over_channel_over_mode_over_org_over_platform() {
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
    let org_id_s = org.get("id").unwrap().as_str().unwrap();
    let org_id = Uuid::parse_str(org_id_s).unwrap();
    let channel = create_channel(&app, t1, org_id_s, "general").await;
    let channel_id_s = channel.get("id").unwrap().as_str().unwrap();

    // Ensure experience mode is work (default) so platform profile is Work Default.
    let ctx0 = get_ctx(&app, t1, org_id_s, channel_id_s).await;
    assert_eq!(
        ctx0.get("profile_source").and_then(|v| v.as_str()),
        Some("platform_default")
    );
    assert_eq!(
        ctx0.get("behavior")
            .and_then(|b| b.get("message_all"))
            .and_then(|v| v.as_bool()),
        Some(false)
    );

    // Org default profile (distinct).
    let org_default = insert_profile(
        &pool,
        org_id,
        "work",
        "Org Default",
        &[("message_mentions", false), ("thread_replies", false)],
    )
    .await;
    sqlx::query(
        r#"update organizations set notification_default_profile_id = $2 where id = $1"#,
    )
    .bind(org_id)
    .bind(org_default)
    .execute(&pool)
    .await
    .unwrap();
    let ctx1 = get_ctx(&app, t1, org_id_s, channel_id_s).await;
    assert_eq!(
        ctx1.get("profile_source").and_then(|v| v.as_str()),
        Some("org_default")
    );
    assert_eq!(
        ctx1.get("behavior")
            .and_then(|b| b.get("message_mentions"))
            .and_then(|v| v.as_bool()),
        Some(false)
    );

    // Mode profile wins over org default.
    let mode_profile = insert_profile(
        &pool,
        org_id,
        "work",
        "Mode Profile",
        &[("message_mentions", true), ("pin_changes", false)],
    )
    .await;
    sqlx::query(
        r#"update organizations set notification_work_profile_id = $2 where id = $1"#,
    )
    .bind(org_id)
    .bind(mode_profile)
    .execute(&pool)
    .await
    .unwrap();
    let ctx2 = get_ctx(&app, t1, org_id_s, channel_id_s).await;
    assert_eq!(
        ctx2.get("profile_source").and_then(|v| v.as_str()),
        Some("mode_profile")
    );
    assert_eq!(
        ctx2.get("behavior")
            .and_then(|b| b.get("pin_changes"))
            .and_then(|v| v.as_bool()),
        Some(false)
    );

    // Channel override wins over mode profile.
    let channel_profile = insert_profile(
        &pool,
        org_id,
        "work",
        "Channel Profile",
        &[("pin_changes", true), ("thread_replies", false)],
    )
    .await;
    assert_eq!(
        patch_channel_override(&app, t1, org_id_s, channel_id_s, Some(&channel_profile.to_string())).await,
        axum::http::StatusCode::OK
    );
    let ctx3 = get_ctx(&app, t1, org_id_s, channel_id_s).await;
    assert_eq!(
        ctx3.get("profile_source").and_then(|v| v.as_str()),
        Some("channel_override")
    );
    assert_eq!(
        ctx3.get("behavior")
            .and_then(|b| b.get("pin_changes"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    // User override wins over channel override.
    let user_profile = insert_profile(
        &pool,
        org_id,
        "work",
        "User Profile",
        &[("message_mentions", false), ("message_all", true)],
    )
    .await;
    assert_eq!(
        patch_user_override(&app, t1, org_id_s, "work", Some(&user_profile.to_string())).await,
        axum::http::StatusCode::OK
    );
    let ctx4 = get_ctx(&app, t1, org_id_s, channel_id_s).await;
    assert_eq!(
        ctx4.get("profile_source").and_then(|v| v.as_str()),
        Some("user_override")
    );
    assert_eq!(
        ctx4.get("behavior")
            .and_then(|b| b.get("message_all"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    // Clearing user override falls back to channel override again.
    assert_eq!(
        patch_user_override(&app, t1, org_id_s, "work", None).await,
        axum::http::StatusCode::OK
    );
    let ctx5 = get_ctx(&app, t1, org_id_s, channel_id_s).await;
    assert_eq!(
        ctx5.get("profile_source").and_then(|v| v.as_str()),
        Some("channel_override")
    );
}

#[tokio::test]
async fn tenant_isolation_for_overrides() {
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

    let org1 = create_org(&app, t1, &format!("org{}", Uuid::now_v7().simple())).await;
    let org2 = create_org(&app, t2, &format!("org{}", Uuid::now_v7().simple())).await;
    let org2_id = org2.get("id").unwrap().as_str().unwrap();
    let org2_uuid = Uuid::parse_str(org2_id).unwrap();
    let ch2 = create_channel(&app, t2, org2_id, "private").await;
    let ch2_id = ch2.get("id").unwrap().as_str().unwrap();

    let prof2 = insert_profile(
        &pool,
        org2_uuid,
        "work",
        "Org2 Profile",
        &[("message_mentions", true)],
    )
    .await;

    // User1 cannot set overrides on org2/channel2.
    let st = patch_user_override(&app, t1, org2_id, "work", Some(&prof2.to_string())).await;
    assert_eq!(st, axum::http::StatusCode::FORBIDDEN);

    let st2 = patch_channel_override(&app, t1, org2_id, ch2_id, Some(&prof2.to_string())).await;
    assert_eq!(st2, axum::http::StatusCode::FORBIDDEN);

    let _ = org1;
}

