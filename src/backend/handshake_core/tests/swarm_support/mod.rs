//! Shared helpers for the MT-142 SurrealDB swarm proof targets
//! (`surreal_swarm_semantics_tests`, `surreal_swarm_load_tests`,
//! `surreal_swarm_lifecycle_tests`): deterministic RNG, in-memory oracle,
//! canonical-state reconciliation, machine context, report writer, timeouts,
//! typed-outcome classification and the loopback document API used for the
//! only public rich-document delete path.
//!
//! No self-tests live here (they would be duplicated into every consuming
//! target); every helper is proven by the consuming proof targets.
#![allow(dead_code)]

#[path = "../knowledge_ingestion_support/mod.rs"]
mod knowledge_ingestion_support;

pub use knowledge_ingestion_support::{open_embedded_store, EmbeddedKnowledgeStore};

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use handshake_core::api::knowledge_documents as docs_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeRichDocument, KnowledgeStore, NewKnowledgeRichDocument,
};
use handshake_core::storage::surreal::retry::{classify_storage_error, RetryClass};
use handshake_core::storage::surreal::swarm_load_report::{
    FailureClass, IntegrityEntry, IntegrityVerdict, MachineContext, OperationClass,
    StoreDriveKind,
};
use handshake_core::storage::surreal::{
    RowFilter, SurrealDatabase, SurrealTestInspector, TableSelector,
};
use handshake_core::storage::StorageError;
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::field::{Field, Visit};
use tracing::subscriber::Interest;
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::Registry;

// ---------------------------------------------------------------------------
// Budgets (research basis `numeric_budgets.ci_profile`; every bound explicit).
// ---------------------------------------------------------------------------

/// Per-operation bound: retry budget 2000 ms + one worst-case HDD attempt.
pub const PER_OPERATION_TIMEOUT: Duration = Duration::from_millis(5_000);
/// Per-worker bound for the CI profile.
pub const PER_WORKER_TIMEOUT: Duration = Duration::from_millis(60_000);
/// Whole-test bound for the CI profile.
pub const WHOLE_TEST_TIMEOUT: Duration = Duration::from_millis(180_000);
/// Pinned engine and SDK version (`Cargo.toml` `surrealdb = "=3.2.0"`).
pub const SURREALDB_VERSION: &str = "3.2.0";
/// Fixed workload seed when `HANDSHAKE_SWARM_SEED` is absent.
pub const DEFAULT_WORKLOAD_SEED: u64 = 0x4d54_3134_3200_0001;
/// Schema id of the small JSON fragments written next to the load report.
pub const REPORT_FRAGMENT_SCHEMA_ID: &str = "hsk.surreal_swarm_report_fragment@1";

/// Seed from `HANDSHAKE_SWARM_SEED` (decimal or `0x` hex) else the fixed default.
pub fn workload_seed() -> u64 {
    match std::env::var("HANDSHAKE_SWARM_SEED") {
        Ok(raw) if !raw.trim().is_empty() => {
            let raw = raw.trim();
            let parsed = raw
                .strip_prefix("0x")
                .map(|hex| u64::from_str_radix(hex, 16))
                .unwrap_or_else(|| raw.parse::<u64>());
            parsed.unwrap_or_else(|_| panic!("HANDSHAKE_SWARM_SEED must be a u64, got {raw:?}"))
        }
        _ => DEFAULT_WORKLOAD_SEED,
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64; `rand` is not a crate dependency).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SwarmRng {
    state: u64,
}

impl SwarmRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Independent stream `stream` derived from `seed` (per worker).
    pub fn derive(seed: u64, stream: u64) -> Self {
        let mut mixer = Self::new(seed ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        mixer.next_u64();
        Self::new(mixer.next_u64() | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[0, n)`; `0` when `n == 0`.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    pub fn chance(&mut self, probability: f64) -> bool {
        self.next_f64() < probability
    }
}

// ---------------------------------------------------------------------------
// Document payload builders.
// ---------------------------------------------------------------------------

pub const DOCUMENT_SCHEMA_VERSION: &str = "hsk_richdoc_v1";

/// ProseMirror doc with one paragraph; every payload contains the word
/// `swarm` so the search class has hits.
pub fn document_content(text: &str) -> Value {
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": format!("swarm {text}") }]
        }]
    })
}

pub fn new_document(workspace_id: &str, title: &str, text: &str) -> NewKnowledgeRichDocument {
    NewKnowledgeRichDocument {
        workspace_id: workspace_id.to_owned(),
        document_id: None,
        title: title.to_owned(),
        schema_version: DOCUMENT_SCHEMA_VERSION.to_owned(),
        content_json: document_content(text),
        crdt_document_id: None,
        crdt_snapshot_id: None,
        promotion_receipt_event_id: None,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Hashing.
// ---------------------------------------------------------------------------

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// SHA-256 over the sorted, newline-joined tuples.
pub fn hash_sorted_tuples(mut tuples: Vec<String>) -> String {
    tuples.sort();
    tuples.dedup();
    sha256_hex(tuples.join("\n").as_bytes())
}

// ---------------------------------------------------------------------------
// In-flight gauge (maximum concurrent operations high-water mark).
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct InFlightGauge {
    current: AtomicUsize,
    high_water: AtomicUsize,
}

impl InFlightGauge {
    pub fn enter(&self) -> InFlightGuard<'_> {
        let now = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.high_water.fetch_max(now, Ordering::SeqCst);
        InFlightGuard { gauge: self }
    }

    pub fn current(&self) -> usize {
        self.current.load(Ordering::SeqCst)
    }

    pub fn high_water(&self) -> usize {
        self.high_water.load(Ordering::SeqCst)
    }
}

pub struct InFlightGuard<'a> {
    gauge: &'a InFlightGauge,
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        self.gauge.current.fetch_sub(1, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Typed-outcome classification.
// ---------------------------------------------------------------------------

/// The API promises optimistic concurrency through `StorageError::Conflict`
/// (`HSK-KRD-SAVE-STALE` maps to
/// `Conflict("knowledge rich document version conflict: expected_version is stale")`,
/// `storage/surreal/knowledge.rs`).
pub fn is_typed_conflict(error: &StorageError) -> bool {
    matches!(
        error,
        StorageError::Conflict(_) | StorageError::ConflictDetails { .. }
    )
}

/// A raw engine commit conflict that leaked to the caller untyped
/// (`Transaction conflict: ... This transaction can be retried` inside
/// `StorageError::Database`). Never an acceptable loser outcome (AC-142-4).
pub fn is_untyped_engine_conflict(error: &StorageError) -> bool {
    classify_storage_error(error) == RetryClass::RetryableTransient
}

pub fn is_not_found(error: &StorageError) -> bool {
    matches!(error, StorageError::NotFound(_))
}

/// Typed closed/cancelled outcome after shutdown stopped admission
/// (`SurrealStorageError::Closed` renders `embedded database is closed`).
pub fn is_closed_error(error: &StorageError) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("is closed")
        || text.contains("closed")
        || text.contains("cancelled")
        || text.contains("canceled")
        || text.contains("shutdown")
}

/// Typed retry-exhaustion code the knowledge store returns
/// (`storage/surreal/knowledge.rs` `RETRY_EXHAUSTED_CONFLICT_CODE`).
pub const RETRY_EXHAUSTED_CODE: &str = "HSK-STORAGE-RETRY-EXHAUSTED";
/// Typed keyed-lock wait timeout code (`LOCK_WAIT_TIMEOUT_CONFLICT_CODE`).
pub const LOCK_WAIT_TIMEOUT_CODE: &str = "HSK-STORAGE-LOCK-WAIT-TIMEOUT";

fn conflict_code(error: &StorageError) -> Option<&str> {
    match error {
        StorageError::ConflictDetails { code, .. } => Some(*code),
        StorageError::Conflict(code) => Some(*code),
        _ => None,
    }
}

pub fn is_retry_exhausted(error: &StorageError) -> bool {
    conflict_code(error).is_some_and(|code| code == RETRY_EXHAUSTED_CODE)
        || error.to_string().to_ascii_lowercase().contains("retry exhausted")
}

pub fn is_lock_wait_timeout(error: &StorageError) -> bool {
    conflict_code(error).is_some_and(|code| code == LOCK_WAIT_TIMEOUT_CODE)
}

/// Outcome of one bounded storage operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpOutcome {
    Ok,
    TypedConflict(String),
    UntypedEngineConflict(String),
    NotFound(String),
    Closed(String),
    RetryExhausted(String),
    LockWaitTimeout(String),
    Terminal(String),
    Timeout,
}

impl OpOutcome {
    pub fn from_result<T>(
        result: &Result<Result<T, StorageError>, tokio::time::error::Elapsed>,
    ) -> Self {
        match result {
            Err(_) => Self::Timeout,
            Ok(Ok(_)) => Self::Ok,
            Ok(Err(error)) => Self::from_error(error),
        }
    }

    pub fn from_error(error: &StorageError) -> Self {
        if is_retry_exhausted(error) {
            Self::RetryExhausted(error.to_string())
        } else if is_lock_wait_timeout(error) {
            Self::LockWaitTimeout(error.to_string())
        } else if is_typed_conflict(error) {
            Self::TypedConflict(error.to_string())
        } else if is_untyped_engine_conflict(error) {
            Self::UntypedEngineConflict(error.to_string())
        } else if is_not_found(error) {
            Self::NotFound(error.to_string())
        } else if is_closed_error(error) {
            Self::Closed(error.to_string())
        } else {
            Self::Terminal(error.to_string())
        }
    }

    pub fn failure_class(&self) -> Option<FailureClass> {
        match self {
            Self::Ok => None,
            Self::Timeout => Some(FailureClass::Timeout),
            Self::Closed(_) => Some(FailureClass::Cancelled),
            Self::RetryExhausted(_) => Some(FailureClass::RetryExhausted),
            Self::LockWaitTimeout(_) => Some(FailureClass::LockWaitTimeout),
            Self::TypedConflict(_)
            | Self::UntypedEngineConflict(_)
            | Self::NotFound(_)
            | Self::Terminal(_) => Some(FailureClass::Terminal),
        }
    }

    pub fn is_conflict(&self) -> bool {
        matches!(
            self,
            Self::TypedConflict(_) | Self::UntypedEngineConflict(_)
        )
    }
}

// ---------------------------------------------------------------------------
// Per-class metrics.
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ClassMetrics {
    pub attempted: u64,
    pub succeeded: u64,
    pub failed: BTreeMap<FailureClass, u64>,
    pub latency_ms: Vec<f64>,
}

#[derive(Debug, Default)]
pub struct SwarmMetrics {
    pub by_class: BTreeMap<OperationClass, ClassMetrics>,
    pub conflicts: u64,
    pub untyped_conflicts: Vec<String>,
    pub retry_exhausted_errors: Vec<String>,
    pub lock_wait_timeouts: Vec<String>,
    pub timeouts: u64,
    pub timed_out_classes: Vec<String>,
    pub cancellations: u64,
    pub unexpected_terminal: Vec<String>,
}

impl SwarmMetrics {
    pub fn record(
        &mut self,
        class: OperationClass,
        outcome: &OpOutcome,
        latency: Duration,
        worker: u32,
        operation: u64,
    ) {
        let entry = self.by_class.entry(class).or_default();
        entry.attempted += 1;
        entry.latency_ms.push(latency.as_secs_f64() * 1000.0);
        match outcome {
            OpOutcome::Ok => entry.succeeded += 1,
            other => {
                if let Some(failure) = other.failure_class() {
                    *entry.failed.entry(failure).or_default() += 1;
                }
            }
        }
        if outcome.is_conflict() {
            self.conflicts += 1;
        }
        match outcome {
            OpOutcome::UntypedEngineConflict(text) => self.untyped_conflicts.push(format!(
                "worker {worker} op {operation} {class:?}: {text}"
            )),
            OpOutcome::RetryExhausted(text) => self.retry_exhausted_errors.push(format!(
                "worker {worker} op {operation} {class:?}: {text}"
            )),
            OpOutcome::LockWaitTimeout(text) => self.lock_wait_timeouts.push(format!(
                "worker {worker} op {operation} {class:?}: {text}"
            )),
            OpOutcome::Timeout => {
                self.timeouts += 1;
                self.timed_out_classes
                    .push(format!("worker {worker} op {operation} {class:?}"));
            }
            OpOutcome::Closed(_) => self.cancellations += 1,
            OpOutcome::Terminal(text) => self.unexpected_terminal.push(format!(
                "worker {worker} op {operation} {class:?}: {text}"
            )),
            _ => {}
        }
    }

    pub fn attempted_total(&self) -> u64 {
        self.by_class.values().map(|m| m.attempted).sum()
    }

    pub fn succeeded_total(&self) -> u64 {
        self.by_class.values().map(|m| m.succeeded).sum()
    }

    pub fn merge(&mut self, other: SwarmMetrics) {
        for (class, metrics) in other.by_class {
            let entry = self.by_class.entry(class).or_default();
            entry.attempted += metrics.attempted;
            entry.succeeded += metrics.succeeded;
            for (failure, count) in metrics.failed {
                *entry.failed.entry(failure).or_default() += count;
            }
            entry.latency_ms.extend(metrics.latency_ms);
        }
        self.conflicts += other.conflicts;
        self.untyped_conflicts.extend(other.untyped_conflicts);
        self.retry_exhausted_errors
            .extend(other.retry_exhausted_errors);
        self.lock_wait_timeouts.extend(other.lock_wait_timeouts);
        self.timeouts += other.timeouts;
        self.timed_out_classes.extend(other.timed_out_classes);
        self.cancellations += other.cancellations;
        self.unexpected_terminal.extend(other.unexpected_terminal);
    }
}

// ---------------------------------------------------------------------------
// Retry diagnostics: counts the A1 retry module's structured events
// (`handshake_core::storage::surreal::retry`: "surreal retry scheduled" at
// debug, "surreal retry exhausted" at warn, emitted exactly once per
// exhaustion) through a process-global tracing layer.
// ---------------------------------------------------------------------------

const RETRY_TARGET: &str = "handshake_core::storage::surreal::retry";
const KEYED_LOCK_TARGET: &str = "handshake_core::storage::surreal::keyed_lock";
/// Product warn/error events under this prefix are captured verbatim
/// (bounded) so an HTTP 500 or an untyped store error keeps its cause.
const PRODUCT_TARGET_PREFIX: &str = "handshake_core::";
const PRODUCT_EVENT_CAPACITY: usize = 200;

static RETRY_SCHEDULED: AtomicU64 = AtomicU64::new(0);
static RETRY_EXHAUSTED: AtomicU64 = AtomicU64::new(0);
static RETRY_DIAGNOSTICS_INSTALLED: OnceLock<bool> = OnceLock::new();
static PRODUCT_EVENTS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

/// Product warn/error events captured since the layer was installed.
pub fn captured_product_events() -> Vec<String> {
    PRODUCT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

pub fn clear_captured_product_events() {
    PRODUCT_EVENTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RetryDiagnosticsSnapshot {
    pub scheduled: u64,
    pub exhausted: u64,
}

impl RetryDiagnosticsSnapshot {
    pub fn delta_since(&self, earlier: &RetryDiagnosticsSnapshot) -> RetryDiagnosticsSnapshot {
        RetryDiagnosticsSnapshot {
            scheduled: self.scheduled.saturating_sub(earlier.scheduled),
            exhausted: self.exhausted.saturating_sub(earlier.exhausted),
        }
    }
}

struct RetryEventLayer;

#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: Vec<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.push(format!("{}={value:?}", field.name()));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            self.fields.push(format!("{}={value}", field.name()));
        }
    }
}

fn is_swarm_diagnostic_target(target: &str) -> bool {
    target.starts_with(RETRY_TARGET) || target.starts_with(KEYED_LOCK_TARGET)
}

fn is_product_error_callsite(metadata: &Metadata<'_>) -> bool {
    metadata.target().starts_with(PRODUCT_TARGET_PREFIX)
        && *metadata.level() <= tracing::Level::WARN
}

impl<S: Subscriber> Layer<S> for RetryEventLayer {
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if is_swarm_diagnostic_target(metadata.target()) || is_product_error_callsite(metadata) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        is_swarm_diagnostic_target(metadata.target()) || is_product_error_callsite(metadata)
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        if metadata.target().starts_with(RETRY_TARGET) {
            if visitor.message.contains("retry scheduled") {
                RETRY_SCHEDULED.fetch_add(1, Ordering::SeqCst);
            } else if visitor.message.contains("retry exhausted") {
                RETRY_EXHAUSTED.fetch_add(1, Ordering::SeqCst);
            }
        }
        if is_product_error_callsite(metadata) {
            let mut events = PRODUCT_EVENTS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if events.len() < PRODUCT_EVENT_CAPACITY {
                events.push(format!(
                    "{} {}: {} {}",
                    metadata.level(),
                    metadata.target(),
                    visitor.message,
                    visitor.fields.join(" ")
                ));
            }
        }
    }
}

/// Installs the counting layer as the process-global subscriber once; returns
/// whether this process owns it (false when another subscriber was set first).
pub fn install_retry_diagnostics() -> bool {
    *RETRY_DIAGNOSTICS_INSTALLED.get_or_init(|| {
        let subscriber = Registry::default().with(RetryEventLayer);
        tracing::subscriber::set_global_default(subscriber).is_ok()
    })
}

pub fn retry_diagnostics_snapshot() -> RetryDiagnosticsSnapshot {
    RetryDiagnosticsSnapshot {
        scheduled: RETRY_SCHEDULED.load(Ordering::SeqCst),
        exhausted: RETRY_EXHAUSTED.load(Ordering::SeqCst),
    }
}

// ---------------------------------------------------------------------------
// Bounded futures.
// ---------------------------------------------------------------------------

/// Runs `future` under `bound`; `Err(label)` names what timed out.
pub async fn bounded<T, F>(label: &str, bound: Duration, future: F) -> Result<T, String>
where
    F: Future<Output = T>,
{
    tokio::time::timeout(bound, future)
        .await
        .map_err(|_| format!("{label} exceeded its explicit bound of {} ms", bound.as_millis()))
}

// ---------------------------------------------------------------------------
// Machine context, source commit, RSS (headless subprocesses only).
// ---------------------------------------------------------------------------

fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn powershell_lines(script: &str) -> Vec<String> {
    let output = hidden_command("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn drive_letter(path: &Path) -> Option<char> {
    let text = path.to_string_lossy();
    let mut chars = text.chars();
    let letter = chars.next()?;
    (chars.next() == Some(':') && letter.is_ascii_alphabetic()).then_some(letter)
}

/// Measured host facts for the report (never paths or credentials).
pub fn machine_context(store_path: &Path) -> MachineContext {
    let logical_cpus = std::thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(0);
    let mut cpu_model =
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_owned());
    let mut total_memory_bytes = 0u64;
    let mut os = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let mut store_drive_kind = StoreDriveKind::Unknown;

    if cfg!(windows) {
        let letter = drive_letter(store_path).unwrap_or('D');
        let script = format!(
            "$ErrorActionPreference='SilentlyContinue'; \
             $p=(Get-CimInstance Win32_Processor | Select-Object -First 1).Name; \
             $m=(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory; \
             $o=Get-CimInstance Win32_OperatingSystem; \
             Write-Output ('CPU=' + $p); \
             Write-Output ('MEM=' + $m); \
             Write-Output ('OS=' + $o.Caption + ' ' + $o.Version); \
             $dn=(Get-Partition -DriveLetter '{letter}').DiskNumber; \
             $d=Get-PhysicalDisk | Where-Object {{ \"$($_.DeviceId)\" -eq \"$dn\" }} | Select-Object -First 1; \
             Write-Output ('DRIVE=' + $d.MediaType);"
        );
        for line in powershell_lines(&script) {
            if let Some(value) = line.strip_prefix("CPU=") {
                if !value.is_empty() {
                    cpu_model = value.to_owned();
                }
            } else if let Some(value) = line.strip_prefix("MEM=") {
                total_memory_bytes = value.parse().unwrap_or(0);
            } else if let Some(value) = line.strip_prefix("OS=") {
                if !value.trim().is_empty() {
                    os = value.to_owned();
                }
            } else if let Some(value) = line.strip_prefix("DRIVE=") {
                store_drive_kind = match value.trim().to_ascii_uppercase().as_str() {
                    "HDD" => StoreDriveKind::Hdd,
                    "SSD" => StoreDriveKind::Ssd,
                    _ => StoreDriveKind::Unknown,
                };
            }
        }
    }

    MachineContext {
        cpu_model,
        logical_cpus,
        total_memory_bytes,
        store_drive_kind,
        os,
    }
}

/// `git rev-parse --short=12 HEAD` of the crate's worktree, else `unknown`.
pub fn source_commit() -> String {
    let output = hidden_command("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if text.is_empty() {
                "unknown".to_owned()
            } else {
                text
            }
        }
        _ => "unknown".to_owned(),
    }
}

/// Resident set size of this process in bytes when cheaply available.
pub fn process_rss_bytes() -> Option<u64> {
    if !cfg!(windows) {
        return None;
    }
    let script = format!(
        "$ErrorActionPreference='SilentlyContinue'; Write-Output ((Get-Process -Id {}).WorkingSet64)",
        std::process::id()
    );
    powershell_lines(&script)
        .into_iter()
        .find_map(|line| line.parse::<u64>().ok())
}

// ---------------------------------------------------------------------------
// Report files.
// ---------------------------------------------------------------------------

/// `HANDSHAKE_SWARM_LOAD_REPORT_DIR`, else `HANDSHAKE_ARTIFACTS_ROOT/handshake-test/swarm-load/`.
pub fn report_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("HANDSHAKE_SWARM_LOAD_REPORT_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .expect("HANDSHAKE_ARTIFACTS_ROOT must be set (absolute) for swarm reports");
    PathBuf::from(root)
        .join("handshake-test")
        .join("swarm-load")
}

/// Writes pretty JSON to `<report_dir>/<file_name>` and returns the path.
pub fn write_report_json(file_name: &str, value: &Value) -> PathBuf {
    let dir = report_dir();
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("create report dir {}: {error}", dir.display()));
    let path = dir.join(file_name);
    let text = serde_json::to_string_pretty(value).expect("report serializes");
    std::fs::write(&path, text)
        .unwrap_or_else(|error| panic!("write report {}: {error}", path.display()));
    path
}

pub fn new_run_id(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::now_v7().simple())
}

// ---------------------------------------------------------------------------
// Loopback document API: the only public rich-document delete path
// (`SurrealDatabase::delete_knowledge_rich_document_atomic` is `pub(crate)`).
// ---------------------------------------------------------------------------

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }
    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }
    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(&self, _diag: Diagnostic) -> Result<(), StorageError> {
        Ok(())
    }
    async fn list_problems(&self, _filter: DiagFilter) -> Result<Vec<ProblemGroup>, StorageError> {
        Ok(Vec::new())
    }
    async fn get_diagnostic(&self, _id: uuid::Uuid) -> Result<Diagnostic, StorageError> {
        Err(StorageError::NotFound("diagnostic"))
    }
    async fn list_diagnostics(&self, _filter: DiagFilter) -> Result<Vec<Diagnostic>, StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }
    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn app_state(store: &EmbeddedKnowledgeStore) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: Arc::new(store.db.clone()),
        surreal: store.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt142-swarm".to_owned(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteAck {
    pub receipt_event_id: String,
    pub loom_block_deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeleteFailure {
    /// HTTP 409: typed stale/replay conflict.
    Conflict(String),
    /// HTTP 404: the document was already gone.
    NotFound(String),
    /// Any other HTTP status.
    Other(u16, String),
    Transport(String),
}

/// Loopback axum server over `api::knowledge_documents::routes` (QUIET: no
/// foreground window, loopback only).
pub struct DocApi {
    pub base_url: String,
    pub client: reqwest::Client,
    server: Option<tokio::task::JoinHandle<()>>,
}

impl DocApi {
    pub async fn boot(store: &EmbeddedKnowledgeStore) -> DocApi {
        let app = docs_api::routes(app_state(store));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback document api listener");
        let addr = listener.local_addr().expect("loopback listener addr");
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve loopback document api");
        });
        DocApi {
            base_url: format!("http://{addr}"),
            client: reqwest::Client::new(),
            server: Some(server),
        }
    }

    /// `DELETE /knowledge/documents/:id` as the operator; the route runs the
    /// atomic tombstone + ledger receipt + projection cleanup transaction.
    pub async fn delete_document(
        &self,
        rich_document_id: &str,
        label: &str,
    ) -> Result<DeleteAck, DeleteFailure> {
        let response = self
            .client
            .delete(format!(
                "{}/knowledge/documents/{rich_document_id}",
                self.base_url
            ))
            .header("x-hsk-actor-id", format!("mt142-{label}"))
            .header("x-hsk-kernel-task-run-id", format!("KTR-MT142-{label}"))
            .header("x-hsk-session-run-id", format!("SR-MT142-{label}"))
            .header("x-hsk-actor-kind", "operator")
            .send()
            .await
            .map_err(|error| DeleteFailure::Transport(error.to_string()))?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or(Value::Null);
        match status.as_u16() {
            200 => Ok(DeleteAck {
                receipt_event_id: body["deleted_receipt_event_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                loom_block_deleted: body["loom_block_deleted"].as_bool().unwrap_or(false),
            }),
            409 => Err(DeleteFailure::Conflict(body.to_string())),
            404 => Err(DeleteFailure::NotFound(body.to_string())),
            code => Err(DeleteFailure::Other(code, body.to_string())),
        }
    }

    pub async fn shutdown(mut self) {
        if let Some(handle) = self.server.take() {
            handle.abort();
            let _ = handle.await;
        }
    }
}

impl Drop for DocApi {
    fn drop(&mut self) {
        if let Some(handle) = self.server.take() {
            handle.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// In-memory oracle: what MUST be true after a run.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectIdentity {
    pub rich_document_id: String,
    pub doc_version: i64,
    pub content_sha256: String,
}

impl EffectIdentity {
    pub fn of(document: &KnowledgeRichDocument) -> Self {
        Self {
            rich_document_id: document.rich_document_id.clone(),
            doc_version: document.doc_version,
            content_sha256: document.content_sha256.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Violation {
    pub class: IntegrityVerdict,
    pub detail: String,
}

impl Violation {
    pub fn render(&self) -> String {
        format!("{:?}: {}", self.class, self.detail)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NaturalKeyKind {
    Entity,
    Title,
}

#[derive(Clone, Debug)]
pub struct NaturalKeyRecord {
    pub kind: NaturalKeyKind,
    pub workspace_id: String,
    pub key: String,
    pub id: String,
    pub creations: u64,
    pub observations: u64,
}

#[derive(Debug, Default)]
pub struct DocOracle {
    pub workspace_id: String,
    pub title: String,
    /// Acknowledged `(doc_version, content_sha256)` pairs.
    pub versions: BTreeMap<i64, String>,
    /// A delete was issued; until acknowledged the row may be live or gone.
    pub delete_pending: bool,
    pub deleted: bool,
    pub deleted_receipt_event_id: Option<String>,
}

impl DocOracle {
    pub fn head_version(&self) -> Option<(i64, &str)> {
        self.versions
            .iter()
            .next_back()
            .map(|(version, sha)| (*version, sha.as_str()))
    }
}

#[derive(Debug, Default)]
pub struct Oracle {
    pub docs: BTreeMap<String, DocOracle>,
    pub idempotency: BTreeMap<String, EffectIdentity>,
    pub natural_keys: BTreeMap<String, NaturalKeyRecord>,
    pub violations: Vec<Violation>,
    pub acknowledged_writes: u64,
    pub dirty_read_checks: u64,
}

impl Oracle {
    fn violation(&mut self, class: IntegrityVerdict, detail: String) {
        self.violations.push(Violation { class, detail });
    }

    /// A create acknowledged by the store (version 1 row + projections).
    pub fn ack_create(&mut self, document: &KnowledgeRichDocument) {
        self.acknowledged_writes += 1;
        if self.docs.contains_key(&document.rich_document_id) {
            self.violation(
                IntegrityVerdict::DuplicateEffect,
                format!(
                    "create acknowledged twice for {}",
                    document.rich_document_id
                ),
            );
            return;
        }
        let mut entry = DocOracle {
            workspace_id: document.workspace_id.clone(),
            title: document.title.clone(),
            ..Default::default()
        };
        entry
            .versions
            .insert(document.doc_version, document.content_sha256.clone());
        self.docs.insert(document.rich_document_id.clone(), entry);
    }

    /// A save acknowledged with the returned `doc_version`/`content_sha256`.
    pub fn ack_save(&mut self, document: &KnowledgeRichDocument) {
        self.acknowledged_writes += 1;
        let Some(entry) = self.docs.get_mut(&document.rich_document_id) else {
            self.violation(
                IntegrityVerdict::LostWrite,
                format!(
                    "save acknowledged for unknown document {}",
                    document.rich_document_id
                ),
            );
            return;
        };
        match entry.versions.get(&document.doc_version) {
            Some(existing) if existing != &document.content_sha256 => {
                let detail = format!(
                    "document {} version {} acknowledged twice with different content ({} vs {})",
                    document.rich_document_id, document.doc_version, existing, document.content_sha256
                );
                self.violation(IntegrityVerdict::DuplicateEffect, detail);
            }
            Some(_) => {}
            None => {
                entry
                    .versions
                    .insert(document.doc_version, document.content_sha256.clone());
            }
        }
    }

    /// An idempotent save outcome: the first effect is recorded, every later
    /// outcome (replayed or not) must carry the same identity.
    pub fn ack_idempotent(
        &mut self,
        idempotency_key: &str,
        document: &KnowledgeRichDocument,
        replayed: bool,
    ) {
        let identity = EffectIdentity::of(document);
        match self.idempotency.get(idempotency_key) {
            None => {
                self.idempotency
                    .insert(idempotency_key.to_owned(), identity.clone());
            }
            Some(existing) if existing != &identity => {
                let detail = format!(
                    "idempotency key {idempotency_key} produced divergent effects: {existing:?} vs {identity:?} (replayed={replayed})"
                );
                self.violation(IntegrityVerdict::DuplicateEffect, detail);
                return;
            }
            Some(_) => {}
        }
        // Whether replayed or not the version row exists; recording it through
        // ack_save tolerates the winner and a replayer arriving in either order.
        self.ack_save(document);
    }

    /// A natural-key upsert/create outcome; `created` marks the call that
    /// reported creating the row.
    pub fn note_natural_key(
        &mut self,
        kind: NaturalKeyKind,
        workspace_id: &str,
        key: &str,
        id: &str,
        created: bool,
    ) {
        let composite = format!("{kind:?}|{workspace_id}|{key}");
        match self.natural_keys.get_mut(&composite) {
            None => {
                self.natural_keys.insert(
                    composite,
                    NaturalKeyRecord {
                        kind,
                        workspace_id: workspace_id.to_owned(),
                        key: key.to_owned(),
                        id: id.to_owned(),
                        creations: u64::from(created),
                        observations: 1,
                    },
                );
            }
            Some(record) => {
                record.observations += 1;
                if created {
                    record.creations += 1;
                }
                if record.id != id {
                    let detail = format!(
                        "natural key {composite} resolved to two ids: {} vs {id}",
                        record.id
                    );
                    self.violation(IntegrityVerdict::DuplicateEffect, detail);
                }
            }
        }
    }

    pub fn mark_delete_pending(&mut self, rich_document_id: &str) {
        if let Some(entry) = self.docs.get_mut(rich_document_id) {
            entry.delete_pending = true;
        }
    }

    pub fn ack_delete(&mut self, rich_document_id: &str, receipt_event_id: &str) {
        self.acknowledged_writes += 1;
        let already_deleted = self.docs.get_mut(rich_document_id).map(|entry| {
            let already_deleted = entry.deleted;
            entry.deleted = true;
            entry.delete_pending = false;
            entry.deleted_receipt_event_id = Some(receipt_event_id.to_owned());
            already_deleted
        });
        match already_deleted {
            Some(true) => self.violation(
                IntegrityVerdict::DuplicateEffect,
                format!("delete acknowledged twice for {rich_document_id}"),
            ),
            Some(false) => {}
            None => self.violation(
                IntegrityVerdict::LostWrite,
                format!("delete acknowledged for unknown document {rich_document_id}"),
            ),
        }
    }

    /// A reader observed `document`; the caller proved the version row exists
    /// (dirty-read check) and reports the result here.
    pub fn note_read(&mut self, document: &KnowledgeRichDocument, version_row_matches: bool) {
        self.dirty_read_checks += 1;
        if !version_row_matches {
            let detail = format!(
                "read of {} observed doc_version {} ({}) without a matching committed version row",
                document.rich_document_id, document.doc_version, document.content_sha256
            );
            self.violation(IntegrityVerdict::DirtyRead, detail);
        }
    }

    /// A reader observed `None` for a document the oracle knows.
    pub fn note_missing_read(&mut self, rich_document_id: &str) {
        let legal = self
            .docs
            .get(rich_document_id)
            .is_some_and(|entry| entry.deleted || entry.delete_pending);
        if !legal {
            let detail = format!(
                "read of {rich_document_id} returned None although no delete was issued"
            );
            self.violation(IntegrityVerdict::LostWrite, detail);
        }
    }

    pub fn live_document_count(&self) -> u64 {
        self.docs.values().filter(|entry| !entry.deleted).count() as u64
    }

    pub fn acknowledged_delete_count(&self) -> u64 {
        self.docs.values().filter(|entry| entry.deleted).count() as u64
    }

    pub fn version_row_count(&self) -> u64 {
        self.docs
            .values()
            .map(|entry| entry.versions.len() as u64)
            .sum()
    }

    pub fn entity_key_count(&self) -> u64 {
        self.natural_keys
            .values()
            .filter(|record| record.kind == NaturalKeyKind::Entity)
            .count() as u64
    }

    /// Expected SHA-256 over sorted `(id, doc_version, content_sha256)` for
    /// every acknowledged version row.
    pub fn expected_versions_hash(&self) -> String {
        hash_sorted_tuples(
            self.docs
                .iter()
                .flat_map(|(id, entry)| {
                    entry
                        .versions
                        .iter()
                        .map(move |(version, sha)| format!("{id}|{version}|{sha}"))
                })
                .collect(),
        )
    }

    /// Expected hash over live documents at their head version.
    pub fn expected_documents_hash(&self) -> String {
        hash_sorted_tuples(
            self.docs
                .iter()
                .filter(|(_, entry)| !entry.deleted && !entry.delete_pending)
                .filter_map(|(id, entry)| {
                    entry
                        .head_version()
                        .map(|(version, sha)| format!("{id}|{version}|{sha}"))
                })
                .collect(),
        )
    }
}

// ---------------------------------------------------------------------------
// Canonical-state reconciliation (exact counts and hashes).
// ---------------------------------------------------------------------------

pub const INTEGRITY_TABLES: [&str; 7] = [
    "knowledge_rich_documents",
    "knowledge_rich_document_versions",
    "loom_blocks",
    "loom_block_search_index",
    "knowledge_idempotency_keys",
    "knowledge_entities",
    "kernel_event_ledger",
];

#[derive(Debug)]
pub struct IntegrityOutcome {
    pub verdict: IntegrityVerdict,
    pub violations: Vec<Violation>,
    pub counts_and_hashes: BTreeMap<String, IntegrityEntry>,
    pub documents_checked: u64,
    pub versions_checked: u64,
    pub live_documents_observed: u64,
}

impl IntegrityOutcome {
    pub fn rendered_violations(&self, limit: usize) -> String {
        self.violations
            .iter()
            .take(limit)
            .map(Violation::render)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub async fn table_counts(inspector: &SurrealTestInspector) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for table in INTEGRITY_TABLES {
        let selector = inspector
            .table_selector(table)
            .await
            .unwrap_or_else(|error| panic!("table selector {table}: {error}"));
        let count = inspector
            .row_count(&selector, RowFilter::All)
            .await
            .unwrap_or_else(|error| panic!("row count {table}: {error}"));
        counts.insert(table.to_owned(), count);
    }
    counts
}

async fn selector(inspector: &SurrealTestInspector, table: &str) -> TableSelector {
    inspector
        .table_selector(table)
        .await
        .unwrap_or_else(|error| panic!("table selector {table}: {error}"))
}

async fn exists(
    inspector: &SurrealTestInspector,
    table: &TableSelector,
    id: &str,
) -> Result<bool, String> {
    bounded(
        &format!("inspector exists {}:{id}", table.name()),
        PER_OPERATION_TIMEOUT,
        inspector.exists(table, RowFilter::IdEquals(id.to_owned())),
    )
    .await?
    .map_err(|error| format!("inspector exists {}:{id}: {error}", table.name()))
}

/// Hash over sorted `record_key|field_value` of every row in `table`.
async fn table_hash(inspector: &SurrealTestInspector, table: &TableSelector, field: &str) -> String {
    let field_selector = table
        .field(field)
        .unwrap_or_else(|error| panic!("field {}.{field}: {error}", table.name()));
    let rows = inspector
        .project(table, std::slice::from_ref(&field_selector), RowFilter::All)
        .await
        .unwrap_or_else(|error| panic!("project {}.{field}: {error}", table.name()));
    hash_sorted_tuples(
        rows.into_iter()
            .map(|row| {
                let key = row
                    .record_id
                    .key_string()
                    .map(str::to_owned)
                    .unwrap_or_else(|| row.record_id.key.to_string());
                let value = row
                    .values
                    .get(field)
                    .map(Value::to_string)
                    .unwrap_or_default();
                format!("{key}|{value}")
            })
            .collect(),
    )
}

/// Reconciles the store against the oracle: per-document version chains,
/// projections, idempotency effects, natural keys, exact table counts and
/// content hashes. `baseline` holds the table counts before the workload.
pub async fn reconcile(
    db: &SurrealDatabase,
    inspector: &SurrealTestInspector,
    oracle: &Oracle,
    baseline: &BTreeMap<String, u64>,
) -> IntegrityOutcome {
    let mut violations: Vec<Violation> = oracle.violations.clone();
    let mut push = |class: IntegrityVerdict, detail: String| {
        violations.push(Violation { class, detail });
    };

    let documents = selector(inspector, "knowledge_rich_documents").await;
    let loom_blocks = selector(inspector, "loom_blocks").await;
    let search_index = selector(inspector, "loom_block_search_index").await;
    let idempotency_keys = selector(inspector, "knowledge_idempotency_keys").await;

    let mut document_tuples = Vec::new();
    let mut version_tuples = Vec::new();
    let mut documents_checked = 0u64;
    let mut versions_checked = 0u64;
    let mut live_observed = 0u64;

    for (id, expected) in &oracle.docs {
        documents_checked += 1;
        let live = match bounded(
            &format!("reconcile read {id}"),
            PER_OPERATION_TIMEOUT,
            db.get_knowledge_rich_document(id),
        )
        .await
        {
            Ok(Ok(live)) => live,
            Ok(Err(error)) => {
                push(
                    IntegrityVerdict::LostWrite,
                    format!("reconcile read of {id} failed: {error}"),
                );
                continue;
            }
            Err(timeout) => {
                push(IntegrityVerdict::Timeout, timeout);
                continue;
            }
        };

        match (&live, expected.deleted, expected.delete_pending) {
            (Some(doc), true, _) => push(
                IntegrityVerdict::LostWrite,
                format!(
                    "document {id} is live at version {} although its delete was acknowledged",
                    doc.doc_version
                ),
            ),
            (None, false, false) => push(
                IntegrityVerdict::LostWrite,
                format!("document {id} is missing although no delete was issued"),
            ),
            (None, false, true) => push(
                IntegrityVerdict::PartialCommit,
                format!("document {id} is gone although its delete was never acknowledged"),
            ),
            _ => {}
        }

        if let Some(doc) = &live {
            live_observed += 1;
            match expected.head_version() {
                Some((version, sha)) if version == doc.doc_version && sha == doc.content_sha256 => {}
                Some((version, sha)) => push(
                    IntegrityVerdict::LostWrite,
                    format!(
                        "document {id} head is version {} ({}) but the oracle acknowledged version {version} ({sha})",
                        doc.doc_version, doc.content_sha256
                    ),
                ),
                None => push(
                    IntegrityVerdict::LostWrite,
                    format!("document {id} is live but the oracle holds no acknowledged version"),
                ),
            }
            document_tuples.push(format!(
                "{id}|{}|{}",
                doc.doc_version, doc.content_sha256
            ));
            match exists(inspector, &loom_blocks, id).await {
                Ok(true) => {}
                Ok(false) => push(
                    IntegrityVerdict::PartialCommit,
                    format!("live document {id} has no loom_blocks projection row"),
                ),
                Err(error) => push(IntegrityVerdict::Timeout, error),
            }
            match exists(inspector, &search_index, id).await {
                Ok(true) => {}
                Ok(false) => push(
                    IntegrityVerdict::PartialCommit,
                    format!("live document {id} has no loom_block_search_index row"),
                ),
                Err(error) => push(IntegrityVerdict::Timeout, error),
            }
        } else if expected.deleted {
            match exists(inspector, &documents, id).await {
                Ok(true) => {}
                Ok(false) => push(
                    IntegrityVerdict::LostWrite,
                    format!("deleted document {id} lost its tombstone row"),
                ),
                Err(error) => push(IntegrityVerdict::Timeout, error),
            }
            match exists(inspector, &loom_blocks, id).await {
                Ok(false) => {}
                Ok(true) => push(
                    IntegrityVerdict::PartialCommit,
                    format!("deleted document {id} still has its loom_blocks row"),
                ),
                Err(error) => push(IntegrityVerdict::Timeout, error),
            }
        }

        let rows = match bounded(
            &format!("reconcile versions {id}"),
            PER_OPERATION_TIMEOUT,
            db.list_knowledge_rich_document_versions(id),
        )
        .await
        {
            Ok(Ok(rows)) => rows,
            Ok(Err(error)) => {
                push(
                    IntegrityVerdict::LostWrite,
                    format!("reconcile versions of {id} failed: {error}"),
                );
                continue;
            }
            Err(timeout) => {
                push(IntegrityVerdict::Timeout, timeout);
                continue;
            }
        };
        let stored: BTreeMap<i64, String> = rows
            .iter()
            .map(|row| (row.doc_version, row.content_sha256.clone()))
            .collect();
        if stored.len() != rows.len() {
            push(
                IntegrityVerdict::DuplicateEffect,
                format!("document {id} has duplicate version rows"),
            );
        }
        versions_checked += stored.len() as u64;
        for (version, sha) in &expected.versions {
            match stored.get(version) {
                Some(actual) if actual == sha => {}
                Some(actual) => push(
                    IntegrityVerdict::LostWrite,
                    format!(
                        "document {id} version {version} stores {actual} but {sha} was acknowledged"
                    ),
                ),
                None => push(
                    IntegrityVerdict::LostWrite,
                    format!("document {id} acknowledged version {version} has no version row"),
                ),
            }
        }
        for (version, sha) in &stored {
            if !expected.versions.contains_key(version) {
                push(
                    IntegrityVerdict::PartialCommit,
                    format!(
                        "document {id} has version row {version} ({sha}) that was never acknowledged"
                    ),
                );
            }
            version_tuples.push(format!("{id}|{version}|{sha}"));
        }
        if let Some(doc) = &live {
            if stored.keys().next_back().copied() != Some(doc.doc_version) {
                push(
                    IntegrityVerdict::PartialCommit,
                    format!(
                        "document {id} doc_version {} does not equal its highest version row {:?}",
                        doc.doc_version,
                        stored.keys().next_back()
                    ),
                );
            }
        }
    }

    for (key, identity) in &oracle.idempotency {
        match exists(inspector, &idempotency_keys, key).await {
            Ok(true) => {}
            Ok(false) => push(
                IntegrityVerdict::LostWrite,
                format!("idempotency key {key} has no committed receipt row"),
            ),
            Err(error) => push(IntegrityVerdict::Timeout, error),
        }
        let stored = oracle
            .docs
            .get(&identity.rich_document_id)
            .and_then(|entry| entry.versions.get(&identity.doc_version));
        if stored != Some(&identity.content_sha256) {
            push(
                IntegrityVerdict::DuplicateEffect,
                format!("idempotency key {key} effect {identity:?} is not the acknowledged version"),
            );
        }
    }

    for record in oracle.natural_keys.values() {
        match record.kind {
            NaturalKeyKind::Entity => {
                let found = bounded(
                    &format!("reconcile entity {}", record.key),
                    PER_OPERATION_TIMEOUT,
                    db.get_knowledge_entity_by_identity(
                        &record.workspace_id,
                        KnowledgeEntityKind::Concept,
                        &record.key,
                    ),
                )
                .await;
                match found {
                    Ok(Ok(Some(entity))) if entity.entity_id == record.id => {}
                    Ok(Ok(Some(entity))) => push(
                        IntegrityVerdict::DuplicateEffect,
                        format!(
                            "entity natural key {} resolves to {} but {} was acknowledged",
                            record.key, entity.entity_id, record.id
                        ),
                    ),
                    Ok(Ok(None)) => push(
                        IntegrityVerdict::LostWrite,
                        format!("entity natural key {} has no row", record.key),
                    ),
                    Ok(Err(error)) => push(
                        IntegrityVerdict::LostWrite,
                        format!("entity natural key {} read failed: {error}", record.key),
                    ),
                    Err(timeout) => push(IntegrityVerdict::Timeout, timeout),
                }
            }
            NaturalKeyKind::Title => {
                if record.creations != 1 {
                    push(
                        IntegrityVerdict::DuplicateEffect,
                        format!(
                            "title natural key {} reported created={} times (must be exactly once)",
                            record.key, record.creations
                        ),
                    );
                }
                if !oracle.docs.contains_key(&record.id) {
                    push(
                        IntegrityVerdict::LostWrite,
                        format!(
                            "title natural key {} resolved to {} which the oracle never acknowledged",
                            record.key, record.id
                        ),
                    );
                }
            }
        }
    }

    // Exact table counts relative to the baseline.
    let counts = table_counts(inspector).await;
    let base = |table: &str| baseline.get(table).copied().unwrap_or(0);
    let expect_exact = |table: &str, expected: u64, push: &mut dyn FnMut(IntegrityVerdict, String)| {
        let actual = counts.get(table).copied().unwrap_or(0);
        if actual != expected {
            push(
                IntegrityVerdict::PartialCommit,
                format!("table {table} has {actual} rows, expected exactly {expected}"),
            );
        }
    };
    expect_exact(
        "knowledge_rich_documents",
        base("knowledge_rich_documents") + oracle.docs.len() as u64,
        &mut push,
    );
    expect_exact(
        "knowledge_rich_document_versions",
        base("knowledge_rich_document_versions") + oracle.version_row_count(),
        &mut push,
    );
    expect_exact(
        "loom_blocks",
        base("loom_blocks") + live_observed,
        &mut push,
    );
    expect_exact(
        "knowledge_idempotency_keys",
        base("knowledge_idempotency_keys") + oracle.idempotency.len() as u64,
        &mut push,
    );
    expect_exact(
        "knowledge_entities",
        base("knowledge_entities") + oracle.entity_key_count(),
        &mut push,
    );
    expect_exact(
        "kernel_event_ledger",
        base("kernel_event_ledger") + oracle.acknowledged_delete_count(),
        &mut push,
    );
    {
        // The delete transaction keeps the search-index row of a tombstoned
        // document, so the exact bound is [live, all documents].
        let actual = counts.get("loom_block_search_index").copied().unwrap_or(0);
        let low = base("loom_block_search_index") + live_observed;
        let high = base("loom_block_search_index") + oracle.docs.len() as u64;
        if actual < low || actual > high {
            push(
                IntegrityVerdict::PartialCommit,
                format!("table loom_block_search_index has {actual} rows, expected within [{low}, {high}]"),
            );
        }
    }

    let documents_hash = hash_sorted_tuples(document_tuples);
    let versions_hash = hash_sorted_tuples(version_tuples);
    if versions_hash != oracle.expected_versions_hash() {
        push(
            IntegrityVerdict::LostWrite,
            format!(
                "version-chain hash {versions_hash} differs from the oracle hash {}",
                oracle.expected_versions_hash()
            ),
        );
    }
    if oracle.docs.values().all(|entry| !entry.delete_pending)
        && documents_hash != oracle.expected_documents_hash()
    {
        push(
            IntegrityVerdict::LostWrite,
            format!(
                "live-document hash {documents_hash} differs from the oracle hash {}",
                oracle.expected_documents_hash()
            ),
        );
    }

    let mut counts_and_hashes = BTreeMap::new();
    for (table, field) in [
        ("loom_blocks", "block_id"),
        ("loom_block_search_index", "content_type"),
        ("knowledge_idempotency_keys", "idempotency_key"),
        ("knowledge_entities", "entity_key"),
        ("kernel_event_ledger", "event_id"),
    ] {
        let table_selector = selector(inspector, table).await;
        let content_hash = table_hash(inspector, &table_selector, field).await;
        counts_and_hashes.insert(
            table.to_owned(),
            IntegrityEntry {
                row_count: counts.get(table).copied().unwrap_or(0),
                content_hash: format!("sha256:{content_hash}"),
            },
        );
    }
    counts_and_hashes.insert(
        "knowledge_rich_documents".to_owned(),
        IntegrityEntry {
            row_count: counts
                .get("knowledge_rich_documents")
                .copied()
                .unwrap_or(0),
            content_hash: format!("sha256:{documents_hash}"),
        },
    );
    counts_and_hashes.insert(
        "knowledge_rich_document_versions".to_owned(),
        IntegrityEntry {
            row_count: counts
                .get("knowledge_rich_document_versions")
                .copied()
                .unwrap_or(0),
            content_hash: format!("sha256:{versions_hash}"),
        },
    );

    let verdict = violations
        .first()
        .map(|violation| violation.class)
        .unwrap_or(IntegrityVerdict::Pass);
    IntegrityOutcome {
        verdict,
        violations,
        counts_and_hashes,
        documents_checked,
        versions_checked,
        live_documents_observed: live_observed,
    }
}

/// Sorted set of the table names the reconciliation hashes (for fragments).
pub fn integrity_table_names() -> BTreeSet<&'static str> {
    INTEGRITY_TABLES.into_iter().collect()
}
