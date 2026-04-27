use crate::{
    models::{auth, channels},
    state::AppState,
};
use axum::{
    routing::{delete, get, patch, post},
    Json, Router,
};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orgs/:org_id/channels", get(list_org_channels))
        .route("/orgs/:org_id/channels", post(create_org_channel))
        .route("/channels/:channel_id", get(get_channel))
        .route("/channels/:channel_id", patch(update_channel))
        .route("/channels/:channel_id", delete(delete_channel))
}

async fn list_org_channels(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(org_id): axum::extract::Path<Uuid>,
) -> Result<Json<channels::ListChannelsResponse>, auth::ApiError> {
    let channels = state
        .channels_service
        .list_channels(ctx.user_id, org_id)
        .await?;
    Ok(Json(channels::ListChannelsResponse { channels }))
}

async fn create_org_channel(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(org_id): axum::extract::Path<Uuid>,
    Json(req): Json<channels::CreateChannelRequest>,
) -> Result<Json<channels::CreateChannelResponse>, auth::ApiError> {
    let channel = state
        .channels_service
        .create_channel(ctx.user_id, org_id, req)
        .await?;
    Ok(Json(channels::CreateChannelResponse { channel }))
}

async fn get_channel(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(channel_id): axum::extract::Path<Uuid>,
) -> Result<Json<channels::GetChannelResponse>, auth::ApiError> {
    let channel = state.channels_service.get_channel(ctx.user_id, channel_id).await?;
    Ok(Json(channels::GetChannelResponse { channel }))
}

async fn update_channel(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(channel_id): axum::extract::Path<Uuid>,
    Json(req): Json<channels::UpdateChannelRequest>,
) -> Result<Json<channels::UpdateChannelResponse>, auth::ApiError> {
    let channel = state
        .channels_service
        .update_channel(ctx.user_id, channel_id, req)
        .await?;
    Ok(Json(channels::UpdateChannelResponse { channel }))
}

async fn delete_channel(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    axum::extract::Path(channel_id): axum::extract::Path<Uuid>,
) -> Result<Json<channels::DeleteChannelResponse>, auth::ApiError> {
    state
        .channels_service
        .delete_channel(ctx.user_id, channel_id)
        .await?;
    Ok(Json(channels::DeleteChannelResponse { ok: true }))
}

