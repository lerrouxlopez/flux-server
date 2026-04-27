use crate::{
    repositories::{
        ChannelRepository, MembershipRepository, OrgRepository, SessionRepository, UserRepository,
    },
    services::{
        auth_service, AuthService, ChannelsService, OrgsService, ReadinessService,
    },
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub orgs_service: OrgsService,
    pub channels_service: ChannelsService,
    pub readiness_service: ReadinessService,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let users = UserRepository::new(pool.clone());
        let sessions = SessionRepository::new(pool.clone());
        let jwt = auth_service::jwt_config_from_env()
            .expect("JWT_ACCESS_SECRET and JWT_REFRESH_SECRET must be set and valid");
        let auth_service = AuthService::new(users, sessions, jwt);

        let orgs = OrgRepository::new(pool.clone());
        let memberships = MembershipRepository::new(pool.clone());
        let orgs_service = OrgsService::new(orgs, memberships.clone());

        let channels = ChannelRepository::new(pool.clone());
        let channels_service = ChannelsService::new(channels, memberships);

        let redis_url = config::required("REDIS_URL").expect("REDIS_URL must be set");
        let nats_url = config::required("NATS_URL").expect("NATS_URL must be set");
        let readiness_service = ReadinessService::new(pool, redis_url, nats_url);

        Self {
            auth_service,
            orgs_service,
            channels_service,
            readiness_service,
        }
    }
}
