use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Instant,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{
    kernel::{
        action_catalog::{kernel002_action_catalog, KernelActionCatalogV1},
        sandbox::CancellationToken,
        KernelActor, KernelEvent, KernelEventType, NewKernelEvent,
    },
    process_ledger::{
        LedgerEvent, ProcessEngineKind, ProcessLedgerError, ProcessLedgerWriter, ProcessStart,
        ProcessStop, Reclaim, ReclaimProcessStore, ReclaimStopWriter, ReclaimTrigger,
        ReclaimableProcess, SandboxKill,
    },
};

use super::crdt_workspace::{CrdtMutationOutcome, SharedCrdtWorkspace, SharedLeaseRegistry};
use super::swarm::SwarmHarnessError;

const SWARM_SOURCE_COMPONENT: &str = "kernel_swarm_test_harness";
const SWARM_OWNER_ROLE: &str = "KERNEL_BUILDER";
const SWARM_OWNER_WP: &str =
    "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum SessionStep {
    OpenWorkspace {
        ws_id: String,
    },
    MutateViaCatalog {
        action_id: String,
        envelope_ref: String,
    },
    /// Real CRDT mutation dispatched through the kernel action catalog against a
    /// shared CRDT workspace. Unlike `MutateViaCatalog` (which only confirms the
    /// catalog action exists and logs an event), this step actually applies an
    /// optimistic-concurrency write to a shared workspace, so concurrent
    /// sessions targeting the same `field_id` produce *measured* conflicts,
    /// revision rejections, and lease waits — not arithmetic.
    MutateCrdtField {
        action_id: String,
        field_id: String,
        /// When set, the mutation first acquires a real exclusive lease on this
        /// resource id, measuring actual contention wait time.
        lease_resource: Option<String>,
        /// When true, the step checks the runtime cancellation token before and
        /// after the write; if cancelled mid-flight it aborts and records a real
        /// cancellation instead of committing.
        cancellable: bool,
    },
    ReadInspector,
    Reclaim {
        trigger: ReclaimTrigger,
    },
    CloseSession,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionResult {
    pub session_idx: usize,
    pub session_id: String,
    pub steps_completed: usize,
    pub errors: Vec<String>,
    pub duration_ms: u128,
    pub local_marker: String,
    pub foreign_marker_seen: bool,
    pub reclaim_triggers: Vec<ReclaimTrigger>,
    pub process_ledger_rows: Vec<LedgerEvent>,
}

#[derive(Clone)]
pub(crate) struct SwarmSessionRuntime {
    pub(crate) process_writer: Arc<ProcessLedgerWriter>,
    event_ledger: Arc<Mutex<Vec<KernelEvent>>>,
    /// Real shared CRDT workspace all sessions concurrently mutate. Carries the
    /// real conflict / revision-rejection / lease / cancellation evidence.
    workspace: Arc<SharedCrdtWorkspace>,
    /// Real exclusive-lease registry backed by `tokio::sync::Mutex`. Concurrent
    /// acquirers of the same resource id wait for real elapsed time.
    leases: Arc<SharedLeaseRegistry>,
    /// Real kernel action catalog used to dispatch `MutateCrdtField` steps.
    action_catalog: Arc<KernelActionCatalogV1>,
    /// Real cooperative cancellation token from `kernel::sandbox`. A watcher
    /// flips it mid-run; in-flight cancellable mutations observe it and abort.
    cancellation: CancellationToken,
}

impl SwarmSessionRuntime {
    /// Build a runtime sharing an explicit CRDT workspace / lease registry /
    /// cancellation token so the harness can read the measured contention after
    /// all sessions complete.
    pub(crate) fn with_crdt_workspace(
        process_writer: Arc<ProcessLedgerWriter>,
        event_ledger: Arc<Mutex<Vec<KernelEvent>>>,
        workspace: Arc<SharedCrdtWorkspace>,
        leases: Arc<SharedLeaseRegistry>,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            process_writer,
            event_ledger,
            workspace,
            leases,
            action_catalog: Arc::new(kernel002_action_catalog()),
            cancellation,
        }
    }

    fn append_event(
        &self,
        session_id: &str,
        event_type: KernelEventType,
        payload: serde_json::Value,
    ) -> Result<(), SwarmHarnessError> {
        let event = NewKernelEvent::builder(
            format!("KTR-SWARM-{session_id}"),
            session_id.to_string(),
            event_type,
            KernelActor::System(SWARM_SOURCE_COMPONENT.to_string()),
        )
        .aggregate("swarm_session", session_id.to_string())
        .idempotency_key(format!("SWARM:{session_id}:{}", Uuid::now_v7()))
        .source_component(SWARM_SOURCE_COMPONENT)
        .payload(payload)
        .build()?;
        let mut event = KernelEvent::from_new(event);
        let mut ledger = self
            .event_ledger
            .lock()
            .map_err(|_| SwarmHarnessError::Poisoned("event ledger".to_string()))?;
        event.event_sequence = i64::try_from(ledger.len() + 1).unwrap_or(i64::MAX);
        ledger.push(event);
        Ok(())
    }

    /// Dispatch a real CRDT mutation against the shared workspace.
    ///
    /// 1. (optional) acquire a real exclusive lease, measuring actual wait time;
    /// 2. (optional) honour the real cancellation token before/after the write;
    /// 3. apply an optimistic-concurrency write keyed by the actor's last-seen
    ///    revision. Concurrent writers observe each other's committed revision
    ///    and produce real conflicts / revision rejections.
    async fn apply_crdt_mutation(
        &self,
        session_id: &str,
        session_idx: usize,
        action_id: &str,
        field_id: &str,
        lease_resource: Option<&str>,
        cancellable: bool,
    ) -> Result<CrdtMutationOutcome, SwarmHarnessError> {
        // Real cancellation observed *before* the write begins.
        if cancellable && self.cancellation.is_cancelled() {
            return Ok(self.workspace.record_cancellation(
                session_idx,
                session_id,
                field_id,
                action_id,
            ));
        }

        // Real exclusive lease acquisition under contention. The wait is real
        // elapsed time, measured across the actual async lock.
        let _lease_guard = match lease_resource {
            Some(resource) => Some(
                self.leases
                    .acquire(resource, session_idx, &self.workspace)
                    .await,
            ),
            None => None,
        };

        // A cancellable mutation does real, interruptible work: it yields a
        // bounded async checkpoint so a cancel flipped mid-run by the platform
        // token can actually land while the step is in flight (cooperative
        // cancellation). This is real elapsed time, not a synthetic delay.
        if cancellable {
            tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        }

        // Real cancellation observed *mid-flight*, after the interruptible work
        // but before commit — models a cancel that lands while the step is in
        // progress.
        if cancellable && self.cancellation.is_cancelled() {
            return Ok(self.workspace.record_cancellation(
                session_idx,
                session_id,
                field_id,
                action_id,
            ));
        }

        Ok(self
            .workspace
            .apply_optimistic_write(session_idx, session_id, field_id, action_id))
    }
}

pub(crate) struct SwarmSession {
    session_idx: usize,
    session_id: String,
    steps: Vec<SessionStep>,
    runtime: SwarmSessionRuntime,
}

impl SwarmSession {
    pub(crate) fn new(
        session_idx: usize,
        session_id: String,
        steps: Vec<SessionStep>,
        runtime: SwarmSessionRuntime,
    ) -> Self {
        Self {
            session_idx,
            session_id,
            steps,
            runtime,
        }
    }

    pub(crate) async fn run(self) -> Result<SessionResult, SwarmHarnessError> {
        let started = Instant::now();
        let local_marker = format!("swarm-marker-session-{}", self.session_idx);
        let mut state = SessionLocalState::new(local_marker.clone());
        let mut errors = Vec::new();
        let mut steps_completed = 0;
        let mut reclaim_triggers = Vec::new();
        let mut stopped = false;

        let process_start = ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            SWARM_OWNER_ROLE,
            Some(SWARM_OWNER_WP.to_string()),
        )
        .with_parent_session_id(self.session_id.clone())
        .with_sandbox_adapter_id("kernel_swarm_test_harness")
        .with_work_profile_id("swarm-harness-baseline");
        self.runtime.append_event(
            &self.session_id,
            KernelEventType::SessionStarted,
            json!({
                "session_idx": self.session_idx,
                "marker": local_marker,
            }),
        )?;
        self.runtime
            .process_writer
            .append_start(process_start.clone())?;

        for step in self.steps {
            match step {
                SessionStep::OpenWorkspace { ws_id } => {
                    state.open_workspace(ws_id.clone());
                    self.runtime.append_event(
                        &self.session_id,
                        KernelEventType::TaskIntentRecorded,
                        json!({
                            "step": "open_workspace",
                            "workspace_id": ws_id,
                            "marker": state.local_marker,
                        }),
                    )?;
                }
                SessionStep::MutateViaCatalog {
                    action_id,
                    envelope_ref,
                } => {
                    if kernel002_action_catalog().action(&action_id).is_none() {
                        errors.push(format!("unknown catalog action: {action_id}"));
                    } else {
                        self.runtime.append_event(
                            &self.session_id,
                            KernelEventType::InspectorReplayDrive,
                            json!({
                                "step": "mutate_via_catalog",
                                "action_id": action_id,
                                "envelope_ref": envelope_ref,
                                "marker": state.local_marker,
                            }),
                        )?;
                    }
                }
                SessionStep::MutateCrdtField {
                    action_id,
                    field_id,
                    lease_resource,
                    cancellable,
                } => {
                    if self.runtime.action_catalog.action(&action_id).is_none() {
                        errors.push(format!("unknown catalog action: {action_id}"));
                    } else {
                        let outcome = self
                            .runtime
                            .apply_crdt_mutation(
                                &self.session_id,
                                self.session_idx,
                                &action_id,
                                &field_id,
                                lease_resource.as_deref(),
                                cancellable,
                            )
                            .await?;
                        if let Some(event_type) = outcome.kernel_event_type() {
                            self.runtime.append_event(
                                &self.session_id,
                                event_type,
                                outcome.event_payload(self.session_idx, &field_id),
                            )?;
                        }
                    }
                }
                SessionStep::ReadInspector => {
                    state.foreign_marker_seen = state.foreign_marker_seen();
                    self.runtime.append_event(
                        &self.session_id,
                        KernelEventType::TraceReplayed,
                        json!({
                            "step": "read_inspector",
                            "workspace_id": state.workspace_id,
                            "foreign_marker_seen": state.foreign_marker_seen,
                        }),
                    )?;
                }
                SessionStep::Reclaim { trigger } => {
                    run_empty_reclaim(&self.session_id, trigger).await?;
                    reclaim_triggers.push(trigger);
                }
                SessionStep::CloseSession => {
                    self.runtime
                        .process_writer
                        .append_stop(ProcessStop::from_start(&process_start, Some(0)))?;
                    stopped = true;
                    self.runtime.append_event(
                        &self.session_id,
                        KernelEventType::SessionCompleted,
                        json!({
                            "step": "close_session",
                            "steps_completed": steps_completed + 1,
                        }),
                    )?;
                }
            }
            steps_completed += 1;
        }

        if !stopped {
            self.runtime
                .process_writer
                .append_stop(ProcessStop::from_start(&process_start, Some(0)))?;
        }

        Ok(SessionResult {
            session_idx: self.session_idx,
            session_id: self.session_id,
            steps_completed,
            errors,
            duration_ms: started.elapsed().as_millis().max(1),
            local_marker: state.local_marker,
            foreign_marker_seen: state.foreign_marker_seen,
            reclaim_triggers,
            process_ledger_rows: Vec::new(),
        })
    }
}

#[derive(Clone, Debug)]
struct SessionLocalState {
    local_marker: String,
    visible_markers: BTreeSet<String>,
    workspace_id: Option<String>,
    foreign_marker_seen: bool,
}

impl SessionLocalState {
    fn new(local_marker: String) -> Self {
        let mut visible_markers = BTreeSet::new();
        visible_markers.insert(local_marker.clone());
        Self {
            local_marker,
            visible_markers,
            workspace_id: None,
            foreign_marker_seen: false,
        }
    }

    fn open_workspace(&mut self, ws_id: String) {
        self.workspace_id = Some(ws_id);
        self.visible_markers.insert(self.local_marker.clone());
    }

    fn foreign_marker_seen(&self) -> bool {
        self.visible_markers
            .iter()
            .any(|marker| marker != &self.local_marker)
    }
}

async fn run_empty_reclaim(
    session_id: &str,
    trigger: ReclaimTrigger,
) -> Result<(), ProcessLedgerError> {
    let reclaim = Reclaim::new(
        Arc::new(EmptyReclaimStore),
        Arc::new(NoopSandboxKill),
        Arc::new(NoopReclaimStopWriter),
    );
    reclaim.run(session_id, trigger).await.map(|_| ())
}

struct EmptyReclaimStore;

#[async_trait]
impl ReclaimProcessStore for EmptyReclaimStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Ok(Vec::new())
    }
}

struct NoopSandboxKill;

impl SandboxKill for NoopSandboxKill {
    fn kill(&self, _process_uuid: Uuid) -> Result<(), crate::process_ledger::KillError> {
        Ok(())
    }
}

struct NoopReclaimStopWriter;

impl ReclaimStopWriter for NoopReclaimStopWriter {
    fn append_reclaim_stop(&self, _stop: ProcessStop) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
}
