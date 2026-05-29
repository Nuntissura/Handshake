use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_ledger::ReclaimableProcess;

pub const FOCUS_AUDIT_LEDGER_DIR: &str = "focus_audit";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusAuditEvent {
    pub run_id: String,
    pub timestamp_utc: DateTime<Utc>,
    pub hwnd: String,
    pub pid: u32,
    pub exe_name: Option<String>,
    #[serde(default)]
    pub expected_foreground: bool,
}

#[derive(Debug, Clone)]
pub struct FocusAuditLedger {
    run_id: String,
    path: PathBuf,
    events: Arc<Mutex<Vec<FocusAuditEvent>>>,
}

impl FocusAuditLedger {
    pub fn new(
        run_id: impl Into<String>,
        runtime_root: impl AsRef<Path>,
    ) -> Result<Self, FocusAuditError> {
        let run_id = run_id.into();
        let safe_run_id = sanitize_run_id(&run_id);
        if safe_run_id.is_empty() {
            return Err(FocusAuditError::InvalidRunId(run_id));
        }
        let dir = runtime_root
            .as_ref()
            .join("gov_runtime")
            .join(FOCUS_AUDIT_LEDGER_DIR);
        fs::create_dir_all(&dir)?;
        Ok(Self {
            run_id,
            path: dir.join(format!("{safe_run_id}.jsonl")),
            events: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, event: FocusAuditEvent) -> Result<(), FocusAuditError> {
        let line = serde_json::to_string(&event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{line}")?;
        self.events
            .lock()
            .map_err(|_| FocusAuditError::PoisonedLedger)?
            .push(event);
        Ok(())
    }

    pub fn events(&self) -> Result<Vec<FocusAuditEvent>, FocusAuditError> {
        Ok(self
            .events
            .lock()
            .map_err(|_| FocusAuditError::PoisonedLedger)?
            .clone())
    }
}

pub trait OwnedProcessPidSource {
    fn owns_pid(&self, pid: u32) -> bool;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnedProcessPidSet {
    pids: HashSet<u32>,
}

impl OwnedProcessPidSet {
    pub fn new(pids: HashSet<u32>) -> Self {
        Self { pids }
    }

    pub fn from_reclaimable<'a>(
        processes: impl IntoIterator<Item = &'a ReclaimableProcess>,
    ) -> Self {
        Self {
            pids: processes
                .into_iter()
                .filter_map(|process| process.os_pid)
                .collect(),
        }
    }

    pub fn insert(&mut self, pid: u32) {
        self.pids.insert(pid);
    }

    pub fn is_empty(&self) -> bool {
        self.pids.is_empty()
    }
}

impl OwnedProcessPidSource for OwnedProcessPidSet {
    fn owns_pid(&self, pid: u32) -> bool {
        self.pids.contains(&pid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusAuditReport {
    pub run_id: String,
    pub total_events: usize,
    pub handshake_owned_events: Vec<FocusAuditEvent>,
    pub foreign_events: Vec<FocusAuditEvent>,
    pub expected_foreground_events: Vec<FocusAuditEvent>,
}

impl FocusAuditReport {
    pub fn from_events(
        run_id: impl Into<String>,
        current_pid: u32,
        owned_processes: &impl OwnedProcessPidSource,
        events: Vec<FocusAuditEvent>,
    ) -> Self {
        let run_id = run_id.into();
        let mut foreign_events = Vec::new();
        let mut handshake_owned_events = Vec::new();
        let mut expected_foreground_events = Vec::new();

        for event in events {
            if event.expected_foreground {
                expected_foreground_events.push(event);
            } else if event.pid == current_pid || owned_processes.owns_pid(event.pid) {
                handshake_owned_events.push(event);
            } else {
                foreign_events.push(event);
            }
        }

        Self {
            run_id,
            total_events: handshake_owned_events.len()
                + foreign_events.len()
                + expected_foreground_events.len(),
            handshake_owned_events,
            foreign_events,
            expected_foreground_events,
        }
    }
}

pub fn assert_no_handshake_foreground(
    report: &FocusAuditReport,
) -> Result<(), FocusAuditViolation> {
    if report.handshake_owned_events.is_empty() {
        return Ok(());
    }
    Err(FocusAuditViolation {
        run_id: report.run_id.clone(),
        owned_events: report.handshake_owned_events.clone(),
    })
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{count} Handshake-owned foreground event(s) during run {run_id}", count = owned_events.len())]
pub struct FocusAuditViolation {
    pub run_id: String,
    pub owned_events: Vec<FocusAuditEvent>,
}

#[derive(Debug, Error)]
pub enum FocusAuditError {
    #[error("FOCUS_AUDIT_INVALID_RUN_ID: {0}")]
    InvalidRunId(String),
    #[error("FOCUS_AUDIT_IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("FOCUS_AUDIT_JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("FOCUS_AUDIT_LEDGER_LOCK_POISONED")]
    PoisonedLedger,
    #[error("FOCUS_AUDIT_UNSUPPORTED_PLATFORM")]
    UnsupportedPlatform,
    #[error("FOCUS_AUDIT_HOOK: {0}")]
    Hook(String),
    #[error("FOCUS_AUDIT_TASK_JOIN: {0}")]
    TaskJoin(String),
}

pub fn sanitize_run_id(run_id: &str) -> String {
    let mut safe = String::with_capacity(run_id.len());
    let mut previous_dash = false;

    for character in run_id.chars() {
        let output = if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            character
        } else {
            '-'
        };

        if output == '-' {
            if !previous_dash {
                safe.push(output);
            }
            previous_dash = true;
        } else {
            safe.push(output);
            previous_dash = false;
        }
    }

    safe.trim_matches('-').to_string()
}

#[cfg(windows)]
mod platform {
    use std::{path::Path, ptr};

    use chrono::Utc;
    use tokio::task::JoinHandle;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::GetWindowThreadProcessId,
    };
    use wineventhook::{raw_event, EventFilter, WindowEvent, WindowEventHook};

    use super::{
        FocusAuditError, FocusAuditEvent, FocusAuditLedger, FocusAuditReport, OwnedProcessPidSet,
    };

    pub struct FocusAuditHandle {
        hook: Option<WindowEventHook>,
        task: JoinHandle<Result<(), FocusAuditError>>,
        ledger: FocusAuditLedger,
        current_pid: u32,
        owned_processes: OwnedProcessPidSet,
    }

    impl FocusAuditHandle {
        pub async fn start(
            run_id: impl Into<String>,
            runtime_root: impl AsRef<Path>,
            owned_processes: OwnedProcessPidSet,
        ) -> Result<Self, FocusAuditError> {
            let run_id = run_id.into();
            let ledger = FocusAuditLedger::new(run_id.clone(), runtime_root)?;
            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
            let hook = WindowEventHook::hook(
                EventFilter::default()
                    .event(raw_event::SYSTEM_FOREGROUND)
                    .skip_own_process(true),
                event_tx,
            )
            .await
            .map_err(|error| FocusAuditError::Hook(error.to_string()))?;
            let task_ledger = ledger.clone();
            let task_run_id = run_id.clone();
            let task = tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    if let Some(focus_event) = focus_event_from_window_event(&task_run_id, &event) {
                        task_ledger.append(focus_event)?;
                    }
                }
                Ok(())
            });

            Ok(Self {
                hook: Some(hook),
                task,
                ledger,
                current_pid: std::process::id(),
                owned_processes,
            })
        }

        pub async fn stop(mut self) -> Result<FocusAuditReport, FocusAuditError> {
            if let Some(hook) = self.hook.take() {
                hook.unhook()
                    .await
                    .map_err(|error| FocusAuditError::Hook(error.to_string()))?;
            }
            self.task
                .await
                .map_err(|error| FocusAuditError::TaskJoin(error.to_string()))??;
            let events = self.ledger.events()?;
            Ok(FocusAuditReport::from_events(
                self.ledger.run_id(),
                self.current_pid,
                &self.owned_processes,
                events,
            ))
        }

        pub fn ledger_path(&self) -> &Path {
            self.ledger.path()
        }
    }

    fn focus_event_from_window_event(run_id: &str, event: &WindowEvent) -> Option<FocusAuditEvent> {
        let hwnd = event
            .window_handle()
            .map_or(ptr::null_mut(), |handle| handle.as_ptr());
        if hwnd.is_null() {
            return None;
        }
        let pid = pid_for_window(hwnd)?;
        Some(FocusAuditEvent {
            run_id: run_id.to_string(),
            timestamp_utc: Utc::now(),
            hwnd: format_hwnd(hwnd),
            pid,
            exe_name: process_exe_name(pid),
            expected_foreground: false,
        })
    }

    fn pid_for_window(hwnd: windows_sys::Win32::Foundation::HWND) -> Option<u32> {
        let mut pid = 0u32;
        let thread_id = unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        if thread_id == 0 || pid == 0 {
            None
        } else {
            Some(pid)
        }
    }

    fn process_exe_name(pid: u32) -> Option<String> {
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 32_768];
        let mut size = buffer.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
        let _ = unsafe { CloseHandle(handle) };
        if ok == 0 || size == 0 {
            return None;
        }

        buffer.truncate(size as usize);
        let full_path = String::from_utf16_lossy(&buffer);
        Some(
            std::path::Path::new(&full_path)
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
                .unwrap_or(full_path),
        )
    }

    fn format_hwnd(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
        format!("0x{:016X}", hwnd as usize)
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    use super::{FocusAuditError, FocusAuditReport, OwnedProcessPidSet};

    pub struct FocusAuditHandle;

    impl FocusAuditHandle {
        pub async fn start(
            _run_id: impl Into<String>,
            _runtime_root: impl AsRef<Path>,
            _owned_processes: OwnedProcessPidSet,
        ) -> Result<Self, FocusAuditError> {
            Err(FocusAuditError::UnsupportedPlatform)
        }

        pub async fn stop(self) -> Result<FocusAuditReport, FocusAuditError> {
            Err(FocusAuditError::UnsupportedPlatform)
        }

        pub fn ledger_path(&self) -> &Path {
            Path::new("")
        }
    }
}

pub use platform::FocusAuditHandle;
