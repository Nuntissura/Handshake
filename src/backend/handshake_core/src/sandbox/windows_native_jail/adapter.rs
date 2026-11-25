use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io::Read,
    path::PathBuf,
    process::ExitStatus,
    sync::atomic::{AtomicU8, Ordering},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use async_trait::async_trait;

#[cfg(target_os = "windows")]
use super::{
    acl_transaction::{
        acl_grant_stage_detail, default_acl_recovery_stage_name, ensure_default_acl_recovery,
        AclGrantTarget, AclGrantTargetKind, AppContainerAclTransaction,
    },
    job_object_wrap::{WindowsNativeJobGuard, WindowsNativeJobLimits},
    restricted_appcontainer::{
        launch_restricted_appcontainer_with_io, StartupCancellation, WindowsNativeLaunchOptions, WindowsNativeLaunchedIo,
    },
};
use super::{
    windows_native_jail_runtime_capabilities, windows_native_jail_target_capabilities,
    windows_native_jail_unavailable_capabilities,
};
use crate::sandbox::{
    AdapterCapabilities, AdapterId, AttachedEphemeralPathGuard, AttachedNetworkMode,
    AttachedProcessSpec, AttachedSandboxProcess, BindMode, BindSpec, Command, ExecResult, NetPolicy,
    ProcessHandle, ProcessSpec, ProcessStatus, ResourceLimits, RestartCleanupOutcome,
    SandboxAdapter, SandboxAdapterError, Signal, WINDOWS_NATIVE_JAIL_ADAPTER_ID,
    WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
};

#[cfg(target_os = "windows")]
pub(crate) fn capture_baseline_bindings(paths: &[PathBuf])
    -> Result<crate::sandbox::adapter::AttachedBaselineBindings, SandboxAdapterError>
{
    super::acl_transaction::capture_baseline_bindings(paths)
        .map_err(|reason| attached_spawn_failed(
            &AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID), reason))
}

#[cfg(target_os = "windows")]
pub(crate) fn baseline_file(
    bindings: &crate::sandbox::adapter::AttachedBaselineBindings, path: &std::path::Path,
) -> Result<std::fs::File, SandboxAdapterError> {
    super::acl_transaction::baseline_file(bindings, path)
        .map_err(|reason| attached_spawn_failed(
            &AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID), reason))
}

#[cfg(target_os = "windows")]
static ATTACHED_STARTUP_ADMISSION: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
#[cfg(target_os = "windows")]
const ATTACHED_TERMINATION_REAP_TIMEOUT: Duration = Duration::from_secs(5);
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
        validate_supported_resource_limits(&spec.resource_limits)?;

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
            let guard = process.job_guard.lock()
                .map_err(|_| spawn_failed("WindowsNativeJailAdapter job guard poisoned"))?;
            match guard.as_ref() {
                Some(job) => {
                    job.terminate(1).map_err(spawn_failed)?;
                    *process.killed_by.lock()
                        .map_err(|_| spawn_failed("WindowsNativeJailAdapter killed state poisoned"))? = Some(signal);
                }
                None => {
                    if process.exit_code.lock()
                        .map_err(|_| spawn_failed("WindowsNativeJailAdapter exit state poisoned"))?.is_none()
                    {
                        return Err(spawn_failed("detached process lost shared Job Object before terminal acknowledgement"));
                    }
                }
            }
            Ok(())
        }

        #[cfg(not(target_os = "windows"))]
        {
            self.ensure_handle(handle)?;
            Err(self.backend_unavailable())
        }
    }

    async fn cleanup_after_restart(
        &self,
        handle: &ProcessHandle,
    ) -> Result<RestartCleanupOutcome, SandboxAdapterError> {
        self.ensure_handle(handle)?;
        if !handle.sandbox_internal_id.starts_with("handshake.mt046.") {
            return Err(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            });
        }
        // Native-jail children are assigned to Job Objects configured with
        // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE (see `job_limits`). This handle is
        // from a prior backend boot, so Windows has closed the prior boot's last
        // job handle and terminated its child. No ephemeral process table is
        // consulted after restart.
        Ok(RestartCleanupOutcome::AlreadyAbsent)
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
        NetPolicy::LoopbackOnly | NetPolicy::Allowlist(_) | NetPolicy::HostInherited => {
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
            startup_cancellation: None,
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
    spawn_waiter(child, profile, exit_code.clone(), job_guard.clone());
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
        NetPolicy::HostInherited => Err(SandboxAdapterError::NetPolicyApplyFailed {
            adapter_id: AdapterId::new(WINDOWS_NATIVE_JAIL_ADAPTER_ID),
            reason: "Windows native AppContainer cannot enforce the attached HostInherited contract".to_string(),
        }),
    }
}

#[cfg(target_os = "windows")]
struct WindowsAttachedProcess {
    adapter_id: AdapterId,
    pid: u32,
    child: Option<WindowsNativeLaunchedIo>,
    acl_transaction: Option<AppContainerAclTransaction>,
    ephemeral_cleanup: AttachedEphemeralPathGuard,
    observed_exit: Option<ExitStatus>,
    receipt: crate::sandbox::adapter::AttachedSandboxReceipt,
    confirmed_killed: bool,
}

#[cfg(target_os = "windows")]
impl WindowsAttachedProcess {
    fn error(&self, operation: &str, reason: impl ToString) -> SandboxAdapterError {
        SandboxAdapterError::SpawnFailed {
            adapter_id: self.adapter_id.clone(),
            reason: format!("attached process {operation}: {}", reason.to_string()),
        }
    }

    fn child_mut(
        &mut self,
        operation: &str,
    ) -> Result<&mut WindowsNativeLaunchedIo, SandboxAdapterError> {
        self.child
            .as_mut()
            .ok_or_else(|| SandboxAdapterError::SpawnFailed {
                adapter_id: self.adapter_id.clone(),
                reason: format!("attached process already reaped before {operation}"),
            })
    }

    fn finish_acl_cleanup(&mut self) -> Result<(), SandboxAdapterError> {
        if let Some(transaction) = self.acl_transaction.as_mut() {
            if let Err(reason) = transaction.cleanup() {
                return Err(self.error("AppContainer ACL cleanup", reason));
            }
        }
        self.ephemeral_cleanup.cleanup()
            .map_err(|reason| self.error("ephemeral data-root cleanup", reason))?;
        self.acl_transaction.take();
        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl AttachedSandboxProcess for WindowsAttachedProcess {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn receipt(&self) -> Option<crate::sandbox::adapter::AttachedSandboxReceipt> {
        Some(self.receipt.clone())
    }

    fn take_stdout_file(&mut self) -> Option<std::fs::File> {
        self.child.as_mut().and_then(|child| child.stdout.take())
    }

    fn take_stderr_file(&mut self) -> Option<std::fs::File> {
        self.child.as_mut().and_then(|child| child.stderr.take())
    }

    fn terminate_tree_and_wait_with_timeout(
        &mut self, timeout: Duration,
    ) -> Result<crate::sandbox::adapter::AttachedTerminationOutcome, SandboxAdapterError> {
        let started = std::time::Instant::now();
        if let Some(child) = self.child.as_ref() {
            let job = child.job_guard.as_ref()
                .ok_or_else(|| self.error("terminate", "creation-time Job Object missing"))?;
            if job.active_processes().map_err(|reason| self.error("tree observation", reason))? != 0 {
                job.terminate(1).map_err(|reason| self.error("TerminateJobObject", reason))?;
                self.confirmed_killed = true;
            }
        }
        let status = self.wait_with_timeout(Some(timeout.saturating_sub(started.elapsed())))?;
        Ok(crate::sandbox::adapter::AttachedTerminationOutcome {
            status, killed: self.confirmed_killed,
        })
    }

    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child
            .as_mut()
            .and_then(|child| child.stdout.take())
            .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>)
    }

    fn take_stderr(&mut self) -> Option<Box<dyn Read + Send>> {
        self.child
            .as_mut()
            .and_then(|child| child.stderr.take())
            .map(|pipe| Box::new(pipe) as Box<dyn Read + Send>)
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>, SandboxAdapterError> {
        if let Some(status) = self.observed_exit {
            return Ok(Some(status));
        }
        let code = self
            .child_mut("try_wait")?
            .try_wait()
            .map_err(|reason| self.error("try_wait", reason))?;
        let status = code.map(windows_exit_status);
        if let Some(status) = status {
            self.observed_exit = Some(status);
        }
        Ok(status)
    }

    fn wait(&mut self) -> Result<ExitStatus, SandboxAdapterError> {
        self.wait_with_timeout(None)
    }

    fn terminate_tree_and_wait(&mut self) -> Result<ExitStatus, SandboxAdapterError> {
        self.terminate_tree_and_wait_with_timeout(ATTACHED_TERMINATION_REAP_TIMEOUT)
            .map(|outcome| outcome.status)
    }
}

#[cfg(target_os = "windows")]
impl WindowsAttachedProcess {
    fn wait_with_timeout(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<ExitStatus, SandboxAdapterError> {
        let status = match self.child.as_mut() {
            Some(child) => child.wait_borrowed(timeout).map(windows_exit_status)
                .map_err(|reason| self.error("wait", reason))?,
            None => self.observed_exit.ok_or_else(|| self.error("wait", "no reaped child"))?,
        };
        self.observed_exit = Some(status);
        self.finish_acl_cleanup()?;
        // Release process/job handles only after both OS and cleanup acknowledgements.
        self.child.take();
        Ok(status)
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsAttachedProcess {
    fn drop(&mut self) {
        while self.child.is_some() {
            if self.terminate_tree_and_wait().is_ok() { break; }
            std::thread::sleep(Duration::from_millis(25));
        }
        while self.finish_acl_cleanup().is_err() {
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_exit_status(code: u32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(code)
}

/// Handshake-native attached execution on Windows. This is not a host-native
/// `Command` path: it composes rappct AppContainer capabilities, a restricted
/// primary token, and creation-time Job Object assignment before resuming the
/// child. The async boundary is deadline-bounded; if Win32 creation outlives
/// the deadline, this same worker is joined and returns ownership of any late child.
#[cfg(target_os = "windows")]
pub(crate) async fn spawn_handshake_native_attached(
    adapter_id: AdapterId, spec: AttachedProcessSpec,
    adapter_capabilities: AdapterCapabilities, ephemeral_cleanup: AttachedEphemeralPathGuard,
    baseline_bindings: Option<crate::sandbox::adapter::AttachedBaselineBindings>,
) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
    let deadline = std::time::Instant::now().checked_add(Duration::from_millis(spec.startup_timeout_ms))
        .unwrap_or_else(std::time::Instant::now);
    match spawn_handshake_native_attached_owned(
        adapter_id, spec, adapter_capabilities, ephemeral_cleanup,
        crate::model_runtime::CancellationToken::new(), deadline, baseline_bindings,
    ).await {
        Ok(process) => Ok(process),
        Err(crate::sandbox::adapter::OwnedAttachedSpawnError::BeforeSpawn(error)) => Err(error),
        Err(crate::sandbox::adapter::OwnedAttachedSpawnError::AfterSpawn { mut process, error }) => {
            // Legacy API has no ownership-bearing error. It must settle this
            // same child before it can return a status-only failure.
            loop {
                if process.terminate_tree_and_wait_with_timeout(ATTACHED_TERMINATION_REAP_TIMEOUT).is_ok() {
                    return Err(error);
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
}

#[cfg(target_os = "windows")]
struct AttachedStartupOwnership {
    child: Option<WindowsNativeLaunchedIo>,
    acl_transaction: Option<AppContainerAclTransaction>,
    ephemeral_cleanup: Option<AttachedEphemeralPathGuard>,
    internal_id: Option<String>,
    baseline_bindings: Option<crate::sandbox::adapter::AttachedBaselineBindings>,
}

#[cfg(target_os = "windows")]
async fn settle_prechild_cleanup(ownership: &Arc<Mutex<AttachedStartupOwnership>>) {
    loop {
        let owned = ownership.clone();
        // This handle is always joined. No timeout/cancel branch drops cleanup.
        let settled = tokio::task::spawn_blocking(move || {
            let mut state = owned.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(acl) = state.acl_transaction.as_mut() { acl.cleanup()?; }
            if let Some(ephemeral) = state.ephemeral_cleanup.as_mut() { ephemeral.cleanup()?; }
            Ok::<(), String>(())
        }).await;
        if matches!(settled, Ok(Ok(()))) { return; }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[cfg(target_os = "windows")]
pub(crate) async fn spawn_handshake_native_attached_owned(
    adapter_id: AdapterId, spec: AttachedProcessSpec,
    adapter_capabilities: AdapterCapabilities, ephemeral_cleanup: AttachedEphemeralPathGuard,
    cancel: crate::model_runtime::CancellationToken, deadline: std::time::Instant,
    baseline_bindings: Option<crate::sandbox::adapter::AttachedBaselineBindings>,
) -> Result<Box<dyn AttachedSandboxProcess>, crate::sandbox::adapter::OwnedAttachedSpawnError> {
    use crate::sandbox::adapter::OwnedAttachedSpawnError;
    let ownership = Arc::new(Mutex::new(AttachedStartupOwnership {
        child: None, acl_transaction: None, ephemeral_cleanup: Some(ephemeral_cleanup),
        internal_id: None, baseline_bindings,
    }));
    let reject = |reason: String| attached_spawn_failed(&adapter_id, reason);
    let authorized = match authorize_attached_process_creation(&adapter_id, spec, &adapter_capabilities) {
        Ok(spec) => spec,
        Err(error) => { settle_prechild_cleanup(&ownership).await; return Err(OwnedAttachedSpawnError::BeforeSpawn(error)); }
    };
    if authorized.0.resolved_execution_policy.as_ref().is_some_and(|policy|
        policy.requested_ref == crate::sandbox::LOCAL_REQUESTED_EXECUTION_POLICY_REF)
        && ownership.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).baseline_bindings.is_none()
    {
        settle_prechild_cleanup(&ownership).await;
        return Err(OwnedAttachedSpawnError::BeforeSpawn(reject("local model requires captured adapter baseline handles".into())));
    }
    // Validate native String representation and exact environment before ACL,
    // profile, pipe, Job Object or process effects.
    if authorized.0.executable_path.to_str().is_none_or(|path| path.contains('\0'))
        || authorized.0.cwd.as_ref().is_some_and(|cwd| cwd.to_str().is_none_or(|path| path.contains('\0')))
        || authorized.0.args.iter().any(|arg| arg.contains('\0'))
        || authorized.0.binds.iter().any(|bind| bind.host_path.to_str().is_none_or(|path| path.contains('\0')))
    {
        settle_prechild_cleanup(&ownership).await;
        return Err(OwnedAttachedSpawnError::BeforeSpawn(reject("unsupported native path or argument encoding".to_string())));
    }
    if let Err(error) = exact_attached_env(&adapter_id, &authorized.0.env) {
        settle_prechild_cleanup(&ownership).await;
        return Err(OwnedAttachedSpawnError::BeforeSpawn(error));
    }
    let startup_deadline = std::time::Instant::now()
        .checked_add(Duration::from_millis(authorized.0.startup_timeout_ms))
        .map(|budget| budget.min(deadline)).unwrap_or(deadline);
    if authorized.0.startup_timeout_ms == 0 || cancel.is_cancelled()
        || std::time::Instant::now() >= startup_deadline
    {
        settle_prechild_cleanup(&ownership).await;
        return Err(OwnedAttachedSpawnError::BeforeSpawn(reject("startup cancelled or original deadline exhausted".to_string())));
    }
    let admission = tokio::select! {
        permit = ATTACHED_STARTUP_ADMISSION.acquire() => permit.ok(),
        _ = cancel.cancelled() => None,
        _ = tokio::time::sleep_until(tokio::time::Instant::from_std(startup_deadline)) => None,
    };
    let Some(admission) = admission else {
        settle_prechild_cleanup(&ownership).await;
        return Err(OwnedAttachedSpawnError::BeforeSpawn(reject("startup admission cancelled, closed or expired".to_string())));
    };
    let cancellation = Arc::new(StartupCancellation::with_deadline(startup_deadline));
    let worker_cancel = cancellation.clone();
    let worker_owner = ownership.clone();
    let worker_id = adapter_id.clone();
    let mut worker = tokio::task::spawn_blocking(move || {
        let _admission = admission;
        let mut state = worker_owner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let stage = AtomicU8::new(ATTACHED_STARTUP_STAGE_VALIDATION);
        spawn_windows_attached_process(worker_id, authorized, &stage, worker_cancel, &mut state)
    });
    let mut cancelled = false;
    let mut expired = false;
    let result = loop {
        tokio::select! {
            result = &mut worker => break match result {
                Ok(result) => result,
                Err(error) => Err(reject(format!("native startup worker failed: {error}"))),
            },
            _ = cancel.cancelled(), if !cancelled => {
                cancelled = true;
                cancellation.cancel_before_resume();
            },
            _ = tokio::time::sleep_until(tokio::time::Instant::from_std(startup_deadline)), if !expired => {
                expired = true;
                cancellation.cancel_before_resume();
            },
        }
    };
    // Startup is joined here, even after the original deadline/cancellation.
    let process = {
        let mut state = ownership.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.child.take().map(|child| {
            let receipt = crate::sandbox::adapter::AttachedSandboxReceipt {
                adapter_id: adapter_id.clone(), capabilities: adapter_capabilities,
                sandbox_internal_id: state.internal_id.take().expect("created child's real ACL instance exists"),
            };
            WindowsAttachedProcess {
                adapter_id: adapter_id.clone(), pid: child.pid, child: Some(child),
                acl_transaction: state.acl_transaction.take(),
                ephemeral_cleanup: state.ephemeral_cleanup.take().expect("startup cleanup guard retained"),
                observed_exit: None, receipt, confirmed_killed: false,
            }
        })
    };
    if let Some(process) = process {
        let failure = result.err().or_else(|| (cancelled || expired || cancellation.is_cancelled())
            .then(|| reject("startup completed after cancellation or original deadline".to_string())));
        let process: Box<dyn AttachedSandboxProcess> = Box::new(process);
        return match failure {
            Some(error) => Err(OwnedAttachedSpawnError::AfterSpawn { process, error }),
            None => Ok(process),
        };
    }
    settle_prechild_cleanup(&ownership).await;
    Err(OwnedAttachedSpawnError::BeforeSpawn(result.err()
        .unwrap_or_else(|| reject("native startup returned no child".to_string()))))
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
struct AuthorizedAttachedProcessSpec(AttachedProcessSpec);

#[cfg(target_os = "windows")]
fn authorize_attached_process_creation(
    adapter_id: &AdapterId,
    spec: AttachedProcessSpec,
    adapter_capabilities: &AdapterCapabilities,
) -> Result<AuthorizedAttachedProcessSpec, SandboxAdapterError> {
    let policy = spec.resolved_execution_policy.as_ref().ok_or_else(|| {
        attached_spawn_failed(
            adapter_id,
            "attached process creation requires typed resolved_execution_policy authority",
        )
    })?;
    let request = crate::sandbox::ExecutionPolicyRequest {
            requested_ref: policy.requested_ref.clone(),
            trust_class: policy.trust_class,
            isolation_tier: policy.isolation_tier,
            required_capabilities: policy.required_capabilities.clone(),
            requested_net_policy: policy.requested_net_policy.clone(),
            effective_attached_network_mode: policy.effective_attached_network_mode,
            resource_limits: policy.resource_limits.clone(),
            startup_timeout_ms: policy.startup_timeout_ms,
        };
    let canonical = match policy.requested_ref.as_str() {
        crate::sandbox::LOCAL_REQUESTED_EXECUTION_POLICY_REF =>
            crate::sandbox::ResolvedExecutionPolicy::resolve_local(request, adapter_capabilities),
        crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF =>
            crate::sandbox::ResolvedExecutionPolicy::resolve_official_cli(request),
        _ => Err(crate::sandbox::ExecutionPolicyError::UnknownReference(policy.requested_ref.clone())),
    }.map_err(|error| {
        attached_spawn_failed(
            adapter_id,
            format!("attached execution-policy boundary rejected launch: {error}"),
        )
    })?;
    if canonical != *policy {
        return Err(attached_spawn_failed(
            adapter_id,
            "attached execution-policy boundary rejected noncanonical typed authority",
        ));
    }
    policy
        .validate_attached_spec(&spec)
        .and_then(|_| policy.validate_adapter_capabilities(adapter_capabilities))
        .map_err(|error| {
            attached_spawn_failed(
                adapter_id,
                format!("attached execution-policy boundary rejected launch: {error}"),
            )
        })?;
    Ok(AuthorizedAttachedProcessSpec(spec))
}

#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_VALIDATION: u8 = 0;
#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_ACL_RECOVERY: u8 = 1;
#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_ACL_PREPARE: u8 = 3;
#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_PROFILE_CREATE: u8 = 4;
#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_ACL_GRANTS: u8 = 5;
#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_SECURITY_CAPABILITIES: u8 = 6;
#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_ENVIRONMENT: u8 = 7;
#[cfg(target_os = "windows")]
const ATTACHED_STARTUP_STAGE_PROCESS_LAUNCH: u8 = 8;

#[cfg(target_os = "windows")]
fn attached_startup_stage_name(stage: u8) -> &'static str {
    match stage {
        ATTACHED_STARTUP_STAGE_ACL_RECOVERY => "ACL recovery",
        ATTACHED_STARTUP_STAGE_ACL_PREPARE => "ACL transaction preparation",
        ATTACHED_STARTUP_STAGE_PROFILE_CREATE => "AppContainer profile creation",
        ATTACHED_STARTUP_STAGE_ACL_GRANTS => "ACL grant application",
        ATTACHED_STARTUP_STAGE_SECURITY_CAPABILITIES => "security capability construction",
        ATTACHED_STARTUP_STAGE_ENVIRONMENT => "environment construction",
        ATTACHED_STARTUP_STAGE_PROCESS_LAUNCH => "restricted AppContainer process launch",
        _ => "request validation",
    }
}

#[cfg(target_os = "windows")]
fn ensure_attached_startup_active(
    adapter_id: &AdapterId,
    cancellation: &StartupCancellation,
    boundary: &str,
) -> Result<(), SandboxAdapterError> {
    if cancellation.is_cancelled() {
        Err(attached_spawn_failed(
            adapter_id,
            format!("attached startup cancelled at {boundary}"),
        ))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn spawn_windows_attached_process(
    adapter_id: AdapterId,
    authorized_spec: AuthorizedAttachedProcessSpec,
    startup_stage: &AtomicU8,
    cancellation: Arc<StartupCancellation>,
    ownership: &mut AttachedStartupOwnership,
) -> Result<(), SandboxAdapterError> {
    let spec = authorized_spec.0;
    if spec.executable_path.as_os_str().is_empty() || !spec.executable_path.is_absolute() {
        return Err(attached_spawn_failed(
            &adapter_id,
            "attached executable path must be non-empty and absolute",
        ));
    }
    if !spec.executable_path.is_file() {
        return Err(attached_spawn_failed(
            &adapter_id,
            format!(
                "attached executable does not exist: {}",
                spec.executable_path.display()
            ),
        ));
    }
    if spec.execution_policy_ref.trim().is_empty() {
        return Err(attached_spawn_failed(
            &adapter_id,
            "attached execution_policy_ref must be non-empty",
        ));
    }
    if spec.requested_isolation_tier != crate::sandbox::IsolationTier::Tier1Container {
        return Err(attached_spawn_failed(
            &adapter_id,
            format!(
                "Handshake-native attached execution enforces Tier1Container, requested {:?}",
                spec.requested_isolation_tier
            ),
        ));
    }
    if spec.trust_class.min_isolation_tier().rank()
        > crate::sandbox::IsolationTier::Tier1Container.rank()
    {
        return Err(attached_spawn_failed(
            &adapter_id,
            format!(
                "trust class {:?} requires {:?}, stronger than the Windows Tier-1 process jail",
                spec.trust_class,
                spec.trust_class.min_isolation_tier()
            ),
        ));
    }
    validate_attached_network_contract(&adapter_id, spec.network_mode, &spec.requested_net_policy)?;
    validate_supported_resource_limits(&spec.resource_limits)
        .map_err(|error| attached_spawn_failed(&adapter_id, error))?;
    validate_bind_hosts(&spec.binds).map_err(|error| attached_spawn_failed(&adapter_id, error))?;
    validate_attached_required_capabilities(&spec.required_capabilities)?;
    if spec
        .binds
        .iter()
        .any(|bind| bind.host_path == spec.executable_path && matches!(bind.mode, BindMode::NoExec))
    {
        return Err(attached_spawn_failed(
            &adapter_id,
            "attached executable cannot also carry a NoExec bind grant",
        ));
    }
    if let Some(cwd) = spec.cwd.as_ref() {
        let cwd_is_granted = spec.binds.iter().any(|bind| bind.host_path == *cwd);
        if !cwd_is_granted {
            return Err(attached_spawn_failed(
                &adapter_id,
                format!(
                    "attached cwd requires an explicit bind grant: {}",
                    cwd.display()
                ),
            ));
        }
    }

    if let Some(bindings) = ownership.baseline_bindings.as_ref() {
        for path in spec.binds.iter().map(|bind| bind.host_path.as_path())
            .chain(std::iter::once(spec.executable_path.as_path()))
            .chain(spec.cwd.as_deref())
        {
            super::acl_transaction::validate_baseline_path(bindings, path)
                .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?;
        }
    }

    startup_stage.store(ATTACHED_STARTUP_STAGE_ACL_RECOVERY, Ordering::Release);
    ensure_default_acl_recovery().map_err(|error| {
        attached_spawn_failed(
            &adapter_id,
            format!("AppContainer ACL recovery failed before attached launch: {error}"),
        )
    })?;
    ensure_attached_startup_active(&adapter_id, &cancellation, "after ACL recovery")?;
    let internal_id = format!(
        "handshake.native.attached.{}",
        uuid::Uuid::now_v7().simple()
    );
    startup_stage.store(ATTACHED_STARTUP_STAGE_ACL_PREPARE, Ordering::Release);
    ownership.acl_transaction = Some(AppContainerAclTransaction::begin(
        internal_id.clone(),
        acl_grant_targets(&spec.binds, Some(&spec.executable_path))
            .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?,
    )
    .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?);
    ownership.internal_id = Some(internal_id);
    let acl_transaction = ownership.acl_transaction.as_mut().expect("ACL transaction just installed");
    acl_transaction.bind_baseline(ownership.baseline_bindings.clone())
        .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?;
    ensure_attached_startup_active(
        &adapter_id,
        &cancellation,
        "before AppContainer profile creation",
    )?;
    startup_stage.store(ATTACHED_STARTUP_STAGE_PROFILE_CREATE, Ordering::Release);
    acl_transaction
        .create_profile(
            "Handshake Native Attached Process",
            Some("Handshake Tier-1 attached process jail"),
        )
        .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?;
    ensure_attached_startup_active(
        &adapter_id,
        &cancellation,
        "after AppContainer profile creation",
    )?;
    startup_stage.store(ATTACHED_STARTUP_STAGE_ACL_GRANTS, Ordering::Release);
    acl_transaction
        .grant_all_cancellable(cancellation.as_ref())
        .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?;

    ensure_attached_startup_active(&adapter_id, &cancellation, "after ACL grants")?;

    startup_stage.store(
        ATTACHED_STARTUP_STAGE_SECURITY_CAPABILITIES,
        Ordering::Release,
    );
    let mut builder = rappct::SecurityCapabilitiesBuilder::new(
        &acl_transaction
            .profile()
            .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?
            .sid,
    );
    if rappct::supports_lpac().is_ok() {
        builder = builder.with_lpac_defaults().lpac(true);
    }
    if spec.network_mode == AttachedNetworkMode::OutboundInternetClient {
        builder = builder.with_known(&[rappct::KnownCapability::InternetClient]);
    }
    let security = builder
        .build()
        .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?;
    ensure_attached_startup_active(
        &adapter_id,
        &cancellation,
        "after security capability construction",
    )?;
    startup_stage.store(ATTACHED_STARTUP_STAGE_ENVIRONMENT, Ordering::Release);
    let env = Some(exact_attached_env(&adapter_id, &spec.env)?);
    ensure_attached_startup_active(
        &adapter_id,
        &cancellation,
        "before restricted AppContainer process creation",
    )?;
    startup_stage.store(ATTACHED_STARTUP_STAGE_PROCESS_LAUNCH, Ordering::Release);
    let result = super::restricted_appcontainer::launch_restricted_appcontainer_into(
        &security,
        WindowsNativeLaunchOptions {
            exe: spec.executable_path,
            args: spec.args,
            cwd: spec.cwd,
            env,
            job_limits: job_limits(&spec.resource_limits),
            startup_timeout: Some(Duration::from_millis(spec.startup_timeout_ms)),
            startup_cancellation: Some(cancellation),
        },
        &mut ownership.child,
    );
    if let Some(child) = ownership.child.as_mut() { child.stdin.take(); }
    result.map_err(|reason| attached_spawn_failed(&adapter_id, reason))
}

#[cfg(target_os = "windows")]
fn exact_attached_env(
    adapter_id: &AdapterId,
    env: &BTreeMap<String, String>,
) -> Result<Vec<(OsString, OsString)>, SandboxAdapterError> {
    let mut normalized_names = BTreeSet::new();
    let mut exact = Vec::with_capacity(env.len());
    for (key, value) in env {
        if key.is_empty() || key.contains('=') || key.contains('\0') || value.contains('\0') {
            return Err(attached_spawn_failed(
                adapter_id,
                format!("invalid attached environment entry {key:?}"),
            ));
        }
        let normalized = key.to_ascii_lowercase();
        if !normalized_names.insert(normalized) {
            return Err(attached_spawn_failed(
                adapter_id,
                format!("duplicate case-insensitive attached environment key {key:?}"),
            ));
        }
        exact.push((OsString::from(key), OsString::from(value)));
    }
    for key in ["SystemDrive", "SystemRoot", "TEMP", "TMP", "LOCALAPPDATA"] {
        if normalized_names.contains(&key.to_ascii_lowercase()) {
            continue;
        }
        let value = std::env::var_os(key).ok_or_else(|| {
            attached_spawn_failed(
                adapter_id,
                format!(
                    "host {key} is unavailable for the required Windows process bootstrap environment"
                ),
            )
        })?;
        exact.push((OsString::from(key), value));
    }
    exact.sort_by(|left, right| {
        left.0
            .to_string_lossy()
            .to_ascii_lowercase()
            .cmp(&right.0.to_string_lossy().to_ascii_lowercase())
    });
    Ok(exact)
}

#[cfg(target_os = "windows")]
fn validate_attached_network_contract(
    adapter_id: &AdapterId,
    mode: AttachedNetworkMode,
    requested: &NetPolicy,
) -> Result<(), SandboxAdapterError> {
    match (mode, requested) {
        (AttachedNetworkMode::DenyAll, NetPolicy::DenyAll)
        | (AttachedNetworkMode::OutboundInternetClient, NetPolicy::HostInherited) => Ok(()),
        _ => Err(SandboxAdapterError::NetPolicyApplyFailed {
            adapter_id: adapter_id.clone(),
            reason: format!(
                "attached network mode {mode:?} cannot truthfully satisfy requested policy {requested:?}"
            ),
        }),
    }
}

#[cfg(target_os = "windows")]
fn validate_attached_required_capabilities(
    required: &BTreeSet<crate::sandbox::RequiredCapability>,
) -> Result<(), SandboxAdapterError> {
    use crate::sandbox::RequiredCapability;

    let capabilities = windows_native_jail_target_capabilities();
    let mut available = BTreeSet::new();
    // The attached launcher owns piped stdout/stderr and drains them on
    // dedicated readers. This is stronger than the detached jail's generic
    // medium-throughput handle contract.
    available.insert(RequiredCapability::HighStdioThroughput);
    if capabilities.win32_native_fidelity {
        available.insert(RequiredCapability::Win32NativeFidelity);
    }
    if capabilities.cross_machine_portable {
        available.insert(RequiredCapability::CrossMachinePortable);
    }
    if capabilities.filesystem_isolation_strength == crate::sandbox::IsolationStrength::VeryStrong {
        available.insert(RequiredCapability::VeryStrongFilesystemIsolation);
    }
    if capabilities.network_isolation_strength == crate::sandbox::IsolationStrength::VeryStrong {
        available.insert(RequiredCapability::VeryStrongNetworkIsolation);
    }
    match capabilities.gpu_passthrough {
        crate::sandbox::GpuPassthrough::NvidiaCuda => {
            available.insert(RequiredCapability::NvidiaCudaPassthrough);
        }
        crate::sandbox::GpuPassthrough::VendorAgnostic => {
            available.insert(RequiredCapability::VendorAgnosticGpu);
        }
        crate::sandbox::GpuPassthrough::None => {}
    }

    let missing = required
        .difference(&available)
        .copied()
        .collect::<BTreeSet<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SandboxAdapterError::CapabilityUnsatisfied {
            required: missing,
            available,
        })
    }
}

#[cfg(target_os = "windows")]
fn attached_spawn_failed(adapter_id: &AdapterId, reason: impl ToString) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: adapter_id.clone(),
        reason: reason.to_string(),
    }
}

#[cfg(target_os = "windows")]
fn acl_grant_targets(
    binds: &[BindSpec],
    executable: Option<&std::path::Path>,
) -> Result<Vec<AclGrantTarget>, SandboxAdapterError> {
    let mut targets = BTreeMap::<PathBuf, (AclGrantTargetKind, u32)>::new();
    for bind in binds {
        let canonical_path = std::fs::canonicalize(&bind.host_path).map_err(|error| {
            spawn_failed(format!(
                "canonicalize AppContainer ACL target {}: {error}",
                bind.host_path.display()
            ))
        })?;
        let canonical_path = windows_named_security_path(canonical_path);
        if appcontainer_path_is_ambient(&canonical_path) {
            continue;
        }
        let kind = if canonical_path.is_dir() {
            AclGrantTargetKind::Directory
        } else {
            AclGrantTargetKind::File
        };
        let access_mask = match bind.mode {
            BindMode::NoExec => rappct::acl::AccessMask::FILE_GENERIC_READ.0,
            BindMode::ReadOnly => {
                rappct::acl::AccessMask::FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE_MASK
            }
            BindMode::ReadWrite => {
                rappct::acl::AccessMask::FILE_GENERIC_READ.0
                    | rappct::acl::AccessMask::FILE_GENERIC_WRITE.0
                    | FILE_GENERIC_EXECUTE_MASK
            }
        };
        targets
            .entry(canonical_path)
            .and_modify(|(_, existing)| *existing |= access_mask)
            .or_insert((kind, access_mask));
    }
    if let Some(executable) = executable {
        let canonical_path = std::fs::canonicalize(executable).map_err(|error| {
            spawn_failed(format!(
                "canonicalize attached executable ACL target {}: {error}",
                executable.display()
            ))
        })?;
        let canonical_path = windows_named_security_path(canonical_path);
        if !appcontainer_path_is_ambient(&canonical_path) {
            let access_mask =
                rappct::acl::AccessMask::FILE_GENERIC_READ.0 | FILE_GENERIC_EXECUTE_MASK;
            targets
                .entry(canonical_path)
                .and_modify(|(_, existing)| *existing |= access_mask)
                .or_insert((AclGrantTargetKind::File, access_mask));
        }
    }
    Ok(targets
        .into_iter()
        .map(|(path, (kind, access_mask))| match kind {
            AclGrantTargetKind::File => AclGrantTarget::file(path, access_mask),
            AclGrantTargetKind::Directory => AclGrantTarget::directory(path, access_mask),
        })
        .collect())
}

#[cfg(target_os = "windows")]
pub(super) fn windows_named_security_path(path: PathBuf) -> PathBuf {
    use std::path::{Component, Prefix};

    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return path;
    };
    let mut normalized = match prefix.kind() {
        Prefix::VerbatimDisk(letter) => PathBuf::from(format!("{}:\\", letter as char)),
        Prefix::VerbatimUNC(server, share) => {
            let mut root = PathBuf::from(r"\\");
            root.push(server);
            root.push(share);
            root
        }
        _ => return path,
    };
    for component in components {
        if !matches!(component, Component::RootDir) {
            normalized.push(component.as_os_str());
        }
    }
    normalized
}

#[cfg(target_os = "windows")]
fn appcontainer_path_is_ambient(path: &std::path::Path) -> bool {
    ["SystemRoot", "ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(std::env::var_os)
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .map(windows_named_security_path)
        .any(|root| path.starts_with(root))
}

fn validate_supported_resource_limits(limits: &ResourceLimits) -> Result<(), SandboxAdapterError> {
    if limits.disk_read_bytes_per_sec.is_some()
        || limits.disk_write_bytes_per_sec.is_some()
        || limits.net_bandwidth_bytes_per_sec.is_some()
    {
        return Err(spawn_failed(
            "WindowsNativeJailAdapter ResourceLimits disk/net bytes-per-second token-bucket limits \
             are not enforceable by this adapter path yet; refusing to silently ignore requested \
             per-device rate limits",
        ));
    }
    Ok(())
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
    mut child: WindowsNativeLaunchedIo,
    profile: rappct::AppContainerProfile,
    exit_code: Arc<Mutex<Option<i32>>>,
    job_guard: Arc<Mutex<Option<WindowsNativeJobGuard>>>,
) {
    thread::spawn(move || {
        let mut observed_exit = None;
        let mut profile_deleted = false;
        loop {
            if observed_exit.is_none() {
                match child.wait_root_borrowed(Some(ATTACHED_TERMINATION_REAP_TIMEOUT)) {
                    Ok(code) => observed_exit = Some(code as i32),
                    Err(error) => {
                        tracing::error!(%error, "detached root reap unconfirmed; ownership retained");
                        thread::sleep(Duration::from_millis(25));
                        continue;
                    }
                }
            }
            let mut guard = match job_guard.lock() {
                Ok(guard) => guard,
                Err(error) => {
                    tracing::error!(%error, "detached shared job lock unavailable; ownership retained");
                    thread::sleep(Duration::from_millis(25));
                    continue;
                }
            };
            let tree_empty = match guard.as_ref() {
                Some(job) => match job.active_processes() {
                    Ok(active) => active == 0,
                    Err(error) => {
                        tracing::error!(%error, "detached Job Object accounting unconfirmed; ownership retained");
                        false
                    }
                },
                None => {
                    tracing::error!("detached shared Job Object missing before terminal acknowledgement; ownership retained");
                    false
                }
            };
            if !tree_empty {
                drop(guard);
                thread::sleep(Duration::from_millis(25));
                continue;
            }
            if !profile_deleted {
                // Keep the original profile/SID until OS deletion is confirmed.
                // Derivation opens no new profile and does not recreate it.
                let deletion = rappct::derive_sid_from_name(&profile.name)
                    .map_err(|error| error.to_string())
                    .and_then(|sid| delete_appcontainer_profile(rappct::AppContainerProfile {
                        name: profile.name.clone(), sid,
                    }));
                if let Err(error) = deletion {
                    drop(guard);
                    tracing::error!(%error, "detached profile cleanup unconfirmed; ownership retained");
                    thread::sleep(Duration::from_millis(25));
                    continue;
                }
                profile_deleted = true;
            }
            match exit_code.lock() {
                Ok(mut slot) => {
                    // Publish only the real root exit after empty-tree and
                    // profile cleanup ACK. Kill/timeout use this same job lock.
                    *slot = observed_exit;
                    guard.take();
                    return;
                }
                Err(error) => {
                    tracing::error!(%error, "detached terminal publication unavailable; ownership retained");
                    drop(guard);
                    thread::sleep(Duration::from_millis(25));
                }
            }
        }
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
            if let Ok(guard) = job_guard.lock() {
                if let Some(job) = guard.as_ref() {
                    match job.terminate(1) {
                        Ok(()) => {
                            if let Ok(mut signal) = killed_by.lock() {
                                *signal = Some(Signal::Kill);
                            }
                        }
                        Err(error) => tracing::error!(%error, "detached timeout termination unconfirmed; shared job retained"),
                    }
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
