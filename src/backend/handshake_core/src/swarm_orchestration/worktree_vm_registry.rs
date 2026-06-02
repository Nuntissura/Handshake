//! WP-KERNEL-004 wave 1: per-worktree microVM binding + snapshot/restore STATE
//! RECOVERY seam.
//!
//! [`WorktreeVmRegistry`] binds a `worktree_id` to a PERSISTENT Cloud Hypervisor
//! microVM (booted with `hsk.sandbox.mode=persistent` so it stays live with an
//! API socket for `ch-remote pause` + `snapshot`), and exposes
//! [`WorktreeVmRegistry::snapshot`] / [`WorktreeVmRegistry::restore`] so a
//! worktree VM's full live state can be checkpointed and resumed across app
//! restarts. The TOCTOU clone-safety the adapter already enforces (single live
//! clone per snapshot; reservation released on every failure path) is REUSED
//! unchanged — this seam adds no new clone-safety code.
//!
//! ## Wave 1 boundary
//!
//! This now lands a REACHABLE, fake-adapter-tested snapshot/restore seam plus
//! warm-start manifests that bind a snapshot to the warm-agent protocol version,
//! ready nonce, guest model path, and model artifact hash. Serving `generate()`
//! from a restored warm VM with no model reload still requires the live
//! serial/vsock guest transport and a model-bearing guest image; the registry
//! prevents stale snapshot reuse but does not fake live token generation.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::model_runtime::{
    validate_ready_frame, WarmAgentGuestFrame, WarmAgentProtocolError, WarmVmSnapshotManifest,
};
use crate::sandbox::{
    AdapterId, ImageRef, NetPolicy, ProcessHandle, ProcessSpec, ResourceLimits, SandboxAdapter,
    SandboxAdapterError, Signal, SnapshotRef, TrustClass, SANDBOX_MODE_METADATA_KEY,
    SANDBOX_MODE_PERSISTENT,
};

/// Error type for the worktree VM registry. Wraps the adapter error plus the
/// "no VM bound for this worktree" lookup miss.
#[derive(Debug, thiserror::Error)]
pub enum WorktreeVmError {
    #[error("no microVM is bound to worktree `{worktree_id}`; call ensure_worktree_vm first")]
    NotBound { worktree_id: String },
    #[error(transparent)]
    WarmAgent(#[from] WarmAgentProtocolError),
    #[error(transparent)]
    Sandbox(#[from] SandboxAdapterError),
}

/// Binds `worktree_id` -> a persistent microVM handle, with snapshot/restore.
pub struct WorktreeVmRegistry {
    adapter: Arc<dyn SandboxAdapter>,
    persistent: Mutex<HashMap<String, ProcessHandle>>,
}

impl WorktreeVmRegistry {
    /// Construct a registry over an injected sandbox adapter (the real
    /// `CloudHypervisorAdapter` in production, a fake in tests).
    pub fn new(adapter: Arc<dyn SandboxAdapter>) -> Self {
        Self {
            adapter,
            persistent: Mutex::new(HashMap::new()),
        }
    }

    /// The persistent-VM [`ProcessSpec`] for a worktree: marks
    /// `hsk.sandbox.mode=persistent` so `spawn` boots a long-lived idle VM with
    /// an API socket (the only mode `snapshot`/`restore` accept). DenyAll net
    /// (CH microVMs have no network device); `UntrustedAgent` trust forces the
    /// Tier-3 minimum at selection.
    fn worktree_spec(worktree_id: &str) -> ProcessSpec {
        let mut metadata = std::collections::BTreeMap::new();
        metadata.insert(
            SANDBOX_MODE_METADATA_KEY.to_string(),
            SANDBOX_MODE_PERSISTENT.to_string(),
        );
        ProcessSpec {
            id: AdapterId::new(format!("worktree-vm:{worktree_id}")),
            image_or_root: ImageRef::new("worktree_idle"),
            cmd: vec![],
            env: std::collections::BTreeMap::new(),
            cwd: None,
            binds: vec![],
            net_policy: NetPolicy::DenyAll,
            resource_limits: ResourceLimits::default(),
            idle_timeout_ms: None,
            required_capabilities: std::collections::BTreeSet::new(),
            trust_class: TrustClass::UntrustedAgent,
            metadata,
        }
    }

    /// Boot (or return the already-bound) persistent microVM for `worktree_id`.
    /// Idempotent: a second call for the same worktree returns the existing
    /// handle rather than booting a second VM.
    pub async fn ensure_worktree_vm(
        &self,
        worktree_id: &str,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        {
            let map = self.persistent.lock().await;
            if let Some(handle) = map.get(worktree_id) {
                return Ok(handle.clone());
            }
        }
        let spec = Self::worktree_spec(worktree_id);
        let handle = self.adapter.spawn(spec).await?;
        self.persistent
            .lock()
            .await
            .insert(worktree_id.to_string(), handle.clone());
        Ok(handle)
    }

    /// Snapshot the worktree's persistent VM (Master Spec §3.5.7 #7). Looks up
    /// the bound handle and calls `adapter.snapshot`, returning the
    /// [`SnapshotRef`] (config.json + state.json + memory dir; carries the
    /// serial `observe_path` for resume confirmation).
    pub async fn snapshot(&self, worktree_id: &str) -> Result<SnapshotRef, WorktreeVmError> {
        let handle = {
            let map = self.persistent.lock().await;
            map.get(worktree_id)
                .cloned()
                .ok_or_else(|| WorktreeVmError::NotBound {
                    worktree_id: worktree_id.to_string(),
                })?
        };
        Ok(self.adapter.snapshot(&handle).await?)
    }

    /// Restore a previously captured snapshot into a fresh microVM and REBIND
    /// the worktree to the restored handle. Reuses the adapter's TOCTOU
    /// clone-safety unchanged (it refuses a second concurrent restore of the
    /// same snapshot). After this returns, the worktree's bound handle is the
    /// restored VM.
    pub async fn restore(
        &self,
        worktree_id: &str,
        snapshot: &SnapshotRef,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        let handle = self.adapter.restore(snapshot).await?;
        self.persistent
            .lock()
            .await
            .insert(worktree_id.to_string(), handle.clone());
        Ok(handle)
    }

    /// Snapshot a worktree VM after its in-guest warm agent has reported a
    /// loaded model. The returned manifest binds the raw VM snapshot to the
    /// warm-agent protocol version, ready nonce, guest model path, and model
    /// artifact hash. A later restore validates this manifest before rebinding
    /// the worktree so stale snapshots cannot masquerade as usable warm model
    /// state.
    pub async fn snapshot_warm_model(
        &self,
        worktree_id: &str,
        model_artifact_sha256: &str,
        model_guest_path: &str,
        ready: &WarmAgentGuestFrame,
    ) -> Result<WarmVmSnapshotManifest, WorktreeVmError> {
        validate_ready_frame(ready)?;
        let (ready_nonce, loaded_model_sha256, loaded_model_guest_path) = match ready {
            WarmAgentGuestFrame::Ready {
                ready_nonce,
                loaded_model_sha256,
                loaded_model_guest_path,
                ..
            } => (
                ready_nonce.as_str(),
                loaded_model_sha256.as_deref(),
                loaded_model_guest_path.as_deref(),
            ),
            _ => unreachable!("validate_ready_frame rejects non-ready frames"),
        };
        if loaded_model_sha256 != Some(model_artifact_sha256) {
            return Err(WarmAgentProtocolError::ModelHashMismatch {
                expected: model_artifact_sha256.to_string(),
                actual: loaded_model_sha256.unwrap_or("<missing>").to_string(),
            }
            .into());
        }
        if loaded_model_guest_path != Some(model_guest_path) {
            return Err(WarmAgentProtocolError::ModelGuestPathMismatch {
                expected: model_guest_path.to_string(),
                actual: loaded_model_guest_path.unwrap_or("<missing>").to_string(),
            }
            .into());
        }
        let snapshot = self.snapshot(worktree_id).await?;
        Ok(WarmVmSnapshotManifest::new(
            worktree_id,
            model_artifact_sha256,
            model_guest_path,
            ready_nonce,
            snapshot,
        ))
    }

    /// Restore a warm-model snapshot only when its protocol, model hash, and
    /// guest model path still match the requested artifact. This is the
    /// warm-start guardrail: restored process state is usable only after the
    /// manifest proves it was captured from the same model identity and guest
    /// location that the caller is about to serve.
    pub async fn restore_warm_model(
        &self,
        manifest: &WarmVmSnapshotManifest,
        expected_model_artifact_sha256: &str,
        expected_model_guest_path: &str,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        manifest.validate_for_restore(expected_model_artifact_sha256, expected_model_guest_path)?;
        self.restore(&manifest.worktree_id, &manifest.snapshot)
            .await
    }

    /// Tear down the worktree's bound VM (best-effort kill) and unbind it.
    pub async fn teardown_worktree_vm(&self, worktree_id: &str) -> Result<(), WorktreeVmError> {
        let handle = self.persistent.lock().await.remove(worktree_id);
        if let Some(handle) = handle {
            self.adapter.kill(&handle, Signal::Term).await?;
        }
        Ok(())
    }

    /// Whether a microVM is currently bound to `worktree_id`.
    pub async fn is_bound(&self, worktree_id: &str) -> bool {
        self.persistent.lock().await.contains_key(worktree_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_runtime::{WARM_AGENT_PROTOCOL_ID, WARM_AGENT_PROTOCOL_VERSION};
    use crate::sandbox::{
        AdapterCapabilities, BindMode, Command, ExecResult, GpuPassthrough, IsolationStrength,
        IsolationTier, ProcessStatus, ThroughputClass,
    };
    use async_trait::async_trait;
    use std::path::PathBuf;
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct Obs {
        spawn_count: usize,
        snapshot_called: bool,
        restore_called: bool,
        kill_called: bool,
        last_persistent_marker: Option<String>,
    }

    struct FakeVmAdapter {
        obs: Arc<StdMutex<Obs>>,
    }

    #[async_trait]
    impl SandboxAdapter for FakeVmAdapter {
        async fn spawn(&self, spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
            let mut o = self.obs.lock().unwrap();
            o.spawn_count += 1;
            o.last_persistent_marker = spec.metadata.get(SANDBOX_MODE_METADATA_KEY).cloned();
            Ok(ProcessHandle::new(
                AdapterId::new("cloud_hypervisor"),
                None,
                format!("hsk-ch-persistent-{}", o.spawn_count),
            ))
        }
        async fn exec(
            &self,
            _handle: &ProcessHandle,
            _cmd: Command,
        ) -> Result<ExecResult, SandboxAdapterError> {
            Ok(ExecResult {
                exit_code: 0,
                stdout: bytes::Bytes::new(),
                stderr: bytes::Bytes::new(),
                duration_ms: 0,
            })
        }
        async fn fs_bind(
            &self,
            _handle: &ProcessHandle,
            _host_path: PathBuf,
            _guest_path: PathBuf,
            _mode: BindMode,
        ) -> Result<(), SandboxAdapterError> {
            Ok(())
        }
        async fn net_policy(
            &self,
            _handle: &ProcessHandle,
            _policy: NetPolicy,
        ) -> Result<(), SandboxAdapterError> {
            Ok(())
        }
        async fn kill(
            &self,
            _handle: &ProcessHandle,
            _signal: Signal,
        ) -> Result<(), SandboxAdapterError> {
            self.obs.lock().unwrap().kill_called = true;
            Ok(())
        }
        async fn status(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<ProcessStatus, SandboxAdapterError> {
            Ok(ProcessStatus::Running)
        }
        async fn exit_code(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<Option<i32>, SandboxAdapterError> {
            Ok(None)
        }
        async fn snapshot(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<SnapshotRef, SandboxAdapterError> {
            self.obs.lock().unwrap().snapshot_called = true;
            Ok(
                SnapshotRef::new(AdapterId::new("cloud_hypervisor"), "/fake/snap")
                    .with_observe_path("/fake/serial.log"),
            )
        }
        async fn restore(
            &self,
            _snapshot: &SnapshotRef,
        ) -> Result<ProcessHandle, SandboxAdapterError> {
            self.obs.lock().unwrap().restore_called = true;
            Ok(ProcessHandle::new(
                AdapterId::new("cloud_hypervisor"),
                None,
                "hsk-ch-restored",
            ))
        }
        fn capabilities(&self) -> AdapterCapabilities {
            AdapterCapabilities {
                adapter_id: AdapterId::new("cloud_hypervisor"),
                runtime_available: true,
                filesystem_isolation_strength: IsolationStrength::VeryStrong,
                network_isolation_strength: IsolationStrength::VeryStrong,
                gpu_passthrough: GpuPassthrough::None,
                stdio_throughput_class: ThroughputClass::Low,
                win32_native_fidelity: false,
                cross_machine_portable: true,
                isolation_tier: IsolationTier::Tier3Microvm,
                requires_nested_virt: true,
                supports_snapshot: true,
                supports_persistent_exec: false,
                supports_warm_agent: false,
                supports_live_token_stream: false,
            }
        }
    }

    fn registry() -> (WorktreeVmRegistry, Arc<StdMutex<Obs>>) {
        let obs = Arc::new(StdMutex::new(Obs::default()));
        let adapter = Arc::new(FakeVmAdapter { obs: obs.clone() });
        (WorktreeVmRegistry::new(adapter), obs)
    }

    fn ready_frame(model_hash: &str, model_guest_path: &str) -> WarmAgentGuestFrame {
        WarmAgentGuestFrame::Ready {
            protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
            protocol_version: WARM_AGENT_PROTOCOL_VERSION,
            agent_id: "warm-agent-1".to_string(),
            ready_nonce: "nonce-1".to_string(),
            loaded_model_sha256: Some(model_hash.to_string()),
            loaded_model_guest_path: Some(model_guest_path.to_string()),
        }
    }

    #[tokio::test]
    async fn ensure_boots_persistent_vm_and_is_idempotent() {
        let (reg, obs) = registry();
        let h1 = reg.ensure_worktree_vm("wt-1").await.expect("boot");
        let h2 = reg.ensure_worktree_vm("wt-1").await.expect("idempotent");
        assert_eq!(h1, h2, "second ensure returns the same handle");
        let o = obs.lock().unwrap();
        assert_eq!(o.spawn_count, 1, "exactly one VM booted for the worktree");
        assert_eq!(
            o.last_persistent_marker.as_deref(),
            Some(SANDBOX_MODE_PERSISTENT),
            "the spec carried the persistent-mode marker"
        );
    }

    #[tokio::test]
    async fn snapshot_then_restore_drives_adapter_and_rebinds() {
        let (reg, obs) = registry();
        reg.ensure_worktree_vm("wt-1").await.expect("boot");
        let snap = reg.snapshot("wt-1").await.expect("snapshot");
        assert!(
            obs.lock().unwrap().snapshot_called,
            "adapter.snapshot driven"
        );
        assert_eq!(snap.observe_path.as_deref(), Some("/fake/serial.log"));

        let restored = reg.restore("wt-1", &snap).await.expect("restore");
        assert!(obs.lock().unwrap().restore_called, "adapter.restore driven");
        assert_eq!(restored.sandbox_internal_id, "hsk-ch-restored");
        // The worktree is rebound to the restored handle.
        assert!(reg.is_bound("wt-1").await);
    }

    #[tokio::test]
    async fn warm_snapshot_manifest_restores_only_matching_model_hash() {
        let (reg, obs) = registry();
        reg.ensure_worktree_vm("wt-warm").await.expect("boot");
        let ready = ready_frame("sha-warm", "/models/model.gguf");
        let manifest = reg
            .snapshot_warm_model("wt-warm", "sha-warm", "/models/model.gguf", &ready)
            .await
            .expect("warm snapshot manifest");
        assert_eq!(manifest.worktree_id, "wt-warm");
        assert_eq!(manifest.model_artifact_sha256, "sha-warm");
        assert_eq!(manifest.model_guest_path, "/models/model.gguf");

        let restored = reg
            .restore_warm_model(&manifest, "sha-warm", "/models/model.gguf")
            .await
            .expect("matching hash restores");
        assert_eq!(restored.sandbox_internal_id, "hsk-ch-restored");
        assert!(obs.lock().unwrap().restore_called);

        obs.lock().unwrap().restore_called = false;
        let stale = reg
            .restore_warm_model(&manifest, "sha-new", "/models/model.gguf")
            .await
            .expect_err("stale model hash fails before restore");
        assert!(matches!(
            stale,
            WorktreeVmError::WarmAgent(WarmAgentProtocolError::ModelHashMismatch { .. })
        ));
        assert!(
            !obs.lock().unwrap().restore_called,
            "stale manifest must not call adapter.restore"
        );

        let stale_path = reg
            .restore_warm_model(&manifest, "sha-warm", "/models/other.gguf")
            .await
            .expect_err("stale guest path fails before restore");
        assert!(matches!(
            stale_path,
            WorktreeVmError::WarmAgent(WarmAgentProtocolError::ModelGuestPathMismatch { .. })
        ));
        assert!(
            !obs.lock().unwrap().restore_called,
            "stale guest path must not call adapter.restore"
        );
    }

    #[tokio::test]
    async fn warm_snapshot_rejects_ready_frame_mismatch_before_snapshot() {
        let (reg, obs) = registry();
        reg.ensure_worktree_vm("wt-warm").await.expect("boot");

        let stale_hash = ready_frame("sha-old", "/models/model.gguf");
        let err = reg
            .snapshot_warm_model("wt-warm", "sha-warm", "/models/model.gguf", &stale_hash)
            .await
            .expect_err("hash mismatch fails before snapshot");
        assert!(matches!(
            err,
            WorktreeVmError::WarmAgent(WarmAgentProtocolError::ModelHashMismatch { .. })
        ));
        assert!(
            !obs.lock().unwrap().snapshot_called,
            "hash mismatch must not capture a VM snapshot"
        );

        let stale_path = ready_frame("sha-warm", "/models/other.gguf");
        let err = reg
            .snapshot_warm_model("wt-warm", "sha-warm", "/models/model.gguf", &stale_path)
            .await
            .expect_err("guest path mismatch fails before snapshot");
        assert!(matches!(
            err,
            WorktreeVmError::WarmAgent(WarmAgentProtocolError::ModelGuestPathMismatch { .. })
        ));
        assert!(
            !obs.lock().unwrap().snapshot_called,
            "path mismatch must not capture a VM snapshot"
        );
    }

    #[tokio::test]
    async fn snapshot_without_bound_vm_is_typed_not_bound() {
        let (reg, _obs) = registry();
        let err = reg.snapshot("wt-missing").await.expect_err("not bound");
        assert!(matches!(err, WorktreeVmError::NotBound { .. }));
    }

    #[tokio::test]
    async fn teardown_kills_and_unbinds() {
        let (reg, obs) = registry();
        reg.ensure_worktree_vm("wt-1").await.expect("boot");
        reg.teardown_worktree_vm("wt-1").await.expect("teardown");
        assert!(obs.lock().unwrap().kill_called, "adapter.kill driven");
        assert!(
            !reg.is_bound("wt-1").await,
            "worktree unbound after teardown"
        );
    }
}
