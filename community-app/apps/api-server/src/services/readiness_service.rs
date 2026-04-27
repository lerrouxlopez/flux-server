use crate::models::auth::ApiError;
use redis::AsyncCommands;
use sqlx::PgPool;

#[derive(Clone)]
pub struct ReadinessService {
    pool: PgPool,
    redis_url: String,
    nats_url: String,
}

impl ReadinessService {
    pub fn new(pool: PgPool, redis_url: String, nats_url: String) -> Self {
        Self {
            pool,
            redis_url,
            nats_url,
        }
    }

    pub async fn check(&self) -> Result<(), ApiError> {
        db::ping(&self.pool).await.map_err(|_| ApiError::internal())?;
        self.check_redis().await?;
        self.check_nats().await?;
        Ok(())
    }

    async fn check_redis(&self) -> Result<(), ApiError> {
        let client = redis::Client::open(self.redis_url.as_str()).map_err(|_| ApiError::internal())?;
        let mut conn = client
            .get_connection_manager()
            .await
            .map_err(|_| ApiError::internal())?;
        let pong: String = conn.ping().await.map_err(|_| ApiError::internal())?;
        if pong.to_lowercase() != "pong" {
            return Err(ApiError::internal());
        }
        Ok(())
    }

    async fn check_nats(&self) -> Result<(), ApiError> {
        let client = async_nats::connect(self.nats_url.as_str())
            .await
            .map_err(|_| ApiError::internal())?;
        client.flush().await.map_err(|_| ApiError::internal())?;
        Ok(())
    }
}

