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
        acl_grant_stage_detail, default_acl_recovery_stage_name, delete_appcontainer_profile,
        ensure_appcontainer_profile, ensure_default_acl_recovery, AclGrantTarget,
        AclGrantTargetKind, AppContainerAclTransaction,
    },
    job_object_wrap::{WindowsNativeJobGuard, WindowsNativeJobLimits},
    restricted_appcontainer::{
        launch_restricted_appcontainer_with_io, StartupCancellation, WindowsNativeLaunchOptions,
        WindowsNativeLaunchedIo,
    },
};
use super::{
    windows_native_jail_runtime_capabilities, windows_native_jail_target_capabilities,
    windows_native_jail_unavailable_capabilities,
};
use crate::sandbox::{
    AdapterCapabilities, AdapterId, AttachedEphemeralPathGuard, AttachedNetworkMode,
    AttachedProcessSpec, AttachedSandboxProcess, BindMode, BindSpec, Command, ExecResult,
    NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus, ResourceLimits, SandboxAdapter,
    SandboxAdapterError, Signal, WINDOWS_NATIVE_JAIL_ADAPTER_ID,
    WINDOWS_NATIVE_JAIL_BACKEND_APPROVED,
};

const FILE_GENERIC_EXECUTE_MASK: u32 = 1_179_808;
#[cfg(target_os = "windows")]
static ATTACHED_STARTUP_ADMISSION: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
#[cfg(target_os = "windows")]
const ATTACHED_TERMINATION_REAP_TIMEOUT: Duration = Duration::from_secs(5);

const BACKEND_NOT_APPROVED_REASON: &str = concat!(
    "WindowsNativeJailAdapter unavailable: this build was not produced on a Windows host with the ",
    "`win-native-integration` cargo feature. MT-045 approved rappct 0.13.3 as the AppContainer ",
    "substrate and MT-046 composes the Restricted Token + Job Object bridge on top, but neither is ",
    "active in the current build. Rebuild on Windows with `--features win-native-integration` to ",
    "enable the runtime backend."
);

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
            ensure_default_acl_recovery().map_err(|error| {
                unavailable_error(format!(
                    "AppContainer ACL recovery failed before native jail initialization: {error}"
                ))
            })?;
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
    let mut acl_transaction = AppContainerAclTransaction::begin(
        internal_id.clone(),
        acl_grant_targets(&spec.binds, None)?,
    )
    .map_err(|error| spawn_failed(format!("prepare AppContainer ACL transaction: {error}")))?;
    acl_transaction
        .create_profile(
            "Handshake Windows Native Jail",
            Some("Handshake MT-046 WindowsNativeJailAdapter"),
        )
        .map_err(|error| {
            spawn_failed(format!(
                "rappct AppContainer profile creation failed: {error}"
            ))
        })?;
    acl_transaction
        .grant_all()
        .map_err(|error| spawn_failed(format!("AppContainer ACL transaction failed: {error}")))?;

    let mut builder = rappct::SecurityCapabilitiesBuilder::new(
        &acl_transaction
            .profile()
            .map_err(|error| spawn_failed(error))?
            .sid,
    );
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

    spawn_waiter(child, acl_transaction, exit_code.clone());
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
        let acl_result = if let Some(mut transaction) = self.acl_transaction.take() {
            transaction
                .cleanup()
                .map_err(|reason| self.error("AppContainer ACL cleanup", reason))
        } else {
            Ok(())
        };
        let ephemeral_result = self
            .ephemeral_cleanup
            .cleanup()
            .map_err(|reason| self.error("ephemeral data-root cleanup", reason));
        acl_result?;
        ephemeral_result
    }
}

#[cfg(target_os = "windows")]
impl AttachedSandboxProcess for WindowsAttachedProcess {
    fn pid(&self) -> u32 {
        self.pid
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
        if self.try_wait()?.is_some() {
            return self.wait();
        }
        let adapter_id = self.adapter_id.clone();
        let child = match self.child.as_mut() {
            Some(child) => child,
            None => return Err(self.error("terminate", "already reaped")),
        };
        let terminate_result = child
            .job_guard
            .as_ref()
            .ok_or_else(|| SandboxAdapterError::SpawnFailed {
                adapter_id: adapter_id.clone(),
                reason: "attached process lost its creation-time Job Object".to_string(),
            })?
            .terminate(1)
            .map_err(|reason| SandboxAdapterError::SpawnFailed {
                adapter_id,
                reason: format!("attached process TerminateJobObject: {reason}"),
            });
        // Cleanup is itself part of timeout/cancellation/unwind recovery. Never
        // turn a failed or ineffective TerminateJobObject into an unbounded
        // WaitForSingleObject(INFINITE). Consuming the child closes its process
        // and Job Object handles; a timeout remains an explicit unreaped error,
        // so GuardedCliChild leaves the ProcessOwnershipLedger START open.
        let wait_result = self.wait_with_timeout(Some(ATTACHED_TERMINATION_REAP_TIMEOUT));
        terminate_result?;
        wait_result
    }
}

#[cfg(target_os = "windows")]
impl WindowsAttachedProcess {
    fn wait_with_timeout(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<ExitStatus, SandboxAdapterError> {
        let child = self
            .child
            .take()
            .ok_or_else(|| self.error("wait", "already reaped"))?;
        let result = child
            .wait(timeout)
            .map(windows_exit_status)
            .map_err(|reason| self.error("wait", reason));
        let cleanup_result = self.finish_acl_cleanup();
        if let Ok(status) = result {
            self.observed_exit = Some(status);
        }
        let status = result?;
        cleanup_result?;
        Ok(status)
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsAttachedProcess {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.terminate_tree_and_wait();
        }
        let _ = self.finish_acl_cleanup();
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
/// the deadline, the worker observes cancellation and reclaims any late child.
#[cfg(target_os = "windows")]
pub(crate) async fn spawn_handshake_native_attached(
    adapter_id: AdapterId,
    spec: AttachedProcessSpec,
    adapter_capabilities: AdapterCapabilities,
    ephemeral_cleanup: AttachedEphemeralPathGuard,
) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
    let authorized_spec =
        authorize_attached_process_creation(&adapter_id, spec, &adapter_capabilities)?;
    let timeout_ms = authorized_spec.0.startup_timeout_ms;
    if timeout_ms == 0 {
        return Err(SandboxAdapterError::SpawnFailed {
            adapter_id,
            reason: "attached startup_timeout_ms must be greater than zero".to_string(),
        });
    }
    let startup_budget = Duration::from_millis(timeout_ms);
    let startup_started = std::time::Instant::now();
    let admission = match tokio::time::timeout(startup_budget, ATTACHED_STARTUP_ADMISSION.acquire())
        .await
    {
        Ok(Ok(admission)) => admission,
        Ok(Err(_)) => {
            return Err(SandboxAdapterError::SpawnFailed {
                adapter_id,
                reason: "Handshake-native attached startup admission is closed".to_string(),
            });
        }
        Err(_) => {
            return Err(SandboxAdapterError::SpawnFailed {
                adapter_id,
                reason: format!(
                    "attached process startup exceeded its {timeout_ms} ms deadline while waiting for bounded Handshake-native startup admission"
                ),
            });
        }
    };
    let remaining_startup_budget = startup_budget.saturating_sub(startup_started.elapsed());
    if remaining_startup_budget.is_zero() {
        return Err(SandboxAdapterError::SpawnFailed {
            adapter_id,
            reason: format!(
                "attached process startup exhausted its {timeout_ms} ms deadline while waiting for bounded Handshake-native startup admission"
            ),
        });
    }
    // Move the permit into the blocking closure so a dropped/cancelled async
    // caller cannot admit another AppContainer startup until the current Win32
    // worker has actually finished or reclaimed its late child.
    let cancellation = Arc::new(StartupCancellation::default());
    let startup_stage = Arc::new(AtomicU8::new(ATTACHED_STARTUP_STAGE_VALIDATION));
    let worker_cancellation = cancellation.clone();
    let worker_startup_stage = startup_stage.clone();
    let worker_adapter_id = adapter_id.clone();
    let mut worker = tokio::task::spawn_blocking(move || {
        let _admission = admission;
        let mut result = spawn_windows_attached_process(
            worker_adapter_id.clone(),
            authorized_spec,
            worker_startup_stage.as_ref(),
            worker_cancellation.clone(),
            ephemeral_cleanup,
        );
        if worker_cancellation.is_cancelled() {
            if let Ok(process) = result.as_mut() {
                process.terminate_tree_and_wait().map_err(|cleanup_error| {
                    SandboxAdapterError::SpawnFailed {
                        adapter_id: worker_adapter_id.clone(),
                        reason: format!(
                            "attached process creation completed after its startup deadline, but deterministic terminate/reap failed: {cleanup_error}"
                        ),
                    }
                })?;
            }
            return Err(SandboxAdapterError::SpawnFailed {
                adapter_id: worker_adapter_id,
                reason: "attached process creation completed after its startup deadline and was reclaimed"
                    .to_string(),
            });
        }
        result
    });

    match tokio::time::timeout(remaining_startup_budget, &mut worker).await {
        Ok(Ok(result)) => result,
        Ok(Err(join_error)) => Err(SandboxAdapterError::SpawnFailed {
            adapter_id,
            reason: format!("attached process creation worker failed: {join_error}"),
        }),
        Err(_) => {
            let cancelled_before_resume = cancellation.cancel_before_resume();
            if !cancelled_before_resume {
                // Resume already crossed the side-effect boundary. Do not
                // return merely because cancellation was requested: wait a
                // second, bounded interval for the worker to prove terminate
                // and reap, and propagate its cleanup result.
                const POST_RESUME_CLEANUP_TIMEOUT: Duration = Duration::from_secs(7);
                return match tokio::time::timeout(POST_RESUME_CLEANUP_TIMEOUT, &mut worker).await {
                    Ok(Ok(Err(cleanup_result))) => Err(cleanup_result),
                    Ok(Ok(Ok(mut late_process))) => {
                        late_process.terminate_tree_and_wait()?;
                        Err(SandboxAdapterError::SpawnFailed {
                            adapter_id,
                            reason: format!(
                                "attached process creation exceeded startup deadline of {timeout_ms} ms after suspended-process resume; late child was deterministically terminated and reaped"
                            ),
                        })
                    }
                    Ok(Err(join_error)) => Err(SandboxAdapterError::SpawnFailed {
                        adapter_id,
                        reason: format!(
                            "attached process creation exceeded startup deadline after resume and cleanup worker failed: {join_error}"
                        ),
                    }),
                    Err(_) => Err(SandboxAdapterError::SpawnFailed {
                        adapter_id,
                        reason: format!(
                            "attached process creation exceeded startup deadline after resume and deterministic cleanup did not complete within {} ms",
                            POST_RESUME_CLEANUP_TIMEOUT.as_millis()
                        ),
                    }),
                };
            }
            let stage_id = startup_stage.load(Ordering::Acquire);
            let stage = if stage_id == ATTACHED_STARTUP_STAGE_ACL_RECOVERY {
                format!("ACL recovery ({})", default_acl_recovery_stage_name())
            } else if stage_id == ATTACHED_STARTUP_STAGE_ACL_GRANTS {
                format!("ACL grant application ({})", acl_grant_stage_detail())
            } else {
                attached_startup_stage_name(stage_id).to_string()
            };
            Err(SandboxAdapterError::SpawnFailed {
                adapter_id,
                reason: format!(
                    "attached process creation exceeded startup deadline of {timeout_ms} ms during {stage}"
                ),
            })
        }
    }
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
    let canonical = crate::sandbox::ResolvedExecutionPolicy::resolve_official_cli(
        crate::sandbox::ExecutionPolicyRequest {
            requested_ref: policy.requested_ref.clone(),
            trust_class: policy.trust_class,
            isolation_tier: policy.isolation_tier,
            required_capabilities: policy.required_capabilities.clone(),
            requested_net_policy: policy.requested_net_policy.clone(),
            effective_attached_network_mode: policy.effective_attached_network_mode,
            resource_limits: policy.resource_limits.clone(),
            startup_timeout_ms: policy.startup_timeout_ms,
        },
    )
    .map_err(|error| {
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
    ephemeral_cleanup: AttachedEphemeralPathGuard,
) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
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
    let mut acl_transaction = AppContainerAclTransaction::begin(
        internal_id.clone(),
        acl_grant_targets(&spec.binds, Some(&spec.executable_path))
            .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?,
    )
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
    let mut child = launch_restricted_appcontainer_with_io(
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
    )
    .map_err(|reason| attached_spawn_failed(&adapter_id, reason))?;
    drop(child.stdin.take());
    let pid = child.pid;
    Ok(Box::new(WindowsAttachedProcess {
        adapter_id,
        pid,
        child: Some(child),
        acl_transaction: Some(acl_transaction),
        ephemeral_cleanup,
        observed_exit: None,
    }))
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
fn windows_named_security_path(path: PathBuf) -> PathBuf {
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
    mut acl_transaction: AppContainerAclTransaction,
    exit_code: Arc<Mutex<Option<i32>>>,
) {
    thread::spawn(move || {
        let code = child.wait(None).map(|code| code as i32).unwrap_or(1);
        if let Ok(mut slot) = exit_code.lock() {
            *slot = Some(code);
        }
        if let Err(error) = acl_transaction.cleanup() {
            tracing::error!(
                target: "handshake_core::windows_native_jail",
                error = %error,
                "detached AppContainer ACL cleanup incomplete; recovery journal retained"
            );
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

#[cfg(all(test, target_os = "windows"))]
mod attached_contract_tests {
    use super::*;

    fn canonical_attached_spec() -> AttachedProcessSpec {
        let resource_limits = ResourceLimits {
            timeout_ms: Some(1_000),
            ..ResourceLimits::default()
        };
        let policy = crate::sandbox::ResolvedExecutionPolicy::resolve_official_cli(
            crate::sandbox::ExecutionPolicyRequest {
                requested_ref: crate::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF
                    .to_string(),
                trust_class: crate::sandbox::TrustClass::Trusted,
                isolation_tier: crate::sandbox::IsolationTier::Tier1Container,
                required_capabilities: BTreeSet::from([
                    crate::sandbox::RequiredCapability::HighStdioThroughput,
                ]),
                requested_net_policy: NetPolicy::HostInherited,
                effective_attached_network_mode: AttachedNetworkMode::OutboundInternetClient,
                resource_limits: resource_limits.clone(),
                startup_timeout_ms: 1_000,
            },
        )
        .expect("canonical Official-CLI policy");
        AttachedProcessSpec {
            executable_path: PathBuf::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            binds: Vec::new(),
            network_mode: policy.effective_attached_network_mode,
            trust_class: policy.trust_class,
            required_capabilities: policy.required_capabilities.clone(),
            requested_isolation_tier: policy.isolation_tier,
            requested_net_policy: policy.requested_net_policy.clone(),
            resource_limits,
            startup_timeout_ms: policy.startup_timeout_ms,
            ephemeral_cleanup_paths: Vec::new(),
            execution_policy_ref: policy.effective_ref.clone(),
            resolved_execution_policy: Some(policy),
            swarm_id: None,
            worktree_id: None,
            checkout_lease_id: None,
            checkout_lease_owner_generation: None,
            checkout_lease_canonical_working_dir: None,
        }
    }

    #[test]
    fn forged_or_absent_policy_cannot_reach_process_creation_type_boundary() {
        let adapter_id = AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
        let capabilities = crate::sandbox::HandshakeNativeSandboxAdapter::new().capabilities();

        let mut absent = canonical_attached_spec();
        absent.execution_policy_ref = "execution-policy://forged/noncanonical".to_string();
        absent.resolved_execution_policy = None;
        let absent_error = authorize_attached_process_creation(&adapter_id, absent, &capabilities)
            .expect_err("untyped authority must fail before an authorized spawn input exists");
        assert!(absent_error.to_string().contains("requires typed"));

        let mut mismatched = canonical_attached_spec();
        mismatched.execution_policy_ref = "execution-policy://forged/noncanonical".to_string();
        let mismatch_error =
            authorize_attached_process_creation(&adapter_id, mismatched, &capabilities)
                .expect_err("free-form reference drift must fail before process creation");
        assert!(mismatch_error.to_string().contains("effective_ref"));
    }

    #[test]
    fn mismatched_adapter_capabilities_cannot_reach_process_creation_type_boundary() {
        let adapter_id = AdapterId::new(crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID);
        let mut capabilities = crate::sandbox::HandshakeNativeSandboxAdapter::new().capabilities();
        capabilities.stdio_throughput_class = crate::sandbox::ThroughputClass::Medium;

        let error = authorize_attached_process_creation(
            &adapter_id,
            canonical_attached_spec(),
            &capabilities,
        )
        .expect_err("capability mismatch must fail before an authorized spawn input exists");
        assert!(error.to_string().contains("stdio_throughput_class"));
    }

    #[tokio::test]
    async fn default_attached_methods_reclaim_ephemeral_path_on_unsupported_spawn() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_nanos();
        let ephemeral_root =
            std::env::temp_dir().join(format!("handshake-default-attached-cleanup-{unique}"));
        std::fs::create_dir_all(&ephemeral_root).expect("create default-method ownership root");
        std::fs::write(ephemeral_root.join("owned.txt"), b"owned")
            .expect("populate default-method ownership root");
        let spec = AttachedProcessSpec {
            executable_path: PathBuf::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            binds: Vec::new(),
            network_mode: AttachedNetworkMode::DenyAll,
            trust_class: crate::sandbox::TrustClass::Reviewed,
            required_capabilities: BTreeSet::new(),
            requested_isolation_tier: crate::sandbox::IsolationTier::Tier1Container,
            requested_net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            startup_timeout_ms: 1_000,
            ephemeral_cleanup_paths: vec![ephemeral_root.clone()],
            execution_policy_ref: "execution-policy://test/default-attached-cleanup".to_string(),
            resolved_execution_policy: None,
            swarm_id: None,
            worktree_id: None,
            checkout_lease_id: None,
            checkout_lease_owner_generation: None,
            checkout_lease_canonical_working_dir: None,
        };

        let result = WindowsNativeJailAdapter::unavailable_for_current_host()
            .spawn_attached_with_stdio(
                spec,
                crate::sandbox::AttachedStdioContract::null_stdin_piped_output(),
            )
            .await;

        assert!(
            result.is_err(),
            "default attached spawn must be unsupported"
        );
        assert!(
            !ephemeral_root.exists(),
            "valid stdio followed by default unsupported spawn must reclaim ownership root"
        );
    }

    #[tokio::test]
    async fn invalid_untyped_policy_reclaims_ephemeral_path_before_worker_admission() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_nanos();
        let ephemeral_root =
            std::env::temp_dir().join(format!("handshake-attached-cleanup-{unique}"));
        std::fs::create_dir_all(&ephemeral_root).expect("create ephemeral startup root");
        std::fs::write(ephemeral_root.join("owned.txt"), b"owned")
            .expect("populate ephemeral startup root");

        let mut spec = AttachedProcessSpec {
            executable_path: PathBuf::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            binds: Vec::new(),
            network_mode: AttachedNetworkMode::DenyAll,
            trust_class: crate::sandbox::TrustClass::Reviewed,
            required_capabilities: BTreeSet::new(),
            requested_isolation_tier: crate::sandbox::IsolationTier::Tier1Container,
            requested_net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            startup_timeout_ms: 0,
            ephemeral_cleanup_paths: vec![ephemeral_root.clone()],
            execution_policy_ref: "execution-policy://test/cleanup".to_string(),
            resolved_execution_policy: None,
            swarm_id: None,
            worktree_id: None,
            checkout_lease_id: None,
            checkout_lease_owner_generation: None,
            checkout_lease_canonical_working_dir: None,
        };
        let ephemeral_cleanup = AttachedEphemeralPathGuard::take_from(&mut spec);
        let result = spawn_handshake_native_attached(
            AdapterId::new("test"),
            spec,
            crate::sandbox::HandshakeNativeSandboxAdapter::new().capabilities(),
            ephemeral_cleanup,
        )
        .await;

        match result {
            Err(SandboxAdapterError::SpawnFailed { adapter_id, reason }) => {
                assert_eq!(adapter_id, AdapterId::new("test"));
                assert!(reason.contains("requires typed"));
            }
            Err(other) => panic!("untyped execution policy returned the wrong error: {other}"),
            Ok(process) => {
                drop(process);
                panic!("untyped execution policy must be rejected");
            }
        }
        assert!(
            !ephemeral_root.exists(),
            "ephemeral root must be reclaimed even when no worker is admitted"
        );
    }

    #[test]
    fn exact_environment_rejects_case_insensitive_duplicates() {
        let env = BTreeMap::from([
            ("PATH".to_string(), "one".to_string()),
            ("Path".to_string(), "two".to_string()),
        ]);
        let error = exact_attached_env(&AdapterId::new("test"), &env)
            .expect_err("Windows environment keys are case-insensitive");
        assert!(error.to_string().contains("duplicate case-insensitive"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn exact_environment_adds_only_required_windows_bootstrap() {
        let adapter_id = AdapterId::new("test");
        let exact = exact_attached_env(
            &adapter_id,
            &BTreeMap::from([("ONLY".to_string(), "value".to_string())]),
        )
        .expect("construct exact Windows environment");

        for required in ["SystemDrive", "SystemRoot", "TEMP", "TMP", "LOCALAPPDATA"] {
            assert!(exact
                .iter()
                .any(|(key, value)| { key.eq_ignore_ascii_case(required) && !value.is_empty() }));
        }
        assert!(exact
            .iter()
            .any(|(key, value)| key == "ONLY" && value == "value"));
        assert!(!exact
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("PATH")));
        assert_eq!(exact.len(), 6, "no other ambient entries may leak");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn exact_environment_preserves_explicit_system_root() {
        let adapter_id = AdapterId::new("test");
        let exact = exact_attached_env(
            &adapter_id,
            &BTreeMap::from([("systemroot".to_string(), "X:\\Windows".to_string())]),
        )
        .expect("construct exact Windows environment");

        let explicit = exact
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case("SystemRoot"))
            .collect::<Vec<_>>();
        assert_eq!(explicit.len(), 1);
        assert_eq!(explicit[0].1, OsString::from("X:\\Windows"));
        assert_eq!(exact.len(), 5, "only the other bootstrap entries are added");
    }

    #[test]
    fn attached_network_contract_rejects_false_host_inheritance() {
        let error = validate_attached_network_contract(
            &AdapterId::new("test"),
            AttachedNetworkMode::DenyAll,
            &NetPolicy::HostInherited,
        )
        .expect_err("deny-all cannot be reported as host-inherited");
        assert!(matches!(
            error,
            SandboxAdapterError::NetPolicyApplyFailed { .. }
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn named_security_paths_remove_only_windows_verbatim_prefixes() {
        assert_eq!(
            windows_named_security_path(PathBuf::from(r"\\?\C:\handshake\runtime")),
            PathBuf::from(r"C:\handshake\runtime")
        );
        assert_eq!(
            windows_named_security_path(PathBuf::from(r"\\?\UNC\server\share\runtime")),
            PathBuf::from(r"\\server\share\runtime")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn canonical_system_root_descendants_are_ambient_after_prefix_normalization() {
        let system_root = std::env::var_os("SystemRoot").expect("Windows defines SystemRoot");
        let canonical_root = std::fs::canonicalize(system_root).expect("canonicalize SystemRoot");
        let named_root = windows_named_security_path(canonical_root);

        assert!(
            appcontainer_path_is_ambient(&named_root.join("System32").join("cmd.exe")),
            "protected SystemRoot descendants must not receive AppContainer DACL rewrites"
        );
    }
}
