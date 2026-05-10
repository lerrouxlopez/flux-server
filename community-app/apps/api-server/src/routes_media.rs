use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use permissions::perms;
use serde::{Deserialize, Serialize};
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org_id}/media/rooms", post(create_media_room))
        .route("/media/rooms/{room_id}", get(get_media_room))
        .route("/media/rooms/{room_id}/token", post(issue_token))
        .route(
            "/media/rooms/{room_id}/participants",
            get(list_participants),
        )
}

#[derive(Debug, Deserialize)]
struct CreateMediaRoomRequest {
    kind: String,
    channel_id: Option<Uuid>,
    name: String,
}

#[derive(Debug, Serialize)]
struct MediaRoomResponse {
    id: Uuid,
    organization_id: Uuid,
    channel_id: Option<Uuid>,
    livekit_room_name: String,
    kind: String,
    name: String,
    created_by: Uuid,
    created_at: time::OffsetDateTime,
}

async fn create_media_room(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateMediaRoomRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::MEDIA_ROOMS_CREATE) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let kind = match normalize_room_kind(&req.kind) {
        Ok(k) => k,
        Err(e) => return *e,
    };

    let room = match media::create_media_room(
        &state.pool,
        org_id,
        req.channel_id,
        kind,
        name,
        auth.user_id,
    )
    .await
    {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    (
        StatusCode::OK,
        Json(MediaRoomResponse {
            id: room.id,
            organization_id: room.organization_id,
            channel_id: room.channel_id,
            livekit_room_name: room.livekit_room_name,
            kind: match room.kind {
                domain::MediaRoomKind::Voice => "voice",
                domain::MediaRoomKind::Meeting => "meeting",
                domain::MediaRoomKind::Stage => "stage",
            }
            .to_string(),
            name: room.name,
            created_by: room.created_by,
            created_at: room.created_at,
        }),
    )
        .into_response()
}

async fn get_media_room(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(room_id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(room) = (match media::get_media_room(&state.pool, room_id).await {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    // Must be org member to view.
    let _perms = match util::member_perms(&state.pool, room.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    Span::current().record(
        "organization_id",
        tracing::field::display(room.organization_id),
    );

    (
        StatusCode::OK,
        Json(MediaRoomResponse {
            id: room.id,
            organization_id: room.organization_id,
            channel_id: room.channel_id,
            livekit_room_name: room.livekit_room_name,
            kind: match room.kind {
                domain::MediaRoomKind::Voice => "voice",
                domain::MediaRoomKind::Meeting => "meeting",
                domain::MediaRoomKind::Stage => "stage",
            }
            .to_string(),
            name: room.name,
            created_by: room.created_by,
            created_at: room.created_at,
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct IssueTokenRequest {
    can_publish: bool,
    can_subscribe: bool,
    can_publish_data: bool,
}

async fn issue_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(room_id): Path<Uuid>,
    Json(req): Json<IssueTokenRequest>,
) -> impl IntoResponse {
    let Some(room) = (match media::get_media_room(&state.pool, room_id).await {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let perms = match util::member_perms(&state.pool, room.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    Span::current().record(
        "organization_id",
        tracing::field::display(room.organization_id),
    );

    let token_req = media::TokenRequest {
        can_publish: req.can_publish,
        can_subscribe: req.can_subscribe,
        can_publish_data: req.can_publish_data,
    };

    let needed = media::required_perms_for_token(room.kind, &token_req);
    if !permissions::has(perms, needed) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let resp = match media::issue_livekit_token(
        &media::LiveKitConfig {
            internal_url: state.livekit_url_internal.clone(),
            public_url: state.livekit_url_public.clone(),
            api_key: state.livekit_api_key.clone(),
            api_secret: state.livekit_api_secret.clone(),
        },
        auth.user_id.to_string(),
        room.livekit_room_name,
        &token_req,
    ) {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    (StatusCode::OK, Json(resp)).into_response()
}

async fn list_participants(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(room_id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(room) = (match media::get_media_room(&state.pool, room_id).await {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let perms = match util::member_perms(&state.pool, room.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    Span::current().record(
        "organization_id",
        tracing::field::display(room.organization_id),
    );
    if !permissions::has(perms, perms::VOICE_JOIN) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let v = match media::list_participants(
        &media::LiveKitConfig {
            internal_url: state.livekit_url_internal.clone(),
            public_url: state.livekit_url_public.clone(),
            api_key: state.livekit_api_key.clone(),
            api_secret: state.livekit_api_secret.clone(),
        },
        &room.livekit_room_name,
    )
    .await
    {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    (StatusCode::OK, Json(v)).into_response()
}

fn normalize_room_kind(
    input: &str,
) -> Result<domain::MediaRoomKind, Box<axum::response::Response>> {
    let k = input.trim().to_lowercase();
    match k.as_str() {
        "voice" => Ok(domain::MediaRoomKind::Voice),
        "meeting" => Ok(domain::MediaRoomKind::Meeting),
        "stage" => Ok(domain::MediaRoomKind::Stage),
        _ => Err(Box::new(util::api_error(ApiErrorCode::ValidationError))),
    }
}
