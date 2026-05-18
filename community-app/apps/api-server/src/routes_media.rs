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
        .route("/media/rooms/{room_id}/join", post(join_room))
        .route("/media/rooms/{room_id}/token", post(issue_token))
        .route(
            "/media/rooms/{room_id}/participants",
            get(list_participants),
        )
        .route("/media/sessions/{session_id}", get(get_session_status))
        .route("/media/sessions/{session_id}/heartbeat", post(heartbeat))
        .route("/media/sessions/{session_id}/leave", post(leave))
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

    // Back-compat: preserve the old token behavior, but route through the durable join path.
    let token_req = media::TokenRequest {
        can_publish: req.can_publish,
        can_subscribe: req.can_subscribe,
        can_publish_data: req.can_publish_data,
    };
    let needed = media::required_perms_for_token(room.kind, &token_req);
    if !permissions::has(perms, needed) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let publish_video = req.can_publish
        && matches!(
            room.kind,
            domain::MediaRoomKind::Meeting | domain::MediaRoomKind::Stage
        );
    let publish_screen = publish_video;

    let join_req = media::JoinRequest {
        intent: if matches!(room.kind, domain::MediaRoomKind::Voice) {
            media::JoinIntent::Voice
        } else {
            media::JoinIntent::Meeting
        },
        publish_audio: Some(req.can_publish),
        publish_video: Some(publish_video),
        publish_screen: Some(publish_screen),
        publish_data: Some(req.can_publish_data),
        subscribe: Some(req.can_subscribe),
    };

    let joined = match media::join_room(
        &state.pool,
        &media::LiveKitConfig {
            internal_url: state.livekit_url_internal.clone(),
            public_url: state.livekit_url_public.clone(),
            api_key: state.livekit_api_key.clone(),
            api_secret: state.livekit_api_secret.clone(),
        },
        &room,
        auth.user_id,
        perms,
        join_req,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            if e.to_string().contains("permission denied") {
                return util::api_error(ApiErrorCode::PermissionDenied);
            }
            return util::api_error(ApiErrorCode::InternalError);
        }
    };

    (StatusCode::OK, Json(media::TokenResponse { token: joined.token, livekit_url: joined.livekit_url })).into_response()
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

#[derive(Debug, Deserialize)]
struct JoinRoomRequest {
    intent: String,
    #[serde(default)]
    publish_audio: Option<bool>,
    #[serde(default)]
    publish_video: Option<bool>,
    #[serde(default)]
    publish_screen: Option<bool>,
    #[serde(default)]
    publish_data: Option<bool>,
    #[serde(default)]
    subscribe: Option<bool>,
}

async fn join_room(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(room_id): Path<Uuid>,
    Json(req): Json<JoinRoomRequest>,
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

    let intent = match req.intent.trim().to_lowercase().as_str() {
        "voice" => media::JoinIntent::Voice,
        "meeting" => media::JoinIntent::Meeting,
        _ => return util::api_error(ApiErrorCode::ValidationError),
    };

    let joined = match media::join_room(
        &state.pool,
        &media::LiveKitConfig {
            internal_url: state.livekit_url_internal.clone(),
            public_url: state.livekit_url_public.clone(),
            api_key: state.livekit_api_key.clone(),
            api_secret: state.livekit_api_secret.clone(),
        },
        &room,
        auth.user_id,
        perms,
        media::JoinRequest {
            intent,
            publish_audio: req.publish_audio,
            publish_video: req.publish_video,
            publish_screen: req.publish_screen,
            publish_data: req.publish_data,
            subscribe: req.subscribe,
        },
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            if e.to_string().contains("permission denied") {
                return util::api_error(ApiErrorCode::PermissionDenied);
            }
            return util::api_error(ApiErrorCode::InternalError);
        }
    };

    (StatusCode::OK, Json(joined)).into_response()
}

async fn get_session_status(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(session_id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(status) = (match media::get_session_status(&state.pool, session_id).await {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    // Org-scoped access: must be member and allowed to join voice.
    let perms = match util::member_perms(&state.pool, status.session.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(status.session.organization_id));
    if !permissions::has(perms, perms::VOICE_JOIN) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    (StatusCode::OK, Json(status)).into_response()
}

async fn heartbeat(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(session_id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(status) = (match media::get_session_status(&state.pool, session_id).await {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let perms = match util::member_perms(&state.pool, status.session.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(status.session.organization_id));
    if !permissions::has(perms, perms::VOICE_JOIN) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let ok = match media::heartbeat(&state.pool, session_id, auth.user_id).await {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if !ok {
        return util::api_error(ApiErrorCode::NotFound);
    }

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}

async fn leave(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(session_id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(status) = (match media::get_session_status(&state.pool, session_id).await {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let perms = match util::member_perms(&state.pool, status.session.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(status.session.organization_id));
    if !permissions::has(perms, perms::VOICE_JOIN) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let ok = match media::leave(
        &state.pool,
        &media::LiveKitConfig {
            internal_url: state.livekit_url_internal.clone(),
            public_url: state.livekit_url_public.clone(),
            api_key: state.livekit_api_key.clone(),
            api_secret: state.livekit_api_secret.clone(),
        },
        session_id,
        auth.user_id,
    )
    .await
    {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if !ok {
        return util::api_error(ApiErrorCode::NotFound);
    }

    (StatusCode::OK, Json(serde_json::json!({"status":"left"}))).into_response()
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
