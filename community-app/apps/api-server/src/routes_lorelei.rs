//! Per-user BYO LLM credentials/preferences, and the message-route hook that triggers a
//! Lorelei reply.
//!
//! Lorelei is a single global user (`lorelei_bridge::LORELEI_USER_ID`) — there is no admin
//! UI for her at all. Two ways to reach her:
//! - **PM her directly** — she's a normal member of every org, so the existing friend/DM
//!   flow (`routes_dms.rs`) works on her exactly like it does on anyone else.
//! - **@mention her in any regular channel, as the org owner** — see `maybe_trigger_reply`.
//!
//! This file is the only place in `api-server` that touches `org_lorelei_settings`,
//! `user_llm_credentials`, or `user_lorelei_preferences`.

use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Router,
};
use events::envelope::EventEnvelope;
use lorelei_bridge::LORELEI_USER_ID;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

const VALID_PROVIDERS: [&str; 2] = ["openai", "anthropic"];
const VALID_CREDENTIAL_PROVIDERS: [&str; 2] = ["openai", "anthropic"];

/// Posted in a DM when the *sender themself* has no usable OpenAI/Anthropic credential.
const NO_KEY_MESSAGE_DM: &str =
    "I can't respond yet — you haven't added an OpenAI or Anthropic API key. Add one under \
     Profile → AI Assistant to chat with me.";

/// Posted in an org channel when the *org owner* (whose credential org-channel runs use)
/// has no usable OpenAI/Anthropic credential. The sender who @mentioned her is necessarily
/// the owner too (see `maybe_trigger_reply`), but the message is phrased for whoever reads
/// the channel, not just them.
const NO_KEY_MESSAGE_CHANNEL: &str =
    "I can't respond yet — the org owner hasn't added an OpenAI or Anthropic API key. Ask \
     them to add one under Profile → AI Assistant.";

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

/// Mention detection is a simple substring heuristic (`@lorelei`, case-insensitive) — same
/// pragmatic approach the existing notification-sound mention check uses
/// (`apps/shell-web/src/realtime/notificationSoundEffects.ts` in the flux repo). Not a real
/// parser; good enough for "does this message address her".
fn mentions_lorelei(body: &str) -> bool {
    body.to_lowercase().contains("@lorelei")
}

/// Two trigger paths:
/// - **PM**: `channel_id` is a DM channel that includes the global Lorelei user as a member
///   (reached via the normal friend/DM flow — no special-casing there). Any member can do
///   this. Runs use the *sender's own* resolved provider/credential, in the sender's personal
///   per-(user, org) memory scope.
/// - **Org channel**: any non-DM channel, if the message contains `@lorelei` *and* the sender
///   is the org's owner. Runs use the *owner's* (== sender's) resolved provider/credential,
///   in the org's shared memory scope.
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
            run_and_reply(&pool, &nats, &lorelei, org_id, channel_id, thread.tenant_id, thread.agent_id, sender_user_id, input, NO_KEY_MESSAGE_DM).await;
        });
        return;
    }

    if !mentions_lorelei(body) {
        return;
    }

    let role: Option<String> = sqlx::query_scalar(
        "select role from organization_members where organization_id = $1 and user_id = $2",
    )
    .bind(org_id)
    .bind(sender_user_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    if role.as_deref() != Some("owner") {
        return;
    }

    tokio::spawn(async move {
        let org = match lorelei_bridge::provision_org_lorelei(&pool, org_id).await {
            Ok(o) => o,
            Err(e) => {
                tracing::error!(error = %e, "failed to provision org lorelei tenant/agent");
                return;
            }
        };
        run_and_reply(&pool, &nats, &lorelei, org_id, channel_id, org.tenant_id, org.agent_id, sender_user_id, input, NO_KEY_MESSAGE_CHANNEL).await;
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
    no_key_message: &str,
) {
    // Bracket the whole resolution+run with a typing indicator, same as a real client's
    // typing.start/typing.stop — Lorelei has no WebSocket connection of her own, so this
    // publishes directly to the NATS subjects realtime-gateway already relays
    // (`channel.{id}.typing.started`/`.stopped`), bypassing the client-only WS protocol.
    emit_lorelei_typing(nats, channel_id, true).await;

    let text = match lorelei_bridge::resolve_provider(pool, &lorelei.credentials_key, credential_user_id).await {
        Ok(Some(provider)) => {
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

            // `output`/the denial text come from the run's last Assistant event in
            // lorelei-harbor — a run that fails before producing any text (e.g. the
            // provider was unreachable) has no such event, so harbor reports an empty
            // string rather than an error. Without this guard that empty string gets
            // posted verbatim, which looks indistinguishable from Lorelei never
            // responding at all.
            match outcome {
                Ok(lorelei_bridge::RunOutcome::Succeeded(text)) if !text.trim().is_empty() => text,
                Ok(lorelei_bridge::RunOutcome::Denied(text)) if !text.trim().is_empty() => text,
                Ok(lorelei_bridge::RunOutcome::Succeeded(_) | lorelei_bridge::RunOutcome::Denied(_)) => {
                    tracing::error!(%org_id, %channel_id, "lorelei run finished with no output text");
                    "Lorelei couldn't respond — try again.".to_string()
                }
                Ok(lorelei_bridge::RunOutcome::TimedOut) => {
                    "Lorelei is taking longer than expected — try again in a moment.".to_string()
                }
                Err(e) => {
                    tracing::error!(error = %e, "lorelei run failed");
                    "Lorelei couldn't respond — try again.".to_string()
                }
            }
        }
        Ok(None) => no_key_message.to_string(),
        Err(e) => {
            tracing::error!(error = %e, "failed to resolve lorelei provider");
            "Lorelei couldn't respond — try again.".to_string()
        }
    };

    emit_lorelei_typing(nats, channel_id, false).await;
    post_bot_message(pool, nats, org_id, channel_id, &text).await;
}

/// Mirrors exactly what realtime-gateway's own `ClientEvent::TypingStart`/`TypingStop`
/// handlers publish (see `apps/realtime-gateway/src/runtime.rs`) — same subject, same raw
/// `ServerEvent`-shaped JSON (no `EventEnvelope` wrapper). The gateway's NATS fanout loop
/// already subscribes to `channel.*.typing.started`/`.stopped` and re-broadcasts whatever
/// it receives, so publishing here is all that's needed; no gateway changes required.
async fn emit_lorelei_typing(nats: &async_nats::Client, channel_id: Uuid, started: bool) {
    let kind = if started { "typing.started" } else { "typing.stopped" };
    let payload = serde_json::json!({ "type": kind, "channel_id": channel_id, "user_id": LORELEI_USER_ID });
    let Ok(bytes) = serde_json::to_vec(&payload) else { return };
    let subject = format!("channel.{channel_id}.{kind}");
    if let Err(err) = nats.publish(subject, bytes.into()).await {
        tracing::error!(error = %err, %channel_id, "failed to publish lorelei typing event");
    }
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
