//! Embedded-model ProcessOwnershipLedger seam (WP-1 MT-013).
//!
//! MT-003 made the embedded `ModelRuntime` the default local `LlmClient` load
//! path (`boot::build_default_local_client` -> `CandleRuntime`/`LlamaCppRuntime`
//! `::load`), but that path emitted NO ProcessOwnershipLedger rows. This module
//! is the ownership-ledger seam the default load path now goes through. It does
//! not claim that an in-process library load is a real `SandboxAdapter` child;
//! subprocess/guest boxing remains a separate runtime-lane obligation.
//!
//! Boxing note (master-spec-v02.197 §3.6.2): the in-process Candle/llama.cpp
//! *library* load spawns NOTHING — there is no `std::process::Command`, no
//! child, no guest — so clause (1) of §3.6.2 ("child of a SandboxAdapter, no
//! bare `std::process::Command`") is satisfied VACUOUSLY. The ENFORCED,
//! unconditional obligation for this path is clause (2): the
//! ProcessOwnershipLedger START-on-load / STOP-on-unload rows (§4.6.1). Boxing
//! under a real SandboxAdapter applies only where a process/guest is actually
//! spawned. This in-process seam makes no claim about those separate lanes.
//!
//! pid honesty (MT-013 pre-impl decision P0): because no OS process exists, the
//! START row carries `os_pid = None` — an honest pid-less in-process row. We
//! FORBID synthesizing a fake pid. `LedgerDecorator` / `record_spawn` are NOT
//! usable here: they require a real spawn and a non-optional `u32` pid.
//! Downstream ledger consumers (reclaim/restart-resume) already tolerate
//! `Option<u32>` pids, so a pid-less row degrades gracefully (no OS-kill target,
//! just an attributable ownership record).

use std::time::Duration;

use serde_json::json;
use uuid::Uuid;

use crate::model_runtime::{ModelId, RuntimeArtifactIntegrityReceipt, RuntimeBinding};
#[cfg(feature = "test-utils")]
use crate::process_ledger::LedgerBatcher;
use crate::process_ledger::{
    ActiveProcessLifecycle, EmbeddedRuntimeInstanceDescriptor, ProcessEngineKind,
    ProcessLedgerDurabilityAck, ProcessLedgerError, ProcessStart, ReservedProcessLifecycle,
    StopRecordOutcome,
};
use crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;

/// Owner-role tag carried on the embedded-model ledger rows. Mirrors the
/// `registered_by` operator id used by `boot::build_default_local_client` so the
/// ledger row attributes back to the same boot actor (MT-008 labeling).
pub const EMBEDDED_MODEL_OWNER_ROLE: &str = "handshake-embedded-default";

/// A ProcessOwnershipLedger ownership record for one in-process embedded model
/// load. Production boot constructs it through the durable-ack transition; the
/// STOP row is emitted only through the explicit [`Self::shutdown`] seam after
/// runtime quiescence and successful `ModelRuntime::unload` have both been
/// proven by the owning client. Dropping an unquiesced or not-unloaded holder
/// deliberately leaves START open for liveness reconciliation.
///
/// This is deliberately NOT keyed on an OS pid: the START/STOP rows are keyed on
/// `process_uuid`. On the valid path it equals the model's minted UUIDv7
/// `ModelId`. If a runtime violates that identity contract, a distinct UUIDv7
/// quarantine key prevents row aliasing while metadata preserves the reported id.
pub struct EmbeddedModelProcess {
    lifecycle: ActiveProcessLifecycle,
}

impl EmbeddedModelProcess {
    /// Emits the ProcessOwnershipLedger START row for a just-loaded in-process
    /// embedded model and returns the ownership handle. `os_pid` is left `None`
    /// (honest pid-less in-process row); on the valid path `process_uuid` is set
    /// to the minted `model_id` UUIDv7.
    /// `display_name` is carried in `metadata_jsonb` for MT-008 labeling.
    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn record_load(
        ledger: LedgerBatcher,
        binding: RuntimeBinding,
        model_id: ModelId,
        display_name: &str,
        artifact_sha256: Option<String>,
    ) -> Result<Self, ProcessLedgerError> {
        let reservation = ledger.try_reserve_lifecycles(1)?.pop().ok_or_else(|| {
            ProcessLedgerError::InvalidConfig(
                "single embedded lifecycle reservation was empty".to_string(),
            )
        })?;
        Self::record_reserved_load(
            reservation,
            binding,
            model_id,
            display_name,
            artifact_sha256,
            None,
        )
    }

    /// Begin a lifecycle from capacity reserved before artifact access.
    /// `runtime_instance` is the OS-owned loopback lease descriptor used by
    /// crash reconciliation to distinguish a stale row from another live
    /// Handshake instance without depending on PostgreSQL session lifetime.
    #[doc(hidden)]
    #[cfg(feature = "test-utils")]
    pub fn record_reserved_load(
        reservation: ReservedProcessLifecycle,
        binding: RuntimeBinding,
        model_id: ModelId,
        display_name: &str,
        artifact_sha256: Option<String>,
        runtime_instance: Option<&EmbeddedRuntimeInstanceDescriptor>,
    ) -> Result<Self, ProcessLedgerError> {
        let start = Self::start_event(
            binding,
            model_id.as_uuid(),
            model_id,
            display_name,
            artifact_sha256,
            runtime_instance,
            None,
            None,
            None,
        )?;
        let lifecycle = reservation.begin(start)?;
        Ok(Self { lifecycle })
    }

    /// Begin a loaded model lifecycle and return proof that its START reaches
    /// the authoritative store. Production boot awaits this acknowledgement
    /// before the model enters any READY registry or client surface.
    pub(super) fn record_reserved_load_with_durable_ack(
        reservation: ReservedProcessLifecycle,
        binding: RuntimeBinding,
        model_id: ModelId,
        display_name: &str,
        artifact_integrity: &RuntimeArtifactIntegrityReceipt,
        runtime_instance: Option<&EmbeddedRuntimeInstanceDescriptor>,
    ) -> Result<(Self, ProcessLedgerDurabilityAck), ProcessLedgerError> {
        Self::record_reserved_load_with_durable_ack_scoped(
            reservation,
            binding,
            model_id,
            display_name,
            artifact_integrity,
            runtime_instance,
            None,
        )
    }

    pub(super) fn record_reserved_load_with_durable_ack_scoped(
        reservation: ReservedProcessLifecycle,
        binding: RuntimeBinding,
        model_id: ModelId,
        display_name: &str,
        artifact_integrity: &RuntimeArtifactIntegrityReceipt,
        runtime_instance: Option<&EmbeddedRuntimeInstanceDescriptor>,
        resource_scope: Option<&ExactResourceScopeAttribution>,
    ) -> Result<(Self, ProcessLedgerDurabilityAck), ProcessLedgerError> {
        let start = Self::start_event(
            binding,
            model_id.as_uuid(),
            model_id,
            display_name,
            None,
            runtime_instance,
            Some(artifact_integrity),
            None,
            resource_scope,
        )?;
        let (lifecycle, durable_ack) = reservation.begin_with_durable_ack(start)?;
        Ok((Self { lifecycle }, durable_ack))
    }

    /// Ledger a runtime that violated the returned-identity contract without
    /// reusing an unsafe/non-UUIDv7 process key. The quarantine UUID is distinct
    /// from every model id while metadata preserves the exact reported id.
    pub(super) fn record_reserved_quarantine_load_with_durable_ack(
        reservation: ReservedProcessLifecycle,
        binding: RuntimeBinding,
        quarantine_process_uuid: Uuid,
        reported_model_id: ModelId,
        display_name: &str,
        artifact_integrity: &RuntimeArtifactIntegrityReceipt,
        runtime_instance: Option<&EmbeddedRuntimeInstanceDescriptor>,
        identity_violation: &str,
    ) -> Result<(Self, ProcessLedgerDurabilityAck), ProcessLedgerError> {
        Self::record_reserved_quarantine_load_with_durable_ack_scoped(
            reservation,
            binding,
            quarantine_process_uuid,
            reported_model_id,
            display_name,
            artifact_integrity,
            runtime_instance,
            identity_violation,
            None,
        )
    }

    pub(super) fn record_reserved_quarantine_load_with_durable_ack_scoped(
        reservation: ReservedProcessLifecycle,
        binding: RuntimeBinding,
        quarantine_process_uuid: Uuid,
        reported_model_id: ModelId,
        display_name: &str,
        artifact_integrity: &RuntimeArtifactIntegrityReceipt,
        runtime_instance: Option<&EmbeddedRuntimeInstanceDescriptor>,
        identity_violation: &str,
        resource_scope: Option<&ExactResourceScopeAttribution>,
    ) -> Result<(Self, ProcessLedgerDurabilityAck), ProcessLedgerError> {
        let start = Self::start_event(
            binding,
            quarantine_process_uuid,
            reported_model_id,
            display_name,
            None,
            runtime_instance,
            Some(artifact_integrity),
            Some(identity_violation),
            resource_scope,
        )?;
        let (lifecycle, durable_ack) = reservation.begin_with_durable_ack(start)?;
        Ok((Self { lifecycle }, durable_ack))
    }

    fn start_event(
        binding: RuntimeBinding,
        process_uuid: Uuid,
        model_id: ModelId,
        display_name: &str,
        artifact_sha256: Option<String>,
        runtime_instance: Option<&EmbeddedRuntimeInstanceDescriptor>,
        artifact_integrity: Option<&RuntimeArtifactIntegrityReceipt>,
        identity_violation: Option<&str>,
        resource_scope: Option<&ExactResourceScopeAttribution>,
    ) -> Result<ProcessStart, ProcessLedgerError> {
        let engine_kind = match binding {
            RuntimeBinding::LlamaCpp => ProcessEngineKind::LlamaCpp,
            RuntimeBinding::Candle => ProcessEngineKind::Candle,
        };

        let mut metadata = json!({
            "model_id": model_id.to_string(),
            "display_name": display_name,
            "in_process": true,
            // Explicit marker so a validator/consumer reading the row knows
            // the missing os_pid is intentional, not a data gap.
            "os_pid_absent_reason": "in_process_library_load_no_os_process",
            "source": "wp1_mt013_embedded_model_load",
        });
        if let Some(runtime_instance) = runtime_instance {
            let metadata_object = metadata.as_object_mut().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "embedded model lifecycle metadata must be a JSON object".to_string(),
                )
            })?;
            let descriptor_fields = runtime_instance.metadata_fields();
            let descriptor_object = descriptor_fields.as_object().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "embedded runtime descriptor metadata must be a JSON object".to_string(),
                )
            })?;
            metadata_object.extend(descriptor_object.clone());
        }
        if let Some(artifact_integrity) = artifact_integrity {
            let metadata_object = metadata.as_object_mut().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "embedded model lifecycle metadata must be a JSON object".to_string(),
                )
            })?;
            metadata_object.insert(
                "artifact_integrity_receipt".to_string(),
                json!(artifact_integrity),
            );
        }
        if let Some(identity_violation) = identity_violation {
            let metadata_object = metadata.as_object_mut().ok_or_else(|| {
                ProcessLedgerError::InvalidConfig(
                    "embedded model lifecycle metadata must be a JSON object".to_string(),
                )
            })?;
            metadata_object.insert(
                "identity_contract_violation".to_string(),
                json!(identity_violation),
            );
            metadata_object.insert(
                "quarantine_process_uuid".to_string(),
                json!(process_uuid.to_string()),
            );
        }
        if let Some(resource_scope) = resource_scope {
            resource_scope
                .stamp_json_object(&mut metadata)
                .map_err(|error| {
                    ProcessLedgerError::InvalidConfig(format!(
                        "embedded model lifecycle resource attribution is invalid: {error}"
                    ))
                })?;
        }

        let mut start = ProcessStart::new(engine_kind, EMBEDDED_MODEL_OWNER_ROLE, None)
            // Normal rows use the minted model UUIDv7. Identity-contract
            // violations use a distinct quarantine UUID to prevent aliasing.
            .with_process_uuid(process_uuid)
            .with_metadata_jsonb(metadata);
        // NOTE: we intentionally do NOT call `.with_os_pid(..)` — a synthetic pid
        // is forbidden for this pid-less in-process load.
        let verified_artifact_sha256 = artifact_integrity
            .map(|receipt| receipt.primary_artifact_sha256().to_string())
            .or(artifact_sha256);
        if let Some(sha) = verified_artifact_sha256 {
            start = start.with_model_artifact_sha256(sha);
        }

        Ok(start)
    }

    /// The ownership record id (normally the model UUIDv7; quarantine UUIDv7 on
    /// a runtime identity-contract violation).
    pub fn process_uuid(&self) -> Uuid {
        self.lifecycle.process_uuid()
    }

    /// Explicit post-unload seam: emits the ProcessOwnershipLedger STOP row.
    /// The owning client may call this only after it has taken unique runtime
    /// ownership and completed `ModelRuntime::unload` for this model.
    /// Idempotent — a second call is a no-op.
    pub fn shutdown(&self, reason: &str) -> Result<(), ProcessLedgerError> {
        match self.lifecycle.stop(Some(0), reason)? {
            StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped => Ok(()),
            StopRecordOutcome::LeftOpenForReconciliation
            | StopRecordOutcome::DurabilityUnconfirmed => {
                Err(ProcessLedgerError::InvalidConfig(format!(
                    "embedded lifecycle {} was left open for reconciliation and cannot later report a graceful STOP",
                    self.process_uuid()
                )))
            }
        }
    }

    /// Graceful shutdown seam for owners that are about to release the final
    /// runtime/liveness handle. Complete STOP capacity was reserved before
    /// artifact access, while the supplied timeout bounds the separate
    /// authoritative PostgreSQL durability acknowledgement.
    pub async fn shutdown_bounded(
        &self,
        reason: &str,
        timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        match self
            .lifecycle
            .stop_with_durable_ack(Some(0), reason, timeout)
            .await?
        {
            StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped => Ok(()),
            StopRecordOutcome::LeftOpenForReconciliation
            | StopRecordOutcome::DurabilityUnconfirmed => {
                Err(ProcessLedgerError::InvalidConfig(format!(
                    "embedded lifecycle {} was left open for reconciliation and cannot later report a graceful durable STOP",
                    self.process_uuid()
                )))
            }
        }
    }

    /// Consume the reserved STOP capacity without publishing STOP because the
    /// caller could not prove runtime quiescence. The open START row is then
    /// closed only by the authoritative liveness reconciler after process death.
    pub fn leave_open_for_reconciliation(&self) -> bool {
        self.lifecycle.leave_open_for_reconciliation()
    }
}

impl Drop for EmbeddedModelProcess {
    fn drop(&mut self) {
        // A dropped client does not prove detached generation or blocking
        // inference workers have stopped. Never forge a clean STOP here: leave
        // the START row open so the OS-owned runtime lease and boot-time
        // reconciliation remain the source of truth after an ungraceful exit.
        if self.leave_open_for_reconciliation() {
            tracing::warn!(
                target: "handshake_core::llm",
                process_uuid = %self.lifecycle.process_uuid(),
                "embedded model holder dropped without a proven graceful STOP; START remains open for reconciliation"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{LlamaCppArtifactIntegrityReceipt, ModelArtifactComponentIntegrity};
    use crate::swarm_orchestration::resource_scope::{
        AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
        WorkspaceScopeRef,
    };

    #[test]
    fn quarantine_start_uses_distinct_uuidv7_and_preserves_reported_model_id() {
        let reported_model_id = ModelId::new_v7();
        let quarantine_process_uuid = Uuid::now_v7();
        assert_ne!(quarantine_process_uuid, reported_model_id.as_uuid());
        let start = EmbeddedModelProcess::start_event(
            RuntimeBinding::Candle,
            quarantine_process_uuid,
            reported_model_id,
            "duplicate-id-quarantine",
            Some("aa".repeat(32)),
            None,
            None,
            Some("duplicate model identity"),
            None,
        )
        .expect("quarantine START shape");

        assert_eq!(start.process_uuid, quarantine_process_uuid);
        assert_eq!(start.process_uuid.get_version_num(), 7);
        assert_eq!(
            start.metadata_jsonb["model_id"],
            reported_model_id.to_string()
        );
        assert_eq!(
            start.metadata_jsonb["identity_contract_violation"],
            "duplicate model identity"
        );
        assert_eq!(
            start.metadata_jsonb["quarantine_process_uuid"],
            quarantine_process_uuid.to_string()
        );
    }

    #[test]
    fn llama_cpp_start_uses_raw_gguf_digest_and_format_specific_receipt() {
        let model_id = ModelId::new_v7();
        let raw_digest = "ab".repeat(32);
        let receipt = RuntimeArtifactIntegrityReceipt::from(
            LlamaCppArtifactIntegrityReceipt::from_gguf_component(
                ModelArtifactComponentIntegrity {
                    sha256: raw_digest.clone(),
                    length_bytes: 4096,
                },
            )
            .expect("canonical GGUF receipt"),
        );
        let start = EmbeddedModelProcess::start_event(
            RuntimeBinding::LlamaCpp,
            model_id.as_uuid(),
            model_id,
            "attested-gguf",
            None,
            None,
            Some(&receipt),
            None,
            None,
        )
        .expect("llama.cpp START shape");

        assert_eq!(
            start.model_artifact_sha256.as_deref(),
            Some(raw_digest.as_str())
        );
        let integrity = &start.metadata_jsonb["artifact_integrity_receipt"];
        assert_eq!(
            integrity["schema_id"],
            "handshake.model_artifact_integrity.gguf.v1"
        );
        assert_eq!(integrity["gguf"]["sha256"], raw_digest);
        assert!(integrity.get("weights").is_none());
        assert!(integrity.get("config").is_none());
        assert!(integrity.get("tokenizer").is_none());
    }

    #[test]
    fn embedded_start_stamps_all_five_server_owned_scope_fields() {
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(
            &ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
                .with_session(AuthenticatedSessionRef::mint())
                .with_access_space(AccessSpaceRef::mint())
                .with_workspace(WorkspaceScopeRef::new("embedded-boot").unwrap()),
        )
        .expect("complete exact scope");
        let model_id = ModelId::new_v7();
        let start = EmbeddedModelProcess::start_event(
            RuntimeBinding::Candle,
            model_id.as_uuid(),
            model_id,
            "scoped-embedded-model",
            Some("cd".repeat(32)),
            None,
            None,
            None,
            Some(&exact),
        )
        .expect("scoped START shape");

        assert_eq!(
            serde_json::from_value::<ExactResourceScopeAttribution>(start.metadata_jsonb.clone())
                .expect("top-level exact attribution"),
            exact
        );
    }
}
