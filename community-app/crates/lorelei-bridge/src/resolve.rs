//! Lorelei is a single global platform user (see `LORELEI_USER_ID`) rather than one bot per
//! org. There are two separate memory/credential scopes:
//!
//! - **Personal PM** (`UserThread`): one Lorelei tenant/agent + DM channel per (user, org)
//!   pair, created lazily on a user's first PM to her in that org. Runs use *that user's
//!   own* resolved provider/credential.
//! - **Org channel** (`OrgLorelei`): one Lorelei tenant/agent per org, provisioned lazily the
//!   first time an admin/owner adds her to a channel. Runs use the *org owner's* resolved
//!   provider/credential — there is no separate "org default" setting anymore; the owner's
//!   own profile preference/credential effectively *is* the org's default.
//!
//! Both scopes share the same `resolve_provider`, parameterized only by which user's
//! credential to resolve — the caller decides whether that's the sender (PM) or the owner
//! (org channel).

use secrets::CredentialsKey;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::BridgeError;

/// Fixed, well-known user ID for the single global Lorelei system user — seeded once via
/// `crates/db/migrations/202606230004_lorelei_global_user.sql`. Mirrored in the flux frontend
/// repo as a TS constant (`LORELEI_USER_ID` in `packages/api`).
pub const LORELEI_USER_ID: Uuid = Uuid::from_bytes([
    0x63, 0xdc, 0xae, 0x57, 0xb2, 0xf5, 0x47, 0x25, 0xa1, 0x61, 0xc1, 0x35, 0x99, 0x11, 0x3a, 0x80,
]);

/// What to send Lorelei for a given run's provider.
#[derive(Debug, Clone)]
pub enum ProviderResolution {
    /// Use lorelei-harbor's statically configured default provider (Ollama). No
    /// `provider_override` is sent — the credential never leaves Lorelei's own config.
    PlatformDefault,
    /// A caller-supplied, request-scoped provider/model/credential.
    Override {
        /// Lorelei's wire `ProviderKind` value (kebab-case: "openai-compatible" | "anthropic").
        kind: String,
        model: String,
        api_key: String,
    },
}

#[derive(sqlx::FromRow)]
struct UserPreferenceRow {
    preferred_provider: Option<String>,
    preferred_model: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CredentialRow {
    encrypted_api_key: Vec<u8>,
}

fn to_wire_kind(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("openai-compatible"),
        "anthropic" => Some("anthropic"),
        _ => None,
    }
}

fn default_model_for(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-4o-mini",
        "anthropic" => "claude-3-5-sonnet-latest",
        _ => "llama3.2:3b",
    }
}

/// Resolves the effective provider for `user_id`. Used identically for a PM sender or an
/// org-channel's owner — the resolution chain doesn't care which role the user plays.
pub async fn resolve_provider(
    pool: &PgPool,
    key: &CredentialsKey,
    user_id: Uuid,
) -> Result<ProviderResolution, BridgeError> {
    let pref = sqlx::query_as::<_, UserPreferenceRow>(
        "select preferred_provider, preferred_model from user_lorelei_preferences where user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    let (preferred_provider, preferred_model) = match pref {
        Some(row) => (row.preferred_provider, row.preferred_model),
        None => (None, None),
    };

    let Some(provider) = preferred_provider else {
        return Ok(ProviderResolution::PlatformDefault);
    };
    let Some(wire_kind) = to_wire_kind(&provider) else {
        return Ok(ProviderResolution::PlatformDefault);
    };

    let cred = sqlx::query_as::<_, CredentialRow>(
        "select encrypted_api_key from user_llm_credentials where user_id = $1 and provider = $2",
    )
    .bind(user_id)
    .bind(&provider)
    .fetch_optional(pool)
    .await?;

    match cred {
        Some(row) => {
            let api_key = secrets::decrypt(&row.encrypted_api_key, key)?;
            let model = preferred_model.unwrap_or_else(|| default_model_for(&provider).to_string());
            Ok(ProviderResolution::Override {
                kind: wire_kind.to_string(),
                model,
                api_key,
            })
        }
        // Wanted a paid provider but never saved a key for it — fall back rather than fail;
        // the caller still gets a real reply, just from the free default.
        None => Ok(ProviderResolution::PlatformDefault),
    }
}

// ---------------------------------------------------------------------------
// Org-channel scope
// ---------------------------------------------------------------------------

pub struct OrgLorelei {
    pub tenant_id: Uuid,
    pub agent_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct OrgLoreleiRow {
    lorelei_tenant_id: Uuid,
    lorelei_agent_id: Uuid,
}

/// Reads an org's Lorelei tenant/agent. Errors `NotEnabled` if she's never been added to a
/// channel in this org — provisioning happens in `provision_org_lorelei`, called from
/// `routes_lorelei.rs::add_channel`, not here.
pub async fn load_org_lorelei(pool: &PgPool, org_id: Uuid) -> Result<OrgLorelei, BridgeError> {
    let row = sqlx::query_as::<_, OrgLoreleiRow>(
        "select lorelei_tenant_id, lorelei_agent_id from org_lorelei_settings where organization_id = $1",
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Err(BridgeError::NotEnabled);
    };
    Ok(OrgLorelei {
        tenant_id: row.lorelei_tenant_id,
        agent_id: row.lorelei_agent_id,
    })
}

/// Idempotently provisions an org's Lorelei tenant/agent if they don't exist yet. Tenant/
/// agent IDs are opaque UUIDs Lorelei never needs pre-registered (confirmed in
/// LORELEI_BUILDPLAN.md Section 2.3), so minting them here is safe.
pub async fn provision_org_lorelei(pool: &PgPool, org_id: Uuid) -> Result<OrgLorelei, BridgeError> {
    sqlx::query(
        r#"
        insert into org_lorelei_settings (organization_id, lorelei_tenant_id, lorelei_agent_id, created_at, updated_at)
        values ($1, $2, $3, now(), now())
        on conflict (organization_id) do nothing
        "#,
    )
    .bind(org_id)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .execute(pool)
    .await?;

    load_org_lorelei(pool, org_id).await
}

// ---------------------------------------------------------------------------
// Personal PM scope
// ---------------------------------------------------------------------------

pub struct UserThread {
    pub tenant_id: Uuid,
    pub agent_id: Uuid,
    pub dm_channel_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct UserThreadRow {
    lorelei_tenant_id: Uuid,
    lorelei_agent_id: Uuid,
    dm_channel_id: Uuid,
}

/// Idempotently returns (creating on first call) a user's personal Lorelei thread within
/// `org_id`: a DM channel (reusing the same `channels`/`dm_channel_members` tables every
/// other DM uses — no new messaging infrastructure) plus a dedicated tenant/agent pair.
///
/// Lorelei is reachable through the normal friend/DM flow (`routes_dms.rs::create_or_get_dm`)
/// like any other member, so a DM channel between `user_id` and her may already exist by the
/// time this is first called — checked before creating a new one, so the two entry points
/// (starting a DM the normal way vs. this being called from the message-trigger hook) always
/// converge on the same channel instead of creating a duplicate.
///
/// Not fully race-safe: two concurrent first-PMs from the same user could each create a DM
/// channel before the `on conflict` resolves, leaving one orphaned (unused, harmless) extra
/// channel behind. Acceptable for v1 — the loser's channel is simply never linked from
/// `user_lorelei_threads` and nothing references it again.
pub async fn load_or_create_user_thread(
    pool: &PgPool,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<UserThread, BridgeError> {
    if let Some(row) = sqlx::query_as::<_, UserThreadRow>(
        "select lorelei_tenant_id, lorelei_agent_id, dm_channel_id from user_lorelei_threads \
         where user_id = $1 and organization_id = $2",
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_optional(pool)
    .await?
    {
        return Ok(UserThread {
            tenant_id: row.lorelei_tenant_id,
            agent_id: row.lorelei_agent_id,
            dm_channel_id: row.dm_channel_id,
        });
    }

    let existing_channel_id: Option<Uuid> = sqlx::query_scalar(
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
    .bind(user_id)
    .bind(LORELEI_USER_ID)
    .fetch_optional(pool)
    .await?;

    let mut tx = pool.begin().await?;

    let channel_id = match existing_channel_id {
        Some(id) => id,
        None => {
            let channel_id = Uuid::now_v7();
            sqlx::query("insert into channels (id, organization_id, name, kind, created_at) values ($1, $2, '', 'dm', now())")
                .bind(channel_id)
                .bind(org_id)
                .execute(&mut *tx)
                .await?;

            sqlx::query(
                "insert into dm_channel_members (channel_id, user_id, added_at) values ($1, $2, now()), ($1, $3, now())",
            )
            .bind(channel_id)
            .bind(user_id)
            .bind(LORELEI_USER_ID)
            .execute(&mut *tx)
            .await?;

            channel_id
        }
    };

    sqlx::query(
        r#"
        insert into user_lorelei_threads (user_id, organization_id, lorelei_tenant_id, lorelei_agent_id, dm_channel_id, created_at)
        values ($1, $2, $3, $4, $5, now())
        on conflict (user_id, organization_id) do nothing
        "#,
    )
    .bind(user_id)
    .bind(org_id)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind(channel_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Re-select rather than trust our own insert — if we lost a race, this returns the
    // winning row (whose dm_channel_id may differ from the one we just created above).
    let row = sqlx::query_as::<_, UserThreadRow>(
        "select lorelei_tenant_id, lorelei_agent_id, dm_channel_id from user_lorelei_threads \
         where user_id = $1 and organization_id = $2",
    )
    .bind(user_id)
    .bind(org_id)
    .fetch_one(pool)
    .await?;

    Ok(UserThread {
        tenant_id: row.lorelei_tenant_id,
        agent_id: row.lorelei_agent_id,
        dm_channel_id: row.dm_channel_id,
    })
}
