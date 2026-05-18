use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "channel.subscribe")]
    ChannelSubscribe { channel_id: Uuid },
    #[serde(rename = "channel.unsubscribe")]
    ChannelUnsubscribe { channel_id: Uuid },
    #[serde(rename = "typing.start")]
    TypingStart { channel_id: Uuid },
    #[serde(rename = "typing.stop")]
    TypingStop { channel_id: Uuid },
    #[serde(rename = "media.subscribe")]
    MediaSubscribe { room_id: Uuid },
    #[serde(rename = "media.unsubscribe")]
    MediaUnsubscribe { room_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "message.created")]
    MessageCreated {
        organization_id: Uuid,
        channel_id: Uuid,
        message: Value,
    },
    #[serde(rename = "presence.changed")]
    PresenceChanged {
        organization_id: Uuid,
        user_id: Uuid,
        status: String,
    },
    #[serde(rename = "typing.started")]
    TypingStarted { channel_id: Uuid, user_id: Uuid },
    #[serde(rename = "typing.stopped")]
    TypingStopped { channel_id: Uuid, user_id: Uuid },

    #[serde(rename = "media.session.started")]
    MediaSessionStarted {
        organization_id: Uuid,
        room_id: Uuid,
        session_id: Uuid,
        started_at: String,
    },
    #[serde(rename = "media.session.ended")]
    MediaSessionEnded {
        organization_id: Uuid,
        room_id: Uuid,
        session_id: Uuid,
        ended_at: String,
        reason: Option<String>,
    },
    #[serde(rename = "media.participant.joined")]
    MediaParticipantJoined {
        organization_id: Uuid,
        room_id: Uuid,
        session_id: Uuid,
        participant_id: Uuid,
        user_id: Uuid,
        device_id: String,
        joined_at: String,
        can_subscribe: bool,
        can_publish_audio: bool,
        can_publish_video: bool,
        can_publish_screen: bool,
        can_publish_data: bool,
    },
    #[serde(rename = "media.participant.left")]
    MediaParticipantLeft {
        organization_id: Uuid,
        room_id: Uuid,
        session_id: Uuid,
        participant_id: Uuid,
        user_id: Uuid,
        device_id: String,
        left_at: String,
        reason: Option<String>,
    },
    #[serde(rename = "media.participant.updated")]
    MediaParticipantUpdated {
        organization_id: Uuid,
        room_id: Uuid,
        session_id: Uuid,
        participant_id: Uuid,
        user_id: Uuid,
        device_id: String,
        occurred_at: String,
        last_heartbeat_at: Option<String>,
    },
}
