use crate::{AppState, AuthContext};
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use permissions::{perms, Perms};
use rand::rngs::OsRng;
use rand::RngCore;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::util;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_org).get(list_orgs))
        .route("/:org_id", get(get_org))
        .route("/:org_id/members", get(list_members).post(add_member))
        .route("/:org_id/invites", post(create_invite))
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
struct OrgsListResponse {
    organizations: Vec<OrgResponse>,
}

#[derive(Debug, Serialize)]
struct MemberResponse {
    user_id: Uuid,
    role: String,
    joined_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct MembersResponse {
    members: Vec<MemberResponse>,
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

async fn create_org(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateOrgRequest>,
) -> impl IntoResponse {
    let name = req.name.trim().to_string();
    let slug = req.slug.trim().to_lowercase();
    if name.is_empty() || slug.is_empty() {
        return api_error(StatusCode::BAD_REQUEST, "invalid_request");
    }

    let org_id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();

    let owner_permissions: Perms = perms::ALL;
    let member_permissions: Perms = perms::CHANNELS_VIEW | perms::MESSAGES_SEND;

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
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
            return api_error(StatusCode::CONFLICT, "slug_taken");
        }
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
    }

    // Default roles
    let owner_role_id = Uuid::now_v7();
    let member_role_id = Uuid::now_v7();

    let roles_insert = sqlx::query(
        r#"
        insert into roles (id, organization_id, name, permissions)
        values
          ($1, $4, 'owner', $2),
          ($3, $4, 'member', $5)
        "#,
    )
    .bind(owner_role_id)
    .bind(owner_permissions)
    .bind(member_role_id)
    .bind(org_id)
    .bind(member_permissions)
    .execute(&mut *tx)
    .await;
    if roles_insert.is_err() {
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
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
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
    }

    // Default channels
    let general_id = Uuid::now_v7();
    let announcements_id = Uuid::now_v7();
    let voice_id = Uuid::now_v7();

    let channels_insert = sqlx::query(
        r#"
        insert into channels (id, organization_id, name, kind, created_at)
        values
          ($1, $4, 'general', 'text', $5),
          ($2, $4, 'announcements', 'announcement', $5),
          ($3, $4, 'General Voice', 'voice', $5)
        "#,
    )
    .bind(general_id)
    .bind(announcements_id)
    .bind(voice_id)
    .bind(org_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if channels_insert.is_err() {
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
    }

    // Default branding profile
    let branding_insert = sqlx::query(
        r#"
        insert into branding_profiles (organization_id, app_name, created_at, updated_at)
        values ($1, $2, $3, $3)
        "#,
    )
    .bind(org_id)
    .bind(name.clone())
    .bind(now)
    .execute(&mut *tx)
    .await;
    if branding_insert.is_err() {
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
    }

    if tx.commit().await.is_err() {
        return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
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

async fn list_orgs(State(state): State<AppState>, Extension(auth): Extension<AuthContext>) -> impl IntoResponse {
    let rows = sqlx::query(
        r#"
        select o.id, o.slug, o.name, o.created_at
        from organizations o
        join organization_members m on m.organization_id = o.id
        where m.user_id = $1
        order by o.created_at desc
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
    };

    let organizations = rows
        .into_iter()
        .map(|r| OrgResponse {
            id: r.get("id"),
            slug: r.get("slug"),
            name: r.get("name"),
            created_at: r.get("created_at"),
        })
        .collect();

    (StatusCode::OK, Json(OrgsListResponse { organizations })).into_response()
}

async fn get_org(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    if !util::is_member(&state.pool, org_id, auth.user_id)
        .await
        .unwrap_or(false)
    {
        return util::api_error(StatusCode::FORBIDDEN, "not_a_member");
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
        Err(_) => return util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
    }) else {
        return util::api_error(StatusCode::NOT_FOUND, "not_found");
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

async fn list_members(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    if !util::is_member(&state.pool, org_id, auth.user_id)
        .await
        .unwrap_or(false)
    {
        return util::api_error(StatusCode::FORBIDDEN, "not_a_member");
    }

    let rows = sqlx::query(
        r#"
        select user_id, role, joined_at
        from organization_members
        where organization_id = $1
        order by joined_at asc
        "#,
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
    };

    let members = rows
        .into_iter()
        .map(|r| MemberResponse {
            user_id: r.get("user_id"),
            role: r.get("role"),
            joined_at: r.get("joined_at"),
        })
        .collect();

    (StatusCode::OK, Json(MembersResponse { members })).into_response()
}

async fn create_invite(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateInviteRequest>,
) -> impl IntoResponse {
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::ORGS_INVITES_CREATE) {
        return util::api_error(StatusCode::FORBIDDEN, "forbidden");
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
        Ok(_) => (
            StatusCode::OK,
            Json(InviteResponse {
                code,
                expires_at,
                max_uses: req.max_uses,
            }),
        )
            .into_response(),
        Err(_) => util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
    }
}

async fn add_member(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> impl IntoResponse {
    // Two modes:
    // - Owner adds a specific user_id (admin action)
    // - A user joins via invite_code (self-serve)
    if let Some(user_id) = req.user_id {
        let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
            Ok(p) => p,
            Err(e) => return e,
        };
        if !permissions::has(perms, perms::ORGS_MEMBERS_MANAGE) {
            return util::api_error(StatusCode::FORBIDDEN, "forbidden");
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
            Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response(),
            Err(_) => util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
        };
    }

    let Some(code) = req.invite_code else {
        return util::api_error(StatusCode::BAD_REQUEST, "invalid_request");
    };

    // Join via invite_code
    let now = OffsetDateTime::now_utc();
    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
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
        Err(_) => return util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error"),
    }) else {
        return util::api_error(StatusCode::NOT_FOUND, "invalid_invite");
    };

    let invite_id: Uuid = invite.get("id");
    let expires_at: Option<OffsetDateTime> = invite.get("expires_at");
    let max_uses: Option<i32> = invite.get("max_uses");
    let use_count: i32 = invite.get("use_count");

    if expires_at.is_some_and(|e| e <= now) {
        return util::api_error(StatusCode::GONE, "invite_expired");
    }
    if max_uses.is_some_and(|m| use_count >= m) {
        return util::api_error(StatusCode::GONE, "invite_maxed");
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
        return util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
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
        return util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
    }

    if tx.commit().await.is_err() {
        return util::api_error(StatusCode::INTERNAL_SERVER_ERROR, "db_error");
    }

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}

fn api_error(status: StatusCode, code: &'static str) -> axum::response::Response {
    util::api_error(status, code)
}
