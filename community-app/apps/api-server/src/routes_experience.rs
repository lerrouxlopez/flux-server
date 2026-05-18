use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Extension, Router,
};
use permissions::perms;
use serde::{Deserialize, Serialize};
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/experience/context", get(get_experience_context))
        .route("/experience/preferences", patch(patch_experience_preferences))
}

#[derive(Debug, Deserialize)]
struct ExperienceContextQuery {
    org_id: Uuid,
    channel_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct ExperienceContextResponse {
    mode: String,   // "work" | "play"
    source: String, // "user_preference" | "channel_hint" | "org_default" | "preset_default"

    density: String,               // "comfortable" | "compact"
    motion: String,                // "full" | "reduced"
    notification_profile: String,  // "all" | "minimal"
    media_defaults: serde_json::Value,
    feature_flags: serde_json::Value,
}

async fn get_experience_context(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Query(q): Query<ExperienceContextQuery>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(q.org_id));

    let perms_v = match util::member_perms(&state.pool, q.org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::CHANNELS_VIEW) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    if let Some(channel_id) = q.channel_id {
        // Ensure channel belongs to org + user can access it (DM rules included).
        let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
            Ok(v) => v,
            Err(e) => return e,
        };
        if !can_access {
            return util::api_error(ApiErrorCode::PermissionDenied);
        }
        let org_ok: Option<Uuid> = sqlx::query_scalar(
            r#"select organization_id from channels where id = $1"#,
        )
        .bind(channel_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
        if org_ok != Some(q.org_id) {
            return util::api_error(ApiErrorCode::PermissionDenied);
        }
    }

    let (mode, source) = match resolve_mode(&state.pool, auth.user_id, q.org_id, q.channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let (density, motion, notification_profile) = if mode == "play" {
        ("compact", "reduced", "minimal")
    } else {
        ("comfortable", "full", "all")
    };

    let media_defaults = if mode == "play" {
        serde_json::json!({
            "room_kind_preference": "voice",
            "join_intent": "voice_only",
            "auto_publish_audio": true,
            "auto_publish_video": false,
            "auto_publish_screen": false,
            "auto_subscribe": true
        })
    } else {
        serde_json::json!({
            "room_kind_preference": "meeting",
            "join_intent": "video",
            "auto_publish_audio": true,
            "auto_publish_video": true,
            "auto_publish_screen": false,
            "auto_subscribe": true
        })
    };

    // Suggested feature flags (server-provided toggles; client may ignore until wired).
    let feature_flags = if mode == "play" {
        serde_json::json!({
            "work_panes": false,
            "threads": true,
            "pins": true,
            "channel_search": true,
            "voice_dock": true,
            "meeting_room": false
        })
    } else {
        serde_json::json!({
            "work_panes": true,
            "threads": true,
            "pins": true,
            "channel_search": true,
            "voice_dock": true,
            "meeting_room": true
        })
    };

    (
        StatusCode::OK,
        Json(ExperienceContextResponse {
            mode,
            source,
            density: density.to_string(),
            motion: motion.to_string(),
            notification_profile: notification_profile.to_string(),
            media_defaults,
            feature_flags,
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct PatchExperiencePreferencesRequest {
    mode_preference: Option<String>, // null clears
}

async fn patch_experience_preferences(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<PatchExperiencePreferencesRequest>,
) -> impl IntoResponse {
    let mode = req
        .mode_preference
        .as_ref()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty());

    if mode.as_deref().is_some_and(|m| m != "work" && m != "play") {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let res = sqlx::query(
        r#"
        update users
        set experience_mode_preference = $2
        where id = $1
        "#,
    )
    .bind(auth.user_id)
    .bind(mode.clone())
    .execute(&state.pool)
    .await;

    match res {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "mode_preference": mode,
            })),
        )
            .into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn resolve_mode(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    org_id: Uuid,
    channel_id: Option<Uuid>,
) -> Result<(String, String), axum::response::Response> {
    // 1) user preference (global)
    let user_pref: Option<String> = sqlx::query_scalar(
        r#"select experience_mode_preference from users where id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
    .flatten();
    if let Some(m) = user_pref.as_deref() {
        if m == "work" || m == "play" {
            return Ok((m.to_string(), "user_preference".to_string()));
        }
    }

    // 2) channel hint
    if let Some(channel_id) = channel_id {
        let hint: Option<String> = sqlx::query_scalar(
            r#"select experience_mode_hint from channels where id = $1 and organization_id = $2"#,
        )
        .bind(channel_id)
        .bind(org_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
        .flatten();
        if let Some(m) = hint.as_deref() {
            if m == "work" || m == "play" {
                return Ok((m.to_string(), "channel_hint".to_string()));
            }
        }
    }

    // 3) org default
    let org_default: Option<String> = sqlx::query_scalar(
        r#"select experience_default_mode from organizations where id = $1"#,
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
    .flatten();
    if let Some(m) = org_default.as_deref() {
        if m == "work" || m == "play" {
            return Ok((m.to_string(), "org_default".to_string()));
        }
    }

    Ok(("work".to_string(), "preset_default".to_string()))
}
