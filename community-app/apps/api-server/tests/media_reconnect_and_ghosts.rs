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

async fn create_media_room(app: &axum::Router, token: &str, org_id: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/orgs/{org_id}/media/rooms"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "kind": "voice",
                "name": "Test Voice",
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

async fn join(app: &axum::Router, token: &str, room_id: &str, device_id: &str) -> serde_json::Value {
    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/rooms/{room_id}/join"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(
            serde_json::json!({
                "intent": "voice_only",
                "device_id": device_id
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
async fn duplicate_reconnect_reuses_participant_row() {
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
    let room = create_media_room(&app, t1, org_id).await;
    let room_id = room.get("id").unwrap().as_str().unwrap();

    let j1 = join(&app, t1, room_id, "dev-1").await;
    let j2 = join(&app, t1, room_id, "dev-1").await;

    assert_eq!(j1.get("session_id").unwrap(), j2.get("session_id").unwrap());
    assert_eq!(
        j1.get("participant_id").unwrap(),
        j2.get("participant_id").unwrap()
    );

    let session_id = Uuid::parse_str(j1.get("session_id").unwrap().as_str().unwrap()).unwrap();
    let user_id = Uuid::parse_str(u1.get("id").unwrap().as_str().unwrap()).unwrap();

    let active: i64 = sqlx::query_scalar(
        r#"
        select count(1)::bigint
        from media_participants
        where media_session_id = $1
          and user_id = $2
          and device_id = 'dev-1'
          and left_at is null
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active, 1);
}

#[tokio::test]
async fn ghost_cleanup_then_reconnect_reuses_recent_participant() {
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
    let user_id = Uuid::parse_str(u1.get("id").unwrap().as_str().unwrap()).unwrap();

    let org = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org_id = org.get("id").unwrap().as_str().unwrap();
    let room = create_media_room(&app, t1, org_id).await;
    let room_id = room.get("id").unwrap().as_str().unwrap();

    let j1 = join(&app, t1, room_id, "dev-ghost").await;
    let session_id = Uuid::parse_str(j1.get("session_id").unwrap().as_str().unwrap()).unwrap();
    let participant_id = Uuid::parse_str(j1.get("participant_id").unwrap().as_str().unwrap()).unwrap();

    // Simulate ghost: old heartbeat.
    let old = time::OffsetDateTime::now_utc() - time::Duration::minutes(10);
    sqlx::query(
        r#"
        update media_participants
        set last_heartbeat_at = $2
        where id = $1
        "#,
    )
    .bind(participant_id)
    .bind(old)
    .execute(&pool)
    .await
    .unwrap();

    let livekit = media::LiveKitConfig {
        internal_url: cfg.livekit_url_internal.clone(),
        public_url: cfg.livekit_url_public.clone(),
        api_key: cfg.livekit_api_key.clone(),
        api_secret: cfg.livekit_api_secret.clone(),
    };

    let cleaned = media::cleanup_stale_participants(&pool, &livekit, time::Duration::seconds(1))
        .await
        .unwrap();
    assert!(cleaned >= 1);

    // Reconnect within window should reuse the same participant id and clear left_at.
    let j2 = join(&app, t1, room_id, "dev-ghost").await;
    assert_eq!(
        j2.get("participant_id").unwrap().as_str().unwrap(),
        participant_id.to_string()
    );

    let left_at: Option<time::OffsetDateTime> = sqlx::query_scalar(
        r#"select left_at from media_participants where id = $1"#,
    )
    .bind(participant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(left_at.is_none());

    // Ensure still only one active row.
    let active: i64 = sqlx::query_scalar(
        r#"
        select count(1)::bigint
        from media_participants
        where media_session_id = $1
          and user_id = $2
          and device_id = 'dev-ghost'
          and left_at is null
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active, 1);
}

