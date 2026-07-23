//! Palmistry: Handshake's out-of-process GUI crash/freeze watcher (Master Spec §6.13).
//!
//! Palmistry is deliberately passive: it reads the frontend's file-backed
//! diagnostics ring, waits on the parent process handle, and writes only typed
//! mechanical survivor records. It never reads project files or payloads.

use crate::internal_diagnostics::{
    read_ring_identity, read_ring_snapshot_for, InternalDiagnosticsSnapshot,
};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use zeroize::Zeroizing;

pub const PALMISTRY_RECORD_SCHEMA_ID: &str = "hsk.palmistry.survivor@1";
pub const PALMISTRY_OBSERVATION_SCHEMA_ID: &str = "hsk.palmistry.observation@1";
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);
pub const DEFAULT_STALE_HEARTBEAT: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct FreezeEpisodeState {
    last_heartbeat: Option<u64>,
    recorded: bool,
}

impl FreezeEpisodeState {
    fn new(last_heartbeat: Option<u64>) -> Self {
        Self {
            last_heartbeat,
            recorded: false,
        }
    }

    fn observe(&mut self, heartbeat: u64) -> bool {
        if self.last_heartbeat == Some(heartbeat) {
            return false;
        }
        self.last_heartbeat = Some(heartbeat);
        self.recorded = false;
        true
    }

    fn should_record(&self) -> bool {
        !self.recorded
    }

    fn mark_recorded(&mut self) {
        self.recorded = true;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PalmistryConfig {
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub parent_pid: u32,
    pub ring: PathBuf,
    pub survivor_dir: PathBuf,
    pub panic_signal: PathBuf,
    pub panic_ack: PathBuf,
    pub shutdown_signal: PathBuf,
    pub ready_signal: PathBuf,
    pub poll_interval: Duration,
    pub stale_heartbeat: Duration,
}

impl PalmistryConfig {
    pub fn parse<I, S>(arguments: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut parent_pid = None;
        let mut session_id = None;
        let mut launch_nonce = None;
        let mut ring = None;
        let mut survivor_dir = None;
        let mut panic_signal = None;
        let mut panic_ack = None;
        let mut shutdown_signal = None;
        let mut ready_signal = None;
        let mut poll_interval = DEFAULT_POLL_INTERVAL;
        let mut stale_heartbeat = DEFAULT_STALE_HEARTBEAT;
        let mut values = arguments.into_iter().map(Into::into);
        while let Some(flag) = values.next() {
            let value = values
                .next()
                .ok_or_else(|| format!("missing value for {flag}"))?;
            match flag.as_str() {
                "--session-id" => session_id = Uuid::parse_str(&value).ok(),
                "--launch-nonce" => launch_nonce = Uuid::parse_str(&value).ok(),
                "--parent-pid" => {
                    parent_pid = Some(value.parse::<u32>().map_err(|_| "invalid --parent-pid")?)
                }
                "--ring" => ring = Some(PathBuf::from(value)),
                "--survivor-dir" => survivor_dir = Some(PathBuf::from(value)),
                "--panic-signal" => panic_signal = Some(PathBuf::from(value)),
                "--panic-ack" => panic_ack = Some(PathBuf::from(value)),
                "--shutdown-signal" => shutdown_signal = Some(PathBuf::from(value)),
                "--ready-signal" => ready_signal = Some(PathBuf::from(value)),
                "--poll-ms" => {
                    let millis = value.parse::<u64>().map_err(|_| "invalid --poll-ms")?;
                    poll_interval = Duration::from_millis(millis.clamp(10, 5_000));
                }
                "--stale-ms" => {
                    let millis = value.parse::<u64>().map_err(|_| "invalid --stale-ms")?;
                    stale_heartbeat = Duration::from_millis(millis.clamp(100, 60_000));
                }
                _ => return Err(format!("unknown argument: {flag}")),
            }
        }
        Ok(Self {
            session_id: session_id.ok_or("missing or invalid --session-id")?,
            launch_nonce: launch_nonce.ok_or("missing or invalid --launch-nonce")?,
            parent_pid: parent_pid.ok_or("missing --parent-pid")?,
            ring: ring.ok_or("missing --ring")?,
            survivor_dir: survivor_dir.ok_or("missing --survivor-dir")?,
            panic_signal: panic_signal.ok_or("missing --panic-signal")?,
            panic_ack: panic_ack.ok_or("missing --panic-ack")?,
            shutdown_signal: shutdown_signal.ok_or("missing --shutdown-signal")?,
            ready_signal: ready_signal.ok_or("missing --ready-signal")?,
            poll_interval,
            stale_heartbeat,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurvivorKind {
    Panic,
    UnexpectedExit,
    GuiFreeze,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinidumpStatus {
    Written,
    FailedWhileRunning,
    FailedAfterExit,
    Unsupported,
    NotRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashClassification {
    Panic,
    AccessViolation,
    Abort,
    HardKillFixture,
    UnexpectedExit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PalmistrySurvivorRecord {
    pub schema_id: String,
    pub record_id: Uuid,
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub watcher_pid: u32,
    pub watcher_creation_time_100ns: u64,
    pub kind: SurvivorKind,
    pub observed_at_unix_ms: u64,
    pub parent_pid: u32,
    pub parent_exit_code: Option<u32>,
    pub heartbeat_stale_ms: Option<u64>,
    pub os_hung_window_confirmed: bool,
    pub minidump_status: MinidumpStatus,
    pub crash_classification: Option<CrashClassification>,
    pub minidump_file_name: Option<String>,
    pub last_snapshot: Option<InternalDiagnosticsSnapshot>,
    pub source_proof: String,
}

pub fn run_from_env() -> Result<(), String> {
    let config = PalmistryConfig::parse(std::env::args().skip(1))?;
    let signing_secret = read_watcher_signing_secret(std::io::stdin())
        .map_err(|error| format!("Palmistry signing-key bootstrap failed: {error}"))?;
    run(config, signing_secret.as_ref()).map_err(|error| error.to_string())
}

fn read_watcher_signing_secret(mut input: impl Read) -> io::Result<Zeroizing<[u8; 32]>> {
    let mut secret = Zeroizing::new([0_u8; 32]);
    input.read_exact(secret.as_mut())?;
    let mut extra = [0_u8; 1];
    if input.read(&mut extra)? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Palmistry signing-key pipe contained trailing bytes",
        ));
    }
    Ok(secret)
}

pub fn run(config: PalmistryConfig, signing_secret: &[u8]) -> io::Result<()> {
    fs::create_dir_all(&config.survivor_dir)?;
    if signing_secret.len() != 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Palmistry requires an exact 32-byte watcher signing key",
        ));
    }
    let process = ParentProcess::open(config.parent_pid)?;
    let watcher_pid = std::process::id();
    let watcher_creation_time_100ns = current_process_creation_time_100ns()?;
    let identity = read_ring_identity(&config.ring)?;
    if identity.session_id != config.session_id
        || identity.launch_nonce != config.launch_nonce
        || identity.process_id != config.parent_pid
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ring identity mismatch",
        ));
    }
    let mut last_snapshot = read_ring_snapshot_for(
        &config.ring,
        config.session_id,
        config.parent_pid,
        config.launch_nonce,
    )
    .ok();
    if let Some(snapshot) = &last_snapshot {
        write_observation(
            &config,
            snapshot,
            watcher_pid,
            watcher_creation_time_100ns,
            signing_secret,
        )?;
    }
    write_ready(
        &config,
        watcher_pid,
        watcher_creation_time_100ns,
        signing_secret,
    )?;
    let mut freeze_episode =
        FreezeEpisodeState::new(last_snapshot.as_ref().map(|value| value.heartbeat_counter));
    let mut last_heartbeat_change = Instant::now();
    let mut panic_recorded = false;
    let mut parent_exit_recorded = false;

    loop {
        if config.shutdown_signal.is_file() {
            return Ok(());
        }

        if let Ok(snapshot) = read_ring_snapshot_for(
            &config.ring,
            config.session_id,
            config.parent_pid,
            config.launch_nonce,
        ) {
            let _ = write_observation(
                &config,
                &snapshot,
                watcher_pid,
                watcher_creation_time_100ns,
                signing_secret,
            );
            if freeze_episode.observe(snapshot.heartbeat_counter) {
                last_heartbeat_change = Instant::now();
            }
            last_snapshot = Some(snapshot);
        }

        let panic_pending = read_ring_identity(&config.ring)
            .map(|value| value.panic_pending)
            .unwrap_or(false);
        if (panic_pending || config.panic_signal.is_file()) && !panic_recorded {
            let (minidump_status, minidump_file_name) =
                write_minidump(&process, config.parent_pid, &config.survivor_dir, false);
            write_survivor(
                &config,
                PalmistrySurvivorRecord {
                    schema_id: PALMISTRY_RECORD_SCHEMA_ID.to_owned(),
                    record_id: Uuid::now_v7(),
                    session_id: config.session_id,
                    launch_nonce: config.launch_nonce,
                    watcher_pid,
                    watcher_creation_time_100ns,
                    kind: SurvivorKind::Panic,
                    observed_at_unix_ms: unix_ms(),
                    parent_pid: config.parent_pid,
                    parent_exit_code: None,
                    heartbeat_stale_ms: None,
                    os_hung_window_confirmed: false,
                    minidump_status,
                    crash_classification: Some(CrashClassification::Panic),
                    minidump_file_name,
                    last_snapshot: last_snapshot.clone(),
                    source_proof: String::new(),
                },
                signing_secret,
            )?;
            write_atomic(&config.panic_ack, b"minidump_attempt_complete\n")?;
            panic_recorded = true;
        }

        let stale_for = last_heartbeat_change.elapsed();
        if freeze_episode.should_record()
            && stale_for >= config.stale_heartbeat
            && os_confirms_hung_window(config.parent_pid)
        {
            write_survivor(
                &config,
                PalmistrySurvivorRecord {
                    schema_id: PALMISTRY_RECORD_SCHEMA_ID.to_owned(),
                    record_id: Uuid::now_v7(),
                    session_id: config.session_id,
                    launch_nonce: config.launch_nonce,
                    watcher_pid,
                    watcher_creation_time_100ns,
                    kind: SurvivorKind::GuiFreeze,
                    observed_at_unix_ms: unix_ms(),
                    parent_pid: config.parent_pid,
                    parent_exit_code: None,
                    heartbeat_stale_ms: Some(duration_ms(stale_for)),
                    os_hung_window_confirmed: true,
                    minidump_status: MinidumpStatus::NotRequested,
                    crash_classification: None,
                    minidump_file_name: None,
                    last_snapshot: last_snapshot.clone(),
                    source_proof: String::new(),
                },
                signing_secret,
            )?;
            freeze_episode.mark_recorded();
        }

        if parent_exit_recorded {
            thread::sleep(config.poll_interval);
            continue;
        }
        match process.wait(config.poll_interval)? {
            ProcessWait::Running => {}
            ProcessWait::Exited(exit_code) => {
                if config.shutdown_signal.is_file() {
                    return Ok(());
                }
                if !panic_recorded {
                    let classification = classify_exit_code(exit_code);
                    let (minidump_status, minidump_file_name) =
                        write_minidump(&process, config.parent_pid, &config.survivor_dir, true);
                    write_survivor(
                        &config,
                        PalmistrySurvivorRecord {
                            schema_id: PALMISTRY_RECORD_SCHEMA_ID.to_owned(),
                            record_id: Uuid::now_v7(),
                            session_id: config.session_id,
                            launch_nonce: config.launch_nonce,
                            watcher_pid,
                            watcher_creation_time_100ns,
                            kind: SurvivorKind::UnexpectedExit,
                            observed_at_unix_ms: unix_ms(),
                            parent_pid: config.parent_pid,
                            parent_exit_code: Some(exit_code),
                            heartbeat_stale_ms: None,
                            os_hung_window_confirmed: false,
                            minidump_status,
                            crash_classification: Some(classification),
                            minidump_file_name,
                            last_snapshot: last_snapshot.clone(),
                            source_proof: String::new(),
                        },
                        signing_secret,
                    )?;
                }
                parent_exit_recorded = true;
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn current_process_creation_time_100ns() -> io::Result<u64> {
    use windows_sys::Win32::{
        Foundation::FILETIME,
        System::Threading::{GetCurrentProcess, GetProcessTimes},
    };

    let process = unsafe { GetCurrentProcess() };
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    if unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
}

#[cfg(not(target_os = "windows"))]
fn current_process_creation_time_100ns() -> io::Result<u64> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Palmistry process-generation identity is currently available only on Windows",
    ))
}

#[derive(Serialize)]
struct PalmistryObservationRecord {
    schema_id: &'static str,
    session_id: Uuid,
    launch_nonce: Uuid,
    heartbeat_counter: u64,
    watcher_pid: u32,
    observed_at_unix_ms: u64,
    behavior_observations: Vec<crate::internal_diagnostics::BehaviorObservation>,
    watcher_creation_time_100ns: u64,
    source_proof: String,
}

fn write_observation(
    config: &PalmistryConfig,
    snapshot: &InternalDiagnosticsSnapshot,
    watcher_pid: u32,
    watcher_creation_time_100ns: u64,
    signing_secret: &[u8],
) -> io::Result<()> {
    let path = config
        .ring
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "ring has no parent"))?
        .join(format!("observation-{}.json", config.session_id));
    let mut record = PalmistryObservationRecord {
        schema_id: PALMISTRY_OBSERVATION_SCHEMA_ID,
        session_id: config.session_id,
        launch_nonce: config.launch_nonce,
        heartbeat_counter: snapshot.heartbeat_counter,
        watcher_pid,
        observed_at_unix_ms: unix_ms(),
        behavior_observations: snapshot.behavior_observations.clone(),
        watcher_creation_time_100ns,
        source_proof: String::new(),
    };
    record.source_proof = source_proof(&record, signing_secret)?;
    let bytes = serde_json::to_vec(&record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_atomic(&path, &bytes)
}

fn write_survivor(
    config: &PalmistryConfig,
    mut record: PalmistrySurvivorRecord,
    signing_secret: &[u8],
) -> io::Result<()> {
    record.source_proof = source_proof(&record, signing_secret)?;
    let bytes = serde_json::to_vec_pretty(&record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let file_name = format!(
        "survivor-{}-{}.json",
        record.observed_at_unix_ms, record.record_id
    );
    write_atomic(&config.survivor_dir.join(&file_name), &bytes)?;
    let mut index = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.survivor_dir.join("survivor-index.jsonl"))?;
    writeln!(index, "{file_name}")?;
    index.sync_all()
}

fn source_proof<T: Serialize>(record: &T, signing_secret: &[u8]) -> io::Result<String> {
    let mut value = serde_json::to_value(record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "record is not an object"))?;
    object.insert(
        "source_proof".to_owned(),
        serde_json::Value::String(String::new()),
    );
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let signing_secret: &[u8; 32] = signing_secret
        .try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid watcher signing key"))?;
    Ok(hex::encode(
        SigningKey::from_bytes(signing_secret)
            .sign(&bytes)
            .to_bytes(),
    ))
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct PalmistryReadyRecord {
    schema_id: &'static str,
    session_id: Uuid,
    launch_nonce: Uuid,
    parent_pid: u32,
    watcher_pid: u32,
    watcher_creation_time_100ns: u64,
    source_proof: String,
}

fn write_ready(
    config: &PalmistryConfig,
    watcher_pid: u32,
    watcher_creation_time_100ns: u64,
    signing_secret: &[u8],
) -> io::Result<()> {
    let mut record = PalmistryReadyRecord {
        schema_id: "hsk.palmistry.ready@1",
        session_id: config.session_id,
        launch_nonce: config.launch_nonce,
        parent_pid: config.parent_pid,
        watcher_pid,
        watcher_creation_time_100ns,
        source_proof: String::new(),
    };
    record.source_proof = source_proof(&record, signing_secret)?;
    let bytes = serde_json::to_vec(&record)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_atomic(&config.ready_signal, &bytes)
}

fn classify_exit_code(exit_code: u32) -> CrashClassification {
    match exit_code {
        0xC000_0005 => CrashClassification::AccessViolation,
        0xC000_0409 | 0x4000_0015 | 3 => CrashClassification::Abort,
        0xDEAD_DEAD => CrashClassification::HardKillFixture,
        _ => CrashClassification::UnexpectedExit,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessWait {
    Running,
    Exited(u32),
}

#[cfg(target_os = "windows")]
struct ParentProcess(windows_sys::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl ParentProcess {
    fn open(pid: u32) -> io::Result<Self> {
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_VM_READ,
        };
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_VM_READ,
                0,
                pid,
            )
        };
        if handle.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }

    fn wait(&self, timeout: Duration) -> io::Result<ProcessWait> {
        use windows_sys::Win32::{
            Foundation::{WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::Threading::{GetExitCodeProcess, WaitForSingleObject},
        };
        match unsafe {
            WaitForSingleObject(self.0, duration_ms(timeout).min(u32::MAX as u64) as u32)
        } {
            WAIT_TIMEOUT => Ok(ProcessWait::Running),
            WAIT_OBJECT_0 => {
                let mut exit_code = 0;
                if unsafe { GetExitCodeProcess(self.0, &mut exit_code) } == 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(ProcessWait::Exited(exit_code))
                }
            }
            WAIT_FAILED => Err(io::Error::last_os_error()),
            result => Err(io::Error::other(format!(
                "unexpected process wait result {result}"
            ))),
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for ParentProcess {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(target_os = "linux")]
struct ParentProcess {
    pid: u32,
    start_time: u64,
}

#[cfg(target_os = "linux")]
impl ParentProcess {
    fn open(pid: u32) -> io::Result<Self> {
        Ok(Self {
            pid,
            start_time: linux_process_start_time(pid)?,
        })
    }

    fn wait(&self, timeout: Duration) -> io::Result<ProcessWait> {
        thread::sleep(timeout);
        match linux_process_start_time(self.pid) {
            Ok(start_time) if start_time == self.start_time => Ok(ProcessWait::Running),
            Ok(_) | Err(_) => Ok(ProcessWait::Exited(0)),
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_process_start_time(pid: u32) -> io::Result<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let close = stat
        .rfind(')')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid /proc stat"))?;
    stat[close + 2..]
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing process start time"))?
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid process start time"))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
struct ParentProcess;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
impl ParentProcess {
    fn open(_pid: u32) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Palmistry process identity pinning is unsupported on this platform",
        ))
    }

    fn wait(&self, _timeout: Duration) -> io::Result<ProcessWait> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "unsupported platform",
        ))
    }
}

#[cfg(target_os = "windows")]
fn write_minidump(
    process: &ParentProcess,
    pid: u32,
    directory: &Path,
    after_exit: bool,
) -> (MinidumpStatus, Option<String>) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Diagnostics::Debug::{
        MiniDumpWithHandleData, MiniDumpWithThreadInfo, MiniDumpWithUnloadedModules,
        MiniDumpWriteDump,
    };
    let file_name = format!("minidump-{}-{}.dmp", unix_ms(), Uuid::now_v7());
    let path = directory.join(&file_name);
    let file = match fs::File::create(&path) {
        Ok(file) => file,
        Err(_) => {
            return (
                if after_exit {
                    MinidumpStatus::FailedAfterExit
                } else {
                    MinidumpStatus::FailedWhileRunning
                },
                None,
            )
        }
    };
    let dump_type = MiniDumpWithThreadInfo | MiniDumpWithHandleData | MiniDumpWithUnloadedModules;
    let written = unsafe {
        MiniDumpWriteDump(
            process.0,
            pid,
            file.as_raw_handle(),
            dump_type,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        )
    } != 0;
    if written && file.sync_all().is_ok() {
        (MinidumpStatus::Written, Some(file_name))
    } else {
        drop(file);
        let _ = fs::remove_file(path);
        (
            if after_exit {
                MinidumpStatus::FailedAfterExit
            } else {
                MinidumpStatus::FailedWhileRunning
            },
            None,
        )
    }
}

#[cfg(not(target_os = "windows"))]
fn write_minidump(
    _process: &ParentProcess,
    _pid: u32,
    _directory: &Path,
    _after_exit: bool,
) -> (MinidumpStatus, Option<String>) {
    (MinidumpStatus::Unsupported, None)
}

#[cfg(target_os = "windows")]
fn os_confirms_hung_window(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM},
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowThreadProcessId, IsHungAppWindow, IsWindowVisible,
        },
    };
    struct Search {
        pid: u32,
        hwnd: HWND,
    }
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> i32 {
        let search = unsafe { &mut *(lparam as *mut Search) };
        let mut window_pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut window_pid);
        }
        if window_pid == search.pid && unsafe { IsWindowVisible(hwnd) } != 0 {
            search.hwnd = hwnd;
            0
        } else {
            1
        }
    }
    let mut search = Search {
        pid,
        hwnd: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(Some(visit), &mut search as *mut Search as LPARAM);
    }
    !search.hwnd.is_null() && unsafe { IsHungAppWindow(search.hwnd) } != 0
}

#[cfg(not(target_os = "windows"))]
fn os_confirms_hung_window(_pid: u32) -> bool {
    false
}

fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".palmistry-{}.tmp", Uuid::now_v7()));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temporary, path)?;
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_contract_requires_every_lifecycle_path() {
        let session_id = Uuid::now_v7();
        let launch_nonce = Uuid::now_v7();
        let config = PalmistryConfig::parse(vec![
            "--session-id".to_owned(),
            session_id.to_string(),
            "--launch-nonce".to_owned(),
            launch_nonce.to_string(),
            "--parent-pid".to_owned(),
            "42".to_owned(),
            "--ring".to_owned(),
            "ring.bin".to_owned(),
            "--survivor-dir".to_owned(),
            "survivors".to_owned(),
            "--panic-signal".to_owned(),
            "panic.signal".to_owned(),
            "--panic-ack".to_owned(),
            "panic.ack".to_owned(),
            "--shutdown-signal".to_owned(),
            "shutdown.signal".to_owned(),
            "--ready-signal".to_owned(),
            "ready.signal".to_owned(),
        ])
        .expect("valid Palmistry arguments");
        assert_eq!(config.parent_pid, 42);
        assert_eq!(config.poll_interval, DEFAULT_POLL_INTERVAL);
        assert_eq!(config.stale_heartbeat, DEFAULT_STALE_HEARTBEAT);
    }

    #[test]
    fn survivor_schema_has_no_free_text_or_project_content_fields() {
        let record = PalmistrySurvivorRecord {
            schema_id: PALMISTRY_RECORD_SCHEMA_ID.to_owned(),
            record_id: Uuid::now_v7(),
            session_id: Uuid::now_v7(),
            launch_nonce: Uuid::now_v7(),
            watcher_pid: 3,
            watcher_creation_time_100ns: 4,
            kind: SurvivorKind::GuiFreeze,
            observed_at_unix_ms: 1,
            parent_pid: 2,
            parent_exit_code: None,
            heartbeat_stale_ms: Some(5_000),
            os_hung_window_confirmed: true,
            minidump_status: MinidumpStatus::NotRequested,
            crash_classification: None,
            minidump_file_name: None,
            last_snapshot: None,
            source_proof: "ab".repeat(64),
        };
        let object = serde_json::to_value(record)
            .expect("serialize")
            .as_object()
            .expect("object")
            .clone();
        assert!(!object.contains_key("message"));
        assert!(!object.contains_key("project"));
        assert!(!object.contains_key("content"));
        assert!(object.contains_key("os_hung_window_confirmed"));
    }

    #[test]
    fn watcher_signing_key_pipe_requires_exact_length_and_eof() {
        assert!(read_watcher_signing_secret(&[7_u8; 31][..]).is_err());
        assert!(read_watcher_signing_secret(&[7_u8; 33][..]).is_err());
        let key = read_watcher_signing_secret(&[7_u8; 32][..]).expect("exact key");
        assert_eq!(key.as_ref(), &[7_u8; 32]);
    }

    #[test]
    fn abrupt_exit_classification_is_explicit_for_crash_fixtures() {
        assert_eq!(
            classify_exit_code(0xC000_0005),
            CrashClassification::AccessViolation
        );
        assert_eq!(classify_exit_code(3), CrashClassification::Abort);
        assert_eq!(
            classify_exit_code(0xDEAD_DEAD),
            CrashClassification::HardKillFixture
        );
        assert_eq!(classify_exit_code(17), CrashClassification::UnexpectedExit);
    }

    #[test]
    fn freeze_unfreeze_freeze_starts_two_distinct_episodes() {
        let mut state = FreezeEpisodeState::new(Some(10));
        assert!(state.should_record());
        state.mark_recorded();
        assert!(!state.should_record());
        assert!(state.observe(11), "heartbeat advance unfreezes the session");
        assert!(
            state.should_record(),
            "a later freeze is independently recordable"
        );
        state.mark_recorded();
        assert!(!state.should_record());
    }
}
