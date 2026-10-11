use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::io;

use async_trait::async_trait;
#[cfg(target_os = "windows")]
use sha2::{Digest, Sha256};

#[cfg(target_os = "windows")]
use crate::sandbox::windows_native_jail_target_capabilities;
use crate::sandbox::{
    AdapterCapabilities, AdapterId, AttachedEphemeralPathGuard, AttachedNetworkMode,
    AttachedProcessSpec, AttachedSandboxProcess, AttachedStdioContract, BindMode, Command,
    DetachedProcessIdentity, ExecResult, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec,
    ProcessStatus, SandboxAdapter, SandboxAdapterError, Signal, ThroughputClass,
};
#[cfg(not(target_os = "windows"))]
use crate::sandbox::{GpuPassthrough, IsolationStrength};

pub const HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID: &str = "handshake_native";

/// Exact Windows process-generation identity persisted beside an attributable
/// START row. Unlike a timestamp tolerance, the raw creation FILETIME cannot
/// accept a same-binary PID reuse from another generation.
#[cfg(target_os = "windows")]
pub fn process_creation_time_100ns(pid: u32) -> std::io::Result<u64> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME},
        System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    };

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let result =
        unsafe { GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) };
    unsafe { CloseHandle(handle) };
    if result == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64)
}

#[cfg(not(target_os = "windows"))]
pub fn process_creation_time_100ns(_pid: u32) -> std::io::Result<u64> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "exact process-generation FILETIME is available only on Windows",
    ))
}

/// Default Handshake-managed Tier-1 process jail.
///
/// Windows is the first supported host: attached children are launched through
/// rappct AppContainer + Restricted Token + creation-time Job Object. Other
/// hosts fail closed until they have an equally real Tier-1 implementation.
#[derive(Debug, Clone, Default)]
pub struct HandshakeNativeSandboxAdapter;

impl HandshakeNativeSandboxAdapter {
    pub fn new() -> Self {
        Self
    }

    fn adapter_id() -> AdapterId {
        AdapterId::new(HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
    }

    fn unavailable(reason: impl Into<String>) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: Self::adapter_id(),
            reason: reason.into(),
        }
    }

    fn unsupported(operation: &str) -> SandboxAdapterError {
        SandboxAdapterError::SpawnFailed {
            adapter_id: Self::adapter_id(),
            reason: format!(
                "HandshakeNativeSandboxAdapter {operation} is unavailable through detached handles; use spawn_attached"
            ),
        }
    }

    async fn spawn_attached_owned(
        &self,
        spec: AttachedProcessSpec,
        ephemeral_cleanup: AttachedEphemeralPathGuard,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        if spec.requested_isolation_tier != IsolationTier::Tier1Container {
            return Err(SandboxAdapterError::SpawnFailed {
                adapter_id: Self::adapter_id(),
                reason: format!(
                    "Handshake-native attached execution requires Tier1Container, requested {:?}",
                    spec.requested_isolation_tier
                ),
            });
        }
        if spec.trust_class.min_isolation_tier().rank() > IsolationTier::Tier1Container.rank() {
            return Err(SandboxAdapterError::SpawnFailed {
                adapter_id: Self::adapter_id(),
                reason: format!(
                    "trust class {:?} requires {:?}; Tier-1 execution is forbidden",
                    spec.trust_class,
                    spec.trust_class.min_isolation_tier()
                ),
            });
        }
        self.validate_attached_network_mode(spec.network_mode)?;

        #[cfg(target_os = "windows")]
        {
            crate::sandbox::windows_native_jail::adapter::spawn_handshake_native_attached(
                Self::adapter_id(),
                spec,
                self.capabilities(),
                ephemeral_cleanup,
            )
            .await
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = (spec, ephemeral_cleanup);
            Err(Self::unavailable(
                "HandshakeNativeSandboxAdapter attached Tier-1 jail is currently implemented only on Windows",
            ))
        }
    }
}

#[async_trait]
impl SandboxAdapter for HandshakeNativeSandboxAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        Err(Self::unsupported("spawn"))
    }

    async fn spawn_attached(
        &self,
        mut spec: AttachedProcessSpec,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        let ephemeral_cleanup = AttachedEphemeralPathGuard::take_from(&mut spec);
        self.spawn_attached_owned(spec, ephemeral_cleanup).await
    }

    async fn spawn_attached_with_stdio(
        &self,
        mut spec: AttachedProcessSpec,
        stdio: AttachedStdioContract,
    ) -> Result<Box<dyn AttachedSandboxProcess>, SandboxAdapterError> {
        let ephemeral_cleanup = AttachedEphemeralPathGuard::take_from(&mut spec);
        stdio.validate(Self::adapter_id())?;
        self.spawn_attached_owned(spec, ephemeral_cleanup).await
    }

    fn validate_attached_network_mode(
        &self,
        mode: AttachedNetworkMode,
    ) -> Result<(), SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            match mode {
                AttachedNetworkMode::DenyAll | AttachedNetworkMode::OutboundInternetClient => {
                    Ok(())
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = mode;
            Err(Self::unavailable(
                "no supported Tier-1 attached network boundary exists on this host",
            ))
        }
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        Err(Self::unsupported("exec"))
    }

    async fn fs_bind(
        &self,
        _handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        Err(Self::unsupported("fs_bind"))
    }

    async fn net_policy(
        &self,
        _handle: &ProcessHandle,
        _policy: NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        Err(SandboxAdapterError::NetPolicyApplyFailed {
            adapter_id: Self::adapter_id(),
            reason: "attached network policy is immutable after AppContainer creation".to_string(),
        })
    }

    async fn kill(
        &self,
        _handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        Err(Self::unsupported("kill"))
    }

    async fn reclaim_detached(
        &self,
        identity: &DetachedProcessIdentity,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            reclaim_verified_detached_process(identity, HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = identity;
            Err(Self::unavailable(
                "crash-detached Handshake-native reclaim is implemented only on Windows",
            ))
        }
    }

    async fn status(&self, _handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        Err(Self::unsupported("status"))
    }

    async fn detached_status(
        &self,
        identity: &DetachedProcessIdentity,
    ) -> Result<ProcessStatus, SandboxAdapterError> {
        #[cfg(target_os = "windows")]
        {
            verified_detached_process_status(identity, HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = identity;
            Err(Self::unavailable(
                "crash-detached Handshake-native status is implemented only on Windows",
            ))
        }
    }

    async fn exit_code(&self, _handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        Err(Self::unsupported("exit_code"))
    }

    fn capabilities(&self) -> AdapterCapabilities {
        #[cfg(target_os = "windows")]
        {
            let target = windows_native_jail_target_capabilities();
            AdapterCapabilities {
                adapter_id: Self::adapter_id(),
                runtime_available: true,
                stdio_throughput_class: ThroughputClass::High,
                supports_live_token_stream: true,
                ..target
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            AdapterCapabilities {
                adapter_id: Self::adapter_id(),
                runtime_available: false,
                filesystem_isolation_strength: IsolationStrength::Weak,
                network_isolation_strength: IsolationStrength::Weak,
                gpu_passthrough: GpuPassthrough::None,
                stdio_throughput_class: ThroughputClass::Low,
                win32_native_fidelity: false,
                cross_machine_portable: false,
                isolation_tier: IsolationTier::Tier1Container,
                requires_nested_virt: false,
                supports_snapshot: false,
                supports_persistent_exec: false,
                supports_warm_agent: false,
                supports_live_token_stream: false,
            }
        }
    }
}

#[cfg(target_os = "windows")]
struct VerifiedDetachedProcessHandle(windows_sys::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl Drop for VerifiedDetachedProcessHandle {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(target_os = "windows")]
fn detached_identity_error(
    identity: &DetachedProcessIdentity,
    reason: impl Into<String>,
) -> SandboxAdapterError {
    SandboxAdapterError::SpawnFailed {
        adapter_id: identity.handle.adapter_id.clone(),
        reason: format!(
            "detached process {}: {}",
            identity.process_uuid,
            reason.into()
        ),
    }
}

#[cfg(target_os = "windows")]
fn open_verified_detached_process(
    identity: &DetachedProcessIdentity,
    expected_adapter_id: &str,
) -> Result<Option<VerifiedDetachedProcessHandle>, SandboxAdapterError> {
    use windows_sys::Win32::{
        Foundation::FILETIME,
        System::Threading::{
            GetProcessTimes, OpenProcess, QueryFullProcessImageNameW,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
        },
    };

    if identity.handle.adapter_id.as_str() != expected_adapter_id {
        return Err(detached_identity_error(
            identity,
            format!("owning adapter identity does not match {expected_adapter_id}"),
        ));
    }
    let pid = identity.handle.pid.ok_or_else(|| {
        detached_identity_error(identity, "missing OS PID; PID-only reclaim is forbidden")
    })?;
    let expected_creation_time = identity.os_creation_time_100ns.ok_or_else(|| {
        detached_identity_error(
            identity,
            "missing exact OS creation-generation identity; reclaim is forbidden",
        )
    })?;
    let expected_hash = identity.executable_sha256.as_deref().ok_or_else(|| {
        detached_identity_error(
            identity,
            "missing immutable executable hash; reclaim is forbidden",
        )
    })?;

    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        let error = io::Error::last_os_error();
        return match error.raw_os_error() {
            Some(87) => Ok(None),
            _ => Err(detached_identity_error(
                identity,
                format!("cannot open pid {pid}: {error}"),
            )),
        };
    }
    let process = VerifiedDetachedProcessHandle(handle);
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    if unsafe { GetProcessTimes(process.0, &mut creation, &mut exit, &mut kernel, &mut user) } == 0
    {
        return Err(detached_identity_error(
            identity,
            format!(
                "cannot read creation identity: {}",
                io::Error::last_os_error()
            ),
        ));
    }
    let creation_time = ((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64;
    if creation_time != expected_creation_time {
        return Err(detached_identity_error(
            identity,
            "PID creation generation does not match durable launch identity",
        ));
    }

    let mut path = vec![0u16; 32_768];
    let mut path_len = path.len() as u32;
    if unsafe { QueryFullProcessImageNameW(process.0, 0, path.as_mut_ptr(), &mut path_len) } == 0 {
        return Err(detached_identity_error(
            identity,
            format!(
                "cannot read executable identity: {}",
                io::Error::last_os_error()
            ),
        ));
    }
    path.truncate(path_len as usize);
    let executable = PathBuf::from(String::from_utf16(&path).map_err(|error| {
        detached_identity_error(identity, format!("invalid executable path: {error}"))
    })?);
    let actual_hash = Sha256::digest(std::fs::read(&executable).map_err(|error| {
        detached_identity_error(
            identity,
            format!("cannot hash executable {}: {error}", executable.display()),
        )
    })?);
    if !hex::encode(actual_hash).eq_ignore_ascii_case(expected_hash) {
        return Err(detached_identity_error(
            identity,
            "executable hash does not match durable launch identity",
        ));
    }
    Ok(Some(process))
}

#[cfg(target_os = "windows")]
fn terminate_verified_detached_process(
    identity: &DetachedProcessIdentity,
    expected_adapter_id: &str,
) -> Result<(), SandboxAdapterError> {
    use windows_sys::Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{TerminateProcess, WaitForSingleObject},
    };

    let Some(process) = open_verified_detached_process(identity, expected_adapter_id)? else {
        return Ok(());
    };
    if unsafe { TerminateProcess(process.0, 1) } == 0 {
        return Err(detached_identity_error(
            identity,
            format!("TerminateProcess failed: {}", io::Error::last_os_error()),
        ));
    }
    match unsafe { WaitForSingleObject(process.0, 5_000) } {
        WAIT_OBJECT_0 => Ok(()),
        WAIT_TIMEOUT => Err(detached_identity_error(
            identity,
            "process did not terminate within 5000ms",
        )),
        result => Err(detached_identity_error(
            identity,
            format!(
                "process wait failed with result {result}: {}",
                io::Error::last_os_error()
            ),
        )),
    }
}

/// Execute exact-generation host-process reclaim on a blocking worker while
/// preserving the adapter id as part of the durable ownership decision.
#[cfg(target_os = "windows")]
pub(crate) async fn reclaim_verified_detached_process(
    identity: &DetachedProcessIdentity,
    expected_adapter_id: &'static str,
) -> Result<(), SandboxAdapterError> {
    let identity = identity.clone();
    tokio::task::spawn_blocking(move || {
        terminate_verified_detached_process(&identity, expected_adapter_id)
    })
    .await
    .map_err(|error| SandboxAdapterError::SpawnFailed {
        adapter_id: identity_adapter_id(expected_adapter_id),
        reason: format!("detached reclaim worker failed to join: {error}"),
    })?
}

#[cfg(target_os = "windows")]
pub(crate) async fn verified_detached_process_status(
    identity: &DetachedProcessIdentity,
    expected_adapter_id: &'static str,
) -> Result<ProcessStatus, SandboxAdapterError> {
    let identity = identity.clone();
    let running = tokio::task::spawn_blocking(move || {
        open_verified_detached_process(&identity, expected_adapter_id)
            .map(|process| process.is_some())
    })
    .await
    .map_err(|error| SandboxAdapterError::SpawnFailed {
        adapter_id: identity_adapter_id(expected_adapter_id),
        reason: format!("detached status worker failed to join: {error}"),
    })??;
    match running {
        true => Ok(ProcessStatus::Running),
        false => Ok(ProcessStatus::Exited { code: 0 }),
    }
}

#[cfg(target_os = "windows")]
fn identity_adapter_id(value: &str) -> AdapterId {
    AdapterId::new(value)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        io::{Read, Write},
        time::{Duration, Instant},
    };

    use super::*;
    use crate::sandbox::{AttachedStdioMode, BindSpec, ResourceLimits, TrustClass};

    #[cfg(target_os = "windows")]
    static APP_CONTAINER_TEST_GUARD: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn populated_ephemeral_root(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "handshake-native-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create ephemeral ownership root");
        std::fs::write(root.join("owned.txt"), b"owned")
            .expect("populate ephemeral ownership root");
        root
    }

    fn probe_spec(mode: &str) -> AttachedProcessSpec {
        let executable = std::env::current_exe().expect("current test executable");
        AttachedProcessSpec {
            executable_path: executable.clone(),
            args: vec![
                "--ignored".into(),
                "--exact".into(),
                "sandbox::handshake_native::tests::attached_child_probe".into(),
                "--nocapture".into(),
            ],
            env: BTreeMap::from([
                ("HANDSHAKE_NATIVE_PROBE_MODE".into(), mode.into()),
                ("HANDSHAKE_EXACT_ENV_MARKER".into(), "present".into()),
            ]),
            cwd: None,
            binds: vec![BindSpec {
                host_path: executable.clone(),
                guest_path: executable,
                mode: BindMode::ReadOnly,
            }],
            network_mode: AttachedNetworkMode::DenyAll,
            trust_class: TrustClass::Reviewed,
            required_capabilities: BTreeSet::new(),
            requested_isolation_tier: IsolationTier::Tier1Container,
            requested_net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits {
                timeout_ms: Some(30_000),
                ..ResourceLimits::default()
            },
            startup_timeout_ms: 60_000,
            ephemeral_cleanup_paths: Vec::new(),
            execution_policy_ref: "execution-policy://test/tier1-probe".to_string(),
            resolved_execution_policy: None,
            swarm_id: Some("tier1-probe-swarm".to_string()),
            worktree_id: Some("tier1-probe-worktree".to_string()),
            checkout_lease_id: None,
            checkout_lease_owner_generation: None,
            checkout_lease_canonical_working_dir: None,
        }
    }

    #[test]
    fn trust_classes_never_select_an_invented_tier_zero() {
        assert_eq!(
            TrustClass::Trusted.min_isolation_tier(),
            IsolationTier::Tier1Container
        );
        assert_eq!(
            TrustClass::Reviewed.min_isolation_tier(),
            IsolationTier::Tier1Container
        );
        assert_eq!(
            TrustClass::UntrustedAgent.min_isolation_tier(),
            IsolationTier::Tier3Microvm
        );
    }

    #[test]
    fn capabilities_never_claim_host_native_execution() {
        let capabilities = HandshakeNativeSandboxAdapter::new().capabilities();
        assert_eq!(capabilities.isolation_tier, IsolationTier::Tier1Container);
        assert_eq!(capabilities.runtime_available, cfg!(target_os = "windows"));
        if cfg!(target_os = "windows") {
            assert_eq!(capabilities.stdio_throughput_class, ThroughputClass::High);
            assert!(capabilities.supports_live_token_stream);
        }
    }

    #[tokio::test]
    async fn zero_startup_deadline_fails_before_process_creation() {
        let mut spec = probe_spec("environment");
        spec.startup_timeout_ms = 0;
        let started = Instant::now();
        let error = match HandshakeNativeSandboxAdapter::new()
            .spawn_attached(spec)
            .await
        {
            Ok(_) => panic!("zero startup deadline must fail closed"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("startup_timeout_ms"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn untrusted_agent_is_rejected_before_tier1_spawn() {
        let mut spec = probe_spec("environment");
        spec.trust_class = TrustClass::UntrustedAgent;
        let error = match HandshakeNativeSandboxAdapter::new()
            .spawn_attached(spec)
            .await
        {
            Ok(_) => panic!("untrusted agent must not enter Tier 1"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("Tier-1 execution is forbidden"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn invalid_tier_reclaims_ephemeral_paths_at_public_boundary() {
        let root = populated_ephemeral_root("invalid-tier");
        let mut spec = probe_spec("environment");
        spec.requested_isolation_tier = IsolationTier::Tier3Microvm;
        spec.ephemeral_cleanup_paths = vec![root.clone()];

        let result = HandshakeNativeSandboxAdapter::new()
            .spawn_attached(spec)
            .await;

        assert!(result.is_err(), "Tier-3 request must not enter Tier-1");
        assert!(!root.exists(), "invalid-tier refusal must reclaim root");
    }

    #[tokio::test]
    async fn invalid_trust_reclaims_ephemeral_paths_at_public_boundary() {
        let root = populated_ephemeral_root("invalid-trust");
        let mut spec = probe_spec("environment");
        spec.trust_class = TrustClass::UntrustedAgent;
        spec.ephemeral_cleanup_paths = vec![root.clone()];

        let result = HandshakeNativeSandboxAdapter::new()
            .spawn_attached(spec)
            .await;

        assert!(result.is_err(), "untrusted request must not enter Tier-1");
        assert!(!root.exists(), "invalid-trust refusal must reclaim root");
    }

    #[tokio::test]
    async fn invalid_stdio_reclaims_ephemeral_paths_at_public_boundary() {
        let root = populated_ephemeral_root("invalid-stdio");
        let mut spec = probe_spec("environment");
        spec.ephemeral_cleanup_paths = vec![root.clone()];
        let invalid_stdio = AttachedStdioContract {
            stdin: AttachedStdioMode::Pipe,
            stdout: AttachedStdioMode::Pipe,
            stderr: AttachedStdioMode::Pipe,
        };

        let result = HandshakeNativeSandboxAdapter::new()
            .spawn_attached_with_stdio(spec, invalid_stdio)
            .await;

        assert!(result.is_err(), "unsupported stdio must fail closed");
        assert!(!root.exists(), "invalid-stdio refusal must reclaim root");
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn appcontainer_attached_spawn_replaces_environment_and_owns_streams() {
        let _serial = APP_CONTAINER_TEST_GUARD.lock().await;
        let adapter = HandshakeNativeSandboxAdapter::new();
        let mut process = adapter
            .spawn_attached(probe_spec("environment"))
            .await
            .expect("spawn real AppContainer attached probe");
        let mut stdout = process.take_stdout().expect("owned stdout");
        let mut stderr = process.take_stderr().expect("owned stderr");
        let status = process.wait().expect("wait and reap AppContainer probe");
        let mut stdout_text = String::new();
        let mut stderr_text = String::new();
        stdout
            .read_to_string(&mut stdout_text)
            .expect("read stdout");
        stderr
            .read_to_string(&mut stderr_text)
            .expect("read stderr");
        assert!(status.success(), "probe exited successfully: {stderr_text}");
        assert!(stdout_text.contains("marker=present"), "{stdout_text}");
        assert!(
            stdout_text.contains("path_inherited=false"),
            "{stdout_text}"
        );
        assert!(stderr_text.contains("owned-stderr"), "{stderr_text}");
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn successful_child_owns_ephemeral_paths_until_terminal_wait() {
        let _serial = APP_CONTAINER_TEST_GUARD.lock().await;
        let root = populated_ephemeral_root("child-lifetime");
        let mut spec = probe_spec("environment");
        spec.ephemeral_cleanup_paths = vec![root.clone()];
        let mut process = HandshakeNativeSandboxAdapter::new()
            .spawn_attached(spec)
            .await
            .expect("spawn child with transferred ephemeral ownership");

        assert!(root.exists(), "live child must retain its ephemeral root");
        let status = process.wait().expect("wait and clean child");
        assert!(status.success(), "ownership probe exits successfully");
        assert!(
            !root.exists(),
            "terminal child cleanup must reclaim its ephemeral root"
        );
    }

    #[test]
    #[ignore]
    fn attached_child_probe() {
        if std::env::var("HANDSHAKE_NATIVE_PROBE_MODE").as_deref() == Ok("environment") {
            println!(
                "marker={}",
                std::env::var("HANDSHAKE_EXACT_ENV_MARKER").unwrap_or_default()
            );
            println!("path_inherited={}", std::env::var_os("PATH").is_some());
            eprintln!("owned-stderr");
            std::io::stdout().flush().expect("flush probe stdout");
        }
    }
}
