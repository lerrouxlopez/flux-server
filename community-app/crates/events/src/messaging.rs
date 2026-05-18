use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadReplyCreatedData {
    pub channel_id: Uuid,
    pub thread_id: Uuid,
    pub thread_root_id: Uuid,
    pub message_id: Uuid,
    pub occurred_at: OffsetDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPinsChangedData {
    pub channel_id: Uuid,
    pub message_id: Uuid,
    pub action: String, // "pinned" | "unpinned"
    pub occurred_at: OffsetDateTime,
}

