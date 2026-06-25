//! The only thing in flux-server that speaks HTTP to Lorelei's Harbor API. Nothing else
//! should construct a request to it directly — see `LORELEI_BUILDPLAN.md` Section 5.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::error::BridgeError;
use crate::resolve::ResolvedProvider;

/// Caps which shells a run may use, mirroring Lorelei's `ShellRisk` (`lorelei-core/src/types.rs`,
/// no `#[serde(rename_all)]`, so the wire form is the literal Rust variant name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MaxRisk {
    None,
    Low,
    Medium,
    High,
    Critical,
}

struct ProviderOverrideWire {
    kind: String,
    model: String,
    api_key: String,
    base_url: Option<String>,
}

impl Serialize for ProviderOverrideWire {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ProviderOverrideWire", 4)?;
        s.serialize_field("kind", &self.kind)?;
        s.serialize_field("model", &self.model)?;
        s.serialize_field("api_key", &self.api_key)?;
        s.serialize_field("base_url", &self.base_url)?;
        s.end()
    }
}

impl std::fmt::Debug for ProviderOverrideWire {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderOverrideWire")
            .field("kind", &self.kind)
            .field("model", &self.model)
            .field("api_key", &"<redacted>")
            .field("base_url", &self.base_url)
            .finish()
    }
}

#[derive(Serialize)]
struct CreateRunRequest {
    tenant_id: Uuid,
    agent_id: Uuid,
    input: String,
    #[serde(rename = "async")]
    async_run: bool,
    no_memory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_override: Option<ProviderOverrideWire>,
    max_risk: MaxRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum RunStatusWire {
    Pending,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Deserialize)]
struct RunResponseWire {
    run_id: Uuid,
    status: RunStatusWire,
    #[serde(default)]
    output: Option<String>,
}

/// The result of a completed (or timed-out) run.
#[derive(Debug)]
pub enum RunOutcome {
    /// The run answered normally.
    Succeeded(String),
    /// The run ended in `Failed` or `Canceled` (Siren denial, max_risk exceedance, approval
    /// pending, or an internal error) — `String` is whatever explanatory text Lorelei wrote
    /// to the Assistant event, suitable for posting back as-is.
    Denied(String),
    /// Didn't reach a terminal state within the caller's timeout.
    TimedOut,
}

pub struct HarborClient {
    base_url: String,
    http: reqwest::Client,
}

impl HarborClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    async fn create_run(
        &self,
        tenant_id: Uuid,
        agent_id: Uuid,
        input: String,
        provider_override: Option<ProviderOverrideWire>,
        max_risk: MaxRisk,
    ) -> Result<Uuid, BridgeError> {
        let body = CreateRunRequest {
            tenant_id,
            agent_id,
            input,
            async_run: true,
            no_memory: false,
            provider_override,
            max_risk,
        };
        let resp = self
            .http
            .post(format!("{}/v1/runs", self.base_url))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(BridgeError::Harbor(text));
        }
        Ok(resp.json::<RunResponseWire>().await?.run_id)
    }

    async fn get_run(
        &self,
        tenant_id: Uuid,
        agent_id: Uuid,
        run_id: Uuid,
    ) -> Result<RunResponseWire, BridgeError> {
        let url = format!(
            "{}/v1/runs/{run_id}?tenant_id={tenant_id}&agent_id={agent_id}",
            self.base_url
        );
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(BridgeError::Harbor(text));
        }
        Ok(resp.json().await?)
    }

    /// Submits a run and polls `GET /v1/runs/:id` with exponential backoff (capped at 3s)
    /// until it reaches a terminal state or `timeout` elapses. Callers must already have a
    /// `ResolvedProvider` (i.e. `resolve_provider` returned `Some` — true by default now,
    /// since an unset preference resolves to the self-hosted Ollama instance) before calling
    /// this; it always sends a `provider_override`.
    pub async fn run_and_wait(
        &self,
        tenant_id: Uuid,
        agent_id: Uuid,
        input: String,
        provider: ResolvedProvider,
        max_risk: MaxRisk,
        timeout: Duration,
    ) -> Result<RunOutcome, BridgeError> {
        let provider_override = Some(ProviderOverrideWire {
            kind: provider.kind,
            model: provider.model,
            api_key: provider.api_key,
            base_url: provider.base_url,
        });

        let run_id = self
            .create_run(tenant_id, agent_id, input, provider_override, max_risk)
            .await?;

        let deadline = tokio::time::Instant::now() + timeout;
        let mut delay = Duration::from_millis(300);

        loop {
            let run = self.get_run(tenant_id, agent_id, run_id).await?;
            match run.status {
                RunStatusWire::Succeeded => {
                    return Ok(RunOutcome::Succeeded(run.output.unwrap_or_default()));
                }
                RunStatusWire::Failed | RunStatusWire::Canceled => {
                    return Ok(RunOutcome::Denied(run.output.unwrap_or_default()));
                }
                RunStatusWire::Pending | RunStatusWire::Running => {
                    let now = tokio::time::Instant::now();
                    if now >= deadline {
                        return Ok(RunOutcome::TimedOut);
                    }
                    tokio::time::sleep(delay.min(deadline - now)).await;
                    delay = (delay * 2).min(Duration::from_secs(3));
                }
            }
        }
    }
}
