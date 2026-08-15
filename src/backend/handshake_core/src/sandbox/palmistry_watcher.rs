//! Process adapter for Palmistry's lifecycle-inverted watcher.
//!
//! Palmistry must outlive the native GUI it observes, so it is intentionally
//! not placed in the GUI's kill-on-close job. This adapter is the only spawn
//! authority; callers still reserve and record the ProcessOwnershipLedger
//! lifecycle before treating the watcher as started.

use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Debug)]
struct PalmistrySpawnStageError {
    stage: &'static str,
    source: io::Error,
}

impl std::fmt::Display for PalmistrySpawnStageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Palmistry launch stage `{}` failed: {}",
            self.stage, self.source
        )
    }
}

impl std::error::Error for PalmistrySpawnStageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

fn palmistry_spawn_stage_error(stage: &'static str, source: io::Error) -> io::Error {
    io::Error::new(source.kind(), PalmistrySpawnStageError { stage, source })
}

pub const PALMISTRY_WATCHER_ADAPTER_ID: &str = "palmistry_watcher";
pub const PALMISTRY_BIN_ENV: &str = "HANDSHAKE_PALMISTRY_BIN";
pub const PALMISTRY_SHA256_ENV: &str = "HANDSHAKE_PALMISTRY_SHA256";
const PALMISTRY_EMBEDDED_SHA256: Option<&str> = option_env!("HANDSHAKE_PALMISTRY_EMBEDDED_SHA256");

#[cfg(target_os = "windows")]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
#[cfg(target_os = "windows")]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(target_os = "windows")]
fn palmistry_creation_flags() -> u32 {
    // Breakaway is deliberate and fail-closed: if the containing job does not permit breakaway,
    // CreateProcess fails instead of silently attaching the only crash observer to a kill-on-close
    // job owned by the process it must outlive.
    CREATE_BREAKAWAY_FROM_JOB | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
}

#[derive(Clone)]
pub struct PalmistrySpawnSpec {
    pub session_id: Uuid,
    pub launch_nonce: Uuid,
    pub parent_pid: u32,
    pub ring: PathBuf,
    pub survivor_dir: PathBuf,
    pub panic_signal: PathBuf,
    pub panic_ack: PathBuf,
    pub shutdown_signal: PathBuf,
    pub ready_signal: PathBuf,
    pub watcher_signing_secret: std::sync::Arc<Zeroizing<[u8; 32]>>,
}

impl std::fmt::Debug for PalmistrySpawnSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PalmistrySpawnSpec")
            .field("session_id", &self.session_id)
            .field("launch_nonce", &self.launch_nonce)
            .field("parent_pid", &self.parent_pid)
            .field("ring", &self.ring)
            .field("survivor_dir", &self.survivor_dir)
            .field("panic_signal", &self.panic_signal)
            .field("panic_ack", &self.panic_ack)
            .field("shutdown_signal", &self.shutdown_signal)
            .field("ready_signal", &self.ready_signal)
            .field("watcher_signing_secret", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug)]
pub struct SpawnedPalmistry {
    pub child: Child,
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub os_creation_time_100ns: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PalmistryWatcherAdapter;

impl PalmistryWatcherAdapter {
    fn adapter_id() -> crate::sandbox::AdapterId {
        crate::sandbox::AdapterId::new(PALMISTRY_WATCHER_ADAPTER_ID)
    }

    fn unsupported(operation: &str) -> crate::sandbox::SandboxAdapterError {
        crate::sandbox::SandboxAdapterError::SpawnFailed {
            adapter_id: Self::adapter_id(),
            reason: format!(
                "PalmistryWatcherAdapter {operation} is unavailable through generic handles"
            ),
        }
    }

    pub fn resolve_executable() -> io::Result<PathBuf> {
        Self::resolve_executable_with_pin().map(|(path, _, _guard)| path)
    }

    fn resolve_executable_with_pin() -> io::Result<(PathBuf, String, File)> {
        let configured = std::env::var_os(PALMISTRY_BIN_ENV).map(PathBuf::from);
        if configured.is_some() && !cfg!(debug_assertions) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "HANDSHAKE_PALMISTRY_BIN is restricted to debug/test builds",
            ));
        }
        let candidate = match configured {
            Some(path) => path,
            None => {
                let current = std::env::current_exe()?;
                current
                    .parent()
                    .ok_or_else(|| io::Error::other("backend executable has no parent"))?
                    .join(if cfg!(windows) {
                        "palmistry.exe"
                    } else {
                        "palmistry"
                    })
            }
        };
        let canonical = fs::canonicalize(candidate)?;
        let expected = if cfg!(windows) {
            OsString::from("palmistry.exe")
        } else {
            OsString::from("palmistry")
        };
        if canonical.file_name() != Some(expected.as_os_str()) || !canonical.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Palmistry executable must be a canonical file named palmistry",
            ));
        }
        let configured_pin = std::env::var(PALMISTRY_SHA256_ENV).ok();
        if configured_pin.is_some() && !cfg!(debug_assertions) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "HANDSHAKE_PALMISTRY_SHA256 is restricted to debug/test builds",
            ));
        }
        let development_sidecar_pin = if cfg!(debug_assertions) && configured_pin.is_none() {
            Some(
                fs::read_to_string(canonical.with_file_name(format!(
                    "{}.sha256",
                    canonical
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("palmistry")
                )))?
                .trim()
                .to_owned(),
            )
        } else {
            None
        };
        let trusted_pin = if cfg!(debug_assertions) {
            configured_pin
                .as_deref()
                .or(development_sidecar_pin.as_deref())
                .or(PALMISTRY_EMBEDDED_SHA256)
        } else {
            PALMISTRY_EMBEDDED_SHA256
        };
        // Retain a read handle that denies write/delete sharing through spawn, launched-image
        // verification, and signing-seed delivery. This closes the Windows hash-then-spawn
        // replacement window without CREATE_SUSPENDED (which would change the lifecycle contract).
        let mut executable_guard = open_pinned_executable_guard(&canonical)?;
        let pinned_sha256 = validate_open_executable_pin(&mut executable_guard, trusted_pin)?;
        Ok((canonical, pinned_sha256, executable_guard))
    }

    pub fn spawn(spec: &PalmistrySpawnSpec) -> io::Result<SpawnedPalmistry> {
        let (executable, executable_sha256, _executable_guard) =
            Self::resolve_executable_with_pin()
                .map_err(|error| palmistry_spawn_stage_error("resolve-and-pin", error))?;
        let mut command = Command::new(&executable);
        command
            .env_clear()
            .arg("--session-id")
            .arg(spec.session_id.to_string())
            .arg("--launch-nonce")
            .arg(spec.launch_nonce.to_string())
            .arg("--parent-pid")
            .arg(spec.parent_pid.to_string())
            .arg("--ring")
            .arg(&spec.ring)
            .arg("--survivor-dir")
            .arg(&spec.survivor_dir)
            .arg("--panic-signal")
            .arg(&spec.panic_signal)
            .arg("--panic-ack")
            .arg(&spec.panic_ack)
            .arg("--shutdown-signal")
            .arg(&spec.shutdown_signal)
            .arg("--ready-signal")
            .arg(&spec.ready_signal)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // A quiet background helper with its own process group. It is not
            // assigned to the monitored GUI's kill-on-close job, so GUI death
            // cannot erase the observer before it writes the survivor record.
            command.creation_flags(palmistry_creation_flags());
        }

        let mut child = command
            .spawn()
            .map_err(|error| palmistry_spawn_stage_error("command-spawn", error))?;
        // The watcher blocks on the exact 32-byte stdin bootstrap before it can write readiness or
        // evidence. `_executable_guard` still denies write/delete sharing while the launched image is
        // verified and remains held until after the signing seed is delivered below.
        if let Err(error) = verify_spawned_image(&child, &executable, &executable_sha256) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(palmistry_spawn_stage_error(
                "launched-image-verification",
                error,
            ));
        }
        let write_secret = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("Palmistry signing-key pipe was not created"))
            .and_then(|mut stdin| {
                stdin.write_all(&spec.watcher_signing_secret.as_ref()[..])?;
                stdin.flush()
            });
        if let Err(error) = write_secret {
            let _ = child.kill();
            let _ = child.wait();
            return Err(palmistry_spawn_stage_error("signing-pipe-delivery", error));
        }
        let os_creation_time_100ns =
            match crate::sandbox::handshake_native::process_creation_time_100ns(child.id()) {
                Ok(value) => value,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(palmistry_spawn_stage_error("creation-time-query", error));
                }
            };
        Ok(SpawnedPalmistry {
            child,
            executable,
            executable_sha256,
            os_creation_time_100ns,
        })
    }
}

/// Palmistry has a bespoke lifecycle-inverted launch contract, but it is still
/// a first-class SandboxAdapter owner. This implementation gives production
/// restart/staleness recovery the same exact adapter authority that launched
/// the watcher instead of falling back to a generic PID killer.
#[async_trait::async_trait]
impl crate::sandbox::SandboxAdapter for PalmistryWatcherAdapter {
    async fn spawn(
        &self,
        _spec: crate::sandbox::ProcessSpec,
    ) -> Result<crate::sandbox::ProcessHandle, crate::sandbox::SandboxAdapterError> {
        Err(Self::unsupported("generic spawn"))
    }

    async fn exec(
        &self,
        _handle: &crate::sandbox::ProcessHandle,
        _cmd: crate::sandbox::Command,
    ) -> Result<crate::sandbox::ExecResult, crate::sandbox::SandboxAdapterError> {
        Err(Self::unsupported("exec"))
    }

    async fn fs_bind(
        &self,
        _handle: &crate::sandbox::ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: crate::sandbox::BindMode,
    ) -> Result<(), crate::sandbox::SandboxAdapterError> {
        Err(Self::unsupported("fs_bind"))
    }

    async fn net_policy(
        &self,
        _handle: &crate::sandbox::ProcessHandle,
        _policy: crate::sandbox::NetPolicy,
    ) -> Result<(), crate::sandbox::SandboxAdapterError> {
        Err(Self::unsupported("net_policy"))
    }

    async fn kill(
        &self,
        _handle: &crate::sandbox::ProcessHandle,
        _signal: crate::sandbox::Signal,
    ) -> Result<(), crate::sandbox::SandboxAdapterError> {
        Err(Self::unsupported("generic kill"))
    }

    async fn reclaim_detached(
        &self,
        identity: &crate::sandbox::DetachedProcessIdentity,
        _signal: crate::sandbox::Signal,
    ) -> Result<(), crate::sandbox::SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            crate::sandbox::handshake_native::reclaim_verified_detached_process(
                identity,
                PALMISTRY_WATCHER_ADAPTER_ID,
            )
            .await
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = identity;
            Err(Self::unsupported("detached reclaim on this host"))
        }
    }

    async fn status(
        &self,
        _handle: &crate::sandbox::ProcessHandle,
    ) -> Result<crate::sandbox::ProcessStatus, crate::sandbox::SandboxAdapterError> {
        Err(Self::unsupported("generic status"))
    }

    async fn detached_status(
        &self,
        identity: &crate::sandbox::DetachedProcessIdentity,
    ) -> Result<crate::sandbox::ProcessStatus, crate::sandbox::SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            crate::sandbox::handshake_native::verified_detached_process_status(
                identity,
                PALMISTRY_WATCHER_ADAPTER_ID,
            )
            .await
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = identity;
            Err(Self::unsupported("detached status on this host"))
        }
    }

    async fn exit_code(
        &self,
        _handle: &crate::sandbox::ProcessHandle,
    ) -> Result<Option<i32>, crate::sandbox::SandboxAdapterError> {
        Err(Self::unsupported("exit_code"))
    }

    fn capabilities(&self) -> crate::sandbox::AdapterCapabilities {
        #[cfg(target_os = "windows")]
        {
            let mut capabilities = crate::sandbox::default_no_op_capabilities();
            capabilities.adapter_id = Self::adapter_id();
            capabilities.runtime_available = true;
            capabilities
        }
        #[cfg(not(target_os = "windows"))]
        {
            let mut capabilities = crate::sandbox::default_no_op_capabilities();
            capabilities.adapter_id = Self::adapter_id();
            capabilities
        }
    }
}

fn verify_spawned_image(child: &Child, expected: &Path, expected_sha256: &str) -> io::Result<()> {
    let launched = launched_image_path(child)?;
    if fs::canonicalize(&launched)? != expected || sha256_file(&launched)? != expected_sha256 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "launched Palmistry image identity differs from the attested executable",
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn launched_image_path(child: &Child) -> io::Result<PathBuf> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Threading::QueryFullProcessImageNameW;
    let mut buffer = vec![0u16; 32_768];
    let mut len = buffer.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(child.as_raw_handle(), 0, buffer.as_mut_ptr(), &mut len)
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(PathBuf::from(String::from_utf16_lossy(
        &buffer[..len as usize],
    )))
}

#[cfg(target_os = "linux")]
fn launched_image_path(child: &Child) -> io::Result<PathBuf> {
    fs::read_link(format!("/proc/{}/exe", child.id()))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn launched_image_path(_child: &Child) -> io::Result<PathBuf> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "post-spawn image identity is unsupported on this platform",
    ))
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    sha256_open_file(&mut file)
}

fn sha256_open_file(file: &mut File) -> io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn validate_executable_pin(path: &Path, expected_sha256: Option<&str>) -> io::Result<String> {
    let mut file = File::open(path)?;
    validate_open_executable_pin(&mut file, expected_sha256)
}

fn validate_open_executable_pin(
    file: &mut File,
    expected_sha256: Option<&str>,
) -> io::Result<String> {
    let expected_sha256 = expected_sha256.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Palmistry launch requires a trusted embedded SHA-256 pin",
        )
    })?;
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Palmistry SHA-256 pin must be exactly 64 hexadecimal characters",
        ));
    }
    if sha256_open_file(file)? != expected_sha256 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Palmistry executable does not match the independently configured SHA-256 pin",
        ));
    }
    Ok(expected_sha256.to_owned())
}

fn open_pinned_executable_guard(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
        // Sharing is symmetric on Windows: this open fails if an existing writer is present, and
        // while retained it prevents new write/delete handles and path replacement.
        options.share_mode(FILE_SHARE_READ);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_id_is_stable_and_model_addressable() {
        assert_eq!(PALMISTRY_WATCHER_ADAPTER_ID, "palmistry_watcher");
        assert_eq!(PALMISTRY_BIN_ENV, "HANDSHAKE_PALMISTRY_BIN");
        assert_eq!(PALMISTRY_SHA256_ENV, "HANDSHAKE_PALMISTRY_SHA256");
    }

    #[test]
    fn spawn_stage_context_preserves_error_kind_and_source() {
        use std::error::Error as _;

        let error = palmistry_spawn_stage_error(
            "command-spawn",
            io::Error::new(io::ErrorKind::PermissionDenied, "access denied fixture"),
        );

        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(
            error.to_string(),
            "Palmistry launch stage `command-spawn` failed: access denied fixture"
        );
        let source = error.source().expect("staged io error preserves source");
        assert_eq!(source.to_string(), "access denied fixture");
    }

    #[test]
    fn executable_pin_is_required_and_must_match() {
        let path = std::env::temp_dir().join(format!("palmistry-pin-{}.bin", Uuid::now_v7()));
        fs::write(&path, b"palmistry-test-image").expect("write fixture");
        let digest = sha256_file(&path).expect("hash fixture");
        assert!(validate_executable_pin(&path, None).is_err());
        assert!(validate_executable_pin(&path, Some(&"00".repeat(32))).is_err());
        assert!(validate_executable_pin(&path, Some(&digest)).is_ok());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn spawn_contract_clears_inherited_environment_and_reports_no_jail() {
        let source = include_str!("palmistry_watcher.rs");
        assert!(source.contains(".env_clear()"));
        assert!(source.contains("options.share_mode(FILE_SHARE_READ)"));
        assert!(source.contains("_executable_guard"));
        assert!(source.contains("default_no_op_capabilities"));
        assert!(!source.contains("windows_native_jail_target_capabilities"));
    }

    #[test]
    fn production_trust_root_is_compiled_in_not_a_co_mutable_sidecar() {
        let source = include_str!("palmistry_watcher.rs");
        assert!(source.contains("option_env!(\"HANDSHAKE_PALMISTRY_EMBEDDED_SHA256\")"));
        assert!(source.contains("cfg!(debug_assertions) && configured_pin.is_none()"));
        assert!(!source.contains("let manifest_pin"));
        assert!(source.contains("HANDSHAKE_PALMISTRY_SHA256 is restricted to debug/test builds"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn watcher_spawn_is_breakaway_and_quiet() {
        let flags = palmistry_creation_flags();
        assert_ne!(flags & CREATE_BREAKAWAY_FROM_JOB, 0);
        assert_ne!(flags & CREATE_NEW_PROCESS_GROUP, 0);
        assert_ne!(flags & CREATE_NO_WINDOW, 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn pinned_executable_guard_denies_write_and_delete_until_released() {
        let path = std::env::temp_dir().join(format!("palmistry-guard-{}.bin", Uuid::now_v7()));
        fs::write(&path, b"palmistry-test-image").expect("write fixture");
        let guard = open_pinned_executable_guard(&path).expect("open pinned executable guard");

        assert!(
            OpenOptions::new().write(true).open(&path).is_err(),
            "retained read handle must deny a competing writer"
        );
        assert!(
            fs::remove_file(&path).is_err(),
            "retained read handle must deny delete/path replacement"
        );

        drop(guard);
        fs::write(&path, b"replacement").expect("write succeeds after guard release");
        fs::remove_file(path).expect("cleanup fixture after guard release");
    }
}
