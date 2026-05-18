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

#[tokio::test]
async fn join_heartbeat_leave_success() {
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

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/rooms/{room_id}/join"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::from(serde_json::json!({ "intent": "voice_only" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let join: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let session_id = join.get("session_id").unwrap().as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/sessions/{session_id}/heartbeat"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/sessions/{session_id}/leave"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());

    let req = axum::http::Request::builder()
        .method("GET")
        .uri(format!("/media/sessions/{session_id}"))
        .header("authorization", format!("Bearer {t1}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert!(res.status().is_success());
}

#[tokio::test]
async fn unauthorized_join_when_not_member() {
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
    let room = create_media_room(&app, t1, org_id).await;
    let room_id = room.get("id").unwrap().as_str().unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/rooms/{room_id}/join"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::from(serde_json::json!({ "intent": "voice_only" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unauthorized_join_when_member_lacks_voice_join_perm() {
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
    let room = create_media_room(&app, t1, org_id).await;
    let room_id = room.get("id").unwrap().as_str().unwrap();

    let org_uuid = Uuid::parse_str(org_id).unwrap();
    let user2_uuid = Uuid::parse_str(user2_id).unwrap();

    // Add user2 to org, but set them to a role with zero permissions.
    sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role)
        values ($1, $2, 'no_voice')
        on conflict do nothing
        "#,
    )
    .bind(org_uuid)
    .bind(user2_uuid)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        insert into roles (id, organization_id, name, permissions)
        values ($1, $2, 'no_voice', 0)
        on conflict do nothing
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(org_uuid)
    .execute(&pool)
    .await
    .unwrap();

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/rooms/{room_id}/join"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::from(serde_json::json!({ "intent": "voice_only" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn tenant_isolation_join_denied() {
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

    let org1 = create_org(&app, t1, &format!("acme{}", Uuid::now_v7().simple())).await;
    let org1_id = org1.get("id").unwrap().as_str().unwrap();
    let room1 = create_media_room(&app, t1, org1_id).await;
    let room1_id = room1.get("id").unwrap().as_str().unwrap();

    let _org2 = create_org(&app, t2, &format!("beta{}", Uuid::now_v7().simple())).await;

    let req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/media/rooms/{room1_id}/join"))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {t2}"))
        .body(Body::from(serde_json::json!({ "intent": "voice_only" }).to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn stale_cleanup_helper_marks_left() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let cfg = config::AppConfig::from_env().expect("config");

    // Minimal seed: org + user + room + session + participant.
    let org_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let room_id = Uuid::now_v7();
    let session_id = Uuid::now_v7();
    let participant_id = Uuid::now_v7();

    sqlx::query(r#"insert into organizations (id, slug, name) values ($1, $2, $3)"#)
        .bind(org_id)
        .bind(format!("org{}", Uuid::now_v7().simple()))
        .bind("Org")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(r#"insert into users (id, email, display_name) values ($1, $2, $3)"#)
        .bind(user_id)
        .bind(format!("u{}@example.com", Uuid::now_v7().simple()))
        .bind("User")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(
        r#"
        insert into media_rooms (id, organization_id, livekit_room_name, kind, created_by)
        values ($1, $2, $3, 'voice', $4)
        "#,
    )
    .bind(room_id)
    .bind(org_id)
    .bind(format!("org-{org_id}-room-{room_id}"))
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        insert into media_sessions (id, organization_id, media_room_id, created_by, started_at)
        values ($1, $2, $3, $4, now())
        "#,
    )
    .bind(session_id)
    .bind(org_id)
    .bind(room_id)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let old = time::OffsetDateTime::now_utc() - time::Duration::minutes(10);
    sqlx::query(
        r#"
        insert into media_participants (
          id, organization_id, media_session_id, user_id, identity,
          can_subscribe, can_publish_audio, can_publish_data,
          joined_at, last_heartbeat_at
        )
        values ($1,$2,$3,$4,$5,true,true,true, now(), $6)
        "#,
    )
    .bind(participant_id)
    .bind(org_id)
    .bind(session_id)
    .bind(user_id)
    .bind(user_id.to_string())
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

    let left_reason: Option<String> = sqlx::query_scalar(
        r#"select left_reason from media_participants where id = $1"#,
    )
    .bind(participant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(left_reason.as_deref(), Some("stale"));
}
