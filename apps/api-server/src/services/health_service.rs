use crate::repositories::HealthRepository;

#[derive(Clone)]
pub struct HealthService {
    repo: HealthRepository,
}

impl HealthService {
    pub fn new(repo: HealthRepository) -> Self {
        Self { repo }
    }

    pub async fn check(&self) -> Result<(), sqlx::Error> {
        self.repo.ping().await
    }
}

