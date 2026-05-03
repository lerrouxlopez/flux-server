use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use permissions::perms;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::OffsetDateTime;
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/orgs/{org_id}/channels",
            get(list_org_channels).post(create_channel),
        )
        .route(
            "/channels/{channel_id}",
            get(get_channel)
                .patch(update_channel)
                .delete(delete_channel),
        )
}

#[derive(Debug, Deserialize)]
struct CreateChannelRequest {
    name: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct UpdateChannelRequest {
    name: Option<String>,
    kind: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChannelResponse {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    kind: String,
    created_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct ChannelsResponse {
    channels: Vec<ChannelResponse>,
}

async fn list_org_channels(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::CHANNELS_VIEW) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select id, organization_id, name, kind, created_at
        from channels
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

    let channels = rows
        .into_iter()
        .map(|r| ChannelResponse {
            id: r.get("id"),
            organization_id: r.get("organization_id"),
            name: r.get("name"),
            kind: r.get("kind"),
            created_at: r.get("created_at"),
        })
        .collect();

    (StatusCode::OK, Json(ChannelsResponse { channels })).into_response()
}

async fn create_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateChannelRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    let ok = match util::can(
        &state.pool,
        auth.user_id,
        org_id,
        permissions::Permission::ChannelsCreate,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !ok {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let name = req.name.trim().to_string();
    let kind = match normalize_kind(&req.kind) {
        Ok(k) => k,
        Err(e) => return *e,
    };
    if name.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let now = OffsetDateTime::now_utc();
    let channel_id = Uuid::now_v7();

    let inserted = sqlx::query(
        r#"
        insert into channels (id, organization_id, name, kind, created_at)
        values ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(channel_id)
    .bind(org_id)
    .bind(name.clone())
    .bind(kind.clone())
    .bind(now)
    .execute(&state.pool)
    .await;

    match inserted {
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "channel.created",
                Some("channel"),
                Some(channel_id),
                serde_json::json!({"kind": kind, "name": name}),
            )
            .await;
            (
                StatusCode::OK,
                Json(ChannelResponse {
                    id: channel_id,
                    organization_id: org_id,
                    name,
                    kind,
                    created_at: now,
                }),
            )
                .into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn get_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> impl IntoResponse {
    let row = sqlx::query(
        r#"
        select id, organization_id, name, kind, created_at
        from channels
        where id = $1
        "#,
    )
    .bind(channel_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let org_id: Uuid = row.get("organization_id");
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::CHANNELS_VIEW) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    (
        StatusCode::OK,
        Json(ChannelResponse {
            id: row.get("id"),
            organization_id: org_id,
            name: row.get("name"),
            kind: row.get("kind"),
            created_at: row.get("created_at"),
        }),
    )
        .into_response()
}

async fn update_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<UpdateChannelRequest>,
) -> impl IntoResponse {
    let row = sqlx::query(
        r#"
        select organization_id
        from channels
        where id = $1
        "#,
    )
    .bind(channel_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let org_id: Uuid = row.get("organization_id");
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::CHANNELS_MANAGE) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let name = req
        .name
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    let kind = match req.kind {
        Some(k) => match normalize_kind(&k) {
            Ok(v) => Some(v),
            Err(e) => return *e,
        },
        None => None,
    };

    let updated = sqlx::query(
        r#"
        update channels
        set
          name = coalesce($2, name),
          kind = coalesce($3, kind)
        where id = $1
        "#,
    )
    .bind(channel_id)
    .bind(name)
    .bind(kind)
    .execute(&state.pool)
    .await;

    match updated {
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "channel.updated",
                Some("channel"),
                Some(channel_id),
                serde_json::json!({}),
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn delete_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> impl IntoResponse {
    let row = sqlx::query(
        r#"
        select organization_id
        from channels
        where id = $1
        "#,
    )
    .bind(channel_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let org_id: Uuid = row.get("organization_id");
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::CHANNELS_MANAGE) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let deleted = sqlx::query(r#"delete from channels where id = $1"#)
        .bind(channel_id)
        .execute(&state.pool)
        .await;

    match deleted {
        Ok(_) => {
            util::write_audit_log(
                &state.pool,
                org_id,
                Some(auth.user_id),
                "channel.deleted",
                Some("channel"),
                Some(channel_id),
                serde_json::json!({}),
            )
            .await;
            (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

fn normalize_kind(input: &str) -> Result<String, Box<axum::response::Response>> {
    let k = input.trim().to_lowercase();
    match k.as_str() {
        "text" | "voice" | "announcement" | "private" => Ok(k),
        _ => Err(Box::new(util::api_error(ApiErrorCode::ValidationError))),
    }
}
