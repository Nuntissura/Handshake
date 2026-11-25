use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

use crate::model_runtime::WarmAgentTransport;
use crate::process_ledger::{
    cap_metadata_jsonb, ActiveProcessLifecycle, LedgerBatcher, ProcessEngineKind, ProcessStart,
    ProcessStop, ReservedProcessLifecycle, StopRecordOutcome,
};

use super::{
    AdapterCapabilities, AttachedNetworkMode, AttachedProcessSpec, AttachedSandboxProcess,
    AttachedStdioContract, BindMode, Command, ExecResult, NetPolicy, ProcessHandle, ProcessSpec,
    ProcessStatus, SandboxAdapter, SandboxAdapterError, Signal, SnapshotRef,
};

#[derive(Clone)]
pub struct LedgerDecorator {
    inner: Arc<dyn SandboxAdapter>,
    ledger: LedgerBatcher,
    starts: Arc<Mutex<HashMap<Uuid, ProcessStart>>>,
    stopped: Arc<Mutex<HashSet<Uuid>>>,
    pending_stops: Arc<Mutex<HashMap<Uuid, ProcessStop>>>,
    lifecycles: Arc<Mutex<HashMap<Uuid, Arc<ActiveProcessLifecycle>>>>,
    acquisitions: Arc<Mutex<HashMap<Uuid, Arc<RetainedAcquisition>>>>,
    process_states: Arc<Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<ProcessStopState>>>>>,
}

enum DecoratorAcquisition {
    Spawn(ProcessSpec),
    Restore(SnapshotRef),
}

struct AcquisitionState {
    caller_live: bool,
    invoked: bool,
    no_effect: bool,
    delivered: bool,
    handle: Option<ProcessHandle>,
    error: Option<SandboxAdapterError>,
    diagnostic: Option<String>,
}

struct RetainedAcquisition {
    keeper: LedgerDecorator,
    reservation: Mutex<Option<ReservedProcessLifecycle>>,
    state: Mutex<AcquisitionState>,
    join: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

static RETAINED_DECORATOR_ACQUISITIONS:
    std::sync::OnceLock<Mutex<HashMap<Uuid, Arc<RetainedAcquisition>>>> =
    std::sync::OnceLock::new();

struct AcquisitionCallerGuard {
    owner: Arc<RetainedAcquisition>,
}

impl Drop for AcquisitionCallerGuard {
    fn drop(&mut self) {
        self.owner.state.lock().unwrap_or_else(|error| error.into_inner()).caller_live = false;
    }
}

struct AcquisitionTaskGuard {
    owner: Arc<RetainedAcquisition>,
    completed: bool,
}

impl Drop for AcquisitionTaskGuard {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let mut state = self.owner.state.lock().unwrap_or_else(|error| error.into_inner());
        if !state.invoked {
            state.no_effect = true;
            self.owner.reservation.lock().unwrap_or_else(|error| error.into_inner()).take();
        }
        state.diagnostic = Some(
            "original acquisition task interrupted; invoked effects remain owned and unknown".into(),
        );
    }
}

enum KillRequestState {
    NotAttempted,
    Attempted,
    Acknowledged,
    Uncertain { reason: String },
}

struct TerminalObservation {
    status: ProcessStatus,
    exit_code: Option<i32>,
    exit_code_observed: bool,
}

struct ProcessStopState {
    handle: ProcessHandle,
    kill_requests: Vec<(Signal, KillRequestState)>,
    terminal: Option<TerminalObservation>,
    observed_exit_code: Option<i32>,
}

impl LedgerDecorator {
    pub fn new(inner: Arc<dyn SandboxAdapter>, ledger: LedgerBatcher) -> Self {
        Self {
            inner,
            ledger,
            starts: Arc::new(Mutex::new(HashMap::new())),
            stopped: Arc::new(Mutex::new(HashSet::new())),
            pending_stops: Arc::new(Mutex::new(HashMap::new())),
            lifecycles: Arc::new(Mutex::new(HashMap::new())),
            acquisitions: Arc::new(Mutex::new(HashMap::new())),
            process_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn acquire_reserved(
        &self,
        operation: DecoratorAcquisition,
    ) -> Result<ProcessHandle, SandboxAdapterError> {
        let adapter_id = self.inner.capabilities().adapter_id;
        match &operation {
            DecoratorAcquisition::Spawn(_) => self.inner.validate_spawn_acquisition()?,
            DecoratorAcquisition::Restore(_) if !self.inner.capabilities().supports_snapshot => {
                return Err(SandboxAdapterError::SnapshotUnsupported { adapter_id });
            }
            DecoratorAcquisition::Restore(_) => {}
        }
        let (id, owner, runtime) = {
            let mut pending = self.acquisitions.lock().unwrap_or_else(|error| error.into_inner());
            pending.retain(|id, owner| {
                let state = owner.state.lock().unwrap_or_else(|error| error.into_inner());
                if state.no_effect || state.delivered {
                    RETAINED_DECORATOR_ACQUISITIONS.get_or_init(Default::default)
                        .lock().unwrap_or_else(|error| error.into_inner()).remove(id);
                    false
                } else {
                    true
                }
            });
            for (id, owner) in pending.iter() {
                let state = owner.state.lock().unwrap_or_else(|error| error.into_inner());
                if !state.caller_live || state.diagnostic.is_some() || state.error.is_some() {
                    return Err(SandboxAdapterError::AdapterUnavailable {
                        adapter_id,
                        reason: format!("original acquisition {id} retains unresolved custody: {:?}",
                            state.diagnostic),
                    });
                }
            }
        let runtime = tokio::runtime::Handle::try_current().map_err(|error| {
            SandboxAdapterError::AdapterUnavailable {
                adapter_id: adapter_id.clone(),
                reason: format!("owned acquisition requires a running executor: {error}"),
            }
        })?;
        let reservation = self.ledger.try_reserve_lifecycles(1)
            .map_err(|error| SandboxAdapterError::SpawnFailed {
                adapter_id: adapter_id.clone(),
                reason: format!("process ledger paired preflight failed: {error}"),
            })?.pop().expect("one complete lifecycle reservation");
        let id = Uuid::now_v7();
        let owner = Arc::new(RetainedAcquisition {
            keeper: self.clone(),
            reservation: Mutex::new(Some(reservation)),
            state: Mutex::new(AcquisitionState {
                caller_live: true, invoked: false, no_effect: false, delivered: false,
                handle: None, error: None, diagnostic: None,
            }),
            join: tokio::sync::Mutex::new(None),
        });
        pending.insert(id, Arc::clone(&owner));
        RETAINED_DECORATOR_ACQUISITIONS.get_or_init(Default::default)
            .lock().unwrap_or_else(|error| error.into_inner()).insert(id, Arc::clone(&owner));
            (id, owner, runtime)
        };
        let _caller = AcquisitionCallerGuard { owner: Arc::clone(&owner) };
        let registry = crate::storage::surreal::configured_capability_registry();
        let worker = Arc::clone(&owner);
        let guard = AcquisitionTaskGuard { owner: Arc::clone(&owner), completed: false };
        let task = runtime.spawn(async move {
            let mut guard = guard;
            let work = async {
                worker.state.lock().unwrap_or_else(|error| error.into_inner()).invoked = true;
                let acquired = match &operation {
                    DecoratorAcquisition::Spawn(spec) => worker.keeper.inner.spawn(spec.clone()).await,
                    DecoratorAcquisition::Restore(snapshot) => worker.keeper.inner.restore(snapshot).await,
                };
                match acquired {
                    Ok(handle) => {
                        worker.state.lock().unwrap_or_else(|error| error.into_inner())
                            .handle = Some(handle.clone());
                        let start = match &operation {
                            DecoratorAcquisition::Spawn(spec) => worker.keeper.start_event(spec, &handle),
                            DecoratorAcquisition::Restore(_) => worker.keeper.restore_start_event(&handle),
                        };
                        let reservation = worker.reservation.lock()
                            .unwrap_or_else(|error| error.into_inner()).take()
                            .expect("original acquisition retains its paired reservation");
                        match reservation.begin_retained(start.clone()) {
                            Ok(lifecycle) => {
                                worker.keeper.starts.lock().unwrap_or_else(|error| error.into_inner())
                                    .insert(handle.id, start);
                                worker.keeper.lifecycles.lock().unwrap_or_else(|error| error.into_inner())
                                    .insert(handle.id, Arc::new(lifecycle));
                            }
                            Err(error) => {
                                let mut state = worker.state.lock().unwrap_or_else(|error| error.into_inner());
                                state.diagnostic = Some(format!("acquired handle START remains unconfirmed: {error}"));
                                state.error = Some(SandboxAdapterError::AdapterUnavailable {
                                    adapter_id: handle.adapter_id.clone(),
                                    reason: state.diagnostic.clone().unwrap_or_default(),
                                });
                            }
                        }
                    }
                    Err(error) => {
                        let mut state = worker.state.lock().unwrap_or_else(|error| error.into_inner());
                        state.diagnostic = Some(format!("invoked acquisition error; native effect unknown: {error}"));
                        state.error = Some(error);
                    }
                }
            };
            crate::storage::surreal::with_captured_capability_registry(registry, work).await;
            guard.completed = true;
        });
        // No await separates dispatch from registration of the original task.
        *owner.join.try_lock().expect("new acquisition join holder") = Some(task);
        let mut join = owner.join.lock().await;
        let observed = join.as_mut().expect("registered original acquisition task").await;
        join.take();
        if let Err(error) = observed {
            return Err(SandboxAdapterError::AdapterUnavailable {
                adapter_id,
                reason: format!("original acquisition {id} did not complete: {error}; custody retained"),
            });
        }
        let mut state = owner.state.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(error) = state.error.take() {
            return Err(error);
        }
        let handle = state.handle.clone().ok_or_else(|| SandboxAdapterError::AdapterUnavailable {
            adapter_id, reason: format!("original acquisition {id} returned no handle; custody retained"),
        })?;
        state.delivered = true;
        drop(state);
        self.acquisitions.lock().unwrap_or_else(|error| error.into_inner()).remove(&id);
        RETAINED_DECORATOR_ACQUISITIONS.get_or_init(Default::default)
            .lock().unwrap_or_else(|error| error.into_inner()).remove(&id);
        Ok(handle)
    }

    pub(crate) fn pending_acquisition_handles(&self) -> Vec<ProcessHandle> {
        self.acquisitions.lock().unwrap_or_else(|error| error.into_inner()).values()
            .filter_map(|owner| owner.state.lock().unwrap_or_else(|error| error.into_inner())
                .handle.clone()).collect()
    }

    fn restore_start_event(&self, handle: &ProcessHandle) -> ProcessStart {
        let mut start = ProcessStart::new(ProcessEngineKind::SandboxContainer, "KERNEL_BUILDER", None)
            .with_process_uuid(handle.id)
            .with_sandbox_adapter_id(handle.adapter_id.as_str().to_string())
            .with_sandbox_internal_id(handle.sandbox_internal_id.clone())
            .with_sandbox_capabilities_snapshot(capabilities_snapshot(&self.inner.capabilities()));
        start.started_at = handle.spawned_at_utc;
        if let Some(pid) = handle.pid {
            start = start.with_os_pid(pid);
        }
        start
    }

    fn start_event(&self, spec: &ProcessSpec, handle: &ProcessHandle) -> ProcessStart {
        let metadata = cap_metadata_jsonb(&spec.metadata);
        let role_id = spec
            .metadata
            .get("role_id")
            .cloned()
            .unwrap_or_else(|| "KERNEL_BUILDER".to_string());
        let wp_id = spec.metadata.get("wp_id").cloned();
        let engine_kind = spec
            .metadata
            .get("engine_kind")
            .and_then(|value| ProcessEngineKind::try_from(value.as_str()).ok())
            .unwrap_or(ProcessEngineKind::SandboxContainer);

        let mut event = ProcessStart::new(engine_kind, role_id.clone(), wp_id.clone())
            .with_process_uuid(handle.id)
            .with_sandbox_adapter_id(handle.adapter_id.as_str().to_string())
            .with_sandbox_internal_id(handle.sandbox_internal_id.clone())
            .with_role_id(role_id)
            .with_sandbox_capabilities_snapshot(capabilities_snapshot(&self.inner.capabilities()))
            .with_metadata_jsonb(metadata.value);

        if let Some(pid) = handle.pid {
            event = event.with_os_pid(pid);
        }
        if let Some(parent_session_id) = spec.metadata.get("parent_session_id") {
            event = event.with_parent_session_id(parent_session_id.clone());
        }
        if let Some(parent_process_id) = spec
            .metadata
            .get("parent_process_id")
            .and_then(|value| Uuid::parse_str(value).ok())
        {
            event = event.with_parent_process_id(parent_process_id);
        }
        if let Some(wp_id) = wp_id {
            event = event.with_wp_id(wp_id);
        }
        if let Some(mt_id) = spec.metadata.get("mt_id") {
            event = event.with_mt_id(mt_id.clone());
        }
        if let Some(work_profile_id) = spec
            .metadata
            .get("work_profile_id")
            .or_else(|| spec.metadata.get("work_profile_override"))
        {
            event = event.with_work_profile_id(work_profile_id.clone());
        }
        if let Some(model_artifact_sha256) = spec.metadata.get("model_artifact_sha256") {
            event = event.with_model_artifact_sha256(model_artifact_sha256.clone());
        }

        event.started_at = handle.spawned_at_utc;
        event
    }

    fn process_state(
        &self,
        handle: &ProcessHandle,
    ) -> Result<Arc<tokio::sync::Mutex<ProcessStopState>>, SandboxAdapterError> {
        let mut states =
            self.process_states
                .lock()
                .map_err(|error| SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: format!("ledger decorator state poisoned: {error}"),
                })?;
        Ok(states
            .entry(handle.id)
            .or_insert_with(|| {
                Arc::new(tokio::sync::Mutex::new(ProcessStopState {
                    handle: handle.clone(),
                    kill_requests: Vec::new(),
                    terminal: None,
                    observed_exit_code: None,
                }))
            })
            .clone())
    }

    async fn observe_terminal(
        &self,
        handle: &ProcessHandle,
        state: &mut ProcessStopState,
    ) -> Result<ProcessStatus, SandboxAdapterError> {
        if state.handle != *handle {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        let status = match &state.terminal {
            Some(terminal) => terminal.status.clone(),
            None => self.inner.status(handle).await?,
        };
        let (exit_code, exit_code_observed) = match &status {
            ProcessStatus::Exited { code } => (Some(*code), true),
            ProcessStatus::Killed { .. } => (
                state.observed_exit_code,
                state.observed_exit_code.is_some(),
            ),
            _ => return Ok(status),
        };
        let terminal = state.terminal.get_or_insert(TerminalObservation {
            status,
            exit_code,
            exit_code_observed,
        });
        if !terminal.exit_code_observed {
            terminal.exit_code = self.inner.exit_code(handle).await?;
            terminal.exit_code_observed = true;
        }
        self.inner.cleanup_terminal(handle).await?;
        let reason = match &terminal.status {
            ProcessStatus::Exited { .. } => "status:exited".to_string(),
            ProcessStatus::Killed { by_signal } => {
                format!("status:killed:{}", signal_name(*by_signal))
            }
            _ => unreachable!("only observed terminal statuses are retained"),
        };
        self.record_stop_once(handle, terminal.exit_code, reason)?;
        Ok(terminal.status.clone())
    }

    fn record_stop_once(
        &self,
        handle: &ProcessHandle,
        exit_code: Option<i32>,
        stop_reason: impl Into<String>,
    ) -> Result<(), SandboxAdapterError> {
        let mut stopped =
            self.stopped
                .lock()
                .map_err(|error| SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: format!("ledger decorator state poisoned: {error}"),
                })?;
        if stopped.contains(&handle.id) {
            return Ok(());
        }
        let mut pending =
            self.pending_stops
                .lock()
                .map_err(|error| SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: format!("ledger decorator state poisoned: {error}"),
                })?;
        let stop = match pending.get(&handle.id) {
            Some(stop) => stop.clone(),
            None => {
                let retained = self
                    .starts
                    .lock()
                    .map_err(|error| SandboxAdapterError::AdapterUnavailable {
                        adapter_id: handle.adapter_id.clone(),
                        reason: format!("ledger decorator state poisoned: {error}"),
                    })?
                    .get(&handle.id)
                    .cloned();
                let start = retained
                    .or_else(|| self.ledger.recorded_start(handle.id))
                    .ok_or_else(|| SandboxAdapterError::AdapterUnavailable {
                        adapter_id: handle.adapter_id.clone(),
                        reason: "original ledger START unavailable; terminal ownership remains outstanding"
                            .to_string(),
                    })?;
                let stop =
                    ProcessStop::from_start(&start, exit_code).with_stop_reason(stop_reason.into());
                pending.insert(handle.id, stop.clone());
                stop
            }
        };
        let lifecycle = self.lifecycles.lock().unwrap_or_else(|error| error.into_inner())
            .get(&handle.id).cloned();
        if let Some(lifecycle) = lifecycle {
            match lifecycle.stop_with_event(stop).map_err(|error| {
                SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: format!("reserved process ledger STOP failed: {error}"),
                }
            })? {
                StopRecordOutcome::Recorded | StopRecordOutcome::AlreadyStopped => {}
                other => return Err(SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: format!("reserved process STOP remains unconfirmed: {other:?}"),
                }),
            }
        } else {
            self.ledger.record_stop_lossless(stop).map_err(|error| {
                SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: format!("process ledger STOP write failed: {error}"),
                }
            })?;
        }
        stopped.insert(handle.id);
        pending.remove(&handle.id);
        Ok(())
    }
}

#[async_trait]
impl SandboxAdapter for LedgerDecorator {
    fn validate_spawn_acquisition(&self) -> Result<(), SandboxAdapterError> {
        self.inner.validate_spawn_acquisition()
    }

    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        self.acquire_reserved(DecoratorAcquisition::Spawn(spec)).await
    }

    async fn spawn_attached(
        &self,
        spec: AttachedProcessSpec,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        self.inner.spawn_attached(spec).await
    }

    async fn spawn_attached_with_stdio(
        &self,
        spec: AttachedProcessSpec,
        stdio: AttachedStdioContract,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        self.inner.spawn_attached_with_stdio(spec, stdio).await
    }

    fn validate_attached_network_mode(
        &self,
        mode: AttachedNetworkMode,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.validate_attached_network_mode(mode)
    }

    async fn exec(
        &self,
        handle: &ProcessHandle,
        cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        self.inner.exec(handle, cmd).await
    }

    async fn fs_bind(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
        mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        self.inner
            .fs_bind(handle, host_path, guest_path, mode)
            .await
    }

    async fn copy_in(
        &self,
        handle: &ProcessHandle,
        host_path: PathBuf,
        guest_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.copy_in(handle, host_path, guest_path).await
    }

    async fn copy_out(
        &self,
        handle: &ProcessHandle,
        guest_path: PathBuf,
        host_path: PathBuf,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.copy_out(handle, guest_path, host_path).await
    }

    async fn net_policy(
        &self,
        handle: &ProcessHandle,
        policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.net_policy(handle, policy).await
    }

    async fn kill(
        &self,
        handle: &ProcessHandle,
        signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        let process = self.process_state(handle)?;
        let mut state = process.lock().await;
        let status = self.observe_terminal(handle, &mut state).await?;
        if matches!(status, ProcessStatus::Exited { .. } | ProcessStatus::Killed { .. }) {
            return Ok(());
        }
        let request = match state
            .kill_requests
            .iter()
            .position(|(requested, _)| *requested == signal)
        {
            Some(index) => index,
            None => {
                let earlier_attempt = state
                    .kill_requests
                    .iter()
                    .any(|(_, request)| !matches!(request, KillRequestState::NotAttempted));
                if earlier_attempt && !matches!(status, ProcessStatus::Running) {
                    return Err(SandboxAdapterError::AdapterUnavailable {
                        adapter_id: handle.adapter_id.clone(),
                        reason: "a different signal requires an observed running process"
                            .to_string(),
                    });
                }
                state
                    .kill_requests
                    .push((signal, KillRequestState::NotAttempted));
                state.kill_requests.len() - 1
            }
        };
        match &state.kill_requests[request].1 {
            KillRequestState::Acknowledged => return Ok(()),
            KillRequestState::Attempted => {
                return Err(SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: "native signal outcome is pending; no signal was repeated".to_string(),
                });
            }
            KillRequestState::Uncertain { reason } => {
                return Err(SandboxAdapterError::AdapterUnavailable {
                    adapter_id: handle.adapter_id.clone(),
                    reason: format!("native signal outcome remains uncertain: {reason}"),
                });
            }
            KillRequestState::NotAttempted => {}
        }
        state.kill_requests[request].1 = KillRequestState::Attempted;
        match self.inner.kill(handle, signal).await {
            Ok(()) => state.kill_requests[request].1 = KillRequestState::Acknowledged,
            Err(error) => {
                state.kill_requests[request].1 = KillRequestState::Uncertain {
                    reason: error.to_string(),
                };
                return Err(error);
            }
        }
        self.observe_terminal(handle, &mut state).await?;
        Ok(())
    }

    async fn cleanup_terminal(
        &self,
        handle: &ProcessHandle,
    ) -> Result<(), SandboxAdapterError> {
        self.inner.cleanup_terminal(handle).await
    }

    async fn reclaim_detached_observed(
        &self, identity: &super::adapter::DetachedProcessIdentity, signal: Signal,
    ) -> Result<super::adapter::DetachedReclaimOutcome, super::adapter::DetachedReclaimError> {
        self.inner.reclaim_detached_observed(identity, signal).await
    }

    async fn detached_status(
        &self,
        identity: &super::adapter::DetachedProcessIdentity,
    ) -> Result<ProcessStatus, SandboxAdapterError> {
        self.inner.detached_status(identity).await
    }

    fn cached_settled_detached_terminal(
        &self,
        identity: &super::adapter::DetachedProcessIdentity,
    ) -> Result<Option<ProcessStatus>, SandboxAdapterError> {
        self.inner.cached_settled_detached_terminal(identity)
    }

    async fn cleanup_after_restart(
        &self,
        handle: &ProcessHandle,
    ) -> Result<super::adapter::RestartCleanupOutcome, SandboxAdapterError> {
        self.inner.cleanup_after_restart(handle).await
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        let process = self.process_state(handle)?;
        let mut state = process.lock().await;
        self.observe_terminal(handle, &mut state).await
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        let process = self.process_state(handle)?;
        let mut state = process.lock().await;
        if state.handle != *handle {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        if let Some(terminal) = &state.terminal {
            let code = terminal.exit_code;
            self.observe_terminal(handle, &mut state).await?;
            return Ok(code);
        }
        let code = match state.observed_exit_code {
            Some(code) => Some(code),
            None => self.inner.exit_code(handle).await?,
        };
        state.observed_exit_code = code;
        if code.is_some() {
            self.observe_terminal(handle, &mut state).await?;
        }
        Ok(code)
    }

    async fn snapshot(&self, handle: &ProcessHandle) -> Result<SnapshotRef, SandboxAdapterError> {
        // Delegate to the wrapped adapter so snapshot capability is preserved
        // through the ledger decorator (e.g. a wrapped CloudHypervisorAdapter).
        self.inner.snapshot(handle).await
    }

    async fn restore(&self, snapshot: &SnapshotRef) -> Result<ProcessHandle, SandboxAdapterError> {
        self.acquire_reserved(DecoratorAcquisition::Restore(snapshot.clone())).await
    }

    async fn delete_snapshot(&self, snapshot: &SnapshotRef) -> Result<(), SandboxAdapterError> {
        self.inner.delete_snapshot(snapshot).await
    }

    async fn warm_agent_transport(
        &self,
        handle: &ProcessHandle,
    ) -> Result<Arc<dyn WarmAgentTransport>, SandboxAdapterError> {
        self.inner.warm_agent_transport(handle).await
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.inner.capabilities()
    }
}

fn signal_name(signal: Signal) -> &'static str {
    match signal {
        Signal::Term => "term",
        Signal::Kill => "kill",
        Signal::Int => "int",
    }
}

fn capabilities_snapshot(capabilities: &AdapterCapabilities) -> serde_json::Value {
    serde_json::to_value(capabilities).unwrap_or_else(|_| {
        let mut fallback = BTreeMap::new();
        fallback.insert("adapter_id", capabilities.adapter_id.as_str().to_string());
        json!(fallback)
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::process_ledger::{LedgerBatcherConfig, NoopOverflowSink};

    use super::*;
    use crate::sandbox::{
        default_no_op_capabilities, AdapterId, IsolationStrength, IsolationTier, ThroughputClass,
    };

    #[derive(Default)]
    struct RecordingCopyAdapter {
        copied_in: Mutex<Vec<(Uuid, PathBuf, PathBuf)>>,
        copied_out: Mutex<Vec<(Uuid, PathBuf, PathBuf)>>,
    }

    impl RecordingCopyAdapter {
        fn unavailable(&self) -> SandboxAdapterError {
            SandboxAdapterError::AdapterUnavailable {
                adapter_id: AdapterId::new("copy-capable"),
                reason: "recording copy adapter only supports copy operations".to_string(),
            }
        }
    }

    #[async_trait]
    impl SandboxAdapter for RecordingCopyAdapter {
        async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn exec(
            &self,
            _handle: &ProcessHandle,
            _cmd: Command,
        ) -> Result<ExecResult, SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn fs_bind(
            &self,
            _handle: &ProcessHandle,
            _host_path: PathBuf,
            _guest_path: PathBuf,
            _mode: BindMode,
        ) -> Result<(), SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn copy_in(
            &self,
            handle: &ProcessHandle,
            host_path: PathBuf,
            guest_path: PathBuf,
        ) -> Result<(), SandboxAdapterError> {
            self.copied_in
                .lock()
                .expect("copy_in call ledger")
                .push((handle.id, host_path, guest_path));
            Ok(())
        }

        async fn copy_out(
            &self,
            handle: &ProcessHandle,
            guest_path: PathBuf,
            host_path: PathBuf,
        ) -> Result<(), SandboxAdapterError> {
            self.copied_out
                .lock()
                .expect("copy_out call ledger")
                .push((handle.id, guest_path, host_path));
            Ok(())
        }

        async fn net_policy(
            &self,
            _handle: &ProcessHandle,
            _policy: NetPolicy,
        ) -> Result<(), SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn kill(
            &self,
            _handle: &ProcessHandle,
            _signal: Signal,
        ) -> Result<(), SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn status(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<ProcessStatus, SandboxAdapterError> {
            Err(self.unavailable())
        }

        async fn exit_code(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<Option<i32>, SandboxAdapterError> {
            Err(self.unavailable())
        }

        fn capabilities(&self) -> AdapterCapabilities {
            let mut capabilities = default_no_op_capabilities();
            capabilities.adapter_id = AdapterId::new("copy-capable");
            capabilities.runtime_available = true;
            capabilities.filesystem_isolation_strength = IsolationStrength::Strong;
            capabilities.network_isolation_strength = IsolationStrength::Strong;
            capabilities.stdio_throughput_class = ThroughputClass::Medium;
            capabilities.isolation_tier = IsolationTier::Tier1Container;
            capabilities
        }
    }

    fn test_ledger() -> LedgerBatcher {
        let (batcher, _drain) = LedgerBatcher::manual_for_tests(
            LedgerBatcherConfig::default(),
            Arc::new(NoopOverflowSink),
        )
        .expect("manual ledger batcher");
        batcher
    }

    #[tokio::test]
    async fn copy_in_and_copy_out_delegate_through_ledger_decorator() {
        let inner = Arc::new(RecordingCopyAdapter::default());
        let adapter = LedgerDecorator::new(inner.clone(), test_ledger());
        let handle = ProcessHandle::new(AdapterId::new("copy-capable"), None, "copy-handle");

        adapter
            .copy_in(
                &handle,
                PathBuf::from("fixtures/input.txt"),
                PathBuf::from("/guest/input.txt"),
            )
            .await
            .expect("copy_in delegates");
        adapter
            .copy_out(
                &handle,
                PathBuf::from("/guest/output.txt"),
                PathBuf::from("fixtures/output.txt"),
            )
            .await
            .expect("copy_out delegates");

        assert_eq!(
            *inner.copied_in.lock().expect("copy_in calls"),
            vec![(
                handle.id,
                PathBuf::from("fixtures/input.txt"),
                PathBuf::from("/guest/input.txt")
            )]
        );
        assert_eq!(
            *inner.copied_out.lock().expect("copy_out calls"),
            vec![(
                handle.id,
                PathBuf::from("/guest/output.txt"),
                PathBuf::from("fixtures/output.txt")
            )]
        );
    }
}
