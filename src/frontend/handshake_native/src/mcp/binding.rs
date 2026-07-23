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
//! - Windows: the file lives under `%LOCALAPPDATA%`, a per-user directory whose ACL already restricts
//!   other standard users; on top of that we make a BEST-EFFORT `icacls` call to drop inherited ACEs
//!   and grant only the current user. The `icacls` step is non-fatal (logged on failure) because the
//!   per-user LocalAppData ACL is the primary control and `icacls` may be unavailable in some
//!   environments — this matches the contract's "best-effort" Windows ACL minimum control.
//!
//! ## Why no `dirs` crate
//!
//! The contract suggested `dirs::data_local_dir()`. To avoid a new dependency family, the local
//! app-data directory is resolved from the platform environment (`%LOCALAPPDATA%` on Windows,
//! `$XDG_DATA_HOME` or `$HOME/.local/share` on Unix), with the contract's `.` fallback. This is the
//! same directory `dirs` returns, resolved dependency-free.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

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
    /// crashed session (and so a multi-window dev session disambiguates which app it is talking to).
    pub pid: u32,
}

impl McpBinding {
    /// Serialize to pretty JSON (the on-disk form). Pretty so an operator can read the file by hand.
    pub fn to_json_string(&self) -> Result<String, BindingError> {
        serde_json::to_string_pretty(self).map_err(|e| BindingError(format!("serialize: {e}")))
    }
}

/// A failure resolving the binding directory, writing the file, or restricting its permissions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingError(pub String);

impl std::fmt::Display for BindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mcp binding: {}", self.0)
    }
}

impl std::error::Error for BindingError {}

/// The fixed file name of the discovery artifact within the `handshake/` app-data subdirectory.
pub const BINDING_FILE_NAME: &str = "swarm_mcp_binding.json";
const BINDING_LOCK_FILE_NAME: &str = "swarm_mcp_binding.lock";
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
        std::fs::create_dir_all(parent)
            .map_err(|e| BindingError(format!("create {}: {e}", parent.display())))?;
    }
    let _ownership_guard = BindingOwnershipGuard::acquire()?;
    let json = binding.to_json_string()?;
    atomic_replace_binding(&path, json.as_bytes())?;
    Ok(path)
}

/// Remove the binding file (called on graceful shutdown so an agent does not connect to a closed port).
/// Missing-file is success (idempotent). Other I/O errors are returned for the caller to log.
pub fn remove_binding() -> Result<(), BindingError> {
    let _ownership_guard = BindingOwnershipGuard::acquire()?;
    let path = binding_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(BindingError(format!("remove {}: {e}", path.display()))),
    }
}

fn binding_lock_path() -> PathBuf {
    binding_path().with_file_name(BINDING_LOCK_FILE_NAME)
}

fn atomic_replace_binding(path: &Path, body: &[u8]) -> Result<(), BindingError> {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp_name = format!(
        "{BINDING_FILE_NAME}.{}.{}.tmp",
        std::process::id(),
        sequence
    );
    let temp_path = path.with_file_name(temp_name);
    let mut temp = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|error| BindingError(format!("create {}: {error}", temp_path.display())))?;
    let write_result = (|| {
        temp.write_all(body)
            .map_err(|error| BindingError(format!("write {}: {error}", temp_path.display())))?;
        temp.sync_all()
            .map_err(|error| BindingError(format!("sync {}: {error}", temp_path.display())))?;
        restrict_to_owner(&temp_path);
        atomic_replace_file(&temp_path, path)
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&temp_path);
    }
    write_result
}

#[cfg(unix)]
fn atomic_replace_file(temp_path: &Path, path: &Path) -> Result<(), BindingError> {
    std::fs::rename(temp_path, path).map_err(|error| {
        BindingError(format!(
            "replace {} with {}: {error}",
            path.display(),
            temp_path.display()
        ))
    })
}

#[cfg(target_os = "windows")]
fn atomic_replace_file(temp_path: &Path, path: &Path) -> Result<(), BindingError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let temp_wide: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let path_wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let moved = unsafe {
        MoveFileExW(
            temp_wide.as_ptr(),
            path_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        return Err(BindingError(format!(
            "replace {} with {}: {}",
            path.display(),
            temp_path.display(),
            std::io::Error::last_os_error()
        )));
    }
    Ok(())
}

#[cfg(not(any(unix, target_os = "windows")))]
fn atomic_replace_file(temp_path: &Path, path: &Path) -> Result<(), BindingError> {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(BindingError(format!("remove {}: {error}", path.display())));
        }
    }
    std::fs::rename(temp_path, path).map_err(|error| {
        BindingError(format!(
            "replace {} with {}: {error}",
            path.display(),
            temp_path.display()
        ))
    })
}

/// Remove the discovery file only when it still belongs to `expected`.
///
/// Multiple native processes share the legacy discovery path. A server that is
/// shutting down must not delete a newer process's binding after that newer
/// process overwrote the file. Missing files and ownership mismatches are
/// idempotent success; malformed files are preserved and surfaced.
pub fn remove_binding_if_owned(expected: &McpBinding) -> Result<(), BindingError> {
    let _ownership_guard = BindingOwnershipGuard::acquire()?;
    let path = binding_path();
    let body = match std::fs::read_to_string(&path) {
        Ok(body) => body,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(BindingError(format!("read {}: {error}", path.display())));
        }
    };
    let current: McpBinding = serde_json::from_str(&body)
        .map_err(|error| BindingError(format!("parse {}: {error}", path.display())))?;
    if current.pid != expected.pid
        || current.tcp_addr != expected.tcp_addr
        || current.token != expected.token
    {
        return Ok(());
    }
    std::fs::remove_file(&path)
        .map_err(|error| BindingError(format!("remove {}: {error}", path.display())))
}

/// Cross-process serialization for the canonical discovery path. Both overwrite
/// and compare-delete take this same lock, closing the old read/compare/remove
/// TOCTOU where a shutting-down process could delete a newer process's binding.
#[cfg(target_os = "windows")]
struct BindingOwnershipGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl BindingOwnershipGuard {
    fn acquire() -> Result<Self, BindingError> {
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_OBJECT_0};
        use windows_sys::Win32::System::Threading::{CreateMutexW, WaitForSingleObject, INFINITE};

        let name: Vec<u16> = "Local\\HandshakeSwarmMcpBindingOwnership\0"
            .encode_utf16()
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(BindingError(
                "create cross-process binding ownership mutex failed".to_owned(),
            ));
        }
        #[cfg(test)]
        publish_binding_lock_attempt();
        let wait = unsafe { WaitForSingleObject(handle, INFINITE) };
        if wait != WAIT_OBJECT_0 && wait != WAIT_ABANDONED {
            unsafe {
                CloseHandle(handle);
            }
            return Err(BindingError(format!(
                "wait for cross-process binding ownership mutex failed: {wait}"
            )));
        }
        Ok(Self(handle))
    }
}

#[cfg(target_os = "windows")]
impl Drop for BindingOwnershipGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Threading::ReleaseMutex;
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

#[cfg(unix)]
struct BindingOwnershipGuard(std::fs::File);

#[cfg(unix)]
impl BindingOwnershipGuard {
    fn acquire() -> Result<Self, BindingError> {
        use std::os::fd::AsRawFd;

        unsafe extern "C" {
            fn flock(fd: std::ffi::c_int, operation: std::ffi::c_int) -> std::ffi::c_int;
        }
        const LOCK_EX: std::ffi::c_int = 2;

        let path = binding_lock_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| BindingError(format!("create {}: {error}", parent.display())))?;
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .map_err(|error| BindingError(format!("open {}: {error}", path.display())))?;
        #[cfg(test)]
        publish_binding_lock_attempt();
        if unsafe { flock(file.as_raw_fd(), LOCK_EX) } != 0 {
            return Err(BindingError(format!(
                "lock {}: {}",
                path.display(),
                std::io::Error::last_os_error()
            )));
        }
        Ok(Self(file))
    }
}

#[cfg(unix)]
impl Drop for BindingOwnershipGuard {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;

        unsafe extern "C" {
            fn flock(fd: std::ffi::c_int, operation: std::ffi::c_int) -> std::ffi::c_int;
        }
        const LOCK_UN: std::ffi::c_int = 8;
        let _ = unsafe { flock(self.0.as_raw_fd(), LOCK_UN) };
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
struct BindingOwnershipGuard(std::sync::MutexGuard<'static, ()>);

#[cfg(not(any(unix, target_os = "windows")))]
impl BindingOwnershipGuard {
    fn acquire() -> Result<Self, BindingError> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        Ok(Self(
            LOCK.get_or_init(|| std::sync::Mutex::new(()))
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        ))
    }
}

#[cfg(test)]
fn publish_binding_lock_attempt() {
    let Some(marker) = std::env::var_os("HSK_BINDING_LOCK_ATTEMPT_MARKER") else {
        return;
    };
    std::fs::write(PathBuf::from(marker), b"attempting")
        .expect("publish binding ownership-lock attempt");
}

/// Best-effort owner-only permission restriction on the binding file. Failures are logged, never fatal:
/// on Unix the explicit `0o600` is authoritative; on Windows the per-user `%LOCALAPPDATA%` ACL is the
/// primary control and `icacls` only hardens it (see module docs).
fn restrict_to_owner(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
            tracing::warn!(error = %e, path = %path.display(), "could not set 0o600 on mcp binding file");
        }
    }
    #[cfg(target_os = "windows")]
    {
        restrict_to_owner_windows(path);
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = path;
    }
}

/// Windows owner-only hardening via a managed `icacls` subprocess: reset inherited ACEs and grant only
/// the current user full control. Non-fatal — the per-user LocalAppData ACL is the primary control.
#[cfg(target_os = "windows")]
fn restrict_to_owner_windows(path: &std::path::Path) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // %USERNAME% is the current user; `icacls <file> /inheritance:r /grant:r "%USERNAME%":F`
    // removes inherited ACEs and grants only that user Full control. Best-effort + quiet (no window).
    let Some(user) = std::env::var_os("USERNAME") else {
        tracing::warn!("USERNAME unset; skipping icacls hardening of mcp binding file");
        return;
    };
    let user = user.to_string_lossy().to_string();
    let grant = format!("{user}:F");
    match std::process::Command::new("icacls")
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(&grant)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
    {
        Ok(status) if status.success() => {
            tracing::debug!(path = %path.display(), "icacls hardened mcp binding file to owner-only");
        }
        Ok(status) => {
            tracing::warn!(?status, path = %path.display(), "icacls hardening returned non-zero (binding still under per-user LocalAppData ACL)");
        }
        Err(e) => {
            tracing::warn!(error = %e, "icacls unavailable; binding relies on per-user LocalAppData ACL");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static BINDING_ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn binding_round_trips_through_json_with_pipe() {
        let b = McpBinding {
            tcp_addr: "127.0.0.1:54321".to_owned(),
            pipe_name: Some(r"\\.\pipe\handshake_swarm_4242".to_owned()),
            token: "a".repeat(64),
            pid: 4242,
        };
        let json = b.to_json_string().expect("serialize");
        let back: McpBinding = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(b, back);
        assert!(json.contains("tcp_addr"));
        assert!(json.contains("pipe_name"));
        assert!(json.contains("token"));
    }

    #[test]
    fn pipe_name_omitted_when_none() {
        let b = McpBinding {
            tcp_addr: "127.0.0.1:1".to_owned(),
            pipe_name: None,
            token: "t".to_owned(),
            pid: 1,
        };
        let json = b.to_json_string().expect("serialize");
        assert!(
            !json.contains("pipe_name"),
            "None pipe_name is skipped: {json}"
        );
    }

    #[test]
    fn binding_path_ends_with_expected_components() {
        let p = binding_path();
        let s = p.to_string_lossy().replace('\\', "/");
        assert!(
            s.ends_with(&format!("handshake/{BINDING_FILE_NAME}")),
            "path was {s}"
        );
    }

    #[test]
    fn write_then_remove_is_idempotent() {
        let _guard = BINDING_ENV_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        };
        let written = write_binding(&b).expect("write");
        assert!(written.exists(), "binding file exists after write");
        let read_back: McpBinding =
            serde_json::from_str(&std::fs::read_to_string(&written).unwrap()).unwrap();
        assert_eq!(read_back, b);

        remove_binding().expect("remove");
        assert!(!written.exists(), "binding file gone after remove");
        // Second remove is a no-op (idempotent).
        remove_binding().expect("remove idempotent");

        // restore env + clean up
        match prev {
            Some(v) => std::env::set_var(var, v),
            None => std::env::remove_var(var),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn ownership_fenced_remove_preserves_a_newer_binding() {
        let _guard = BINDING_ENV_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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

        let older = McpBinding {
            tcp_addr: "127.0.0.1:10001".to_owned(),
            pipe_name: None,
            token: "a".repeat(64),
            pid: 10001,
        };
        let newer = McpBinding {
            tcp_addr: "127.0.0.1:10002".to_owned(),
            pipe_name: None,
            token: "b".repeat(64),
            pid: 10002,
        };
        let path = write_binding(&older).expect("write older");
        write_binding(&newer).expect("write newer");

        remove_binding_if_owned(&older).expect("older shutdown is harmless");
        let current: McpBinding =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(current, newer, "older server must preserve newer binding");

        remove_binding_if_owned(&newer).expect("newer removes itself");
        assert!(!path.exists());

        match prev {
            Some(value) => std::env::set_var(var, value),
            None => std::env::remove_var(var),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    #[ignore = "invoked as a child process by cross_process_writes_overlap_under_one_lock"]
    fn binding_cross_process_helper() {
        let Ok(mode) = std::env::var("HSK_BINDING_HELPER_MODE") else {
            return;
        };
        let root = PathBuf::from(
            std::env::var_os("HSK_BINDING_HELPER_ROOT").expect("helper root environment"),
        );
        #[cfg(target_os = "windows")]
        std::env::set_var("LOCALAPPDATA", &root);
        #[cfg(not(target_os = "windows"))]
        std::env::set_var("XDG_DATA_HOME", &root);

        let binding = McpBinding {
            tcp_addr: if mode == "hold" {
                "127.0.0.1:21001"
            } else {
                "127.0.0.1:21002"
            }
            .to_owned(),
            pipe_name: None,
            token: if mode == "hold" {
                "a".repeat(64)
            } else {
                "b".repeat(64)
            },
            pid: if mode == "hold" { 21001 } else { 21002 },
        };
        if matches!(mode.as_str(), "write" | "write-bypass") {
            let ready = root.join("contender-ready");
            std::fs::write(&ready, b"ready").expect("publish contender readiness");
            let go = root.join("contender-go");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            while !go.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "parent did not release contender start barrier"
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let attempting = root.join("contender-lock-attempt");
            if mode == "write" {
                std::env::set_var("HSK_BINDING_LOCK_ATTEMPT_MARKER", &attempting);
                write_binding(&binding).expect("contending helper writes binding");
            } else {
                // Test-only mutation control: use the exact atomic writer while intentionally bypassing
                // BindingOwnershipGuard. The parent applies the same overlap assertion and requires it
                // to go RED, proving the helper protocol detects a missing cross-process lock.
                let path = binding_path();
                std::fs::create_dir_all(path.parent().expect("binding parent"))
                    .expect("create binding dir");
                let json = binding.to_json_string().expect("serialize bypass binding");
                std::fs::write(&attempting, b"attempting")
                    .expect("publish bypassed binding-lock attempt");
                atomic_replace_binding(&path, json.as_bytes())
                    .expect("test-only bypass writes binding atomically");
            }
            std::fs::write(root.join("contender-complete"), b"complete")
                .expect("publish contender completion");
            return;
        }
        assert_eq!(mode, "hold");
        let path = binding_path();
        std::fs::create_dir_all(path.parent().expect("binding parent"))
            .expect("create binding dir");
        let _guard = BindingOwnershipGuard::acquire().expect("holder acquires ownership lock");
        let json = binding.to_json_string().expect("serialize holder binding");
        atomic_replace_binding(&path, json.as_bytes()).expect("holder writes atomically");
        let ready = root.join("holder-ready");
        std::fs::write(&ready, b"ready").expect("publish holder readiness");
        let release = root.join("holder-release");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !release.exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "parent did not release helper lock"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[cfg(any(unix, target_os = "windows"))]
    fn cross_process_overlap_case(contender_mode: &str) -> bool {
        let _guard = BINDING_ENV_GUARD
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let root = std::env::temp_dir().join(format!(
            "hsk_binding_cross_process_{}_{}_{}",
            std::process::id(),
            contender_mode,
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create helper root");
        let executable = std::env::current_exe().expect("current test executable");
        let spawn_helper = |mode: &str| {
            std::process::Command::new(&executable)
                .args([
                    "--ignored",
                    "--exact",
                    "mcp::binding::tests::binding_cross_process_helper",
                    "--nocapture",
                ])
                .env("HSK_BINDING_HELPER_MODE", mode)
                .env("HSK_BINDING_HELPER_ROOT", &root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("spawn binding helper")
        };

        let mut holder = spawn_helper("hold");
        let ready = root.join("holder-ready");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !ready.exists() {
            assert!(
                holder.try_wait().expect("poll holder").is_none(),
                "holder exited before acquiring the ownership lock"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "holder did not publish ownership-lock readiness"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let mut contender = spawn_helper(contender_mode);
        let contender_ready = root.join("contender-ready");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !contender_ready.exists() {
            assert!(
                contender.try_wait().expect("poll contender").is_none(),
                "contender exited before reaching the start barrier"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "contender did not publish start-barrier readiness"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::fs::write(root.join("contender-go"), b"go").expect("release contender start barrier");

        let attempting = root.join("contender-lock-attempt");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !attempting.exists() {
            assert!(
                contender.try_wait().expect("poll contender").is_none(),
                "contender exited before reaching the binding ownership acquire boundary"
            );
            assert!(
                std::time::Instant::now() < deadline,
                "contender did not reach the binding ownership acquire boundary"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let complete = root.join("contender-complete");
        let observation_deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !complete.exists() && std::time::Instant::now() < observation_deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let completed_while_holder_owned_lock = complete.exists();
        assert!(
            completed_while_holder_owned_lock
                || contender
                    .try_wait()
                    .expect("poll blocked contender")
                    .is_none(),
            "contender exited without publishing completion"
        );

        std::fs::write(root.join("holder-release"), b"release").expect("release holder");
        assert!(holder.wait().expect("wait holder").success());
        assert!(contender.wait().expect("wait contender").success());

        #[cfg(target_os = "windows")]
        let app_data_var = "LOCALAPPDATA";
        #[cfg(not(target_os = "windows"))]
        let app_data_var = "XDG_DATA_HOME";
        let previous_app_data = std::env::var_os(app_data_var);
        std::env::set_var(app_data_var, &root);
        let body = std::fs::read_to_string(binding_path()).expect("read final binding");
        let final_binding: McpBinding = serde_json::from_str(&body).expect("complete binding JSON");
        assert_eq!(final_binding.pid, 21002);
        assert_eq!(final_binding.token, "b".repeat(64));
        match previous_app_data {
            Some(value) => std::env::set_var(app_data_var, value),
            None => std::env::remove_var(app_data_var),
        }
        let _ = std::fs::remove_dir_all(&root);
        completed_while_holder_owned_lock
    }

    #[cfg(any(unix, target_os = "windows"))]
    fn assert_cross_process_write_is_serialized(contender_mode: &str) {
        assert!(
            !cross_process_overlap_case(contender_mode),
            "cross-process overlap detector: contender completed while holder owned the ownership lock"
        );
    }

    #[cfg(any(unix, target_os = "windows"))]
    #[test]
    fn cross_process_writes_overlap_under_one_lock() {
        assert_cross_process_write_is_serialized("write");
    }

    #[cfg(any(unix, target_os = "windows"))]
    #[test]
    #[should_panic(
        expected = "cross-process overlap detector: contender completed while holder owned the ownership lock"
    )]
    fn cross_process_overlap_detector_goes_red_when_lock_is_bypassed() {
        assert_cross_process_write_is_serialized("write-bypass");
    }
}
