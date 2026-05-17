//! MT-018 `SandboxAdapter` trait.
//!
//! Acceptance (MT-018.json): "define adapter boundary independent of Docker,
//! WSL, Deno, or WASM. At least one adapter can be implemented without
//! changing caller code." The trait deliberately does not name an isolation
//! technology in its signatures; all adapter-tier specifics
//! (process / hard-isolation container / hard-isolation microVM / wasm) plug
//! in through `AdapterKind`.
//!
//! Downstream batches (Wave C: hard isolation, Wave E: artifacts + promotion)
//! must reuse this trait. New adapter implementations add a new `AdapterKind`
//! variant and implement the same trait surface — no caller-side change.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::denial::SandboxDenialRecordV1;
use super::policy::{SandboxCapability, SandboxPolicyV1};
use super::run::{SandboxRunStatus, SandboxRunV1};
use super::workspace::SandboxWorkspaceV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterIsolationTier {
    /// Native Rust child process under capped permissions (Loom-style).
    Process,
    /// Container or microVM adapter; opt-in via Wave C MTs.
    HardIsolation,
    /// In-process wasm sandbox; reserved extension slot.
    Wasm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterKind {
    pub id: String,
    pub tier: AdapterIsolationTier,
    pub version: u32,
    pub label: String,
}

impl AdapterKind {
    pub fn process_tier(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tier: AdapterIsolationTier::Process,
            version: 1,
            label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterRunOutcome {
    Started,
    Completed { artifact_refs: Vec<String> },
    Denied(SandboxDenialRecordV1),
}

impl AdapterRunOutcome {
    pub fn to_status(&self) -> SandboxRunStatus {
        match self {
            Self::Started => SandboxRunStatus::Started,
            Self::Completed { .. } => SandboxRunStatus::Completed,
            Self::Denied(_) => SandboxRunStatus::Rejected,
        }
    }
}

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("adapter unavailable: {0}")]
    Unavailable(String),
    #[error("workspace boundary violation: {0}")]
    WorkspaceViolation(String),
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("internal adapter error: {0}")]
    Internal(String),
}

/// Trait every sandbox adapter implements. Method set is intentionally tight:
/// a single `prepare`+`run` pair plus capability and identity surface. New
/// isolation tiers slot in by adding an `AdapterIsolationTier` variant and a
/// trait impl; callers stay the same.
pub trait SandboxAdapter: Send + Sync {
    fn kind(&self) -> AdapterKind;

    /// Verify the policy permits the requested capability set against the
    /// active policy. Default impl iterates `requested` and returns the first
    /// denial; adapters MAY override for batched checks.
    fn pre_check(
        &self,
        run: &SandboxRunV1,
        policy: &SandboxPolicyV1,
        requested: &[SandboxCapability],
    ) -> Result<(), SandboxDenialRecordV1> {
        use super::denial::DenialKind;
        for cap in requested {
            match policy.decide(*cap) {
                super::policy::CapabilityDecision::Deny => {
                    return Err(SandboxDenialRecordV1::new(
                        run.run_id.0.clone(),
                        policy.version_id(),
                        DenialKind::PolicyDenied,
                        Some(*cap),
                        format!("adapter `{}` requested {}", self.kind().id, cap.as_str()),
                        format!(
                            "policy `{}` default {:?} denies {}",
                            policy.version_id(),
                            policy.default_decision,
                            cap.as_str()
                        ),
                    ));
                }
                _ => continue,
            }
        }
        Ok(())
    }

    /// Run the sandbox against an already-prepared workspace.
    fn run(
        &self,
        run: &SandboxRunV1,
        workspace: &SandboxWorkspaceV1,
        policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::sandbox::denial::DenialKind;

    struct NullAdapter;
    impl SandboxAdapter for NullAdapter {
        fn kind(&self) -> AdapterKind {
            AdapterKind::process_tier("null", "Null process-tier adapter")
        }
        fn run(
            &self,
            _run: &SandboxRunV1,
            _workspace: &SandboxWorkspaceV1,
            _policy: &SandboxPolicyV1,
        ) -> Result<AdapterRunOutcome, AdapterError> {
            Ok(AdapterRunOutcome::Completed {
                artifact_refs: vec![],
            })
        }
    }

    fn fixture_run() -> SandboxRunV1 {
        SandboxRunV1::new_requested("KTR-1", "SES-1", "null", "POL-1@1", "WSP-1")
    }

    #[test]
    fn adapter_can_be_swapped_without_changing_caller_code() {
        // Caller code only sees `dyn SandboxAdapter` — no Docker/WSL/Deno/Wasm name.
        fn invoke(adapter: &dyn SandboxAdapter) -> SandboxRunStatus {
            let run = fixture_run();
            let workspace = SandboxWorkspaceV1::new_default("k", "handshake-product/kb003/w");
            let policy = SandboxPolicyV1::default_deny("baseline");
            adapter.run(&run, &workspace, &policy).unwrap().to_status()
        }
        assert_eq!(invoke(&NullAdapter), SandboxRunStatus::Completed);
    }

    #[test]
    fn default_pre_check_blocks_default_deny() {
        let adapter = NullAdapter;
        let run = fixture_run();
        let policy = SandboxPolicyV1::default_deny("baseline");
        let denial = adapter
            .pre_check(&run, &policy, &[SandboxCapability::Network])
            .expect_err("default-deny must reject NETWORK");
        assert_eq!(denial.kind, DenialKind::PolicyDenied);
        assert_eq!(denial.capability, Some(SandboxCapability::Network));
    }

    #[test]
    fn adapter_kind_is_serialisable_and_carries_tier() {
        let kind = AdapterKind::process_tier("policy_scoped_local", "Default local proof adapter");
        let s = serde_json::to_string(&kind).unwrap();
        assert!(s.contains("\"tier\":\"process\""));
        assert!(s.contains("policy_scoped_local"));
    }
}
