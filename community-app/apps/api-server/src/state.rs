use crate::{
    repositories::{
        ChannelRepository, InviteRepository, MembershipRepository, OrgRepository, RoleRepository,
        SessionRepository, UserRepository,
    },
    services::{
        auth_service, AuthService, ChannelsService, MessagesService, OrgsService, PermissionsService,
        ReadinessService,
    },
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: AuthService,
    pub orgs_service: OrgsService,
    pub channels_service: ChannelsService,
    pub permissions_service: PermissionsService,
    pub messages_service: MessagesService,
    pub readiness_service: ReadinessService,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let pool_for_readiness = pool.clone();
        let pool_for_messages = pool.clone();

        let users = UserRepository::new(pool.clone());
        let sessions = SessionRepository::new(pool.clone());
        let jwt = auth_service::jwt_config_from_env()
            .expect("JWT_ACCESS_SECRET and JWT_REFRESH_SECRET must be set and valid");
        let auth_service = AuthService::new(users, sessions, jwt);

        let orgs = OrgRepository::new(pool.clone());
        let memberships = MembershipRepository::new(pool.clone());
        let roles = RoleRepository::new(pool.clone());
        let permissions_service = PermissionsService::new(memberships.clone(), roles.clone());
        let invites = InviteRepository::new(pool.clone());
        let orgs_service = OrgsService::new(
            orgs,
            memberships.clone(),
            invites,
            roles.clone(),
            permissions_service.clone(),
        );

        let channels = ChannelRepository::new(pool.clone());
        let channels_service = ChannelsService::new(channels, permissions_service.clone());

        let redis_url = config::required("REDIS_URL").expect("REDIS_URL must be set");
        let nats_url = config::required("NATS_URL").expect("NATS_URL must be set");
        let readiness_service = ReadinessService::new(pool_for_readiness, redis_url, nats_url.clone());

        let messages_service = MessagesService::new(
            pool_for_messages,
            nats_url,
            memberships.clone(),
            roles,
        );

        Self {
            auth_service,
            orgs_service,
            channels_service,
            permissions_service,
            messages_service,
            readiness_service,
        }
    }
}
