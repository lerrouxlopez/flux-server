use crate::{
    models::auth::ApiError,
    models::channels::{ChannelView, CreateChannelRequest},
    repositories::{ChannelRepository, MembershipRepository},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct ChannelsService {
    channels: ChannelRepository,
    memberships: MembershipRepository,
}

impl ChannelsService {
    pub fn new(channels: ChannelRepository, memberships: MembershipRepository) -> Self {
        Self {
            channels,
            memberships,
        }
    }

    pub async fn list_channels(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<ChannelView>, ApiError> {
        self.ensure_member(user_id, organization_id).await?;

        let rows = self
            .channels
            .list_channels_by_org(organization_id, 200)
            .await
            .map_err(|_| ApiError::internal())?;

        Ok(rows
            .into_iter()
            .map(|c| ChannelView {
                id: c.id,
                organization_id: c.organization_id,
                name: c.name,
                kind: c.kind,
                created_at: c.created_at,
            })
            .collect())
    }

    pub async fn create_channel(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        req: CreateChannelRequest,
    ) -> Result<ChannelView, ApiError> {
        self.ensure_member(user_id, organization_id).await?;

        let name = req.name.trim();
        if name.is_empty() || name.len() > 80 {
            return Err(ApiError::bad_request());
        }

        let row = self
            .channels
            .insert_channel(Uuid::now_v7(), organization_id, name, req.kind)
            .await
            .map_err(|_| ApiError::internal())?;

        Ok(ChannelView {
            id: row.id,
            organization_id: row.organization_id,
            name: row.name,
            kind: row.kind,
            created_at: row.created_at,
        })
    }

    async fn ensure_member(&self, user_id: Uuid, organization_id: Uuid) -> Result<(), ApiError> {
        let is_member = self
            .memberships
            .find_membership(organization_id, user_id)
            .await
            .map_err(|_| ApiError::internal())?
            .is_some();
        if !is_member {
            return Err(ApiError::unauthorized());
        }
        Ok(())
    }
}
