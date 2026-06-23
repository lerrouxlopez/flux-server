//! Resolves which org a Lorelei conversation belongs to, and which LLM provider/model/
//! credential a given member's message should run with.
//!
//! Resolution chain (mirrors the existing notification-settings pattern in this codebase):
//! user override (requires a stored credential) -> org default -> platform default (Ollama).

use secrets::CredentialsKey;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::BridgeError;

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
struct OrgLoreleiRow {
    enabled: bool,
    display_name: String,
    default_provider: String,
    default_model: String,
    bot_user_id: Option<Uuid>,
    lorelei_tenant_id: Option<Uuid>,
    lorelei_agent_id: Option<Uuid>,
    default_channel_id: Option<Uuid>,
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

/// An org's resolved, fully-provisioned Lorelei settings. Construction fails (with
/// `BridgeError::NotEnabled`) if the org hasn't enabled Lorelei or its bot user / tenant /
/// agent / default channel haven't been provisioned yet (see `LORELEI_BUILDPLAN.md` Section 7
/// — provisioning happens in the admin-settings enable endpoint, not here).
pub struct OrgLorelei {
    pub display_name: String,
    pub default_provider: String,
    pub default_model: String,
    pub bot_user_id: Uuid,
    pub tenant_id: Uuid,
    pub agent_id: Uuid,
    pub default_channel_id: Uuid,
}

pub async fn load_org_lorelei(pool: &PgPool, org_id: Uuid) -> Result<OrgLorelei, BridgeError> {
    let row = sqlx::query_as::<_, OrgLoreleiRow>(
        r#"
        select enabled, display_name, default_provider, default_model,
               bot_user_id, lorelei_tenant_id, lorelei_agent_id, default_channel_id
        from org_lorelei_settings
        where organization_id = $1
        "#,
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Err(BridgeError::NotEnabled);
    };

    let (Some(bot_user_id), Some(tenant_id), Some(agent_id), Some(default_channel_id)) = (
        row.bot_user_id,
        row.lorelei_tenant_id,
        row.lorelei_agent_id,
        row.default_channel_id,
    ) else {
        return Err(BridgeError::NotEnabled);
    };

    if !row.enabled {
        return Err(BridgeError::NotEnabled);
    }

    Ok(OrgLorelei {
        display_name: row.display_name,
        default_provider: row.default_provider,
        default_model: row.default_model,
        bot_user_id,
        tenant_id,
        agent_id,
        default_channel_id,
    })
}

/// Maps a FLUX-stored provider name to Lorelei's wire `ProviderKind`. `None` means "use the
/// platform default" — covers `"ollama"` and any value that isn't a BYO-key provider.
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

/// Resolves the effective provider for `user_id` acting within `org`.
pub async fn resolve_provider(
    pool: &PgPool,
    key: &CredentialsKey,
    org: &OrgLorelei,
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

    let effective_provider = preferred_provider.unwrap_or_else(|| org.default_provider.clone());

    let Some(wire_kind) = to_wire_kind(&effective_provider) else {
        return Ok(ProviderResolution::PlatformDefault);
    };

    let cred = sqlx::query_as::<_, CredentialRow>(
        "select encrypted_api_key from user_llm_credentials where user_id = $1 and provider = $2",
    )
    .bind(user_id)
    .bind(&effective_provider)
    .fetch_optional(pool)
    .await?;

    match cred {
        Some(row) => {
            let api_key = secrets::decrypt(&row.encrypted_api_key, key)?;
            let model = preferred_model.unwrap_or_else(|| default_model_for(&effective_provider).to_string());
            Ok(ProviderResolution::Override {
                kind: wire_kind.to_string(),
                model,
                api_key,
            })
        }
        // No credential for the resolved provider — fall back to the org default, unless
        // the org default *is* that same paid provider (then there's nothing left to fall
        // back to, and the caller should surface a clear "no API key configured" error).
        None if to_wire_kind(&org.default_provider).is_none() => Ok(ProviderResolution::PlatformDefault),
        None => Err(BridgeError::NoCredentialAvailable),
    }
}
