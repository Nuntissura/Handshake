//! Handshake-native in-process diagnostics (Master Spec §5.8).
//!
//! This module is intentionally independent from the backend Flight Recorder:
//! it remains available when the backend is unreachable and writes only a
//! typed allowlist of mechanical health data. A file-backed shared-memory ring
//! gives the out-of-process Palmistry watcher a passive, non-blocking read path.

use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use curve25519_dalek::montgomery::MontgomeryPoint;
use memmap2::{Mmap, MmapMut};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    backtrace::Backtrace,
    collections::VecDeque,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};

pub const INTERNAL_DIAGNOSTICS_SCHEMA_ID: &str = "hsk.internal_diagnostics.snapshot@1";
pub const INTERNAL_DIAGNOSTICS_RING_VERSION: u32 = 1;
pub const INTERNAL_DIAGNOSTICS_EVENT_CAP: usize = 128;
pub const INTERNAL_DIAGNOSTICS_FRAME_CAP: usize = 240;
pub const INTERNAL_DIAGNOSTICS_CRASH_EVENT_CAP: usize = 32;
pub const INTERNAL_DIAGNOSTICS_BEHAVIOR_OBSERVATION_CAP: usize = 32;
pub const INTERNAL_DIAGNOSTICS_BACKTRACE_LINE_CAP: usize = 128;
pub const DEFAULT_RESOURCE_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
pub const DEFAULT_RING_PUBLISH_INTERVAL: Duration = Duration::from_millis(250);
pub const PALMISTRY_START_PATH: &str = "/internal-diagnostics/palmistry/start";

const RING_MAGIC: &[u8; 8] = b"HSKIDG01";
const RING_HEADER_BYTES: usize = 128;
const RING_SLOT_BYTES: usize = 128 * 1024;
const RING_BYTES: usize = RING_HEADER_BYTES + (RING_SLOT_BYTES * 2);
const SLOT_HEADER_BYTES: usize = 8 + 4 + 32;
const MAX_SLOT_PAYLOAD_BYTES: usize = RING_SLOT_BYTES - SLOT_HEADER_BYTES;
const OFFSET_VERSION: usize = 8;
const OFFSET_ACTIVE_SLOT: usize = 12;
const OFFSET_GENERATION: usize = 16;
const OFFSET_HEARTBEAT_COUNTER: usize = 24;
const OFFSET_HEARTBEAT_UNIX_MS: usize = 32;
const OFFSET_HEARTBEAT_MONOTONIC_MS: usize = 40;
const OFFSET_PANIC_PENDING: usize = 48;
const OFFSET_PID: usize = 52;
const OFFSET_SESSION_ID: usize = 56;
const OFFSET_LAUNCH_NONCE: usize = 72;

static ACTIVE_DIAGNOSTICS: OnceLock<Mutex<Weak<SharedDiagnostics>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticMechanism {
    Heartbeat,
    FrameTime,
    ResourceSampler,
    BackendRoute,
    GuiAction,
    MechanicalJob,
    Panic,
    RingBuffer,
    Palmistry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventState {
    Started,
    Healthy,
    Degraded,
    Failed,
    Recovered,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    BackendUnavailable,
    BackendRecovered,
    FrameSlow,
    ResourceSampleUnavailable,
    PanicObserved,
    RingPublishFailed,
    WatcherUnavailable,
    WatcherRecoveredRecord,
}

/// The open diagnostic-event API accepts only these mechanical fields. There
/// is deliberately no free-form payload, project id, path, prompt, or content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalDiagnosticEvent {
    pub event_id: Uuid,
    pub observed_at_unix_ms: u64,
    pub mechanism: DiagnosticMechanism,
    pub state: DiagnosticEventState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<DiagnosticCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_micros: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counter: Option<u64>,
}

impl InternalDiagnosticEvent {
    pub fn mechanical(
        mechanism: DiagnosticMechanism,
        state: DiagnosticEventState,
        code: Option<DiagnosticCode>,
    ) -> Self {
        Self {
            event_id: Uuid::now_v7(),
            observed_at_unix_ms: unix_ms(),
            mechanism,
            state,
            code,
            duration_micros: None,
            counter: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameTimeStats {
    pub sample_count: u64,
    pub last_micros: u64,
    pub min_micros: u64,
    pub max_micros: u64,
    pub p50_micros: u64,
    pub p95_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCounters {
    pub sampled_at_unix_ms: u64,
    pub cpu_percent: Option<f32>,
    pub rss_bytes: Option<u64>,
    pub gpu_percent: Option<f32>,
    pub gpu_status: ResourceMetricStatus,
}

impl Default for ResourceCounters {
    fn default() -> Self {
        Self {
            sampled_at_unix_ms: 0,
            cpu_percent: None,
            rss_bytes: None,
            gpu_percent: None,
            gpu_status: ResourceMetricStatus::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMetricStatus {
    Sampled,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InternalDiagnosticsSnapshot {
    pub schema_id: String,
    pub ring_version: u32,
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub process_id: u32,
    pub build_id: String,
    pub heartbeat_counter: u64,
    pub heartbeat_unix_ms: u64,
    pub heartbeat_monotonic_ms: u64,
    pub frame_time: FrameTimeStats,
    pub resources: ResourceCounters,
    pub events: Vec<InternalDiagnosticEvent>,
    pub behavior_observations: Vec<BehaviorObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorObservation {
    pub mechanism: String,
    pub heartbeat_counter: u64,
    pub observed_at_unix_ms: u64,
    pub correlation_hmac: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PanicPayloadClass {
    String,
    StaticStr,
    Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableCrashRecord {
    pub schema_id: String,
    pub crash_id: Uuid,
    pub session_id: Uuid,
    pub process_id: u32,
    pub observed_at_unix_ms: u64,
    pub build_id: String,
    pub payload_class: PanicPayloadClass,
    pub location_file_sha256: Option<String>,
    pub location_line: Option<u32>,
    pub location_column: Option<u32>,
    pub redacted_backtrace: String,
    pub last_events: Vec<InternalDiagnosticEvent>,
}

#[derive(Debug, Clone)]
pub struct InternalDiagnosticsPaths {
    pub root: PathBuf,
    pub ring: PathBuf,
    pub crash_dir: PathBuf,
    pub survivor_dir: PathBuf,
    pub panic_signal: PathBuf,
    pub panic_ack: PathBuf,
    pub shutdown_signal: PathBuf,
    pub ready_signal: PathBuf,
    pub recovered_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
struct PalmistryLaunchRequest<'a> {
    session_id: Uuid,
    launch_nonce: Uuid,
    parent_pid: u32,
    ring: &'a Path,
    survivor_dir: &'a Path,
    panic_signal: &'a Path,
    panic_ack: &'a Path,
    shutdown_signal: &'a Path,
    ready_signal: &'a Path,
    transport_public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveredSurvivorSummary {
    pub record_id: Uuid,
    pub source_session_id: Uuid,
    pub kind: String,
    pub observed_at_unix_ms: u64,
    pub parent_pid: u32,
    pub parent_exit_code: Option<u32>,
    pub heartbeat_stale_ms: Option<u64>,
    pub os_hung_window_confirmed: bool,
    pub minidump_status: String,
    pub imported: bool,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct PalmistryRecoverRequest<'a> {
    current_session_id: Uuid,
    launch_nonce: Uuid,
    summary: &'a RecoveredSurvivorSummary,
    proof: String,
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PalmistryLaunchReceipt {
    pub session_id: Uuid,
    pub process_uuid: Uuid,
    pub os_pid: u32,
    pub sandbox_adapter_id: String,
    pub ledger_start_durable: bool,
    pub os_creation_time_100ns: u64,
    argus_signing_secret_envelope: TransportSigningSecretEnvelope,
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransportSigningSecretEnvelope {
    server_public_key: String,
    nonce: String,
    ciphertext: String,
}

impl fmt::Debug for PalmistryLaunchReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PalmistryLaunchReceipt")
            .field("session_id", &self.session_id)
            .field("process_uuid", &self.process_uuid)
            .field("os_pid", &self.os_pid)
            .field("sandbox_adapter_id", &self.sandbox_adapter_id)
            .field("ledger_start_durable", &self.ledger_start_durable)
            .field("os_creation_time_100ns", &self.os_creation_time_100ns)
            .field("argus_signing_secret_envelope", &"[SEALED]")
            .finish()
    }
}

impl InternalDiagnosticsPaths {
    pub fn resolve(session_id: Uuid) -> io::Result<Self> {
        let root = diagnostics_root();
        let crash_dir = root.join("crashes");
        let survivor_dir = root.join("survivors");
        let recovered_dir = root.join("recovered");
        fs::create_dir_all(&crash_dir)?;
        fs::create_dir_all(&survivor_dir)?;
        fs::create_dir_all(&recovered_dir)?;
        restrict_diagnostics_path(&root)?;
        restrict_diagnostics_path(&crash_dir)?;
        restrict_diagnostics_path(&survivor_dir)?;
        restrict_diagnostics_path(&recovered_dir)?;
        let session = session_id.to_string();
        Ok(Self {
            ring: root.join(format!("ring-{session}.bin")),
            panic_signal: root.join(format!("panic-{session}.signal.json")),
            panic_ack: root.join(format!("panic-{session}.ack")),
            shutdown_signal: root.join(format!("shutdown-{session}.signal")),
            ready_signal: root.join(format!("ready-{session}.json")),
            root,
            crash_dir,
            survivor_dir,
            recovered_dir,
        })
    }
}

#[derive(Clone)]
pub struct InternalDiagnostics {
    shared: Arc<SharedDiagnostics>,
}

pub struct DiagnosticObservationProvenance {
    pub session_id: Uuid,
    pub signing_secret: Zeroizing<[u8; 32]>,
    pub heartbeat_counter: u64,
}

pub fn active_observation_provenance() -> Option<DiagnosticObservationProvenance> {
    let shared =
        lock_unpoisoned(ACTIVE_DIAGNOSTICS.get_or_init(|| Mutex::new(Weak::new()))).upgrade()?;
    let heartbeat_counter = lock_unpoisoned(&shared.state).heartbeat_counter;
    let signing_secret = lock_unpoisoned(&shared.argus_signing_secret)
        .as_ref()
        .cloned()?;
    Some(DiagnosticObservationProvenance {
        session_id: shared.session_id,
        signing_secret,
        heartbeat_counter,
    })
}

pub fn emit_behavior_observation_open(
    behavior_id: &str,
    run_id: &str,
    lane_id: &str,
) -> Option<DiagnosticObservationProvenance> {
    let shared =
        lock_unpoisoned(ACTIVE_DIAGNOSTICS.get_or_init(|| Mutex::new(Weak::new()))).upgrade()?;
    InternalDiagnostics { shared }.emit_behavior_observation(behavior_id, run_id, lane_id)
}

struct SharedDiagnostics {
    session_id: Uuid,
    launch_nonce: Uuid,
    argus_signing_secret: Mutex<Option<Zeroizing<[u8; 32]>>>,
    paths: InternalDiagnosticsPaths,
    started: Instant,
    ring: Mutex<MappedRingWriter>,
    panic_latch: PanicLatch,
    state: Mutex<RuntimeState>,
    stop_sampler: AtomicBool,
    /// WP-1: operator-controllable pause for the resource sampler ONLY. When `true`, the sampler thread
    /// skips its `sample()` + snapshot publish for that tick but keeps looping (so it resumes without a
    /// respawn). Deliberately independent of `stop_sampler`, the panic hook, the frame-time tick, and
    /// Palmistry — pausing background counters must never blind the crash path or starve the Argus
    /// signing-secret rotation the Palmistry watcher performs.
    sampler_paused: AtomicBool,
}

struct RuntimeState {
    heartbeat_counter: u64,
    frame_samples: VecDeque<u64>,
    resources: ResourceCounters,
    events: VecDeque<InternalDiagnosticEvent>,
    behavior_observations: VecDeque<BehaviorObservation>,
    last_publish: Instant,
    recovered_survivors: Vec<RecoveredSurvivorSummary>,
}

struct PanicLatch {
    map: MmapMut,
    fired: AtomicBool,
}

impl PanicLatch {
    fn signal(&self) {
        if self
            .fired
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            mapped_atomic_u32(&self.map, OFFSET_PANIC_PENDING).store(1, Ordering::Release);
            let _ = self.map.flush_range(0, RING_HEADER_BYTES);
        }
    }
}

impl Drop for SharedDiagnostics {
    fn drop(&mut self) {
        self.stop_sampler.store(true, Ordering::Release);
        if let Ok(secret) = self.argus_signing_secret.get_mut() {
            secret.zeroize();
        }
    }
}

impl InternalDiagnostics {
    pub fn start() -> io::Result<Self> {
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let paths = InternalDiagnosticsPaths::resolve(session_id)?;
        remove_if_exists(&paths.panic_ack)?;
        remove_if_exists(&paths.shutdown_signal)?;
        remove_if_exists(&paths.ready_signal)?;
        let recovered_survivors = scan_survivors(&paths)?;
        let mut ring =
            MappedRingWriter::create(&paths.ring, std::process::id(), session_id, launch_nonce)?;
        restrict_diagnostics_path(&paths.ring)?;
        let panic_latch = ring.take_panic_latch()?;
        let started = Instant::now();
        let snapshot = InternalDiagnosticsSnapshot {
            schema_id: INTERNAL_DIAGNOSTICS_SCHEMA_ID.to_owned(),
            ring_version: INTERNAL_DIAGNOSTICS_RING_VERSION,
            session_id,
            launch_nonce,
            process_id: std::process::id(),
            build_id: build_id(),
            heartbeat_counter: 0,
            heartbeat_unix_ms: unix_ms(),
            heartbeat_monotonic_ms: 0,
            frame_time: FrameTimeStats::default(),
            resources: ResourceCounters::default(),
            events: Vec::new(),
            behavior_observations: Vec::new(),
        };
        ring.publish(&snapshot)?;
        let diagnostics = Self {
            shared: Arc::new(SharedDiagnostics {
                session_id,
                launch_nonce,
                argus_signing_secret: Mutex::new(None),
                paths,
                started,
                ring: Mutex::new(ring),
                panic_latch,
                state: Mutex::new(RuntimeState {
                    heartbeat_counter: 0,
                    frame_samples: VecDeque::with_capacity(INTERNAL_DIAGNOSTICS_FRAME_CAP),
                    resources: ResourceCounters::default(),
                    events: VecDeque::with_capacity(INTERNAL_DIAGNOSTICS_EVENT_CAP),
                    behavior_observations: VecDeque::with_capacity(
                        INTERNAL_DIAGNOSTICS_BEHAVIOR_OBSERVATION_CAP,
                    ),
                    last_publish: Instant::now(),
                    recovered_survivors,
                }),
                stop_sampler: AtomicBool::new(false),
                sampler_paused: AtomicBool::new(false),
            }),
        };
        *lock_unpoisoned(ACTIVE_DIAGNOSTICS.get_or_init(|| Mutex::new(Weak::new()))) =
            Arc::downgrade(&diagnostics.shared);
        diagnostics.record(InternalDiagnosticEvent::mechanical(
            DiagnosticMechanism::Heartbeat,
            DiagnosticEventState::Started,
            None,
        ));
        diagnostics.start_resource_sampler(DEFAULT_RESOURCE_SAMPLE_INTERVAL);
        Ok(diagnostics)
    }

    pub fn session_id(&self) -> Uuid {
        self.shared.session_id
    }

    pub fn paths(&self) -> &InternalDiagnosticsPaths {
        &self.shared.paths
    }

    pub fn launch_nonce(&self) -> Uuid {
        self.shared.launch_nonce
    }

    pub fn argus_signing_secret(&self) -> Option<Zeroizing<[u8; 32]>> {
        lock_unpoisoned(&self.shared.argus_signing_secret)
            .as_ref()
            .cloned()
    }

    /// Live secret lookup used by the already-bound MCP server. Keeping a
    /// weak reference avoids extending diagnostics lifetime while allowing a
    /// successful backend reauthentication to rotate receipt signing without
    /// restarting the listener.
    pub fn argus_signing_secret_provider(
        &self,
    ) -> Arc<dyn Fn() -> Option<Zeroizing<[u8; 32]>> + Send + Sync + 'static> {
        let shared = Arc::downgrade(&self.shared);
        Arc::new(move || {
            let shared = shared.upgrade()?;
            let signing_secret = {
                let guard = lock_unpoisoned(&shared.argus_signing_secret);
                guard.as_ref().cloned()
            };
            signing_secret
        })
    }

    pub fn recovered_survivors(&self) -> Vec<RecoveredSurvivorSummary> {
        lock_unpoisoned(&self.shared.state)
            .recovered_survivors
            .clone()
    }

    /// WP-1: enable/disable the background resource sampler at runtime. `enabled == false` pauses only
    /// the CPU/RSS/GPU counter sampling + snapshot publish; the sampler thread keeps looping so a later
    /// re-enable resumes without a respawn. The panic hook, frame-time tick, and Palmistry maintenance
    /// are untouched, so this can never blind the crash path or the Argus signing-secret rotation.
    /// Idempotent; safe to call every frame from the UI thread.
    pub fn set_resource_sampling_enabled(&self, enabled: bool) {
        self.shared
            .sampler_paused
            .store(!enabled, Ordering::Release);
    }

    /// WP-1: whether background resource sampling is currently enabled (not paused). Real state for the
    /// Settings Diagnostics status display + tests.
    pub fn resource_sampling_enabled(&self) -> bool {
        !self.shared.sampler_paused.load(Ordering::Acquire)
    }

    /// Called once per real egui frame. The heartbeat atomics are updated every
    /// call; the larger JSON snapshot is rate-limited to avoid frame jank.
    pub fn tick_frame(&self, frame_duration: Duration) {
        let now_unix_ms = unix_ms();
        let monotonic_ms = duration_ms_u64(self.shared.started.elapsed());
        let duration_micros = duration_micros_u64(frame_duration);
        let (heartbeat_counter, should_publish, slow_event) = {
            let mut state = lock_unpoisoned(&self.shared.state);
            state.heartbeat_counter = state.heartbeat_counter.saturating_add(1);
            if state.frame_samples.len() == INTERNAL_DIAGNOSTICS_FRAME_CAP {
                state.frame_samples.pop_front();
            }
            state.frame_samples.push_back(duration_micros);
            let slow_event =
                (frame_duration >= Duration::from_millis(100)).then_some(InternalDiagnosticEvent {
                    duration_micros: Some(duration_micros),
                    ..InternalDiagnosticEvent::mechanical(
                        DiagnosticMechanism::FrameTime,
                        DiagnosticEventState::Degraded,
                        Some(DiagnosticCode::FrameSlow),
                    )
                });
            let should_publish = state.last_publish.elapsed() >= DEFAULT_RING_PUBLISH_INTERVAL
                || slow_event.is_some();
            (state.heartbeat_counter, should_publish, slow_event)
        };
        {
            let ring = lock_unpoisoned(&self.shared.ring);
            ring.publish_heartbeat(heartbeat_counter, now_unix_ms, monotonic_ms);
        }
        if let Some(event) = slow_event {
            self.record(event);
        } else if should_publish {
            let _ = self.publish_snapshot();
        }
    }

    pub fn record(&self, event: InternalDiagnosticEvent) {
        {
            let mut state = lock_unpoisoned(&self.shared.state);
            if state.events.len() == INTERNAL_DIAGNOSTICS_EVENT_CAP {
                state.events.pop_front();
            }
            state.events.push_back(event);
        }
        if let Err(error) = self.publish_snapshot() {
            tracing::error!(error = %error, "internal_diagnostics ring publish failed");
        }
    }

    pub fn emit_behavior_observation(
        &self,
        behavior_id: &str,
        run_id: &str,
        lane_id: &str,
    ) -> Option<DiagnosticObservationProvenance> {
        let observed_at_unix_ms = unix_ms();
        let signing_secret = self.argus_signing_secret()?;
        let heartbeat_counter = {
            let mut state = lock_unpoisoned(&self.shared.state);
            state.heartbeat_counter = state.heartbeat_counter.max(1);
            let heartbeat_counter = state.heartbeat_counter;
            let correlation_hmac = behavior_observation_hmac(
                behavior_id,
                run_id,
                lane_id,
                heartbeat_counter,
                observed_at_unix_ms,
                signing_secret.as_ref(),
            );
            if state.behavior_observations.len() == INTERNAL_DIAGNOSTICS_BEHAVIOR_OBSERVATION_CAP {
                state.behavior_observations.pop_front();
            }
            state.behavior_observations.push_back(BehaviorObservation {
                mechanism: "model_lane_behavior_observation".to_owned(),
                heartbeat_counter,
                observed_at_unix_ms,
                correlation_hmac,
            });
            heartbeat_counter
        };
        self.publish_snapshot().ok()?;
        Some(DiagnosticObservationProvenance {
            session_id: self.shared.session_id,
            signing_secret,
            heartbeat_counter,
        })
    }

    pub fn snapshot(&self) -> InternalDiagnosticsSnapshot {
        let state = lock_unpoisoned(&self.shared.state);
        snapshot_from_state(&self.shared, &state)
    }

    pub fn publish_snapshot(&self) -> io::Result<()> {
        let snapshot = {
            let mut state = lock_unpoisoned(&self.shared.state);
            state.last_publish = Instant::now();
            snapshot_from_state(&self.shared, &state)
        };
        lock_unpoisoned(&self.shared.ring).publish(&snapshot)
    }

    pub fn install_panic_hook(&self) -> bool {
        static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
        if PANIC_HOOK_INSTALLED.set(()).is_err() {
            return false;
        }
        let diagnostics = self.clone();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            diagnostics.shared.panic_latch.signal();
            diagnostics.write_durable_crash_record(info);
            previous(info);
        }));
        true
    }

    fn write_durable_crash_record(&self, info: &std::panic::PanicHookInfo<'_>) {
        static WRITING_CRASH_RECORD: AtomicBool = AtomicBool::new(false);
        if WRITING_CRASH_RECORD.swap(true, Ordering::AcqRel) {
            return;
        }
        let write = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let crash_id = Uuid::now_v7();
            let payload_class = panic_payload_class(info.payload());
            let (location_file_sha256, location_line, location_column) = info
                .location()
                .map(|location| {
                    (
                        Some(hex_sha256(location.file().as_bytes())),
                        Some(location.line()),
                        Some(location.column()),
                    )
                })
                .unwrap_or((None, None, None));
            let last_events = self
                .shared
                .state
                .try_lock()
                .map(|state| {
                    state
                        .events
                        .iter()
                        .rev()
                        .take(INTERNAL_DIAGNOSTICS_CRASH_EVENT_CAP)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect()
                })
                .unwrap_or_default();
            let record = DurableCrashRecord {
                schema_id: "hsk.internal_diagnostics.crash@1".to_owned(),
                crash_id,
                session_id: self.shared.session_id,
                process_id: std::process::id(),
                observed_at_unix_ms: unix_ms(),
                build_id: build_id(),
                payload_class,
                location_file_sha256,
                location_line,
                location_column,
                redacted_backtrace: redacted_backtrace(),
                last_events,
            };
            let bytes = serde_json::to_vec(&record)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            let path = self
                .shared
                .paths
                .crash_dir
                .join(format!("crash-{crash_id}.json"));
            write_atomic_durable(&path, &bytes)
        }));
        if !matches!(write, Ok(Ok(()))) {
            self.shared.panic_latch.signal();
        }
    }

    /// Ask the production backend to launch Palmistry through its dedicated
    /// process adapter and durably register the complete watcher lifecycle.
    /// The GUI never uses a bare `Command`, so a backend/ledger failure leaves
    /// the watcher unavailable rather than silently unmanaged.
    pub async fn launch_palmistry(
        &self,
        backend_base_url: &str,
    ) -> Result<(PalmistryLaunchReceipt, bool), String> {
        let transport_secret = Zeroizing::new(rand::random::<[u8; 32]>());
        let transport_public = MontgomeryPoint::mul_base_clamped(*transport_secret);
        let request = PalmistryLaunchRequest {
            session_id: self.shared.session_id,
            launch_nonce: self.shared.launch_nonce,
            parent_pid: std::process::id(),
            ring: &self.shared.paths.ring,
            survivor_dir: &self.shared.paths.survivor_dir,
            panic_signal: &self.shared.paths.panic_signal,
            panic_ack: &self.shared.paths.panic_ack,
            shutdown_signal: &self.shared.paths.shutdown_signal,
            ready_signal: &self.shared.paths.ready_signal,
            transport_public_key: hex::encode(transport_public.as_bytes()),
        };
        let url = format!(
            "{}{}",
            backend_base_url.trim_end_matches('/'),
            PALMISTRY_START_PATH
        );
        let mut response = reqwest::Client::new()
            .post(url)
            .timeout(Duration::from_secs(8))
            .json(&request)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        // The signing secret is AEAD-sealed before it reaches this response, so
        // reqwest/Axum transport allocations retain ciphertext only. Still do
        // not aggregate even that response with `Response::bytes()`: copy
        // bounded chunks into a Zeroizing buffer and wipe the retained body.
        const MAX_LAUNCH_RESPONSE_BYTES: usize = 64 * 1024;
        let mut body = Zeroizing::new(Vec::with_capacity(1024));
        while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
            if body.len().saturating_add(chunk.len()) > MAX_LAUNCH_RESPONSE_BYTES {
                return Err("Palmistry start response exceeded 64 KiB".to_owned());
            }
            body.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            return Err(format!(
                "Palmistry start returned HTTP {status}: {}",
                String::from_utf8_lossy(body.as_slice())
            ));
        }
        let receipt: PalmistryLaunchReceipt =
            serde_json::from_slice(body.as_slice()).map_err(|error| error.to_string())?;
        if receipt.session_id != self.shared.session_id
            || !receipt.ledger_start_durable
            || receipt.sandbox_adapter_id != "palmistry_watcher"
        {
            return Err("Palmistry start receipt failed identity/durability validation".to_owned());
        }
        let signing_secret = open_transport_signing_secret(
            &receipt.argus_signing_secret_envelope,
            &transport_secret,
            self.shared.session_id,
            self.shared.launch_nonce,
        )?;
        let rotated = {
            let mut active = lock_unpoisoned(&self.shared.argus_signing_secret);
            let rotated = active
                .as_ref()
                .is_none_or(|current| current.as_ref() != signing_secret.as_ref());
            *active = Some(signing_secret);
            rotated
        };
        if rotated {
            self.record(InternalDiagnosticEvent::mechanical(
                DiagnosticMechanism::Palmistry,
                DiagnosticEventState::Started,
                None,
            ));
        }
        Ok((receipt, rotated))
    }

    /// Maintain backend authentication for the whole native session. Every
    /// request has a hard timeout, while the supervisor itself is intentionally
    /// durable: it retries through an initially unavailable backend and keeps
    /// reconciling after success so a later backend restart reattaches the
    /// durable watcher and atomically rotates the live MCP signing secret.
    pub async fn maintain_palmistry(
        &self,
        backend_base_url: &str,
        on_signing_secret_rotated: impl Fn() + Send + Sync,
    ) {
        let mut delay = Duration::from_millis(200);
        self.start_survivor_forwarder(backend_base_url.to_owned());
        loop {
            if self.shared.stop_sampler.load(Ordering::Acquire) {
                return;
            }
            match self.launch_palmistry(backend_base_url).await {
                Ok((_, rotated)) => {
                    if rotated {
                        on_signing_secret_rotated();
                    }
                    self.import_pending_survivors(backend_base_url).await;
                    delay = Duration::from_secs(5);
                }
                Err(error) => {
                    self.record(InternalDiagnosticEvent::mechanical(
                        DiagnosticMechanism::Palmistry,
                        DiagnosticEventState::Degraded,
                        Some(DiagnosticCode::WatcherUnavailable),
                    ));
                    tracing::warn!(error = %error, "Palmistry authentication reconciliation failed");
                    delay = (delay * 2).min(Duration::from_secs(10));
                }
            }
            tokio::time::sleep(delay).await;
        }
    }

    async fn import_pending_survivors(&self, backend_base_url: &str) {
        if let Ok(records) = scan_survivors(&self.shared.paths) {
            let mut state = lock_unpoisoned(&self.shared.state);
            for record in records {
                if let Some(existing) = state
                    .recovered_survivors
                    .iter_mut()
                    .find(|existing| existing.record_id == record.record_id)
                {
                    *existing = record;
                } else {
                    state.recovered_survivors.push(record);
                }
            }
            state
                .recovered_survivors
                .sort_by_key(|record| (record.observed_at_unix_ms, record.record_id));
        }
        let pending = self.recovered_survivors();
        let client = reqwest::Client::new();
        for summary in pending.into_iter().filter(|value| !value.imported) {
            let Some(signing_secret) = self.argus_signing_secret() else {
                return;
            };
            let proof = recovery_proof(
                self.shared.session_id,
                self.shared.launch_nonce,
                &summary,
                &signing_secret,
            );
            let url = format!(
                "{}/internal-diagnostics/palmistry/recover",
                backend_base_url.trim_end_matches('/')
            );
            let result = client
                .post(url)
                .timeout(Duration::from_secs(8))
                .json(&PalmistryRecoverRequest {
                    current_session_id: self.shared.session_id,
                    launch_nonce: self.shared.launch_nonce,
                    summary: &summary,
                    proof,
                })
                .send()
                .await;
            let Ok(response) = result else { continue };
            if !response.status().is_success() {
                if response.status().is_client_error()
                    && response.status() != reqwest::StatusCode::CONFLICT
                    && response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS
                {
                    quarantine_survivor_record(&self.shared.paths, &summary);
                }
                continue;
            }
            let ack = self
                .shared
                .paths
                .recovered_dir
                .join(format!("{}.ack", summary.record_id));
            if write_atomic_durable(&ack, b"flight_recorder_imported\n").is_err() {
                continue;
            }
            {
                let mut state = lock_unpoisoned(&self.shared.state);
                if let Some(item) = state
                    .recovered_survivors
                    .iter_mut()
                    .find(|item| item.record_id == summary.record_id)
                {
                    item.imported = true;
                }
            }
            self.record(InternalDiagnosticEvent::mechanical(
                DiagnosticMechanism::Palmistry,
                DiagnosticEventState::Recovered,
                Some(DiagnosticCode::WatcherRecoveredRecord),
            ));
        }
    }

    fn start_survivor_forwarder(&self, backend_base_url: String) {
        let shared = Arc::downgrade(&self.shared);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let Some(shared) = shared.upgrade() else {
                    break;
                };
                if shared.stop_sampler.load(Ordering::Acquire) {
                    break;
                }
                InternalDiagnostics { shared }
                    .import_pending_survivors(&backend_base_url)
                    .await;
            }
        });
    }

    pub fn mark_explicit_shutdown(&self) -> io::Result<()> {
        self.shared.stop_sampler.store(true, Ordering::Release);
        self.record(InternalDiagnosticEvent::mechanical(
            DiagnosticMechanism::Heartbeat,
            DiagnosticEventState::Stopped,
            None,
        ));
        write_atomic_durable(&self.shared.paths.shutdown_signal, b"explicit_shutdown\n")
    }

    fn start_resource_sampler(&self, interval: Duration) {
        let shared = Arc::downgrade(&self.shared);
        let _ = thread::Builder::new()
            .name("internal-diagnostics-resource".to_owned())
            .spawn(move || {
                let mut sampler = PlatformResourceSampler::default();
                loop {
                    thread::sleep(interval);
                    let Some(shared) = shared.upgrade() else {
                        break;
                    };
                    let diagnostics = InternalDiagnostics { shared };
                    if diagnostics.shared.stop_sampler.load(Ordering::Acquire) {
                        break;
                    }
                    // WP-1: operator-paused sampling skips the sample + publish but keeps the thread
                    // alive so a later resume needs no respawn. Panic hook / frame-time / Palmistry
                    // are untouched.
                    if diagnostics.shared.sampler_paused.load(Ordering::Acquire) {
                        continue;
                    }
                    let counters = sampler.sample();
                    let unavailable =
                        counters.cpu_percent.is_none() || counters.rss_bytes.is_none();
                    {
                        lock_unpoisoned(&diagnostics.shared.state).resources = counters;
                    }
                    if unavailable {
                        diagnostics.record(InternalDiagnosticEvent::mechanical(
                            DiagnosticMechanism::ResourceSampler,
                            DiagnosticEventState::Degraded,
                            Some(DiagnosticCode::ResourceSampleUnavailable),
                        ));
                    } else {
                        let _ = diagnostics.publish_snapshot();
                    }
                }
            });
    }
}

fn open_transport_signing_secret(
    envelope: &TransportSigningSecretEnvelope,
    transport_secret: &[u8; 32],
    session_id: Uuid,
    launch_nonce: Uuid,
) -> Result<Zeroizing<[u8; 32]>, String> {
    let server_public = decode_hex_array::<32>(&envelope.server_public_key)
        .map_err(|_| "Palmistry returned an invalid transport public key".to_owned())?;
    let nonce = decode_hex_array::<12>(&envelope.nonce)
        .map_err(|_| "Palmistry returned an invalid transport nonce".to_owned())?;
    let mut plaintext = Zeroizing::new(
        hex::decode(&envelope.ciphertext)
            .map_err(|_| "Palmistry returned invalid sealed secret bytes".to_owned())?,
    );
    let shared = Zeroizing::new(
        MontgomeryPoint(server_public)
            .mul_clamped(*transport_secret)
            .to_bytes(),
    );
    let key = palmistry_transport_key(&shared, session_id, launch_nonce);
    let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref())
        .map_err(|_| "cannot initialize Palmistry transport cipher".to_owned())?;
    cipher
        .decrypt_in_place(
            Nonce::from_slice(&nonce),
            palmistry_transport_aad(session_id, launch_nonce).as_slice(),
            &mut *plaintext,
        )
        .map_err(|_| "Palmistry sealed signing secret authentication failed".to_owned())?;
    if plaintext.len() != 32 {
        return Err("Palmistry returned an invalid signing secret length".to_owned());
    }
    let mut signing_secret = Zeroizing::new([0_u8; 32]);
    signing_secret.copy_from_slice(&plaintext);
    Ok(signing_secret)
}

fn palmistry_transport_key(
    shared: &[u8; 32],
    session_id: Uuid,
    launch_nonce: Uuid,
) -> Zeroizing<[u8; 32]> {
    let mut digest = Sha256::new();
    digest.update(b"hsk.palmistry.transport-key@1\0");
    digest.update(shared);
    digest.update(session_id.as_bytes());
    digest.update(launch_nonce.as_bytes());
    Zeroizing::new(digest.finalize().into())
}

fn palmistry_transport_aad(session_id: Uuid, launch_nonce: Uuid) -> [u8; 32] {
    let mut aad = [0_u8; 32];
    aad[..16].copy_from_slice(session_id.as_bytes());
    aad[16..].copy_from_slice(launch_nonce.as_bytes());
    aad
}

fn decode_hex_array<const N: usize>(value: &str) -> Result<[u8; N], hex::FromHexError> {
    let mut bytes = [0_u8; N];
    hex::decode_to_slice(value, &mut bytes)?;
    Ok(bytes)
}

fn recovery_proof(
    current_session_id: Uuid,
    launch_nonce: Uuid,
    summary: &RecoveredSurvivorSummary,
    signing_secret: &[u8; 32],
) -> String {
    use hmac::{Hmac, Mac};

    let mut bytes = Vec::new();
    for value in [
        current_session_id.to_string(),
        launch_nonce.to_string(),
        summary.record_id.to_string(),
        summary.source_session_id.to_string(),
        summary.kind.clone(),
        summary.observed_at_unix_ms.to_string(),
        summary.parent_pid.to_string(),
        summary
            .parent_exit_code
            .map(|value| value.to_string())
            .unwrap_or_default(),
        summary
            .heartbeat_stale_ms
            .map(|value| value.to_string())
            .unwrap_or_default(),
        summary.os_hung_window_confirmed.to_string(),
        summary.minidump_status.clone(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(signing_secret)
        .expect("HMAC accepts a 32-byte Palmistry recovery key");
    mac.update(&bytes);
    hex::encode(mac.finalize().into_bytes())
}

/// Open producer seam for native subsystems that cannot own the diagnostics lifecycle. The event
/// shape is a strict mechanical allowlist; when diagnostics startup failed this is a no-op.
pub fn record_open(event: InternalDiagnosticEvent) -> bool {
    let Some(shared) = ACTIVE_DIAGNOSTICS
        .get()
        .and_then(|slot| lock_unpoisoned(slot).upgrade())
    else {
        return false;
    };
    InternalDiagnostics { shared }.record(event);
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingIdentity {
    pub version: u32,
    pub process_id: u32,
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub panic_pending: bool,
}

pub fn read_ring_identity(path: &Path) -> io::Result<RingIdentity> {
    let file = File::open(path)?;
    let map = unsafe { Mmap::map(&file)? };
    if map.len() != RING_BYTES || &map[0..8] != RING_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid internal_diagnostics ring header",
        ));
    }
    let version = u32::from_le_bytes(map[OFFSET_VERSION..OFFSET_VERSION + 4].try_into().unwrap());
    if version != INTERNAL_DIAGNOSTICS_RING_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported internal_diagnostics ring version",
        ));
    }
    Ok(RingIdentity {
        version,
        process_id: u32::from_le_bytes(map[OFFSET_PID..OFFSET_PID + 4].try_into().unwrap()),
        session_id: Uuid::from_bytes(
            map[OFFSET_SESSION_ID..OFFSET_SESSION_ID + 16]
                .try_into()
                .unwrap(),
        ),
        launch_nonce: Uuid::from_bytes(
            map[OFFSET_LAUNCH_NONCE..OFFSET_LAUNCH_NONCE + 16]
                .try_into()
                .unwrap(),
        ),
        panic_pending: mapped_atomic_u32(&map, OFFSET_PANIC_PENDING).load(Ordering::Acquire) != 0,
    })
}

pub fn read_ring_snapshot(path: &Path) -> io::Result<InternalDiagnosticsSnapshot> {
    let identity = read_ring_identity(path)?;
    read_ring_snapshot_for(
        path,
        identity.session_id,
        identity.process_id,
        identity.launch_nonce,
    )
}

pub fn read_ring_snapshot_for(
    path: &Path,
    expected_session_id: Uuid,
    expected_process_id: u32,
    expected_launch_nonce: Uuid,
) -> io::Result<InternalDiagnosticsSnapshot> {
    let file = File::open(path)?;
    let map = unsafe { Mmap::map(&file)? };
    if map.len() != RING_BYTES || &map[0..8] != RING_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid ring header",
        ));
    }
    let identity = RingIdentity {
        version: u32::from_le_bytes(map[OFFSET_VERSION..OFFSET_VERSION + 4].try_into().unwrap()),
        process_id: u32::from_le_bytes(map[OFFSET_PID..OFFSET_PID + 4].try_into().unwrap()),
        session_id: Uuid::from_bytes(
            map[OFFSET_SESSION_ID..OFFSET_SESSION_ID + 16]
                .try_into()
                .unwrap(),
        ),
        launch_nonce: Uuid::from_bytes(
            map[OFFSET_LAUNCH_NONCE..OFFSET_LAUNCH_NONCE + 16]
                .try_into()
                .unwrap(),
        ),
        panic_pending: mapped_atomic_u32(&map, OFFSET_PANIC_PENDING).load(Ordering::Acquire) != 0,
    };
    if identity.version != INTERNAL_DIAGNOSTICS_RING_VERSION
        || identity.process_id != expected_process_id
        || identity.session_id != expected_session_id
        || identity.launch_nonce != expected_launch_nonce
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ring identity mismatch",
        ));
    }
    for _ in 0..4 {
        let generation_before = mapped_atomic_u64(&map, OFFSET_GENERATION).load(Ordering::Acquire);
        let active_slot =
            mapped_atomic_u32(&map, OFFSET_ACTIVE_SLOT).load(Ordering::Acquire) as usize;
        if active_slot > 1 || generation_before == 0 {
            thread::yield_now();
            continue;
        }
        let slot_offset = RING_HEADER_BYTES + (active_slot * RING_SLOT_BYTES);
        let slot_generation =
            u64::from_le_bytes(map[slot_offset..slot_offset + 8].try_into().unwrap());
        let len =
            u32::from_le_bytes(map[slot_offset + 8..slot_offset + 12].try_into().unwrap()) as usize;
        if slot_generation != generation_before || len > MAX_SLOT_PAYLOAD_BYTES {
            thread::yield_now();
            continue;
        }
        let expected_hash = &map[slot_offset + 12..slot_offset + 44];
        let payload_start = slot_offset + SLOT_HEADER_BYTES;
        let payload = map[payload_start..payload_start + len].to_vec();
        let generation_after = mapped_atomic_u64(&map, OFFSET_GENERATION).load(Ordering::Acquire);
        if generation_before != generation_after
            || sha256_bytes(&payload).as_slice() != expected_hash
        {
            thread::yield_now();
            continue;
        }
        let mut snapshot: InternalDiagnosticsSnapshot = serde_json::from_slice(&payload)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if snapshot.schema_id != INTERNAL_DIAGNOSTICS_SCHEMA_ID
            || snapshot.ring_version != INTERNAL_DIAGNOSTICS_RING_VERSION
            || snapshot.session_id != expected_session_id
            || snapshot.process_id != expected_process_id
            || snapshot.launch_nonce != expected_launch_nonce
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ring snapshot identity mismatch",
            ));
        }
        snapshot.heartbeat_counter =
            mapped_atomic_u64(&map, OFFSET_HEARTBEAT_COUNTER).load(Ordering::Acquire);
        snapshot.heartbeat_unix_ms =
            mapped_atomic_u64(&map, OFFSET_HEARTBEAT_UNIX_MS).load(Ordering::Acquire);
        snapshot.heartbeat_monotonic_ms =
            mapped_atomic_u64(&map, OFFSET_HEARTBEAT_MONOTONIC_MS).load(Ordering::Acquire);
        return Ok(snapshot);
    }
    Err(io::Error::new(
        io::ErrorKind::WouldBlock,
        "internal_diagnostics ring changed during bounded read",
    ))
}

struct MappedRingWriter {
    map: MmapMut,
    panic_map: Option<MmapMut>,
}

impl MappedRingWriter {
    fn create(path: &Path, pid: u32, session_id: Uuid, launch_nonce: Uuid) -> io::Result<Self> {
        let mut options = OpenOptions::new();
        options.create(true).truncate(true).read(true).write(true);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;

            // The backend must authenticate the live ring before it is allowed to launch
            // Palmistry. Permit that independent read while the mapping remains active, but keep
            // write and delete sharing denied so another process cannot replace or mutate the
            // authenticated diagnostics authority.
            options.share_mode(FILE_SHARE_READ);
        }
        let file = options.open(path)?;
        file.set_len(RING_BYTES as u64)?;
        let mut map = unsafe { MmapMut::map_mut(&file)? };
        // Create the lock-free panic view from the already authenticated handle. Opening a second
        // write handle by path after this point would either require widening FILE_SHARE_WRITE to
        // every process or fail on Windows; neither is acceptable for the crash authority.
        let panic_map = unsafe { MmapMut::map_mut(&file)? };
        map.fill(0);
        map[0..8].copy_from_slice(RING_MAGIC);
        map[OFFSET_VERSION..OFFSET_VERSION + 4]
            .copy_from_slice(&INTERNAL_DIAGNOSTICS_RING_VERSION.to_le_bytes());
        map[OFFSET_PID..OFFSET_PID + 4].copy_from_slice(&pid.to_le_bytes());
        map[OFFSET_SESSION_ID..OFFSET_SESSION_ID + 16].copy_from_slice(session_id.as_bytes());
        map[OFFSET_LAUNCH_NONCE..OFFSET_LAUNCH_NONCE + 16].copy_from_slice(launch_nonce.as_bytes());
        map.flush()?;
        Ok(Self {
            map,
            panic_map: Some(panic_map),
        })
    }

    fn take_panic_latch(&mut self) -> io::Result<PanicLatch> {
        let map = self
            .panic_map
            .take()
            .ok_or_else(|| io::Error::other("diagnostics panic mapping was already transferred"))?;
        Ok(PanicLatch {
            map,
            fired: AtomicBool::new(false),
        })
    }

    fn publish(&mut self, snapshot: &InternalDiagnosticsSnapshot) -> io::Result<()> {
        let payload = serde_json::to_vec(snapshot)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if payload.len() > MAX_SLOT_PAYLOAD_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "internal_diagnostics snapshot exceeded bounded ring slot",
            ));
        }
        let generation = self.generation().load(Ordering::Acquire).saturating_add(1);
        let active_slot = (generation & 1) as usize;
        let offset = RING_HEADER_BYTES + (active_slot * RING_SLOT_BYTES);
        self.map[offset..offset + 8].copy_from_slice(&generation.to_le_bytes());
        self.map[offset + 8..offset + 12].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        self.map[offset + 12..offset + 44].copy_from_slice(&sha256_bytes(&payload));
        let payload_start = offset + SLOT_HEADER_BYTES;
        self.map[payload_start..payload_start + payload.len()].copy_from_slice(&payload);
        self.map
            .flush_range(offset, SLOT_HEADER_BYTES + payload.len())?;
        self.active_slot()
            .store(active_slot as u32, Ordering::Release);
        self.generation().store(generation, Ordering::Release);
        self.publish_heartbeat(
            snapshot.heartbeat_counter,
            snapshot.heartbeat_unix_ms,
            snapshot.heartbeat_monotonic_ms,
        );
        self.map.flush_range(0, RING_HEADER_BYTES)?;
        Ok(())
    }

    fn publish_heartbeat(&self, counter: u64, unix_ms: u64, monotonic_ms: u64) {
        self.heartbeat_counter().store(counter, Ordering::Release);
        self.heartbeat_unix_ms().store(unix_ms, Ordering::Release);
        self.heartbeat_monotonic_ms()
            .store(monotonic_ms, Ordering::Release);
    }

    fn active_slot(&self) -> &AtomicU32 {
        mapped_atomic_u32(&self.map, OFFSET_ACTIVE_SLOT)
    }

    fn generation(&self) -> &AtomicU64 {
        mapped_atomic_u64(&self.map, OFFSET_GENERATION)
    }

    fn heartbeat_counter(&self) -> &AtomicU64 {
        mapped_atomic_u64(&self.map, OFFSET_HEARTBEAT_COUNTER)
    }

    fn heartbeat_unix_ms(&self) -> &AtomicU64 {
        mapped_atomic_u64(&self.map, OFFSET_HEARTBEAT_UNIX_MS)
    }

    fn heartbeat_monotonic_ms(&self) -> &AtomicU64 {
        mapped_atomic_u64(&self.map, OFFSET_HEARTBEAT_MONOTONIC_MS)
    }
}

fn snapshot_from_state(
    shared: &SharedDiagnostics,
    state: &RuntimeState,
) -> InternalDiagnosticsSnapshot {
    InternalDiagnosticsSnapshot {
        schema_id: INTERNAL_DIAGNOSTICS_SCHEMA_ID.to_owned(),
        ring_version: INTERNAL_DIAGNOSTICS_RING_VERSION,
        session_id: shared.session_id,
        launch_nonce: shared.launch_nonce,
        process_id: std::process::id(),
        build_id: build_id(),
        heartbeat_counter: state.heartbeat_counter,
        heartbeat_unix_ms: unix_ms(),
        heartbeat_monotonic_ms: duration_ms_u64(shared.started.elapsed()),
        frame_time: frame_stats(&state.frame_samples),
        resources: state.resources.clone(),
        events: state.events.iter().cloned().collect(),
        behavior_observations: state.behavior_observations.iter().cloned().collect(),
    }
}

fn behavior_observation_hmac(
    behavior_id: &str,
    run_id: &str,
    lane_id: &str,
    heartbeat_counter: u64,
    observed_at_unix_ms: u64,
    signing_secret: &[u8],
) -> String {
    use hmac::{Hmac, Mac};

    let mut bytes = Vec::new();
    for value in [
        behavior_id.to_owned(),
        run_id.to_owned(),
        lane_id.to_owned(),
        heartbeat_counter.to_string(),
        observed_at_unix_ms.to_string(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(signing_secret)
        .expect("HMAC accepts the authenticated diagnostics key");
    mac.update(&bytes);
    hex::encode(mac.finalize().into_bytes())
}

fn frame_stats(samples: &VecDeque<u64>) -> FrameTimeStats {
    if samples.is_empty() {
        return FrameTimeStats::default();
    }
    let mut ordered = samples.iter().copied().collect::<Vec<_>>();
    ordered.sort_unstable();
    let percentile = |percent: usize| {
        let index = ((ordered.len() - 1) * percent) / 100;
        ordered[index]
    };
    FrameTimeStats {
        sample_count: samples.len() as u64,
        last_micros: *samples.back().unwrap_or(&0),
        min_micros: ordered[0],
        max_micros: *ordered.last().unwrap_or(&0),
        p50_micros: percentile(50),
        p95_micros: percentile(95),
    }
}

#[derive(Default)]
struct PlatformResourceSampler {
    #[cfg(target_os = "windows")]
    previous_cpu_100ns: Option<u64>,
    #[cfg(target_os = "windows")]
    previous_wall: Option<Instant>,
}

impl PlatformResourceSampler {
    fn sample(&mut self) -> ResourceCounters {
        #[cfg(target_os = "windows")]
        {
            return self.sample_windows();
        }
        #[cfg(not(target_os = "windows"))]
        {
            ResourceCounters::default()
        }
    }

    #[cfg(target_os = "windows")]
    fn sample_windows(&mut self) -> ResourceCounters {
        use windows_sys::Win32::{
            Foundation::FILETIME,
            System::{
                ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
                Threading::{GetCurrentProcess, GetProcessTimes},
            },
        };

        let process = unsafe { GetCurrentProcess() };
        let mut memory = PROCESS_MEMORY_COUNTERS::default();
        memory.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let memory_ok = unsafe {
            GetProcessMemoryInfo(
                process,
                &mut memory,
                std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            )
        } != 0;
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let times_ok =
            unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) }
                != 0;
        let now = Instant::now();
        let cpu_total = filetime_u64(kernel).saturating_add(filetime_u64(user));
        let cpu_percent = if times_ok {
            match (
                self.previous_cpu_100ns.replace(cpu_total),
                self.previous_wall.replace(now),
            ) {
                (Some(previous_cpu), Some(previous_wall)) => {
                    let cpu_delta_seconds =
                        cpu_total.saturating_sub(previous_cpu) as f64 / 10_000_000.0;
                    let wall_seconds = now.saturating_duration_since(previous_wall).as_secs_f64();
                    (wall_seconds > 0.0)
                        .then_some(((cpu_delta_seconds / wall_seconds) * 100.0) as f32)
                }
                _ => None,
            }
        } else {
            None
        };
        ResourceCounters {
            sampled_at_unix_ms: unix_ms(),
            cpu_percent,
            rss_bytes: memory_ok.then_some(memory.WorkingSetSize as u64),
            // Windows does not expose a reliable process GPU percentage through
            // the process APIs used here. Keep the field typed and honest rather
            // than fabricating a zero; a later DXCore provider can mark Sampled.
            gpu_percent: None,
            gpu_status: ResourceMetricStatus::Unavailable,
        }
    }
}

#[cfg(target_os = "windows")]
fn filetime_u64(value: windows_sys::Win32::Foundation::FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

fn diagnostics_root() -> PathBuf {
    if let Some(path) = std::env::var_os("HANDSHAKE_DIAGNOSTICS_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local).join("Handshake").join("diagnostics");
    }
    std::env::temp_dir().join("handshake").join("diagnostics")
}

fn mapped_atomic_u32(bytes: &[u8], offset: usize) -> &AtomicU32 {
    assert_eq!(
        (bytes.as_ptr() as usize + offset) % std::mem::align_of::<AtomicU32>(),
        0
    );
    unsafe { &*(bytes.as_ptr().add(offset) as *const AtomicU32) }
}

fn mapped_atomic_u64(bytes: &[u8], offset: usize) -> &AtomicU64 {
    assert_eq!(
        (bytes.as_ptr() as usize + offset) % std::mem::align_of::<AtomicU64>(),
        0
    );
    unsafe { &*(bytes.as_ptr().add(offset) as *const AtomicU64) }
}

fn build_id() -> String {
    format!(
        "{}-{}",
        env!("CARGO_PKG_VERSION"),
        option_env!("HANDSHAKE_BUILD_DATE").unwrap_or("dev")
    )
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn duration_ms_u64(value: Duration) -> u64 {
    value.as_millis().min(u128::from(u64::MAX)) as u64
}

fn duration_micros_u64(value: Duration) -> u64 {
    value.as_micros().min(u128::from(u64::MAX)) as u64
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hex_sha256(bytes: &[u8]) -> String {
    sha256_bytes(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn panic_payload_class(payload: &(dyn std::any::Any + Send)) -> PanicPayloadClass {
    if payload.is::<String>() {
        PanicPayloadClass::String
    } else if payload.is::<&'static str>() {
        PanicPayloadClass::StaticStr
    } else {
        PanicPayloadClass::Opaque
    }
}

fn redacted_backtrace() -> String {
    Backtrace::force_capture()
        .to_string()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(INTERNAL_DIAGNOSTICS_BACKTRACE_LINE_CAP)
        .enumerate()
        .map(|(index, line)| format!("{index}:{}", hex_sha256(line.trim().as_bytes())))
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_atomic_durable(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join(format!(
        ".{}-{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("diagnostic"),
        Uuid::now_v7()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp, path)?;
    restrict_diagnostics_path(path)?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn scan_survivors(paths: &InternalDiagnosticsPaths) -> io::Result<Vec<RecoveredSurvivorSummary>> {
    const MAX_SURVIVOR_FILES: usize = 256;
    const MAX_SURVIVOR_BYTES: u64 = 2 * 1024 * 1024;
    const MAX_SURVIVOR_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

    let mut records = Vec::new();
    let (paths_to_scan, next_cursor) = select_survivor_scan_page(paths, MAX_SURVIVOR_FILES)?;
    for path in paths_to_scan {
        let Ok(file_type) = fs::symlink_metadata(&path) else {
            quarantine_survivor(paths, &path);
            continue;
        };
        if file_type.file_type().is_symlink() || !file_type.is_file() {
            quarantine_survivor(paths, &path);
            continue;
        }
        let now = SystemTime::now();
        let modified = file_type.modified().ok();
        let timestamp_ok =
            modified.is_some_and(|value| survivor_file_time_is_fresh(now, value, MAX_SURVIVOR_AGE));
        if file_type.len() > MAX_SURVIVOR_BYTES || !timestamp_ok {
            quarantine_survivor(paths, &path);
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            quarantine_survivor(paths, &path);
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            quarantine_survivor(paths, &path);
            continue;
        };
        if value.get("schema_id").and_then(|v| v.as_str()) != Some("hsk.palmistry.survivor@1")
            || !value
                .get("source_proof")
                .and_then(|value| value.as_str())
                .is_some_and(|proof| {
                    proof.len() == 128 && proof.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
        {
            quarantine_survivor(paths, &path);
            continue;
        }
        let Some(record_id) = value
            .get("record_id")
            .and_then(|v| v.as_str())
            .and_then(|v| Uuid::parse_str(v).ok())
        else {
            quarantine_survivor(paths, &path);
            continue;
        };
        let Some(source_session_id) = value
            .get("session_id")
            .or_else(|| value.pointer("/last_snapshot/session_id"))
            .and_then(|v| v.as_str())
            .and_then(|v| Uuid::parse_str(v).ok())
        else {
            quarantine_survivor(paths, &path);
            continue;
        };
        let observed_at_unix_ms = value
            .get("observed_at_unix_ms")
            .and_then(|value| value.as_u64());
        if value.get("kind").and_then(|value| value.as_str()).is_none()
            || !observed_at_unix_ms
                .is_some_and(|value| survivor_observed_at_is_fresh(unix_ms(), value))
            || value
                .get("parent_pid")
                .and_then(|value| value.as_u64())
                .is_none()
            || value
                .get("minidump_status")
                .and_then(|value| value.as_str())
                .is_none()
        {
            quarantine_survivor(paths, &path);
            continue;
        }
        records.push(RecoveredSurvivorSummary {
            record_id,
            source_session_id,
            kind: value
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_owned(),
            observed_at_unix_ms: observed_at_unix_ms.unwrap_or(0),
            parent_pid: value
                .get("parent_pid")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            parent_exit_code: value
                .get("parent_exit_code")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            heartbeat_stale_ms: value.get("heartbeat_stale_ms").and_then(|v| v.as_u64()),
            os_hung_window_confirmed: value
                .get("os_hung_window_confirmed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            minidump_status: value
                .get("minidump_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_owned(),
            imported: paths
                .recovered_dir
                .join(format!("{record_id}.ack"))
                .is_file(),
        });
    }
    write_atomic_durable(
        &paths.recovered_dir.join("survivor-scan.cursor.json"),
        &serde_json::to_vec(&next_cursor)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
    )?;
    records.sort_by_key(|record| (record.observed_at_unix_ms, record.record_id));
    Ok(records)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SurvivorScanCursor {
    byte_offset: u64,
    retry_after_unix_ms: u64,
}

fn select_survivor_scan_page(
    paths: &InternalDiagnosticsPaths,
    limit: usize,
) -> io::Result<(Vec<PathBuf>, SurvivorScanCursor)> {
    const WORK_BUDGET: Duration = Duration::from_millis(25);
    let cursor_path = paths.recovered_dir.join("survivor-scan.cursor.json");
    let mut cursor: SurvivorScanCursor = fs::read(&cursor_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    if unix_ms() < cursor.retry_after_unix_ms {
        return Ok((Vec::new(), cursor));
    }
    let index_path = paths.survivor_dir.join("survivor-index.jsonl");
    let file = match File::open(index_path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            cursor.retry_after_unix_ms = unix_ms().saturating_add(1_000);
            return Ok((Vec::new(), cursor));
        }
        Err(error) => return Err(error),
    };
    let length = file.metadata()?.len();
    if cursor.byte_offset >= length {
        cursor.byte_offset = 0;
    }
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(cursor.byte_offset))?;
    let started = Instant::now();
    let mut selected = Vec::new();
    let mut line = String::new();
    while selected.len() < limit && started.elapsed() < WORK_BUDGET {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            cursor.byte_offset = 0;
            break;
        }
        cursor.byte_offset = reader.stream_position()?;
        let name = line.trim();
        if name.starts_with("survivor-")
            && name.ends_with(".json")
            && name.len() <= 128
            && !name.bytes().any(|byte| matches!(byte, b'/' | b'\\'))
        {
            selected.push(paths.survivor_dir.join(name));
        }
    }
    cursor.retry_after_unix_ms = if selected.is_empty() {
        unix_ms().saturating_add(1_000)
    } else {
        0
    };
    Ok((selected, cursor))
}

fn survivor_file_time_is_fresh(now: SystemTime, modified: SystemTime, max_age: Duration) -> bool {
    match now.duration_since(modified) {
        Ok(age) => age <= max_age,
        Err(_) => false,
    }
}

fn survivor_observed_at_is_fresh(now: u64, observed_at: u64) -> bool {
    const MAX_AGE_MS: u64 = 30 * 24 * 60 * 60 * 1_000;
    const FUTURE_SKEW_MS: u64 = 2 * 60 * 1_000;
    observed_at <= now.saturating_add(FUTURE_SKEW_MS)
        && now.saturating_sub(observed_at) <= MAX_AGE_MS
}

fn quarantine_survivor(paths: &InternalDiagnosticsPaths, path: &Path) {
    let rejected = paths.survivor_dir.join("rejected");
    if fs::create_dir_all(&rejected).is_err() {
        return;
    }
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return;
    };
    let target = rejected.join(format!("{file_name}.rejected-{}", Uuid::now_v7()));
    let _ = fs::rename(path, target);
}

fn quarantine_survivor_record(
    paths: &InternalDiagnosticsPaths,
    summary: &RecoveredSurvivorSummary,
) {
    quarantine_survivor(
        paths,
        &paths.survivor_dir.join(format!(
            "survivor-{}-{}.json",
            summary.observed_at_unix_ms, summary.record_id
        )),
    );
}

#[cfg(not(target_os = "windows"))]
fn restrict_diagnostics_path(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)?;
        let mode = if metadata.is_dir() { 0o700 } else { 0o600 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn restrict_diagnostics_path(path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::{
                ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
            },
            SetFileSecurityW, DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR,
        },
    };
    let sddl: Vec<u16> = std::ffi::OsStr::new("D:P(A;;FA;;;OW)(A;;FA;;;SY)")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let applied = unsafe {
        SetFileSecurityW(
            name.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        )
    };
    unsafe { LocalFree(descriptor.cast()) };
    if applied == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

#[derive(Clone, Default)]
pub struct InternalDiagnosticsPaneFactory {
    diagnostics: Option<InternalDiagnostics>,
}

impl InternalDiagnosticsPaneFactory {
    pub fn new(diagnostics: Option<InternalDiagnostics>) -> Self {
        Self { diagnostics }
    }
}

impl PaneFactory for InternalDiagnosticsPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::Problems
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let heading = ui.heading("Diagnostics");
        set_diagnostic_author_id(ui, heading.id, "problems.diagnostics.surface");
        let Some(diagnostics) = &self.diagnostics else {
            let unavailable = ui.colored_label(
                ui.visuals().warn_fg_color,
                "internal_diagnostics unavailable",
            );
            set_diagnostic_author_id(ui, unavailable.id, "problems.diagnostics.unavailable");
            return;
        };
        let snapshot = diagnostics.snapshot();
        egui::Grid::new("internal-diagnostics-summary")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Session");
                diagnostic_monospace(
                    ui,
                    "problems.diagnostics.session",
                    snapshot.session_id.to_string(),
                );
                ui.end_row();
                ui.label("Heartbeat");
                diagnostic_monospace(
                    ui,
                    "problems.diagnostics.heartbeat",
                    snapshot.heartbeat_counter.to_string(),
                );
                ui.end_row();
                ui.label("Frame p95");
                diagnostic_monospace(
                    ui,
                    "problems.diagnostics.frame-p95",
                    format!("{} us", snapshot.frame_time.p95_micros),
                );
                ui.end_row();
                ui.label("CPU");
                diagnostic_monospace(
                    ui,
                    "problems.diagnostics.cpu",
                    snapshot
                        .resources
                        .cpu_percent
                        .map(|v| format!("{v:.1}%"))
                        .unwrap_or_else(|| "unavailable".to_owned()),
                );
                ui.end_row();
                ui.label("RSS");
                diagnostic_monospace(
                    ui,
                    "problems.diagnostics.rss",
                    snapshot
                        .resources
                        .rss_bytes
                        .map(|v| format!("{} MiB", v / 1_048_576))
                        .unwrap_or_else(|| "unavailable".to_owned()),
                );
                ui.end_row();
                ui.label("GPU");
                diagnostic_monospace(
                    ui,
                    "problems.diagnostics.gpu",
                    snapshot
                        .resources
                        .gpu_percent
                        .map(|value| format!("{value:.1}%"))
                        .unwrap_or_else(|| format!("{:?}", snapshot.resources.gpu_status)),
                );
                ui.end_row();
            });
        ui.separator();
        let recovered_heading = ui.strong("Recovered Palmistry records");
        set_diagnostic_author_id(ui, recovered_heading.id, "problems.diagnostics.palmistry");
        let recovered = diagnostics.recovered_survivors();
        if recovered.is_empty() {
            let empty = ui.label("No recovered crash or freeze records.");
            set_diagnostic_author_id(ui, empty.id, "problems.diagnostics.palmistry.empty");
        } else {
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    for record in recovered.iter().rev() {
                        ui.horizontal(|ui| {
                            let base = format!(
                                "problems.diagnostics.palmistry.record.{}",
                                record.record_id
                            );
                            diagnostic_monospace(ui, &format!("{base}.kind"), record.kind.as_str());
                            diagnostic_label(
                                ui,
                                &format!("{base}.minidump"),
                                format!("minidump: {}", record.minidump_status),
                            );
                            diagnostic_label(
                                ui,
                                &format!("{base}.os-probe"),
                                if record.os_hung_window_confirmed {
                                    "OS hung-window probe: confirmed"
                                } else {
                                    "OS hung-window probe: not confirmed"
                                },
                            );
                            diagnostic_label(
                                ui,
                                &format!("{base}.flight-recorder"),
                                if record.imported {
                                    "Flight Recorder: imported"
                                } else {
                                    "Flight Recorder: pending"
                                },
                            );
                        });
                    }
                });
        }
        ui.separator();
        let events_heading = ui.strong("Recent mechanical events");
        set_diagnostic_author_id(ui, events_heading.id, "problems.diagnostics.events");
        for event in snapshot.events.iter().rev().take(20) {
            diagnostic_monospace(
                ui,
                &format!("problems.diagnostics.event.{}", event.event_id),
                format!(
                    "{:?} · {:?}{}",
                    event.mechanism,
                    event.state,
                    event
                        .code
                        .map(|code| format!(" · {code:?}"))
                        .unwrap_or_default()
                ),
            );
        }
    }
}

fn set_diagnostic_author_id(ui: &egui::Ui, widget_id: egui::Id, author_id: &str) {
    ui.ctx().accesskit_node_builder(widget_id, |node| {
        node.set_author_id(author_id.to_owned());
    });
}

fn diagnostic_label(ui: &mut egui::Ui, author_id: &str, text: impl Into<egui::WidgetText>) {
    let response = ui.label(text);
    set_diagnostic_author_id(ui, response.id, author_id);
}

fn diagnostic_monospace(ui: &mut egui::Ui, author_id: &str, text: impl Into<String>) {
    let response = ui.monospace(text.into());
    set_diagnostic_author_id(ui, response.id, author_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palmistry_supervisor_is_lifetime_scoped_and_never_retains_plaintext_http_body() {
        let source = include_str!("internal_diagnostics.rs");
        assert!(source.contains("pub async fn maintain_palmistry"));
        assert!(source.contains("loop {"));
        assert!(!source.contains("launch_palmistry_with_retry"));
        assert!(!source.contains("let body = response.bytes().await"));
    }

    #[test]
    fn sealed_transport_secret_rejects_tamper_and_wrong_session_binding() {
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let client_secret = Zeroizing::new([3_u8; 32]);
        let server_secret = Zeroizing::new([4_u8; 32]);
        let client_public = MontgomeryPoint::mul_base_clamped(*client_secret);
        let server_public = MontgomeryPoint::mul_base_clamped(*server_secret);
        let shared = Zeroizing::new(client_public.mul_clamped(*server_secret).to_bytes());
        let key = palmistry_transport_key(&shared, session_id, launch_nonce);
        let cipher = ChaCha20Poly1305::new_from_slice(key.as_ref()).unwrap();
        let nonce = [5_u8; 12];
        let mut ciphertext = Zeroizing::new(vec![7_u8; 32]);
        cipher
            .encrypt_in_place(
                Nonce::from_slice(&nonce),
                palmistry_transport_aad(session_id, launch_nonce).as_slice(),
                &mut *ciphertext,
            )
            .unwrap();
        let envelope = TransportSigningSecretEnvelope {
            server_public_key: hex::encode(server_public.as_bytes()),
            nonce: hex::encode(nonce),
            ciphertext: hex::encode(ciphertext.as_slice()),
        };
        assert_eq!(
            open_transport_signing_secret(&envelope, &client_secret, session_id, launch_nonce)
                .unwrap()
                .as_ref(),
            &[7_u8; 32]
        );
        assert!(open_transport_signing_secret(
            &envelope,
            &client_secret,
            Uuid::now_v7(),
            launch_nonce
        )
        .is_err());
        let mut tampered = envelope;
        tampered.ciphertext.replace_range(0..2, "00");
        assert!(
            open_transport_signing_secret(&tampered, &client_secret, session_id, launch_nonce)
                .is_err()
        );
    }

    #[test]
    fn open_event_shape_rejects_content_by_construction() {
        let event = InternalDiagnosticEvent::mechanical(
            DiagnosticMechanism::BackendRoute,
            DiagnosticEventState::Degraded,
            Some(DiagnosticCode::BackendUnavailable),
        );
        let value = serde_json::to_value(event).expect("serialize typed event");
        let object = value.as_object().expect("event object");
        assert_eq!(
            object
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>(),
            [
                "code",
                "event_id",
                "mechanism",
                "observed_at_unix_ms",
                "state"
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
    }

    #[test]
    fn survivor_scan_cursor_reaches_valid_record_after_more_than_1024_shaped_noise_files() {
        let root = tempfile::tempdir().expect("temp diagnostics root");
        let survivor_dir = root.path().join("survivors");
        let recovered_dir = root.path().join("recovered");
        fs::create_dir_all(&survivor_dir).expect("survivor directory");
        fs::create_dir_all(&recovered_dir).expect("recovered directory");
        let session_id = Uuid::now_v7();
        let paths = InternalDiagnosticsPaths {
            root: root.path().to_path_buf(),
            ring: root.path().join("ring.bin"),
            crash_dir: root.path().join("crashes"),
            survivor_dir: survivor_dir.clone(),
            panic_signal: root.path().join("panic.signal.json"),
            panic_ack: root.path().join("panic.ack"),
            shutdown_signal: root.path().join("shutdown.signal"),
            ready_signal: root.path().join("ready.json"),
            recovered_dir,
        };
        for index in 0..1_100 {
            let name = format!("survivor-{index:06}-noise.json");
            fs::write(survivor_dir.join(&name), b"{}").expect("noise fixture");
        }
        let record_id = Uuid::now_v7();
        let valid = serde_json::json!({
            "schema_id": "hsk.palmistry.survivor@1",
            "record_id": record_id,
            "session_id": session_id,
            "kind": "unexpected_exit",
            "observed_at_unix_ms": unix_ms(),
            "parent_pid": 42,
            "parent_exit_code": 1,
            "heartbeat_stale_ms": null,
            "os_hung_window_confirmed": false,
            "minidump_status": "failed_after_exit",
            "source_proof": "ab".repeat(64),
        });
        let valid_name = format!("survivor-{}-{record_id}.json", valid["observed_at_unix_ms"]);
        fs::write(
            survivor_dir.join(&valid_name),
            serde_json::to_vec(&valid).expect("valid survivor JSON"),
        )
        .expect("valid survivor fixture");
        let mut index = String::new();
        for noise in 0..1_100 {
            index.push_str(&format!("survivor-{noise:06}-noise.json\n"));
        }
        index.push_str(&valid_name);
        index.push('\n');
        fs::write(survivor_dir.join("survivor-index.jsonl"), index)
            .expect("durable survivor index");

        let mut found = false;
        for _ in 0..6 {
            found |= scan_survivors(&paths)
                .expect("bounded survivor scan")
                .iter()
                .any(|record| record.record_id == record_id);
        }
        assert!(
            found,
            "rotating scan must eventually reach the valid record"
        );
    }

    #[test]
    fn future_mtime_and_signed_observation_are_rejected() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        assert!(!survivor_file_time_is_fresh(
            now,
            now + Duration::from_secs(1),
            Duration::from_secs(60)
        ));
        let now_ms = 10_000_000;
        assert!(!survivor_observed_at_is_fresh(
            now_ms,
            now_ms + 2 * 60 * 1_000 + 1
        ));
    }

    #[test]
    fn durable_crash_shape_classifies_but_never_serializes_panic_content() {
        let canary = String::from("panic-payload-secret-canary");
        let record = DurableCrashRecord {
            schema_id: "hsk.internal_diagnostics.crash@1".to_owned(),
            crash_id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            process_id: 42,
            observed_at_unix_ms: 7,
            build_id: "test-build".to_owned(),
            payload_class: panic_payload_class(&canary),
            location_file_sha256: Some(hex_sha256(b"private/source/path.rs")),
            location_line: Some(9),
            location_column: Some(3),
            redacted_backtrace: format!("0:{}", hex_sha256(b"private stack frame")),
            last_events: Vec::new(),
        };
        let encoded = serde_json::to_string(&record).expect("serialize crash record");
        assert_eq!(record.payload_class, PanicPayloadClass::String);
        assert!(!encoded.contains(&canary));
        assert!(!encoded.contains("private/source/path.rs"));
        assert!(!encoded.contains("private stack frame"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded).expect("typed crash JSON")
                ["payload_class"],
            "string"
        );
    }

    #[test]
    fn redacted_backtrace_is_bounded_to_hashed_lines() {
        let trace = redacted_backtrace();
        assert!(
            trace.lines().count() <= INTERNAL_DIAGNOSTICS_BACKTRACE_LINE_CAP,
            "redacted crash backtrace must remain bounded"
        );
        for line in trace.lines() {
            let (_, digest) = line.split_once(':').expect("index:digest line");
            assert_eq!(digest.len(), 64);
            assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn mapped_ring_round_trips_and_heartbeat_advances_without_snapshot_rewrite() {
        let dir = tempfile::tempdir().expect("temp diagnostics root");
        let path = dir.path().join("ring-test.bin");
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let mut writer =
            MappedRingWriter::create(&path, 42, session_id, launch_nonce).expect("create ring");
        let mut snapshot = InternalDiagnosticsSnapshot {
            schema_id: INTERNAL_DIAGNOSTICS_SCHEMA_ID.to_owned(),
            ring_version: INTERNAL_DIAGNOSTICS_RING_VERSION,
            session_id,
            launch_nonce,
            process_id: 42,
            build_id: "test".to_owned(),
            heartbeat_counter: 1,
            heartbeat_unix_ms: 2,
            heartbeat_monotonic_ms: 3,
            frame_time: FrameTimeStats::default(),
            resources: ResourceCounters::default(),
            events: Vec::new(),
            behavior_observations: Vec::new(),
        };
        writer.publish(&snapshot).expect("publish snapshot");
        writer.publish_heartbeat(9, 10, 11);
        snapshot = read_ring_snapshot(&path).expect("read ring");
        assert_eq!(snapshot.heartbeat_counter, 9);
        assert_eq!(snapshot.heartbeat_unix_ms, 10);
        assert_eq!(snapshot.heartbeat_monotonic_ms, 11);
        assert!(read_ring_snapshot_for(&path, Uuid::now_v7(), 42, launch_nonce).is_err());
        assert!(read_ring_snapshot_for(&path, session_id, 43, launch_nonce).is_err());
        assert!(read_ring_snapshot_for(&path, session_id, 42, Uuid::now_v7()).is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn mapped_ring_allows_independent_read_but_denies_write_and_delete_until_drop() {
        let dir = tempfile::tempdir().expect("temp diagnostics root");
        let path = dir.path().join("ring-windows-sharing.bin");
        let writer = MappedRingWriter::create(&path, 42, Uuid::now_v7(), Uuid::now_v7())
            .expect("create live mapped ring");

        let bytes = fs::read(&path).expect("backend-style independent read of live ring");
        assert_eq!(bytes.len(), 262_272);
        assert_eq!(bytes.len(), RING_BYTES);
        assert_eq!(&bytes[..8], RING_MAGIC);
        assert!(
            OpenOptions::new().write(true).open(&path).is_err(),
            "live ring must deny a competing writer"
        );
        assert!(
            fs::remove_file(&path).is_err(),
            "live ring must deny delete/path replacement"
        );

        drop(writer);
        fs::remove_file(&path).expect("ring delete succeeds after mapped writer drops");
    }

    #[test]
    fn panic_latch_is_lock_free_and_durably_visible_in_the_ring_header() {
        let dir = tempfile::tempdir().expect("temp diagnostics root");
        let path = dir.path().join("ring-panic.bin");
        let session_id = Uuid::now_v7();
        let nonce = Uuid::now_v7();
        let mut writer =
            MappedRingWriter::create(&path, 77, session_id, nonce).expect("create ring");
        let latch = writer
            .take_panic_latch()
            .expect("transfer independent panic mapping");
        let normal_writer_lock = Mutex::new(writer);
        let _held_normal_writer_lock = normal_writer_lock.lock().unwrap();
        latch.signal();
        assert!(
            read_ring_identity(&path)
                .expect("read header")
                .panic_pending
        );
    }

    #[test]
    fn frame_stats_are_bounded_and_percentile_based() {
        let samples = VecDeque::from([1, 100, 3, 4, 5]);
        let stats = frame_stats(&samples);
        assert_eq!(stats.sample_count, 5);
        assert_eq!(stats.last_micros, 5);
        assert_eq!(stats.min_micros, 1);
        assert_eq!(stats.max_micros, 100);
        assert_eq!(stats.p50_micros, 4);
        assert_eq!(stats.p95_micros, 5);
    }
}
