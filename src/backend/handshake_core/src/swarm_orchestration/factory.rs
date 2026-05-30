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

use crate::model_runtime::{CancellationToken, ModelId, ModelRuntime};
use crate::process_ledger::ProcessOwnershipRecordId;

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
pub type SessionTeardown = Box<dyn FnOnce() -> BoxFuture<'static, SwarmResult<()>> + Send>;

/// A live model session produced by a [`ModelSessionFactory`]. Owns the live
/// runtime adapter (shared `Arc` so the coordinator and the generation path
/// can both reference it) and its own [`CancellationToken`], plus the ledger
/// record id so teardown can write the matching stop row.
pub struct LiveSession {
    /// The live model runtime this session drives. Real in production, a real
    /// controllable adapter in tests — never a no-op stub.
    pub runtime: Arc<dyn ModelRuntime>,
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
        Self {
            runtime,
            model_id,
            cancel,
            teardown,
            process_record_id,
            os_pid,
        }
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
}
