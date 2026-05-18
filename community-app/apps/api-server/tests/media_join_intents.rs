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

async fn create_media_room(
    app: &axum::Router,
    token: &str,
    org_id: &str,
    kind: &str,
) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/media/rooms"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "kind": kind,
                "name": format!("Test {kind}"),
                "channel_id": null
            })
            .to_string(),
        ))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn join(app: &axum::Router, token: &str, room_id: &str, intent: &str) -> (axum::http::StatusCode, serde_json::Value) {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/rooms/{room_id}/join"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(serde_json::json!({ "intent": intent, "device_id": "dev-1" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
    (status, json)
}

#[tokio::test]
async fn intents_grant_expected_capabilities() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let cfg = config::AppConfig::from_env().expect("config");
    let state = bootstrap_state(pool).await;
    let app = api_server::app::build_app(&cfg, state);

    let u1 = register(&app, &format!("u1+{}@example.com", Uuid::now_v7())).await;
    let t1 = u1.get("access_token").unwrap().as_str().unwrap();

    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = org.get("id").unwrap().as_str().unwrap();

    let voice = create_media_room(&app, t1, org_id, "voice").await;
    let meeting = create_media_room(&app, t1, org_id, "meeting").await;
    let stage = create_media_room(&app, t1, org_id, "stage").await;

    let voice_id = voice.get("id").unwrap().as_str().unwrap();
    let meeting_id = meeting.get("id").unwrap().as_str().unwrap();
    let stage_id = stage.get("id").unwrap().as_str().unwrap();

    // voice_only in a voice room
    let (st, j) = join(&app, t1, voice_id, "voice_only").await;
    assert!(st.is_success());
    assert_eq!(j.pointer("/granted/can_publish_audio").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(j.pointer("/granted/can_publish_video").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(j.pointer("/granted/can_publish_screen").and_then(|v| v.as_bool()), Some(false));

    // video in a meeting room
    let (st, j) = join(&app, t1, meeting_id, "video").await;
    assert!(st.is_success());
    assert_eq!(j.pointer("/granted/can_publish_audio").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(j.pointer("/granted/can_publish_video").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(j.pointer("/granted/can_publish_screen").and_then(|v| v.as_bool()), Some(false));

    // screen_share in a meeting room
    let (st, j) = join(&app, t1, meeting_id, "screen_share").await;
    assert!(st.is_success());
    assert_eq!(j.pointer("/granted/can_publish_screen").and_then(|v| v.as_bool()), Some(true));

    // stage_viewer in a stage room
    let (st, j) = join(&app, t1, stage_id, "stage_viewer").await;
    assert!(st.is_success());
    assert_eq!(j.pointer("/granted/can_publish_audio").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(j.pointer("/granted/can_publish_video").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(j.pointer("/granted/can_publish_screen").and_then(|v| v.as_bool()), Some(false));

    // stage_speaker in a stage room
    let (st, j) = join(&app, t1, stage_id, "stage_speaker").await;
    assert!(st.is_success());
    assert_eq!(j.pointer("/granted/can_publish_audio").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(j.pointer("/granted/can_publish_video").and_then(|v| v.as_bool()), Some(true));
}

#[tokio::test]
async fn intent_permission_denial() {
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
    let user2_id = u2.get("id").unwrap().as_str().unwrap();

    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = org.get("id").unwrap().as_str().unwrap();
    let voice = create_media_room(&app, t1, org_id, "voice").await;
    let meeting = create_media_room(&app, t1, org_id, "meeting").await;
    let stage = create_media_room(&app, t1, org_id, "stage").await;

    let voice_id = voice.get("id").unwrap().as_str().unwrap();
    let meeting_id = meeting.get("id").unwrap().as_str().unwrap();
    let stage_id = stage.get("id").unwrap().as_str().unwrap();

    let org_uuid = Uuid::parse_str(org_id).unwrap();
    let user2_uuid = Uuid::parse_str(user2_id).unwrap();

    // Add user2 as a limited member: VOICE_JOIN only.
    let voice_join_only: i64 = permissions::perms::VOICE_JOIN;
    sqlx::query(
        r#"
        insert into roles (id, organization_id, name, permissions)
        values ($1, $2, 'voice_join_only', $3)
        on conflict do nothing
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(org_uuid)
    .bind(voice_join_only)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role)
        values ($1, $2, 'voice_join_only')
        on conflict do nothing
        "#,
    )
    .bind(org_uuid)
    .bind(user2_uuid)
    .execute(&pool)
    .await
    .unwrap();

    // stage_viewer should be allowed with VOICE_JOIN only.
    let (st, _) = join(&app, t2, stage_id, "stage_viewer").await;
    assert!(st.is_success());

    // Speaker-ish intents should be denied without VOICE_SPEAK/VIDEO_START/SCREEN_SHARE.
    let (st, _) = join(&app, t2, voice_id, "voice_only").await;
    assert_eq!(st, axum::http::StatusCode::FORBIDDEN);
    let (st, _) = join(&app, t2, meeting_id, "video").await;
    assert_eq!(st, axum::http::StatusCode::FORBIDDEN);
    let (st, _) = join(&app, t2, meeting_id, "screen_share").await;
    assert_eq!(st, axum::http::StatusCode::FORBIDDEN);
    let (st, _) = join(&app, t2, stage_id, "stage_speaker").await;
    assert_eq!(st, axum::http::StatusCode::FORBIDDEN);
}
