use std::{
    collections::BTreeMap,
    ffi::OsString,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use async_trait::async_trait;

#[cfg(target_os = "windows")]
use super::{
    job_object_wrap::{WindowsNativeJobGuard, WindowsNativeJobLimits},
    restricted_appcontainer::{
        launch_restricted_appcontainer_with_io, WindowsNativeLaunchOptions, WindowsNativeLaunchedIo,
    },
};
use super::{
    windows_native_jail_runtime_capabilities, windows_native_jail_target_capabilities,
    windows_native_jail_unavailable_capabilities,
};
use crate::sandbox::{
    AdapterCapabilities, AdapterId, BindMode, BindSpec, Command, ExecResult, NetPolicy,
    ProcessHandle, ProcessSpec, ProcessStatus, ResourceLimits, SandboxAdapter, SandboxAdapterError,
    Signal, WINDOWS_NATIVE_JAIL_ADAPTER_ID, WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
};

const FILE_GENERIC_EXECUTE_MASK: u32 = 1_179_808;

const BACKEND_NOT_APPROVED_REASON: &str = concat!(
    "WindowsNativeJailAdapter unavailable: this build was not produced on a Windows host with the ",
    "`win-native-integration` cargo feature. MT-045 approved rappct 0.13.3 as the AppContainer ",
    "substrate and MT-046 composes the Restricted Token + Job Object bridge on top, but neither is ",
    "active in the current build. Rebuild on Windows with `--features win-native-integration` to ",
    "enable the runtime backend."
);

#[cfg(target_os = "windows")]
static APP_CONTAINER_PROFILE_API_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone)]
pub struct WindowsNativeJailAdapter {
    backend: WindowsNativeJailBackend,
}

#[derive(Debug, Clone)]
enum WindowsNativeJailBackend {
    Unavailable {
        reason: String,
    },
    #[cfg(target_os = "windows")]
    Native {
        lpac_supported: bool,
        processes: Arc<Mutex<BTreeMap<String, WindowsNativeProcess>>>,
    },
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct WindowsNativeProcess {
    exit_code: Arc<Mutex<Option<i32>>>,
    killed_by: Arc<Mutex<Option<Signal>>>,
    job_guard: Arc<Mutex<Option<WindowsNativeJobGuard>>>,
}

impl WindowsNativeJailAdapter {
    pub fn unavailable_for_current_host() -> Self {
        Self::unavailable(BACKEND_NOT_APPROVED_REASON)
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            backend: WindowsNativeJailBackend::Unavailable {
                reason: reason.into(),
            },
        }
    }

    /// Planned MT-046 capability contract for a future approved backend.
    ///
    /// Runtime selection must use `SandboxAdapter::capabilities()` instead.
    pub fn target_capability_contract() -> AdapterCapabilities {
        windows_native_jail_target_capabilities()
    }

    pub fn unavailable_runtime_capabilities() -> AdapterCapabilities {
        windows_native_jail_unavailable_capabilities()
    }

    pub async fn try_new() -> Result<Self, SandboxAdapterError> {
        if !WINDOWS_NATIVE_JAIL_BACKEND_APPROVED {
            return Err(unavailable_error(BACKEND_NOT_APPROVED_REASON));
        }

        #[cfg(target_os = "windows")]
        {
            probe_rappct_appcontainer()?;

            Ok(Self {
                backend: WindowsNativeJailBackend::Native {
                    lpac_supported: rappct::supports_lpac().is_ok(),
                    processes: Arc::new(Mutex::new(BTreeMap::new())),
                },
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err(unavailable_error(
                "WindowsNativeJailAdapter requires a Windows host",
            ))
        }
    }

    fn ensure_handle(&self, handle: &ProcessHandle) -> Result<(), SandboxAdapterError> {
        if handle.adapter_id != AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID) {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn backend_unavailable(&self) -> SandboxAdapterError {
        match &self.backend {
            WindowsNativeJailBackend::Unavailable { reason } => unavailable_error(reason),
            #[cfg(target_os = "windows")]
            WindowsNativeJailBackend::Native { .. } => unavailable_error(
                "WindowsNativeJailAdapter backend unexpectedly unavailable after initialization",
            ),
        }
    }

    #[cfg(target_os = "windows")]
    fn runtime_state(
        &self,
    ) -> Result<(bool, Arc<Mutex<BTreeMap<String, WindowsNativeProcess>>>), SandboxAdapterError>
    {
        match &self.backend {
            WindowsNativeJailBackend::Native {
                lpac_supported,
                processes,
            } => Ok((*lpac_supported, processes.clone())),
            WindowsNativeJailBackend::Unavailable { reason } => Err(unavailable_error(reason)),
        }
    }

    #[cfg(target_os = "windows")]
    fn process_for_handle(
        &self,
        handle: &ProcessHandle,
    ) -> Result<WindowsNativeProcess, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        let (_, processes) = self.runtime_state()?;
        let process = processes
            .lock()
            .map_err(|_| spawn_failed("WindowsNativeJailAdapter process table poisoned"))?
            .get(&handle.sandbox_internal_id)
            .cloned()
            .ok_or(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            })?;
        Ok(process)
    }
}

#[cfg(target_os = "windows")]
fn ensure_appcontainer_profile(
    name: &str,
    display: &str,
    description: Option<&str>,
) -> Result<rappct::AppContainerProfile, String> {
    let _guard = APP_CONTAINER_PROFILE_API_LOCK
        .lock()
        .map_err(|_| "AppContainer profile API lock poisoned".to_string())?;
    rappct::AppContainerProfile::ensure(name, display, description)
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
fn delete_appcontainer_profile(profile: rappct::AppContainerProfile) -> Result<(), String> {
    let _guard = APP_CONTAINER_PROFILE_API_LOCK
        .lock()
        .map_err(|_| "AppContainer profile API lock poisoned".to_string())?;
    profile.delete().map_err(|error| error.to_string())
}

#[cfg(target_os = "windows")]
struct AppContainerProfileCleanup {
    profile: Option<rappct::AppContainerProfile>,
}

#[cfg(target_os = "windows")]
impl AppContainerProfileCleanup {
    fn new(profile: rappct::AppContainerProfile) -> Self {
        Self {
            profile: Some(profile),
        }
    }

    fn profile(&self) -> &rappct::AppContainerProfile {
        self.profile
            .as_ref()
            .expect("AppContainer profile cleanup guard should hold a profile")
    }

    fn into_profile(mut self) -> rappct::AppContainerProfile {
        self.profile
            .take()
            .expect("AppContainer profile cleanup guard should hold a profile")
    }
}

#[cfg(target_os = "windows")]
impl Drop for AppContainerProfileCleanup {
    fn drop(&mut self) {
        if let Some(profile) = self.profile.take() {
            let _ = delete_appcontainer_profile(profile);
        }
    }
}

#[cfg(target_os = "windows")]
fn probe_rappct_appcontainer() -> Result<(), SandboxAdapterError> {
    let mut last_error = None;
    for attempt in 1..=3 {
        let probe_name = format!(
            "handshake.mt046.probe.{}.{}",
            std::process::id(),
            uuid::Uuid::now_v7().simple()
        );
        match ensure_appcontainer_profile(&probe_name, &probe_name, Some("Handshake MT-046 probe"))
        {
            Ok(profile) => {
                let _ = delete_appcontainer_profile(profile);
                return Ok(());
            }
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(25 * attempt));
            }
        }
    }

    Err(unavailable_error(format!(
        "rappct AppContainer probe failed for MT-046 backend after retries: {}",
        last_error.unwrap_or_else(|| "unknown AppContainer profile error".to_string())
    )))
}

#[async_trait]
impl SandboxAdapter for WindowsNativeJailAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        if spec.cmd.is_empty() {
            return Err(spawn_failed(
                "WindowsNativeJailAdapter requires ProcessSpec.cmd; empty command refused",
            ));
        }

        #[cfg(target_os = "windows")]
        {
            let (lpac_supported, processes) = self.runtime_state()?;
            spawn_windows_native_process(spec, lpac_supported, processes)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = spec;
            Err(self.backend_unavailable())
        }
    }

    async fn exec(
        &self,
        handle: &ProcessHandle,
        _cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        Err(spawn_failed(
            "WindowsNativeJailAdapter does not support exec; declare cmd in ProcessSpec",
        ))
    }

    async fn fs_bind(
        &self,
        handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        Err(spawn_failed(
            "WindowsNativeJailAdapter post-spawn fs_bind unsupported; declare binds in ProcessSpec.binds",
        ))
    }

    async fn net_policy(
        &self,
        handle: &ProcessHandle,
        _policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        self.ensure_handle(handle)?;
        Err(SandboxAdapterError::NetPolicyApplyFailed {
            adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
            reason: "WindowsNativeJailAdapter post-spawn net_policy unsupported; declare policy before spawn".to_string(),
        })
    }

    async fn kill(
        &self,
        handle: &ProcessHandle,
        signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            let process = self.process_for_handle(handle)?;
            *process
                .killed_by
                .lock()
                .map_err(|_| spawn_failed("WindowsNativeJailAdapter killed state poisoned"))? =
                Some(signal);
            let guard = process
                .job_guard
                .lock()
                .map_err(|_| spawn_failed("WindowsNativeJailAdapter job guard poisoned"))?
                .take();
            if let Some(guard) = guard {
                let _ = guard.terminate(1);
            }
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.ensure_handle(handle)?;
            Err(self.backend_unavailable())
        }
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            let process = self.process_for_handle(handle)?;
            let exit_code = *process
                .exit_code
                .lock()
                .map_err(|_| spawn_failed("WindowsNativeJailAdapter exit state poisoned"))?;
            let killed_by = *process
                .killed_by
                .lock()
                .map_err(|_| spawn_failed("WindowsNativeJailAdapter killed state poisoned"))?;
            Ok(match (exit_code, killed_by) {
                (Some(_), Some(signal)) => ProcessStatus::Killed { by_signal: signal },
                (Some(code), None) => ProcessStatus::Exited { code },
                (None, _) => ProcessStatus::Running,
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.ensure_handle(handle)?;
            Err(self.backend_unavailable())
        }
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            let process = self.process_for_handle(handle)?;
            let exit_code = *process
                .exit_code
                .lock()
                .map_err(|_| spawn_failed("WindowsNativeJailAdapter exit state poisoned"))?;
            Ok(exit_code)
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.ensure_handle(handle)?;
            Err(self.backend_unavailable())
        }
    }

    fn capabilities(&self) -> AdapterCapabilities {
        match &self.backend {
            WindowsNativeJailBackend::Unavailable { .. } => {
                Self::unavailable_runtime_capabilities()
            }
            #[cfg(target_os = "windows")]
            WindowsNativeJailBackend::Native { .. } => windows_native_jail_runtime_capabilities(),
        }
    }
}

#[cfg(target_os = "windows")]
fn spawn_windows_native_process(
    spec: ProcessSpec,
    lpac_supported: bool,
    processes: Arc<Mutex<BTreeMap<String, WindowsNativeProcess>>>,
) -> Result<ProcessHandle, SandboxAdapterError> {
    reject_unsupported_network_policy(&spec.net_policy)?;
    validate_bind_hosts(&spec.binds)?;

    let internal_id = format!("handshake.mt046.{}", uuid::Uuid::now_v7().simple());
    let profile = ensure_appcontainer_profile(
        &internal_id,
        "Handshake Windows Native Jail",
        Some("Handshake MT-046 WindowsNativeJailAdapter"),
    )
    .map_err(|error| {
        spawn_failed(format!(
            "rappct AppContainer profile creation failed: {error}"
        ))
    })?;
    let profile_cleanup = AppContainerProfileCleanup::new(profile);

    apply_bind_grants(profile_cleanup.profile(), &spec.binds)?;

    let mut builder = rappct::SecurityCapabilitiesBuilder::new(&profile_cleanup.profile().sid);
    if lpac_supported {
        builder = builder.with_lpac_defaults().lpac(true);
    }
    match spec.net_policy {
        NetPolicy::DenyAll => {}
        NetPolicy::LoopbackOnly | NetPolicy::Allowlist(_) => {
            unreachable!("unsupported network policy rejected before capability build")
        }
    }
    let security = builder.build().map_err(|error| {
        spawn_failed(format!("rappct security capability build failed: {error}"))
    })?;

    let exe = resolve_executable(&spec)?;
    let args = spec.cmd.iter().skip(1).cloned().collect::<Vec<_>>();
    let mut child = launch_restricted_appcontainer_with_io(
        &security,
        WindowsNativeLaunchOptions {
            exe,
            args,
            cwd: launch_cwd(&spec),
            env: launch_env(&spec.env),
            job_limits: job_limits(&spec.resource_limits),
            startup_timeout: Some(Duration::from_secs(10)),
        },
    )
    .map_err(|error| {
        spawn_failed(format!(
            "Windows native AppContainer + Restricted Token launch failed: {error}"
        ))
    })?;

    let pid = child.pid;
    let job_guard = Arc::new(Mutex::new(child.job_guard.take()));
    drop(child.stdin.take());
    drain_pipe(child.stdout.take());
    drain_pipe(child.stderr.take());

    let exit_code = Arc::new(Mutex::new(None));
    let killed_by = Arc::new(Mutex::new(None));
    let process = WindowsNativeProcess {
        exit_code: exit_code.clone(),
        killed_by: killed_by.clone(),
        job_guard: job_guard.clone(),
    };

    processes
        .lock()
        .map_err(|_| spawn_failed("WindowsNativeJailAdapter process table poisoned"))?
        .insert(internal_id.clone(), process);

    let profile = profile_cleanup.into_profile();
    spawn_waiter(child, profile, exit_code.clone());
    spawn_timeout_guard(
        spec.resource_limits.timeout_ms,
        exit_code,
        killed_by,
        job_guard,
    );

    Ok(ProcessHandle::new(
        AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        Some(pid),
        internal_id,
    ))
}

#[cfg(target_os = "windows")]
fn reject_unsupported_network_policy(policy: &NetPolicy) -> Result<(), SandboxAdapterError> {
    match policy {
        NetPolicy::DenyAll => Ok(()),
        NetPolicy::LoopbackOnly => Err(SandboxAdapterError::NetPolicyApplyFailed {
            adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
            reason: "Windows native AppContainer launch cannot honor LoopbackOnly without loopback exemption support; use DenyAll for MT-046".to_string(),
        }),
        NetPolicy::Allowlist(_) => Err(SandboxAdapterError::NetPolicyApplyFailed {
            adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
            reason: "Windows native AppContainer launch cannot enforce host allowlists without broad internetClient capability; use DenyAll for MT-046".to_string(),
        }),
    }
}

#[cfg(target_os = "windows")]
fn validate_bind_hosts(binds: &[BindSpec]) -> Result<(), SandboxAdapterError> {
    for bind in binds {
        if !bind.host_path.exists() {
            return Err(SandboxAdapterError::BindHostPathMissing {
                host_path: bind.host_path.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_bind_grants(
    profile: &rappct::AppContainerProfile,
    binds: &[BindSpec],
) -> Result<(), SandboxAdapterError> {
    for bind in binds {
        let target = if bind.host_path.is_dir() {
            rappct::acl::ResourcePath::Directory(bind.host_path.clone())
        } else {
            rappct::acl::ResourcePath::File(bind.host_path.clone())
        };
        let access = match bind.mode {
            BindMode::NoExec => rappct::acl::AccessMask::FILE_GENERIC_READ,
            BindMode::ReadOnly => rappct::acl::AccessMask(
                rappct::acl::AccessMask::FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE_MASK,
            ),
            BindMode::ReadWrite => rappct::acl::AccessMask(
                rappct::acl::AccessMask::FILE_GENERIC_READ.0
                    | rappct::acl::AccessMask::FILE_GENERIC_WRITE.0
                    | FILE_GENERIC_EXECUTE_MASK,
            ),
        };
        rappct::acl::grant_to_package(target, &profile.sid, access).map_err(|error| {
            spawn_failed(format!(
                "rappct AppContainer ACL grant failed for {}: {error}",
                bind.host_path.display()
            ))
        })?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn resolve_executable(spec: &ProcessSpec) -> Result<PathBuf, SandboxAdapterError> {
    let exe_arg = spec
        .cmd
        .first()
        .ok_or_else(|| spawn_failed("WindowsNativeJailAdapter requires ProcessSpec.cmd"))?;
    let requested = PathBuf::from(exe_arg);
    if requested.is_absolute() {
        return Ok(requested);
    }

    let root = PathBuf::from(spec.image_or_root.as_str());
    if root.is_dir() {
        let joined = root.join(&requested);
        if joined.exists() {
            return Ok(joined);
        }
    }

    which::which(exe_arg)
        .or_else(|_| which::which(format!("{exe_arg}.exe")))
        .map_err(|_| {
            spawn_failed(format!(
                "WindowsNativeJailAdapter executable not found: {exe_arg}"
            ))
        })
}

#[cfg(target_os = "windows")]
fn launch_cwd(spec: &ProcessSpec) -> Option<PathBuf> {
    spec.cwd
        .clone()
        .or_else(|| {
            let root = PathBuf::from(spec.image_or_root.as_str());
            root.is_dir().then_some(root)
        })
        .or_else(|| Some(PathBuf::from("C:/Windows/System32")))
}

#[cfg(target_os = "windows")]
fn launch_env(env: &BTreeMap<String, String>) -> Option<Vec<(OsString, OsString)>> {
    if env.is_empty() {
        return None;
    }
    let mut merged = rappct::launch::merge_parent_env(
        env.iter()
            .map(|(key, value)| (OsString::from(key), OsString::from(value)))
            .collect(),
    );
    merged.sort_by(|left, right| {
        left.0
            .to_string_lossy()
            .to_ascii_lowercase()
            .cmp(&right.0.to_string_lossy().to_ascii_lowercase())
    });
    Some(merged)
}

#[cfg(target_os = "windows")]
fn job_limits(limits: &ResourceLimits) -> WindowsNativeJobLimits {
    WindowsNativeJobLimits {
        memory_bytes: limits
            .memory_bytes
            .and_then(|bytes| usize::try_from(bytes).ok()),
        cpu_rate_percent: limits.cpu_cores.and_then(cpu_cores_to_rate_percent),
        kill_on_job_close: true,
    }
}

#[cfg(target_os = "windows")]
fn cpu_cores_to_rate_percent(cpu_cores: u16) -> Option<u32> {
    let available = thread::available_parallelism().ok()?.get() as u32;
    let requested = u32::from(cpu_cores).max(1);
    Some(((requested * 100) / available).clamp(1, 100))
}

#[cfg(target_os = "windows")]
fn drain_pipe(pipe: Option<std::fs::File>) {
    if let Some(mut pipe) = pipe {
        thread::spawn(move || {
            let mut sink = Vec::new();
            let _ = pipe.read_to_end(&mut sink);
        });
    }
}

#[cfg(target_os = "windows")]
fn spawn_waiter(
    child: WindowsNativeLaunchedIo,
    profile: rappct::AppContainerProfile,
    exit_code: Arc<Mutex<Option<i32>>>,
) {
    thread::spawn(move || {
        let code = child.wait(None).map(|code| code as i32).unwrap_or(1);
        if let Ok(mut slot) = exit_code.lock() {
            *slot = Some(code);
        }
        let _ = delete_appcontainer_profile(profile);
    });
}

#[cfg(target_os = "windows")]
fn spawn_timeout_guard(
    timeout_ms: Option<u64>,
    exit_code: Arc<Mutex<Option<i32>>>,
    killed_by: Arc<Mutex<Option<Signal>>>,
    job_guard: Arc<Mutex<Option<WindowsNativeJobGuard>>>,
) {
    if let Some(timeout_ms) = timeout_ms {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(timeout_ms));
            let exited = exit_code.lock().map(|slot| slot.is_some()).unwrap_or(true);
            if exited {
                return;
            }
            if let Ok(mut signal) = killed_by.lock() {
                *signal = Some(Signal::Kill);
            }
            if let Ok(mut guard) = job_guard.lock() {
                if let Some(guard) = guard.take() {
                    let _ = guard.terminate(1);
                }
            }
        });
    }
}

fn unavailable_error(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::AdapterUnavailable {
        adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        reason: reason.to_string(),
    }
}

fn spawn_failed(reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
        reason: reason.to_string(),
    }
}
