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

