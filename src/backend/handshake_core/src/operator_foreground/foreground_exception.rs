use std::{
    fmt,
    fs::{self, OpenOptions},
    future::pending,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::timeout;
use uuid::Uuid;

use super::focus_audit::sanitize_run_id;

pub const FOREGROUND_EXCEPTION_LOG_DIR: &str = "foreground_log";
pub const FOREGROUND_WARNING_REQUEST_EVENT_TYPE: &str = "FOREGROUND_WARNING_REQUEST";
pub const DIAG_BANNER_REQUEST_EVENT_TYPE: &str = "DIAG_BANNER_REQUEST";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundPacketPolicy {
    pub wp_id: String,
    pub requires_foreground: bool,
}

impl ForegroundPacketPolicy {
    pub fn new(wp_id: impl Into<String>, requires_foreground: bool) -> Self {
        Self {
            wp_id: wp_id.into(),
            requires_foreground,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundWarningRequest {
    pub event_type: &'static str,
    pub diagnostics_event_type: &'static str,
    pub exception_id: Uuid,
    pub wp_id: String,
    pub reason: String,
    pub max_duration_ms: u64,
    pub notification_title: String,
    pub notification_body: String,
    pub diagnostics_banner_body: String,
    pub timestamp_utc: DateTime<Utc>,
}

pub trait ForegroundWarningSink {
    fn emit_warning(
        &self,
        request: ForegroundWarningRequest,
    ) -> Result<(), ForegroundExceptionError>;
}

#[derive(Debug, Default)]
pub struct RecordingForegroundWarningSink {
    requests: Mutex<Vec<ForegroundWarningRequest>>,
}

impl RecordingForegroundWarningSink {
    pub fn requests(&self) -> Vec<ForegroundWarningRequest> {
        self.requests
            .lock()
            .expect("foreground warning requests lock")
            .clone()
    }
}

impl ForegroundWarningSink for RecordingForegroundWarningSink {
    fn emit_warning(
        &self,
        request: ForegroundWarningRequest,
    ) -> Result<(), ForegroundExceptionError> {
        self.requests
            .lock()
            .map_err(|_| ForegroundExceptionError::PoisonedState)?
            .push(request);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ForegroundExceptionHandle {
    exception_id: Uuid,
    wp_id: String,
    reason: String,
    max_duration: Duration,
    log_path: PathBuf,
    ended: Arc<AtomicBool>,
}

pub struct ForegroundException;

impl ForegroundException {
    pub fn declare(
        packet: ForegroundPacketPolicy,
        reason: impl Into<String>,
        max_duration: Duration,
        repo_root: impl AsRef<Path>,
        warning_sink: &impl ForegroundWarningSink,
    ) -> Result<ForegroundExceptionHandle, ForegroundExceptionError> {
        let reason = reason.into();
        if !packet.requires_foreground {
            return Err(ForegroundExceptionError::MissingPacketDeclaration {
                wp_id: packet.wp_id,
            });
        }
        if packet.wp_id.trim().is_empty() {
            return Err(ForegroundExceptionError::InvalidWpId);
        }
        if reason.trim().is_empty() {
            return Err(ForegroundExceptionError::InvalidReason);
        }
        if max_duration.is_zero() {
            return Err(ForegroundExceptionError::InvalidDuration);
        }

        let exception_id = Uuid::now_v7();
        let log_path = foreground_log_path(repo_root.as_ref(), &packet.wp_id);
        let handle = ForegroundExceptionHandle {
            exception_id,
            wp_id: packet.wp_id,
            reason: reason.trim().to_string(),
            max_duration,
            log_path,
            ended: Arc::new(AtomicBool::new(false)),
        };

        handle.append_row("FOREGROUND_EXCEPTION_START", None, None, None)?;
        warning_sink.emit_warning(handle.warning_request())?;
        Ok(handle)
    }

    pub fn has_active_declaration(
        repo_root: impl AsRef<Path>,
        wp_id: &str,
        exception_id: Uuid,
    ) -> Result<bool, ForegroundExceptionError> {
        if wp_id.trim().is_empty() {
            return Err(ForegroundExceptionError::InvalidWpId);
        }
        let log_path = foreground_log_path(repo_root.as_ref(), wp_id);
        if !log_path.exists() {
            return Ok(false);
        }

        let mut saw_start = false;
        for line in fs::read_to_string(&log_path)?.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let row: ForegroundExceptionLogRow = serde_json::from_str(line)?;
            if row.wp_id == wp_id && row.exception_id == exception_id {
                match row.event_type.as_str() {
                    "FOREGROUND_EXCEPTION_START" if row.expected_foreground => saw_start = true,
                    "FOREGROUND_EXCEPTION_END" => return Ok(false),
                    _ => {}
                }
            }
        }
        Ok(saw_start)
    }
}

impl ForegroundExceptionHandle {
    pub fn exception_id(&self) -> Uuid {
        self.exception_id
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Build a bounded controlled window backed by an in-process recording
    /// surface (no real display). Use this on headless / seam-level paths and in
    /// tests; the app layer uses [`Self::bounded_window_with_surface`] to attach
    /// a real Tauri-backed window.
    pub fn bounded_window(
        &self,
        label: impl Into<String>,
        url: impl Into<String>,
    ) -> Result<ControlledWindow, ForegroundExceptionError> {
        self.bounded_window_with_surface(
            label,
            url,
            Arc::new(RecordingControlledWindowSurface::shown_and_focused()),
        )
    }

    /// Build a bounded controlled window over a caller-supplied real-window
    /// surface. The `surface` MUST already have created/shown/focused a real
    /// bounded window before this returns — `visible()`/`focused()` report the
    /// live surface state, and `auto_dismiss_at_deadline`/`dismiss` close it.
    pub fn bounded_window_with_surface(
        &self,
        label: impl Into<String>,
        url: impl Into<String>,
        surface: Arc<dyn ControlledWindowSurface>,
    ) -> Result<ControlledWindow, ForegroundExceptionError> {
        let label = label.into();
        let url = url.into();
        if label.trim().is_empty() {
            return Err(ForegroundExceptionError::InvalidWindowLabel);
        }
        if url.trim().is_empty() {
            return Err(ForegroundExceptionError::InvalidWindowUrl);
        }

        self.append_row(
            "CONTROLLED_WINDOW_OPEN",
            Some(label.as_str()),
            Some(url.as_str()),
            None,
        )?;

        Ok(ControlledWindow {
            exception_id: self.exception_id,
            wp_id: self.wp_id.clone(),
            reason: self.reason.clone(),
            max_duration: self.max_duration,
            log_path: self.log_path.clone(),
            ended: Arc::clone(&self.ended),
            dismissed: Arc::new(AtomicBool::new(false)),
            label,
            url,
            surface,
        })
    }

    fn warning_request(&self) -> ForegroundWarningRequest {
        ForegroundWarningRequest {
            event_type: FOREGROUND_WARNING_REQUEST_EVENT_TYPE,
            diagnostics_event_type: DIAG_BANNER_REQUEST_EVENT_TYPE,
            exception_id: self.exception_id,
            wp_id: self.wp_id.clone(),
            reason: self.reason.clone(),
            max_duration_ms: duration_ms(self.max_duration),
            notification_title: "Foreground interaction requested".to_string(),
            notification_body: format!(
                "{} requests foreground control for at most {} ms.",
                self.wp_id,
                duration_ms(self.max_duration)
            ),
            diagnostics_banner_body: format!(
                "{} declared HBR-QUIET-004 foreground interaction: {}",
                self.wp_id, self.reason
            ),
            timestamp_utc: Utc::now(),
        }
    }

    fn append_row(
        &self,
        event_type: impl Into<String>,
        window_label: Option<&str>,
        window_url: Option<&str>,
        dismissal_reason: Option<&str>,
    ) -> Result<(), ForegroundExceptionError> {
        append_row(
            &self.log_path,
            ForegroundExceptionLogRow {
                event_type: event_type.into(),
                timestamp_utc: Utc::now(),
                exception_id: self.exception_id,
                wp_id: self.wp_id.clone(),
                reason: self.reason.clone(),
                max_duration_ms: duration_ms(self.max_duration),
                expected_foreground: true,
                window_label: window_label.map(str::to_string),
                window_url: window_url.map(str::to_string),
                dismissal_reason: dismissal_reason.map(str::to_string),
            },
        )
    }
}

/// Real-window seam for the controlled foreground-exception window.
///
/// `handshake_core` cannot depend on `tauri`, so the app layer implements this
/// trait over a real `tauri::WebviewWindow`. The implementation is responsible
/// for actually CREATING, SHOWING, and FOCUSING a bounded window, for reporting
/// its live visibility/focus state, and for CLOSING it on dismissal. A
/// logging-only path is no longer permitted: spec §6.6.7 requires a shown,
/// focused, bounded window with real auto-dismiss.
///
/// All methods are synchronous and may be invoked from inside async dismissal;
/// implementations that must hop to the UI/main thread should block internally
/// (the app surface does this via `run_on_main_thread`).
pub trait ControlledWindowSurface: Send + Sync {
    /// True once the real window has been created and shown.
    fn is_visible(&self) -> Result<bool, ForegroundExceptionError>;
    /// True once the real window has received focus.
    fn is_focused(&self) -> Result<bool, ForegroundExceptionError>;
    /// Close/destroy the real window. Must be idempotent-safe: it is only ever
    /// called once by `ControlledWindow` (guarded by the `dismissed` flag), but
    /// closing an already-closed window must not panic.
    fn close(&self) -> Result<(), ForegroundExceptionError>;
}

impl fmt::Debug for dyn ControlledWindowSurface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ControlledWindowSurface")
    }
}

/// Test/headless surface that records lifecycle transitions without a real
/// display. Used by seam-level tests and by `bounded_window` callers that have
/// no Tauri runtime (the real window is wired at the app layer via
/// [`ForegroundExceptionHandle::bounded_window_with_surface`]).
#[derive(Debug, Default)]
pub struct RecordingControlledWindowSurface {
    shown: AtomicBool,
    focused: AtomicBool,
    closed: AtomicBool,
    close_calls: std::sync::atomic::AtomicU32,
}

impl RecordingControlledWindowSurface {
    /// Build a surface that is already shown + focused, modelling a window the
    /// app layer created and brought to the foreground before handing control
    /// back to the bounded-window state machine.
    pub fn shown_and_focused() -> Self {
        Self {
            shown: AtomicBool::new(true),
            focused: AtomicBool::new(true),
            closed: AtomicBool::new(false),
            close_calls: std::sync::atomic::AtomicU32::new(0),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    pub fn close_calls(&self) -> u32 {
        self.close_calls.load(Ordering::SeqCst)
    }
}

impl ControlledWindowSurface for RecordingControlledWindowSurface {
    fn is_visible(&self) -> Result<bool, ForegroundExceptionError> {
        // A closed window is no longer visible.
        Ok(self.shown.load(Ordering::SeqCst) && !self.closed.load(Ordering::SeqCst))
    }

    fn is_focused(&self) -> Result<bool, ForegroundExceptionError> {
        Ok(self.focused.load(Ordering::SeqCst) && !self.closed.load(Ordering::SeqCst))
    }

    fn close(&self) -> Result<(), ForegroundExceptionError> {
        self.close_calls.fetch_add(1, Ordering::SeqCst);
        self.closed.store(true, Ordering::SeqCst);
        self.focused.store(false, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Clone)]
pub struct ControlledWindow {
    exception_id: Uuid,
    wp_id: String,
    reason: String,
    max_duration: Duration,
    log_path: PathBuf,
    ended: Arc<AtomicBool>,
    dismissed: Arc<AtomicBool>,
    label: String,
    url: String,
    surface: Arc<dyn ControlledWindowSurface>,
}

impl fmt::Debug for ControlledWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlledWindow")
            .field("exception_id", &self.exception_id)
            .field("wp_id", &self.wp_id)
            .field("label", &self.label)
            .field("url", &self.url)
            .field("max_duration", &self.max_duration)
            .field("dismissed", &self.dismissed.load(Ordering::SeqCst))
            .finish()
    }
}

impl ControlledWindow {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    /// Live visibility of the real bounded window, read from the surface. A
    /// dismissed/closed window reports `false`.
    pub fn visible(&self) -> bool {
        if self.dismissed.load(Ordering::SeqCst) {
            return false;
        }
        self.surface.is_visible().unwrap_or(false)
    }

    /// Live focus state of the real bounded window, read from the surface. A
    /// dismissed/closed window reports `false`.
    pub fn focused(&self) -> bool {
        if self.dismissed.load(Ordering::SeqCst) {
            return false;
        }
        self.surface.is_focused().unwrap_or(false)
    }

    pub fn expected_foreground(&self) -> bool {
        true
    }

    /// Wait for the bounded deadline, then CLOSE the real window and record the
    /// end row. The close is enforced via `tokio::time::timeout` against the
    /// configured `max_duration` -- it is not best-effort: even if the window
    /// never closes itself, the timeout fires and `dismiss` tears it down.
    pub async fn auto_dismiss_at_deadline(
        &self,
    ) -> Result<ControlledWindowDismissal, ForegroundExceptionError> {
        let _ = timeout(self.max_duration, pending::<()>()).await;
        self.dismiss("auto-dismiss-timeout")
    }

    pub fn dismiss(
        &self,
        dismissal_reason: impl Into<String>,
    ) -> Result<ControlledWindowDismissal, ForegroundExceptionError> {
        let dismissal_reason = dismissal_reason.into();
        if self.dismissed.swap(true, Ordering::SeqCst) {
            return Err(ForegroundExceptionError::AlreadyDismissed {
                label: self.label.clone(),
            });
        }

        // Tear down the real window first so it cannot outlive its bound, then
        // record the audit rows. The dismissed flag is already set, so a failure
        // here still leaves the window in a terminal state for the audit trail.
        self.surface.close()?;

        self.append_row(
            "CONTROLLED_WINDOW_DISMISSED",
            Some(dismissal_reason.as_str()),
        )?;
        if !self.ended.swap(true, Ordering::SeqCst) {
            self.append_row("FOREGROUND_EXCEPTION_END", Some(dismissal_reason.as_str()))?;
        }

        Ok(ControlledWindowDismissal {
            label: self.label.clone(),
            reason: dismissal_reason,
            timestamp_utc: Utc::now(),
        })
    }

    fn append_row(
        &self,
        event_type: impl Into<String>,
        dismissal_reason: Option<&str>,
    ) -> Result<(), ForegroundExceptionError> {
        append_row(
            &self.log_path,
            ForegroundExceptionLogRow {
                event_type: event_type.into(),
                timestamp_utc: Utc::now(),
                exception_id: self.exception_id,
                wp_id: self.wp_id.clone(),
                reason: self.reason.clone(),
                max_duration_ms: duration_ms(self.max_duration),
                expected_foreground: true,
                window_label: Some(self.label.clone()),
                window_url: Some(self.url.clone()),
                dismissal_reason: dismissal_reason.map(str::to_string),
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlledWindowDismissal {
    pub label: String,
    pub reason: String,
    pub timestamp_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundExceptionLogRow {
    pub event_type: String,
    pub timestamp_utc: DateTime<Utc>,
    pub exception_id: Uuid,
    pub wp_id: String,
    pub reason: String,
    pub max_duration_ms: u64,
    pub expected_foreground: bool,
    pub window_label: Option<String>,
    pub window_url: Option<String>,
    pub dismissal_reason: Option<String>,
}

#[derive(Debug, Error)]
pub enum ForegroundExceptionError {
    #[error("FOREGROUND_REQUIRES_PACKET_DECLARATION: packet {wp_id} must set requires_foreground=true before any foreground run")]
    MissingPacketDeclaration { wp_id: String },
    #[error("FOREGROUND_INVALID_WP_ID")]
    InvalidWpId,
    #[error("FOREGROUND_INVALID_REASON")]
    InvalidReason,
    #[error("FOREGROUND_INVALID_DURATION")]
    InvalidDuration,
    #[error("FOREGROUND_INVALID_WINDOW_LABEL")]
    InvalidWindowLabel,
    #[error("FOREGROUND_INVALID_WINDOW_URL")]
    InvalidWindowUrl,
    #[error("FOREGROUND_WINDOW_ALREADY_DISMISSED: {label}")]
    AlreadyDismissed { label: String },
    #[error("FOREGROUND_WINDOW_SURFACE: {0}")]
    WindowSurface(String),
    #[error("FOREGROUND_IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("FOREGROUND_JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("FOREGROUND_STATE_LOCK_POISONED")]
    PoisonedState,
}

fn foreground_log_path(repo_root: &Path, wp_id: &str) -> PathBuf {
    repo_root
        .join(".GOV")
        .join("runtime")
        .join(FOREGROUND_EXCEPTION_LOG_DIR)
        .join(format!("{}.jsonl", sanitize_run_id(wp_id)))
}

fn append_row(path: &Path, row: ForegroundExceptionLogRow) -> Result<(), ForegroundExceptionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(&row)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
