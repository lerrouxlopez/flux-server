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
async fn non_member_cannot_subscribe_media_room() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let org_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let room_id = Uuid::now_v7();

    // Seed org + user + room without membership.
    sqlx::query(r#"insert into organizations (id, slug, name) values ($1,$2,$3)"#)
        .bind(org_id)
        .bind(format!("org{}", Uuid::now_v7().simple()))
        .bind("Org")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into users (id, email, display_name) values ($1,$2,$3)"#)
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

    let res = realtime_gateway::runtime::ensure_media_room_access(&pool, user_id, &[], room_id).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn member_in_other_org_cannot_subscribe_media_room() {
    if test_database_url().is_none() {
        eprintln!("skipping: set TEST_DATABASE_URL to run integration tests");
        return;
    }

    let pool = setup_pool().await;
    let org_a = Uuid::now_v7();
    let org_b = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let room_id = Uuid::now_v7();

    sqlx::query(r#"insert into organizations (id, slug, name) values ($1,$2,$3)"#)
        .bind(org_a)
        .bind(format!("orga{}", Uuid::now_v7().simple()))
        .bind("OrgA")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into organizations (id, slug, name) values ($1,$2,$3)"#)
        .bind(org_b)
        .bind(format!("orgb{}", Uuid::now_v7().simple()))
        .bind("OrgB")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(r#"insert into users (id, email, display_name) values ($1,$2,$3)"#)
        .bind(user_id)
        .bind(format!("u{}@example.com", Uuid::now_v7().simple()))
        .bind("User")
        .execute(&pool)
        .await
        .unwrap();

    // User is member of org_b only.
    sqlx::query(
        r#"insert into organization_members (organization_id, user_id, role) values ($1,$2,'member')"#,
    )
    .bind(org_b)
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    // Room belongs to org_a.
    sqlx::query(
        r#"
        insert into media_rooms (id, organization_id, livekit_room_name, kind, created_by)
        values ($1, $2, $3, 'voice', $4)
        "#,
    )
    .bind(room_id)
    .bind(org_a)
    .bind(format!("org-{org_a}-room-{room_id}"))
    .bind(user_id)
    .execute(&pool)
    .await
    .unwrap();

    let res = realtime_gateway::runtime::ensure_media_room_access(&pool, user_id, &[org_b], room_id).await;
    assert!(res.is_err());
}

