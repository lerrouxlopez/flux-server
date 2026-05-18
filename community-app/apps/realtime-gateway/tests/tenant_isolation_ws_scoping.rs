use sqlx::PgPool;
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

#[tokio::test]
async fn cannot_subscribe_to_other_org_channel() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;

    let org1 = Uuid::now_v7();
    let org2 = Uuid::now_v7();
    let u1 = Uuid::now_v7();
    let u2 = Uuid::now_v7();
    let ch2 = Uuid::now_v7();

    sqlx::query(r#"insert into organizations (id, slug, name) values ($1,$2,$3)"#)
        .bind(org1)
        .bind(format!("o{}", org1.simple()))
        .bind("Org1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into organizations (id, slug, name) values ($1,$2,$3)"#)
        .bind(org2)
        .bind(format!("o{}", org2.simple()))
        .bind("Org2")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(r#"insert into users (id, email, display_name) values ($1,$2,$3)"#)
        .bind(u1)
        .bind(format!("u1+{}@example.com", u1.simple()))
        .bind("U1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into users (id, email, display_name) values ($1,$2,$3)"#)
        .bind(u2)
        .bind(format!("u2+{}@example.com", u2.simple()))
        .bind("U2")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(r#"insert into organization_members (organization_id, user_id, role) values ($1,$2,'member')"#)
        .bind(org1)
        .bind(u1)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into organization_members (organization_id, user_id, role) values ($1,$2,'member')"#)
        .bind(org2)
        .bind(u2)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(r#"insert into channels (id, organization_id, name, kind) values ($1,$2,$3,$4)"#)
        .bind(ch2)
        .bind(org2)
        .bind("general")
        .bind("text")
        .execute(&pool)
        .await
        .unwrap();

    let org_ids_for_u1 = vec![org1];
    let res = realtime_gateway::runtime::ensure_channel_access(&pool, u1, &org_ids_for_u1, ch2).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn cannot_subscribe_to_other_org_media_room() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;

    let org1 = Uuid::now_v7();
    let org2 = Uuid::now_v7();
    let u1 = Uuid::now_v7();
    let u2 = Uuid::now_v7();
    let room2 = Uuid::now_v7();

    sqlx::query(r#"insert into organizations (id, slug, name) values ($1,$2,$3)"#)
        .bind(org1)
        .bind(format!("o{}", org1.simple()))
        .bind("Org1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into organizations (id, slug, name) values ($1,$2,$3)"#)
        .bind(org2)
        .bind(format!("o{}", org2.simple()))
        .bind("Org2")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(r#"insert into users (id, email, display_name) values ($1,$2,$3)"#)
        .bind(u1)
        .bind(format!("u1+{}@example.com", u1.simple()))
        .bind("U1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into users (id, email, display_name) values ($1,$2,$3)"#)
        .bind(u2)
        .bind(format!("u2+{}@example.com", u2.simple()))
        .bind("U2")
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query(r#"insert into organization_members (organization_id, user_id, role) values ($1,$2,'member')"#)
        .bind(org1)
        .bind(u1)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into organization_members (organization_id, user_id, role) values ($1,$2,'member')"#)
        .bind(org2)
        .bind(u2)
        .execute(&pool)
        .await
        .unwrap();

    // Minimal media room row (schema fields).
    sqlx::query(
        r#"
        insert into media_rooms (id, organization_id, channel_id, livekit_room_name, kind, name, created_by)
        values ($1,$2,null,$3,$4,$5,$6)
        "#,
    )
    .bind(room2)
    .bind(org2)
    .bind(format!("lk_{}", room2.simple()))
    .bind("voice")
    .bind("Room2")
    .bind(u2)
    .execute(&pool)
    .await
    .unwrap();

    let org_ids_for_u1 = vec![org1];
    let res = realtime_gateway::runtime::ensure_media_room_access(&pool, u1, &org_ids_for_u1, room2).await;
    assert!(res.is_err());
}

