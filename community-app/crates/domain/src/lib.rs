use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

// ---- Organizations / membership ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: OffsetDateTime,
}

// ---- Channels / chat ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub kind: ChannelKind,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Text,
    Voice,
    Announcement,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub body: Option<String>,
    pub kind: MessageKind,
    pub created_at: OffsetDateTime,
    pub edited_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Text,
    System,
    Attachment,
}

// ---- Media rooms (LiveKit) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaRoom {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub livekit_room_name: String,
    pub kind: MediaRoomKind,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaRoomKind {
    Voice,
    Meeting,
    Stage,
}

// ---- Branding ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingProfile {
    pub organization_id: Uuid,
    pub app_name: String,
    pub logo_url: Option<String>,
    pub icon_url: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub custom_domain: Option<String>,
    pub email_from_name: Option<String>,
    pub privacy_url: Option<String>,
    pub terms_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// ---- Media sessions (durable lifecycle) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSession {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub media_room_id: Uuid,
    pub created_by: Uuid,
    pub started_at: OffsetDateTime,
    pub ended_at: Option<OffsetDateTime>,
    pub ended_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaParticipant {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub media_session_id: Uuid,
    pub user_id: Uuid,
    pub identity: String,
    pub can_subscribe: bool,
    pub can_publish_audio: bool,
    pub can_publish_video: bool,
    pub can_publish_screen: bool,
    pub can_publish_data: bool,
    pub joined_at: OffsetDateTime,
    pub last_heartbeat_at: OffsetDateTime,
    pub left_at: Option<OffsetDateTime>,
    pub left_reason: Option<String>,
    pub kick_attempted_at: Option<OffsetDateTime>,
    pub kicked_at: Option<OffsetDateTime>,
}
