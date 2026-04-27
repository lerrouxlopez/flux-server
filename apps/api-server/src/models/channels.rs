use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct ChannelView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub kind: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct ListChannelsResponse {
    pub channels: Vec<ChannelView>,
}

#[derive(Debug, Serialize)]
pub struct CreateChannelResponse {
    pub channel: ChannelView,
}

