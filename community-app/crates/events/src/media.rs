use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSessionStartedData {
    pub room_id: Uuid,
    pub session_id: Uuid,
    pub started_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSessionEndedData {
    pub room_id: Uuid,
    pub session_id: Uuid,
    pub ended_at: OffsetDateTime,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaParticipantJoinedData {
    pub room_id: Uuid,
    pub session_id: Uuid,
    pub participant_id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub joined_at: OffsetDateTime,
    pub can_subscribe: bool,
    pub can_publish_audio: bool,
    pub can_publish_video: bool,
    pub can_publish_screen: bool,
    pub can_publish_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaParticipantLeftData {
    pub room_id: Uuid,
    pub session_id: Uuid,
    pub participant_id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub left_at: OffsetDateTime,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaParticipantUpdatedData {
    pub room_id: Uuid,
    pub session_id: Uuid,
    pub participant_id: Uuid,
    pub user_id: Uuid,
    pub device_id: String,
    pub occurred_at: OffsetDateTime,
    pub last_heartbeat_at: Option<OffsetDateTime>,
}

