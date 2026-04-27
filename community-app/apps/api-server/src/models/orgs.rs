use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct OrganizationView {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct CreateOrgResponse {
    pub organization: OrganizationView,
}

#[derive(Debug, Serialize)]
pub struct CurrentOrgResponse {
    pub organization: OrganizationView,
}

#[derive(Debug, Serialize)]
pub struct ListOrgsResponse {
    pub organizations: Vec<OrganizationView>,
}

#[derive(Debug, Serialize)]
pub struct GetOrgResponse {
    pub organization: OrganizationView,
}

#[derive(Debug, Serialize)]
pub struct OrgMemberView {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub joined_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct ListMembersResponse {
    pub members: Vec<OrgMemberView>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub role: Option<String>,
    pub expires_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize)]
pub struct CreateInviteResponse {
    pub token: String,
    pub role: String,
    pub expires_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct AddMemberResponse {
    pub ok: bool,
}
