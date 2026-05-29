use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Router,
};
use permissions::perms;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications/context", get(get_notifications_context))
        .route("/notifications/overrides/user", patch(patch_user_override))
        .route("/notifications/overrides/channel", patch(patch_channel_override))
}

#[derive(Debug, Deserialize)]
struct NotificationsContextQuery {
    org_id: Uuid,
    channel_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct NotificationsContextResponse {
    mode: String,
    profile_source: String,
    profile_id: Option<Uuid>,
    behavior: notifications::NotificationBehavior,
}

async fn get_notifications_context(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<NotificationsContextQuery>,
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

    let (mode, _mode_source) = match resolve_mode(&state.pool, auth.user_id, q.org_id, q.channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let resolved = match resolve_notification_profile(&state.pool, auth.user_id, q.org_id, q.channel_id, &mode).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    (
        StatusCode::OK,
        Json(NotificationsContextResponse {
            mode,
            profile_source: resolved.source,
            profile_id: resolved.profile_id,
            behavior: resolved.behavior,
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct PatchUserOverrideRequest {
    org_id: Uuid,
    mode: String,              // work|play
    profile_id: Option<Uuid>,  // null clears
}

async fn patch_user_override(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<PatchUserOverrideRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(req.org_id));

    let mode = req.mode.trim().to_lowercase();
    if mode != "work" && mode != "play" {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let perms_v = match util::member_perms(&state.pool, req.org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::CHANNELS_VIEW) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    if let Some(pid) = req.profile_id {
        let ok: Option<bool> = sqlx::query_scalar(
            r#"
            select true
            from notification_profiles
            where id = $1 and (organization_id = $2 or organization_id is null)
            "#,
        )
        .bind(pid)
        .bind(req.org_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
        if ok != Some(true) {
            return util::api_error(ApiErrorCode::ValidationError);
        }
    }

    let res = sqlx::query(
        r#"
        insert into user_notification_overrides (organization_id, user_id, mode, profile_id)
        values ($1, $2, $3, $4)
        on conflict (organization_id, user_id, mode)
        do update set profile_id = excluded.profile_id, updated_at = now()
        "#,
    )
    .bind(req.org_id)
    .bind(auth.user_id)
    .bind(mode.clone())
    .bind(req.profile_id)
    .execute(&state.pool)
    .await;

    match res {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "ok" })),
        )
            .into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

#[derive(Debug, Deserialize)]
struct PatchChannelOverrideRequest {
    org_id: Uuid,
    channel_id: Uuid,
    profile_id: Option<Uuid>, // null clears
}

async fn patch_channel_override(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<PatchChannelOverrideRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(req.org_id));

    let can_access = match util::can_access_channel(&state.pool, auth.user_id, req.channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }
    let org_ok: Option<Uuid> = sqlx::query_scalar(
        r#"select organization_id from channels where id = $1"#,
    )
    .bind(req.channel_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    if org_ok != Some(req.org_id) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    if let Some(pid) = req.profile_id {
        let ok: Option<bool> = sqlx::query_scalar(
            r#"
            select true
            from notification_profiles
            where id = $1 and (organization_id = $2 or organization_id is null)
            "#,
        )
        .bind(pid)
        .bind(req.org_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
        if ok != Some(true) {
            return util::api_error(ApiErrorCode::ValidationError);
        }
    }

    let res = sqlx::query(
        r#"
        insert into channel_notification_overrides (channel_id, user_id, profile_id)
        values ($1, $2, $3)
        on conflict (channel_id, user_id)
        do update set profile_id = excluded.profile_id, updated_at = now()
        "#,
    )
    .bind(req.channel_id)
    .bind(auth.user_id)
    .bind(req.profile_id)
    .execute(&state.pool)
    .await;

    match res {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "ok" })),
        )
            .into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn resolve_notification_profile(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    org_id: Uuid,
    channel_id: Option<Uuid>,
    mode: &str,
) -> Result<notifications::ResolvedNotificationProfile, axum::response::Response> {
    // 1) user override (org-scoped, mode-aware)
    let user_override: Option<Uuid> = sqlx::query_scalar(
        r#"
        select profile_id
        from user_notification_overrides
        where organization_id = $1 and user_id = $2 and mode = $3
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(mode)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
    .flatten();
    if let Some(pid) = user_override {
        let behavior = load_profile_behavior(pool, pid).await?;
        return Ok(notifications::ResolvedNotificationProfile {
            profile_id: Some(pid),
            source: "user_override".to_string(),
            behavior,
        });
    }

    // 2) channel override
    if let Some(channel_id) = channel_id {
        let channel_override: Option<Uuid> = sqlx::query_scalar(
            r#"
            select profile_id
            from channel_notification_overrides
            where channel_id = $1 and user_id = $2
            "#,
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
        .flatten();
        if let Some(pid) = channel_override {
            let behavior = load_profile_behavior(pool, pid).await?;
            return Ok(notifications::ResolvedNotificationProfile {
                profile_id: Some(pid),
                source: "channel_override".to_string(),
                behavior,
            });
        }
    }

    // 3) mode profile (org configured)
    let mode_profile: Option<Uuid> = if mode == "play" {
        sqlx::query_scalar(
            r#"select notification_play_profile_id from organizations where id = $1"#,
        )
        .bind(org_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
        .flatten()
    } else {
        sqlx::query_scalar(
            r#"select notification_work_profile_id from organizations where id = $1"#,
        )
        .bind(org_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
        .flatten()
    };
    if let Some(pid) = mode_profile {
        let behavior = load_profile_behavior(pool, pid).await?;
        return Ok(notifications::ResolvedNotificationProfile {
            profile_id: Some(pid),
            source: "mode_profile".to_string(),
            behavior,
        });
    }

    // 4) org default
    let org_default: Option<Uuid> = sqlx::query_scalar(
        r#"select notification_default_profile_id from organizations where id = $1"#,
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
    .flatten();
    if let Some(pid) = org_default {
        let behavior = load_profile_behavior(pool, pid).await?;
        return Ok(notifications::ResolvedNotificationProfile {
            profile_id: Some(pid),
            source: "org_default".to_string(),
            behavior,
        });
    }

    // 5) platform default
    let platform_id = if mode == "play" {
        Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
    } else {
        Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
    };
    let behavior = load_profile_behavior(pool, platform_id).await.unwrap_or_default();
    Ok(notifications::ResolvedNotificationProfile {
        profile_id: Some(platform_id),
        source: "platform_default".to_string(),
        behavior,
    })
}

async fn load_profile_behavior(
    pool: &sqlx::PgPool,
    profile_id: Uuid,
) -> Result<notifications::NotificationBehavior, axum::response::Response> {
    let rows = sqlx::query(
        r#"
        select rule, enabled
        from notification_profile_rules
        where profile_id = $1
        "#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    let mut m: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for r in rows {
        let rule: String = r.try_get("rule").unwrap_or_default();
        let enabled: bool = r.try_get("enabled").unwrap_or(false);
        m.insert(rule, enabled);
    }
    Ok(notifications::NotificationBehavior::from_rules(&m))
}

// Keep experience-mode resolution internal for now (same logic as routes_experience).
async fn resolve_mode(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    org_id: Uuid,
    channel_id: Option<Uuid>,
) -> Result<(String, String), axum::response::Response> {
    // 1) user preference (global)
    let user_pref: Option<String> =
        sqlx::query_scalar(r#"select experience_mode_preference from users where id = $1"#)
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
    let org_default: Option<String> =
        sqlx::query_scalar(r#"select experience_default_mode from organizations where id = $1"#)
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
