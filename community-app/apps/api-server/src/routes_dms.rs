use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Serialize;
use sqlx::Row;
use time::OffsetDateTime;
use tracing::Span;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orgs/{org_id}/dms", get(list_dms))
        .route("/orgs/{org_id}/dms/{user_id}", post(create_or_get_dm))
}

#[derive(Debug, Serialize)]
struct UserSummary {
    id: Uuid,
    email: String,
    display_name: String,
}

#[derive(Debug, Serialize)]
struct DmThread {
    channel_id: Uuid,
    peer: UserSummary,
}

#[derive(Debug, Serialize)]
struct DmsResponse {
    dms: Vec<DmThread>,
}

async fn list_dms(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
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
        select
          c.id as channel_id,
          u.id as peer_id,
          u.email as peer_email,
          u.display_name as peer_display_name
        from channels c
        join dm_channel_members m_me on m_me.channel_id = c.id and m_me.user_id = $2
        join dm_channel_members m_peer on m_peer.channel_id = c.id and m_peer.user_id <> $2
        join users u on u.id = m_peer.user_id
        where c.organization_id = $1
          and c.kind = 'dm'
        order by u.display_name asc
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut dms = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        dms.push(DmThread {
            channel_id: r.get("channel_id"),
            peer: UserSummary {
                id: r.get("peer_id"),
                email: r.get("peer_email"),
                display_name: r.get("peer_display_name"),
            },
        });
    }

    (StatusCode::OK, Json(DmsResponse { dms })).into_response()
}

async fn create_or_get_dm(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));
    if user_id == auth.user_id {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let me_member = match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    let them_member = match util::is_member(&state.pool, org_id, user_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !me_member || !them_member {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let friends = sqlx::query_scalar::<_, i64>(
        r#"
        select 1::bigint
        from friend_requests
        where organization_id = $1
          and status = 'accepted'
          and (
            (requester_id = $2 and addressee_id = $3)
            or
            (requester_id = $3 and addressee_id = $2)
          )
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await;

    let friends = match friends {
        Ok(v) => v.is_some(),
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if !friends {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    // Existing DM?
    let existing = sqlx::query_scalar::<_, Uuid>(
        r#"
        select c.id
        from channels c
        join dm_channel_members a on a.channel_id = c.id and a.user_id = $2
        join dm_channel_members b on b.channel_id = c.id and b.user_id = $3
        where c.organization_id = $1 and c.kind = 'dm'
        limit 1
        "#,
    )
    .bind(org_id)
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await;

    if let Ok(Some(channel_id)) = existing {
        return (StatusCode::OK, Json(serde_json::json!({ "channel_id": channel_id }))).into_response();
    }

    let channel_id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let inserted_channel = sqlx::query(
        r#"
        insert into channels (id, organization_id, name, kind, created_at)
        values ($1, $2, '', 'dm', $3)
        "#,
    )
    .bind(channel_id)
    .bind(org_id)
    .bind(now)
    .execute(&mut *tx)
    .await;

    if inserted_channel.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let inserted_members = sqlx::query(
        r#"
        insert into dm_channel_members (channel_id, user_id, added_at)
        values
          ($1, $2, $4),
          ($1, $3, $4)
        "#,
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .bind(user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;

    if inserted_members.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    (StatusCode::OK, Json(serde_json::json!({ "channel_id": channel_id }))).into_response()
}

