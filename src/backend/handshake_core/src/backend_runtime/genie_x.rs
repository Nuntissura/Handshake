//! Host-owned configuration, registration and retirement for the pinned GenieX adapters.
//! Vendor resolver: qualcomm/GenieX commit 7b234f6c2f75c64b64bcbfd24eb47e6dec4028dd,
//! sdk/model-manager/crates/core/src/{config,store}.rs. Only Store::get_paths is used;
//! pull/query/download/environment discovery are not invoked. The inspected v0.9.3
//! CLI infer path can auto-pull absent models, so admission verifies the complete cached
//! selection first. Planner and worker own arguments and child custody respectively.
//! File leases currently implement Windows sharing semantics; other platforms fail closed.
//! This source requires owning dependency proof and host type/runtime validation.
use handshake_dexterity::{Device, Engine, HostBinding, RuntimeCapability};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::Arc,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GenieXHostConfiguration {
    schema: String,
    executable: PathBuf,
    executable_sha256: String,
    cwd: PathBuf,
    data: PathBuf,
    environment: BTreeMap<String, String>,
    models: Vec<GenieXConfiguredArtifact>,
    #[serde(default)]
    local_execution_policies: Vec<crate::kernel::local_execution_authority::LocalExecutionPolicyRecord>,
    max_workers: usize,
    max_in_flight_bytes: u64,
    admission_bytes: u64,
    spawn_timeout_ms: u64,
    operation_timeout_ms: u64,
    chunk_bytes: usize,
    stdout_bytes: u64,
    stderr_bytes: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GenieXConfiguredArtifact {
    // Stable EngineProfile.model.id, separate from the real per-boot ModelId.
    model_id: String,
    selector: String,
    engine: Engine,
    devices: Vec<Device>,
    content_sha256: String,
    files: BTreeMap<PathBuf, GenieXPinnedFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GenieXPinnedFile {
    bytes: u64,
    sha256: String,
}

struct GenieXVerifiedArtifact {
    model_id: String,
    engine: Engine,
    primary_artifact: PathBuf,
    model_directory: PathBuf,
    compiled_context: Option<u32>,
    binding: HostBinding,
}

struct GenieXVerifiedConfiguration {
    configuration: GenieXHostConfiguration,
    // Captured host source provenance. This source alone never authorizes execution.
    policy_source: crate::kernel::local_execution_authority::LocalExecutionPolicySource,
    artifacts: Vec<GenieXVerifiedArtifact>,
    temporary_directories: BTreeSet<PathBuf>,
    baseline: crate::sandbox::AttachedBaselineBindings,
}

fn genie_x_config_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "GenieX configuration or artifact identity rejected",
    )
}

fn genie_x_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn genie_x_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn genie_x_absolute(base: &Path, path: &Path) -> io::Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    };
    path.canonicalize()
}

fn genie_x_read_lease(path: &Path) -> io::Result<File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "GenieX input leases require a supported immutable-file implementation",
        ))
    }
}

fn genie_x_verify_file(
    baseline: &crate::sandbox::AttachedBaselineBindings,
    path: &Path,
    expected: &GenieXPinnedFile,
) -> io::Result<()> {
    if !genie_x_digest(&expected.sha256) {
        return Err(genie_x_config_error());
    }
    let mut lease = baseline.try_clone_file(path).map_err(|_| genie_x_config_error())?;
    lease.seek(SeekFrom::Start(0))?;
    let before = lease.metadata()?;
    if !before.is_file() || before.len() != expected.bytes {
        return Err(genie_x_config_error());
    }
    let mut hash = Sha256::new();
    let mut block = [0u8; 65_536];
    loop {
        let count = lease.read(&mut block)?;
        if count == 0 {
            break;
        }
        hash.update(&block[..count]);
    }
    let after = lease.metadata()?;
    if after.len() != before.len()
        || after.modified()? != before.modified()?
        || hex::encode(hash.finalize()) != expected.sha256
    {
        return Err(genie_x_config_error());
    }
    Ok(())
}

fn genie_x_baseline_bytes(
    baseline: &crate::sandbox::AttachedBaselineBindings,
    path: &Path,
) -> io::Result<Vec<u8>> {
    let mut file = baseline.try_clone_file(path).map_err(|_| genie_x_config_error())?;
    file.seek(SeekFrom::Start(0))?;
    if file.metadata()?.len() > 1_048_576 { return Err(genie_x_config_error()); }
    let mut bytes = Vec::new();
    file.take(1_048_577).read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 { return Err(genie_x_config_error()); }
    Ok(bytes)
}

fn genie_x_manifest_files(
    manifest: &serde_json::Value,
    quantization: &str,
) -> io::Result<BTreeSet<PathBuf>> {
    let mut required = BTreeSet::from([PathBuf::from("geniex.json")]);
    let primary = manifest
        .get("ModelFile")
        .and_then(|value| value.get(quantization))
        .ok_or_else(genie_x_config_error)?;
    let entries = std::iter::once(primary)
        .chain(
            ["MMProjFile", "TokenizerFile"]
                .into_iter()
                .filter_map(|name| manifest.get(name)),
        )
        .chain(
            manifest
                .get("ExtraFiles")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten(),
        );
    for entry in entries {
        let Some(name) = entry.get("Name").and_then(serde_json::Value::as_str) else {
            return Err(genie_x_config_error());
        };
        if name.is_empty() {
            continue;
        }
        let path = PathBuf::from(name);
        if entry.get("Downloaded").and_then(serde_json::Value::as_bool) != Some(true)
            || !genie_x_relative(&path)
        {
            return Err(genie_x_config_error());
        }
        required.insert(path);
    }
    Ok(required)
}

fn load_genie_x_configuration(
    path: &Path,
    ledger: &crate::process_ledger::LedgerBatcher,
) -> io::Result<(
    GenieXVerifiedConfiguration,
    Vec<crate::process_ledger::ReservedProcessLifecycle>,
)> {
    use model_manager_core::{config::StoreConfig, store::Store};
    let path = path.canonicalize()?;
    let parent = path.parent().ok_or_else(genie_x_config_error)?;
    let mut configuration_lease = genie_x_read_lease(&path)?;
    if configuration_lease.metadata()?.len() > 1_048_576 {
        return Err(genie_x_config_error());
    }
    let mut input = Vec::new();
    configuration_lease
        .by_ref()
        .take(1_048_577)
        .read_to_end(&mut input)?;
    if input.len() > 1_048_576 {
        return Err(genie_x_config_error());
    }
    let mut configuration: GenieXHostConfiguration =
        serde_json::from_slice(&input).map_err(|_| genie_x_config_error())?;
    if configuration.schema != "handshake.geniex.host-configuration@1"
        || configuration.models.is_empty()
        || configuration.models.len() > 32
        || configuration.max_workers == 0
        || configuration.max_workers > 32
        || configuration.max_in_flight_bytes == 0
        || configuration.admission_bytes == 0
        || configuration.admission_bytes > configuration.max_in_flight_bytes
        || configuration.spawn_timeout_ms == 0
        || configuration.operation_timeout_ms == 0
        || configuration.chunk_bytes == 0
        || configuration.chunk_bytes > 65_536
        || configuration.stdout_bytes == 0
        || configuration.stderr_bytes == 0
        || configuration
            .environment
            .iter()
            .any(|(key, value)| key.is_empty() || key.contains(['=', '\0']) || value.contains('\0'))
    {
        return Err(genie_x_config_error());
    }
    for policy in &configuration.local_execution_policies {
        if policy.resource_limits.memory_bytes.is_none_or(|limit| limit > configuration.admission_bytes)
            || policy.startup_timeout_ms > configuration.spawn_timeout_ms
            || policy.resource_limits.timeout_ms.is_none_or(|limit| limit > configuration.operation_timeout_ms)
        {
            return Err(genie_x_config_error());
        }
    }
    let configuration_sha256 = hex::encode(Sha256::digest(&input));
    let policy_source = crate::kernel::local_execution_authority::LocalExecutionPolicySource::from_captured_configuration(
        path.clone(), Arc::from(input), configuration_sha256, configuration.local_execution_policies.clone(),
    ).map_err(|_| genie_x_config_error())?;
    let reservations = ledger
        .try_reserve_lifecycles(configuration.models.len())
        .map_err(|_| genie_x_config_error())?;
    configuration.executable = genie_x_absolute(parent, &configuration.executable)?;
    configuration.cwd = genie_x_absolute(parent, &configuration.cwd)?;
    configuration.data = genie_x_absolute(parent, &configuration.data)?;
    if !configuration.cwd.is_dir() || !configuration.data.join("models").is_dir() {
        return Err(genie_x_config_error());
    }
    let mut temporary_directories = BTreeSet::new();
    for (key, value) in &configuration.environment {
            if !["TMP", "TEMP", "TMPDIR"].iter().any(|name| key.eq_ignore_ascii_case(name)) {
                continue;
            }
            let configured = PathBuf::from(value);
            if !configured.is_absolute() { return Err(genie_x_config_error()); }
            let temporary = configured.canonicalize().map_err(|_| genie_x_config_error())?;
            if !temporary.is_dir() || temporary.parent().is_none()
                || temporary.starts_with(&configuration.cwd) || configuration.cwd.starts_with(&temporary)
                || temporary.starts_with(&configuration.data)
                || configuration.data.starts_with(&temporary)
                || configuration.executable.starts_with(&temporary)
            { return Err(genie_x_config_error()); }
            temporary_directories.insert(temporary);
        }
    // Resolve the finite boot path set before capturing its native object identities.
    let store = Store::new(StoreConfig::new(configuration.data.clone()))
        .map_err(|_| genie_x_config_error())?;
    let mut baseline_paths = BTreeSet::from([
        path.clone(), configuration.executable.clone(), configuration.cwd.clone(),
        configuration.data.clone(),
    ]);
    baseline_paths.extend(temporary_directories.iter().cloned());
    for model in &configuration.models {
        let (_, resolved) = store.get_paths(&model.selector).map_err(|_| genie_x_config_error())?;
        let directory = resolved.model_dir.canonicalize()?;
        if !directory.starts_with(configuration.data.join("models")) {
            return Err(genie_x_config_error());
        }
        baseline_paths.insert(directory.clone());
        for relative in model.files.keys() {
            if !genie_x_relative(relative) { return Err(genie_x_config_error()); }
            let file = directory.join(relative).canonicalize()?;
            if !file.starts_with(&directory) { return Err(genie_x_config_error()); }
            baseline_paths.insert(file);
        }
    }
    let baseline = crate::sandbox::HandshakeNativeSandboxAdapter::capture_baseline_bindings(
        &baseline_paths.into_iter().collect::<Vec<_>>(),
    ).map_err(|_| genie_x_config_error())?;
    if genie_x_baseline_bytes(&baseline, &path)?.as_slice() != policy_source.bytes() {
        return Err(genie_x_config_error());
    }
    let executable_bytes = baseline.try_clone_file(&configuration.executable)
        .map_err(|_| genie_x_config_error())?.metadata()?.len();
    genie_x_verify_file(&baseline, &configuration.executable, &GenieXPinnedFile {
        bytes: executable_bytes, sha256: configuration.executable_sha256.clone(),
    })?;
    let mut artifacts = Vec::new();
    let mut identities = BTreeSet::new();
    for model in &configuration.models {
        if model.model_id.trim().is_empty()
            || !identities.insert(model.model_id.clone())
            || !genie_x_digest(&model.content_sha256)
            || model.devices.is_empty()
            || model.files.is_empty()
            || model.files.len() > 128
        {
            return Err(genie_x_config_error());
        }
        let (quantization, resolved) = store
            .get_paths(&model.selector)
            .map_err(|_| genie_x_config_error())?;
        let model_directory = resolved.model_dir.canonicalize()?;
        if !model_directory.starts_with(configuration.data.join("models")) {
            return Err(genie_x_config_error());
        }
        let mut component_identity = BTreeMap::new();
        let mut verified_file_paths = BTreeMap::new();
        for (relative, expected) in &model.files {
            if !genie_x_relative(relative) {
                return Err(genie_x_config_error());
            }
            let actual = model_directory.join(relative).canonicalize()?;
            if !actual.starts_with(&model_directory) {
                return Err(genie_x_config_error());
            }
            genie_x_verify_file(&baseline, &actual, expected)?;
            verified_file_paths.insert(relative.clone(), actual);
            component_identity.insert(
                relative
                    .to_str()
                    .ok_or_else(genie_x_config_error)?
                    .replace('\\', "/"),
                serde_json::json!({"bytes": expected.bytes, "sha256": expected.sha256}),
            );
        }
        let manifest_path = verified_file_paths.get(Path::new("geniex.json"))
            .ok_or_else(genie_x_config_error)?;
        let manifest: serde_json::Value = serde_json::from_slice(&genie_x_baseline_bytes(&baseline, manifest_path)?)
            .map_err(|_| genie_x_config_error())?;
        if manifest
            .get("ModelType")
            .and_then(serde_json::Value::as_str)
            != Some("llm")
            || manifest.get("Name").and_then(serde_json::Value::as_str)
                != Some(
                    model
                        .selector
                        .split(':')
                        .next()
                        .ok_or_else(genie_x_config_error)?,
                )
            || manifest.get("PluginId").and_then(serde_json::Value::as_str)
                != Some(resolved.plugin_id.as_str())
        {
            return Err(genie_x_config_error());
        }
        let required = genie_x_manifest_files(&manifest, &quantization)?;
        if required != model.files.keys().cloned().collect() {
            return Err(genie_x_config_error());
        }
        let primary_artifact = resolved.model_path.canonicalize()?;
        let primary_relative = primary_artifact
            .strip_prefix(&model_directory)
            .map_err(|_| genie_x_config_error())?;
        let primary = model
            .files
            .get(primary_relative)
            .ok_or_else(genie_x_config_error)?;
        let mut compiled_context = None;
        let digest = match model.engine {
            Engine::GenieXQairt
                if resolved.plugin_id == "qairt"
                    && model.devices.iter().all(|device| *device == Device::Npu) =>
            {
                let config_path = PathBuf::from("genie_config.json");
                if !required.contains(&config_path) {
                    return Err(genie_x_config_error());
                }
                let config_bytes = &model.files[&config_path];
                if config_bytes.bytes > 1_048_576 {
                    return Err(genie_x_config_error());
                }
                let config: serde_json::Value =
                    serde_json::from_slice(&genie_x_baseline_bytes(&baseline, verified_file_paths.get(&config_path)
                        .ok_or_else(genie_x_config_error)?)?)
                        .map_err(|_| genie_x_config_error())?;
                if config
                    .pointer("/dialog/engine/backend/type")
                    .and_then(serde_json::Value::as_str)
                    != Some("QnnHtp")
                {
                    return Err(genie_x_config_error());
                }
                compiled_context = config
                    .pointer("/dialog/context/size")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok());
                if compiled_context.is_none_or(|value| value == 0) {
                    return Err(genie_x_config_error());
                }
                let binaries = config
                    .pointer("/dialog/engine/model/binary/ctx-bins")
                    .and_then(serde_json::Value::as_array)
                    .ok_or_else(genie_x_config_error)?;
                if binaries.is_empty() {
                    return Err(genie_x_config_error());
                }
                for reference in binaries.iter().map(Some).chain([
                    config.pointer("/dialog/tokenizer/path"),
                    config.pointer("/dialog/engine/backend/extensions"),
                ]) {
                    let path = reference
                        .and_then(serde_json::Value::as_str)
                        .map(PathBuf::from)
                        .ok_or_else(genie_x_config_error)?;
                    if !genie_x_relative(&path) || !required.contains(&path) {
                        return Err(genie_x_config_error());
                    }
                }
                crate::flight_recorder::canonical_json_sha256_hex(&serde_json::json!({
                    "schema": "handshake.geniex.qairt-bundle@1", "files": component_identity,
                }))
            }
            Engine::GenieXLlamaCpp
                if resolved.plugin_id == "llama_cpp" && quantization == "Q4_0" =>
            {
                primary.sha256.clone()
            }
            _ => return Err(genie_x_config_error()),
        };
        if digest != model.content_sha256 {
            return Err(genie_x_config_error());
        }
        // Resolve again while the verified manifest and every selected file are leased.
        let (quantization_after, resolved_after) = store
            .get_paths(&model.selector)
            .map_err(|_| genie_x_config_error())?;
        if quantization_after != quantization
            || resolved_after.model_path.canonicalize()? != primary_artifact
            || resolved_after.model_dir.canonicalize()? != model_directory
            || resolved_after.plugin_id != resolved.plugin_id
        {
            return Err(genie_x_config_error());
        }
        artifacts.push(GenieXVerifiedArtifact {
            model_id: model.model_id.clone(),
            engine: model.engine,
            primary_artifact,
            model_directory,
            compiled_context,
            binding: HostBinding {
                executable: configuration.executable.clone(),
                cwd: configuration.cwd.clone(),
                data: configuration.data.clone(),
                model_selector: model.selector.clone(),
                content_sha256: digest,
                capabilities: model
                    .devices
                    .iter()
                    .map(|device| RuntimeCapability {
                        engine: model.engine,
                        device: *device,
                    })
                    .collect(),
                environment: configuration
                    .environment
                    .iter()
                    .map(|(key, value)| (OsString::from(key), OsString::from(value)))
                    .collect(),
            },
        });
    }
    Ok((
        GenieXVerifiedConfiguration {
            configuration,
            policy_source,
            temporary_directories,
            artifacts,
            baseline,
        },
        reservations,
    ))
}

struct ScopedGenieXHostResolver {
    verified: Arc<GenieXVerifiedConfiguration>,
    scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
    policy: crate::swarm_orchestration::production_factory::GenieXExecutionPolicy,
    policy_source: crate::kernel::local_execution_authority::LocalExecutionPolicySource,
    sandbox_adapter: Arc<dyn crate::sandbox::SandboxAdapter>,
    stopped: std::sync::atomic::AtomicBool,
}

impl ScopedGenieXHostResolver {
    fn new(
        verified: Arc<GenieXVerifiedConfiguration>,
        scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
    ) -> crate::swarm_orchestration::SwarmResult<Arc<Self>> {
        use crate::swarm_orchestration::production_factory::GenieXExecutionPolicy;
        let config = &verified.configuration;
        let policy_source = verified.policy_source.clone();
        let sandbox_adapter = sandbox_registry.get(&crate::sandbox::AdapterId::new(
            crate::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID,
        )).ok_or_else(|| crate::swarm_orchestration::SwarmError::FactoryFailed(
            "configured native Sandbox adapter is unavailable".into(),
        ))?;
        let bound_adapter = crate::sandbox::HandshakeNativeSandboxAdapter::with_baseline_bindings(
            verified.baseline.clone(),
        );
        if sandbox_adapter.capabilities() != crate::sandbox::SandboxAdapter::capabilities(&bound_adapter) {
            return Err(crate::swarm_orchestration::SwarmError::FactoryFailed(
                "configured native Sandbox adapter capabilities changed".into(),
            ));
        }
        let sandbox_adapter: Arc<dyn crate::sandbox::SandboxAdapter> = Arc::new(bound_adapter);
        let supervisor = hsk_studio_worker::Supervisor::new(hsk_studio_worker::SupervisorConfig {
            program: config.executable.clone(),
            args: Vec::new(),
            env: config
                .environment
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            max_workers: config.max_workers,
            max_in_flight_bytes: config.max_in_flight_bytes,
            spawn_timeout: std::time::Duration::from_millis(config.spawn_timeout_ms),
            os_limits: None,
            reply_buffer: Some(1),
            io_thread_name: "handshake-geniex-io".into(),
        });
        let policy = GenieXExecutionPolicy::new(
            supervisor,
            hsk_studio_worker::OutputBounds {
                chunk_bytes: std::num::NonZeroUsize::new(config.chunk_bytes).ok_or_else(|| {
                    crate::swarm_orchestration::SwarmError::FactoryFailed(
                        "invalid GenieX chunk bound".into(),
                    )
                })?,
                stdout_bytes: config.stdout_bytes,
                stderr_bytes: config.stderr_bytes,
            },
            std::time::Duration::from_millis(config.operation_timeout_ms),
        )?;
        Ok(Arc::new(Self {
            verified,
            scope,
            policy,
            policy_source,
            sandbox_adapter,
            stopped: std::sync::atomic::AtomicBool::new(false),
        }))
    }
}

impl crate::swarm_orchestration::production_factory::GenieXHostBindingResolver
    for ScopedGenieXHostResolver
{
    fn resource_scope(
        &self,
    ) -> &crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution {
        &self.scope
    }

    fn execution_policy(
        &self,
    ) -> crate::swarm_orchestration::production_factory::GenieXExecutionPolicy {
        self.policy.clone()
    }

    fn local_execution_policy_source(&self)
        -> &crate::kernel::local_execution_authority::LocalExecutionPolicySource {
        &self.policy_source
    }

    fn sandbox_adapter(&self) -> Arc<dyn crate::sandbox::SandboxAdapter> {
        self.sandbox_adapter.clone()
    }

    fn sandbox_binds(
        &self,
        request: &crate::swarm_orchestration::SpawnRequest,
        snapshot: &crate::model_runtime::EngineProfileSnapshotV1,
    ) -> crate::swarm_orchestration::SwarmResult<Vec<crate::sandbox::BindSpec>> {
        use crate::sandbox::{BindMode, BindSpec};
        let rejected = || crate::swarm_orchestration::SwarmError::FactoryFailed(
            "verified GenieX Sandbox paths rejected".into(),
        );
        let binding = self.resolve(request, snapshot)?;
        let artifact = self.verified.artifacts.iter()
            .find(|artifact| artifact.model_id == snapshot.profile.model.id)
            .ok_or_else(rejected)?;
        let mut paths = BTreeMap::from([
            (binding.executable.clone(), BindMode::ReadOnly),
            (binding.cwd.clone(), BindMode::ReadOnly),
            (artifact.model_directory.clone(), BindMode::ReadOnly),
        ]);
        // Grant only configured runtime/model roots and explicit host temporary
        // directories. No requested workspace or ambient environment is added.
        for temporary in &self.verified.temporary_directories {
            if paths.insert(temporary.clone(), BindMode::ReadWrite)
                .is_some_and(|mode| mode != BindMode::ReadWrite) {
                return Err(rejected());
            }
        }
        Ok(paths.into_iter().map(|(host_path, mode)| BindSpec {
            guest_path: host_path.clone(), host_path, mode,
        }).collect())
    }

    fn resolve(
        &self,
        _request: &crate::swarm_orchestration::SpawnRequest,
        snapshot: &crate::model_runtime::EngineProfileSnapshotV1,
    ) -> crate::swarm_orchestration::SwarmResult<HostBinding> {
        let rejected = || {
            crate::swarm_orchestration::SwarmError::FactoryFailed(
                "GenieX profile does not match the verified scoped configuration".into(),
            )
        };
        if self.stopped.load(std::sync::atomic::Ordering::Acquire) {
            return Err(rejected());
        }
        snapshot.validate().map_err(|_| rejected())?;
        let profile = &snapshot.profile;
        let artifact = self
            .verified
            .artifacts
            .iter()
            .find(|artifact| {
                artifact.model_id == profile.model.id
                    && artifact.engine == profile.engine
                    && artifact.binding.content_sha256 == profile.model.content_sha256
                    && artifact.compiled_context == profile.model.compiled_context
                    && artifact.binding.capabilities.contains(&RuntimeCapability {
                        engine: profile.engine,
                        device: profile.device,
                    })
            })
            .ok_or_else(rejected)?;
        Ok(artifact.binding.clone())
    }
}

type GenieXStartAck = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<(), crate::process_ledger::ProcessLedgerError>>
            + Send,
    >,
>;

struct GenieXBootModel {
    model: crate::swarm_orchestration::production_factory::GenieXConfiguredModel,
    lifecycle: Option<crate::process_ledger::ActiveProcessLifecycle>,
    start_ack: Option<GenieXStartAck>,
    start_result: Option<Result<(), String>>,
    retired: bool,
}

struct GenieXBootOwner {
    resolver: Arc<ScopedGenieXHostResolver>,
    config: crate::swarm_orchestration::production_factory::GenieXFactoryConfig,
    models: Vec<GenieXBootModel>,
    reservations: Vec<crate::process_ledger::ReservedProcessLifecycle>,
    catalog: Arc<crate::model_runtime::ModelCatalog>,
}

pub(super) struct GenieXBootClient {
    inner: Arc<dyn crate::llm::LlmClient>,
    owner: tokio::sync::Mutex<GenieXBootOwner>,
    resolver: Arc<ScopedGenieXHostResolver>,
    catalog: Arc<crate::model_runtime::ModelCatalog>,
    authority: crate::model_runtime::ScopedModelRegistryAuthority,
}

impl GenieXBootClient {
    pub(super) async fn prepare(
        path: PathBuf,
        inner: Arc<dyn crate::llm::LlmClient>,
        authority: crate::model_runtime::ScopedModelRegistryAuthority,
        ledger: crate::process_ledger::LedgerBatcher,
        recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
        sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
    ) -> Result<Arc<Self>, String> {
        let (verified, reservations) =
            tokio::task::spawn_blocking(move || load_genie_x_configuration(&path, &ledger))
                .await
                .map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())?;
        let owner = GenieXBootOwner::new(
            verified,
            reservations,
            authority.scope().clone(),
            recorder,
            inner.model_catalog(),
            sandbox_registry,
        )?;
        Ok(Arc::new(Self {
            inner,
            resolver: owner.resolver.clone(),
            catalog: owner.catalog.clone(),
            owner: tokio::sync::Mutex::new(owner),
            authority,
        }))
    }

    pub(super) async fn initialize(
        &self,
        ledger: crate::process_ledger::LedgerBatcher,
    ) -> Result<(), String> {
        self.owner
            .lock()
            .await
            .initialize(&self.authority, ledger)
            .await
    }

    pub(super) fn binding_resolver(
        &self,
    ) -> Arc<dyn crate::swarm_orchestration::production_factory::GenieXHostBindingResolver> {
        self.resolver.clone()
    }
}

#[async_trait::async_trait]
impl crate::llm::LlmClient for GenieXBootClient {
    fn owned_completion(
        self: Arc<Self>,
        req: crate::llm::CompletionRequest,
        deadline: tokio::time::Instant,
    ) -> crate::llm::OwnedLlmCompletion {
        self.inner.clone().owned_completion(req, deadline)
    }
    async fn completion(
        &self,
        req: crate::llm::CompletionRequest,
    ) -> Result<crate::llm::CompletionResponse, crate::llm::LlmError> {
        self.inner.completion(req).await
    }
    async fn completion_with_control(
        &self,
        req: crate::llm::CompletionRequest,
        control: crate::llm::LlmInvocationControl,
    ) -> Result<crate::llm::CompletionResponse, crate::llm::LlmError> {
        self.inner.completion_with_control(req, control).await
    }
    fn stream_completion(
        self: Arc<Self>,
        req: crate::model_runtime::GenerateRequest,
    ) -> crate::model_runtime::TokenStream {
        self.inner.clone().stream_completion(req)
    }
    fn stream_completion_with_context(
        self: Arc<Self>,
        req: crate::model_runtime::GenerateRequest,
        context: crate::llm::LlmInvocationContext,
    ) -> crate::model_runtime::TokenStream {
        self.inner
            .clone()
            .stream_completion_with_context(req, context)
    }
    async fn embedding(
        &self,
        req: crate::llm::EmbeddingRequest,
    ) -> Result<crate::llm::EmbeddingResponse, crate::llm::LlmError> {
        self.inner.embedding(req).await
    }
    async fn score(
        &self,
        id: crate::model_runtime::ModelId,
        sequence: Vec<u32>,
    ) -> Result<crate::model_runtime::Score, crate::llm::LlmError> {
        self.inner.score(id, sequence).await
    }
    async fn control_model_runtime(
        &self,
        req: crate::llm::ModelRuntimeControlRequest,
    ) -> Result<crate::llm::ModelRuntimeControlReceipt, crate::llm::LlmError> {
        self.inner.control_model_runtime(req).await
    }
    fn scoped_model_registry_authority(
        &self,
    ) -> Option<crate::model_runtime::ScopedModelRegistryAuthority> {
        Some(self.authority.clone())
    }
    fn model_runtime_control_capabilities(
        &self,
        model_id: &str,
    ) -> crate::llm::ModelRuntimeControlCapabilities {
        self.inner.model_runtime_control_capabilities(model_id)
    }
    fn cancel(&self, model_id: &str, token: crate::model_runtime::CancellationToken) {
        self.inner.cancel(model_id, token);
    }
    async fn swap_model(
        &self,
        req: crate::workflows::ModelSwapRequestV0_4,
    ) -> Result<(), crate::llm::LlmError> {
        self.inner.swap_model(req).await
    }
    fn profile(&self) -> &crate::llm::ModelProfile {
        self.inner.profile()
    }
    fn selected_model_id(&self) -> String {
        self.inner.selected_model_id()
    }
    fn model_catalog(&self) -> Option<Arc<crate::model_runtime::ModelCatalog>> {
        Some(self.catalog.clone())
    }
    fn inspect_model_runtime(&self, model_id: &str) -> crate::llm::ModelRuntimeInspection {
        self.inner.inspect_model_runtime(model_id)
    }
    fn shutdown(&self) {
        self.resolver
            .stopped
            .store(true, std::sync::atomic::Ordering::Release);
        self.inner.shutdown();
    }
    fn leave_open_for_reconciliation(&self) {
        self.resolver
            .stopped
            .store(true, std::sync::atomic::Ordering::Release);
        self.inner.leave_open_for_reconciliation();
    }
    async fn shutdown_gracefully(&self) -> Result<(), crate::llm::LlmError> {
        self.resolver
            .stopped
            .store(true, std::sync::atomic::Ordering::Release);
        self.owner
            .lock()
            .await
            .retire()
            .await
            .map_err(crate::llm::LlmError::ProviderError)?;
        self.inner.shutdown_gracefully().await
    }
}

impl GenieXBootModel {
    async fn await_start(&mut self, timeout: std::time::Duration) -> Result<(), String> {
        if let Some(result) = &self.start_result {
            return result.clone();
        }
        let pending = self
            .start_ack
            .as_mut()
            .ok_or("GenieX START acknowledgement is absent")?;
        let result = tokio::time::timeout(timeout, pending)
            .await
            .map_err(|_| "GenieX START durability is still unconfirmed".to_string())?
            .map_err(|error| error.to_string());
        self.start_ack = None;
        self.start_result = Some(result.clone());
        result
    }
}

impl GenieXBootOwner {
    fn new(
        verified: GenieXVerifiedConfiguration,
        reservations: Vec<crate::process_ledger::ReservedProcessLifecycle>,
        scope: crate::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
        recorder: Arc<dyn crate::flight_recorder::FlightRecorder>,
        existing_catalog: Option<Arc<crate::model_runtime::ModelCatalog>>,
        sandbox_registry: Arc<crate::sandbox::SandboxAdapterRegistry>,
    ) -> Result<Self, String> {
        let resolver = ScopedGenieXHostResolver::new(Arc::new(verified), scope, sandbox_registry)
            .map_err(|error| error.to_string())?;
        let config = crate::swarm_orchestration::production_factory::GenieXFactoryConfig::new(
            resolver.clone(),
            recorder,
        );
        Ok(Self {
            resolver,
            config,
            models: Vec::new(),
            reservations,
            catalog: existing_catalog.unwrap_or_else(crate::model_runtime::ModelCatalog::empty),
        })
    }

    async fn initialize(
        &mut self,
        authority: &crate::model_runtime::ScopedModelRegistryAuthority,
        ledger: crate::process_ledger::LedgerBatcher,
    ) -> Result<(), String> {
        use crate::model_runtime::{
            BaseModelTag, KvCachePolicy, LoadSpec, ModelCapabilities, ModelRegistration,
            OperatorId, ProviderKind, RoleBoundModelRegistration, RuntimeBinding, RuntimeKind,
            SamplingParams,
        };
        use crate::process_ledger::{ProcessEngineKind, ProcessStart};
        if authority.scope() != &self.resolver.scope || !self.models.is_empty() {
            return Err("GenieX boot scope or initialization state mismatch".into());
        }
        // The activation has not been published. No runtime mutator can race this snapshot.
        let (mut whole, _) = self
            .catalog
            .snapshot_role_bound_registrations()
            .map_err(|error| error.to_string())?;
        let mut additions = Vec::new();
        for artifact in &self.resolver.verified.artifacts {
            let reservation = self
                .reservations
                .pop()
                .ok_or("GenieX lifecycle reservation is absent")?;
            let scope = &self.resolver.scope;
            let mut start = ProcessStart::new(
                ProcessEngineKind::GenieX,
                "handshake-configured-model",
                None,
            )
            .with_model_artifact_sha256(artifact.binding.content_sha256.clone())
            .with_metadata_jsonb(serde_json::json!({
                "lifecycle_kind": "in_process_genie_x_configured_adapter",
                "execution_model": "on_demand_one_shot_no_residency_claim",
                "runtime_binding": "genie_x",
                "owner_account_id": scope.owner_account_id.as_uuid(),
                "actor_principal_id": scope.actor_principal_id.as_uuid(),
                "authenticated_session_id": scope.authenticated_session_id.as_uuid(),
                "access_space_id": scope.access_space_id.as_uuid(),
                "workspace_id": scope.workspace_id.as_str(),
            }));
            let artifact_sha256: [u8; 32] = hex::decode(&artifact.binding.content_sha256)
                .map_err(|_| "GenieX digest rejected")?
                .try_into()
                .map_err(|_| "GenieX digest length rejected")?;
            let base_model_tag =
                BaseModelTag::try_new(&artifact.model_id).map_err(|error| error.to_string())?;
            let registered_by = OperatorId::try_new("handshake-configured-model")
                .map_err(|error| error.to_string())?;
            let model = self
                .config
                .load_configured_model(
                    artifact.binding.clone(),
                    LoadSpec {
                        artifact_path: artifact.primary_artifact.clone(),
                        sha256_expected: artifact.binding.content_sha256.clone(),
                        runtime_kind: RuntimeKind::GenieX,
                        sampling_defaults: SamplingParams::default(),
                        kv_cache_policy: KvCachePolicy::Disabled,
                        declared_capabilities: ModelCapabilities::default(),
                        provider: ProviderKind::Local,
                        engine_origin: Some(
                            crate::model_runtime::invariant::GENIE_X_LOCAL_ENGINE_ORIGIN.into(),
                        ),
                        external_engine_import: None,
                    },
                    ledger.clone(),
                    "handshake-configured-adapter".into(),
                    start.clone(),
                    self.resolver.verified.configuration.admission_bytes,
                )
                .await
                .map_err(|error| error.to_string())?;
            self.models.push(GenieXBootModel {
                model,
                lifecycle: None,
                start_ack: None,
                start_result: None,
                retired: false,
            });
            let owned = self
                .models
                .last_mut()
                .ok_or("GenieX boot ownership is absent")?;
            start.process_uuid = owned.model.model_id.as_uuid();
            start.metadata_jsonb["model_id"] = serde_json::json!(owned.model.model_id.to_string());
            let registration = ModelRegistration {
                model_id: owned.model.model_id,
                artifact_path: artifact.primary_artifact.clone(),
                sha256: artifact_sha256,
                runtime_binding: RuntimeBinding::GenieX,
                declared_capabilities: owned
                    .model
                    .runtime
                    .capabilities(owned.model.model_id)
                    .map_err(|error| error.to_string())?
                    .clone(),
                base_model_tag,
                registered_at_utc: chrono::Utc::now(),
                registered_by,
                provider: ProviderKind::Local,
            };
            let (lifecycle, ack) = reservation
                .begin_with_durable_ack(start)
                .map_err(|error| error.to_string())?;
            owned.lifecycle = Some(lifecycle);
            owned.start_ack = Some(Box::pin(ack.wait_unbounded()));
            owned
                .await_start(std::time::Duration::from_secs(30))
                .await?;
            additions.push(RoleBoundModelRegistration::completion(registration));
        }
        whole.extend_from_slice(&additions);
        let persisted = authority
            .store()
            .persist_role_bound_boot_set_and_read_back(authority.scope(), &whole)
            .await
            .map_err(|error| error.to_string())?;
        if persisted.len() != whole.len()
            || whole.iter().any(|item| {
                persisted
                    .iter()
                    .filter(|row| {
                        row.last_observed_runtime_model_id == item.registration.model_id
                            && row.selection() == item.selection()
                            && row.lifecycle_state
                                == crate::model_runtime::ModelRegistryLifecycleState::Active
                    })
                    .count()
                    != 1
            })
        {
            return Err(
                "GenieX whole boot registry readback differs from the loaded role-bound set".into(),
            );
        }
        let loaded = self
            .models
            .iter()
            .map(|owned| (owned.model.model_id, owned.model.runtime.clone()))
            .collect::<Vec<_>>();
        self.catalog
            .publish_loaded_registrations(&additions, &loaded)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    async fn retire(&mut self) -> Result<(), String> {
        // Existing coordinator sessions are drained by the enclosing runtime before this call.
        for owned in &mut self.models {
            if !owned.retired {
                owned
                    .model
                    .retire()
                    .await
                    .map_err(|error| error.to_string())?;
                owned.retired = true;
            }
        }
        self.config
            .drain_pending_operations()
            .await
            .map_err(|error| error.to_string())?;
        for owned in &mut self.models {
            if owned.lifecycle.is_some() {
                if owned.start_result.as_ref() != Some(&Ok(())) {
                    owned
                        .await_start(std::time::Duration::from_secs(30))
                        .await?;
                }
                let lifecycle = owned
                    .lifecycle
                    .as_ref()
                    .ok_or("GenieX lifecycle disappeared")?;
                let result = lifecycle
                    .stop_with_durable_ack(
                        None,
                        "configured adapter unloaded",
                        std::time::Duration::from_secs(30),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                if !matches!(
                    result,
                    crate::process_ledger::StopRecordOutcome::Recorded
                        | crate::process_ledger::StopRecordOutcome::AlreadyStopped
                ) {
                    return Err("GenieX adapter STOP durability remains unconfirmed".into());
                }
            }
        }
        Ok(())
    }
}
