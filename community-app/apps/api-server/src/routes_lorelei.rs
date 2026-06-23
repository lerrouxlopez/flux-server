//! Lorelei admin settings, per-user BYO LLM credentials/preferences, and the hook that
//! triggers a reply when a member messages a Lorelei-enabled channel.
//!
//! See `LORELEI_BUILDPLAN.md` (flux frontend repo) for the full design. This file is the
//! only place in `api-server` that touches `org_lorelei_settings`, `org_lorelei_channels`,
//! `user_llm_credentials`, or `user_lorelei_preferences`.

use crate::{util, AppState, AuthContext};
use api::ApiErrorCode;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
    Router,
};
use events::envelope::EventEnvelope;
use permissions::Permission;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use tracing::Span;
use uuid::Uuid;

const VALID_PROVIDERS: [&str; 3] = ["ollama", "openai", "anthropic"];
const VALID_CREDENTIAL_PROVIDERS: [&str; 2] = ["openai", "anthropic"];
const DEFAULT_DISPLAY_NAME: &str = "Lorelei";
const DEFAULT_PROVIDER: &str = "ollama";
const DEFAULT_MODEL: &str = "llama3.2:3b";

/// Mounted under `/orgs` in `app.rs` (merged alongside `routes_orgs::router()`).
pub fn org_router() -> Router<AppState> {
    Router::new()
        .route(
            "/{org_id}/lorelei/settings",
            get(get_settings).patch(update_settings),
        )
        .route(
            "/{org_id}/lorelei/channels",
            get(list_channels).post(add_channel),
        )
        .route(
            "/{org_id}/lorelei/channels/{channel_id}",
            delete(remove_channel),
        )
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
// Org settings
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OrgLoreleiSettingsResponse {
    enabled: bool,
    display_name: String,
    default_provider: String,
    default_model: String,
}

async fn get_settings(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
) -> impl IntoResponse {
    match util::can(&state.pool, auth.user_id, org_id, Permission::LoreleiManage).await {
        Ok(true) => {}
        Ok(false) => return util::api_error(ApiErrorCode::PermissionDenied),
        Err(e) => return e,
    }

    let row = sqlx::query(
        "select enabled, display_name, default_provider, default_model \
         from org_lorelei_settings where organization_id = $1",
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;

    let row = match row {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let resp = match row {
        Some(r) => OrgLoreleiSettingsResponse {
            enabled: r.get("enabled"),
            display_name: r.get("display_name"),
            default_provider: r.get("default_provider"),
            default_model: r.get("default_model"),
        },
        None => OrgLoreleiSettingsResponse {
            enabled: false,
            display_name: DEFAULT_DISPLAY_NAME.to_string(),
            default_provider: DEFAULT_PROVIDER.to_string(),
            default_model: DEFAULT_MODEL.to_string(),
        },
    };

    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Debug, Deserialize)]
struct UpdateSettingsRequest {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    default_provider: Option<String>,
    #[serde(default)]
    default_model: Option<String>,
}

async fn update_settings(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(org_id): Path<Uuid>,
    Json(req): Json<UpdateSettingsRequest>,
) -> impl IntoResponse {
    Span::current().record("organization_id", tracing::field::display(org_id));

    match util::can(&state.pool, auth.user_id, org_id, Permission::LoreleiManage).await {
        Ok(true) => {}
        Ok(false) => return util::api_error(ApiErrorCode::PermissionDenied),
        Err(e) => return e,
    }

    if let Some(p) = &req.default_provider {
        if !VALID_PROVIDERS.contains(&p.as_str()) {
            return util::api_error(ApiErrorCode::ValidationError);
        }
    }
    if let Some(n) = &req.display_name {
        if n.trim().is_empty() || n.len() > 100 {
            return util::api_error(ApiErrorCode::ValidationError);
        }
    }

    let existing = sqlx::query(
        "select enabled, display_name, default_provider, default_model, \
                bot_user_id, lorelei_tenant_id, lorelei_agent_id, default_channel_id \
         from org_lorelei_settings where organization_id = $1",
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await;
    let existing = match existing {
        Ok(v) => v,
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };

    let was_enabled = existing.as_ref().map(|r| r.get::<bool, _>("enabled")).unwrap_or(false);
    let already_provisioned = existing
        .as_ref()
        .map(|r| r.get::<Option<Uuid>, _>("bot_user_id").is_some())
        .unwrap_or(false);

    let display_name = req.display_name.clone().unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|r| r.get::<String, _>("display_name"))
            .unwrap_or_else(|| DEFAULT_DISPLAY_NAME.to_string())
    });
    let default_provider = req.default_provider.clone().unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|r| r.get::<String, _>("default_provider"))
            .unwrap_or_else(|| DEFAULT_PROVIDER.to_string())
    });
    let default_model = req.default_model.clone().unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|r| r.get::<String, _>("default_model"))
            .unwrap_or_else(|| DEFAULT_MODEL.to_string())
    });
    let enabled = req.enabled.unwrap_or(was_enabled);

    let (bot_user_id, tenant_id, agent_id, default_channel_id): (
        Option<Uuid>,
        Option<Uuid>,
        Option<Uuid>,
        Option<Uuid>,
    ) = if enabled && !already_provisioned {
        match provision_org(&state.pool, org_id, &display_name).await {
            Ok(v) => (Some(v.0), Some(v.1), Some(v.2), Some(v.3)),
            Err(e) => return e,
        }
    } else {
        (
            existing.as_ref().and_then(|r| r.get("bot_user_id")),
            existing.as_ref().and_then(|r| r.get("lorelei_tenant_id")),
            existing.as_ref().and_then(|r| r.get("lorelei_agent_id")),
            existing.as_ref().and_then(|r| r.get("default_channel_id")),
        )
    };

    let res = sqlx::query(
        r#"
        insert into org_lorelei_settings
            (organization_id, enabled, display_name, default_provider, default_model,
             bot_user_id, lorelei_tenant_id, lorelei_agent_id, default_channel_id, updated_at)
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
        on conflict (organization_id) do update set
            enabled = excluded.enabled,
            display_name = excluded.display_name,
            default_provider = excluded.default_provider,
            default_model = excluded.default_model,
            bot_user_id = coalesce(org_lorelei_settings.bot_user_id, excluded.bot_user_id),
            lorelei_tenant_id = coalesce(org_lorelei_settings.lorelei_tenant_id, excluded.lorelei_tenant_id),
            lorelei_agent_id = coalesce(org_lorelei_settings.lorelei_agent_id, excluded.lorelei_agent_id),
            default_channel_id = coalesce(org_lorelei_settings.default_channel_id, excluded.default_channel_id),
            updated_at = now()
        "#,
    )
    .bind(org_id)
    .bind(enabled)
    .bind(&display_name)
    .bind(&default_provider)
    .bind(&default_model)
    .bind(bot_user_id)
    .bind(tenant_id)
    .bind(agent_id)
    .bind(default_channel_id)
    .execute(&state.pool)
    .await;

    if res.is_err() {
        return util::api_error(ApiErrorCode::InternalError);
    }

    // Renaming Lorelei is just a normal display_name update on its bot user row.
    if let Some(bot_user_id) = bot_user_id {
        let _ = sqlx::query("update users set display_name = $1 where id = $2")
            .bind(&display_name)
            .bind(bot_user_id)
            .execute(&state.pool)
            .await;
    }

    util::write_audit_log(
        &state.pool,
        org_id,
        Some(auth.user_id),
        "lorelei.settings_updated",
        None,
        None,
        serde_json::json!({ "enabled": enabled, "default_provider": default_provider }),
    )
    .await;

    (
        StatusCode::OK,
        Json(OrgLoreleiSettingsResponse {
            enabled,
            display_name,
            default_provider,
            default_model,
        }),
    )
        .into_response()
}

/// First-enable side effects: bot user, org membership, and the dedicated channel. Tenant/
/// agent IDs are minted client-side (Lorelei's `TenantId`/`AgentId` are opaque UUIDs with no
/// registration table — see `LORELEI_BUILDPLAN.md` Section 2.3) and stored alongside.
async fn provision_org(
    pool: &PgPool,
    org_id: Uuid,
    display_name: &str,
) -> Result<(Uuid, Uuid, Uuid, Uuid), axum::response::Response> {
    let bot_user_id = Uuid::now_v7();
    let tenant_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();
    let channel_id = Uuid::now_v7();
    let now = OffsetDateTime::now_utc();
    let bot_email = format!("lorelei+{org_id}@system.flux.internal");

    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return Err(util::api_error(ApiErrorCode::InternalError)),
    };

    let user_inserted = sqlx::query(
        "insert into users (id, email, display_name, password_hash, is_system_bot) \
         values ($1, $2, $3, null, true)",
    )
    .bind(bot_user_id)
    .bind(&bot_email)
    .bind(display_name)
    .execute(&mut *tx)
    .await;
    if user_inserted.is_err() {
        return Err(util::api_error(ApiErrorCode::InternalError));
    }

    let member_inserted = sqlx::query(
        "insert into organization_members (organization_id, user_id, role, joined_at) \
         values ($1, $2, 'member', $3)",
    )
    .bind(org_id)
    .bind(bot_user_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if member_inserted.is_err() {
        return Err(util::api_error(ApiErrorCode::InternalError));
    }

    let channel_inserted = sqlx::query(
        "insert into channels (id, organization_id, name, kind, experience_mode_hint, created_at) \
         values ($1, $2, 'lorelei', 'lorelei', 'work', $3)",
    )
    .bind(channel_id)
    .bind(org_id)
    .bind(now)
    .execute(&mut *tx)
    .await;
    if channel_inserted.is_err() {
        return Err(util::api_error(ApiErrorCode::InternalError));
    }

    if tx.commit().await.is_err() {
        return Err(util::api_error(ApiErrorCode::InternalError));
    }

    Ok((bot_user_id, tenant_id, agent_id, channel_id))
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

    let channel_org: Option<Uuid> = match sqlx::query_scalar(
        "select organization_id from channels where id = $1",
    )
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

    let default_channel_id: Option<Uuid> = match sqlx::query_scalar(
        "select default_channel_id from org_lorelei_settings where organization_id = $1",
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(v) => v.flatten(),
        Err(_) => return util::api_error(ApiErrorCode::InternalError),
    };
    if default_channel_id == Some(req.channel_id) {
        // Already always available — adding it to the extension table is meaningless.
        return util::api_error(ApiErrorCode::ValidationError);
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
/// `null`/omitted both mean "inherit the org default". A real sparse PATCH (distinguishing
/// "don't touch" from "clear") needs a double-`Option` deserializer; not worth it until the
/// frontend (Phase L4) actually needs that distinction.
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

/// Checks whether `channel_id` is Lorelei-enabled for `org_id` and, if so, kicks off a
/// background run. Cheap to call on every message send — the eligibility check is 1-2
/// indexed point lookups; the actual LLM call happens in a spawned task so it never delays
/// the sender's own response.
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

    let org = match lorelei_bridge::load_org_lorelei(&state.pool, org_id).await {
        Ok(o) => o,
        Err(_) => return,
    };

    if sender_user_id == org.bot_user_id {
        return; // defensive: never let the bot reply to itself
    }

    let is_default_channel = channel_id == org.default_channel_id;
    if !is_default_channel {
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
            return; // not a Lorelei-enabled channel
        }
        match util::can(
            &state.pool,
            sender_user_id,
            org_id,
            Permission::LoreleiInvokeChannel,
        )
        .await
        {
            Ok(true) => {}
            _ => return,
        }
    }

    let pool = state.pool.clone();
    let nats = state.nats.clone();
    let input = body.to_string();

    tokio::spawn(async move {
        let provider =
            match lorelei_bridge::resolve_provider(&pool, &lorelei.credentials_key, &org, sender_user_id)
                .await
            {
                Ok(p) => p,
                Err(_) => {
                    post_bot_message(
                        &pool,
                        &nats,
                        org_id,
                        channel_id,
                        org.bot_user_id,
                        "Lorelei couldn't respond — no API key is configured for the selected model.",
                    )
                    .await;
                    return;
                }
            };

        let outcome = lorelei
            .harbor
            .run_and_wait(
                org.tenant_id,
                org.agent_id,
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

        post_bot_message(&pool, &nats, org_id, channel_id, org.bot_user_id, &text).await;
    });
}

async fn post_bot_message(
    pool: &PgPool,
    nats: &async_nats::Client,
    org_id: Uuid,
    channel_id: Uuid,
    bot_user_id: Uuid,
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
    .bind(bot_user_id)
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
        Some(bot_user_id),
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
