//! Host GUI observations through the existing typed diagnostics ring.
//! Counter meanings below require review together with each consuming callsite.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use handshake_diag_ring::{DiagEventCode, DiagPhase, DiagSeverity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum GuiDiagnosticOperation {
    SettingsTab = 0x4755_4900_0000_0001,
    SettingsOutcome = 0x4755_4900_0000_0002,
    EngineProfileRefresh = 0x4755_4900_0000_0003,
    EngineProfileMutation = 0x4755_4900_0000_0004,
    RendererBinding = 0x4755_4900_0000_0005,
    HandpickInput = 0x4755_4900_0000_0006,
    HandpickRetrieval = 0x4755_4900_0000_0007,
    HandpickNavigation = 0x4755_4900_0000_0008,
    HandpickWriting = 0x4755_4900_0000_000e,
    KumihimoOwner = 0x4755_4900_0000_0009,
    KumihimoLayout = 0x4755_4900_0000_000a,
    KumihimoInteraction = 0x4755_4900_0000_000b,
    KumihimoCleanup = 0x4755_4900_0000_000c,
    KumihimoWindowRetired = 0x4755_4900_0000_000d,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum GuiDiagnosticPhase {
    Accepted = 1,
    Pending = 2,
    Completed = 3,
    Rejected = 4,
    Stale = 5,
    Unavailable = 6,
    Failed = 7,
}

static OBSERVATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static OBSERVATION_START: OnceLock<Instant> = OnceLock::new();

/// Submits a mechanical observation only after an existing ring writer is installed.
/// The return value is not a delivery, watcher, persistence or backend-success receipt.
/// No setting values, query text, credentials or user content are accepted.
pub fn record_gui_observation(
    operation: GuiDiagnosticOperation,
    observation: GuiDiagnosticPhase,
) -> bool {
    if !super::has_ring_writer() {
        return false;
    }
    let sequence = OBSERVATION_SEQUENCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| Some(value.saturating_add(1)))
        .unwrap_or_else(|value| value);
    let timestamp_nanos = OBSERVATION_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_nanos()
        .min(u64::MAX as u128) as u64;
    let (phase, severity) = match observation {
        GuiDiagnosticPhase::Accepted | GuiDiagnosticPhase::Completed => (DiagPhase::End, DiagSeverity::Info),
        GuiDiagnosticPhase::Pending => (DiagPhase::Start, DiagSeverity::Info),
        GuiDiagnosticPhase::Rejected | GuiDiagnosticPhase::Stale | GuiDiagnosticPhase::Unavailable | GuiDiagnosticPhase::Failed => (DiagPhase::Degraded, DiagSeverity::Warn),
    };
    super::record_with(
        DiagEventCode::Other, phase, severity, 0, sequence,
        operation as u64, observation as u64, 0, timestamp_nanos,
    );
    true
}
