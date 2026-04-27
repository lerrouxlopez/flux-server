use crate::{
    models::auth::ApiError,
    repositories::{MembershipRepository, RoleRepository},
};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone)]
pub struct PermissionsService {
    memberships: MembershipRepository,
    roles: RoleRepository,
}

impl PermissionsService {
    pub fn new(memberships: MembershipRepository, roles: RoleRepository) -> Self {
        Self { memberships, roles }
    }

    pub async fn require(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        permission: &'static str,
    ) -> Result<(), ApiError> {
        let membership = self
            .memberships
            .find_membership(organization_id, user_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        // Owner shortcut (still backed by Postgres role table, but allow bootstrap).
        if membership.role == "owner" {
            return Ok(());
        }

        let role = self
            .roles
            .find_by_org_and_name(organization_id, &membership.role)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::forbidden)?;

        if has_permission(&role.permissions, permission) {
            Ok(())
        } else {
            Err(ApiError::forbidden())
        }
    }
}

fn has_permission(perms: &Value, key: &str) -> bool {
    perms.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

