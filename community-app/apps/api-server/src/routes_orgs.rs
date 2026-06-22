use crate::{AppState, AuthContext};
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use base64::Engine;
use permissions::{perms, Perms};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::util;
use api::ApiErrorCode;
use tracing::Span;

pub fn router() -> Router<AppState> {
    Router::new()
        // Organization gallery / discovery endpoints (see blueprint v3):
        // - GET /orgs          (membership-scoped)
        // - GET /orgs/discover (discoverable orgs; no closed leakage)
        // - POST /orgs/{org_id}/join (open orgs only)
        // - POST/GET moderation for join requests under /orgs/{org_id}/join-requests
        .route("/", post(create_org).get(list_orgs))
        .route("/discover", get(discover_orgs))
        .route("/join", post(join_org_by_invite))
        .route("/{org_id}", get(get_org).delete(delete_org))
        .route("/{org_id}/join", post(join_open_org))
        .route(
            "/{org_id}/join-requests",
            post(create_join_request).get(list_join_requests),
        )
        .route(
            "/{org_id}/join-requests/{request_id}/approve",
            post(approve_join_request),
        )
        .route(
            "/{org_id}/join-requests/{request_id}/reject",
            post(reject_join_request),
        )
        .route(
            "/{org_id}/discovery-settings",
            get(get_discovery_settings).patch(patch_discovery_settings),
        )
        .route("/{org_id}/members", get(list_members).post(add_member))
        .route(
            "/{org_id}/members/{user_id}",
            axum::routing::patch(update_member_role),
        )
        .route("/{org_id}/invites", post(create_invite))
        .route("/{org_id}/roles", get(list_roles))
}

#[derive(Debug, Deserialize)]
struct CreateOrgRequest {
    name: String,
    slug: String,
}

#[derive(Debug, Serialize)]
struct OrgResponse {
    id: Uuid,
    slug: String,
    name: String,
    created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct OrgListItemResponse {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    join_policy: String,
    member_count: i64,
    created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct OrgsListResponse {
    organizations: Vec<OrgListItemResponse>,
}

#[derive(Debug, Serialize)]
struct MemberResponse {
    user_id: Uuid,
    email: String,
    display_name: String,
    role: String,
    joined_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct MembersResponse {
    members: Vec<MemberResponse>,
}

#[derive(Debug, Serialize)]
struct RoleResponse {
    id: Uuid,
    name: String,
    permissions: i64,
    created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct RolesResponse {
    roles: Vec<RoleResponse>,
}

#[derive(Debug, Deserialize)]
struct CreateInviteRequest {
    expires_in_seconds: Option<i64>,
    max_uses: Option<i32>,
}

#[derive(Debug, Serialize)]
struct InviteResponse {
    code: String,
    expires_at: Option<OffsetDateTime>,
    max_uses: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct AddMemberRequest {
    user_id: Option<Uuid>,
    invite_code: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct JoinOrgRequest {
    slug: String,
    invite_code: String,
}

#[derive(Debug, Deserialize)]
struct DiscoverQuery {
    q: Option<String>,
    tag: Option<String>,
    policy: Option<String>,
    limit: Option<i64>,
    cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct DiscoverOrgResponse {
    id: Uuid,
    slug: String,
    name: String,
    description: Option<String>,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    join_policy: String,
    category: Option<String>,
    tags: Vec<String>,
    member_count: Option<i64>,
    online_count: Option<i64>,
    current_user_status: String, // "member" | "not_member" | "pending_request" | "rejected" | "invited"
}

#[derive(Debug, Serialize)]
struct DiscoverOrgsResponse {
    organizations: Vec<DiscoverOrgResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateJoinRequest {
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct JoinRequestResponse {
    id: Uuid,
    user_id: Uuid,
    status: String,
    message: Option<String>,
    created_at: OffsetDateTime,
    responded_at: Option<OffsetDateTime>,
    responded_by: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct JoinRequestsListResponse {
    requests: Vec<JoinRequestResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct PatchDiscoverySettingsRequest {
    discoverable: Option<bool>,
    join_policy: Option<String>,
    description: Option<String>,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    member_count_visible: Option<bool>,
    online_count_visible: Option<bool>,
    category: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct DiscoverySettingsResponse {
    discoverable: bool,
    join_policy: String,
    description: Option<String>,
    avatar_url: Option<String>,
    banner_url: Option<String>,
    member_count_visible: bool,
    online_count_visible: bool,
    category: Option<String>,
    tags: Vec<String>,
}

async fn create_org(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateOrgRequest>,
) -> impl IntoResponse {
    let name = req.name.trim().to_string();
    let slug = req.slug.trim().to_lowercase();
    if name.is_empty() || slug.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let org_id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();

    let owner_permissions: Perms = perms::ALL;
    let admin_permissions: Perms = perms::ORG_MANAGE
        | perms::ORG_MANAGE_MEMBERS
        | perms::ORG_INVITES_CREATE
        | perms::BRANDING_MANAGE
        | perms::ADMIN_AUDIT_LOG_VIEW
        | perms::CHANNELS_VIEW
        | perms::CHANNELS_CREATE
        | perms::CHANNELS_MANAGE
        | perms::MESSAGES_SEND
        | perms::MESSAGES_EDIT_OWN
        | perms::MESSAGES_DELETE_OWN
        | perms::MESSAGES_DELETE_ANY
        | perms::MESSAGES_REACT
        | perms::MEDIA_ROOMS_CREATE
        | perms::VOICE_JOIN
        | perms::VOICE_SPEAK
        | perms::VIDEO_START
        | perms::SCREEN_SHARE;

    let moderator_permissions: Perms = perms::CHANNELS_VIEW
        | perms::MESSAGES_SEND
        | perms::MESSAGES_DELETE_ANY
        | perms::MESSAGES_REACT
        | perms::VOICE_JOIN;

    let member_permissions: Perms = perms::CHANNELS_VIEW
        | perms::MESSAGES_SEND
        | perms::MESSAGES_EDIT_OWN
        | perms::MESSAGES_DELETE_OWN
        | perms::MESSAGES_REACT
        | perms::VOICE_JOIN
        | perms::VOICE_SPEAK
        | perms::VIDEO_START
        | perms::SCREEN_SHARE;

    let guest_permissions: Perms = perms::CHANNELS_VIEW | perms::VOICE_JOIN;

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let org_inserted = sqlx::query(
        r#"
        insert into organizations (id, slug, name, created_at)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(org_id)
    .bind(slug.clone())
    .bind(name.clone())
    .bind(now)
    .execute(&mut *tx)
    .await;
    if let Err(err) = org_inserted {
        if util::is_unique_violation(&err) {
            return util::api_error(ApiErrorCode::Conflict);
        }
        return util::api_error(ApiErrorCode::InternalError);
    }

    // Default roles
    let owner_role_id = Uuid::now_v7();
    let admin_role_id = Uuid::now_v7();
    let moderator_role_id = Uuid::now_v7();
    let member_role_id = Uuid::now_v7();
    let guest_role_id = Uuid::now_v7();

    let roles_insert = sqlx::query(
        r#"
        insert into roles (id, organization_id, name, permissions)
        values
          ($1, $6, 'owner', $2),
          ($3, $6, 'admin', $4),
          ($5, $6, 'moderator', $7),
          ($8, $6, 'member', $9),
          ($10, $6, 'guest', $11)
        "#,
    )
    .bind(owner_role_id)
    .bind(owner_permissions)
    .bind(admin_role_id)
    .bind(admin_permissions)
    .bind(moderator_role_id)
    .bind(org_id)
    .bind(moderator_permissions)
    .bind(member_role_id)
    .bind(member_permissions)
    .bind(guest_role_id)
    .bind(guest_permissions)
    .execute(&mut *tx)
    .await;
    if roles_insert.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    // Creator membership
    let member_insert = sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role, joined_at)
        values ($1, $2, 'owner', $3)
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if member_insert.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    // Default channels:
    //   Work mode:  General, Announcements, Reports
    //   Play mode:  General, Announcements, Voice
    // Announcements has no mode hint so it appears in both modes.
    let work_general_id = Uuid::now_v7();
    let announcements_id = Uuid::now_v7();
    let work_reports_id = Uuid::now_v7();
    let play_general_id = Uuid::now_v7();
    let play_voice_id = Uuid::now_v7();

    let channels_insert = sqlx::query(
        r#"
        insert into channels (id, organization_id, name, kind, experience_mode_hint, created_at)
        values
          ($1, $6, 'General',       'text',         'work', $7),
          ($2, $6, 'Announcements', 'announcement',  NULL,   $7),
          ($3, $6, 'Reports',       'text',          'work', $7),
          ($4, $6, 'General',       'text',          'play', $7),
          ($5, $6, 'Voice',         'voice',         'play', $7)
        "#,
    )
    .bind(work_general_id)
    .bind(announcements_id)
    .bind(work_reports_id)
    .bind(play_general_id)
    .bind(play_voice_id)
    .bind(org_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if channels_insert.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    // Default branding profile
    let branding_insert = sqlx::query(
        r#"
        insert into branding_profiles (organization_id, app_name, theme, created_at, updated_at)
        values ($1, $2, 'dark', $3, $3)
        "#,
    )
    .bind(org_id)
    .bind(name.clone())
    .bind(now)
    .execute(&mut *tx)
    .await;
    if branding_insert.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    (
        StatusCode::OK,
        Json(OrgResponse {
            id: org_id,
            slug,
            name,
            created_at: now,
        }),
    )
        .into_response()
}

fn normalize_join_policy(v: &str) -> Option<&'static str> {
    match v.trim().to_lowercase().as_str() {
        "open" => Some("open"),
        "invite_only" => Some("invite_only"),
        "request" => Some("request"),
        "closed" => Some("closed"),
        _ => None,
    }
}

fn is_safe_http_url(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    t.starts_with("http://") || t.starts_with("https://")
}

fn is_safe_image_source(s: &str, max_bytes: usize) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    if is_safe_http_url(t) {
        return true;
    }
    if t.starts_with("data:") {
        let (mime, bytes) = match crate::attachments_storage::parse_data_url(t) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if !mime.starts_with("image/") {
            return false;
        }
        return bytes.len() <= max_bytes;
    }
    false
}

async fn discover_orgs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<DiscoverQuery>,
) -> impl IntoResponse {
    let q_str = q.q.unwrap_or_default().trim().to_string();
    let tag = q.tag.unwrap_or_default().trim().to_string();
    let policy = q.policy.and_then(|p| normalize_join_policy(&p).map(|s| s.to_string()));
    let limit = q.limit.unwrap_or(50).clamp(1, 100);

    // cursor reserved for keyset pagination; currently not implemented (return null next_cursor).
    let _cursor = q.cursor.unwrap_or_default();

    // Privacy: only include discoverable orgs that are not closed, unless user is a member.
    let rows = sqlx::query(
        r#"
        with my as (
          select organization_id
          from organization_members
          where user_id = $1
        ),
        jr as (
          select organization_id,
                 status,
                 responded_at,
                 created_at
          from organization_join_requests
          where user_id = $1
        ),
        member_counts as (
          select organization_id, count(*)::bigint as c
          from organization_members
          group by organization_id
        )
        select
          o.id,
          o.slug,
          o.name,
          o.description,
          o.avatar_url,
          o.banner_url,
          o.join_policy,
          o.category,
          o.tags,
          o.member_count_visible,
          o.online_count_visible,
          (my.organization_id is not null) as is_member,
          jr.status as jr_status,
          coalesce(member_counts.c, 0) as member_count
        from organizations o
        left join my on my.organization_id = o.id
        left join jr on jr.organization_id = o.id
        left join member_counts on member_counts.organization_id = o.id
        where
          (
            my.organization_id is not null
            or (o.discoverable = true and o.join_policy != 'closed')
          )
          and ($2 = '' or (o.name ilike '%' || $2 || '%' or o.slug ilike '%' || $2 || '%' or coalesce(o.description,'') ilike '%' || $2 || '%'))
          and ($3 = '' or $3 = any(o.tags))
          and ($4 is null or o.join_policy = $4)
        order by (my.organization_id is not null) desc, o.created_at desc
        limit $5
        "#,
    )
    .bind(auth.user_id)
    .bind(&q_str)
    .bind(&tag)
    .bind(policy.as_deref())
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let is_member: bool = row.get("is_member");
        let jr_status: Option<String> = row.try_get("jr_status").ok();
        let status = if is_member {
            "member".to_string()
        } else if jr_status.as_deref() == Some("pending") {
            "pending_request".to_string()
        } else if jr_status.as_deref() == Some("rejected") {
            "rejected".to_string()
        } else {
            "not_member".to_string()
        };

        let member_count_visible: bool = row.get("member_count_visible");
        let online_count_visible: bool = row.get("online_count_visible");
        let member_count: i64 = row.get("member_count");

        out.push(DiscoverOrgResponse {
            id: row.get("id"),
            slug: row.get("slug"),
            name: row.get("name"),
            description: row.try_get("description").ok(),
            avatar_url: row.try_get("avatar_url").ok(),
            banner_url: row.try_get("banner_url").ok(),
            join_policy: row.get("join_policy"),
            category: row.try_get("category").ok(),
            tags: row.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
            member_count: if is_member || member_count_visible {
                Some(member_count)
            } else {
                None
            },
            online_count: if is_member || online_count_visible {
                Some(0)
            } else {
                None
            },
            current_user_status: status,
        });
    }

    (
        StatusCode::OK,
        Json(DiscoverOrgsResponse {
            organizations: out,
            next_cursor: None,
        }),
    )
        .into_response()
}

async fn join_open_org(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let row = sqlx::query(
        r#"
        select join_policy
        from organizations
        where id = $1
        "#,
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let join_policy: String = row.get("join_policy");
    if join_policy != "open" {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let now = OffsetDateTime::now_utc();
    let res = sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role, joined_at)
        values ($1, $2, 'member', $3)
        on conflict do nothing
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&state.pool)
    .await;

    match res {
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "org.join.open",
                Some("user"),
                Some(auth.user_id),
                serde_json::json!({}),
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn create_join_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateJoinRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    // Only allow if org is request-access or invite-only (invite-only can still accept requests).
    let row = sqlx::query(
        r#"
        select join_policy
        from organizations
        where id = $1
        "#,
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let join_policy: String = row.get("join_policy");
    if join_policy != "request" && join_policy != "invite_only" {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    // If already member, no-op.
    let existing_member: Option<Uuid> = sqlx::query_scalar(
        r#"select organization_id from organization_members where organization_id = $1 and user_id = $2"#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    if existing_member.is_some() {
        return (StatusCode::OK, Json(serde_json::json!({"status":"ok","already_member":true}))).into_response();
    }

    let message = req.message.map(|m| m.trim().to_string()).filter(|m| !m.is_empty());
    let id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();

    let inserted = sqlx::query(
        r#"
        insert into organization_join_requests (id, organization_id, user_id, message, status, created_at)
        values ($1, $2, $3, $4, 'pending', $5)
        on conflict (organization_id, user_id)
        do update set
          message = excluded.message,
          status = case
            when organization_join_requests.status = 'approved' then 'approved'
            else 'pending'
          end,
          responded_at = case
            when organization_join_requests.status = 'approved' then organization_join_requests.responded_at
            else null
          end,
          responded_by = case
            when organization_join_requests.status = 'approved' then organization_join_requests.responded_by
            else null
          end
        returning id, status
        "#,
    )
    .bind(id)
    .bind(org_id)
    .bind(auth.user_id)
    .bind(message.clone())
    .bind(now)
    .fetch_one(&state.pool)
    .await;

    match inserted {
        Ok(row) => {
            let rid: Uuid = row.get("id");
            let status: String = row.get("status");
            if status == "approved" {
                return (StatusCode::OK, Json(serde_json::json!({"status":"ok","already_member":true}))).into_response();
            }
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "org.join_request.created",
                Some("join_request"),
                Some(rid),
                serde_json::json!({}),
            )
            .await;
            (
                StatusCode::OK,
                Json(serde_json::json!({"status":"ok","request_id":rid})),
            )
                .into_response()
        }
        Err(err) => {
            if util::is_unique_violation(&err) {
                util::api_error(ApiErrorCode::Conflict)
            } else {
                util::api_error(ApiErrorCode::InternalError)
            }
        }
    }
}

async fn list_join_requests(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms_v = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::ORG_MANAGE_MEMBERS) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select id, user_id, status, message, created_at, responded_at, responded_by
        from organization_join_requests
        where organization_id = $1
        order by created_at desc
        limit 200
        "#,
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(JoinRequestResponse {
            id: r.get("id"),
            user_id: r.get("user_id"),
            status: r.get("status"),
            message: r.try_get("message").ok(),
            created_at: r.get("created_at"),
            responded_at: r.try_get("responded_at").ok(),
            responded_by: r.try_get("responded_by").ok(),
        });
    }

    (StatusCode::OK, Json(JoinRequestsListResponse { requests: out })).into_response()
}

async fn approve_join_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((org_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms_v = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::ORG_MANAGE_MEMBERS) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let now = OffsetDateTime::now_utc();
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let req_row = sqlx::query(
        r#"
        select user_id, status
        from organization_join_requests
        where id = $1 and organization_id = $2
        "#,
    )
    .bind(request_id)
    .bind(org_id)
    .fetch_optional(&mut *tx)
    .await;

    let Some(req_row) = (match req_row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let user_id: Uuid = req_row.get("user_id");
    let status: String = req_row.get("status");
    if status != "pending" {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let updated = sqlx::query(
        r#"
        update organization_join_requests
        set status = 'approved', responded_at = $3, responded_by = $4
        where id = $1 and organization_id = $2
        "#,
    )
    .bind(request_id)
    .bind(org_id)
    .bind(now)
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await;
    if updated.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let member_insert = sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role, joined_at)
        values ($1, $2, 'member', $3)
        on conflict do nothing
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if member_insert.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    util::write_audit_log(
        &state.pool,
        org_id,
        Some(auth.user_id),
        "org.join_request.approved",
        Some("join_request"),
        Some(request_id),
        serde_json::json!({ "user_id": user_id }),
    )
    .await;

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}

async fn reject_join_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((org_id, request_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms_v = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::ORG_MANAGE_MEMBERS) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let now = OffsetDateTime::now_utc();
    let updated = sqlx::query(
        r#"
        update organization_join_requests
        set status = 'rejected', responded_at = $3, responded_by = $4
        where id = $1 and organization_id = $2 and status = 'pending'
        "#,
    )
    .bind(request_id)
    .bind(org_id)
    .bind(now)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(res) => {
            if res.rows_affected() == 0 {
                return util::api_error(ApiErrorCode::NotFound);
            }
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "org.join_request.rejected",
                Some("join_request"),
                Some(request_id),
                serde_json::json!({}),
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn patch_discovery_settings(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Json(req): Json<PatchDiscoverySettingsRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms_v = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::ORG_MANAGE) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let join_policy = match req.join_policy.as_deref() {
        None => None,
        Some(v) => match normalize_join_policy(v) {
            Some(p) => Some(p.to_string()),
            None => return util::api_error(ApiErrorCode::ValidationError),
        },
    };

    let avatar_url = req
        .avatar_url
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if req.avatar_url.is_some()
        && avatar_url
            .as_deref()
            .is_some_and(|u| !is_safe_image_source(u, 256 * 1024))
    {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let banner_url = req
        .banner_url
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if req.banner_url.is_some()
        && banner_url
            .as_deref()
            .is_some_and(|u| !is_safe_image_source(u, 1024 * 1024))
    {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let description = req
        .description
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let category = req
        .category
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let tags = req.tags.as_ref().map(|v| {
        v.iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .take(20)
            .collect::<Vec<_>>()
    });

    let res = sqlx::query(
        r#"
        update organizations
        set
          discoverable = coalesce($2, discoverable),
          join_policy = coalesce($3, join_policy),
          description = case when $4 then $5 else description end,
          avatar_url = case when $6 then $7 else avatar_url end,
          banner_url = case when $8 then $9 else banner_url end,
          member_count_visible = coalesce($10, member_count_visible),
          online_count_visible = coalesce($11, online_count_visible),
          category = case when $12 then $13 else category end,
          tags = case when $14 then $15 else tags end
        where id = $1
        "#,
    )
    .bind(org_id)
    .bind(req.discoverable)
    .bind(join_policy)
    .bind(req.description.is_some())
    .bind(description)
    .bind(req.avatar_url.is_some())
    .bind(avatar_url)
    .bind(req.banner_url.is_some())
    .bind(banner_url)
    .bind(req.member_count_visible)
    .bind(req.online_count_visible)
    .bind(req.category.is_some())
    .bind(category)
    .bind(req.tags.is_some())
    .bind(tags)
    .execute(&state.pool)
    .await;

    match res {
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "org.discovery_settings.changed",
                Some("organization"),
                Some(org_id),
                serde_json::json!({}),
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn get_discovery_settings(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms_v = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms_v, perms::ORG_MANAGE) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let row = sqlx::query(
        r#"
        select
          discoverable,
          join_policy,
          description,
          avatar_url,
          banner_url,
          member_count_visible,
          online_count_visible,
          category,
          tags
        from organizations
        where id = $1
        "#,
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    (
        StatusCode::OK,
        Json(DiscoverySettingsResponse {
            discoverable: row.get("discoverable"),
            join_policy: row.get("join_policy"),
            description: row.try_get("description").ok(),
            avatar_url: row.try_get("avatar_url").ok(),
            banner_url: row.try_get("banner_url").ok(),
            member_count_visible: row.get("member_count_visible"),
            online_count_visible: row.get("online_count_visible"),
            category: row.try_get("category").ok(),
            tags: row.try_get::<Vec<String>, _>("tags").unwrap_or_default(),
        }),
    )
        .into_response()
}

async fn list_orgs(
    State(state): State<AppState>,
    auth: AuthContext,
) -> impl IntoResponse {
    let rows = sqlx::query(
        r#"
        with member_counts as (
          select organization_id, count(*)::bigint as c
          from organization_members
          group by organization_id
        )
        select
          o.id, o.slug, o.name, o.description, o.avatar_url, o.banner_url,
          o.join_policy, o.created_at,
          coalesce(member_counts.c, 0) as member_count
        from organizations o
        join organization_members m on m.organization_id = o.id
        left join member_counts on member_counts.organization_id = o.id
        where m.user_id = $1
        order by o.created_at desc
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let organizations = rows
        .into_iter()
        .map(|r| OrgListItemResponse {
            id: r.get("id"),
            slug: r.get("slug"),
            name: r.get("name"),
            description: r.try_get("description").ok(),
            avatar_url: r.try_get("avatar_url").ok(),
            banner_url: r.try_get("banner_url").ok(),
            join_policy: r.get("join_policy"),
            member_count: r.get("member_count"),
            created_at: r.get("created_at"),
        })
        .collect();

    (StatusCode::OK, Json(OrgsListResponse { organizations })).into_response()
}

async fn get_org(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let row = sqlx::query(
        r#"
        select id, slug, name, created_at
        from organizations
        where id = $1
        "#,
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    (
        StatusCode::OK,
        Json(OrgResponse {
            id: row.get("id"),
            slug: row.get("slug"),
            name: row.get("name"),
            created_at: row.get("created_at"),
        }),
    )
        .into_response()
}

async fn delete_org(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    // Deletion is restricted to the owner specifically (not just ORG_MANAGE, which the
    // admin role also carries) — same special-casing `update_member_role` uses for
    // owner-only actions, since this is irreversible and destroys every org-scoped row.
    let role = sqlx::query_scalar::<_, String>(
        r#"
        select role
        from organization_members
        where organization_id = $1 and user_id = $2
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await;

    match role {
        Ok(Some(r)) if r == "owner" => {}
        Ok(Some(_)) => return util::api_error(ApiErrorCode::PermissionDenied),
        Ok(None) => return util::api_error(ApiErrorCode::NotFound),
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }

    // Every organization_id foreign key in the schema is ON DELETE CASCADE
    // (members, roles, channels, messages, branding, invites, audit logs, etc.),
    // so deleting the row here is sufficient to clean up the rest.
    let deleted = sqlx::query(r#"delete from organizations where id = $1"#)
        .bind(org_id)
        .execute(&state.pool)
        .await;

    match deleted {
        Ok(r) if r.rows_affected() == 0 => util::api_error(ApiErrorCode::NotFound),
        Ok(_) => {
            tracing::info!(organization_id = %org_id, actor_id = %auth.user_id, "organization deleted");
            (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn list_members(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let is_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !is_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select m.user_id, u.email, u.display_name, m.role, m.joined_at
        from organization_members m
        join users u on u.id = m.user_id
        where m.organization_id = $1
        order by joined_at asc
        "#,
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let members = rows
        .into_iter()
        .map(|r| MemberResponse {
            user_id: r.get("user_id"),
            email: r.get("email"),
            display_name: r.get("display_name"),
            role: r.get("role"),
            joined_at: r.get("joined_at"),
        })
        .collect();

    (StatusCode::OK, Json(MembersResponse { members })).into_response()
}

async fn join_org_by_invite(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<JoinOrgRequest>,
) -> impl IntoResponse {
    let slug = req.slug.trim().to_lowercase();
    let code = req.invite_code.trim().to_string();
    if slug.is_empty() || code.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let org_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        select id
        from organizations
        where slug = $1
        "#,
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await;

    let Some(org_id) = (match org_id {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    Span::current().record("organization_id", tracing::field::display(org_id));

    // Join via invite_code (same logic as add_member's invite flow).
    let now = OffsetDateTime::now_utc();
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let invite = sqlx::query(
        r#"
        select id, expires_at, max_uses, use_count
        from organization_invites
        where organization_id = $1 and code = $2
        "#,
    )
    .bind(org_id)
    .bind(code)
    .fetch_optional(&mut *tx)
    .await;

    let Some(invite) = (match invite {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let invite_id: Uuid = invite.get("id");
    let expires_at: Option<OffsetDateTime> = invite.get("expires_at");
    let max_uses: Option<i32> = invite.get("max_uses");
    let use_count: i32 = invite.get("use_count");

    if expires_at.is_some_and(|e| e <= now) {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    if max_uses.is_some_and(|m| use_count >= m) {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let member_insert = sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role, joined_at)
        values ($1, $2, 'member', $3)
        on conflict do nothing
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if member_insert.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let bump = sqlx::query(
        r#"
        update organization_invites
        set use_count = use_count + 1
        where id = $1
        "#,
    )
    .bind(invite_id)
    .execute(&mut *tx)
    .await;
    if bump.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    util::write_audit_log(
        &state.pool,
        org_id,
        Some(auth.user_id),
        "org.member.joined_by_invite",
        Some("invite"),
        Some(invite_id),
        serde_json::json!({}),
    )
    .await;

    (
        StatusCode::OK,
        Json(serde_json::json!({"status":"ok","organization_id": org_id, "slug": slug})),
    )
        .into_response()
}

#[derive(Debug, serde::Deserialize)]
struct UpdateMemberRoleRequest {
    role: String,
}

async fn update_member_role(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateMemberRoleRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    let ok = match util::can(
        &state.pool,
        auth.user_id,
        org_id,
        permissions::Permission::OrgManageMembers,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !ok {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let role = req.role.trim().to_lowercase();
    if role.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    // Ownership transfer is intentionally not supported in MVP role management.
    if role == "owner" {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    // Ensure target exists and is in this org; disallow changing owners.
    let current_role = sqlx::query_scalar::<_, String>(
        r#"
        select role
        from organization_members
        where organization_id = $1 and user_id = $2
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(current_role) = (match current_role {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    if current_role == "owner" {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    // Role must exist in this org.
    let exists = sqlx::query_scalar::<_, i64>(
        r#"
        select 1::bigint
        from roles
        where organization_id = $1 and name = $2
        "#,
    )
    .bind(org_id)
    .bind(&role)
    .fetch_optional(&state.pool)
    .await;

    if (match exists {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    })
    .is_none()
    {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let updated = sqlx::query(
        r#"
        update organization_members
        set role = $3
        where organization_id = $1 and user_id = $2
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(&role)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(r) if r.rows_affected() == 0 => util::api_error(ApiErrorCode::NotFound),
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "org.member.role_updated",
                Some("user"),
                Some(user_id),
                serde_json::json!({ "role": role }),
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn list_roles(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    // Must be org member to read.
    let _perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };

    let rows = sqlx::query(
        r#"
        select id, name, permissions, created_at
        from roles
        where organization_id = $1
        order by created_at asc
        "#,
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let roles = rows
        .into_iter()
        .map(|r| RoleResponse {
            id: r.get("id"),
            name: r.get("name"),
            permissions: r.get("permissions"),
            created_at: r.get("created_at"),
        })
        .collect();

    (StatusCode::OK, Json(RolesResponse { roles })).into_response()
}

async fn create_invite(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateInviteRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::ORGS_INVITES_CREATE) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let mut bytes = [0u8; 18];
    OsRng.fill_bytes(&mut bytes);
    let code = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);

    let now = OffsetDateTime::now_utc();
    let expires_at = req
        .expires_in_seconds
        .map(|s| now + time::Duration::seconds(s.max(0)));

    let invite_id = Uuid::now_v7();

    let inserted = sqlx::query(
        r#"
        insert into organization_invites (id, organization_id, code, created_by, created_at, expires_at, max_uses)
        values ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(invite_id)
    .bind(org_id)
    .bind(code.clone())
    .bind(auth.user_id)
    .bind(now)
    .bind(expires_at)
    .bind(req.max_uses)
    .execute(&state.pool)
    .await;

    match inserted {
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "org.invite.created",
                Some("invite"),
                Some(invite_id),
                serde_json::json!({"max_uses": req.max_uses, "expires_at": expires_at}),
            )
            .await;
            (
                StatusCode::OK,
                Json(InviteResponse {
                    code,
                    expires_at,
                    max_uses: req.max_uses,
                }),
            )
                .into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn add_member(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    // Two modes:
    // - Owner adds a specific user_id (admin action)
    // - A user joins via invite_code (self-serve)
    if let Some(user_id) = req.user_id {
        let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
            Ok(p) => p,
            Err(e) => return e,
        };
        if !permissions::has(perms, perms::ORGS_MEMBERS_MANAGE) {
            return util::api_error(ApiErrorCode::PermissionDenied);
        }

        let now = OffsetDateTime::now_utc();
        let res = sqlx::query(
            r#"
            insert into organization_members (organization_id, user_id, role, joined_at)
            values ($1, $2, 'member', $3)
            on conflict do nothing
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(now)
        .execute(&state.pool)
        .await;

        return match res {
            Ok(_) => {
                util::write_audit_log(
                    &state.pool,
                    org_id,
                    Some(auth.user_id),
                    "org.member.added",
                    Some("user"),
                    Some(user_id),
                    serde_json::json!({}),
                )
                .await;
                (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
            }
            Err(_) => util::api_error(ApiErrorCode::InternalError),
        };
    }

    let Some(code) = req.invite_code else {
        return util::api_error(ApiErrorCode::ValidationError);
    };

    // Join via invite_code
    let now = OffsetDateTime::now_utc();
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let invite = sqlx::query(
        r#"
        select id, expires_at, max_uses, use_count
        from organization_invites
        where organization_id = $1 and code = $2
        "#,
    )
    .bind(org_id)
    .bind(code)
    .fetch_optional(&mut *tx)
    .await;

    let Some(invite) = (match invite {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let invite_id: Uuid = invite.get("id");
    let expires_at: Option<OffsetDateTime> = invite.get("expires_at");
    let max_uses: Option<i32> = invite.get("max_uses");
    let use_count: i32 = invite.get("use_count");

    if expires_at.is_some_and(|e| e <= now) {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    if max_uses.is_some_and(|m| use_count >= m) {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let member_insert = sqlx::query(
        r#"
        insert into organization_members (organization_id, user_id, role, joined_at)
        values ($1, $2, 'member', $3)
        on conflict do nothing
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if member_insert.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let bump = sqlx::query(
        r#"
        update organization_invites
        set use_count = use_count + 1
        where id = $1
        "#,
    )
    .bind(invite_id)
    .execute(&mut *tx)
    .await;
    if bump.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}
