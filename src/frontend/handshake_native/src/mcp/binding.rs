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

use std::path::PathBuf;

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
    local_app_data_dir().join("handshake").join(BINDING_FILE_NAME)
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
    let json = binding.to_json_string()?;
    std::fs::write(&path, json).map_err(|e| BindingError(format!("write {}: {e}", path.display())))?;
    restrict_to_owner(&path);
    Ok(path)
}

/// Remove the binding file (called on graceful shutdown so an agent does not connect to a closed port).
/// Missing-file is success (idempotent). Other I/O errors are returned for the caller to log.
pub fn remove_binding() -> Result<(), BindingError> {
    let path = binding_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(BindingError(format!("remove {}: {e}", path.display()))),
    }
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
        assert!(!json.contains("pipe_name"), "None pipe_name is skipped: {json}");
    }

    #[test]
    fn binding_path_ends_with_expected_components() {
        let p = binding_path();
        let s = p.to_string_lossy().replace('\\', "/");
        assert!(s.ends_with(&format!("handshake/{BINDING_FILE_NAME}")), "path was {s}");
    }

    #[test]
    fn write_then_remove_is_idempotent() {
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
}
