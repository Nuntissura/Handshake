//! The MCP discovery/binding artifact (WP-KERNEL-011 MT-027 transport).
//!
//! When the [`crate::mcp::server::SwarmMcpServer`] binds its localhost TCP listener (and, on Windows,
//! its named pipe), it records the resolved endpoint plus the per-session token into an [`McpBinding`]
//! and persists it to `{local_app_data}/handshake/swarm_mcp_binding.json`. This is the discovery
//! contract for an external agent: **read the binding file, then connect to `tcp_addr` (or `pipe_name`)
//! and present `token` in every JSON-RPC request's `session_token` field**.
//!
//! ## Why the binding file is owner-restricted
//!
//! The token in this file authorizes full UI steering of the running app. Any local process that can
//! read the file can impersonate an authorized agent (red-team: token exfiltration). The file is
//! therefore written with owner-only permissions:
//!
//! - Unix: mode `0o600` set explicitly via [`std::os::unix::fs::PermissionsExt`] (no dependency).
//! - Windows: the temporary file is hardened with `icacls` before it becomes the discovery path. ACL
//!   failure is fatal, so a readable token file is never published under weaker permissions.
//!
//! ## Why no `dirs` crate
//!
//! The contract suggested `dirs::data_local_dir()`. To avoid a new dependency family, the local
//! app-data directory is resolved from the platform environment (`%LOCALAPPDATA%` on Windows,
//! `$XDG_DATA_HOME` or `$HOME/.local/share` on Unix), with the contract's `.` fallback. This is the
//! same directory `dirs` returns, resolved dependency-free.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// OS-issued identity for one concrete process instance. A PID alone is reusable after exit, so
/// consumers must compare both the PID and this birth identity before trusting a binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProcessBirthIdentity {
    Windows {
        creation_time_100ns: u64,
    },
    Linux {
        boot_id: String,
        start_time_ticks: u64,
    },
    MacOs {
        start_time_seconds: u64,
        start_time_microseconds: u64,
    },
}

/// The discovery record an external agent reads to find + authenticate to the running MCP server.
///
/// Serialized to `swarm_mcp_binding.json`. `pipe_name` is `None` on non-Windows builds (and on Windows
/// if the named-pipe bind failed — the server then runs TCP-only and records that here honestly).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpBinding {
    /// The bound localhost TCP address, e.g. `127.0.0.1:54321` (the OS-picked ephemeral port). Always
    /// present: the TCP listener is the cross-platform transport.
    pub tcp_addr: String,
    /// The Windows named-pipe path, e.g. `\\.\pipe\handshake_swarm_<pid>`. `None` off Windows or when
    /// the pipe bind failed (TCP-only fallback).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipe_name: Option<String>,
    /// The per-session token a caller must present in every request's `session_token` field (64 hex
    /// chars). Treat as a secret; the file is owner-restricted for this reason.
    pub token: String,
    /// The process id of the server that wrote this binding, so an agent can detect a stale file from a
    /// crashed session. The canonical file intentionally names only the most recently published app
    /// instance; parallel agents share that instance rather than treating this as a multi-instance registry.
    pub pid: u32,
    /// Birth identity of the exact process instance named by `pid`. This closes the PID-reuse window:
    /// a later process assigned the same numeric PID has a different OS-issued birth identity.
    pub process_birth: ProcessBirthIdentity,
}

impl McpBinding {
    /// Build a binding owned by the current process, failing closed when the host cannot provide a
    /// verifiable process birth identity.
    pub fn for_current_process(
        tcp_addr: String,
        pipe_name: Option<String>,
        token: String,
    ) -> Result<Self, BindingError> {
        let pid = std::process::id();
        let process_birth = process_birth_identity(pid)?;
        Ok(Self {
            tcp_addr,
            pipe_name,
            token,
            pid,
            process_birth,
        })
    }

    /// Serialize to pretty JSON (the on-disk form). Pretty so an operator can read the file by hand.
    pub fn to_json_string(&self) -> Result<String, BindingError> {
        serde_json::to_string_pretty(self).map_err(|e| BindingError(format!("serialize: {e}")))
    }
}

/// Read the OS-issued birth identity for `pid` only while that exact process is live and non-zombie.
/// Unsupported or unverifiable targets fail closed instead of publishing a PID-only credential.
pub fn process_birth_identity(pid: u32) -> Result<ProcessBirthIdentity, BindingError> {
    platform_process_birth_identity(pid).ok_or_else(|| {
        BindingError(format!(
            "process {pid} has no verifiable live birth identity on this host"
        ))
    })
}

#[cfg(windows)]
fn platform_process_birth_identity(pid: u32) -> Option<ProcessBirthIdentity> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    const SYNCHRONIZE_RIGHT: u32 = 0x0010_0000;
    if pid == 0 {
        return None;
    }
    // SAFETY: the numeric PID is passed to a documented read-only process query. The handle is checked
    // before use and closed exactly once after the zero-time liveness wait and creation-time query.
    let handle = unsafe {
        OpenProcess(
            SYNCHRONIZE_RIGHT | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return None;
    }
    let live = unsafe { WaitForSingleObject(handle, 0) } == WAIT_TIMEOUT;
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let queried = live
        && unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) }
            != 0;
    unsafe {
        let _ = CloseHandle(handle);
    }
    queried.then(|| ProcessBirthIdentity::Windows {
        creation_time_100ns: (u64::from(creation.dwHighDateTime) << 32)
            | u64::from(creation.dwLowDateTime),
    })
}

#[cfg(target_os = "linux")]
fn platform_process_birth_identity(pid: u32) -> Option<ProcessBirthIdentity> {
    if pid == 0 {
        return None;
    }
    let boot_id = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()?
        .trim()
        .to_owned();
    if boot_id.is_empty() {
        return None;
    }
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, tail) = stat.rsplit_once(") ")?;
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let state = fields.first()?.as_bytes().first().copied()?;
    if matches!(state, b'Z' | b'X' | b'x') {
        return None;
    }
    // `tail` starts at proc(5) field 3 (state); field 22 (starttime) is therefore index 19.
    let start_time_ticks = fields.get(19)?.parse().ok()?;
    Some(ProcessBirthIdentity::Linux {
        boot_id,
        start_time_ticks,
    })
}

#[cfg(target_os = "macos")]
fn platform_process_birth_identity(pid: u32) -> Option<ProcessBirthIdentity> {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct ProcBsdInfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32,
        pbi_uid: u32,
        pbi_gid: u32,
        pbi_ruid: u32,
        pbi_rgid: u32,
        pbi_svuid: u32,
        pbi_svgid: u32,
        pbi_reserved: u32,
        pbi_comm: [u8; 16],
        pbi_name: [u8; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        e_tdev: u32,
        e_tpgid: u32,
        pbi_nice: i32,
        pbi_start_tvsec: u64,
        pbi_start_tvusec: u64,
    }

    #[link(name = "proc")]
    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut c_void,
            buffer_size: i32,
        ) -> i32;
    }

    const PROC_PIDTBSDINFO: i32 = 3;
    const SZOMB: u32 = 5;
    const PROC_FLAG_INEXIT: u32 = 4;
    if pid == 0 {
        return None;
    }
    let mut info = ProcBsdInfo::default();
    let expected_size = std::mem::size_of::<ProcBsdInfo>();
    let queried = unsafe {
        proc_pidinfo(
            i32::try_from(pid).ok()?,
            PROC_PIDTBSDINFO,
            0,
            std::ptr::from_mut(&mut info).cast::<c_void>(),
            i32::try_from(expected_size).ok()?,
        )
    };
    if queried != i32::try_from(expected_size).ok()?
        || info.pbi_pid != pid
        || info.pbi_status == SZOMB
        || info.pbi_flags & PROC_FLAG_INEXIT != 0
        || info.pbi_start_tvsec == 0
        || info.pbi_start_tvusec >= 1_000_000
    {
        return None;
    }
    Some(ProcessBirthIdentity::MacOs {
        start_time_seconds: info.pbi_start_tvsec,
        start_time_microseconds: info.pbi_start_tvusec,
    })
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn platform_process_birth_identity(_pid: u32) -> Option<ProcessBirthIdentity> {
    None
}

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))
))]
fn platform_process_birth_identity(_pid: u32) -> Option<ProcessBirthIdentity> {
    None
}

#[cfg(not(any(unix, windows)))]
fn platform_process_birth_identity(_pid: u32) -> Option<ProcessBirthIdentity> {
    None
}

/// A failure resolving the binding directory, writing the file, or restricting its permissions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingError(pub String);

#[cfg(windows)]
fn win32_full_verbatim_path(
    path: &std::path::Path,
    operand: &'static str,
) -> std::io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt as _;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFullPathNameW(
            file_name: *const u16,
            buffer_length: u32,
            buffer: *mut u16,
            file_part: *mut *mut u16,
        ) -> u32;
    }

    let mut input = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if input.contains(&0) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{operand} MoveFileExW operand contains an interior NUL"),
        ));
    }
    input.push(0);

    // First pass asks Win32 for the required UTF-16 capacity. The second pass resolves an absolute,
    // dot-segment-normalized path even when the destination does not exist.
    let required = unsafe {
        GetFullPathNameW(
            input.as_ptr(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if required == 0 {
        return Err(std::io::Error::new(
            std::io::Error::last_os_error().kind(),
            format!(
                "resolve {operand} MoveFileExW operand {} with GetFullPathNameW: {}",
                path.display(),
                std::io::Error::last_os_error()
            ),
        ));
    }
    let mut output = vec![0_u16; required as usize];
    loop {
        let written = unsafe {
            GetFullPathNameW(
                input.as_ptr(),
                u32::try_from(output.len()).map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("{operand} MoveFileExW operand is too long for Win32"),
                    )
                })?,
                output.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        if written == 0 {
            let error = std::io::Error::last_os_error();
            return Err(std::io::Error::new(
                error.kind(),
                format!(
                    "resolve {operand} MoveFileExW operand {} with GetFullPathNameW: {error}",
                    path.display()
                ),
            ));
        }
        if written as usize >= output.len() {
            output.resize(written as usize + 1, 0);
            continue;
        }
        output.truncate(written as usize);
        return win32_apply_verbatim_prefix(output, operand);
    }
}

#[cfg(windows)]
fn win32_apply_verbatim_prefix(
    mut absolute: Vec<u16>,
    operand: &'static str,
) -> std::io::Result<Vec<u16>> {
    const BACKSLASH: u16 = b'\\' as u16;
    const SLASH: u16 = b'/' as u16;
    const COLON: u16 = b':' as u16;
    const VERBATIM: &[u16] = &[BACKSLASH, BACKSLASH, b'?' as u16, BACKSLASH];
    const DEVICE: &[u16] = &[BACKSLASH, BACKSLASH, b'.' as u16, BACKSLASH];
    const UNC: &[u16] = &[BACKSLASH, BACKSLASH];
    const VERBATIM_UNC: &[u16] = &[
        BACKSLASH,
        BACKSLASH,
        b'?' as u16,
        BACKSLASH,
        b'U' as u16,
        b'N' as u16,
        b'C' as u16,
        BACKSLASH,
    ];

    for unit in &mut absolute {
        if *unit == SLASH {
            *unit = BACKSLASH;
        }
    }
    if absolute.starts_with(DEVICE) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{operand} MoveFileExW operand resolved to an unsupported device path"),
        ));
    }
    let mut result = if absolute.starts_with(VERBATIM) {
        absolute
    } else if absolute.starts_with(UNC) {
        VERBATIM_UNC
            .iter()
            .copied()
            .chain(absolute[UNC.len()..].iter().copied())
            .collect()
    } else if absolute.len() >= 3
        && absolute[1] == COLON
        && absolute[2] == BACKSLASH
        && (absolute[0] as u8).is_ascii_alphabetic()
    {
        VERBATIM.iter().copied().chain(absolute).collect()
    } else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "{operand} MoveFileExW operand did not resolve to a drive, UNC, or verbatim path"
            ),
        ));
    };
    result.push(0);
    Ok(result)
}

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mcp binding: {}", self.0)
    }
}

impl std::error::Error for BindingError {}

/// The fixed file name of the discovery artifact within the `handshake/` app-data subdirectory.
pub const BINDING_FILE_NAME: &str = "swarm_mcp_binding.json";

/// Persistent coordination file used to serialize publication and owner-checked teardown across app
/// processes. The file itself carries no token; the OS lock on its open handle is the authority.
const BINDING_LOCK_FILE_NAME: &str = "swarm_mcp_binding.lock";

/// Resolve the local app-data directory dependency-free (see module docs). Returns the platform
/// per-user data dir, or `.` as the contract-specified last-resort fallback.
fn local_app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(p) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(p);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(p) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(p);
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(".local").join("share");
        }
    }
    PathBuf::from(".")
}

/// The full path the binding file is written to: `{local_app_data}/handshake/swarm_mcp_binding.json`.
pub fn binding_path() -> PathBuf {
    local_app_data_dir()
        .join("handshake")
        .join(BINDING_FILE_NAME)
}

/// Write the binding to its canonical path, creating the `handshake/` subdirectory if absent, and
/// restrict the file to owner-only access. Overwrites unconditionally (the ephemeral port changes each
/// restart, making a stale file harmless — the contract's "overwrite unconditionally" control).
///
/// Returns the path written on success so the caller can log/expose it.
pub fn write_binding(binding: &McpBinding) -> Result<PathBuf, BindingError> {
    let path = binding_path();
    if let Some(parent) = path.parent() {
        prepare_private_binding_directory(parent)?;
    }
    let _binding_lock = acquire_binding_lock(&path)?;
    let json = binding.to_json_string()?;
    atomic_replace(&path, json.as_bytes())?;
    Ok(path)
}

/// Acquire the process-shared lock that makes binding replacement and owner-checked deletion one
/// serializable operation. A persistent sibling file is required: locking the binding itself would
/// not protect across the atomic rename that publishes a new inode/file identity.
fn acquire_binding_lock(binding_path: &std::path::Path) -> Result<std::fs::File, BindingError> {
    let parent = binding_path.parent().ok_or_else(|| {
        BindingError(format!(
            "binding path {} has no parent directory",
            binding_path.display()
        ))
    })?;
    prepare_private_binding_directory(parent)?;
    let lock_path = parent.join(BINDING_LOCK_FILE_NAME);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| {
            BindingError(format!(
                "open binding lock {}: {error}",
                lock_path.display()
            ))
        })?;
    // Publishing holds this lock through owner-only ACL application plus a durable atomic replace.
    // On a loaded Windows host that bounded filesystem/ACL work can exceed 500 ms; competing app
    // startup/shutdown must serialize instead of falsely reporting a coordination failure.
    const LOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    const LOCK_RETRY: std::time::Duration = std::time::Duration::from_millis(10);
    let deadline = std::time::Instant::now() + LOCK_TIMEOUT;
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) if std::time::Instant::now() < deadline => {
                std::thread::sleep(LOCK_RETRY);
            }
            Err(error) => {
                return Err(BindingError(format!(
                    "lock binding coordination file {} within {} ms: {error}",
                    lock_path.display(),
                    LOCK_TIMEOUT.as_millis()
                )))
            }
        }
    }
    Ok(file)
}

fn atomic_replace(path: &std::path::Path, bytes: &[u8]) -> Result<(), BindingError> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp)
        .map_err(|error| BindingError(format!("create {}: {error}", temp.display())))?;
    let mut unpublished = UnpublishedBindingTemp::new(temp.clone());
    if let Err(error) = restrict_to_owner(&temp) {
        return Err(unpublished.cleanup_on_error(error));
    }
    use std::io::Write as _;
    if let Err(error) = file
        .write_all(bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
    {
        drop(file);
        return Err(unpublished.cleanup_on_error(BindingError(format!(
            "write and sync {}: {error}",
            temp.display()
        ))));
    }
    drop(file);

    #[cfg(windows)]
    let replace_result = {
        #[link(name = "kernel32")]
        extern "system" {
            fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
        }
        const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
        const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

        // WP-KERNEL-012 MT-129: both operands go to Win32 in EXTENDED-LENGTH (verbatim) form.
        //
        // Every other operation in this function — create_new, the icacls harden, write_all,
        // sync_all — goes through Rust `std::fs`, which normalizes to `\\?\` internally and so
        // tolerates paths past the legacy 260-char limit. This ONE call did not: it handed
        // `encode_wide()` output straight to `MoveFileExW`. The result was a publish that could
        // create, harden, write and fsync its temp file and then fail to rename it with
        // ERROR_PATH_NOT_FOUND (os error 3) — a "path not found" for a directory that demonstrably
        // existed, because the SOURCE name had crossed MAX_PATH.
        //
        // Measured, not inferred: with the crate root as CWD the destination `.json` resolves at 235
        // chars while the temp name `swarm_mcp_binding.{pid}.{19-digit-nanos}.tmp` reaches exactly
        // 260 for a 5-digit PID — and Windows test PIDs are 5-6 digits. The affected binaries carry
        // NO application manifest at all, so machine-wide LongPathsEnabled cannot rescue them.
        //
        (|| -> std::io::Result<()> {
            let from = win32_full_verbatim_path(&temp, "source")?;
            let to = win32_full_verbatim_path(path, "destination")?;
            // SAFETY: both pointers reference NUL-terminated UTF-16 buffers for the duration of the call.
            let replaced = unsafe {
                MoveFileExW(
                    from.as_ptr(),
                    to.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                ) != 0
            };
            if replaced {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        })()
    };

    #[cfg(not(windows))]
    let replace_result = std::fs::rename(&temp, path);

    if let Err(error) = replace_result {
        // MT-129 AC-129-3: name BOTH operands and their lengths.
        //
        // This message previously named only the destination. When the long-path defect fired, the
        // destination was a perfectly valid 235-character path while the 260-character SOURCE was
        // what Win32 rejected — so the error pointed diagnosis at the one path that was fine, and
        // cost this packet a full cycle chasing a stale-lock theory instead. The lengths are
        // included because "path not found" for a directory that visibly exists is only
        // interpretable once you can see which operand crossed the limit.
        let temp_display = temp.display().to_string();
        let path_display = path.display().to_string();
        return Err(unpublished.cleanup_on_error(BindingError(format!(
            "replace {path_display} atomically (from {temp_display}; source_len={} dest_len={}): {error}",
            temp_display.chars().count(),
            path_display.chars().count(),
        ))));
    }
    unpublished.disarm();
    Ok(())
}

fn prepare_private_binding_directory(path: &std::path::Path) -> Result<(), BindingError> {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(BindingError(format!(
                "binding directory {} must be a real directory, not a symlink or file",
                path.display()
            )));
        }
    } else {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt as _;
            builder.mode(0o700);
        }
        builder
            .create(path)
            .map_err(|error| BindingError(format!("create {}: {error}", path.display())))?;
    }
    restrict_binding_directory_to_owner(path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_binding_directory_to_owner(path: &std::path::Path) -> Result<(), BindingError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| BindingError(format!("restrict {} to 0o700: {error}", path.display())))?;
    let mode = std::fs::metadata(path)
        .map_err(|error| BindingError(format!("inspect {}: {error}", path.display())))?
        .permissions()
        .mode()
        & 0o777;
    if mode != 0o700 {
        return Err(BindingError(format!(
            "binding directory {} has mode {mode:o}, expected 700",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn restrict_binding_directory_to_owner(path: &std::path::Path) -> Result<(), BindingError> {
    restrict_to_owner_windows(path)
}

#[cfg(not(any(unix, target_os = "windows")))]
fn restrict_binding_directory_to_owner(path: &std::path::Path) -> Result<(), BindingError> {
    Err(BindingError(format!(
        "owner-only binding directories are unsupported for {}",
        path.display()
    )))
}

struct UnpublishedBindingTemp {
    path: PathBuf,
    armed: bool,
}

impl UnpublishedBindingTemp {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn cleanup_on_error(&mut self, primary: BindingError) -> BindingError {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {
                self.disarm();
                primary
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.disarm();
                primary
            }
            Err(cleanup) => BindingError(format!(
                "{}; additionally failed to remove unpublished token file {}: {cleanup}",
                primary.0,
                self.path.display()
            )),
        }
    }
}

impl Drop for UnpublishedBindingTemp {
    fn drop(&mut self) {
        if self.armed {
            match std::fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {}
            }
        }
    }
}

/// Remove the binding file only when it still names `owner` (called on graceful shutdown).
///
/// Another concurrently running app instance may have atomically replaced the canonical discovery
/// record since this server started. In that case shutdown must leave the newer live binding intact.
/// Missing-file and a current record owned by another server are both successful no-ops.
pub fn remove_binding(owner: &McpBinding) -> Result<(), BindingError> {
    let path = binding_path();
    let _binding_lock = acquire_binding_lock(&path)?;
    let current = match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str::<McpBinding>(&json)
            .map_err(|error| BindingError(format!("parse current {}: {error}", path.display())))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(BindingError(format!("read {}: {error}", path.display()))),
    };
    if current != *owner {
        return Ok(());
    }
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(BindingError(format!("remove {}: {e}", path.display()))),
    }
}

/// Restore a binding displaced by a scoped test/helper only if the canonical record still names the
/// exact binding that helper installed. A newer app publication wins and is never overwritten.
pub fn restore_binding_if_current(
    installed: &McpBinding,
    previous: Option<&McpBinding>,
) -> Result<(), BindingError> {
    let path = binding_path();
    let _binding_lock = acquire_binding_lock(&path)?;
    let current = match std::fs::read_to_string(&path) {
        Ok(json) => serde_json::from_str::<McpBinding>(&json)
            .map_err(|error| BindingError(format!("parse current {}: {error}", path.display())))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(BindingError(format!("read {}: {error}", path.display()))),
    };
    if current != *installed {
        return Ok(());
    }
    match previous {
        Some(binding) => atomic_replace(&path, binding.to_json_string()?.as_bytes()),
        None => match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(BindingError(format!("remove {}: {error}", path.display()))),
        },
    }
}

/// Apply owner-only permissions before the temporary binding file is atomically published. Failure is
/// fatal because the file contains the full UI-steering session token.
fn restrict_to_owner(path: &std::path::Path) -> Result<(), BindingError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(
            |error| BindingError(format!("restrict {} to 0o600: {error}", path.display())),
        )?;
    }
    #[cfg(target_os = "windows")]
    {
        restrict_to_owner_windows(path)?;
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        return Err(BindingError(format!(
            "owner-only permissions are unsupported for {}",
            path.display()
        )));
    }
    Ok(())
}

/// Windows owner-only hardening via a quiet `icacls` subprocess: reset inherited ACEs and grant only
/// the current user full control. A non-zero exit prevents publication.
#[cfg(target_os = "windows")]
fn restrict_to_owner_windows(path: &std::path::Path) -> Result<(), BindingError> {
    use std::os::windows::process::CommandExt as _;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // %USERNAME% is the current user; `icacls <file> /inheritance:r /grant:r "%USERNAME%":F`
    // removes inherited ACEs and grants only that user Full control.
    let user = std::env::var_os("USERNAME").ok_or_else(|| {
        BindingError("USERNAME unset; cannot harden MCP binding token file".to_owned())
    })?;
    let user = user.to_string_lossy().to_string();
    let grant = format!("{user}:F");
    let status = std::process::Command::new("icacls")
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(&grant)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|error| BindingError(format!("run icacls for {}: {error}", path.display())))?;
    if !status.success() {
        return Err(BindingError(format!(
            "icacls rejected owner-only ACL for {} with status {status}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static BINDING_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn test_birth(label: &str) -> ProcessBirthIdentity {
        ProcessBirthIdentity::Linux {
            boot_id: label.to_owned(),
            start_time_ticks: 1,
        }
    }

    #[test]
    fn current_process_binding_carries_stable_birth_identity() {
        let first = McpBinding::for_current_process("127.0.0.1:1".to_owned(), None, "a".repeat(64))
            .expect("first current-process binding");
        let second =
            McpBinding::for_current_process("127.0.0.1:2".to_owned(), None, "b".repeat(64))
                .expect("second current-process binding");
        assert_eq!(first.pid, std::process::id());
        assert_eq!(first.process_birth, second.process_birth);
    }

    #[test]
    fn binding_round_trips_through_json_with_pipe() {
        let b = McpBinding {
            tcp_addr: "127.0.0.1:54321".to_owned(),
            pipe_name: Some(r"\\.\pipe\handshake_swarm_4242".to_owned()),
            token: "a".repeat(64),
            pid: 4242,
            process_birth: test_birth("server-4242"),
        };
        let json = b.to_json_string().expect("serialize");
        let back: McpBinding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(b, back);
        assert!(json.contains("tcp_addr"));
        assert!(json.contains("pipe_name"));
        assert!(json.contains("token"));
        assert!(json.contains("process_birth"));
    }

    #[test]
    fn pipe_name_omitted_when_none() {
        let b = McpBinding {
            tcp_addr: "127.0.0.1:1".to_owned(),
            pipe_name: None,
            token: "t".to_owned(),
            pid: 1,
            process_birth: test_birth("server-1"),
        };
        let json = b.to_json_string().expect("serialize");
        assert!(
            !json.contains("pipe_name"),
            "None pipe_name is skipped: {json}"
        );
    }

    #[test]
    fn binding_path_ends_with_expected_components() {
        let _guard = BINDING_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let p = binding_path();
        let s = p.to_string_lossy().replace('\\', "/");
        assert!(
            s.ends_with(&format!("handshake/{BINDING_FILE_NAME}")),
            "path was {s}"
        );
    }

    #[test]
    fn unpublished_binding_temp_is_removed_during_unwind() {
        let tmp = std::env::temp_dir().join(format!(
            "hsk_mcp_binding_unwind_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).expect("create unpublished binding test root");
        let unpublished_path = tmp.join("swarm_mcp_binding.unpublished.tmp");

        let unwind = std::panic::catch_unwind(|| {
            std::fs::write(&unpublished_path, b"secret-session-token")
                .expect("write unpublished binding token");
            let _unpublished = UnpublishedBindingTemp::new(unpublished_path.clone());
            panic!("exercise unpublished binding unwind cleanup");
        });

        assert!(unwind.is_err(), "test must exercise the unwinding path");
        assert!(
            !unpublished_path.exists(),
            "unwinding must not leave an unpublished token file"
        );
        let residue = std::fs::read_dir(&tmp)
            .expect("read unpublished binding test root")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "tmp"))
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert!(residue.is_empty(), "unpublished temp residue: {residue:?}");
        std::fs::remove_dir_all(&tmp).expect("remove unpublished binding test root");
    }

    #[test]
    fn write_then_remove_is_idempotent() {
        let _guard = BINDING_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Point the resolver at a temp dir via the platform env var so the test never touches the real
        // user app-data location. We restore the var after.
        let tmp = std::env::temp_dir().join(format!("hsk_mcp_binding_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("mk tmp");

        #[cfg(target_os = "windows")]
        let var = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let var = "XDG_DATA_HOME";
        let prev = std::env::var_os(var);
        std::env::set_var(var, &tmp);

        let b = McpBinding {
            tcp_addr: "127.0.0.1:9".to_owned(),
            pipe_name: None,
            token: "z".repeat(64),
            pid: std::process::id(),
            process_birth: process_birth_identity(std::process::id())
                .expect("current test process birth identity"),
        };
        let written = write_binding(&b).expect("write");
        assert!(written.exists(), "binding file exists after write");
        let read_back: McpBinding =
            serde_json::from_str(&std::fs::read_to_string(&written).unwrap()).unwrap();
        assert_eq!(read_back, b);

        remove_binding(&b).expect("remove");
        assert!(!written.exists(), "binding file gone after remove");
        // Second remove is a no-op (idempotent).
        remove_binding(&b).expect("remove idempotent");

        // restore env + clean up
        match prev {
            Some(v) => std::env::set_var(var, v),
            None => std::env::remove_var(var),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn older_server_shutdown_never_removes_newer_server_binding() {
        let _guard = BINDING_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp =
            std::env::temp_dir().join(format!("hsk_mcp_binding_owner_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("mk tmp");

        #[cfg(target_os = "windows")]
        let var = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let var = "XDG_DATA_HOME";
        let prev = std::env::var_os(var);
        std::env::set_var(var, &tmp);

        let server_a = McpBinding {
            tcp_addr: "127.0.0.1:41001".to_owned(),
            pipe_name: None,
            token: "a".repeat(64),
            pid: 41001,
            process_birth: test_birth("server-a"),
        };
        let server_b = McpBinding {
            tcp_addr: "127.0.0.1:41002".to_owned(),
            pipe_name: None,
            token: "b".repeat(64),
            pid: 41002,
            process_birth: test_birth("server-b"),
        };
        let path = write_binding(&server_a).expect("server A publishes");
        write_binding(&server_b).expect("server B replaces active discovery");

        remove_binding(&server_a).expect("server A shutdown is ownership checked");
        let still_current: McpBinding = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("server B binding remains discoverable"),
        )
        .expect("parse server B binding");
        assert_eq!(still_current, server_b);

        remove_binding(&server_b).expect("current owner removes its own binding");
        assert!(!path.exists());

        match prev {
            Some(value) => std::env::set_var(var, value),
            None => std::env::remove_var(var),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn concurrent_stale_teardown_cannot_remove_newer_live_binding() {
        let _guard = BINDING_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let tmp = std::env::temp_dir().join(format!(
            "hsk_mcp_binding_concurrent_owner_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("mk tmp");

        #[cfg(target_os = "windows")]
        let var = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let var = "XDG_DATA_HOME";
        let prev = std::env::var_os(var);
        std::env::set_var(var, &tmp);

        let stale = McpBinding {
            tcp_addr: "127.0.0.1:42001".to_owned(),
            pipe_name: None,
            token: "a".repeat(64),
            pid: 42001,
            process_birth: test_birth("stale-server"),
        };
        let live = McpBinding {
            tcp_addr: "127.0.0.1:42002".to_owned(),
            pipe_name: None,
            token: "b".repeat(64),
            pid: 42002,
            process_birth: test_birth("live-server"),
        };
        let path = write_binding(&stale).expect("stale server publishes first");

        // Hold the exact OS lock used by production, then release a stale teardown and replacement
        // writer together. Both production operations must block here; after release, either legal
        // serialization order leaves the newer binding discoverable.
        let held_lock = acquire_binding_lock(&path).expect("test owns coordination lock");
        let ready = std::sync::Arc::new(std::sync::Barrier::new(3));
        let (done_tx, done_rx) = std::sync::mpsc::channel();

        let remove_ready = ready.clone();
        let remove_done = done_tx.clone();
        let stale_owner = stale.clone();
        let remover = std::thread::spawn(move || {
            remove_ready.wait();
            let result = remove_binding(&stale_owner);
            remove_done.send(("remove", result)).expect("report remove");
        });

        let write_ready = ready.clone();
        let write_done = done_tx;
        let live_owner = live.clone();
        let writer = std::thread::spawn(move || {
            write_ready.wait();
            let result = write_binding(&live_owner).map(|_| ());
            write_done.send(("write", result)).expect("report write");
        });

        ready.wait();
        assert!(
            done_rx
                .recv_timeout(std::time::Duration::from_millis(100))
                .is_err(),
            "publication and teardown must both wait for the process-shared binding lock"
        );
        drop(held_lock);

        for _ in 0..2 {
            let (operation, result) = done_rx
                .recv_timeout(std::time::Duration::from_secs(15))
                .expect("both operations complete after lock release");
            result.unwrap_or_else(|error| panic!("{operation} failed: {error}"));
        }
        remover.join().expect("remover thread");
        writer.join().expect("writer thread");

        let still_current: McpBinding = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("newer binding remains discoverable"),
        )
        .expect("parse newer binding");
        assert_eq!(still_current, live);
        remove_binding(&live).expect("live owner removes its binding");

        match prev {
            Some(value) => std::env::set_var(var, value),
            None => std::env::remove_var(var),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// WP-KERNEL-012 MT-129: publishing must survive a root long enough to cross the legacy
    /// MAX_PATH limit.
    ///
    /// RED before the fix: every other operation in `atomic_replace` goes through `std::fs`, which
    /// normalizes to the verbatim form internally, but `MoveFileExW` received raw `encode_wide()`
    /// output. The publish therefore created, hardened, wrote and fsynced its temp file and then
    /// failed the rename with ERROR_PATH_NOT_FOUND (os error 3) - a path-not-found for a directory
    /// that demonstrably existed, because the SOURCE name had crossed the limit. The temp name is
    /// `swarm_mcp_binding.{pid}.{19-digit-nanos}.tmp`, so it crosses well before the destination
    /// `.json` does.
    ///
    /// The root below is padded so the temp name lands past 260 characters on any host.
    #[test]
    fn write_binding_survives_a_root_past_the_legacy_max_path_limit() {
        let _guard = BINDING_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Build a deep-but-legal root. Each segment is short enough to be a valid file name; it is the
        // TOTAL length that matters, which is exactly the property under test.
        let mut deep = std::env::temp_dir().join(format!("hsk_mt129_{}", std::process::id()));
        while deep.as_os_str().len() < 200 {
            deep = deep.join("mt129_depth_segment");
        }
        let _ = std::fs::remove_dir_all(&deep);
        std::fs::create_dir_all(&deep).expect("create deep MT-129 root");

        #[cfg(target_os = "windows")]
        let var = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let var = "XDG_DATA_HOME";
        let prev = std::env::var_os(var);
        std::env::set_var(var, &deep);

        let binding = McpBinding {
            tcp_addr: "127.0.0.1:9".to_owned(),
            pipe_name: None,
            token: "m".repeat(64),
            pid: std::process::id(),
            process_birth: process_birth_identity(std::process::id())
                .expect("current test process birth identity"),
        };

        // The env var stays pointed at the deep root until AFTER remove_binding, otherwise the
        // cleanup resolves against the operator REAL app-data path instead of the test root.
        let written = write_binding(&binding)
            .expect("MT-129: publishing under a >MAX_PATH root must succeed");
        assert!(
            written.exists(),
            "binding file exists after a long-path write"
        );
        let read_back: McpBinding = serde_json::from_str(
            &std::fs::read_to_string(&written).expect("read long-path binding"),
        )
        .expect("parse long-path binding");
        assert_eq!(read_back, binding);

        remove_binding(&binding).expect("remove long-path binding");

        match prev {
            Some(value) => std::env::set_var(var, value),
            None => std::env::remove_var(var),
        }
        let _ = std::fs::remove_dir_all(&deep);
    }

    #[test]
    #[cfg(windows)]
    fn verbatim_prefix_preserves_utf16_and_normalizes_drive_and_unc_forms() {
        use std::os::windows::ffi::OsStringExt as _;

        let drive =
            win32_full_verbatim_path(std::path::Path::new(r"C:\hsk\.\one\..\two"), "source")
                .expect("normalize nonexistent drive operand");
        let drive_text = std::ffi::OsString::from_wide(&drive[..drive.len() - 1]);
        assert_eq!(drive_text, std::ffi::OsString::from(r"\\?\C:\hsk\two"));

        let unc = win32_full_verbatim_path(
            std::path::Path::new(r"\\server\share\hsk\.\one\..\two"),
            "destination",
        )
        .expect("normalize nonexistent UNC operand");
        let unc_text = std::ffi::OsString::from_wide(&unc[..unc.len() - 1]);
        assert_eq!(
            unc_text,
            std::ffi::OsString::from(r"\\?\UNC\server\share\hsk\two")
        );

        let non_unicode = vec![b'C' as u16, b':' as u16, b'\\' as u16, 0xd800];
        let prefixed = win32_apply_verbatim_prefix(non_unicode, "source")
            .expect("prefixing stays entirely in UTF-16");
        assert_eq!(prefixed[prefixed.len() - 2], 0xd800);
    }

    #[test]
    #[cfg(windows)]
    fn relative_dot_mixed_separator_long_publish_normalizes_and_cleans_up() {
        let _guard = BINDING_ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        struct OwnedArtifactEnv {
            key: &'static str,
            previous: Option<std::ffi::OsString>,
            path: std::path::PathBuf,
        }
        impl Drop for OwnedArtifactEnv {
            fn drop(&mut self) {
                match self.previous.take() {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }

        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("handshake_native remains below the repository root");
        let artifacts_root = repo_root
            .parent()
            .expect("repository worktree has a parent")
            .join("Handshake_Artifacts");
        assert!(
            artifacts_root.is_dir(),
            "the authorized external Handshake_Artifacts root must already exist: {}",
            artifacts_root.display()
        );
        let mt_root = artifacts_root
            .join("WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1")
            .join("MT-129");
        std::fs::create_dir_all(&mt_root).expect("create external MT-129 artifact directory");
        static NEXT_ENVELOPE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT_ENVELOPE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock remains after Unix epoch")
            .as_nanos();
        let envelope = mt_root.join(format!(
            "binding-test-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&envelope)
            .expect("create collision-resistant MT-129 publication envelope");
        let envelope_name = envelope
            .file_name()
            .expect("temporary envelope has a final component");
        let relative_envelope = std::path::PathBuf::from(
            r"..\..\..\..\Handshake_Artifacts\WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1\MT-129",
        )
        .join(envelope_name);
        let mut deep_tail = std::path::PathBuf::from("live");
        while envelope
            .join(&deep_tail)
            .join("handshake")
            .join(BINDING_FILE_NAME)
            .as_os_str()
            .len()
            <= 260
        {
            deep_tail.push("mt129_depth_segment");
        }
        let mixed_root = format!(
            "{}/.\\discarded\\..\\{}",
            relative_envelope.to_string_lossy().replace('\\', "/"),
            deep_tail.display()
        );
        let expected_root = envelope.join(&deep_tail);
        let expected_binding = expected_root.join("handshake").join(BINDING_FILE_NAME);
        assert!(
            expected_binding.as_os_str().len() > 260,
            "the first-publish destination itself must exceed MAX_PATH"
        );
        assert!(
            !expected_binding.exists(),
            "the regression must exercise first publication to a nonexistent destination"
        );

        let var = "LOCALAPPDATA";
        let _owned_artifact = OwnedArtifactEnv {
            key: var,
            previous: std::env::var_os(var),
            path: envelope.clone(),
        };
        std::env::set_var(var, &mixed_root);
        let binding = McpBinding {
            tcp_addr: "127.0.0.1:9".to_owned(),
            pipe_name: None,
            token: "r".repeat(64),
            pid: std::process::id(),
            process_birth: process_birth_identity(std::process::id())
                .expect("current test process birth identity"),
        };

        let written = write_binding(&binding)
            .expect("publish through relative dot-segment mixed-separator long root");
        assert_eq!(
            std::fs::canonicalize(&written).expect("canonicalize published binding"),
            std::fs::canonicalize(&expected_binding).expect("canonicalize expected binding"),
            "GetFullPathNameW must resolve the nonexistent destination by the same rule as the source"
        );
        let parsed: McpBinding = serde_json::from_str(
            &std::fs::read_to_string(&expected_binding).expect("read normalized binding"),
        )
        .expect("parse normalized binding JSON");
        assert_eq!(parsed, binding);
        remove_binding(&binding).expect("remove normalized binding through the same relative root");
        assert!(!expected_binding.exists());
        let residue = std::fs::read_dir(expected_root.join("handshake"))
            .expect("inspect binding directory after cleanup")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "tmp"))
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert!(residue.is_empty(), "unpublished temp residue: {residue:?}");
    }
}
