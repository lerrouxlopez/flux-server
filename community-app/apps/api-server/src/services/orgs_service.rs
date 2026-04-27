use crate::{
    models::auth::ApiError,
    models::orgs::{self, CreateOrgRequest, OrganizationView},
    repositories::{InviteRepository, MembershipRepository, OrgRepository, RoleRepository},
    services::PermissionsService,
};
use base64::Engine;
use rand::RngCore;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrgsService {
    orgs: OrgRepository,
    memberships: MembershipRepository,
    invites: InviteRepository,
    roles: RoleRepository,
    permissions: PermissionsService,
}

impl OrgsService {
    pub fn new(
        orgs: OrgRepository,
        memberships: MembershipRepository,
        invites: InviteRepository,
        roles: RoleRepository,
        permissions: PermissionsService,
    ) -> Self {
        Self {
            orgs,
            memberships,
            invites,
            roles,
            permissions,
        }
    }

    pub async fn create_org(
        &self,
        user_id: Uuid,
        req: CreateOrgRequest,
    ) -> Result<OrganizationView, ApiError> {
        let slug = normalize_slug(&req.slug).ok_or_else(ApiError::bad_request)?;
        let name = req.name.trim();
        if name.is_empty() {
            return Err(ApiError::bad_request());
        }

        let org_id = Uuid::now_v7();
        let owner_permissions = default_owner_permissions();
        let member_permissions = default_member_permissions();
        let org = self
            .orgs
            .create_org_with_defaults(
                org_id,
                &slug,
                name,
                user_id,
                &owner_permissions,
                &member_permissions,
            )
            .await
            .map_err(map_unique_violation)?;

        Ok(OrganizationView {
            id: org.id,
            slug: org.slug,
            name: org.name,
            created_at: org.created_at,
        })
    }

    pub async fn list_orgs(&self, user_id: Uuid) -> Result<Vec<OrganizationView>, ApiError> {
        let org_ids = self
            .memberships
            .list_orgs_for_user(user_id, 100)
            .await
            .map_err(|_| ApiError::internal())?;

        let mut out = Vec::with_capacity(org_ids.len());
        for org_id in org_ids {
            if let Some(org) = self
                .orgs
                .find_by_id(org_id)
                .await
                .map_err(|_| ApiError::internal())?
            {
                out.push(OrganizationView {
                    id: org.id,
                    slug: org.slug,
                    name: org.name,
                    created_at: org.created_at,
                });
            }
        }
        Ok(out)
    }

    pub async fn get_org(&self, user_id: Uuid, org_id: Uuid) -> Result<OrganizationView, ApiError> {
        let is_member = self
            .memberships
            .find_membership(org_id, user_id)
            .await
            .map_err(|_| ApiError::internal())?
            .is_some();
        if !is_member {
            return Err(ApiError::unauthorized());
        }

        let org = self
            .orgs
            .find_by_id(org_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::not_found)?;

        Ok(OrganizationView {
            id: org.id,
            slug: org.slug,
            name: org.name,
            created_at: org.created_at,
        })
    }

    pub async fn list_members(
        &self,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<Vec<orgs::OrgMemberView>, ApiError> {
        self.permissions
            .require(user_id, org_id, "org.manage_members")
            .await?;

        let rows = self
            .memberships
            .list_members_with_users(org_id, 200)
            .await
            .map_err(|_| ApiError::internal())?;

        Ok(rows
            .into_iter()
            .map(|(m, email, display_name)| orgs::OrgMemberView {
                user_id: m.user_id,
                email,
                display_name,
                role: m.role,
                joined_at: m.joined_at,
            })
            .collect())
    }

    pub async fn create_invite(
        &self,
        user_id: Uuid,
        org_id: Uuid,
        req: orgs::CreateInviteRequest,
    ) -> Result<orgs::CreateInviteResponse, ApiError> {
        self.permissions
            .require(user_id, org_id, "org.manage_members")
            .await?;

        let role = req.role.unwrap_or_else(|| "member".to_string());
        let role_exists = self
            .roles
            .find_by_org_and_name(org_id, &role)
            .await
            .map_err(|_| ApiError::internal())?
            .is_some();
        if !role_exists {
            return Err(ApiError::bad_request());
        }

        let token = generate_invite_token();
        let row = self
            .invites
            .insert_invite(Uuid::now_v7(), org_id, &token, &role, user_id, req.expires_at)
            .await
            .map_err(map_unique_violation)?;

        Ok(orgs::CreateInviteResponse {
            token: row.token,
            role: row.role,
            expires_at: row.expires_at,
        })
    }

    pub async fn add_member(
        &self,
        user_id: Uuid,
        org_id: Uuid,
        req: orgs::AddMemberRequest,
    ) -> Result<(), ApiError> {
        self.permissions
            .require(user_id, org_id, "org.manage_members")
            .await?;

        let role_exists = self
            .roles
            .find_by_org_and_name(org_id, &req.role)
            .await
            .map_err(|_| ApiError::internal())?
            .is_some();
        if !role_exists {
            return Err(ApiError::bad_request());
        }

        self.memberships
            .insert_membership(org_id, req.user_id, &req.role)
            .await
            .map_err(map_unique_violation)?;

        Ok(())
    }

    pub async fn resolve_org_for_user(
        &self,
        user_id: Uuid,
        requested_org_id: Option<Uuid>,
    ) -> Result<OrganizationView, ApiError> {
        let org_id = if let Some(org_id) = requested_org_id {
            let is_member = self
                .memberships
                .find_membership(org_id, user_id)
                .await
                .map_err(|_| ApiError::internal())?
                .is_some();
            if !is_member {
                return Err(ApiError::unauthorized());
            }
            org_id
        } else {
            self.memberships
                .find_first_org_for_user(user_id)
                .await
                .map_err(|_| ApiError::internal())?
                .ok_or_else(ApiError::unauthorized)?
        };

        let org = self
            .orgs
            .find_by_id(org_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::unauthorized)?;

        Ok(OrganizationView {
            id: org.id,
            slug: org.slug,
            name: org.name,
            created_at: org.created_at,
        })
    }
}

fn normalize_slug(slug: &str) -> Option<String> {
    let s = slug.trim().to_lowercase();
    if s.len() < 3 || s.len() > 32 {
        return None;
    }
    if !s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    Some(s)
}

fn map_unique_violation(err: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.is_unique_violation() {
            return ApiError::conflict();
        }
    }
    ApiError::internal()
}

fn default_owner_permissions() -> serde_json::Value {
    serde_json::json!({
        "channels.view": true,
        "channels.create": true,
        "channels.manage": true,
        "messages.send": true,
        "messages.edit_own": true,
        "messages.delete_own": true,
        "messages.delete_any": true,
        "org.manage_members": true,
        "branding.manage": true
    })
}

fn default_member_permissions() -> serde_json::Value {
    serde_json::json!({
        "channels.view": true,
        "channels.create": false,
        "channels.manage": false,
        "messages.send": true,
        "messages.edit_own": true,
        "messages.delete_own": true,
        "messages.delete_any": false,
        "org.manage_members": false,
        "branding.manage": false
    })
}

fn generate_invite_token() -> String {
    let mut buf = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}
