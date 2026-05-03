use crate::{util, AppState, AuthContext};
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Extension, Router,
};
use permissions::perms;
use base64::Engine;
use redis::AsyncCommands;
use sqlx::PgPool;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing::error;
use uuid::Uuid;
use events::envelope::EventEnvelope;
use api::ApiErrorCode;
use tracing::Span;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/channels/{channel_id}/messages", get(list_messages).post(send_message))
        .route("/messages/{message_id}", patch(edit_message).delete(delete_message))
        .route("/messages/{message_id}/reactions", post(add_reaction))
        .route(
            "/messages/{message_id}/reactions/{emoji}",
            delete(remove_reaction),
        )
}

#[derive(Debug, Deserialize)]
struct ListMessagesQuery {
    limit: Option<u32>,
    before: Option<String>,
}

#[derive(Debug, Serialize)]
struct ListMessagesResponse {
    messages: Vec<MessageResponse>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SendMessageRequest {
    body: String,
}

#[derive(Debug, Deserialize)]
struct EditMessageRequest {
    body: Option<String>,
}

#[derive(Debug, Serialize)]
struct MessageResponse {
    id: Uuid,
    organization_id: Uuid,
    channel_id: Uuid,
    sender_id: Uuid,
    body: Option<String>,
    kind: String,
    created_at: OffsetDateTime,
    edited_at: Option<OffsetDateTime>,
    deleted_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
struct ReactionRequest {
    emoji: String,
}

async fn list_messages(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> impl IntoResponse {
    // Reusable helper for channel access (membership + channels.view).
    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let (org_id, _channel_kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));

    let limit = query.limit.unwrap_or(50).min(100) as i64;
    let before = query.before.as_deref();
    let cursor = match parse_before(&state.pool, channel_id, before).await {
        Ok(c) => c,
        Err(e) => return e,
    };

    let rows = if let Some((created_at, id)) = cursor {
        sqlx::query(
            r#"
            select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
            from messages
            where channel_id = $1
              and deleted_at is null
              and (created_at, id) < ($2, $3)
            order by created_at desc, id desc
            limit $4
            "#,
        )
        .bind(channel_id)
        .bind(created_at)
        .bind(id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    } else {
        sqlx::query(
            r#"
            select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
            from messages
            where channel_id = $1
              and deleted_at is null
            order by created_at desc, id desc
            limit $2
            "#,
        )
        .bind(channel_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await
    };

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut messages: Vec<MessageResponse> = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        messages.push(MessageResponse {
            id: r.get("id"),
            organization_id: r.get("organization_id"),
            channel_id: r.get("channel_id"),
            sender_id: r.get("sender_id"),
            body: r.get("body"),
            kind: r.get("kind"),
            created_at: r.get("created_at"),
            edited_at: r.get("edited_at"),
            deleted_at: r.get("deleted_at"),
        });
    }

    let next_cursor = rows.last().map(|r| encode_cursor(r.get("created_at"), r.get("id")));

    (StatusCode::OK, Json(ListMessagesResponse { messages, next_cursor })).into_response()
}

async fn send_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    // Simple Redis-backed rate limiting (per user per channel).
    // Key scheme: rate:{user_id}:{action} where action encodes channel_id.
    let mut redis = state.redis.clone();
    let rl_key = format!("rate:{}:message.send:{}", auth.user_id, channel_id);
    let current: i64 = redis.incr(&rl_key, 1).await.unwrap_or(1);
    if current == 1 {
        let _: () = redis.expire(&rl_key, 10).await.unwrap_or(());
    }
    if current > 20 {
        return util::api_error(ApiErrorCode::RateLimited);
    }

    let body = req.body.trim().to_string();
    if body.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    if body.len() > 2000 {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let (org_id, _channel_kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));

    // 2. validate membership + 3. permission check
    let ok = match util::can(&state.pool, auth.user_id, org_id, permissions::Permission::MessagesSend).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !ok {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    // 4. insert message in short transaction
    let now = OffsetDateTime::now_utc();
    let message_id = Uuid::now_v7();
    tracing::info!(organization_id=%org_id, channel_id=%channel_id, message_id=%message_id, "creating message");

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let inserted = sqlx::query(
        r#"
        insert into messages (id, organization_id, channel_id, sender_id, body, kind, created_at)
        values ($1, $2, $3, $4, $5, 'text', $6)
        "#,
    )
    .bind(message_id)
    .bind(org_id)
    .bind(channel_id)
    .bind(auth.user_id)
    .bind(body.clone())
    .bind(now)
    .execute(&mut *tx)
    .await;

    if inserted.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    // 6. publish event to NATS Core after commit
    #[derive(serde::Serialize)]
    struct MessageCreatedData {
        channel_id: Uuid,
        message_id: Uuid,
    }
    let env = EventEnvelope::new(
        "message.created",
        org_id,
        Some(auth.user_id),
        MessageCreatedData {
            channel_id,
            message_id,
        },
    );
    let subject = events::subjects::message_created(org_id, channel_id);
    if let Err(err) = events::core::publish(&state.nats, subject, &env).await {
        error!(error = %err, "failed to publish message.created");
    }

    (
        StatusCode::OK,
        Json(MessageResponse {
            id: message_id,
            organization_id: org_id,
            channel_id,
            sender_id: auth.user_id,
            body: Some(body),
            kind: "text".to_string(),
            created_at: now,
            edited_at: None,
            deleted_at: None,
        }),
    )
        .into_response()
}

async fn edit_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(message_id): Path<Uuid>,
    Json(req): Json<EditMessageRequest>,
) -> impl IntoResponse {
    let Some(body) = req.body.map(|b| b.trim().to_string()) else {
        return util::api_error(ApiErrorCode::ValidationError);
    };

    let row = sqlx::query(
        r#"
        select organization_id, channel_id, sender_id
        from messages
        where id = $1 and deleted_at is null
        "#,
    )
    .bind(message_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let org_id: Uuid = row.get("organization_id");
    let sender_id: Uuid = row.get("sender_id");

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if sender_id != auth.user_id {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }
    if !permissions::has(perms, perms::MESSAGES_EDIT_OWN) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let updated = sqlx::query(
        r#"
        update messages
        set body = $2, edited_at = now()
        where id = $1
        "#,
    )
    .bind(message_id)
    .bind(if body.is_empty() { None::<String> } else { Some(body) })
    .execute(&state.pool)
    .await;

    match updated {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn delete_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(message_id): Path<Uuid>,
) -> impl IntoResponse {
    let row = sqlx::query(
        r#"
        select organization_id, sender_id
        from messages
        where id = $1 and deleted_at is null
        "#,
    )
    .bind(message_id)
    .fetch_optional(&state.pool)
    .await;

    let Some(row) = (match row {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    }) else {
        return util::api_error(ApiErrorCode::NotFound);
    };

    let org_id: Uuid = row.get("organization_id");
    let sender_id: Uuid = row.get("sender_id");

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if sender_id == auth.user_id {
        if !permissions::has(perms, perms::MESSAGES_DELETE_OWN) {
            return util::api_error(ApiErrorCode::PermissionDenied);
        }
    } else if !permissions::has(perms, perms::MESSAGES_DELETE_ANY) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let deleted = sqlx::query(
        r#"
        update messages
        set deleted_at = now()
        where id = $1
        "#,
    )
    .bind(message_id)
    .execute(&state.pool)
    .await;

    match deleted {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn add_reaction(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(message_id): Path<Uuid>,
    Json(req): Json<ReactionRequest>,
) -> impl IntoResponse {
    let emoji = req.emoji.trim().to_string();
    if emoji.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let org_id = match get_message_org(&state.pool, message_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::MESSAGES_REACT) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let inserted = sqlx::query(
        r#"
        insert into message_reactions (organization_id, message_id, user_id, emoji)
        values ($1, $2, $3, $4)
        on conflict do nothing
        "#,
    )
    .bind(org_id)
    .bind(message_id)
    .bind(auth.user_id)
    .bind(emoji)
    .execute(&state.pool)
    .await;

    match inserted {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn remove_reaction(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((message_id, emoji)): Path<(Uuid, String)>,
) -> impl IntoResponse {
    let emoji = emoji.trim().to_string();
    if emoji.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let org_id = match get_message_org(&state.pool, message_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, perms::MESSAGES_REACT) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let deleted = sqlx::query(
        r#"
        delete from message_reactions
        where message_id = $1 and user_id = $2 and emoji = $3
        "#,
    )
    .bind(message_id)
    .bind(auth.user_id)
    .bind(emoji)
    .execute(&state.pool)
    .await;

    match deleted {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response(),
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn get_channel_org(pool: &PgPool, channel_id: Uuid) -> Result<(Uuid, String), axum::response::Response> {
    let row = sqlx::query(
        r#"
        select organization_id, kind
        from channels
        where id = $1
        "#,
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    let Some(row) = row else {
        return Err(util::api_error(ApiErrorCode::NotFound));
    };

    Ok((row.get("organization_id"), row.get("kind")))
}

async fn get_message_org(pool: &PgPool, message_id: Uuid) -> Result<Uuid, axum::response::Response> {
    let row = sqlx::query(
        r#"
        select organization_id
        from messages
        where id = $1
        "#,
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    let Some(row) = row else {
        return Err(util::api_error(ApiErrorCode::NotFound));
    };
    Ok(row.get("organization_id"))
}

async fn parse_before(
    pool: &PgPool,
    channel_id: Uuid,
    before: Option<&str>,
) -> Result<Option<(OffsetDateTime, Uuid)>, axum::response::Response> {
    let Some(before) = before else { return Ok(None); };

    // Allow UUID (message id) or RFC3339 timestamp
    if let Ok(message_id) = Uuid::parse_str(before) {
        let row = sqlx::query(
            r#"
            select created_at, id
            from messages
            where id = $1 and channel_id = $2
            "#,
        )
        .bind(message_id)
        .bind(channel_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

        let Some(row) = row else {
            return Err(util::api_error(ApiErrorCode::ValidationError));
        };
        let created_at: OffsetDateTime = row.get("created_at");
        let id: Uuid = row.get("id");
        return Ok(Some((created_at, id)));
    }

    let created_at = OffsetDateTime::parse(before, &Rfc3339)
        .map_err(|_| util::api_error(ApiErrorCode::ValidationError))?;
    Ok(Some((created_at, Uuid::max())))
}

fn encode_cursor(created_at: OffsetDateTime, id: Uuid) -> String {
    let ts = created_at.format(&Rfc3339).unwrap_or_default();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("{ts}|{id}"))
}
