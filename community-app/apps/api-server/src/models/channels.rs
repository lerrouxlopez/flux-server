use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use domain::ChannelKind;

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub kind: ChannelKind,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub kind: Option<ChannelKind>,
}

#[derive(Debug, Serialize)]
pub struct ChannelView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub kind: ChannelKind,
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

#[derive(Debug, Serialize)]
pub struct GetChannelResponse {
    pub channel: ChannelView,
}

#[derive(Debug, Serialize)]
pub struct UpdateChannelResponse {
    pub channel: ChannelView,
}

#[derive(Debug, Serialize)]
pub struct DeleteChannelResponse {
    pub ok: bool,
}
