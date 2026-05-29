use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use events::envelope::EventEnvelope;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use tracing::{error, Span};
use uuid::Uuid;

const PIN_LIMIT_PER_CHANNEL: i64 = 50;

pub fn router() -> Router<AppState> {
    Router::new()
        // Threads
        .route(
            "/channels/{channel_id}/threads",
            get(list_threads).post(create_thread_root),
        )
        .route("/threads/{thread_id}", get(get_thread))
        .route(
            "/threads/{thread_id}/replies",
            post(create_thread_reply),
        )
        // Pins
        .route(
            "/channels/{channel_id}/pins",
            get(list_pins).post(pin_message),
        )
        .route(
            "/channels/{channel_id}/pins/{message_id}",
            axum::routing::delete(unpin_message),
        )
        // Search
        .route("/channels/{channel_id}/search", get(search_channel))
}

#[derive(Debug, Deserialize)]
struct CreateThreadRequest {
    body: Option<String>,
    root_message_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct ThreadResponse {
    id: Uuid,
    organization_id: Uuid,
    channel_id: Uuid,
    root_message_id: Uuid,
    created_by: Uuid,
    created_at: OffsetDateTime,
    last_reply_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize)]
struct ThreadWithMessagesResponse {
    thread: ThreadResponse,
    root: serde_json::Value,
    replies: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ThreadListEntry {
    thread: ThreadResponse,
    root: serde_json::Value,
    reply_count: i64,
}

#[derive(Debug, Serialize)]
struct ThreadsListResponse {
    threads: Vec<ThreadListEntry>,
}

async fn list_threads(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> impl IntoResponse {
    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let (org_id, _kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select id, organization_id, channel_id, root_message_id, created_by, created_at, last_reply_at
        from threads
        where channel_id = $1
        order by coalesce(last_reply_at, created_at) desc, created_at desc
        limit 50
        "#,
    )
    .bind(channel_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let mut threads: Vec<ThreadListEntry> = Vec::with_capacity(rows.len());
    for row in rows {
        let thread_id: Uuid = row.get("id");
        let root_message_id: Uuid = row.get("root_message_id");
        let thread = ThreadResponse {
            id: thread_id,
            organization_id: row.get("organization_id"),
            channel_id: row.get("channel_id"),
            root_message_id,
            created_by: row.get("created_by"),
            created_at: row.get("created_at"),
            last_reply_at: row
                .try_get::<Option<OffsetDateTime>, _>("last_reply_at")
                .ok()
                .flatten(),
        };

        let root = fetch_message(&state.pool, root_message_id)
            .await
            .unwrap_or(None)
            .unwrap_or(serde_json::Value::Null);

        let reply_count: i64 = sqlx::query_scalar(
            r#"
            select count(1)::bigint
            from messages
            where thread_id = $1
              and deleted_at is null
              and id <> $2
            "#,
        )
        .bind(thread_id)
        .bind(root_message_id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

        threads.push(ThreadListEntry {
            thread,
            root,
            reply_count,
        });
    }

    (StatusCode::OK, Json(ThreadsListResponse { threads })).into_response()
}

async fn create_thread_root(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<CreateThreadRequest>,
) -> impl IntoResponse {
    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let (org_id, _kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let now = OffsetDateTime::now_utc();
    let thread_id = Uuid::now_v7();

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    // Root can be an existing message or a new message body.
    let message_id = if let Some(root_id) = req.root_message_id {
        // Validate root message belongs to channel + org and isn't deleted.
        let root_row = sqlx::query(
            r#"
            select id, thread_id
            from messages
            where id = $1 and channel_id = $2 and organization_id = $3 and deleted_at is null
            "#,
        )
        .bind(root_id)
        .bind(channel_id)
        .bind(org_id)
        .fetch_optional(&mut *tx)
        .await;

        let Some(root_row) = root_row.ok().flatten() else {
            return util::api_error(ApiErrorCode::NotFound);
        };

        let existing_thread: Option<Uuid> = root_row.try_get("thread_id").ok();
        if let Some(existing_thread) = existing_thread {
            // Thread already exists; treat as idempotent.
            let _ = tx.rollback().await;

            let thread = match get_thread_row(&state.pool, existing_thread).await {
                Ok(t) => t,
                Err(e) => return e,
            };

            return (
                StatusCode::OK,
                Json(ThreadResponse {
                    id: thread.id,
                    organization_id: thread.organization_id,
                    channel_id: thread.channel_id,
                    root_message_id: thread.root_message_id,
                    created_by: thread.created_by,
                    created_at: thread.created_at,
                    last_reply_at: thread.last_reply_at,
                }),
            )
                .into_response();
        }

        root_id
    } else {
        let body = req.body.map(|b| b.trim().to_string()).filter(|b| !b.is_empty());
        if body.is_none() {
            return util::api_error(ApiErrorCode::ValidationError);
        }

        let message_id = Uuid::now_v7();
        let inserted = sqlx::query(
            r#"
            insert into messages (id, organization_id, channel_id, sender_id, body, kind, created_at)
            values ($1,$2,$3,$4,$5,'text',$6)
            "#,
        )
        .bind(message_id)
        .bind(org_id)
        .bind(channel_id)
        .bind(auth.user_id)
        .bind(body)
        .bind(now)
        .execute(&mut *tx)
        .await;
        if inserted.is_err() {
            return util::api_error(ApiErrorCode::InternalError);
        }
        message_id
    };

    let thread_inserted = sqlx::query(
        r#"
        insert into threads (id, organization_id, channel_id, root_message_id, created_by, created_at, last_reply_at)
        values ($1,$2,$3,$4,$5,$6,null)
        "#,
    )
    .bind(thread_id)
    .bind(org_id)
    .bind(channel_id)
    .bind(message_id)
    .bind(auth.user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if thread_inserted.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let linked = sqlx::query(
        r#"
        update messages
        set thread_id = $2
        where id = $1
        "#,
    )
    .bind(message_id)
    .bind(thread_id)
    .execute(&mut *tx)
    .await;
    if linked.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    (
        StatusCode::OK,
        Json(ThreadResponse {
            id: thread_id,
            organization_id: org_id,
            channel_id,
            root_message_id: message_id,
            created_by: auth.user_id,
            created_at: now,
            last_reply_at: None,
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct ReplyRequest {
    body: Option<String>,
}

async fn create_thread_reply(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(thread_id): Path<Uuid>,
    Json(req): Json<ReplyRequest>,
) -> impl IntoResponse {
    let thread = match get_thread_row(&state.pool, thread_id).await {
        Ok(t) => t,
        Err(e) => return e,
    };

    Span::current().record(
        "organization_id",
        tracing::field::display(thread.organization_id),
    );

    let can_access =
        match util::can_access_channel(&state.pool, auth.user_id, thread.channel_id).await {
            Ok(v) => v,
            Err(e) => return e,
        };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }
    let perms = match util::member_perms(&state.pool, thread.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let body = req.body.map(|b| b.trim().to_string()).filter(|b| !b.is_empty());
    if body.is_none() {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let now = OffsetDateTime::now_utc();
    let message_id = Uuid::now_v7();

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let inserted = sqlx::query(
        r#"
        insert into messages (id, organization_id, channel_id, sender_id, body, kind, created_at, thread_id)
        values ($1,$2,$3,$4,$5,'text',$6,$7)
        "#,
    )
    .bind(message_id)
    .bind(thread.organization_id)
    .bind(thread.channel_id)
    .bind(auth.user_id)
    .bind(body)
    .bind(now)
    .bind(thread_id)
    .execute(&mut *tx)
    .await;
    if inserted.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    let updated = sqlx::query(
        r#"
        update threads
        set last_reply_at = $2
        where id = $1
        "#,
    )
    .bind(thread_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if updated.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    if tx.commit().await.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    // Publish realtime event (best-effort)
    let env = EventEnvelope::new(
        "thread.reply.created",
        thread.organization_id,
        Some(auth.user_id),
        events::messaging::ThreadReplyCreatedData {
            channel_id: thread.channel_id,
            thread_id,
            thread_root_id: thread.root_message_id,
            message_id,
            occurred_at: now,
        },
    );
    let subject =
        events::subjects::thread_reply_created(thread.organization_id, thread.channel_id, thread_id);
    if let Err(err) = events::core::publish(&state.nats, subject, &env).await {
        error!(error=%err, "failed to publish thread.reply.created");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"message_id": message_id})),
    )
        .into_response()
}

async fn get_thread(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(thread_id): Path<Uuid>,
) -> impl IntoResponse {
    let thread = match get_thread_row(&state.pool, thread_id).await {
        Ok(t) => t,
        Err(e) => return e,
    };

    Span::current().record(
        "organization_id",
        tracing::field::display(thread.organization_id),
    );

    let can_access =
        match util::can_access_channel(&state.pool, auth.user_id, thread.channel_id).await {
            Ok(v) => v,
            Err(e) => return e,
        };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let perms = match util::member_perms(&state.pool, thread.organization_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let root = fetch_message(&state.pool, thread.root_message_id)
        .await
        .unwrap_or(None)
        .unwrap_or(serde_json::Value::Null);
    let replies = fetch_thread_replies(&state.pool, thread_id, thread.root_message_id)
        .await
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(ThreadWithMessagesResponse {
            thread: ThreadResponse {
                id: thread.id,
                organization_id: thread.organization_id,
                channel_id: thread.channel_id,
                root_message_id: thread.root_message_id,
                created_by: thread.created_by,
                created_at: thread.created_at,
                last_reply_at: thread.last_reply_at,
            },
            root,
            replies,
        }),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
struct PinRequest {
    message_id: Uuid,
}

async fn pin_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<PinRequest>,
) -> impl IntoResponse {
    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let (org_id, _kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    // Validate message belongs to channel + org.
    let ok_res = sqlx::query_scalar::<_, i64>(
        r#"
        select 1::bigint
        from messages
        where id = $1 and channel_id = $2 and organization_id = $3
        "#,
    )
    .bind(req.message_id)
    .bind(channel_id)
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;

    let ok = match ok_res {
        Ok(v) => v.is_some(),
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if !ok {
        return util::api_error(ApiErrorCode::NotFound);
    }

    let pin_count: i64 = sqlx::query_scalar(
        r#"select count(1)::bigint from channel_pins where channel_id = $1"#,
    )
    .bind(channel_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);
    if pin_count >= PIN_LIMIT_PER_CHANNEL {
        return util::api_error_msg(ApiErrorCode::ValidationError, "Pin limit reached.");
    }

    let inserted = sqlx::query(
        r#"
        insert into channel_pins (organization_id, channel_id, message_id, pinned_by, pinned_at)
        values ($1,$2,$3,$4,now())
        on conflict do nothing
        "#,
    )
    .bind(org_id)
    .bind(channel_id)
    .bind(req.message_id)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await;

    match inserted {
        Ok(_) => {
            let now = OffsetDateTime::now_utc();
            let env = EventEnvelope::new(
                "channel.pins.changed",
                org_id,
                Some(auth.user_id),
                events::messaging::ChannelPinsChangedData {
                    channel_id,
                    message_id: req.message_id,
                    action: "pinned".to_string(),
                    occurred_at: now,
                },
            );
            let subject = events::subjects::channel_pins_changed(org_id, channel_id);
            let _ = events::core::publish(&state.nats, subject, &env).await;

            (StatusCode::OK, Json(serde_json::json!({"status":"pinned"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

async fn unpin_message(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }
    let (org_id, _kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));
    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let deleted = sqlx::query(
        r#"delete from channel_pins where channel_id = $1 and message_id = $2"#,
    )
    .bind(channel_id)
    .bind(message_id)
    .execute(&state.pool)
    .await;

    match deleted {
        Ok(r) if r.rows_affected() == 0 => util::api_error(ApiErrorCode::NotFound),
        Ok(_) => {
            let now = OffsetDateTime::now_utc();
            let env = EventEnvelope::new(
                "channel.pins.changed",
                org_id,
                Some(auth.user_id),
                events::messaging::ChannelPinsChangedData {
                    channel_id,
                    message_id,
                    action: "unpinned".to_string(),
                    occurred_at: now,
                },
            );
            let subject = events::subjects::channel_pins_changed(org_id, channel_id);
            let _ = events::core::publish(&state.nats, subject, &env).await;

            (StatusCode::OK, Json(serde_json::json!({"status":"unpinned"}))).into_response()
        }
        Err(_) => util::api_error(ApiErrorCode::InternalError),
    }
}

#[derive(Debug, Serialize)]
struct PinEntry {
    message_id: Uuid,
    pinned_by: Uuid,
    pinned_at: OffsetDateTime,
    message: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct PinsResponse {
    pins: Vec<PinEntry>,
}

async fn list_pins(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
) -> impl IntoResponse {
    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let (org_id, _kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let rows = sqlx::query(
        r#"
        select p.message_id, p.pinned_by, p.pinned_at,
               m.organization_id, m.channel_id, m.sender_id, m.thread_id, m.body, m.kind, m.created_at
        from channel_pins p
        join messages m on m.id = p.message_id
        where p.channel_id = $1
          and m.deleted_at is null
        order by pinned_at desc
        limit 50
        "#,
    )
    .bind(channel_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    let pins = rows
        .into_iter()
        .map(|r| PinEntry {
            message_id: r.get("message_id"),
            pinned_by: r.get("pinned_by"),
            pinned_at: r.get("pinned_at"),
            message: serde_json::json!({
                "id": r.get::<Uuid,_>("message_id"),
                "organization_id": r.get::<Uuid,_>("organization_id"),
                "channel_id": r.get::<Uuid,_>("channel_id"),
                "sender_id": r.get::<Uuid,_>("sender_id"),
                "thread_id": r.try_get::<Option<Uuid>,_>("thread_id").ok().flatten(),
                "body": r.try_get::<Option<String>,_>("body").ok().flatten(),
                "kind": r.get::<String,_>("kind"),
                "created_at": r.get::<OffsetDateTime,_>("created_at"),
            }),
        })
        .collect();
    (StatusCode::OK, Json(PinsResponse { pins })).into_response()
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<i64>,
}

async fn search_channel(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(channel_id): Path<Uuid>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    let can_access = match util::can_access_channel(&state.pool, auth.user_id, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    if !can_access {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }
    let (org_id, _kind) = match get_channel_org(&state.pool, channel_id).await {
        Ok(v) => v,
        Err(e) => return e,
    };
    Span::current().record("organization_id", tracing::field::display(org_id));

    let perms = match util::member_perms(&state.pool, org_id, auth.user_id).await {
        Ok(p) => p,
        Err(e) => return e,
    };
    if !permissions::has(perms, permissions::perms::MESSAGES_SEND) {
        return util::api_error(ApiErrorCode::PermissionDenied);
    }

    let query = q.q.trim();
    if query.is_empty() {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    let limit = q.limit.unwrap_or(25).clamp(1, 100);
    let pattern = format!("%{}%", query.to_lowercase());

    // Exclude thread replies from search results; keep thread roots.
    let rows = sqlx::query(
        r#"
        select m.id, m.organization_id, m.channel_id, m.sender_id, m.thread_id, m.body, m.kind, m.created_at, m.edited_at, m.deleted_at
        from messages m
        left join threads t on t.id = m.thread_id
        where m.organization_id = $1
          and m.channel_id = $2
          and m.deleted_at is null
          and (m.thread_id is null or t.root_message_id = m.id)
          and m.body is not null
          and lower(m.body) like $3
        order by m.created_at desc
        limit $4
        "#,
    )
    .bind(org_id)
    .bind(channel_id)
    .bind(pattern)
    .bind(limit)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(r) => r,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let out = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
              "id": row.get::<Uuid,_>("id"),
              "organization_id": row.get::<Uuid,_>("organization_id"),
              "channel_id": row.get::<Uuid,_>("channel_id"),
              "sender_id": row.get::<Uuid,_>("sender_id"),
              "thread_id": row.try_get::<Option<Uuid>,_>("thread_id").ok().flatten(),
              "body": row.try_get::<Option<String>,_>("body").ok().flatten(),
              "kind": row.get::<String,_>("kind"),
              "created_at": row.get::<OffsetDateTime,_>("created_at"),
            })
        })
        .collect::<Vec<_>>();

    (StatusCode::OK, Json(serde_json::json!({ "messages": out }))).into_response()
}

// --- helpers ---

#[derive(Debug)]
struct ThreadRow {
    id: Uuid,
    organization_id: Uuid,
    channel_id: Uuid,
    root_message_id: Uuid,
    created_by: Uuid,
    created_at: OffsetDateTime,
    last_reply_at: Option<OffsetDateTime>,
}

async fn get_thread_row(pool: &PgPool, thread_id: Uuid) -> Result<ThreadRow, axum::response::Response> {
    let row = sqlx::query(
        r#"
        select id, organization_id, channel_id, root_message_id, created_by, created_at, last_reply_at
        from threads
        where id = $1
        "#,
    )
    .bind(thread_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| util::api_error(ApiErrorCode::InternalError))?;

    let Some(row) = row else {
        return Err(util::api_error(ApiErrorCode::NotFound));
    };
    Ok(ThreadRow {
        id: row.get("id"),
        organization_id: row.get("organization_id"),
        channel_id: row.get("channel_id"),
        root_message_id: row.get("root_message_id"),
        created_by: row.get("created_by"),
        created_at: row.get("created_at"),
        last_reply_at: row
            .try_get::<Option<OffsetDateTime>, _>("last_reply_at")
            .ok()
            .flatten(),
    })
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

async fn fetch_message(pool: &PgPool, message_id: Uuid) -> anyhow::Result<Option<serde_json::Value>> {
    let row = sqlx::query(
        r#"
        select id, organization_id, channel_id, sender_id, body, kind, created_at, edited_at, deleted_at
        from messages
        where id = $1
        "#,
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(serde_json::json!({
        "id": row.get::<Uuid,_>("id"),
        "organization_id": row.get::<Uuid,_>("organization_id"),
        "channel_id": row.get::<Uuid,_>("channel_id"),
        "sender_id": row.get::<Uuid,_>("sender_id"),
        "body": row.try_get::<Option<String>,_>("body").ok().flatten(),
        "kind": row.get::<String,_>("kind"),
        "created_at": row.get::<OffsetDateTime,_>("created_at"),
        "thread_id": row.try_get::<Option<Uuid>,_>("thread_id").ok().flatten(),
    })))
}

async fn fetch_thread_replies(
    pool: &PgPool,
    thread_id: Uuid,
    root_message_id: Uuid,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let rows = sqlx::query(
        r#"
        select id, organization_id, channel_id, sender_id, body, kind, created_at
        from messages
        where thread_id = $1
          and deleted_at is null
          and id <> $2
        order by created_at asc
        "#,
    )
    .bind(thread_id)
    .bind(root_message_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<Uuid,_>("id"),
                "organization_id": row.get::<Uuid,_>("organization_id"),
                "channel_id": row.get::<Uuid,_>("channel_id"),
                "sender_id": row.get::<Uuid,_>("sender_id"),
                "body": row.try_get::<Option<String>,_>("body").ok().flatten(),
                "kind": row.get::<String,_>("kind"),
                "created_at": row.get::<OffsetDateTime,_>("created_at"),
                "thread_id": thread_id,
            })
        })
        .collect())
}
