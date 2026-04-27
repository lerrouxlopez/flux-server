use crate::{repositories::HealthRepository, services::HealthService};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub health_service: HealthService,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let health_repo = HealthRepository::new(pool);
        let health_service = HealthService::new(health_repo);

        Self { health_service }
    }
}

