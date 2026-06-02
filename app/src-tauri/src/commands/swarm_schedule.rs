//! TRACK 1 -- calendar app-wiring for the rank-7 swarm scheduler.
//!
//! The rank-7 schedule CORE (`handshake_core::swarm_orchestration::schedule`)
//! ships `SwarmSchedule`, `ScheduledAction`, `SwarmScheduler` (tokio-cron), and
//! `schedules_to_ics`, but it only fires an INJECTED callback -- nothing in the
//! deployed app constructs the scheduler or wires that callback to the live
//! `SwarmCoordinator`. This module is that wiring:
//!
//! 1. [`SwarmSchedulerState`] -- managed app state holding the `SwarmScheduler`,
//!    the registered schedules + their spawn TEMPLATES (keyed by schedule id),
//!    and the live coordinator, all behind a `Mutex`. Built + started in
//!    production wiring (`lib.rs` `setup`).
//! 2. The `on_fire` callback -- `SpinUp` builds a `SpawnRequest` from the stored
//!    template + `.with_time_box(time_box)` + `.with_swarm(swarm_id)` and calls
//!    `coordinator.spawn_session` on the Tokio runtime; `Teardown` cancels the
//!    swarm's live sessions. Each fire EMITS the matching `FR-EVT-SWARM-SCHED-*`
//!    via a direct Flight-Recorder write (the schedule fires are NOT coordinator
//!    `SwarmEvent`s, so they take the recorder path, not the coordinator sink).
//! 3. Tauri commands (pub, NOT registered -- the Integrate phase owns lib.rs):
//!    [`kernel_swarm_schedule_add`], [`kernel_swarm_schedule_list`],
//!    [`kernel_swarm_schedule_remove`], [`kernel_swarm_schedule_export_ics`].
//! 4. Persistence -- registered schedules + templates persist to a JSON file
//!    under `app_data_root` (see [`super::swarm_schedule_store`]); on startup the
//!    app LOADS them, RE-ARMS the scheduler, and applies a SKIP catch-up policy
//!    for fires missed while the app was closed (an emitted FR note surfaces the
//!    skip rather than silently running a stale fire late).
//!
//! ## Why schedule fires take the recorder path, not the coordinator sink
//!
//! `SwarmEventSink`/`SwarmEvent` is the coordinator's session-lifecycle channel;
//! the rank-3 `FR-EVT-SWARM-SCHED-*` ids are NOT members of `SwarmEvent`. A
//! schedule fire is an event ABOUT the scheduler, emitted before/around the
//! coordinator call, so it is written straight to the durable `FlightRecorder`
//! (the same recorder `lib.rs` hands the swarm runtime). When the resulting
//! `spawn_session` succeeds it independently emits the session-lifecycle
//! `SwarmEvent`s through the coordinator's own sink -- the two are complementary.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use handshake_core::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};
use handshake_core::model_runtime::registry::RuntimeBinding;
use handshake_core::model_runtime::{ModelId, ProviderKind};
use handshake_core::swarm_orchestration::ids::LocalExecutionMode;
use handshake_core::swarm_orchestration::{
    schedules_to_ics, ModelInstanceId, ScheduledAction, ScheduledFire, SpawnRequest,
    SwarmCoordinator, SwarmFrEventId, SwarmSchedule, SwarmScheduler,
};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use super::swarm_schedule_store::{
    PersistedAction, RegisteredSchedule, SpawnTemplate, SwarmScheduleDoc, SwarmScheduleStore,
    TemplateIsolationTier, TemplateLocalExecutionMode, TemplateProvider, TemplateRuntimeBinding,
};

pub const KERNEL_SWARM_SCHEDULE_ADD_IPC_CHANNEL: &str = "kernel_swarm_schedule_add";
pub const KERNEL_SWARM_SCHEDULE_LIST_IPC_CHANNEL: &str = "kernel_swarm_schedule_list";
pub const KERNEL_SWARM_SCHEDULE_REMOVE_IPC_CHANNEL: &str = "kernel_swarm_schedule_remove";
pub const KERNEL_SWARM_SCHEDULE_EXPORT_ICS_IPC_CHANNEL: &str = "kernel_swarm_schedule_export_ics";

/// FR-EVT tag stamped on the stderr fallback line when no durable recorder is
/// wired, so a schedule fire is still observable in logs.
pub const FR_EVT_SWARM_SCHED: &str = "FR-EVT-SWARM-SCHED";

// ---------------------------------------------------------------------------
// The fire handler: turns a ScheduledFire into a real coordinator call + FR note
// ---------------------------------------------------------------------------

/// Everything the `on_fire` callback needs, captured once when the scheduler is
/// built. Cloned into the callback closure. Drives REAL coordinator calls.
#[derive(Clone)]
struct FireContext {
    coordinator: Arc<SwarmCoordinator>,
    /// Tokio runtime handle: the `on_fire` callback is SYNCHRONOUS (the core
    /// `ScheduleFireFn` is `Fn`), but `spawn_session` / `cancel_session` are
    /// async. We `handle.spawn(...)` the coordinator call onto the live runtime.
    runtime: tokio::runtime::Handle,
    /// Spawn templates keyed by schedule id. A `SpinUp` fire looks its template
    /// up here to know WHAT to launch. Shared with the managed state so an
    /// add/remove is reflected in subsequent fires.
    templates: Arc<Mutex<BTreeMap<String, SpawnTemplate>>>,
    /// Durable recorder for the `FR-EVT-SWARM-SCHED-*` fire notes. `None` falls
    /// back to a structured stderr line (mirrors the swarm runtime's fallback).
    recorder: Option<Arc<dyn FlightRecorder>>,
    trace_id: Uuid,
    /// Instances spawned by scheduled `SpinUp` fires, grouped by `swarm_id`, so a
    /// later `Teardown` fire cancels exactly the sessions THIS scheduler launched
    /// for that swarm. The coordinator exposes no per-swarm instance iterator and
    /// is off-limits to edit this wave, so the scheduler owns this attribution
    /// table itself. Entries are reconciled (dropped) when a session is no longer
    /// live, so the table cannot leak.
    scheduled_instances: Arc<Mutex<BTreeMap<String, Vec<ModelInstanceId>>>>,
}

impl FireContext {
    /// Handle one scheduled fire: emit the FR note, then drive the coordinator.
    fn handle(&self, fire: ScheduledFire) {
        // Mint a FRESH correlation id per fire so the Flight Recorder can
        // distinguish individual fires of the same schedule. The synchronous
        // fire event and its async follow-up note (spawned/cancelled) for THIS
        // fire share this one id, so they correlate as a single causal chain;
        // the next fire of the same schedule gets a new id. (A single id reused
        // across every fire would collapse all fires into one correlation.)
        let trace_id = Uuid::now_v7();
        match &fire.action {
            ScheduledAction::SpinUp { swarm_id, time_box } => {
                self.emit_fr(
                    trace_id,
                    SwarmFrEventId::ScheduledSpinupFired,
                    &fire.schedule_id,
                    swarm_id,
                    *time_box,
                );
                let template = self
                    .templates
                    .lock()
                    .expect("schedule templates poisoned")
                    .get(&fire.schedule_id)
                    .cloned();
                let Some(template) = template else {
                    // A SpinUp with no stored template cannot launch anything;
                    // surface the gap rather than silently dropping the fire.
                    self.emit_note(
                        trace_id,
                        &fire.schedule_id,
                        swarm_id,
                        "spinup_missing_template",
                    );
                    return;
                };
                let spawn = match build_spawn_request(&template, swarm_id, *time_box) {
                    Ok(spawn) => spawn,
                    Err(reason) => {
                        self.emit_note(trace_id, &fire.schedule_id, swarm_id, &reason);
                        return;
                    }
                };
                let coordinator = self.coordinator.clone();
                let schedule_id = fire.schedule_id.clone();
                let swarm = swarm_id.clone();
                let recorder = self.recorder.clone();
                let scheduled_instances = self.scheduled_instances.clone();
                // Spawn the async coordinator call onto the live runtime. The
                // fire callback returns immediately; the spawn happens for real.
                self.runtime.spawn(async move {
                    match coordinator.spawn_session(spawn).await {
                        Ok(iid) => {
                            // Record the spawned instance under its swarm so a
                            // later Teardown fire can cancel exactly it.
                            scheduled_instances
                                .lock()
                                .expect("scheduled instances poisoned")
                                .entry(swarm.clone())
                                .or_default()
                                .push(iid);
                            emit_fr_note(
                                &recorder,
                                trace_id,
                                &schedule_id,
                                &swarm,
                                "spinup_spawned",
                                Some(iid.to_string()),
                            );
                        }
                        Err(error) => {
                            emit_fr_note(
                                &recorder,
                                trace_id,
                                &schedule_id,
                                &swarm,
                                &format!("spinup_spawn_failed: {error}"),
                                None,
                            );
                        }
                    }
                });
            }
            ScheduledAction::Teardown { swarm_id } => {
                self.emit_fr(
                    trace_id,
                    SwarmFrEventId::ScheduledTeardownFired,
                    &fire.schedule_id,
                    swarm_id,
                    None,
                );
                let coordinator = self.coordinator.clone();
                let schedule_id = fire.schedule_id.clone();
                let swarm = swarm_id.clone();
                let recorder = self.recorder.clone();
                let scheduled_instances = self.scheduled_instances.clone();
                self.runtime.spawn(async move {
                    let cancelled =
                        cancel_swarm_sessions(&coordinator, &swarm, &scheduled_instances).await;
                    emit_fr_note(
                        &recorder,
                        trace_id,
                        &schedule_id,
                        &swarm,
                        &format!("teardown_cancelled_{cancelled}_sessions"),
                        None,
                    );
                });
            }
        }
    }

    /// Emit the canonical `FR-EVT-SWARM-SCHED-*` fire event under the per-fire
    /// correlation id.
    fn emit_fr(
        &self,
        trace_id: Uuid,
        id: SwarmFrEventId,
        schedule_id: &str,
        swarm_id: &str,
        time_box: Option<Duration>,
    ) {
        let payload = serde_json::json!({
            "fr_event_id": id.as_str(),
            "schedule_id": schedule_id,
            "swarm_id": swarm_id,
            "time_box_secs": time_box.map(|d| d.as_secs()),
        });
        record_or_stderr(&self.recorder, trace_id, id.as_str(), payload);
    }

    /// Emit a follow-up note (not one of the two canonical fire ids) so an
    /// in-fire condition (missing template, build error) is observable. Shares
    /// the per-fire `trace_id` so it correlates with the fire that raised it.
    fn emit_note(&self, trace_id: Uuid, schedule_id: &str, swarm_id: &str, note: &str) {
        emit_fr_note(&self.recorder, trace_id, schedule_id, swarm_id, note, None);
    }
}

/// Cancel every live session under `swarm_id` — including sessions spawned
/// manually, not just by this scheduler. Returns how many were actually
/// cancelled.
///
/// The authoritative source is the coordinator's read-only
/// `live_instances_in_swarm(swarm_id)` accessor, which enumerates ALL live
/// instances registered under the swarm. We additionally drain the scheduler's
/// own attribution table for this swarm and union in any recorded id the
/// snapshot missed (covers the race where a just-spawned instance is recorded
/// by `SpinUp` but a marginally-stale snapshot has not observed it yet); the
/// drain also guarantees the table cannot leak. Each target is cancelled via
/// the coordinator's real `cancel_session` path (token cancel + teardown +
/// ledger STOP + evict). A `Teardown` schedule is thus the explicit complement
/// to a `SpinUp`'s time-box reaping. Instances already terminal/reaped are
/// skipped (re-checked via `session_state`), so no dead id is re-cancelled.
async fn cancel_swarm_sessions(
    coordinator: &Arc<SwarmCoordinator>,
    swarm_id: &str,
    scheduled_instances: &Arc<Mutex<BTreeMap<String, Vec<ModelInstanceId>>>>,
) -> usize {
    // Authoritative: every live session in this swarm, manually-spawned included.
    let mut instances: Vec<ModelInstanceId> = coordinator.live_instances_in_swarm(swarm_id);
    {
        // Drain this swarm's recorded instances (prevents the table from
        // leaking) and union any not already in the authoritative snapshot.
        let mut table = scheduled_instances
            .lock()
            .expect("scheduled instances poisoned");
        if let Some(recorded) = table.remove(swarm_id) {
            for iid in recorded {
                if !instances.contains(&iid) {
                    instances.push(iid);
                }
            }
        }
    }
    let mut cancelled = 0usize;
    for iid in instances {
        // Skip instances the coordinator has already evicted (reaped via the
        // time-box, or cancelled manually): no spurious cancel of a dead id.
        match coordinator.session_state(iid) {
            Some(state) if !state.is_terminal() => {
                if coordinator
                    .cancel_session(iid, "calendar_schedule_teardown")
                    .await
                    .is_ok()
                {
                    cancelled += 1;
                }
            }
            _ => {}
        }
    }
    cancelled
}

/// Build a `SpawnRequest` from a stored template, applying the fire's
/// `swarm_id` (`with_swarm`) and `time_box` (`with_time_box`). Validates the
/// provider-appropriate fields, mirroring the manual spawn command's contract.
fn build_spawn_request(
    template: &SpawnTemplate,
    swarm_id: &str,
    time_box: Option<Duration>,
) -> Result<SpawnRequest, String> {
    let model_id = ModelId::new_v7();
    let instance_id =
        handshake_core::swarm_orchestration::ModelInstanceId::new(model_id, template.instance);
    let parent = template
        .parent_session_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("calendar-schedule")
        .to_string();

    let mut spawn = match template.provider {
        TemplateProvider::Local => {
            let binding = match template.runtime_binding {
                Some(TemplateRuntimeBinding::Candle) => RuntimeBinding::Candle,
                Some(TemplateRuntimeBinding::LlamaCpp) => RuntimeBinding::LlamaCpp,
                None => {
                    return Err(
                        "local template requires a runtime_binding (candle | llama_cpp)"
                            .to_string(),
                    )
                }
            };
            let path = template
                .artifact_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "local template requires a non-empty artifact_path".to_string())?;
            let sha = template
                .sha256_expected
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    "local template requires sha256_expected (the integrity gate)".to_string()
                })?;
            let mut spawn = SpawnRequest::new(instance_id, binding, "operator", parent)
                .with_local_artifact(path.to_string(), sha.to_string());
            match template.local_execution_mode {
                Some(TemplateLocalExecutionMode::WarmVm) => {
                    validate_warm_template(template, binding)?;
                    spawn = spawn.with_warm_vm_execution();
                }
                Some(TemplateLocalExecutionMode::Cold) => {
                    spawn.local_execution_mode = Some(LocalExecutionMode::Cold);
                }
                None => {}
            }
            spawn
        }
        TemplateProvider::ByokCloud | TemplateProvider::OfficialCli => {
            if template.local_execution_mode.is_some() {
                return Err("local_execution_mode is only valid for local templates".to_string());
            }
            let model_name = template
                .cloud_model_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    "cloud template requires a non-empty cloud_model_name".to_string()
                })?;
            let provider = match template.provider {
                TemplateProvider::ByokCloud => ProviderKind::ByokCloud,
                TemplateProvider::OfficialCli => ProviderKind::OfficialCli,
                TemplateProvider::Local => unreachable!(),
            };
            SpawnRequest::new(instance_id, RuntimeBinding::Candle, "operator", parent)
                .with_cloud_provider(provider, model_name.to_string())
        }
    };

    // The orchestrator's spawn-template decision applied: the FIRE supplies the
    // swarm grouping + the time-box (so the existing reaper reclaims the
    // time-boxed scheduled session -- no new teardown path).
    spawn = spawn.with_swarm(swarm_id.to_string());
    if let Some(tb) = time_box {
        spawn = spawn.with_time_box(tb);
    }
    if let Some(worktree) = template
        .worktree_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        spawn = spawn.with_worktree(worktree.to_string());
    }
    if let Some(tier) = template.isolation_tier {
        spawn = spawn.with_isolation_tier(template_isolation_to_core(tier));
    }
    if matches!(template.provider, TemplateProvider::Local) {
        if let Some(bytes) = template.committed_memory_bytes.filter(|bytes| *bytes > 0) {
            spawn = spawn.with_committed_memory_bytes(bytes);
        }
    }
    Ok(spawn)
}

fn validate_schedule_template_for_registration(template: &SpawnTemplate) -> Result<(), String> {
    match template.provider {
        TemplateProvider::Local => {
            if matches!(
                template.local_execution_mode,
                Some(TemplateLocalExecutionMode::WarmVm)
            ) {
                let binding = match template.runtime_binding {
                    Some(TemplateRuntimeBinding::LlamaCpp) => RuntimeBinding::LlamaCpp,
                    Some(TemplateRuntimeBinding::Candle) | None => RuntimeBinding::Candle,
                };
                validate_warm_template(template, binding)?;
            }
        }
        TemplateProvider::ByokCloud | TemplateProvider::OfficialCli => {
            if template.local_execution_mode.is_some() {
                return Err("local_execution_mode is only valid for local templates".to_string());
            }
        }
    }
    Ok(())
}

fn validate_warm_template(template: &SpawnTemplate, binding: RuntimeBinding) -> Result<(), String> {
    if binding != RuntimeBinding::LlamaCpp {
        return Err("warm_vm scheduled spin-up requires runtime_binding=llama_cpp".to_string());
    }
    if template.isolation_tier != Some(TemplateIsolationTier::Tier3Microvm) {
        return Err("warm_vm scheduled spin-up requires isolation_tier=tier3_microvm".to_string());
    }
    let has_worktree = template
        .worktree_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    if !has_worktree {
        return Err("warm_vm scheduled spin-up requires a non-empty worktree_id".to_string());
    }
    Ok(())
}

fn template_isolation_to_core(
    tier: TemplateIsolationTier,
) -> handshake_core::sandbox::adapter::IsolationTier {
    use handshake_core::sandbox::adapter::IsolationTier;
    match tier {
        TemplateIsolationTier::Tier1Container => IsolationTier::Tier1Container,
        TemplateIsolationTier::Tier2Syscall => IsolationTier::Tier2Syscall,
        TemplateIsolationTier::Tier3Microvm => IsolationTier::Tier3Microvm,
    }
}

/// Record an `FR-EVT-SWARM-SCHED-*` event into the durable recorder, or emit a
/// structured stderr line when no recorder is wired.
fn record_or_stderr(
    recorder: &Option<Arc<dyn FlightRecorder>>,
    trace_id: Uuid,
    fr_id: &str,
    payload: serde_json::Value,
) {
    match recorder {
        Some(rec) => {
            let event = FlightRecorderEvent::new(
                FlightRecorderEventType::System,
                FlightRecorderActor::System,
                trace_id,
                payload,
            );
            let rec = rec.clone();
            // record_event is async + fallible; the fire path is sync. Spawn the
            // write onto the current runtime if one is present, else block-in-
            // place is unsafe here, so fall back to stderr on no-runtime.
            match tokio::runtime::Handle::try_current() {
                Ok(handle) => {
                    handle.spawn(async move {
                        if let Err(error) = rec.record_event(event).await {
                            eprintln!("{FR_EVT_SWARM_SCHED}: recorder write failed: {error}");
                        }
                    });
                }
                Err(_) => {
                    eprintln!(
                        "{FR_EVT_SWARM_SCHED}: {fr_id} {}",
                        serde_json::to_string(&event.payload).unwrap_or_default()
                    );
                }
            }
        }
        None => {
            eprintln!("{FR_EVT_SWARM_SCHED}: {fr_id} {payload}");
        }
    }
}

/// Emit a free-form schedule note (a non-canonical follow-up to a fire).
fn emit_fr_note(
    recorder: &Option<Arc<dyn FlightRecorder>>,
    trace_id: Uuid,
    schedule_id: &str,
    swarm_id: &str,
    note: &str,
    instance_id: Option<String>,
) {
    let payload = serde_json::json!({
        "fr_event_id": "FR-EVT-SWARM-SCHED-NOTE",
        "schedule_id": schedule_id,
        "swarm_id": swarm_id,
        "note": note,
        "instance_id": instance_id,
    });
    record_or_stderr(recorder, trace_id, "FR-EVT-SWARM-SCHED-NOTE", payload);
}

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

/// Managed app state holding the live `SwarmScheduler`, the registered
/// schedules + their spawn templates, the persistence store, and everything the
/// fire callback needs. All mutable parts are behind a `Mutex` (the scheduler
/// itself is internally async + `Send`).
pub struct SwarmSchedulerState {
    /// The live tokio-cron scheduler behind a `tokio::sync::Mutex` (NOT
    /// `std::sync::Mutex`): every scheduler operation (`add` / `start` /
    /// `shutdown`) is async, and a `tokio::sync::MutexGuard` is `Send`, so it may
    /// be held across the `.await`. A `std::sync::MutexGuard` is `!Send` and would
    /// make the Tauri command futures non-`Send` (Tauri requires `Send`
    /// command futures). The sync bookkeeping (`registered`, `started`) stays in
    /// the `std::sync::Mutex<SchedulerInner>` below and is NEVER held across an
    /// `.await`.
    scheduler: tokio::sync::Mutex<SwarmScheduler>,
    inner: Mutex<SchedulerInner>,
    /// Shared template map the fire callback reads. Kept OUTSIDE the inner mutex
    /// so the (sync) fire callback never contends with an add/remove holding the
    /// scheduler lock across an `.await`.
    templates: Arc<Mutex<BTreeMap<String, SpawnTemplate>>>,
    store: SwarmScheduleStore,
    fire_ctx: FireContext,
}

struct SchedulerInner {
    /// Registered schedules keyed by id (the durable set, mirrored to disk).
    registered: BTreeMap<String, RegisteredSchedule>,
    started: bool,
}

impl std::fmt::Debug for SwarmSchedulerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwarmSchedulerState")
            .field("registered_count", &self.registered_count())
            .field("store", &self.store.path())
            .finish()
    }
}

impl SwarmSchedulerState {
    /// Build the scheduler state bound to the live coordinator + the persistence
    /// store under `app_data_root`. Does NOT start ticking or load yet; call
    /// [`Self::bootstrap`] inside the Tokio runtime to load + re-arm + start.
    pub async fn new(
        coordinator: Arc<SwarmCoordinator>,
        store: SwarmScheduleStore,
        recorder: Option<Arc<dyn FlightRecorder>>,
        runtime: tokio::runtime::Handle,
    ) -> Result<Self, String> {
        let scheduler = SwarmScheduler::new()
            .await
            .map_err(|error| format!("failed to build swarm scheduler: {error}"))?;
        let templates: Arc<Mutex<BTreeMap<String, SpawnTemplate>>> =
            Arc::new(Mutex::new(BTreeMap::new()));
        let fire_ctx = FireContext {
            coordinator,
            runtime,
            templates: templates.clone(),
            recorder,
            trace_id: Uuid::now_v7(),
            scheduled_instances: Arc::new(Mutex::new(BTreeMap::new())),
        };
        Ok(Self {
            scheduler: tokio::sync::Mutex::new(scheduler),
            inner: Mutex::new(SchedulerInner {
                registered: BTreeMap::new(),
                started: false,
            }),
            templates,
            store,
            fire_ctx,
        })
    }

    /// PRODUCTION wiring one-shot: build the scheduler bound to the live
    /// coordinator + the store under `app_data_root` + the durable recorder,
    /// then LOAD persisted schedules, RE-ARM them, apply the SKIP catch-up
    /// policy, and START ticking. The Integrate phase calls this from the Tauri
    /// `setup` block (inside the async runtime) and `.manage()`s the result:
    ///
    /// ```ignore
    /// let scheduler = commands::swarm_schedule::SwarmSchedulerState::production_bootstrap(
    ///     swarm_state.coordinator(),          // the SAME managed coordinator
    ///     &app_data_root,                     // disk-agnostic store root
    ///     swarm_recorder.clone(),             // durable FR-EVT-SWARM-SCHED-* sink
    /// ).await?;
    /// app.manage(scheduler);
    /// ```
    ///
    /// Built with `tokio::runtime::Handle::current()` so scheduled fires spawn
    /// onto the live Tauri async runtime. Must be called from within the runtime.
    pub async fn production_bootstrap(
        coordinator: Arc<SwarmCoordinator>,
        app_data_root: impl AsRef<std::path::Path>,
        recorder: Option<Arc<dyn FlightRecorder>>,
    ) -> Result<Self, String> {
        let store = SwarmScheduleStore::new(app_data_root);
        let state = Self::new(
            coordinator,
            store,
            recorder,
            tokio::runtime::Handle::current(),
        )
        .await?;
        state.bootstrap().await?;
        Ok(state)
    }

    /// Load persisted schedules, RE-ARM them on the scheduler, apply the SKIP
    /// catch-up policy (emit an FR note for the closed window), then START the
    /// scheduler. Idempotent on `started`.
    pub async fn bootstrap(&self) -> Result<(), String> {
        let doc = self.store.load()?;
        let now = Utc::now();

        for reg in &doc.schedules {
            // Re-arm: add the cron job back with the fire callback.
            self.arm_schedule(reg).await?;
            // SKIP catch-up policy: the cron scheduler only fires FUTURE
            // occurrences, so any occurrence due while the app was closed is
            // skipped. Surface that skip with an emitted FR note (run-once-late
            // is deliberately NOT chosen -- a stale spin-up running hours late is
            // worse than skipping it). The note records the closed-window bound.
            self.emit_catch_up_skip(reg, now);
        }

        // Decide whether to start under the sync lock, then release it BEFORE the
        // async `start()` (a std MutexGuard must never cross an `.await`).
        let needs_start = {
            let inner = self.inner.lock().expect("scheduler state poisoned");
            !inner.started
        };
        if needs_start {
            self.scheduler
                .lock()
                .await
                .start()
                .await
                .map_err(|error| format!("failed to start swarm scheduler: {error}"))?;
            self.inner.lock().expect("scheduler state poisoned").started = true;
        }
        Ok(())
    }

    /// Arm one registered schedule on the live scheduler + record its template.
    /// Does NOT persist (the caller batches persistence).
    async fn arm_schedule(&self, reg: &RegisteredSchedule) -> Result<(), String> {
        validate_schedule_template_for_registration(&reg.template)?;
        let schedule = reg.to_swarm_schedule();
        let ctx = self.fire_ctx.clone();
        let on_fire: handshake_core::swarm_orchestration::schedule::ScheduleFireFn =
            Arc::new(move |fire: ScheduledFire| ctx.handle(fire));
        // The async scheduler lock is `tokio::sync::Mutex` so the guard is `Send`
        // and may legally be held across the `.await` below.
        self.scheduler
            .lock()
            .await
            .add(schedule, on_fire)
            .await
            .map_err(|error| format!("invalid cron '{}': {error}", reg.cron))?;
        self.templates
            .lock()
            .expect("schedule templates poisoned")
            .insert(reg.id.clone(), reg.template.clone());
        self.inner
            .lock()
            .expect("scheduler state poisoned")
            .registered
            .insert(reg.id.clone(), reg.clone());
        Ok(())
    }

    /// Emit the SKIP catch-up FR note for a re-armed schedule.
    fn emit_catch_up_skip(&self, reg: &RegisteredSchedule, now: DateTime<Utc>) {
        let payload = serde_json::json!({
            "fr_event_id": "FR-EVT-SWARM-SCHED-CATCHUP-SKIP",
            "schedule_id": reg.id,
            "swarm_id": reg.action.swarm_id(),
            "cron": reg.cron,
            "policy": "skip",
            "registered_at": reg.registered_at.to_rfc3339(),
            "rearmed_at": now.to_rfc3339(),
            "detail": "fires due while the app was closed are skipped (not run late)",
        });
        record_or_stderr(
            &self.fire_ctx.recorder,
            self.fire_ctx.trace_id,
            "FR-EVT-SWARM-SCHED-CATCHUP-SKIP",
            payload,
        );
    }

    /// Register a NEW schedule: arm it, record its template, and PERSIST the
    /// updated set. Rejects a duplicate id.
    pub async fn add(
        &self,
        schedule: SwarmSchedule,
        template: SpawnTemplate,
    ) -> Result<(), String> {
        {
            let inner = self.inner.lock().expect("scheduler state poisoned");
            if inner.registered.contains_key(&schedule.id) {
                return Err(format!("schedule id '{}' already registered", schedule.id));
            }
        }
        let reg = RegisteredSchedule::new(&schedule, template);
        self.arm_schedule(&reg).await?;
        self.persist()?;
        Ok(())
    }

    /// List the registered schedules (id, cron, summary, action, template) in
    /// stable id order.
    pub fn list(&self) -> Vec<RegisteredSchedule> {
        self.inner
            .lock()
            .expect("scheduler state poisoned")
            .registered
            .values()
            .cloned()
            .collect()
    }

    /// Remove a registered schedule. Because `tokio-cron-scheduler` jobs added
    /// through the core `SwarmScheduler::add` are not individually addressable
    /// from this layer, removal REBUILDS the scheduler from the surviving set so
    /// the removed cron truly stops firing (no zombie job). Persists the result.
    pub async fn remove(&self, schedule_id: &str) -> Result<(), String> {
        let survivors: Vec<RegisteredSchedule> = {
            let mut inner = self.inner.lock().expect("scheduler state poisoned");
            if inner.registered.remove(schedule_id).is_none() {
                return Err(format!("no schedule registered with id '{schedule_id}'"));
            }
            inner.registered.values().cloned().collect()
        };
        self.templates
            .lock()
            .expect("schedule templates poisoned")
            .remove(schedule_id);
        // Rebuild a fresh scheduler with only the survivors so the removed job
        // stops firing (the old scheduler is shut down + replaced).
        self.rebuild(survivors).await?;
        self.persist()?;
        Ok(())
    }

    /// Replace the live scheduler with a fresh one armed only with `survivors`.
    /// Shuts the old one down so its jobs stop. Re-arms each survivor.
    async fn rebuild(&self, survivors: Vec<RegisteredSchedule>) -> Result<(), String> {
        let fresh = SwarmScheduler::new()
            .await
            .map_err(|error| format!("failed to rebuild swarm scheduler: {error}"))?;
        // Read + reset the sync bookkeeping under the std lock, dropping the guard
        // BEFORE any `.await`.
        let was_started = {
            let mut inner = self.inner.lock().expect("scheduler state poisoned");
            let started = inner.started;
            inner.registered.clear();
            inner.started = false;
            started
        };
        // Swap the live scheduler for the fresh one under the async lock (the
        // guard is `Send`, so it may be held across the shutdown `.await`), then
        // best-effort shut the old one's background task down.
        {
            let mut scheduler = self.scheduler.lock().await;
            let mut old = std::mem::replace(&mut *scheduler, fresh);
            let _ = old.shutdown().await;
        }
        // Re-arm survivors on the fresh scheduler.
        for reg in &survivors {
            self.arm_schedule(reg).await?;
        }
        if was_started {
            self.scheduler
                .lock()
                .await
                .start()
                .await
                .map_err(|error| format!("failed to restart swarm scheduler: {error}"))?;
            self.inner.lock().expect("scheduler state poisoned").started = true;
        }
        Ok(())
    }

    /// Export the registered schedules as an RFC-5545 ICS calendar.
    pub fn export_ics(&self) -> String {
        let schedules: Vec<SwarmSchedule> = self
            .inner
            .lock()
            .expect("scheduler state poisoned")
            .registered
            .values()
            .map(|reg| reg.to_swarm_schedule())
            .collect();
        schedules_to_ics(&schedules, Utc::now())
    }

    /// Number of registered schedules.
    pub fn registered_count(&self) -> usize {
        self.inner
            .lock()
            .expect("scheduler state poisoned")
            .registered
            .len()
    }

    /// Persist the current registered set to disk atomically.
    fn persist(&self) -> Result<(), String> {
        let doc = SwarmScheduleDoc {
            schema_version: super::swarm_schedule_store::SWARM_SCHEDULES_SCHEMA_VERSION,
            schedules: self
                .inner
                .lock()
                .expect("scheduler state poisoned")
                .registered
                .values()
                .cloned()
                .collect(),
        };
        self.store.save(&doc)
    }
}

// ---------------------------------------------------------------------------
// IPC DTOs
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleProviderIpc {
    Local,
    ByokCloud,
    OfficialCli,
}

impl From<ScheduleProviderIpc> for TemplateProvider {
    fn from(value: ScheduleProviderIpc) -> Self {
        match value {
            ScheduleProviderIpc::Local => TemplateProvider::Local,
            ScheduleProviderIpc::ByokCloud => TemplateProvider::ByokCloud,
            ScheduleProviderIpc::OfficialCli => TemplateProvider::OfficialCli,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleRuntimeBindingIpc {
    Candle,
    LlamaCpp,
}

impl From<ScheduleRuntimeBindingIpc> for TemplateRuntimeBinding {
    fn from(value: ScheduleRuntimeBindingIpc) -> Self {
        match value {
            ScheduleRuntimeBindingIpc::Candle => TemplateRuntimeBinding::Candle,
            ScheduleRuntimeBindingIpc::LlamaCpp => TemplateRuntimeBinding::LlamaCpp,
        }
    }
}

/// The spawn TEMPLATE from the calendar form: WHAT a scheduled spin-up launches.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnTemplateIpc {
    pub provider: ScheduleProviderIpc,
    #[serde(default)]
    pub artifact_path: Option<String>,
    #[serde(default)]
    pub sha256_expected: Option<String>,
    #[serde(default)]
    pub runtime_binding: Option<ScheduleRuntimeBindingIpc>,
    #[serde(default)]
    pub local_execution_mode: Option<TemplateLocalExecutionMode>,
    #[serde(default)]
    pub cloud_model_name: Option<String>,
    #[serde(default)]
    pub instance: u32,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub isolation_tier: Option<TemplateIsolationTier>,
    #[serde(default)]
    pub committed_memory_bytes: Option<u64>,
    #[serde(default)]
    pub parent_session_id: Option<String>,
}

impl From<SpawnTemplateIpc> for SpawnTemplate {
    fn from(value: SpawnTemplateIpc) -> Self {
        SpawnTemplate {
            provider: value.provider.into(),
            artifact_path: value.artifact_path,
            sha256_expected: value.sha256_expected,
            runtime_binding: value.runtime_binding.map(Into::into),
            local_execution_mode: value.local_execution_mode,
            cloud_model_name: value.cloud_model_name,
            instance: value.instance,
            worktree_id: value.worktree_id,
            isolation_tier: value.isolation_tier,
            committed_memory_bytes: value.committed_memory_bytes,
            parent_session_id: value.parent_session_id,
        }
    }
}

/// The action a schedule fires: spin-up (with optional time-box) or teardown.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ScheduledActionIpc {
    SpinUp {
        swarm_id: String,
        #[serde(default)]
        time_box_secs: Option<u64>,
    },
    Teardown {
        swarm_id: String,
    },
}

impl ScheduledActionIpc {
    fn to_core(&self) -> ScheduledAction {
        match self {
            ScheduledActionIpc::SpinUp {
                swarm_id,
                time_box_secs,
            } => ScheduledAction::SpinUp {
                swarm_id: swarm_id.clone(),
                time_box: time_box_secs.map(Duration::from_secs),
            },
            ScheduledActionIpc::Teardown { swarm_id } => ScheduledAction::Teardown {
                swarm_id: swarm_id.clone(),
            },
        }
    }
}

/// Add-schedule request from the calendar form: the schedule (id + cron +
/// summary + action) plus the spawn template.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScheduleRequestIpc {
    pub id: String,
    pub cron: String,
    pub summary: String,
    pub action: ScheduledActionIpc,
    pub template: SpawnTemplateIpc,
}

/// A registered schedule row returned to the calendar view.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredScheduleIpc {
    pub id: String,
    pub cron: String,
    pub summary: String,
    /// `spin_up` or `teardown`.
    pub action_kind: String,
    pub swarm_id: String,
    pub time_box_secs: Option<u64>,
    pub provider: String,
    pub artifact_path: Option<String>,
    pub cloud_model_name: Option<String>,
    pub worktree_id: Option<String>,
    pub local_execution_mode: Option<TemplateLocalExecutionMode>,
    pub isolation_tier: Option<TemplateIsolationTier>,
    pub committed_memory_bytes: Option<u64>,
    pub registered_at: String,
}

impl From<RegisteredSchedule> for RegisteredScheduleIpc {
    fn from(reg: RegisteredSchedule) -> Self {
        let (action_kind, time_box_secs) = match &reg.action {
            PersistedAction::SpinUp { time_box_secs, .. } => ("spin_up", *time_box_secs),
            PersistedAction::Teardown { .. } => ("teardown", None),
        };
        let provider = match reg.template.provider {
            TemplateProvider::Local => "local",
            TemplateProvider::ByokCloud => "byok_cloud",
            TemplateProvider::OfficialCli => "official_cli",
        };
        Self {
            id: reg.id,
            cron: reg.cron,
            summary: reg.summary,
            action_kind: action_kind.to_string(),
            swarm_id: reg.action.swarm_id().to_string(),
            time_box_secs,
            provider: provider.to_string(),
            artifact_path: reg.template.artifact_path,
            cloud_model_name: reg.template.cloud_model_name,
            worktree_id: reg.template.worktree_id,
            local_execution_mode: reg.template.local_execution_mode,
            isolation_tier: reg.template.isolation_tier,
            committed_memory_bytes: reg.template.committed_memory_bytes,
            registered_at: reg.registered_at.to_rfc3339(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands (pub, NOT registered -- the Integrate phase owns lib.rs)
// ---------------------------------------------------------------------------

/// Register a calendar schedule + its spawn template. Arms the cron job on the
/// live scheduler and persists the set. Returns an error on a duplicate id or an
/// invalid cron expression.
#[tauri::command]
pub async fn kernel_swarm_schedule_add(
    request: AddScheduleRequestIpc,
    state: State<'_, SwarmSchedulerState>,
) -> Result<(), String> {
    let _ = KERNEL_SWARM_SCHEDULE_ADD_IPC_CHANNEL;
    let id = request.id.trim();
    if id.is_empty() {
        return Err("schedule id must not be empty".to_string());
    }
    let cron = request.cron.trim();
    if cron.is_empty() {
        return Err("cron expression must not be empty".to_string());
    }
    let schedule = SwarmSchedule {
        id: id.to_string(),
        cron: cron.to_string(),
        summary: request.summary.trim().to_string(),
        action: request.action.to_core(),
    };
    state.add(schedule, request.template.into()).await
}

/// List the registered calendar schedules.
#[tauri::command]
pub async fn kernel_swarm_schedule_list(
    state: State<'_, SwarmSchedulerState>,
) -> Result<Vec<RegisteredScheduleIpc>, String> {
    let _ = KERNEL_SWARM_SCHEDULE_LIST_IPC_CHANNEL;
    Ok(state.list().into_iter().map(Into::into).collect())
}

/// Remove a registered calendar schedule by id (stops it firing + persists).
#[tauri::command]
pub async fn kernel_swarm_schedule_remove(
    schedule_id: String,
    state: State<'_, SwarmSchedulerState>,
) -> Result<(), String> {
    let _ = KERNEL_SWARM_SCHEDULE_REMOVE_IPC_CHANNEL;
    state.remove(schedule_id.trim()).await
}

/// Export the registered schedules as an RFC-5545 ICS calendar string the
/// operator can subscribe to in any calendar app (CalDAV / Google / Apple).
#[tauri::command]
pub async fn kernel_swarm_schedule_export_ics(
    state: State<'_, SwarmSchedulerState>,
) -> Result<String, String> {
    let _ = KERNEL_SWARM_SCHEDULE_EXPORT_ICS_IPC_CHANNEL;
    Ok(state.export_ics())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use handshake_core::process_ledger::{
        LedgerBatcher, LedgerBatcherConfig, LedgerEvent, LedgerOverflowEvent, ProcessLedgerError,
        ProcessLedgerOverflowSink, ProcessLedgerStore,
    };
    use handshake_core::swarm_orchestration::{
        LiveSession, ModelSessionFactory, RecordingSwarmSink, RunBudget, SwarmConfig, SwarmError,
        SwarmEventSink, SwarmResult,
    };

    // --- minimal real ledger plumbing for a test coordinator ----------------

    #[derive(Clone, Default)]
    struct VecLedgerStore;

    #[async_trait]
    impl ProcessLedgerStore for VecLedgerStore {
        async fn write_batch(&self, _events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
            Ok(())
        }
    }

    #[derive(Clone, Default)]
    struct NoopOverflow;
    impl ProcessLedgerOverflowSink for NoopOverflow {
        fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
            Ok(())
        }
    }

    /// A test factory that CAPTURES every SpawnRequest the coordinator hands it
    /// and then fails the load honestly (FactoryFailed). Capturing-then-failing
    /// needs no real ModelRuntime, yet proves the full fire -> build_spawn_request
    /// -> coordinator.spawn_session wiring carried the template + time_box +
    /// swarm grouping through verbatim.
    struct CapturingFactory {
        captured: Arc<Mutex<Vec<SpawnRequest>>>,
        create_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ModelSessionFactory for CapturingFactory {
        async fn create(&self, request: &SpawnRequest) -> SwarmResult<LiveSession> {
            self.create_calls.fetch_add(1, Ordering::SeqCst);
            self.captured
                .lock()
                .expect("captured poisoned")
                .push(request.clone());
            Err(SwarmError::FactoryFailed(
                "test capturing factory: intentional load failure after capture".to_string(),
            ))
        }
    }

    /// Build a real `SwarmCoordinator` wired to the capturing factory + a real
    /// ledger batcher + a recording sink. The coordinator code is the production
    /// code; only the factory is the controllable test seam.
    fn test_coordinator() -> (
        Arc<SwarmCoordinator>,
        Arc<Mutex<Vec<SpawnRequest>>>,
        Arc<AtomicUsize>,
    ) {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let create_calls = Arc::new(AtomicUsize::new(0));
        let (ledger, _writer) = LedgerBatcher::spawn(
            Arc::new(VecLedgerStore),
            Arc::new(NoopOverflow),
            LedgerBatcherConfig::default(),
        );
        // Keep the writer task alive for the test's duration by leaking it into a
        // long-lived holder; dropping it would stop the ledger writer.
        std::mem::forget(_writer);
        let budget = RunBudget::defaulted(4).with_concurrency(4);
        let config = SwarmConfig::new(budget);
        let factory = Arc::new(CapturingFactory {
            captured: captured.clone(),
            create_calls: create_calls.clone(),
        });
        let sink: Arc<dyn SwarmEventSink> = Arc::new(RecordingSwarmSink::new());
        let coordinator = Arc::new(SwarmCoordinator::new(config, factory, sink, ledger));
        coordinator.start_reaper();
        (coordinator, captured, create_calls)
    }

    fn local_template() -> SpawnTemplate {
        SpawnTemplate {
            provider: TemplateProvider::Local,
            artifact_path: Some("D:/__sched_test_model__/model.safetensors".to_string()),
            sha256_expected: Some("cd".repeat(32)),
            runtime_binding: Some(TemplateRuntimeBinding::Candle),
            local_execution_mode: None,
            cloud_model_name: None,
            instance: 0,
            worktree_id: Some("wt-sched".to_string()),
            isolation_tier: Some(TemplateIsolationTier::Tier3Microvm),
            committed_memory_bytes: Some(6 * 1024 * 1024 * 1024),
            parent_session_id: Some("calendar".to_string()),
        }
    }

    fn spinup_schedule(id: &str) -> SwarmSchedule {
        SwarmSchedule {
            id: id.to_string(),
            cron: "0 0 9 * * *".to_string(),
            summary: "scheduled spin-up".to_string(),
            action: ScheduledAction::SpinUp {
                swarm_id: "research".to_string(),
                time_box: Some(Duration::from_secs(3600)),
            },
        }
    }

    /// THE END-TO-END PROOF: a registered SpinUp schedule fires the callback,
    /// which builds a SpawnRequest from the stored template + the fire's
    /// time_box + swarm_id and calls the (test) coordinator's spawn_session. We
    /// assert the captured SpawnRequest carries exactly the template's artifact
    /// path + sha + binding, the fire's time_box, the swarm grouping, and the
    /// worktree. Uses a 1-second cron so the live tokio-cron scheduler fires.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn registered_spinup_fires_and_spawns_via_coordinator_with_template_and_time_box() {
        let (coordinator, captured, create_calls) = test_coordinator();
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SwarmScheduleStore::new(tmp.path());

        let state = SwarmSchedulerState::new(
            coordinator,
            store,
            None, // no recorder -> FR notes go to stderr; not asserted here
            tokio::runtime::Handle::current(),
        )
        .await
        .expect("scheduler state");
        // Bootstrap from the (empty) store + start ticking.
        state.bootstrap().await.expect("bootstrap");

        // Register a SpinUp every second, time-boxed at 90s, grouped under
        // 'nightly-research'.
        let schedule = SwarmSchedule {
            id: "every-sec-spinup".to_string(),
            cron: "* * * * * *".to_string(),
            summary: "tick spin-up".to_string(),
            action: ScheduledAction::SpinUp {
                swarm_id: "nightly-research".to_string(),
                time_box: Some(Duration::from_secs(90)),
            },
        };
        state
            .add(schedule, local_template())
            .await
            .unwrap_or_else(|e| panic!("add: {e}"));

        // Wait up to ~3.5s for at least one cron fire to reach the coordinator.
        let mut waited = 0u64;
        while create_calls.load(Ordering::SeqCst) == 0 && waited < 3500 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            waited += 100;
        }
        assert!(
            create_calls.load(Ordering::SeqCst) >= 1,
            "the SpinUp schedule must fire and reach the coordinator's factory within 3.5s"
        );

        let reqs = captured.lock().expect("captured poisoned").clone();
        let first = reqs.first().expect("at least one captured spawn request");
        // Template fields carried through verbatim.
        assert_eq!(
            first.model_artifact_path(),
            Some("D:/__sched_test_model__/model.safetensors"),
            "artifact path from template"
        );
        assert_eq!(
            first.model_artifact_sha256.as_deref(),
            Some(&*"cd".repeat(32)),
            "sha from template"
        );
        assert_eq!(first.runtime_binding, RuntimeBinding::Candle);
        // Fire-supplied grouping + time-box applied.
        assert_eq!(first.swarm_id(), Some("nightly-research"));
        assert_eq!(first.worktree_id(), Some("wt-sched"));
        assert_eq!(
            first.isolation_tier(),
            Some(handshake_core::sandbox::adapter::IsolationTier::Tier3Microvm)
        );
        assert_eq!(first.committed_memory_bytes, Some(6 * 1024 * 1024 * 1024));
        assert_eq!(first.time_box, Some(Duration::from_secs(90)));
        assert_eq!(first.parent_session_id, "calendar");
    }

    #[test]
    fn scheduled_warm_vm_template_sets_warm_mode_and_validates_prerequisites() {
        let mut template = local_template();
        template.runtime_binding = Some(TemplateRuntimeBinding::LlamaCpp);
        template.local_execution_mode = Some(TemplateLocalExecutionMode::WarmVm);
        template.isolation_tier = Some(TemplateIsolationTier::Tier3Microvm);

        let spawn = build_spawn_request(&template, "warm-swarm", Some(Duration::from_secs(60)))
            .expect("valid warm schedule template");
        assert!(spawn.wants_warm_vm_execution());
        assert_eq!(spawn.runtime_binding, RuntimeBinding::LlamaCpp);
        assert_eq!(spawn.worktree_id(), Some("wt-sched"));

        template.isolation_tier = Some(TemplateIsolationTier::Tier2Syscall);
        let err = build_spawn_request(&template, "warm-swarm", None)
            .expect_err("warm mode rejects non-tier3 schedule");
        assert!(
            err.contains("isolation_tier=tier3_microvm"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn schedule_add_rejects_cloud_local_execution_mode_before_persisting() {
        let (coordinator, _cap, _calls) = test_coordinator();
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SwarmScheduleStore::new(tmp.path());
        let state = SwarmSchedulerState::new(
            coordinator,
            store.clone(),
            None,
            tokio::runtime::Handle::current(),
        )
        .await
        .expect("state");
        state.bootstrap().await.expect("bootstrap");

        let mut template = SpawnTemplate {
            provider: TemplateProvider::ByokCloud,
            artifact_path: None,
            sha256_expected: None,
            runtime_binding: None,
            local_execution_mode: Some(TemplateLocalExecutionMode::WarmVm),
            cloud_model_name: Some("gpt-4o".to_string()),
            instance: 0,
            worktree_id: None,
            isolation_tier: None,
            committed_memory_bytes: None,
            parent_session_id: Some("calendar".to_string()),
        };

        let err = state
            .add(spinup_schedule("cloud-warm"), template.clone())
            .await
            .expect_err("cloud schedule must reject warm_vm before storage");
        assert!(
            err.contains("local_execution_mode"),
            "unexpected error: {err}"
        );
        assert_eq!(state.registered_count(), 0);
        assert!(
            store.load().expect("load").schedules.is_empty(),
            "invalid cloud warm schedule must not persist"
        );

        template.local_execution_mode = Some(TemplateLocalExecutionMode::Cold);
        let err = state
            .add(spinup_schedule("cloud-cold"), template)
            .await
            .expect_err("cloud schedule must reject any local execution mode before storage");
        assert!(
            err.contains("local_execution_mode"),
            "unexpected error: {err}"
        );
        assert_eq!(state.registered_count(), 0);
        assert!(
            store.load().expect("load").schedules.is_empty(),
            "invalid cloud cold-mode schedule must not persist"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn schedule_add_rejects_invalid_warm_vm_before_persisting() {
        let (coordinator, _cap, _calls) = test_coordinator();
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SwarmScheduleStore::new(tmp.path());
        let state = SwarmSchedulerState::new(
            coordinator,
            store.clone(),
            None,
            tokio::runtime::Handle::current(),
        )
        .await
        .expect("state");
        state.bootstrap().await.expect("bootstrap");

        let mut template = local_template();
        template.runtime_binding = Some(TemplateRuntimeBinding::Candle);
        template.local_execution_mode = Some(TemplateLocalExecutionMode::WarmVm);
        template.isolation_tier = Some(TemplateIsolationTier::Tier3Microvm);

        let err = state
            .add(spinup_schedule("local-warm-invalid-binding"), template)
            .await
            .expect_err("invalid warm schedule must reject before storage");
        assert!(
            err.contains("runtime_binding=llama_cpp"),
            "unexpected error: {err}"
        );
        assert_eq!(state.registered_count(), 0);
        assert!(
            store.load().expect("load").schedules.is_empty(),
            "invalid local warm schedule must not persist"
        );
    }

    /// Persistence round-trips: an added schedule survives a NEW state built from
    /// the SAME store (simulating an app restart). The reloaded schedule re-arms
    /// and the template is intact.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_persists_and_reload_restores_schedule_and_template() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let store_path = tmp.path().to_path_buf();

        // First app instance: add a schedule, which persists to disk.
        {
            let (coordinator, _cap, _calls) = test_coordinator();
            let state = SwarmSchedulerState::new(
                coordinator,
                SwarmScheduleStore::new(&store_path),
                None,
                tokio::runtime::Handle::current(),
            )
            .await
            .expect("state");
            state.bootstrap().await.expect("bootstrap");
            let schedule = SwarmSchedule {
                id: "morning".to_string(),
                cron: "0 0 9 * * *".to_string(),
                summary: "Morning research".to_string(),
                action: ScheduledAction::SpinUp {
                    swarm_id: "research".to_string(),
                    time_box: Some(Duration::from_secs(3600)),
                },
            };
            state.add(schedule, local_template()).await.expect("add");
            assert_eq!(state.registered_count(), 1);
        }

        // The file exists on disk.
        let store = SwarmScheduleStore::new(&store_path);
        assert!(store.path().exists(), "persisted file must exist after add");
        let doc = store.load().expect("load");
        assert_eq!(doc.schedules.len(), 1);
        assert_eq!(doc.schedules[0].id, "morning");
        assert_eq!(
            doc.schedules[0].template.artifact_path.as_deref(),
            Some("D:/__sched_test_model__/model.safetensors")
        );

        // Second app instance: bootstrap from the SAME store re-arms it.
        {
            let (coordinator, _cap, _calls) = test_coordinator();
            let state = SwarmSchedulerState::new(
                coordinator,
                SwarmScheduleStore::new(&store_path),
                None,
                tokio::runtime::Handle::current(),
            )
            .await
            .expect("state");
            state.bootstrap().await.expect("bootstrap reload");
            assert_eq!(
                state.registered_count(),
                1,
                "the persisted schedule must be re-armed on restart"
            );
            let listed = state.list();
            assert_eq!(listed[0].id, "morning");
            assert_eq!(
                listed[0].action.swarm_id(),
                "research",
                "re-armed action target survives"
            );
        }
    }

    /// ICS export contains the registered schedule (cron + summary + swarm id).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn export_ics_contains_the_registered_schedule() {
        let (coordinator, _cap, _calls) = test_coordinator();
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SwarmSchedulerState::new(
            coordinator,
            SwarmScheduleStore::new(tmp.path()),
            None,
            tokio::runtime::Handle::current(),
        )
        .await
        .expect("state");
        state.bootstrap().await.expect("bootstrap");

        let schedule = SwarmSchedule {
            id: "morning-research".to_string(),
            cron: "0 0 9 * * *".to_string(),
            summary: "Morning research swarm".to_string(),
            action: ScheduledAction::SpinUp {
                swarm_id: "research".to_string(),
                time_box: Some(Duration::from_secs(3600)),
            },
        };
        state.add(schedule, local_template()).await.expect("add");

        let ics = state.export_ics();
        assert!(ics.contains("BEGIN:VCALENDAR"), "{ics}");
        assert!(ics.contains("BEGIN:VEVENT"), "{ics}");
        assert!(ics.contains("Morning research swarm"), "summary: {ics}");
        assert!(ics.contains("0 0 9 * * *"), "cron must appear: {ics}");
        assert!(ics.contains("research"), "swarm id must appear: {ics}");
        assert!(
            ics.contains("handshake-swarm-morning-research@handshake"),
            "uid: {ics}"
        );
    }

    /// Removing a registered schedule stops it from firing and updates the
    /// persisted set + the live registry.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remove_drops_schedule_from_registry_and_disk() {
        let (coordinator, _cap, _calls) = test_coordinator();
        let tmp = tempfile::tempdir().expect("tempdir");
        let store = SwarmScheduleStore::new(tmp.path());
        let state = SwarmSchedulerState::new(
            coordinator,
            store.clone(),
            None,
            tokio::runtime::Handle::current(),
        )
        .await
        .expect("state");
        state.bootstrap().await.expect("bootstrap");

        let schedule = SwarmSchedule {
            id: "to-remove".to_string(),
            cron: "0 0 9 * * *".to_string(),
            summary: "removable".to_string(),
            action: ScheduledAction::Teardown {
                swarm_id: "research".to_string(),
            },
        };
        state.add(schedule, local_template()).await.expect("add");
        assert_eq!(state.registered_count(), 1);

        state.remove("to-remove").await.expect("remove");
        assert_eq!(state.registered_count(), 0);
        assert!(store.load().expect("load").schedules.is_empty());

        // Removing an unknown id is a typed error.
        assert!(state.remove("nope").await.is_err());
    }
}
