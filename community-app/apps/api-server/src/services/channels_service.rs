use crate::{
    models::auth::ApiError,
    models::channels::{ChannelView, CreateChannelRequest},
    models::channels::UpdateChannelRequest,
    repositories::ChannelRepository,
    services::PermissionsService,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct ChannelsService {
    channels: ChannelRepository,
    permissions: PermissionsService,
}

impl ChannelsService {
    pub fn new(channels: ChannelRepository, permissions: PermissionsService) -> Self {
        Self { channels, permissions }
    }

    pub async fn list_channels(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<ChannelView>, ApiError> {
        self.permissions
            .require(user_id, organization_id, "channels.view")
            .await?;

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
        self.permissions
            .require(user_id, organization_id, "channels.create")
            .await?;

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

    pub async fn get_channel(&self, user_id: Uuid, channel_id: Uuid) -> Result<ChannelView, ApiError> {
        let ch = self
            .channels
            .find_by_id(channel_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::not_found)?;

        self.permissions
            .require(user_id, ch.organization_id, "channels.view")
            .await?;

        Ok(ChannelView {
            id: ch.id,
            organization_id: ch.organization_id,
            name: ch.name,
            kind: ch.kind,
            created_at: ch.created_at,
        })
    }

    pub async fn update_channel(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        req: UpdateChannelRequest,
    ) -> Result<ChannelView, ApiError> {
        let existing = self
            .channels
            .find_by_id(channel_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::not_found)?;

        self.permissions
            .require(user_id, existing.organization_id, "channels.manage")
            .await?;

        if let Some(ref name) = req.name {
            let name = name.trim();
            if name.is_empty() || name.len() > 80 {
                return Err(ApiError::bad_request());
            }
        }

        let updated = self
            .channels
            .update_channel(
                channel_id,
                req.name.as_deref().map(|s| s.trim()),
                req.kind,
            )
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::not_found)?;

        Ok(ChannelView {
            id: updated.id,
            organization_id: updated.organization_id,
            name: updated.name,
            kind: updated.kind,
            created_at: updated.created_at,
        })
    }

    pub async fn delete_channel(&self, user_id: Uuid, channel_id: Uuid) -> Result<(), ApiError> {
        let existing = self
            .channels
            .find_by_id(channel_id)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(ApiError::not_found)?;

        self.permissions
            .require(user_id, existing.organization_id, "channels.manage")
            .await?;

        let ok = self
            .channels
            .delete_channel(channel_id)
            .await
            .map_err(|_| ApiError::internal())?;
        if !ok {
            return Err(ApiError::not_found());
        }
        Ok(())
    }
}
