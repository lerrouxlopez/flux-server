use crate::{
    models::auth::ApiError,
    models::orgs::{CreateOrgRequest, OrganizationView},
    repositories::{MembershipRepository, OrgRepository},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct OrgsService {
    orgs: OrgRepository,
    memberships: MembershipRepository,
}

impl OrgsService {
    pub fn new(orgs: OrgRepository, memberships: MembershipRepository) -> Self {
        Self { orgs, memberships }
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
        let (org, _membership) = self
            .orgs
            .create_org_with_owner(org_id, &slug, name, user_id, "owner")
            .await
            .map_err(map_unique_violation)?;

        Ok(OrganizationView {
            id: org.id,
            slug: org.slug,
            name: org.name,
            created_at: org.created_at,
        })
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
