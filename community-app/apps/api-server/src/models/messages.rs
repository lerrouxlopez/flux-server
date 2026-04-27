use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMessageRequest {
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReactionRequest {
    pub emoji: String,
}

#[derive(Debug, Serialize)]
pub struct MessageView {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub body: Option<String>,
    pub kind: String,
    pub created_at: OffsetDateTime,
    pub edited_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<MessageView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateMessageResponse {
    pub message: MessageView,
}

#[derive(Debug, Serialize)]
pub struct UpdateMessageResponse {
    pub message: MessageView,
}

#[derive(Debug, Serialize)]
pub struct DeleteMessageResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize)]
pub struct ReactionResponse {
    pub ok: bool,
}

