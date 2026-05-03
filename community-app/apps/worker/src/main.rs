use redis::AsyncCommands;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
struct MessageCreatedData {
    channel_id: Uuid,
    message_id: Uuid,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init();

    let cfg = config::AppConfig::from_env()?;
    let pool = db::connect(&cfg.database_url).await?;

    let redis_client = redis::Client::open(cfg.redis_url)?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;

    let nats = events::connect(&cfg.nats_url).await?;
    let js = events::jetstream::context(nats.clone());
    info!("worker connected (nats+jetstream)");

    // Streams for durable consumers (notifications, audit logs, indexing, email, cleanup).
    let js_cfg = events::jetstream::JetStreamConfig::default();
    if let Err(e) = events::jetstream::ensure_streams(&js, &js_cfg).await {
        warn!(?e, "failed to ensure jetstream streams");
    }

    // Consume message.created for notifications + audit (durable).
    let consumer = events::jetstream::ensure_durable_consumer(
        &js,
        &js_cfg.audit_stream,
        "worker-message-created",
        Some("org.*.channel.*.message.created"),
    )
    .await?;

    let mut cleanup_tick = tokio::time::interval(std::time::Duration::from_secs(60 * 10));
    let mut redis = redis;

    loop {
        tokio::select! {
            _ = cleanup_tick.tick() => {
                if let Err(e) = run_cleanup(&pool, &mut redis).await {
                    warn!(?e, "cleanup failed");
                }
            }
            res = async { consumer.fetch().max_messages(256).messages().await } => {
                let mut messages = match res {
                    Ok(m) => m,
                    Err(e) => {
                        warn!(?e, "jetstream fetch failed");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };

                while let Some(Ok(msg)) = messages.next().await {
                    if let Err(e) = handle_message_created(&pool, &mut redis, &msg.payload).await {
                        warn!(?e, "failed to process message.created");
                        // Leave unacked for redelivery.
                        continue;
                    }
                    if let Err(e) = msg.ack().await {
                        warn!(?e, "ack failed");
                    }
                }
            }
        }
    }
}

async fn handle_message_created(
    pool: &PgPool,
    redis: &mut redis::aio::ConnectionManager,
    payload: &[u8],
) -> anyhow::Result<()> {
    let env: events::envelope::EventEnvelope<MessageCreatedData> = serde_json::from_slice(payload)?;
    let org_id = env.organization_id;
    let actor_id = env.actor_id;
    let message_id = env.data.message_id;
    let channel_id = env.data.channel_id;

    // Fetch message sender + created_at for audit metadata.
    let row = sqlx::query(
        r#"
        select sender_id, created_at
        from messages
        where id = $1
        "#,
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        // Message deleted quickly; nothing to do.
        return Ok(());
    };

    let sender_id: Uuid = row.get("sender_id");
    let created_at: OffsetDateTime = row.get("created_at");

    // Audit log (durable DB write).
    let audit_id = Uuid::now_v7();
    let _ = sqlx::query(
        r#"
        insert into audit_logs (id, organization_id, actor_user_id, action, target_type, target_id, metadata, created_at)
        values ($1, $2, $3, 'message.created', 'message', $4, $5, $6)
        "#,
    )
    .bind(audit_id)
    .bind(org_id)
    .bind(actor_id.or(Some(sender_id)))
    .bind(message_id)
    .bind(serde_json::json!({"channel_id": channel_id}))
    .bind(created_at)
    .execute(pool)
    .await;

    // Offline notifications: create records for org members without a presence key.
    let members = sqlx::query_scalar::<_, Uuid>(
        r#"
        select user_id
        from organization_members
        where organization_id = $1 and user_id <> $2
        "#,
    )
    .bind(org_id)
    .bind(sender_id)
    .fetch_all(pool)
    .await?;

    for user_id in members {
        let presence_key = format!("presence:user:{user_id}");
        let online: bool = redis.exists(&presence_key).await.unwrap_or(false);
        if online {
            continue;
        }

        let notification_id = Uuid::now_v7();
        let _ = sqlx::query(
            r#"
            insert into notifications (id, organization_id, user_id, kind, message_id, created_at)
            values ($1, $2, $3, 'message.created', $4, now())
            "#,
        )
        .bind(notification_id)
        .bind(org_id)
        .bind(user_id)
        .bind(message_id)
        .execute(pool)
        .await;

        // Optionally publish a durable notification event (for email jobs, etc.) later.
    }

    Ok(())
}

async fn run_cleanup(pool: &PgPool, redis: &mut redis::aio::ConnectionManager) -> anyhow::Result<()> {
    // Expired refresh tokens.
    let _ = sqlx::query(
        r#"
        delete from refresh_tokens
        where expires_at < now()
           or (revoked_at is not null and revoked_at < now() - interval '1 day')
        "#,
    )
    .execute(pool)
    .await;

    // Typing/presence are TTL-based; no action needed, but keep hook for future leftovers.
    let _ = redis;
    Ok(())
}

// Needed for JetStream fetch stream.
use futures_util::StreamExt;
