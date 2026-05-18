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
                "name": "general",
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
        .body(Body::from(serde_json::json!({ "body": body }).to_string()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn add_readonly_member(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    let readonly_perms = permissions::perms::CHANNELS_VIEW as i64;
    let role_id = Uuid::now_v7();
    let _ = sqlx::query(
        r#"
        insert into roles (id, organization_id, name, permissions)
        values ($1, $2, 'readonly', $3)
        on conflict (organization_id, name) do update set permissions = excluded.permissions
        "#,
    )
    .bind(role_id)
    .bind(org_id)
    .bind(readonly_perms)
    .execute(pool)
    .await;

    let _ = sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role)
        values ($1, $2, 'readonly')
        on conflict (organization_id, user_id) do update set role = excluded.role
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .execute(pool)
    .await;
}

#[tokio::test]
async fn threads_pins_search_happy_path() {
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
    let ch = create_channel(&app, t1, org_id).await;
    let channel_id = ch.get("id").unwrap().as_str().unwrap();

    let msg = send_message(&app, t1, channel_id, "hello world").await;
    let message_id = msg.get("id").unwrap().as_str().unwrap();

    // Create a thread from an existing root message.
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/channels/{channel_id}/threads"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(
            serde_json::json!({
                "root_message_id": message_id
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let thread_resp: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = thread_resp.get("id").unwrap().as_str().unwrap();

    // Reply to thread.
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/threads/{thread_id}/replies"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(serde_json::json!({ "body": "reply 1" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    // Fetch thread: root should not be duplicated in replies.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/threads/{thread_id}"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        data.get("root").and_then(|v| v.get("id")).and_then(|v| v.as_str()),
        Some(message_id)
    );
    let replies = data.get("replies").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert_eq!(replies.len(), 1);
    assert_ne!(replies[0].get("id").and_then(|v| v.as_str()), Some(message_id));

    // Pin/unpin.
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/channels/{channel_id}/pins"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(serde_json::json!({ "message_id": message_id }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{channel_id}/pins"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let pins: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let pins_arr = pins.get("pins").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert_eq!(pins_arr.len(), 1);
    assert_eq!(
        pins_arr[0]
            .get("message")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str()),
        Some(message_id)
    );

    let req = axum::http::Request::builder()
        .method("DELETE")
        .uri(format!("/channels/{channel_id}/pins/{message_id}"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    // Search should find the message.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{channel_id}/search?q=hello&limit=10"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let search: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let msgs = search.get("messages").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert!(msgs.iter().any(|m| m.get("id").and_then(|v| v.as_str()) == Some(message_id)));
}

#[tokio::test]
async fn permission_denial_for_readonly_member() {
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
    let u2_id = Uuid::parse_str(u2.get("user").unwrap().get("id").unwrap().as_str().unwrap()).unwrap();

    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = Uuid::parse_str(org.get("id").unwrap().as_str().unwrap()).unwrap();
    let ch = create_channel(&app, t1, org.get("id").unwrap().as_str().unwrap()).await;
    let channel_id = ch.get("id").unwrap().as_str().unwrap();

    add_readonly_member(&pool, org_id, u2_id).await;

    let msg = send_message(&app, t1, channel_id, "readonly check").await;
    let message_id = msg.get("id").unwrap().as_str().unwrap();

    // Pins should be denied (403) when user lacks MESSAGES_SEND.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{channel_id}/pins"))
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Threads create should be denied (403).
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/channels/{channel_id}/threads"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::from(
            serde_json::json!({
                "root_message_id": message_id
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Search should be denied (403).
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{channel_id}/search?q=readonly&limit=5"))
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn cross_tenant_denial_for_pins_threads_search() {
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

    let ch2 = create_channel(&app, t2, org2.get("id").unwrap().as_str().unwrap()).await;
    let channel2_id = ch2.get("id").unwrap().as_str().unwrap();
    let msg2 = send_message(&app, t2, channel2_id, "tenant2").await;
    let msg2_id = msg2.get("id").unwrap().as_str().unwrap();

    // User1 (org1) should not access org2 channel pins/search/threads.
    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{channel2_id}/pins"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/channels/{channel2_id}/search?q=tenant2&limit=5"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/channels/{channel2_id}/threads"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(
            serde_json::json!({
                "root_message_id": msg2_id
            })
            .to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);

    // Keep org1 referenced to avoid unused warnings (future expansion).
    let _ = org1;
}

#[tokio::test]
async fn pin_limit_enforced_per_channel() {
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
    let u1_id = Uuid::parse_str(u1.get("user").unwrap().get("id").unwrap().as_str().unwrap()).unwrap();

    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = Uuid::parse_str(org.get("id").unwrap().as_str().unwrap()).unwrap();
    let ch = create_channel(&app, t1, org.get("id").unwrap().as_str().unwrap()).await;
    let channel_id = Uuid::parse_str(ch.get("id").unwrap().as_str().unwrap()).unwrap();

    // Seed 50 pinned messages quickly via SQL.
    for _ in 0..50 {
        let mid = Uuid::now_v7();
        sqlx::query(
            r#"
            insert into messages (id, organization_id, channel_id, sender_id, body, kind, created_at)
            values ($1,$2,$3,$4,$5,'text',now())
            "#,
        )
        .bind(mid)
        .bind(org_id)
        .bind(channel_id)
        .bind(u1_id)
        .bind("seed pin")
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            insert into channel_pins (organization_id, channel_id, message_id, pinned_by, pinned_at)
            values ($1,$2,$3,$4,now())
            "#,
        )
        .bind(org_id)
        .bind(channel_id)
        .bind(mid)
        .bind(u1_id)
        .execute(&pool)
        .await
        .unwrap();
    }

    // 51st pin should be rejected by the API.
    let msg = send_message(&app, t1, &channel_id.to_string(), "one too many").await;
    let message_id = msg.get("id").unwrap().as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/channels/{channel_id}/pins"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(serde_json::json!({ "message_id": message_id }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::BAD_REQUEST);
}
