use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use handshake_core::{
    kernel::{PromotionDecisionKind, PromotionGate},
    sandbox::{
        build_promotion_request, docker_run_args, podman_run_args, replay_sandbox_promotion_events,
        AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
        IsolationStrength, ProcessHandle, ProcessSpec, ProcessStatus, SandboxAdapter,
        SandboxAdapterError, SandboxPromotionOutcome, SandboxValidationEvidence, Signal,
        ThroughputClass, ValidationJobSpec, ValidationProcessSpecBuilder, ValidationSandboxRunner,
        DOCKER_ADAPTER_ID, WSL2_PODMAN_ADAPTER_ID,
    },
};
use serde::Deserialize;

#[cfg(feature = "docker-integration")]
use handshake_core::sandbox::{DockerAdapter, DockerConfig};
#[cfg(feature = "wsl2-integration")]
use handshake_core::sandbox::{Wsl2PodmanAdapter, Wsl2PodmanConfig};

const FIXTURE_DIR: &str = "tests/fixtures/kernel_003_migration";
const FIXTURES: [&str; 3] = [
    "validation_job_001.json",
    "validation_job_002.json",
    "validation_job_003.json",
];

#[derive(Debug, Deserialize)]
struct MigrationFixture {
    name: String,
    job: ValidationJobSpec,
    expected_process_spec: ProcessSpec,
    expected_docker_args: Vec<String>,
    expected_podman_args: Vec<String>,
    candidate_artifact_id: String,
}

#[derive(Debug, Clone)]
struct RecordingAdapter {
    capabilities: AdapterCapabilities,
    spawned: Arc<Mutex<Vec<ProcessSpec>>>,
}

impl RecordingAdapter {
    fn new(capabilities: AdapterCapabilities) -> Self {
        Self {
            capabilities,
            spawned: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn spawned(&self) -> Vec<ProcessSpec> {
        self.spawned.lock().expect("spawn log").clone()
    }

    fn unavailable(&self) -> SandboxAdapterError {
        SandboxAdapterError::AdapterUnavailable {
            adapter_id: self.capabilities.adapter_id.clone(),
            reason: "recording adapter has no runtime backend".to_string(),
        }
    }
}

#[async_trait]
impl SandboxAdapter for RecordingAdapter {
    async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        self.spawned.lock().expect("spawn log").push(spec);
        Ok(ProcessHandle::new(
            self.capabilities.adapter_id.clone(),
            Some(7001),
            format!("{}-validation", self.capabilities.adapter_id.as_str()),
        ))
    }

    async fn exec(
        &self,
        _handle: &ProcessHandle,
        _cmd: Command,
    ) -> Result<ExecResult, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn fs_bind(
        &self,
        _handle: &ProcessHandle,
        _host_path: PathBuf,
        _guest_path: PathBuf,
        _mode: BindMode,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn net_policy(
        &self,
        _handle: &ProcessHandle,
        _policy: handshake_core::sandbox::NetPolicy,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn kill(
        &self,
        _handle: &ProcessHandle,
        _signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn status(&self, _handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        Err(self.unavailable())
    }

    async fn exit_code(&self, _handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        Err(self.unavailable())
    }

    fn capabilities(&self) -> AdapterCapabilities {
        self.capabilities.clone()
    }
}

#[test]
fn fixtures_snapshot_validation_job_to_process_spec_and_adapter_argv() {
    for fixture in fixtures() {
        let process_spec = ValidationProcessSpecBuilder
            .build(fixture.job.clone())
            .expect("validation process spec");

        assert_eq!(
            process_spec, fixture.expected_process_spec,
            "fixture {} process spec drifted",
            fixture.name
        );
        assert_eq!(
            docker_run_args(&process_spec, "hsk-k003-migration").expect("docker args"),
            fixture.expected_docker_args,
            "fixture {} docker argv drifted",
            fixture.name
        );
        assert_eq!(
            podman_run_args(&process_spec).expect("podman args"),
            fixture.expected_podman_args,
            "fixture {} podman argv drifted",
            fixture.name
        );
    }
}

#[tokio::test]
async fn fixtures_spawn_through_trait_object_matrix_and_promote_without_concrete_adapters() {
    for fixture in fixtures() {
        for capabilities in [strong_capabilities(WSL2_PODMAN_ADAPTER_ID)] {
            let adapter = RecordingAdapter::new(capabilities.clone());
            let mut registry = handshake_core::sandbox::SandboxAdapterRegistry::new(
                capabilities.adapter_id.clone(),
            );
            registry.register(Arc::new(adapter.clone()));
            let runner = ValidationSandboxRunner::from_registry(&registry, &fixture.job)
                .expect("trait object runner selected through registry");
            let handle = runner.spawn().await.expect("trait object spawn");

            assert_eq!(handle.adapter_id, capabilities.adapter_id);
            assert_eq!(
                adapter.spawned(),
                vec![fixture.expected_process_spec.clone()]
            );

            let decision = PromotionGate::decide_sandbox_validated(build_promotion_request(
                evidence(handle, capabilities.clone(), 0, &fixture.name),
                fixture.candidate_artifact_id.clone(),
            ));
            assert_eq!(
                decision.promotion_decision_kind,
                PromotionDecisionKind::Approved
            );
            assert!(matches!(
                decision.outcome,
                SandboxPromotionOutcome::Accepted
            ));
        }
    }
}

#[test]
fn replay_equivalence_ignores_process_handle_identity_and_timestamps_per_adapter() {
    for fixture in fixtures() {
        for capabilities in [
            strong_capabilities(DOCKER_ADAPTER_ID),
            strong_capabilities(WSL2_PODMAN_ADAPTER_ID),
            strong_capabilities("noop_trait_object"),
        ] {
            let first = PromotionGate::decide_sandbox_validated(build_promotion_request(
                evidence(
                    ProcessHandle::new(capabilities.adapter_id.clone(), Some(8001), "first-run"),
                    capabilities.clone(),
                    0,
                    &fixture.name,
                ),
                fixture.candidate_artifact_id.clone(),
            ))
            .event_row
            .expect("first event");
            let second = PromotionGate::decide_sandbox_validated(build_promotion_request(
                evidence(
                    ProcessHandle::new(capabilities.adapter_id.clone(), Some(8002), "second-run"),
                    capabilities.clone(),
                    0,
                    &fixture.name,
                ),
                fixture.candidate_artifact_id.clone(),
            ))
            .event_row
            .expect("second event");

            assert_ne!(first.process_handle_id, second.process_handle_id);
            assert_ne!(first.sandbox_internal_id, second.sandbox_internal_id);
            assert_eq!(
                replay_sandbox_promotion_events([first]),
                replay_sandbox_promotion_events([second]),
                "fixture {} adapter {} replay projection drifted",
                fixture.name,
                capabilities.adapter_id
            );
        }
    }
}

#[tokio::test]
#[cfg(feature = "docker-integration")]
async fn docker_integration_fixture_can_spawn_when_backend_available() {
    let adapter = match DockerAdapter::try_new(DockerConfig::default()).await {
        Ok(adapter) => adapter,
        Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
        | Err(SandboxAdapterError::SpawnFailed { reason, .. })
            if reason.contains("docker unavailable")
                || reason.contains("Docker daemon unreachable")
                || reason.contains("failed to spawn") =>
        {
            eprintln!("skipping live Docker MT-055 integration: {reason}");
            return;
        }
        Err(error) => panic!("Docker MT-055 setup failed unexpectedly: {error:?}"),
    };
    live_spawn_smoke(Arc::new(adapter), DOCKER_ADAPTER_ID).await;
}

#[tokio::test]
#[cfg(feature = "wsl2-integration")]
async fn wsl2_podman_integration_fixture_can_spawn_when_backend_available() {
    let adapter = match Wsl2PodmanAdapter::try_new(Wsl2PodmanConfig::for_distro("Ubuntu")).await {
        Ok(adapter) => adapter,
        Err(SandboxAdapterError::AdapterUnavailable { reason, .. })
        | Err(SandboxAdapterError::SpawnFailed { reason, .. })
            if reason.contains("podman unavailable")
                || reason.contains("not registered")
                || reason.contains("WSL")
                || reason.contains("wsl")
                || reason.contains("failed to spawn") =>
        {
            eprintln!("skipping live WSL2 Podman MT-055 integration: {reason}");
            return;
        }
        Err(error) => panic!("WSL2 Podman MT-055 setup failed unexpectedly: {error:?}"),
    };
    live_spawn_smoke(Arc::new(adapter), WSL2_PODMAN_ADAPTER_ID).await;
}

#[cfg(any(feature = "docker-integration", feature = "wsl2-integration"))]
async fn live_spawn_smoke(adapter: Arc<dyn SandboxAdapter>, expected_adapter_id: &str) {
    let fixture = fixtures()
        .into_iter()
        .find(|fixture| fixture.name == "trivial-no-bind")
        .expect("trivial no-bind fixture");
    let mut registry =
        handshake_core::sandbox::SandboxAdapterRegistry::new(AdapterId::new(expected_adapter_id));
    if expected_adapter_id == DOCKER_ADAPTER_ID {
        registry.set_docker_explicit_opt_in(true);
    }
    registry.register(Arc::clone(&adapter));
    let mut job = fixture.job;
    if expected_adapter_id == DOCKER_ADAPTER_ID {
        job.work_profile_override = Some(AdapterId::new(DOCKER_ADAPTER_ID));
    }
    let runner =
        ValidationSandboxRunner::from_registry(&registry, &job).expect("live runner selection");
    let handle = runner.spawn().await.expect("live spawn");
    assert_eq!(handle.adapter_id, AdapterId::new(expected_adapter_id));

    match adapter.status(&handle).await {
        Ok(ProcessStatus::Exited { code }) => {
            assert_eq!(code, 0);
            adapter
                .kill(&handle, Signal::Kill)
                .await
                .expect("cleanup exited validation fixture");
        }
        Ok(ProcessStatus::Running) => adapter
            .kill(&handle, Signal::Kill)
            .await
            .expect("cleanup running validation fixture"),
        Ok(other) => panic!("unexpected live fixture status: {other:?}"),
        Err(error) => panic!("failed to inspect live fixture status: {error:?}"),
    }
}

fn fixtures() -> Vec<MigrationFixture> {
    FIXTURES
        .into_iter()
        .map(|file_name| {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(FIXTURE_DIR)
                .join(file_name);
            let text = fs::read_to_string(&path).expect("kernel 003 migration fixture");
            serde_json::from_str(&text).expect("kernel 003 migration fixture json")
        })
        .collect()
}

fn evidence(
    process_handle: ProcessHandle,
    adapter_capabilities: AdapterCapabilities,
    validation_exit_code: i32,
    fixture_name: &str,
) -> SandboxValidationEvidence {
    SandboxValidationEvidence {
        process_handle,
        adapter_capabilities,
        validation_exit_code,
        validation_stdout_artifact_id: format!("ART-{fixture_name}-stdout"),
        validation_stderr_artifact_id: format!("ART-{fixture_name}-stderr"),
        sandbox_runtime_ms: 42,
    }
}

fn strong_capabilities(adapter_id: &str) -> AdapterCapabilities {
    AdapterCapabilities {
        adapter_id: AdapterId::new(adapter_id),
        runtime_available: true,
        filesystem_isolation_strength: IsolationStrength::Strong,
        network_isolation_strength: IsolationStrength::Strong,
        gpu_passthrough: GpuPassthrough::None,
        stdio_throughput_class: ThroughputClass::High,
        win32_native_fidelity: false,
        cross_machine_portable: true,
    }
}
