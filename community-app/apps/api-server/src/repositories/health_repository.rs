use sqlx::PgPool;

#[derive(Clone)]
pub struct HealthRepository {
    pool: PgPool,
}

impl HealthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        db::ping(&self.pool).await
    }
}

