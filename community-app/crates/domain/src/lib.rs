use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

pub type OrganizationId = Uuid;
pub type UserId = Uuid;
pub type ChannelId = Uuid;
pub type MessageId = Uuid;
pub type MediaRoomId = Uuid;

pub type Timestamp = OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Text,
    Voice,
    Announcement,
    Private,
}

impl ChannelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChannelKind::Text => "text",
            ChannelKind::Voice => "voice",
            ChannelKind::Announcement => "announcement",
            ChannelKind::Private => "private",
        }
    }
}

#[derive(Debug, Error)]
#[error("invalid enum value: {0}")]
pub struct InvalidEnumValue(pub String);

impl TryFrom<&str> for ChannelKind {
    type Error = InvalidEnumValue;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "text" => Ok(ChannelKind::Text),
            "voice" => Ok(ChannelKind::Voice),
            "announcement" => Ok(ChannelKind::Announcement),
            "private" => Ok(ChannelKind::Private),
            other => Err(InvalidEnumValue(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Text,
    System,
    Attachment,
}

impl MessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MessageKind::Text => "text",
            MessageKind::System => "system",
            MessageKind::Attachment => "attachment",
        }
    }
}

impl TryFrom<&str> for MessageKind {
    type Error = InvalidEnumValue;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "text" => Ok(MessageKind::Text),
            "system" => Ok(MessageKind::System),
            "attachment" => Ok(MessageKind::Attachment),
            other => Err(InvalidEnumValue(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaRoomKind {
    Voice,
    Meeting,
    Stage,
}

impl MediaRoomKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MediaRoomKind::Voice => "voice",
            MediaRoomKind::Meeting => "meeting",
            MediaRoomKind::Stage => "stage",
        }
    }
}

impl TryFrom<&str> for MediaRoomKind {
    type Error = InvalidEnumValue;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "voice" => Ok(MediaRoomKind::Voice),
            "meeting" => Ok(MediaRoomKind::Meeting),
            "stage" => Ok(MediaRoomKind::Stage),
            other => Err(InvalidEnumValue(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub slug: String,
    pub name: String,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub organization_id: OrganizationId,
    pub user_id: UserId,
    pub role: String,
    pub joined_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: ChannelId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub kind: ChannelKind,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub organization_id: OrganizationId,
    pub channel_id: ChannelId,
    pub sender_id: UserId,
    pub body: Option<String>,
    pub kind: MessageKind,
    pub created_at: Timestamp,
    pub edited_at: Option<Timestamp>,
    pub deleted_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaRoom {
    pub id: MediaRoomId,
    pub organization_id: OrganizationId,
    pub channel_id: Option<ChannelId>,
    pub livekit_room_name: String,
    pub kind: MediaRoomKind,
    pub created_by: UserId,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingProfile {
    pub organization_id: OrganizationId,
    pub app_name: String,
    pub logo_url: Option<String>,
    pub icon_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub custom_domain: Option<String>,
    pub email_from_name: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
