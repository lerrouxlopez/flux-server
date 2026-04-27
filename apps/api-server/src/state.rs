use crate::{
    repositories::{HealthRepository, SessionRepository, UserRepository},
    services::{auth_service, AuthService, HealthService},
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub health_service: HealthService,
    pub auth_service: AuthService,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let health_repo = HealthRepository::new(pool.clone());
        let health_service = HealthService::new(health_repo);

        let users = UserRepository::new(pool.clone());
        let sessions = SessionRepository::new(pool);
        let jwt = auth_service::jwt_config_from_env().expect("JWT_SECRET must be set and valid");
        let auth_service = AuthService::new(users, sessions, jwt);

        Self {
            health_service,
            auth_service,
        }
    }
}
