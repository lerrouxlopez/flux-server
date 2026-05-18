use media::LiveKitConfig;
use std::time::Duration;

pub async fn check_postgres(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, i64>("select 1::bigint")
        .fetch_one(pool)
        .await
        .is_ok()
}

pub async fn check_redis_url(redis_url: &str, timeout: Duration) -> bool {
    let fut = async {
        let client = redis::Client::open(redis_url).ok()?;
        let mut conn = redis::aio::ConnectionManager::new(client).await.ok()?;
        let pong: redis::RedisResult<String> = redis::cmd("PING").query_async(&mut conn).await;
        pong.ok()?;
        Some(())
    };
    tokio::time::timeout(timeout, fut).await.ok().flatten().is_some()
}

pub async fn check_nats_url(nats_url: &str, timeout: Duration) -> bool {
    let fut = async {
        let client = async_nats::connect(nats_url).await.ok()?;
        let _ = client.flush().await.ok()?;
        Some(())
    };
    tokio::time::timeout(timeout, fut).await.ok().flatten().is_some()
}

pub async fn check_livekit_roomservice(cfg: &LiveKitConfig, timeout: Duration) -> bool {
    let fut = async { media::roomservice_health_check(cfg).await.ok() };
    tokio::time::timeout(timeout, fut)
        .await
        .ok()
        .flatten()
        .is_some()
}

