use crate::{
    models::{auth, messages},
    state::AppState,
};
use axum::{
    extract::Query,
    routing::{delete, get, patch, post},
    Json, Router,
};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(serde::Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    before: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels/:channel_id/messages", get(list_messages))
        .route("/channels/:channel_id/messages", post(create_message))
        .route("/messages/:message_id", patch(update_message))
        .route("/messages/:message_id", delete(delete_message))
        .route("/messages/:message_id/reactions", post(add_reaction))
        .route(
            "/messages/:message_id/reactions/:emoji",
            delete(remove_reaction),
        )
}

async fn list_messages(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(channel_id): axum::extract::Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> Result<Json<messages::ListMessagesResponse>, auth::ApiError> {
    let limit = q.limit.unwrap_or(50);

    let mut before_id: Option<Uuid> = None;
    let mut before_ts: Option<OffsetDateTime> = None;
    if let Some(before) = q.before {
        if let Ok(id) = Uuid::parse_str(&before) {
            before_id = Some(id);
        } else if let Ok(ts) = OffsetDateTime::parse(&before, &time::format_description::well_known::Rfc3339) {
            before_ts = Some(ts);
        } else {
            return Err(auth::ApiError::bad_request());
        }
    }

    let res = state
        .messages_service
        .list_messages(ctx.user_id, channel_id, limit, before_id, before_ts)
        .await?;

    Ok(Json(messages::ListMessagesResponse {
        messages: res
            .messages
            .into_iter()
            .map(map_message)
            .collect(),
        next_cursor: res.next_cursor,
    }))
}

async fn create_message(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(channel_id): axum::extract::Path<Uuid>,
    Json(req): Json<messages::CreateMessageRequest>,
) -> Result<Json<messages::CreateMessageResponse>, auth::ApiError> {
    let body = req.body.trim().to_string();
    if body.is_empty() || body.len() > 4000 {
        return Err(auth::ApiError::bad_request());
    }

    let msg = state
        .messages_service
        .create_message(ctx.user_id, channel_id, Some(body))
        .await?;

    Ok(Json(messages::CreateMessageResponse {
        message: map_message(msg),
    }))
}

async fn update_message(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(message_id): axum::extract::Path<Uuid>,
    Json(req): Json<messages::UpdateMessageRequest>,
) -> Result<Json<messages::UpdateMessageResponse>, auth::ApiError> {
    let body = req.body.trim().to_string();
    if body.is_empty() || body.len() > 4000 {
        return Err(auth::ApiError::bad_request());
    }

    let msg = state
        .messages_service
        .update_message(ctx.user_id, message_id, body)
        .await?;

    Ok(Json(messages::UpdateMessageResponse {
        message: map_message(msg),
    }))
}

async fn delete_message(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(message_id): axum::extract::Path<Uuid>,
) -> Result<Json<messages::DeleteMessageResponse>, auth::ApiError> {
    state
        .messages_service
        .delete_message(ctx.user_id, message_id)
        .await?;
    Ok(Json(messages::DeleteMessageResponse { ok: true }))
}

async fn add_reaction(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(message_id): axum::extract::Path<Uuid>,
    Json(req): Json<messages::CreateReactionRequest>,
) -> Result<Json<messages::ReactionResponse>, auth::ApiError> {
    let emoji = req.emoji.trim().to_string();
    if emoji.is_empty() || emoji.len() > 32 {
        return Err(auth::ApiError::bad_request());
    }
    state
        .messages_service
        .add_reaction(ctx.user_id, message_id, emoji)
        .await?;
    Ok(Json(messages::ReactionResponse { ok: true }))
}

async fn remove_reaction(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path((message_id, emoji)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<messages::ReactionResponse>, auth::ApiError> {
    let emoji = emoji.trim().to_string();
    if emoji.is_empty() || emoji.len() > 32 {
        return Err(auth::ApiError::bad_request());
    }
    state
        .messages_service
        .remove_reaction(ctx.user_id, message_id, emoji)
        .await?;
    Ok(Json(messages::ReactionResponse { ok: true }))
}

fn map_message(m: crate::services::messages_service::MessageView) -> messages::MessageView {
    messages::MessageView {
        id: m.id,
        organization_id: m.organization_id,
        channel_id: m.channel_id,
        sender_id: m.sender_id,
        body: m.body,
        kind: m.kind,
        created_at: m.created_at,
        edited_at: m.edited_at,
        deleted_at: m.deleted_at,
    }
}

