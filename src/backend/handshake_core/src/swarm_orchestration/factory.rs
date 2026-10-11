//! Injectable session creation + the live session handle the registry owns.
//!
//! The coordinator must not bake in candle/llama specifics. It depends only on
//! [`ModelSessionFactory`], which yields a [`LiveSession`] bound to an
//! `Arc<dyn ModelRuntime>` plus the process-ledger record id that attributes
//! the spawned process. Production backs the factory with a real
//! [`crate::model_runtime::ModelRuntime`] load + a `process_ledger` start
//! record; tests back it with a real controllable worker adapter (genuine
//! async work + state) — never a result-faking mock.

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;

use crate::llm::{LlmClient, ModelRuntimeLlmClient};
use crate::model_runtime::{CancellationToken, ModelId, ModelRuntime, WarmVmSnapshotManifest};
use crate::process_ledger::{
    ActiveProcessLifecycle, ProcessEngineKind, ProcessOwnershipRecordId, ProcessStart,
};

use super::error::SwarmResult;
use super::ids::SpawnRequest;

/// A boxed, single-shot async teardown handle that actually frees the engine
/// resource backing a [`LiveSession`].
///
/// # Hard contract
///
/// Invoking the teardown MUST free the loaded model from the underlying
/// runtime. This is not optional telemetry — it is the only thing that returns
/// the GPU/CPU memory the load consumed. Concretely:
///
/// - For an **owned** candle runtime, the teardown drops the owning `Arc` (the
///   last strong reference), which runs the runtime's `Drop` and detaches the
///   model — mirroring `kernel_model_runtime_unload`'s detach-drop. No engine
///   call is needed because dropping *is* the free.
/// - For a **shared** runtime (one `Arc<dyn ModelRuntime>` driving several
///   instances), the teardown calls `unload(model_id)` on the runtime so only
///   this instance's model is freed while the runtime stays live for siblings.
///
/// The coordinator invokes the teardown exactly once, on EVERY terminal path
/// (complete, cancel, lease-expiry reap, and duplicate-spawn rollback), AFTER
/// cancelling the session token and BEFORE/at the ledger STOP. A factory that
/// returns a no-op teardown is a resource leak and violates this contract.
pub type SessionTeardown = Arc<dyn Fn() -> BoxFuture<'static, SwarmResult<()>> + Send + Sync>;

/// Idempotent, synchronous side-effect committed only after the coordinator has persisted
/// every required launch record and transitioned the session to `Ready`.
/// Factories may use this to publish a secondary runtime handle without
/// leaking it when cancellation, duplicate detection, or persistence fails
/// after resource creation. The hook executes while the coordinator holds its
/// Ready/terminal registry fence, so it MUST NOT call back into SwarmCoordinator
/// or block on work that does; it should only publish already-prepared state.
pub type SessionReadyHook = Arc<dyn Fn() -> SwarmResult<()> + Send + Sync>;

/// A live model session produced by a [`ModelSessionFactory`]. Owns the live
/// runtime adapter (shared `Arc` so the coordinator and the generation path
/// can both reference it) and its own [`CancellationToken`], plus the ledger
/// record id so teardown can write the matching stop row.
pub struct LiveSession {
    /// The live model runtime this session drives. Real in production, a real
    /// controllable adapter in tests — never a no-op stub.
    pub runtime: Arc<dyn ModelRuntime>,
    /// Application-facing generation boundary. This client mediates every
    /// completion before the session runtime adapter is dispatched.
    pub llm_client: Arc<dyn LlmClient>,
    /// The concrete `ModelId` the factory's `load` returned. The coordinator
    /// keeps it so teardown can free *this* model from a shared runtime, and so
    /// the loaded model is never silently discarded (D1).
    pub model_id: ModelId,
    /// Per-session cancellation token. Cancelling it must abort in-flight
    /// generation on the underlying runtime.
    pub cancel: CancellationToken,
    /// Single-shot async teardown that actually frees the engine resource. See
    /// [`SessionTeardown`] for the hard contract. The coordinator MUST invoke
    /// this after cancel on every terminal path.
    pub teardown: SessionTeardown,
    /// Process-ledger ownership record id for the spawned process. The
    /// coordinator writes the matching stop row on teardown so the ledger
    /// never carries an orphan start.
    pub process_record_id: ProcessOwnershipRecordId,
    /// OS pid (or synthetic id for an in-process worker) recorded in the
    /// ledger; carried here so the stop row matches the start row.
    pub os_pid: u32,
    /// Exact PID value carried by the ProcessOwnershipLedger row. In-process
    /// sessions keep the coordinator's synthetic scheduling id in `os_pid` but
    /// set this to `None`, preventing that id from masquerading as a host PID.
    pub ledger_os_pid: Option<u32>,
    pub ledger_engine_kind_override: Option<ProcessEngineKind>,
    pub ledger_start_override: Option<ProcessStart>,
    /// Complete START/STOP capacity reserved before resource creation. Present
    /// for pidless cloud sessions whose START was durably acknowledged before
    /// this session could be published.
    pub ledger_lifecycle: Option<Arc<ActiveProcessLifecycle>>,
    /// Warm-VM restore metadata captured by the factory after a successful warm
    /// local load/restore. The coordinator ignores it; app/runtime side-tables
    /// can persist it so later warm VM spawns can skip a cold in-guest load.
    pub warm_vm_restore_manifest: Option<WarmVmSnapshotManifest>,
    /// Optional publication hook. The coordinator owns the commit point; a
    /// factory must not publish secondary handles before this hook runs.
    pub ready_hook: Option<SessionReadyHook>,
}

impl LiveSession {
    pub fn new(
        runtime: Arc<dyn ModelRuntime>,
        model_id: ModelId,
        cancel: CancellationToken,
        teardown: SessionTeardown,
        process_record_id: ProcessOwnershipRecordId,
        os_pid: u32,
    ) -> Self {
        let llm_client: Arc<dyn LlmClient> = Arc::new(
            ModelRuntimeLlmClient::new_coordinator_delegated(runtime.clone(), model_id),
        );
        Self {
            runtime,
            llm_client,
            model_id,
            cancel,
            teardown,
            process_record_id,
            os_pid,
            ledger_os_pid: Some(os_pid),
            ledger_engine_kind_override: None,
            ledger_start_override: None,
            ledger_lifecycle: None,
            warm_vm_restore_manifest: None,
            ready_hook: None,
        }
    }

    /// Replace the default runtime-backed facade with a provider-specific or
    /// instrumented LlmClient while retaining the runtime for lifecycle work.
    pub fn with_llm_client(mut self, llm_client: Arc<dyn LlmClient>) -> Self {
        self.llm_client = llm_client;
        self
    }

    pub fn with_pidless_ledger(
        mut self,
        engine_kind: ProcessEngineKind,
        start: ProcessStart,
    ) -> Self {
        self.ledger_os_pid = None;
        self.ledger_engine_kind_override = Some(engine_kind);
        self.ledger_start_override = Some(start);
        self
    }

    /// Retain the exact START identity for a session that has a real OS PID.
    /// The coordinator must derive STOP from this record so owner/WP/MT lineage
    /// cannot silently fall back to coordinator defaults.
    pub fn with_ledger_start(
        mut self,
        engine_kind: ProcessEngineKind,
        start: ProcessStart,
    ) -> Self {
        self.ledger_engine_kind_override = Some(engine_kind);
        self.ledger_start_override = Some(start);
        self
    }

    pub fn with_pidless_reserved_ledger(
        mut self,
        engine_kind: ProcessEngineKind,
        start: ProcessStart,
        lifecycle: Arc<ActiveProcessLifecycle>,
    ) -> Self {
        self.ledger_os_pid = None;
        self.ledger_engine_kind_override = Some(engine_kind);
        self.ledger_start_override = Some(start);
        self.ledger_lifecycle = Some(lifecycle);
        self
    }

    pub fn with_warm_vm_restore_manifest(mut self, manifest: WarmVmSnapshotManifest) -> Self {
        self.warm_vm_restore_manifest = Some(manifest);
        self
    }

    pub fn with_ready_hook(mut self, hook: SessionReadyHook) -> Self {
        self.ready_hook = Some(hook);
        self
    }
}

/// Creates live model sessions on demand. The single async seam between the
/// coordinator's orchestration logic and the concrete runtime/process world.
#[async_trait]
pub trait ModelSessionFactory: Send + Sync + 'static {
    /// Create a live session for `request`. The factory is responsible for the
    /// real model load and for recording the process-ledger start row (so the
    /// returned [`LiveSession::process_record_id`] is already attributable).
    ///
    /// # Teardown is a hard contract
    ///
    /// The returned [`LiveSession`] MUST carry a real [`SessionTeardown`] (see
    /// its docs) that frees the engine resource the load consumed. The
    /// coordinator invokes it on every terminal path; a factory that returns a
    /// no-op teardown leaks the loaded model forever. The returned
    /// [`LiveSession::model_id`] MUST be the `ModelId` `load` produced so a
    /// shared runtime can free exactly this instance.
    ///
    /// On failure it returns a typed [`super::error::SwarmError`] whose detail
    /// feeds the failure-fingerprint breaker. A failing factory MUST NOT leave
    /// an orphan ledger START row: if it recorded a START before failing, it
    /// must record the matching STOP before returning the error.
    async fn create(&self, request: &SpawnRequest) -> SwarmResult<LiveSession>;

    /// Compensate an abandoned [`Self::create`] future after coordinator
    /// cancellation. The default is intentionally a no-op because most
    /// factories do not cross an external process boundary before returning a
    /// [`LiveSession`]. Factories that do MUST use the request's exact instance
    /// identity and clean only resources created by that pending attempt.
    async fn cancel_pending_create(&self, _request: &SpawnRequest) -> SwarmResult<()> {
        Ok(())
    }
}
