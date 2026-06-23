//! Admin control over which org channels Lorelei is added to, per-user BYO LLM credentials/
//! preferences, the personal-PM start endpoint, and the message-route hook that triggers a
//! reply.
//!
//! Lorelei is a single global user (`lorelei_bridge::LORELEI_USER_ID`), not a bot per org —
//! see `LORELEI_BUILDPLAN.md` (flux frontend repo) Section 0 for the full redesign. This file
//! is the only place in `api-server` that touches `org_lorelei_settings`,
//! `org_lorelei_channels`, `user_llm_credentials`, `user_lorelei_preferences`, or
//! `user_lorelei_threads`.

use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Router,
};
use events::envelope::EventEnvelope;
use lorelei_bridge::LORELEI_USER_ID;
use permissions::Permission;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

const VALID_PROVIDERS: [&str; 3] = ["ollama", "openai", "anthropic"];
const VALID_CREDENTIAL_PROVIDERS: [&str; 2] = ["openai", "anthropic"];

/// Mounted under `/orgs` in `app.rs` (merged alongside `routes_orgs::router()`).
pub fn org_router() -> Router<AppState> {
    Router::new()
        .route(
            "/{org_id}/lorelei/channels",
            get(list_channels).post(add_channel),
        )
        .route(
            "/{org_id}/lorelei/channels/{channel_id}",
            delete(remove_channel),
        )
        .route("/{org_id}/lorelei/pm", post(start_pm))
}

/// Mounted under `/auth` in `app.rs` (merged alongside `routes_auth::router()`).
pub fn me_router() -> Router<AppState> {
    Router::new()
        .route("/me/llm-credentials", get(list_credentials))
        .route(
            "/me/llm-credentials/{provider}",
            put(upsert_credential).delete(remove_credential),
        )
        .route(
            "/me/lorelei-preferences",
            get(get_preferences).patch(update_preferences),
        )
}

// ---------------------------------------------------------------------------
// Personal PM
// ---------------------------------------------------------------------------

/// Starts (or resumes) the caller's personal DM thread with Lorelei in this org. Idempotent —
/// reuses the existing channel/tenant/agent on every call after the first. Unlike a normal
/// DM start, no friend-request relationship is required; you don't need to "friend" an AI
/// assistant to talk to it.
async fn start_pm(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    match util::is_member(&state.pool, org_id, auth.user_id).await {
        Ok(true) => {}
        Ok(false) => return util::api_error(ApiErrorCode::PermissionDenied),
        Err(e) => return e,
    }

    match lorelei_bridge::load_or_create_user_thread(&state.pool, auth.user_id, org_id).await {
        Ok(thread) => (
            StatusCode::OK,
            Json(serde_json::json!({ "channel_id": thread.dm_channel_id })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to start lorelei pm");
            util::api_error(ApiErrorCode::InternalError)
        }
    }
}

// ---------------------------------------------------------------------------
// Admin-extended channel availability
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct LoreleiChannelResponse {
    channel_id: Uuid,
    channel_name: String,
    enabled_by: Uuid,
    created_at: OffsetDateTime,
}

async fn list_channels(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    match util::can(&state.pool, auth.user_id, org_id, Permission::LoreleiManage).await {
        Ok(true) => {}
        Ok(false) => return util::api_error(ApiErrorCode::PermissionDenied),
        Err(e) => return e,
    }

    let rows = sqlx::query(
        "select c.id as channel_id, c.name as channel_name, oc.enabled_by, oc.created_at \
         from org_lorelei_channels oc \
         join channels c on c.id = oc.channel_id \
         where oc.organization_id = $1 \
         order by oc.created_at asc",
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let out: Vec<LoreleiChannelResponse> = rows
        .iter()
        .map(|r| LoreleiChannelResponse {
            channel_id: r.get("channel_id"),
            channel_name: r.get("channel_name"),
            enabled_by: r.get("enabled_by"),
            created_at: r.get("created_at"),
        })
        .collect();

    (StatusCode::OK, Json(out)).into_response()
}

#[derive(Debug, Deserialize)]
struct AddChannelRequest {
    channel_id: Uuid,
}

async fn add_channel(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Json(req): Json<AddChannelRequest>,
) -> impl IntoResponse {
    match util::can(&state.pool, auth.user_id, org_id, Permission::LoreleiManage).await {
        Ok(true) => {}
        Ok(false) => return util::api_error(ApiErrorCode::PermissionDenied),
        Err(e) => return e,
    }

    let channel_org: Option<Uuid> =
        match sqlx::query_scalar("select organization_id from channels where id = $1")
            .bind(req.channel_id)
            .fetch_optional(&state.pool)
            .await
        {
            Ok(v) => v,
            Err(_) => return util::api_error(ApiErrorCode::InternalError),
        };
    if channel_org != Some(org_id) {
        return util::api_error(ApiErrorCode::NotFound);
    }

    // Lazily provisions the org's Lorelei tenant/agent on the very first channel added.
    if let Err(e) = lorelei_bridge::provision_org_lorelei(&state.pool, org_id).await {
        tracing::error!(error = %e, "failed to provision org lorelei tenant/agent");
        return util::api_error(ApiErrorCode::InternalError);
    }

    let inserted = sqlx::query(
        "insert into org_lorelei_channels (id, organization_id, channel_id, enabled_by, created_at) \
         values ($1, $2, $3, $4, now()) \
         on conflict (organization_id, channel_id) do nothing",
    )
    .bind(Uuid::now_v7())
    .bind(org_id)
    .bind(req.channel_id)
    .bind(auth.user_id)
    .execute(&state.pool)
    .await;

    if inserted.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    util::write_audit_log(
        &state.pool,
        org_id,
        Some(auth.user_id),
        "lorelei.channel_added",
        Some("channel"),
        Some(req.channel_id),
        serde_json::json!({}),
    )
    .await;

    StatusCode::NO_CONTENT.into_response()
}

async fn remove_channel(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((org_id, channel_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    match util::can(&state.pool, auth.user_id, org_id, Permission::LoreleiManage).await {
        Ok(true) => {}
        Ok(false) => return util::api_error(ApiErrorCode::PermissionDenied),
        Err(e) => return e,
    }

    let res = sqlx::query(
        "delete from org_lorelei_channels where organization_id = $1 and channel_id = $2",
    )
    .bind(org_id)
    .bind(channel_id)
    .execute(&state.pool)
    .await;

    if res.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    util::write_audit_log(
        &state.pool,
        org_id,
        Some(auth.user_id),
        "lorelei.channel_removed",
        Some("channel"),
        Some(channel_id),
        serde_json::json!({}),
    )
    .await;

    StatusCode::NO_CONTENT.into_response()
}

// ---------------------------------------------------------------------------
// Per-user BYO LLM credentials
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct CredentialResponse {
    provider: String,
    fingerprint: String,
    updated_at: OffsetDateTime,
}

async fn list_credentials(State(state): State<AppState>, auth: AuthContext) -> impl IntoResponse {
    let rows = sqlx::query(
        "select provider, key_fingerprint, updated_at from user_llm_credentials where user_id = $1 \
         order by provider asc",
    )
    .bind(auth.user_id)
    .fetch_all(&state.pool)
    .await;

    let rows = match rows {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let out: Vec<CredentialResponse> = rows
        .iter()
        .map(|r| CredentialResponse {
            provider: r.get("provider"),
            fingerprint: r.get("key_fingerprint"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    (StatusCode::OK, Json(out)).into_response()
}

#[derive(Debug, Deserialize)]
struct UpsertCredentialRequest {
    api_key: String,
}

async fn upsert_credential(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(provider): Path<String>,
    Json(req): Json<UpsertCredentialRequest>,
) -> impl IntoResponse {
    if !VALID_CREDENTIAL_PROVIDERS.contains(&provider.as_str()) {
        return util::api_error(ApiErrorCode::ValidationError);
    }
    let api_key = req.api_key.trim();
    if api_key.is_empty() || api_key.len() > 512 {
        return util::api_error(ApiErrorCode::ValidationError);
    }

    let Some(lorelei) = state.lorelei.as_ref() else {
        return util::api_error_msg(
            ApiErrorCode::InternalError,
            "Lorelei credential encryption is not configured on this server.",
        );
    };

    let encrypted = match secrets::encrypt(api_key, &lorelei.credentials_key) {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    let fingerprint = secrets::fingerprint(api_key);

    let res = sqlx::query(
        r#"
        insert into user_llm_credentials (id, user_id, provider, encrypted_api_key, key_fingerprint, created_at, updated_at)
        values ($1, $2, $3, $4, $5, now(), now())
        on conflict (user_id, provider) do update set
            encrypted_api_key = excluded.encrypted_api_key,
            key_fingerprint = excluded.key_fingerprint,
            updated_at = now()
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(auth.user_id)
    .bind(&provider)
    .bind(&encrypted)
    .bind(&fingerprint)
    .execute(&state.pool)
    .await;

    if res.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    (
        StatusCode::OK,
        Json(CredentialResponse {
            provider,
            fingerprint,
            updated_at: OffsetDateTime::now_utc(),
        }),
    )
        .into_response()
}

async fn remove_credential(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(provider): Path<String>,
) -> impl IntoResponse {
    let res = sqlx::query("delete from user_llm_credentials where user_id = $1 and provider = $2")
        .bind(auth.user_id)
        .bind(&provider)
        .execute(&state.pool)
        .await;

    if res.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    StatusCode::NO_CONTENT.into_response()
}

// ---------------------------------------------------------------------------
// Per-user Lorelei preferences
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct PreferencesResponse {
    preferred_provider: Option<String>,
    preferred_model: Option<String>,
}

async fn get_preferences(State(state): State<AppState>, auth: AuthContext) -> impl IntoResponse {
    let row = sqlx::query(
        "select preferred_provider, preferred_model from user_lorelei_preferences where user_id = $1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await;

    let row = match row {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let resp = match row {
        Some(r) => PreferencesResponse {
            preferred_provider: r.get("preferred_provider"),
            preferred_model: r.get("preferred_model"),
        },
        None => PreferencesResponse {
            preferred_provider: None,
            preferred_model: None,
        },
    };

    (StatusCode::OK, Json(resp)).into_response()
}

/// Note: full-replace semantics, not a sparse patch — both fields are always written.
/// `null`/omitted both mean "inherit the platform default". A real sparse PATCH
/// (distinguishing "don't touch" from "clear") needs a double-`Option` deserializer; not
/// worth it until the frontend actually needs that distinction.
#[derive(Debug, Deserialize)]
struct UpdatePreferencesRequest {
    #[serde(default)]
    preferred_provider: Option<String>,
    #[serde(default)]
    preferred_model: Option<String>,
}

async fn update_preferences(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdatePreferencesRequest>,
) -> impl IntoResponse {
    if let Some(p) = &req.preferred_provider {
        if !VALID_PROVIDERS.contains(&p.as_str()) {
            return util::api_error(ApiErrorCode::ValidationError);
        }
    }

    let res = sqlx::query(
        r#"
        insert into user_lorelei_preferences (user_id, preferred_provider, preferred_model, updated_at)
        values ($1, $2, $3, now())
        on conflict (user_id) do update set
            preferred_provider = excluded.preferred_provider,
            preferred_model = excluded.preferred_model,
            updated_at = now()
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.preferred_provider)
    .bind(&req.preferred_model)
    .execute(&state.pool)
    .await;

    if res.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    (
        StatusCode::OK,
        Json(PreferencesResponse {
            preferred_provider: req.preferred_provider,
            preferred_model: req.preferred_model,
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Message-route hook (called from `routes_messages::send_message`)
// ---------------------------------------------------------------------------

/// Two trigger paths:
/// - **PM**: `channel_id` is a DM channel that includes the global Lorelei user as a member.
///   Any member can do this (same as messaging any other user) — runs use the *sender's own*
///   resolved provider/credential, in the sender's personal per-(user, org) memory scope.
/// - **Org channel**: `channel_id` is in `org_lorelei_channels`. Requires
///   `LORELEI_INVOKE_CHANNEL`. Runs use the *org owner's* resolved provider/credential, in the
///   org's shared memory scope.
///
/// Cheap to call on every message send — the eligibility check is 1-2 indexed point lookups;
/// the actual LLM call happens in a spawned task so it never delays the sender's own response.
pub(crate) async fn maybe_trigger_reply(
    state: &AppState,
    org_id: Uuid,
    channel_id: Uuid,
    sender_user_id: Uuid,
    body: &str,
) {
    let Some(lorelei) = state.lorelei.clone() else {
        return;
    };
    if sender_user_id == LORELEI_USER_ID {
        return; // defensive: never let her reply to herself
    }

    let kind: Option<String> = sqlx::query_scalar("select kind from channels where id = $1")
        .bind(channel_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
    let Some(kind) = kind else {
        return;
    };

    let pool = state.pool.clone();
    let nats = state.nats.clone();
    let input = body.to_string();

    if kind == "dm" {
        let is_lorelei_dm: Option<i32> = sqlx::query_scalar(
            "select 1 from dm_channel_members where channel_id = $1 and user_id = $2",
        )
        .bind(channel_id)
        .bind(LORELEI_USER_ID)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();
        if is_lorelei_dm.is_none() {
            return;
        }

        tokio::spawn(async move {
            let thread = match lorelei_bridge::load_or_create_user_thread(&pool, sender_user_id, org_id).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!(error = %e, "failed to load lorelei user thread");
                    return;
                }
            };
            run_and_reply(&pool, &nats, &lorelei, org_id, channel_id, thread.tenant_id, thread.agent_id, sender_user_id, input).await;
        });
        return;
    }

    let is_extended: Option<i32> = sqlx::query_scalar(
        "select 1 from org_lorelei_channels where organization_id = $1 and channel_id = $2",
    )
    .bind(org_id)
    .bind(channel_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    if is_extended.is_none() {
        return;
    }
    match util::can(&state.pool, sender_user_id, org_id, Permission::LoreleiInvokeChannel).await {
        Ok(true) => {}
        _ => return,
    }

    let owner_id: Option<Uuid> = sqlx::query_scalar(
        "select user_id from organization_members where organization_id = $1 and role = 'owner' limit 1",
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    let Some(owner_id) = owner_id else {
        return;
    };

    tokio::spawn(async move {
        let org = match lorelei_bridge::load_org_lorelei(&pool, org_id).await {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(error = %e, "org channel lorelei tenant/agent missing");
                return;
            }
        };
        run_and_reply(&pool, &nats, &lorelei, org_id, channel_id, org.tenant_id, org.agent_id, owner_id, input).await;
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_and_reply(
    pool: &PgPool,
    nats: &async_nats::Client,
    lorelei: &crate::LoreleiRuntime,
    org_id: Uuid,
    channel_id: Uuid,
    tenant_id: Uuid,
    agent_id: Uuid,
    credential_user_id: Uuid,
    input: String,
) {
    let provider = match lorelei_bridge::resolve_provider(pool, &lorelei.credentials_key, credential_user_id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "failed to resolve lorelei provider");
            post_bot_message(
                pool,
                nats,
                org_id,
                channel_id,
                "Lorelei couldn't respond — no API key is configured for the selected model.",
            )
            .await;
            return;
        }
    };

    let outcome = lorelei
        .harbor
        .run_and_wait(
            tenant_id,
            agent_id,
            input,
            provider,
            lorelei_bridge::MaxRisk::Low,
            std::time::Duration::from_secs(60),
        )
        .await;

    let text = match outcome {
        Ok(lorelei_bridge::RunOutcome::Succeeded(text)) => text,
        Ok(lorelei_bridge::RunOutcome::Denied(text)) => text,
        Ok(lorelei_bridge::RunOutcome::TimedOut) => {
            "Lorelei is taking longer than expected — try again in a moment.".to_string()
        }
        Err(e) => {
            tracing::error!(error = %e, "lorelei run failed");
            "Lorelei couldn't respond — try again.".to_string()
        }
    };

    post_bot_message(pool, nats, org_id, channel_id, &text).await;
}

async fn post_bot_message(
    pool: &PgPool,
    nats: &async_nats::Client,
    org_id: Uuid,
    channel_id: Uuid,
    body: &str,
) {
    let message_id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();

    let inserted = sqlx::query(
        "insert into messages (id, organization_id, channel_id, sender_id, body, kind, created_at) \
         values ($1, $2, $3, $4, $5, 'text', $6)",
    )
    .bind(message_id)
    .bind(org_id)
    .bind(channel_id)
    .bind(LORELEI_USER_ID)
    .bind(body)
    .bind(now)
    .execute(pool)
    .await;

    if inserted.is_err() {
        tracing::error!(%org_id, %channel_id, "failed to insert lorelei reply message");
        return;
    }

    #[derive(Serialize)]
    struct MessageCreatedData {
        channel_id: Uuid,
        message_id: Uuid,
    }
    let env = EventEnvelope::new(
        "message.created",
        org_id,
        Some(LORELEI_USER_ID),
        MessageCreatedData {
            channel_id,
            message_id,
        },
    );
    let subject = events::subjects::message_created(org_id, channel_id);
    if let Err(err) = events::core::publish(nats, subject, &env).await {
        tracing::error!(error = %err, "failed to publish lorelei message.created");
    }
}
