//! Embedded-model ProcessOwnershipLedger seam (WP-1 MT-013).
//!
//! MT-003 made the embedded `ModelRuntime` the default local `LlmClient` load
//! path (`boot::build_default_local_client` -> `CandleRuntime`/`LlamaCppRuntime`
//! `::load`), but that path emitted NO ProcessOwnershipLedger rows. This module
//! is the ledger seam the default load path now goes through so the boxed +
//! ledgered obligation the swarm factory already satisfies (`process_ledger::
//! record_spawn`) also holds for the default `LlmClient` path.
//!
//! Boxing note (master-spec-v02.197 §3.6.2): the in-process Candle/llama.cpp
//! *library* load spawns NOTHING — there is no `std::process::Command`, no
//! child, no guest — so clause (1) of §3.6.2 ("child of a SandboxAdapter, no
//! bare `std::process::Command`") is satisfied VACUOUSLY. The ENFORCED,
//! unconditional obligation for this path is clause (2): the
//! ProcessOwnershipLedger START-on-load / STOP-on-unload rows (§4.6.1). Boxing
//! under a real SandboxAdapter applies only where a process/guest is actually
//! spawned (the swarm + CLI-bridge lanes, which are already boxed).
//!
//! pid honesty (MT-013 pre-impl decision P0): because no OS process exists, the
//! START row carries `os_pid = None` — an honest pid-less in-process row. We
//! FORBID synthesizing a fake pid. `LedgerDecorator` / `record_spawn` are NOT
//! usable here: they require a real spawn and a non-optional `u32` pid.
//! Downstream ledger consumers (reclaim/restart-resume) already tolerate
//! `Option<u32>` pids, so a pid-less row degrades gracefully (no OS-kill target,
//! just an attributable ownership record).

use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use uuid::Uuid;

use crate::model_runtime::{ModelId, RuntimeBinding};
use crate::process_ledger::{
    LedgerBatcher, ProcessEngineKind, ProcessLedgerError, ProcessStart, ProcessStop,
};

/// Owner-role tag carried on the embedded-model ledger rows. Mirrors the
/// `registered_by` operator id used by `boot::build_default_local_client` so the
/// ledger row attributes back to the same boot actor (MT-008 labeling).
pub const EMBEDDED_MODEL_OWNER_ROLE: &str = "handshake-embedded-default";

/// A ProcessOwnershipLedger ownership record for one in-process embedded model
/// load. Constructed via [`EmbeddedModelProcess::record_load`] (emits the START
/// row); the STOP row is emitted through the explicit [`Self::shutdown`] seam
/// (wired into app shutdown) and, as a safety net, on `Drop` of the
/// runtime-owning holder.
///
/// This is deliberately NOT keyed on an OS pid: the START/STOP rows are keyed on
/// `process_uuid`, which we set equal to the model's minted UUIDv7 `ModelId`, so
/// the ownership record is 1:1 with the loaded model instance.
pub struct EmbeddedModelProcess {
    ledger: LedgerBatcher,
    start: ProcessStart,
    stopped: AtomicBool,
}

impl EmbeddedModelProcess {
    /// Emits the ProcessOwnershipLedger START row for a just-loaded in-process
    /// embedded model and returns the ownership handle. `os_pid` is left `None`
    /// (honest pid-less in-process row); `process_uuid` is set to the minted
    /// `model_id` UUIDv7 so the ownership record is keyed to the model instance.
    /// `display_name` is carried in `metadata_jsonb` for MT-008 labeling.
    pub fn record_load(
        ledger: LedgerBatcher,
        binding: RuntimeBinding,
        model_id: ModelId,
        display_name: &str,
        artifact_sha256: Option<String>,
    ) -> Result<Self, ProcessLedgerError> {
        let engine_kind = match binding {
            RuntimeBinding::LlamaCpp => ProcessEngineKind::LlamaCpp,
            RuntimeBinding::Candle => ProcessEngineKind::Candle,
        };

        let mut start = ProcessStart::new(engine_kind, EMBEDDED_MODEL_OWNER_ROLE, None)
            // Key the ownership record on the minted model UUIDv7 so START and
            // STOP rows correlate to the model instance without an OS pid.
            .with_process_uuid(model_id.as_uuid())
            .with_metadata_jsonb(json!({
                "model_id": model_id.to_string(),
                "display_name": display_name,
                "in_process": true,
                // Explicit marker so a validator/consumer reading the row knows
                // the missing os_pid is intentional, not a data gap.
                "os_pid_absent_reason": "in_process_library_load_no_os_process",
                "source": "wp1_mt013_embedded_model_load",
            }));
        // NOTE: we intentionally do NOT call `.with_os_pid(..)` — a synthetic pid
        // is forbidden for this pid-less in-process load.
        if let Some(sha) = artifact_sha256 {
            start = start.with_model_artifact_sha256(sha);
        }

        ledger.record_start_lossless(start.clone())?;

        Ok(Self {
            ledger,
            start,
            stopped: AtomicBool::new(false),
        })
    }

    /// The ownership record id (equal to the model's minted UUIDv7).
    pub fn process_uuid(&self) -> Uuid {
        self.start.process_uuid
    }

    /// Explicit shutdown seam: emits the ProcessOwnershipLedger STOP row. This is
    /// the seam app-shutdown drives (NOT `ModelRuntime::unload`, which the
    /// default client never calls because it holds `Arc<dyn ModelRuntime>`).
    /// Idempotent — a second call (including the `Drop` safety net) is a no-op.
    pub fn shutdown(&self, reason: &str) -> Result<(), ProcessLedgerError> {
        if self
            .stopped
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }
        let stop = ProcessStop::from_start(&self.start, Some(0)).with_stop_reason(reason);
        if let Err(err) = self.ledger.record_stop_lossless(stop) {
            self.stopped.store(false, Ordering::SeqCst);
            return Err(err);
        }
        Ok(())
    }
}

impl Drop for EmbeddedModelProcess {
    fn drop(&mut self) {
        // Safety net: if app shutdown never called the explicit seam, still emit
        // the STOP row when the runtime-owning client is dropped at teardown.
        if let Err(err) = self.shutdown("embedded-model-holder-dropped") {
            tracing::warn!(
                target: "handshake_core::llm",
                error = %err,
                process_uuid = %self.start.process_uuid,
                "embedded model ProcessOwnershipLedger STOP on drop failed"
            );
        }
    }
}
