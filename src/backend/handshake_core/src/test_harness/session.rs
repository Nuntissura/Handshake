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
        action_catalog::kernel002_action_catalog, KernelActor, KernelEvent, KernelEventType,
        NewKernelEvent,
    },
    process_ledger::{
        LedgerEvent, ProcessEngineKind, ProcessLedgerError, ProcessLedgerWriter, ProcessStart,
        ProcessStop, Reclaim, ReclaimProcessStore, ReclaimStopWriter, ReclaimTrigger,
        ReclaimableProcess, SandboxKill,
    },
};

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
}

impl SwarmSessionRuntime {
    pub(crate) fn new(
        process_writer: Arc<ProcessLedgerWriter>,
        event_ledger: Arc<Mutex<Vec<KernelEvent>>>,
    ) -> Self {
        Self {
            process_writer,
            event_ledger,
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
