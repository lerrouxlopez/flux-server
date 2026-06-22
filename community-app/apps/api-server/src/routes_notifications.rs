use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch},
    Router,
};
use notifications::{NotificationBehavior, NotificationRule, QuietHours, RuleChannels};
use permissions::perms;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications/context", get(get_notifications_context))
        .route("/notifications/overrides/user", patch(patch_user_override))
        .route("/notifications/overrides/channel", patch(patch_channel_override))
        .route(
            "/orgs/{org_id}/notification-profiles",
            get(list_notification_profiles).post(create_notification_profile),
        )
        .route(
            "/notification-profiles/{id}",
            patch(patch_notification_profile).delete(delete_notification_profile),
        )
}

fn parse_hhmm(s: &str) -> Option<time::Time> {
    let fmt = time::format_description::parse("[hour]:[minute]").ok()?;
    time::Time::parse(s, &fmt[..]).ok()
}

fn format_hhmm(t: time::Time) -> String {
    let fmt = time::format_description::parse("[hour]:[minute]").expect("valid format");
    t.format(&fmt[..]).unwrap_or_default()
}

fn validate_mode(mode: &str) -> Option<&'static str> {
    match mode {
        "work" => Some("work"),
        "play" => Some("play"),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct NotificationsContextQuery {
    org_id: Uuid,
    channel_id: Option<Uuid>,
    /// When present, use this mode directly instead of resolving the user's
    /// active experience mode -- lets the settings screen inspect/edit the
    /// *other* mode's settings without switching the user's real active mode.
    mode: Option<String>,
}

#[derive(Debug, Serialize)]
struct NotificationsContextResponse {
    mode: String,
    profile_source: String,
    profile_id: Option<Uuid>,
    profile_created_by: Option<Uuid>,
    behavior: NotificationBehavior,
    quiet_hours: QuietHours,
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

    let mode = match q.mode.as_deref().and_then(validate_mode) {
        Some(m) => m.to_string(),
        None => match resolve_mode(&state.pool, auth.user_id, q.org_id, q.channel_id).await {
            Ok((m, _source)) => m,
            Err(e) => return e,
        },
    };

    let resolved = match resolve_notification_profile(&state.pool, auth.user_id, q.org_id, q.channel_id, &mode).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let quiet_hours = match load_quiet_hours(&state.pool, q.org_id, auth.user_id, &mode).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    (
        StatusCode::OK,
        Json(NotificationsContextResponse {
            mode,
            profile_source: resolved.source,
            profile_id: resolved.profile_id,
            profile_created_by: resolved.profile_created_by,
            behavior: resolved.behavior,
            quiet_hours,
        }),
    )
        .into_response()
}

async fn load_quiet_hours(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    user_id: Uuid,
    mode: &str,
) -> Result<QuietHours, axum::response::Response> {
    let row = sqlx::query(
        r#"
        select quiet_hours_enabled, quiet_from, quiet_to, quiet_priority_override
        from user_notification_overrides
        where organization_id = $1 and user_id = $2 and mode = $3
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(mode)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    let Some(row) = row else { return Ok(QuietHours::default()) };
    let enabled: bool = row.try_get("quiet_hours_enabled").unwrap_or(false);
    let from: Option<time::Time> = row.try_get("quiet_from").ok();
    let to: Option<time::Time> = row.try_get("quiet_to").ok();
    let priority_override: bool = row.try_get("quiet_priority_override").unwrap_or(true);
    Ok(QuietHours {
        enabled,
        from: from.map(format_hhmm),
        to: to.map(format_hhmm),
        priority_override,
    })
}

#[derive(Debug, Deserialize)]
struct PatchUserOverrideRequest {
    org_id: Uuid,
    mode: String, // work|play
    profile_id: Option<Uuid>, // null clears
    quiet_hours_enabled: bool,
    quiet_from: Option<String>,
    quiet_to: Option<String>,
    quiet_priority_override: bool,
}

async fn patch_user_override(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<PatchUserOverrideRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(req.org_id));

    let Some(mode) = validate_mode(&req.mode) else {
        return util::api_error(ApiErrorCode::ValidationError);
    };

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

    let quiet_from = match req.quiet_from.as_deref().map(parse_hhmm) {
        Some(None) => return util::api_error(ApiErrorCode::ValidationError),
        Some(Some(t)) => Some(t),
        None => None,
    };
    let quiet_to = match req.quiet_to.as_deref().map(parse_hhmm) {
        Some(None) => return util::api_error(ApiErrorCode::ValidationError),
        Some(Some(t)) => Some(t),
        None => None,
    };

    let res = sqlx::query(
        r#"
        insert into user_notification_overrides
          (organization_id, user_id, mode, profile_id, quiet_hours_enabled, quiet_from, quiet_to, quiet_priority_override)
        values ($1, $2, $3, $4, $5, $6, $7, $8)
        on conflict (organization_id, user_id, mode)
        do update set
          profile_id = excluded.profile_id,
          quiet_hours_enabled = excluded.quiet_hours_enabled,
          quiet_from = excluded.quiet_from,
          quiet_to = excluded.quiet_to,
          quiet_priority_override = excluded.quiet_priority_override,
          updated_at = now()
        "#,
    )
    .bind(req.org_id)
    .bind(auth.user_id)
    .bind(mode)
    .bind(req.profile_id)
    .bind(req.quiet_hours_enabled)
    .bind(quiet_from)
    .bind(quiet_to)
    .bind(req.quiet_priority_override)
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

// ── Notification profile CRUD ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct NotificationProfileResponse {
    id: Uuid,
    scope: String,
    mode: String,
    label: String,
    description: Option<String>,
    rules: NotificationBehavior,
    created_by: Option<Uuid>,
}

async fn rules_for_profile(
    pool: &sqlx::PgPool,
    profile_id: Uuid,
) -> Result<NotificationBehavior, axum::response::Response> {
    let rows = sqlx::query(
        r#"select rule, in_app, desktop, sound from notification_profile_rules where profile_id = $1"#,
    )
    .bind(profile_id)
    .fetch_all(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    let mut m: HashMap<String, RuleChannels> = HashMap::new();
    for r in rows {
        let rule: String = r.try_get("rule").unwrap_or_default();
        m.insert(
            rule,
            RuleChannels {
                in_app: r.try_get("in_app").unwrap_or(false),
                desktop: r.try_get("desktop").unwrap_or(false),
                sound: r.try_get("sound").unwrap_or(false),
            },
        );
    }
    Ok(NotificationBehavior::from_rows(&m))
}

#[derive(Debug, Deserialize)]
struct ListProfilesQuery {
    mode: String,
}

async fn list_notification_profiles(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Query(q): Query<ListProfilesQuery>,
) -> impl IntoResponse {
    let Some(mode) = validate_mode(&q.mode) else {
        return util::api_error(ApiErrorCode::ValidationError);
    };

    let perms_v = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::CHANNELS_VIEW) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select id, scope, mode, label, description, created_by
        from notification_profiles
        where mode = $1 and (organization_id = $2 or organization_id is null)
        order by scope desc, label asc
        "#,
    )
    .bind(mode)
    .bind(org_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("id").unwrap_or_default();
        let rules = match rules_for_profile(&state.pool, id).await {
            Ok(v) => v,
            Err(e) => return e,
        };
        out.push(NotificationProfileResponse {
            id,
            scope: row.try_get("scope").unwrap_or_default(),
            mode: row.try_get("mode").unwrap_or_default(),
            label: row.try_get("label").unwrap_or_default(),
            description: row.try_get("description").ok(),
            rules,
            created_by: row.try_get("created_by").ok(),
        });
    }

    (StatusCode::OK, Json(out)).into_response()
}

#[derive(Debug, Deserialize)]
struct RuleChannelsInput {
    in_app: bool,
    desktop: bool,
    sound: bool,
}

#[derive(Debug, Deserialize)]
struct CreateProfileRequest {
    mode: String,
    label: String,
    description: Option<String>,
    rules: HashMap<String, RuleChannelsInput>,
}

async fn upsert_profile_rules(
    pool: &sqlx::PgPool,
    profile_id: Uuid,
    rules: &HashMap<String, RuleChannelsInput>,
) -> Result<(), axum::response::Response> {
    let valid: std::collections::HashSet<&'static str> =
        NotificationRule::all().iter().map(|r| r.as_str()).collect();

    for (rule, channels) in rules {
        if !valid.contains(rule.as_str()) {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        }
        sqlx::query(
            r#"
            insert into notification_profile_rules (profile_id, rule, in_app, desktop, sound)
            values ($1, $2, $3, $4, $5)
            on conflict (profile_id, rule)
            do update set in_app = excluded.in_app, desktop = excluded.desktop, sound = excluded.sound
            "#,
        )
        .bind(profile_id)
        .bind(rule)
        .bind(channels.in_app)
        .bind(channels.desktop)
        .bind(channels.sound)
        .execute(pool)
        .await
        .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;
    }
    Ok(())
}

async fn create_notification_profile(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateProfileRequest>,
) -> impl IntoResponse {
    let Some(mode) = validate_mode(&req.mode) else {
        return util::api_error(ApiErrorCode::ValidationError);
    };
    if req.label.trim().is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let perms_v = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::CHANNELS_VIEW) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let profile_id = Uuid::now_v7();
    let inserted = sqlx::query(
        r#"
        insert into notification_profiles (id, organization_id, scope, mode, label, description, created_by)
        values ($1, $2, 'org', $3, $4, $5, $6)
        "#,
    )
    .bind(profile_id)
    .bind(org_id)
    .bind(mode)
    .bind(req.label.trim())
    .bind(req.description.as_deref())
    .bind(auth.user_id)
    .execute(&state.pool)
    .await;
    if inserted.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if let Err(e) = upsert_profile_rules(&state.pool, profile_id, &req.rules).await {
        return e;
    }

    let rules = match rules_for_profile(&state.pool, profile_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    (
        StatusCode::OK,
        Json(NotificationProfileResponse {
            id: profile_id,
            scope: "org".to_string(),
            mode: mode.to_string(),
            label: req.label.trim().to_string(),
            description: req.description,
            rules,
            created_by: Some(auth.user_id),
        }),
    )
        .into_response()
}

struct ProfileOwnership {
    organization_id: Option<Uuid>,
    scope: String,
    created_by: Option<Uuid>,
}

async fn load_profile_ownership(
    pool: &sqlx::PgPool,
    profile_id: Uuid,
) -> Result<Option<ProfileOwnership>, axum::response::Response> {
    let row = sqlx::query(
        r#"select organization_id, scope, created_by from notification_profiles where id = $1"#,
    )
    .bind(profile_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    Ok(row.map(|r| ProfileOwnership {
        organization_id: r.try_get("organization_id").ok(),
        scope: r.try_get("scope").unwrap_or_default(),
        created_by: r.try_get("created_by").ok(),
    }))
}

async fn authorize_profile_mutation(
    state: &AppState,
    auth: &AuthContext,
    profile_id: Uuid,
) -> Result<ProfileOwnership, axum::response::Response> {
    let Some(ownership) = load_profile_ownership(&state.pool, profile_id).await? else {
        return Err(util::api_error(ApiErrorCode::NotFound));
    };
    if ownership.scope == "platform" {
        return Err(util::api_error(ApiErrorCode::PermissionDenied));
    }
    let Some(org_id) = ownership.organization_id else {
        return Err(util::api_error(ApiErrorCode::PermissionDenied));
    };
    if ownership.created_by == Some(auth.user_id) {
        return Ok(ownership);
    }
    let perms_v = util::member_perms(&state.pool, org_id, auth.user_id).await?;
    if permissions::has(perms_v, perms::NOTIFICATIONS_MANAGE) {
        return Ok(ownership);
    }
    Err(util::api_error(ApiErrorCode::PermissionDenied))
}

#[derive(Debug, Deserialize)]
struct PatchProfileRequest {
    label: Option<String>,
    description: Option<String>,
    rules: Option<HashMap<String, RuleChannelsInput>>,
}

async fn patch_notification_profile(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(profile_id): Path<Uuid>,
    Json(req): Json<PatchProfileRequest>,
) -> impl IntoResponse {
    if let Err(e) = authorize_profile_mutation(&state, &auth, profile_id).await {
        return e;
    }

    if let Some(label) = &req.label {
        if label.trim().is_empty() {
            return util::api_error(ApiErrorCode::ValidationError);
        }
        let res = sqlx::query(
            r#"update notification_profiles set label = $1, description = $2 where id = $3"#,
        )
        .bind(label.trim())
        .bind(req.description.as_deref())
        .bind(profile_id)
        .execute(&state.pool)
        .await;
        if res.is_err() {
            return util::api_error(ApiErrorCode::InternalError);
        }
    }

    if let Some(rules) = &req.rules {
        if let Err(e) = upsert_profile_rules(&state.pool, profile_id, rules).await {
            return e;
        }
    }

    let rules = match rules_for_profile(&state.pool, profile_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    let row = sqlx::query(
        r#"select scope, mode, label, description, created_by from notification_profiles where id = $1"#,
    )
    .bind(profile_id)
    .fetch_one(&state.pool)
    .await;
    let row = match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    (
        StatusCode::OK,
        Json(NotificationProfileResponse {
            id: profile_id,
            scope: row.try_get("scope").unwrap_or_default(),
            mode: row.try_get("mode").unwrap_or_default(),
            label: row.try_get("label").unwrap_or_default(),
            description: row.try_get("description").ok(),
            rules,
            created_by: row.try_get("created_by").ok(),
        }),
    )
        .into_response()
}

async fn delete_notification_profile(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(profile_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(e) = authorize_profile_mutation(&state, &auth, profile_id).await {
        return e;
    }

    let res = sqlx::query(r#"delete from notification_profiles where id = $1"#)
        .bind(profile_id)
        .execute(&state.pool)
        .await;

    match res {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

// ── Resolution (unchanged logic, updated for RuleChannels) ─────────────────

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
        return load_resolved_profile(pool, pid, "user_override").await;
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
            return load_resolved_profile(pool, pid, "channel_override").await;
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
        return load_resolved_profile(pool, pid, "mode_profile").await;
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
        return load_resolved_profile(pool, pid, "org_default").await;
    }

    // 5) platform default
    let platform_id = if mode == "play" {
        Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
    } else {
        Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()
    };
    load_resolved_profile(pool, platform_id, "platform_default")
        .await
        .or_else(|_| {
            Ok(notifications::ResolvedNotificationProfile {
                profile_id: Some(platform_id),
                profile_created_by: None,
                source: "platform_default".to_string(),
                behavior: NotificationBehavior::default(),
            })
        })
}

async fn load_resolved_profile(
    pool: &sqlx::PgPool,
    profile_id: Uuid,
    source: &str,
) -> Result<notifications::ResolvedNotificationProfile, axum::response::Response> {
    let created_by: Option<Uuid> = sqlx::query_scalar(
        r#"select created_by from notification_profiles where id = $1"#,
    )
    .bind(profile_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?
    .flatten();
    let behavior = rules_for_profile(pool, profile_id).await?;
    Ok(notifications::ResolvedNotificationProfile {
        profile_id: Some(profile_id),
        profile_created_by: created_by,
        source: source.to_string(),
        behavior,
    })
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
