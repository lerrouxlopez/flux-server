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

    // Preserve legacy semantics: treat the body booleans as "requested" (not trusted),
    // then cap by room kind + permission bits server-side.
    let requested = media::RequestedCapabilities {
        subscribe: req.can_subscribe,
        publish_audio: req.can_publish,
        publish_video: publish_video,
        publish_screen: publish_screen,
        publish_data: req.can_publish_data,
    };

    let joined = match media::join_room_with_requested(
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
        requested,
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
    device_id: String,
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
        "voice_only" => media::JoinIntent::VoiceOnly,
        "video" => media::JoinIntent::Video,
        "screen_share" => media::JoinIntent::ScreenShare,
        "stage_viewer" => media::JoinIntent::StageViewer,
        "stage_speaker" => media::JoinIntent::StageSpeaker,
        _ => return util::api_error(ApiErrorCode::ValidationError),
    };

    let livekit = media::LiveKitConfig {
        internal_url: state.livekit_url_internal.clone(),
        public_url: state.livekit_url_public.clone(),
        api_key: state.livekit_api_key.clone(),
        api_secret: state.livekit_api_secret.clone(),
    };

    let joined = match media::join_room_with_meta(
        &state.pool,
        &livekit,
        &room,
        auth.user_id,
        perms,
        media::JoinRequest {
            intent,
            device_id: req.device_id.clone(),
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

    let (resp, meta) = joined;
    let now = time::OffsetDateTime::now_utc();

    // Publish typed media events (best-effort, post-commit).
    if meta.session_started {
        let env = events::envelope::EventEnvelope::new(
            "media.session.started",
            room.organization_id,
            Some(auth.user_id),
            events::media::MediaSessionStartedData {
                room_id: room.id,
                session_id: resp.session_id,
                started_at: now,
            },
        );
        let subject = events::subjects::media_session_started(room.organization_id, room.id, resp.session_id);
        let _ = events::core::publish(&state.nats, subject, &env).await;
    }

    if meta.participant_reused {
        let env = events::envelope::EventEnvelope::new(
            "media.participant.updated",
            room.organization_id,
            Some(auth.user_id),
            events::media::MediaParticipantUpdatedData {
                room_id: room.id,
                session_id: resp.session_id,
                participant_id: resp.participant_id,
                user_id: auth.user_id,
                device_id: meta.device_id.clone(),
                occurred_at: now,
                last_heartbeat_at: Some(now),
            },
        );
        let subject = events::subjects::media_participant_updated(room.organization_id, room.id, resp.session_id);
        let _ = events::core::publish(&state.nats, subject, &env).await;
    } else {
        let env = events::envelope::EventEnvelope::new(
            "media.participant.joined",
            room.organization_id,
            Some(auth.user_id),
            events::media::MediaParticipantJoinedData {
                room_id: room.id,
                session_id: resp.session_id,
                participant_id: resp.participant_id,
                user_id: auth.user_id,
                device_id: meta.device_id.clone(),
                joined_at: now,
                can_subscribe: resp.granted.can_subscribe,
                can_publish_audio: resp.granted.can_publish_audio,
                can_publish_video: resp.granted.can_publish_video,
                can_publish_screen: resp.granted.can_publish_screen,
                can_publish_data: resp.granted.can_publish_data,
            },
        );
        let subject = events::subjects::media_participant_joined(room.organization_id, room.id, resp.session_id);
        let _ = events::core::publish(&state.nats, subject, &env).await;
    }

    (StatusCode::OK, Json(resp)).into_response()
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

#[derive(Debug, Deserialize)]
struct HeartbeatBody {
    device_id: Option<String>,
}

async fn heartbeat(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(session_id): Path<Uuid>,
    body: Option<Json<HeartbeatBody>>,
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

    let device_id = body.and_then(|Json(b)| b.device_id);
    let participant_id = if let Some(ref device_id) = device_id {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            select id
            from media_participants
            where media_session_id = $1
              and user_id = $2
              and device_id = $3
              and left_at is null
            order by joined_at desc
            limit 1
            "#,
        )
        .bind(session_id)
        .bind(auth.user_id)
        .bind(device_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    let ok = match device_id.as_deref() {
        Some(device_id) => media::heartbeat_device(&state.pool, session_id, auth.user_id, device_id).await,
        None => media::heartbeat(&state.pool, session_id, auth.user_id).await,
    };
    let ok = match ok {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if !ok {
        return util::api_error(ApiErrorCode::NotFound);
    }

    if let (Some(device_id), Some(participant_id)) = (device_id, participant_id) {
        let now = time::OffsetDateTime::now_utc();
        let env = events::envelope::EventEnvelope::new(
            "media.participant.updated",
            status.session.organization_id,
            Some(auth.user_id),
            events::media::MediaParticipantUpdatedData {
                room_id: status.session.media_room_id,
                session_id,
                participant_id,
                user_id: auth.user_id,
                device_id,
                occurred_at: now,
                last_heartbeat_at: Some(now),
            },
        );
        let subject = events::subjects::media_participant_updated(
            status.session.organization_id,
            status.session.media_room_id,
            session_id,
        );
        let _ = events::core::publish(&state.nats, subject, &env).await;
    }

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}

#[derive(Debug, Deserialize)]
struct LeaveBody {
    device_id: Option<String>,
}

async fn leave(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(session_id): Path<Uuid>,
    body: Option<Json<LeaveBody>>,
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

    let livekit = media::LiveKitConfig {
        internal_url: state.livekit_url_internal.clone(),
        public_url: state.livekit_url_public.clone(),
        api_key: state.livekit_api_key.clone(),
        api_secret: state.livekit_api_secret.clone(),
    };

    let device_id = body.and_then(|Json(b)| b.device_id);
    let meta = if let Some(ref device_id) = device_id {
        match media::leave_device_with_meta(&state.pool, &livekit, session_id, auth.user_id, device_id).await {
            Ok(m) => m,
            Err(_) => return util::api_error(ApiErrorCode::InternalError),
        }
    } else {
        None
    };

    let ok = if let Some(meta) = meta.as_ref() {
        Ok(true)
    } else {
        media::leave(&state.pool, &livekit, session_id, auth.user_id).await
    };
    let ok = match ok {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if !ok {
        return util::api_error(ApiErrorCode::NotFound);
    }

    if let Some(meta) = meta {
        let now = time::OffsetDateTime::now_utc();
        let env = events::envelope::EventEnvelope::new(
            "media.participant.left",
            status.session.organization_id,
            Some(auth.user_id),
            events::media::MediaParticipantLeftData {
                room_id: status.session.media_room_id,
                session_id,
                participant_id: meta.participant_id,
                user_id: auth.user_id,
                device_id: meta.device_id.clone(),
                left_at: now,
                reason: Some(meta.left_reason.clone()),
            },
        );
        let subject = events::subjects::media_participant_left(
            status.session.organization_id,
            status.session.media_room_id,
            session_id,
        );
        let _ = events::core::publish(&state.nats, subject, &env).await;

        if meta.session_ended {
            let ended_at = meta.ended_at.unwrap_or_else(time::OffsetDateTime::now_utc);
            let env = events::envelope::EventEnvelope::new(
                "media.session.ended",
                status.session.organization_id,
                Some(auth.user_id),
                events::media::MediaSessionEndedData {
                    room_id: status.session.media_room_id,
                    session_id,
                    ended_at,
                    reason: meta.ended_reason.clone(),
                },
            );
            let subject = events::subjects::media_session_ended(
                status.session.organization_id,
                status.session.media_room_id,
                session_id,
            );
            let _ = events::core::publish(&state.nats, subject, &env).await;
        }
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
