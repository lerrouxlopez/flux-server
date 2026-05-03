use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "typing.start")]
    TypingStart { channel_id: Uuid },
    #[serde(rename = "typing.stop")]
    TypingStop { channel_id: Uuid },
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
}
