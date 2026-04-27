use crate::{
    models::{auth, channels},
    state::AppState,
};
use axum::{
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels", get(list_channels))
        .route("/channels", post(create_channel))
}

async fn list_channels(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    headers: HeaderMap,
) -> Result<Json<channels::ListChannelsResponse>, auth::ApiError> {
    let requested_org_id = parse_org_id_header(&headers)?;
    let org = state
        .orgs_service
        .resolve_org_for_user(ctx.user_id, requested_org_id)
        .await?;

    let channels = state
        .channels_service
        .list_channels(ctx.user_id, org.id)
        .await?;
    Ok(Json(channels::ListChannelsResponse { channels }))
}

async fn create_channel(
    axum::extract::State(state): axum::extract::State<AppState>,
    auth::CurrentAuth(ctx): auth::CurrentAuth,
    headers: HeaderMap,
    Json(req): Json<channels::CreateChannelRequest>,
) -> Result<Json<channels::CreateChannelResponse>, auth::ApiError> {
    let requested_org_id = parse_org_id_header(&headers)?;
    let org = state
        .orgs_service
        .resolve_org_for_user(ctx.user_id, requested_org_id)
        .await?;

    let channel = state
        .channels_service
        .create_channel(ctx.user_id, org.id, req)
        .await?;

    Ok(Json(channels::CreateChannelResponse { channel }))
}

fn parse_org_id_header(headers: &HeaderMap) -> Result<Option<Uuid>, auth::ApiError> {
    let Some(v) = headers.get("x-organization-id") else {
        return Ok(None);
    };
    let s = v.to_str().map_err(|_| auth::ApiError::bad_request())?;
    let id = Uuid::parse_str(s).map_err(|_| auth::ApiError::bad_request())?;
    Ok(Some(id))
}

