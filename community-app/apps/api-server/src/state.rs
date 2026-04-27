use crate::{
    repositories::{
        ChannelRepository, HealthRepository, MembershipRepository, OrgRepository, SessionRepository,
        UserRepository,
    },
    services::{auth_service, AuthService, ChannelsService, HealthService, OrgsService},
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub health_service: HealthService,
    pub auth_service: AuthService,
    pub orgs_service: OrgsService,
    pub channels_service: ChannelsService,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let health_repo = HealthRepository::new(pool.clone());
        let health_service = HealthService::new(health_repo);

        let users = UserRepository::new(pool.clone());
        let sessions = SessionRepository::new(pool.clone());
        let jwt = auth_service::jwt_config_from_env()
            .expect("JWT_ACCESS_SECRET and JWT_REFRESH_SECRET must be set and valid");
        let auth_service = AuthService::new(users, sessions, jwt);

        let orgs = OrgRepository::new(pool.clone());
        let memberships = MembershipRepository::new(pool.clone());
        let orgs_service = OrgsService::new(orgs, memberships.clone());

        let channels = ChannelRepository::new(pool);
        let channels_service = ChannelsService::new(channels, memberships);

        Self {
            health_service,
            auth_service,
            orgs_service,
            channels_service,
        }
    }
}
