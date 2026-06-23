//! The single chokepoint between flux-server and Lorelei. Nothing else in this workspace
//! should call Lorelei's Harbor HTTP API directly, decrypt a stored LLM credential, or
//! resolve a FLUX org/user to a Lorelei tenant/agent — that all lives here.
//!
//! See `LORELEI_BUILDPLAN.md` (flux frontend repo) for the full design.

pub mod client;
pub mod error;
pub mod resolve;

pub use client::{HarborClient, MaxRisk, RunOutcome};
pub use error::BridgeError;
pub use resolve::{load_org_lorelei, resolve_provider, OrgLorelei, ProviderResolution};
