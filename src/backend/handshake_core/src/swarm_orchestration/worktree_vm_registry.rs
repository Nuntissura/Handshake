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

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::model_runtime::{
    validate_ready_frame, WarmAgentGuestFrame, WarmAgentProtocolError, WarmVmSnapshotManifest,
    WarmVmSnapshotResourceScope,
};
use crate::sandbox::{
    AdapterId, DetachedProcessIdentity, ImageRef, NetPolicy, ProcessHandle, ProcessSpec,
    ResourceLimits, SandboxAdapter, SandboxAdapterError, Signal, SnapshotRef, TrustClass,
    SANDBOX_MODE_METADATA_KEY, SANDBOX_MODE_PERSISTENT,
};
use crate::swarm_orchestration::resource_scope::{
    stored_resource_scope_from_row, ResourceAccessContext, ResourceScope, ScopeDenied,
    RESOURCE_SCOPE_SELECT_COLUMNS,
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
    #[error(transparent)]
    Storage(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    ScopeDenied(#[from] ScopeDenied),
    #[error("durable worktree VM binding requires an account-scoped write context")]
    ScopeRequired,
    #[error("durable worktree VM binding requires a workspace-scoped write context")]
    WorkspaceScopeRequired,
    #[error("durable worktree VM binding requires an authenticated-session write context")]
    AuthenticatedSessionScopeRequired,
    #[error("durable worktree VM binding requires an AccessSpace-scoped write context")]
    AccessSpaceScopeRequired,
    #[error(
        "durable microVM handle for worktree `{worktree_id}` cannot be adopted by adapter `{adapter_id}`: {reason}"
    )]
    DurableHandleUnavailable {
        worktree_id: String,
        adapter_id: String,
        reason: String,
    },
    #[error("worktree VM spec for `{worktree_id}` is not persistent")]
    NonPersistentSpec { worktree_id: String },
    #[error("worktree `{worktree_id}` is already bound to a live microVM")]
    AlreadyBound { worktree_id: String },
    #[error("durable warm snapshot manifest is missing canonical source binding provenance")]
    SnapshotSourceMissing,
    #[error("durable warm snapshot source is outside the caller's exact `{dimension}` scope")]
    SnapshotScopeMismatch { dimension: &'static str },
    #[error("durable warm snapshot source does not match its canonical binding record")]
    SnapshotSourceMismatch,
    #[error("durable worktree VM binding is outside the caller's exact `{dimension}` scope")]
    BindingScopeMismatch { dimension: &'static str },
    #[error("persisted worktree VM binding has unknown state `{state}`")]
    InvalidPersistedState { state: String },
    #[error("worktree VM binding for `{worktree_id}` changed while `{operation}` was in progress")]
    StaleBinding {
        worktree_id: String,
        operation: &'static str,
    },
    #[error(
        "worktree VM `{operation}` failed for `{worktree_id}` ({primary}); compensating cleanup also failed ({cleanup})"
    )]
    CompensationFailed {
        worktree_id: String,
        operation: &'static str,
        primary: String,
        cleanup: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeVmBindingState {
    Active,
    Snapshotted,
    Terminated,
    Failed,
}

impl WorktreeVmBindingState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Snapshotted => "snapshotted",
            Self::Terminated => "terminated",
            Self::Failed => "failed",
        }
    }

    fn parse(value: &str) -> Result<Self, WorktreeVmError> {
        match value {
            "active" => Ok(Self::Active),
            "snapshotted" => Ok(Self::Snapshotted),
            "terminated" => Ok(Self::Terminated),
            "failed" => Ok(Self::Failed),
            other => Err(WorktreeVmError::InvalidPersistedState {
                state: other.to_string(),
            }),
        }
    }

    fn is_live(self) -> bool {
        matches!(self, Self::Active | Self::Snapshotted)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeVmBindingRecord {
    pub binding_id: Uuid,
    pub worktree_id: String,
    pub adapter_id: String,
    pub process_handle: ProcessHandle,
    pub latest_snapshot: Option<SnapshotRef>,
    pub binding_state: WorktreeVmBindingState,
    pub generation: i64,
    pub failure_reason: Option<String>,
}

/// Exact durable ownership fence captured when a create/restore attempt binds
/// its VM. Compensation must present this identity so a delayed attempt cannot
/// tear down a later binding for the same worktree key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeVmBindingIdentity {
    pub binding_id: Uuid,
    pub generation: i64,
    pub process_handle: ProcessHandle,
}

/// Result of an idempotent worktree-VM ensure. `created` distinguishes the
/// caller that crossed the VM side-effect boundary from a caller that merely
/// adopted an existing binding, so pending-create compensation never tears
/// down a VM it did not create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeVmEnsureOutcome {
    pub handle: ProcessHandle,
    pub created: bool,
    pub binding_identity: Option<WorktreeVmBindingIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeVmRestoreOutcome {
    pub handle: ProcessHandle,
    pub binding_identity: Option<WorktreeVmBindingIdentity>,
}

#[derive(Clone)]
struct WorktreeVmDurableStore {
    pool: PgPool,
    access: ResourceAccessContext,
}

/// Binds `worktree_id` -> a persistent microVM handle, with snapshot/restore.
pub struct WorktreeVmRegistry {
    adapter: Arc<dyn SandboxAdapter>,
    persistent: Mutex<HashMap<String, ProcessHandle>>,
    /// Handles that crossed the adapter spawn boundary but have not yet
    /// committed their durable binding. Keeping this separate from
    /// `persistent` prevents an aborted INSERT from making a live VM both
    /// undiscoverable to teardown and falsely adoptable as a committed bind.
    pending: Mutex<HashMap<String, ProcessHandle>>,
    durable: Option<WorktreeVmDurableStore>,
}

impl WorktreeVmRegistry {
    /// Construct a registry over an injected sandbox adapter (the real
    /// `CloudHypervisorAdapter` in production, a fake in tests).
    pub fn new(adapter: Arc<dyn SandboxAdapter>) -> Self {
        Self {
            adapter,
            persistent: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            durable: None,
        }
    }

    /// Construct a registry whose binding authority survives a registry/app
    /// component restart. Reads are filtered in PostgreSQL and authorized again
    /// after deserialization through the supplied account scope.
    pub fn new_durable(
        adapter: Arc<dyn SandboxAdapter>,
        pool: PgPool,
        access: ResourceAccessContext,
    ) -> Self {
        Self {
            adapter,
            persistent: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            durable: Some(WorktreeVmDurableStore { pool, access }),
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
        let spec = Self::worktree_spec(worktree_id);
        self.ensure_worktree_vm_with_spec(worktree_id, spec).await
    }

    /// Boot the exact production VM specification and make this registry the
    /// owner of its lifecycle. The per-worktree mutex remains held across the
    /// spawn + durable write so concurrent local callers cannot create two VMs.
    pub async fn ensure_worktree_vm_with_spec(
        &self,
        worktree_id: &str,
        spec: ProcessSpec,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        Ok(self
            .ensure_worktree_vm_with_spec_outcome(worktree_id, spec)
            .await?
            .handle)
    }

    pub(crate) async fn ensure_worktree_vm_with_spec_outcome(
        &self,
        worktree_id: &str,
        spec: ProcessSpec,
    ) -> Result<WorktreeVmEnsureOutcome, WorktreeVmError> {
        if spec
            .metadata
            .get(SANDBOX_MODE_METADATA_KEY)
            .map(String::as_str)
            != Some(SANDBOX_MODE_PERSISTENT)
        {
            return Err(WorktreeVmError::NonPersistentSpec {
                worktree_id: worktree_id.to_string(),
            });
        }
        let mut map = self.persistent.lock().await;
        if self.durable.is_none() {
            if let Some(handle) = map.get(worktree_id) {
                return Ok(WorktreeVmEnsureOutcome {
                    handle: handle.clone(),
                    created: false,
                    binding_identity: Some(transient_binding_identity(handle)),
                });
            }
        }
        if let Some(handle) = self.pending.lock().await.get(worktree_id).cloned() {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(),
                adapter_id: handle.adapter_id.to_string(),
                reason: "adapter spawn completed but durable binding did not commit; pending VM must be torn down before retry"
                    .to_string(),
            });
        }

        // The in-memory mutex serializes one registry instance only. A
        // transaction-scoped PostgreSQL advisory lock closes the cross-registry
        // and cross-process load -> spawn -> bind race. Dropping this future
        // rolls the transaction back and releases the lock automatically.
        let mut durable_serialization = if let Some(durable) = &self.durable {
            let lock_key = durable.serialization_key(worktree_id)?;
            let mut transaction = durable.pool.begin().await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
                .bind(lock_key)
                .execute(&mut *transaction)
                .await?;
            Some(transaction)
        } else {
            None
        };

        if let (Some(durable), Some(transaction)) = (&self.durable, durable_serialization.as_mut())
        {
            if let Some(binding) = durable
                .load_physical_key_in_transaction(worktree_id, true, transaction)
                .await?
            {
                let binding_id = binding.binding_id;
                let generation = binding.generation;
                let adopted = self.adopt_durable_binding(&mut map, binding).await?;
                if let Some(transaction) = durable_serialization.take() {
                    transaction.commit().await?;
                }
                return Ok(WorktreeVmEnsureOutcome {
                    handle: adopted.clone(),
                    created: false,
                    binding_identity: Some(WorktreeVmBindingIdentity {
                        binding_id,
                        generation,
                        process_handle: adopted,
                    }),
                });
            }
        }

        let handle = self.adapter.spawn(spec).await?;
        self.pending
            .lock()
            .await
            .insert(worktree_id.to_string(), handle.clone());
        let mut binding_identity = None;
        if let (Some(durable), Some(transaction)) = (&self.durable, durable_serialization.as_mut())
        {
            match durable
                .persist_handle_in_transaction(
                    worktree_id,
                    &handle,
                    None,
                    WorktreeVmBindingState::Active,
                    transaction,
                )
                .await
            {
                Ok(identity) => binding_identity = Some(identity),
                Err(error) => {
                    let cleanup = self.adapter.kill(&handle, Signal::Kill).await;
                    return match cleanup {
                        Ok(()) => {
                            self.pending.lock().await.remove(worktree_id);
                            Err(error)
                        }
                        Err(cleanup_error) => Err(WorktreeVmError::DurableHandleUnavailable {
                            worktree_id: worktree_id.to_string(),
                            adapter_id: handle.adapter_id.to_string(),
                            reason: format!(
                                "durable binding write failed ({error}); rollback kill also failed ({cleanup_error})"
                            ),
                        }),
                    };
                }
            }
        }
        if let Some(transaction) = durable_serialization.take() {
            if let Err(error) = transaction.commit().await {
                let cleanup = self.adapter.kill(&handle, Signal::Kill).await;
                return match cleanup {
                    Ok(()) => {
                        self.pending.lock().await.remove(worktree_id);
                        Err(error.into())
                    }
                    Err(cleanup_error) => Err(WorktreeVmError::DurableHandleUnavailable {
                        worktree_id: worktree_id.to_string(),
                        adapter_id: handle.adapter_id.to_string(),
                        reason: format!(
                            "durable binding commit failed ({error}); rollback kill also failed ({cleanup_error})"
                        ),
                    }),
                };
            }
        }
        self.pending.lock().await.remove(worktree_id);
        map.insert(worktree_id.to_string(), handle.clone());
        if binding_identity.is_none() {
            binding_identity = Some(transient_binding_identity(&handle));
        }
        Ok(WorktreeVmEnsureOutcome {
            handle,
            created: true,
            binding_identity,
        })
    }

    /// Snapshot the worktree's persistent VM (Master Spec §3.5.7 #7). Looks up
    /// the bound handle and calls `adapter.snapshot`, returning the
    /// [`SnapshotRef`] (config.json + state.json + memory dir; carries the
    /// serial `observe_path` for resume confirmation).
    pub async fn snapshot(&self, worktree_id: &str) -> Result<SnapshotRef, WorktreeVmError> {
        let mut map = self.persistent.lock().await;
        let Some(durable) = &self.durable else {
            let handle =
                map.get(worktree_id)
                    .cloned()
                    .ok_or_else(|| WorktreeVmError::NotBound {
                        worktree_id: worktree_id.to_string(),
                    })?;
            return Ok(self.adapter.snapshot(&handle).await?);
        };

        let mut transaction = durable.begin_serialized(worktree_id).await?;
        let binding = durable
            .load_in_transaction(worktree_id, true, &mut transaction)
            .await?
            .ok_or_else(|| WorktreeVmError::NotBound {
                worktree_id: worktree_id.to_string(),
            })?;
        let snapshot = self.adapter.snapshot(&binding.process_handle).await?;
        if let Err(primary) = durable
            .record_snapshot_in_transaction(&binding, &snapshot, &mut transaction)
            .await
        {
            return Err(match self.adapter.delete_snapshot(&snapshot).await {
                Ok(()) => primary,
                Err(cleanup) => WorktreeVmError::CompensationFailed {
                    worktree_id: worktree_id.to_string(),
                    operation: "snapshot persistence",
                    primary: primary.to_string(),
                    cleanup: cleanup.to_string(),
                },
            });
        }
        if let Err(primary) = transaction.commit().await {
            return Err(match self.adapter.delete_snapshot(&snapshot).await {
                Ok(()) => primary.into(),
                Err(cleanup) => WorktreeVmError::CompensationFailed {
                    worktree_id: worktree_id.to_string(),
                    operation: "snapshot commit",
                    primary: primary.to_string(),
                    cleanup: cleanup.to_string(),
                },
            });
        }
        map.insert(worktree_id.to_string(), binding.process_handle);
        Ok(snapshot)
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
        Ok(self
            .restore_serialized(worktree_id, snapshot, None)
            .await?
            .handle)
    }

    async fn restore_serialized(
        &self,
        worktree_id: &str,
        snapshot: &SnapshotRef,
        manifest: Option<&WarmVmSnapshotManifest>,
    ) -> Result<WorktreeVmRestoreOutcome, WorktreeVmError> {
        let mut map = self.persistent.lock().await;
        if map.contains_key(worktree_id) {
            return Err(WorktreeVmError::AlreadyBound {
                worktree_id: worktree_id.to_string(),
            });
        }
        if let Some(handle) = self.pending.lock().await.get(worktree_id).cloned() {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(),
                adapter_id: handle.adapter_id.to_string(),
                reason: "adapter restore completed but durable binding did not commit; pending VM must be torn down before retry".to_string(),
            });
        }

        let mut durable_serialization = if let Some(durable) = &self.durable {
            let mut transaction = durable.begin_serialized(worktree_id).await?;
            if let Some(manifest) = manifest {
                durable
                    .authorize_snapshot_source_in_transaction(manifest, &mut transaction)
                    .await?;
            }
            if durable
                .load_physical_key_in_transaction(worktree_id, true, &mut transaction)
                .await?
                .is_some()
            {
                return Err(WorktreeVmError::AlreadyBound {
                    worktree_id: worktree_id.to_string(),
                });
            }
            Some(transaction)
        } else {
            None
        };

        let handle = self.adapter.restore(snapshot).await?;
        self.pending
            .lock()
            .await
            .insert(worktree_id.to_string(), handle.clone());
        let mut binding_identity = None;
        if let (Some(durable), Some(transaction)) = (&self.durable, durable_serialization.as_mut())
        {
            match durable
                .persist_handle_in_transaction(
                    worktree_id,
                    &handle,
                    Some(snapshot),
                    WorktreeVmBindingState::Snapshotted,
                    transaction,
                )
                .await
            {
                Ok(identity) => binding_identity = Some(identity),
                Err(primary) => {
                    return Err(self
                        .compensate_uncommitted_handle(
                            worktree_id,
                            &handle,
                            "restore persistence",
                            primary,
                        )
                        .await);
                }
            }
        }
        if let Some(transaction) = durable_serialization.take() {
            if let Err(primary) = transaction.commit().await {
                return Err(self
                    .compensate_uncommitted_handle(
                        worktree_id,
                        &handle,
                        "restore commit",
                        primary.into(),
                    )
                    .await);
            }
        }
        self.pending.lock().await.remove(worktree_id);
        map.insert(worktree_id.to_string(), handle.clone());
        if binding_identity.is_none() {
            binding_identity = Some(transient_binding_identity(&handle));
        }
        Ok(WorktreeVmRestoreOutcome {
            handle,
            binding_identity,
        })
    }

    async fn compensate_uncommitted_handle(
        &self,
        worktree_id: &str,
        handle: &ProcessHandle,
        operation: &'static str,
        primary: WorktreeVmError,
    ) -> WorktreeVmError {
        match self.adapter.kill(handle, Signal::Kill).await {
            Ok(()) => {
                self.pending.lock().await.remove(worktree_id);
                primary
            }
            Err(cleanup) => WorktreeVmError::CompensationFailed {
                worktree_id: worktree_id.to_string(),
                operation,
                primary: primary.to_string(),
                cleanup: cleanup.to_string(),
            },
        }
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
        let mut manifest = WarmVmSnapshotManifest::new(
            worktree_id,
            model_artifact_sha256,
            model_guest_path,
            ready_nonce,
            snapshot.clone(),
        );
        if let Some(durable) = &self.durable {
            let stamp = async {
                let binding = durable.load(worktree_id, false).await?.ok_or_else(|| {
                    WorktreeVmError::NotBound {
                        worktree_id: worktree_id.to_string(),
                    }
                })?;
                if binding.binding_state != WorktreeVmBindingState::Snapshotted
                    || binding.latest_snapshot.as_ref() != Some(&snapshot)
                {
                    return Err(WorktreeVmError::StaleBinding {
                        worktree_id: worktree_id.to_string(),
                        operation: "warm snapshot provenance stamp",
                    });
                }
                let scope = durable.require_scope()?;
                Ok::<_, WorktreeVmError>(manifest.with_durable_source(
                    binding.binding_id,
                    binding.generation,
                    snapshot_resource_scope(scope),
                ))
            }
            .await;
            manifest = match stamp {
                Ok(manifest) => manifest,
                Err(error) => {
                    let _ = self.adapter.delete_snapshot(&snapshot).await;
                    return Err(error);
                }
            };
        }
        Ok(manifest)
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
        Ok(self
            .restore_warm_model_with_identity(
                manifest,
                expected_model_artifact_sha256,
                expected_model_guest_path,
            )
            .await?
            .handle)
    }

    pub(crate) async fn restore_warm_model_with_identity(
        &self,
        manifest: &WarmVmSnapshotManifest,
        expected_model_artifact_sha256: &str,
        expected_model_guest_path: &str,
    ) -> Result<WorktreeVmRestoreOutcome, WorktreeVmError> {
        manifest.validate_for_restore(expected_model_artifact_sha256, expected_model_guest_path)?;
        self.restore_serialized(&manifest.worktree_id, &manifest.snapshot, Some(manifest))
            .await
    }

    /// Tear down the worktree's bound VM (best-effort kill) and unbind it.
    pub async fn teardown_worktree_vm(&self, worktree_id: &str) -> Result<(), WorktreeVmError> {
        self.teardown_worktree_vm_inner(worktree_id, None).await
    }

    pub async fn teardown_worktree_vm_if_current(
        &self,
        worktree_id: &str,
        expected: &WorktreeVmBindingIdentity,
    ) -> Result<(), WorktreeVmError> {
        self.teardown_worktree_vm_inner(worktree_id, Some(expected))
            .await
    }

    /// Reconcile adapter-local ownership after another registry/process has
    /// already terminalized the canonical durable row. A terminal database row
    /// proves the external VM should be gone; it does not release an exact local
    /// adapter handle or its process-global committed-memory reservation.
    async fn reconcile_terminal_local_binding(
        &self,
        map: &mut HashMap<String, ProcessHandle>,
        worktree_id: &str,
        binding: &WorktreeVmBindingRecord,
    ) -> Result<(), WorktreeVmError> {
        debug_assert!(!binding.binding_state.is_live());
        let local = map.get(worktree_id).cloned();
        let pending = self.pending.lock().await.get(worktree_id).cloned();

        // Validate every local candidate before touching either one. A mismatch
        // is an ABA successor or state drift and must fail closed without a
        // partial kill.
        for candidate in [local.as_ref(), pending.as_ref()].into_iter().flatten() {
            if candidate != &binding.process_handle {
                return Err(WorktreeVmError::StaleBinding {
                    worktree_id: worktree_id.to_string(),
                    operation: "terminal local reconciliation",
                });
            }
        }

        if local.as_ref() == Some(&binding.process_handle) {
            self.adapter
                .kill(&binding.process_handle, Signal::Term)
                .await?;
        } else if pending.as_ref() == Some(&binding.process_handle) {
            self.adapter
                .kill(&binding.process_handle, Signal::Term)
                .await?;
        }
        if pending.as_ref() == Some(&binding.process_handle) {
            self.pending.lock().await.remove(worktree_id);
        }
        Ok(())
    }

    async fn teardown_worktree_vm_inner(
        &self,
        worktree_id: &str,
        expected: Option<&WorktreeVmBindingIdentity>,
    ) -> Result<(), WorktreeVmError> {
        let mut map = self.persistent.lock().await;
        if let Some(durable) = &self.durable {
            let mut transaction = durable.begin_serialized(worktree_id).await?;
            let binding = durable
                .load_in_transaction(worktree_id, false, &mut transaction)
                .await?;
            if binding.is_none() {
                let local = map.get(worktree_id).cloned();
                let pending = self.pending.lock().await.get(worktree_id).cloned();

                // A missing canonical row is idempotent only when this registry
                // owns no local candidate. Validate BOTH maps before touching a
                // process: cancelled replacement creation can legitimately leave
                // an old committed local handle plus a different pending handle.
                // Choosing one by map order would orphan the other or destroy an
                // ABA successor while falsely reporting successful cleanup.
                let cleanup = if let Some(expected) = expected {
                    for candidate in [local.as_ref(), pending.as_ref()].into_iter().flatten() {
                        if candidate != &expected.process_handle {
                            return Err(WorktreeVmError::StaleBinding {
                                worktree_id: worktree_id.to_string(),
                                operation: "owned teardown",
                            });
                        }
                    }
                    (local.is_some() || pending.is_some())
                        .then(|| (expected.process_handle.clone(), expected.binding_id))
                } else {
                    match (local.as_ref(), pending.as_ref()) {
                        (Some(local), Some(pending)) if local != pending => {
                            return Err(WorktreeVmError::StaleBinding {
                                worktree_id: worktree_id.to_string(),
                                operation: "absent-row teardown",
                            });
                        }
                        (Some(handle), _) | (None, Some(handle)) => {
                            Some((handle.clone(), handle.id))
                        }
                        (None, None) => None,
                    }
                };

                if let Some((handle, process_uuid)) = cleanup.as_ref() {
                    let detached = !matches!(
                        self.adapter.status(handle).await,
                        Ok(crate::sandbox::ProcessStatus::Running)
                    );
                    if detached {
                        self.adapter
                            .reclaim_detached(
                                &DetachedProcessIdentity {
                                    process_uuid: *process_uuid,
                                    handle: handle.clone(),
                                    executable_sha256: None,
                                    os_creation_time_100ns: None,
                                },
                                Signal::Term,
                            )
                            .await?;
                    } else {
                        self.adapter.kill(handle, Signal::Term).await?;
                    }
                }
                transaction.commit().await?;
                if let Some((handle, _)) = cleanup {
                    if map.get(worktree_id) == Some(&handle) {
                        map.remove(worktree_id);
                    }
                    let mut pending = self.pending.lock().await;
                    if pending.get(worktree_id) == Some(&handle) {
                        pending.remove(worktree_id);
                    }
                }
                return Ok(());
            }
            if let Some(expected) = expected {
                let binding = binding
                    .as_ref()
                    .expect("absent durable binding returned through the reconciliation branch");
                if binding.binding_id != expected.binding_id
                    || binding.generation != expected.generation
                    || binding.process_handle != expected.process_handle
                {
                    return Err(WorktreeVmError::StaleBinding {
                        worktree_id: worktree_id.to_string(),
                        operation: "owned teardown",
                    });
                }
            }
            if let Some(binding) = binding
                .as_ref()
                .filter(|binding| binding.binding_state.is_live())
            {
                let detached = !matches!(
                    self.adapter.status(&binding.process_handle).await,
                    Ok(crate::sandbox::ProcessStatus::Running)
                );
                if detached {
                    self.adapter
                        .reclaim_detached(
                            &DetachedProcessIdentity {
                                process_uuid: binding.binding_id,
                                handle: binding.process_handle.clone(),
                                executable_sha256: None,
                                os_creation_time_100ns: None,
                            },
                            Signal::Term,
                        )
                        .await?;
                } else {
                    self.adapter
                        .kill(&binding.process_handle, Signal::Term)
                        .await?;
                }
                durable
                    .mark_terminated_in_transaction(&binding, &mut transaction)
                    .await?;
                transaction.commit().await?;
                map.remove(worktree_id);
                self.pending.lock().await.remove(worktree_id);
                return Ok(());
            }

            let binding = binding
                .as_ref()
                .expect("absent durable binding returned through the reconciliation branch");
            self.reconcile_terminal_local_binding(&mut map, worktree_id, binding)
                .await?;
            transaction.commit().await?;
            map.remove(worktree_id);
            return Ok(());
        }

        let handle =
            map.get(worktree_id)
                .cloned()
                .or(self.pending.lock().await.get(worktree_id).cloned());
        let Some(handle) = handle else {
            return Ok(());
        };
        if let Some(expected) = expected {
            let current = transient_binding_identity(&handle);
            if &current != expected {
                return Err(WorktreeVmError::StaleBinding {
                    worktree_id: worktree_id.to_string(),
                    operation: "owned teardown",
                });
            }
        }
        self.adapter.kill(&handle, Signal::Term).await?;
        map.remove(worktree_id);
        self.pending.lock().await.remove(worktree_id);
        Ok(())
    }

    /// Whether a microVM is currently bound to `worktree_id`.
    pub async fn is_bound(&self, worktree_id: &str) -> bool {
        self.resolve_worktree_vm(worktree_id).await.is_ok()
    }

    /// Resolve the currently bound VM. A fresh registry instance consults the
    /// durable row and asks the adapter to prove that exact handle is still
    /// running. An adapter that cannot adopt it produces a named fail-closed
    /// error instead of silently booting a replacement.
    pub async fn resolve_worktree_vm(
        &self,
        worktree_id: &str,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        let mut map = self.persistent.lock().await;
        if let Some(handle) = map.get(worktree_id) {
            return Ok(handle.clone());
        }
        let Some(durable) = &self.durable else {
            return Err(WorktreeVmError::NotBound {
                worktree_id: worktree_id.to_string(),
            });
        };
        let binding =
            durable
                .load(worktree_id, true)
                .await?
                .ok_or_else(|| WorktreeVmError::NotBound {
                    worktree_id: worktree_id.to_string(),
                })?;
        self.adopt_durable_binding(&mut map, binding).await
    }

    /// Read this caller's durable binding, including terminal state, for
    /// diagnostics and independent PostgreSQL verification.
    pub async fn durable_binding(
        &self,
        worktree_id: &str,
    ) -> Result<Option<WorktreeVmBindingRecord>, WorktreeVmError> {
        match &self.durable {
            Some(durable) => durable.load(worktree_id, false).await,
            None => Ok(None),
        }
    }

    async fn adopt_durable_binding(
        &self,
        map: &mut HashMap<String, ProcessHandle>,
        binding: WorktreeVmBindingRecord,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        match self.adapter.status(&binding.process_handle).await {
            Ok(crate::sandbox::ProcessStatus::Running) => {
                map.insert(binding.worktree_id.clone(), binding.process_handle.clone());
                Ok(binding.process_handle)
            }
            Ok(status) => Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: binding.worktree_id,
                adapter_id: binding.adapter_id,
                reason: format!("persisted handle is not running: {status:?}"),
            }),
            Err(error) => Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: binding.worktree_id,
                adapter_id: binding.adapter_id,
                reason: error.to_string(),
            }),
        }
    }
}

impl WorktreeVmDurableStore {
    fn require_scope(&self) -> Result<&ResourceScope, WorktreeVmError> {
        let scope = self
            .access
            .write_scope()
            .ok_or(WorktreeVmError::ScopeRequired)?;
        if scope.authenticated_session.is_none() {
            return Err(WorktreeVmError::AuthenticatedSessionScopeRequired);
        }
        if scope.access_space.is_none() {
            return Err(WorktreeVmError::AccessSpaceScopeRequired);
        }
        if scope.workspace.is_none() {
            return Err(WorktreeVmError::WorkspaceScopeRequired);
        }
        Ok(scope)
    }

    fn serialization_key(&self, worktree_id: &str) -> Result<String, WorktreeVmError> {
        let scope = self.require_scope()?;
        Ok(format!(
            "worktree-vm:{}:{}:{}",
            scope.owner_account_id,
            scope
                .workspace
                .as_ref()
                .map(|workspace| workspace.as_str())
                .unwrap_or("<none>"),
            worktree_id
        ))
    }

    async fn begin_serialized(
        &self,
        worktree_id: &str,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, WorktreeVmError> {
        let lock_key = self.serialization_key(worktree_id)?;
        let mut transaction = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(lock_key)
            .execute(&mut *transaction)
            .await?;
        Ok(transaction)
    }

    async fn persist_handle_in_transaction(
        &self,
        worktree_id: &str,
        handle: &ProcessHandle,
        snapshot: Option<&SnapshotRef>,
        state: WorktreeVmBindingState,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<WorktreeVmBindingIdentity, WorktreeVmError> {
        self.require_scope()?;
        let scope = self.access.insert_columns();
        let query = sqlx::query(
            r#"
            INSERT INTO worktree_vm_bindings (
                binding_id, worktree_id, adapter_id, process_handle,
                latest_snapshot, binding_state, failure_reason,
                owner_account_id, actor_principal_id, authenticated_session_id,
                access_space_id, workspace_id
            ) VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, $8, $9, $10, $11)
            ON CONFLICT (owner_account_id, workspace_id, worktree_id)
            DO UPDATE SET
                binding_id = EXCLUDED.binding_id,
                adapter_id = EXCLUDED.adapter_id,
                process_handle = EXCLUDED.process_handle,
                latest_snapshot = EXCLUDED.latest_snapshot,
                binding_state = EXCLUDED.binding_state,
                failure_reason = NULL,
                actor_principal_id = EXCLUDED.actor_principal_id,
                authenticated_session_id = EXCLUDED.authenticated_session_id,
                access_space_id = EXCLUDED.access_space_id,
                generation = worktree_vm_bindings.generation + 1,
                updated_at = NOW()
            WHERE worktree_vm_bindings.actor_principal_id IS NOT DISTINCT FROM EXCLUDED.actor_principal_id
              AND worktree_vm_bindings.authenticated_session_id IS NOT DISTINCT FROM EXCLUDED.authenticated_session_id
              AND worktree_vm_bindings.access_space_id IS NOT DISTINCT FROM EXCLUDED.access_space_id
            RETURNING binding_id, generation
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(worktree_id)
        .bind(handle.adapter_id.as_str())
        .bind(serde_json::to_value(handle)?)
        .bind(snapshot.map(serde_json::to_value).transpose()?)
        .bind(state.as_str());
        let row = scope.bind(query).fetch_optional(&mut **transaction).await?;
        let Some(row) = row else {
            return Err(WorktreeVmError::BindingScopeMismatch {
                dimension: "exact_resource_scope",
            });
        };
        Ok(WorktreeVmBindingIdentity {
            binding_id: row.try_get("binding_id")?,
            generation: row.try_get("generation")?,
            process_handle: handle.clone(),
        })
    }

    async fn record_snapshot_in_transaction(
        &self,
        binding: &WorktreeVmBindingRecord,
        snapshot: &SnapshotRef,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), WorktreeVmError> {
        self.require_scope()?;
        let predicate = self.access.sql_predicate(5);
        let sql = format!(
            "UPDATE worktree_vm_bindings SET latest_snapshot = $4, binding_state = 'snapshotted', updated_at = NOW() WHERE worktree_id = $1 AND binding_id = $2 AND generation = $3 AND binding_state IN ('active', 'snapshotted'){}",
            predicate.clause()
        );
        let result = predicate
            .bind(
                sqlx::query(&sql)
                    .bind(&binding.worktree_id)
                    .bind(binding.binding_id)
                    .bind(binding.generation)
                    .bind(serde_json::to_value(snapshot)?),
            )
            .execute(&mut **transaction)
            .await?;
        if result.rows_affected() == 0 {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: binding.worktree_id.clone(),
                operation: "snapshot",
            });
        }
        Ok(())
    }

    async fn mark_terminated_in_transaction(
        &self,
        binding: &WorktreeVmBindingRecord,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), WorktreeVmError> {
        self.require_scope()?;
        let predicate = self.access.sql_predicate(4);
        let sql = format!(
            "UPDATE worktree_vm_bindings SET binding_state = 'terminated', updated_at = NOW() WHERE worktree_id = $1 AND binding_id = $2 AND generation = $3 AND binding_state IN ('active', 'snapshotted'){}",
            predicate.clause()
        );
        let result = predicate
            .bind(
                sqlx::query(&sql)
                    .bind(&binding.worktree_id)
                    .bind(binding.binding_id)
                    .bind(binding.generation),
            )
            .execute(&mut **transaction)
            .await?;
        if result.rows_affected() == 0 {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: binding.worktree_id.clone(),
                operation: "teardown",
            });
        }
        Ok(())
    }

    async fn load(
        &self,
        worktree_id: &str,
        live_only: bool,
    ) -> Result<Option<WorktreeVmBindingRecord>, WorktreeVmError> {
        self.require_scope()?;
        let predicate = self.access.sql_predicate(2);
        let live_clause = if live_only {
            " AND binding_state IN ('active', 'snapshotted')"
        } else {
            ""
        };
        let sql = format!(
            "SELECT binding_id, worktree_id, adapter_id, process_handle, latest_snapshot, binding_state, generation, failure_reason, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM worktree_vm_bindings WHERE worktree_id = $1{live_clause}{}",
            predicate.clause()
        );
        let row = predicate
            .bind(sqlx::query(&sql).bind(worktree_id))
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let stored_scope = stored_resource_scope_from_row(&row)?;
        self.access.authorize_row(&stored_scope)?;
        authorize_exact_binding_scope(self.require_scope()?, &stored_scope)?;
        let state = WorktreeVmBindingState::parse(row.try_get("binding_state")?)?;
        if live_only && !state.is_live() {
            return Ok(None);
        }
        Ok(Some(WorktreeVmBindingRecord {
            binding_id: row.try_get("binding_id")?,
            worktree_id: row.try_get("worktree_id")?,
            adapter_id: row.try_get("adapter_id")?,
            process_handle: serde_json::from_value(row.try_get("process_handle")?)?,
            latest_snapshot: row
                .try_get::<Option<serde_json::Value>, _>("latest_snapshot")?
                .map(serde_json::from_value)
                .transpose()?,
            binding_state: state,
            generation: row.try_get("generation")?,
            failure_reason: row.try_get("failure_reason")?,
        }))
    }

    async fn load_in_transaction(
        &self,
        worktree_id: &str,
        live_only: bool,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Option<WorktreeVmBindingRecord>, WorktreeVmError> {
        self.require_scope()?;
        let predicate = self.access.sql_predicate(2);
        let live_clause = if live_only {
            " AND binding_state IN ('active', 'snapshotted')"
        } else {
            ""
        };
        let sql = format!(
            "SELECT binding_id, worktree_id, adapter_id, process_handle, latest_snapshot, binding_state, generation, failure_reason, {RESOURCE_SCOPE_SELECT_COLUMNS} FROM worktree_vm_bindings WHERE worktree_id = $1{live_clause}{}",
            predicate.clause()
        );
        let row = predicate
            .bind(sqlx::query(&sql).bind(worktree_id))
            .fetch_optional(&mut **transaction)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let stored_scope = stored_resource_scope_from_row(&row)?;
        self.access.authorize_row(&stored_scope)?;
        authorize_exact_binding_scope(self.require_scope()?, &stored_scope)?;
        let state = WorktreeVmBindingState::parse(row.try_get("binding_state")?)?;
        if live_only && !state.is_live() {
            return Ok(None);
        }
        Ok(Some(WorktreeVmBindingRecord {
            binding_id: row.try_get("binding_id")?,
            worktree_id: row.try_get("worktree_id")?,
            adapter_id: row.try_get("adapter_id")?,
            process_handle: serde_json::from_value(row.try_get("process_handle")?)?,
            latest_snapshot: row
                .try_get::<Option<serde_json::Value>, _>("latest_snapshot")?
                .map(serde_json::from_value)
                .transpose()?,
            binding_state: state,
            generation: row.try_get("generation")?,
            failure_reason: row.try_get("failure_reason")?,
        }))
    }

    /// Probe the canonical physical uniqueness key while the matching advisory
    /// lock is held. This intentionally uses only owner/workspace/worktree in
    /// SQL so a row owned by another exact actor/session/AccessSpace cannot be
    /// mistaken for absence and trigger a second VM spawn. Exact attribution is
    /// checked before returning or crossing the adapter side-effect boundary.
    async fn load_physical_key_in_transaction(
        &self,
        worktree_id: &str,
        live_only: bool,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Option<WorktreeVmBindingRecord>, WorktreeVmError> {
        let scope = self.require_scope()?;
        let workspace = scope
            .workspace
            .as_ref()
            .ok_or(WorktreeVmError::WorkspaceScopeRequired)?;
        let row = sqlx::query(&format!(
            "SELECT binding_id, worktree_id, adapter_id, process_handle, latest_snapshot, binding_state, generation, failure_reason, {RESOURCE_SCOPE_SELECT_COLUMNS} \
             FROM worktree_vm_bindings \
             WHERE owner_account_id = $1::uuid AND workspace_id = $2 AND worktree_id = $3"
        ))
        .bind(scope.owner_account_id.as_uuid())
        .bind(workspace.as_str())
        .bind(worktree_id)
        .fetch_optional(&mut **transaction)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let stored_scope = stored_resource_scope_from_row(&row)?;
        authorize_exact_binding_scope(scope, &stored_scope)?;
        let state = WorktreeVmBindingState::parse(row.try_get("binding_state")?)?;
        if live_only && !state.is_live() {
            return Ok(None);
        }
        Ok(Some(WorktreeVmBindingRecord {
            binding_id: row.try_get("binding_id")?,
            worktree_id: row.try_get("worktree_id")?,
            adapter_id: row.try_get("adapter_id")?,
            process_handle: serde_json::from_value(row.try_get("process_handle")?)?,
            latest_snapshot: row
                .try_get::<Option<serde_json::Value>, _>("latest_snapshot")?
                .map(serde_json::from_value)
                .transpose()?,
            binding_state: state,
            generation: row.try_get("generation")?,
            failure_reason: row.try_get("failure_reason")?,
        }))
    }

    async fn authorize_snapshot_source_in_transaction(
        &self,
        manifest: &WarmVmSnapshotManifest,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), WorktreeVmError> {
        let expected_scope = snapshot_resource_scope(self.require_scope()?);
        let manifest_scope = manifest
            .resource_scope
            .as_ref()
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        authorize_snapshot_scope(&expected_scope, manifest_scope)?;
        let binding_id = manifest
            .source_binding_id
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        let generation = manifest
            .source_binding_generation
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;

        let predicate = self.access.sql_predicate(2);
        let sql = format!(
            "SELECT binding_id, generation, latest_snapshot, {RESOURCE_SCOPE_SELECT_COLUMNS} \
             FROM worktree_vm_bindings WHERE binding_id = $1{}",
            predicate.clause()
        );
        let row = predicate
            .bind(sqlx::query(&sql).bind(binding_id))
            .fetch_optional(&mut **transaction)
            .await?;
        let Some(row) = row else {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        };
        let stored_scope = stored_resource_scope_from_row(&row)?;
        self.access.authorize_row(&stored_scope)?;
        authorize_exact_binding_scope(self.require_scope()?, &stored_scope)?;
        let stored_generation: i64 = row.try_get("generation")?;
        let stored_snapshot: Option<serde_json::Value> = row.try_get("latest_snapshot")?;
        if stored_generation != generation
            || stored_snapshot
                .map(serde_json::from_value::<SnapshotRef>)
                .transpose()?
                .as_ref()
                != Some(&manifest.snapshot)
        {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        Ok(())
    }
}

fn transient_binding_identity(handle: &ProcessHandle) -> WorktreeVmBindingIdentity {
    WorktreeVmBindingIdentity {
        binding_id: handle.id,
        generation: 0,
        process_handle: handle.clone(),
    }
}

fn snapshot_resource_scope(scope: &ResourceScope) -> WarmVmSnapshotResourceScope {
    WarmVmSnapshotResourceScope {
        owner_account_id: scope.owner_account_id.as_uuid(),
        actor_principal_id: scope.actor_principal_id.as_uuid(),
        authenticated_session_id: scope.authenticated_session.map(|value| value.as_uuid()),
        access_space_id: scope.access_space.map(|value| value.as_uuid()),
        workspace_id: scope
            .workspace
            .as_ref()
            .map(|value| value.as_str().to_string()),
    }
}

fn authorize_snapshot_scope(
    expected: &WarmVmSnapshotResourceScope,
    actual: &WarmVmSnapshotResourceScope,
) -> Result<(), WorktreeVmError> {
    if expected.owner_account_id != actual.owner_account_id {
        return Err(WorktreeVmError::SnapshotScopeMismatch {
            dimension: "owner_account_id",
        });
    }
    if expected.actor_principal_id != actual.actor_principal_id {
        return Err(WorktreeVmError::SnapshotScopeMismatch {
            dimension: "actor_principal_id",
        });
    }
    if expected.authenticated_session_id != actual.authenticated_session_id {
        return Err(WorktreeVmError::SnapshotScopeMismatch {
            dimension: "authenticated_session_id",
        });
    }
    if expected.access_space_id != actual.access_space_id {
        return Err(WorktreeVmError::SnapshotScopeMismatch {
            dimension: "access_space_id",
        });
    }
    if expected.workspace_id != actual.workspace_id {
        return Err(WorktreeVmError::SnapshotScopeMismatch {
            dimension: "workspace_id",
        });
    }
    Ok(())
}

fn authorize_exact_binding_scope(
    expected: &ResourceScope,
    actual: &crate::swarm_orchestration::resource_scope::StoredResourceScope,
) -> Result<(), WorktreeVmError> {
    let exact = snapshot_resource_scope(expected);
    let stored = WarmVmSnapshotResourceScope {
        owner_account_id: actual
            .owner_account_id
            .ok_or(WorktreeVmError::BindingScopeMismatch {
                dimension: "owner_account_id",
            })?
            .as_uuid(),
        actor_principal_id: actual
            .actor_principal_id
            .ok_or(WorktreeVmError::BindingScopeMismatch {
                dimension: "actor_principal_id",
            })?
            .as_uuid(),
        authenticated_session_id: actual.authenticated_session.map(|value| value.as_uuid()),
        access_space_id: actual.access_space.map(|value| value.as_uuid()),
        workspace_id: actual
            .workspace
            .as_ref()
            .map(|value| value.as_str().to_string()),
    };
    authorize_snapshot_scope(&exact, &stored).map_err(|error| match error {
        WorktreeVmError::SnapshotScopeMismatch { dimension } => {
            WorktreeVmError::BindingScopeMismatch { dimension }
        }
        other => other,
    })
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
        status: Option<ProcessStatus>,
        committed_memory_mib: u32,
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
            o.status = Some(ProcessStatus::Running);
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
            signal: Signal,
        ) -> Result<(), SandboxAdapterError> {
            let mut observation = self.obs.lock().unwrap();
            observation.kill_called = true;
            observation.status = Some(ProcessStatus::Killed { by_signal: signal });
            observation.committed_memory_mib = 0;
            Ok(())
        }
        async fn status(
            &self,
            _handle: &ProcessHandle,
        ) -> Result<ProcessStatus, SandboxAdapterError> {
            Ok(self
                .obs
                .lock()
                .unwrap()
                .status
                .clone()
                .unwrap_or(ProcessStatus::Running))
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

        reg.teardown_worktree_vm("wt-1")
            .await
            .expect("source VM must be torn down before restoring a successor");
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

        reg.teardown_worktree_vm("wt-warm")
            .await
            .expect("source VM must be torn down before warm restore");
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
    async fn production_spec_spawn_is_registry_owned_and_requires_persistent_mode() {
        let (reg, obs) = registry();
        let mut spec = WorktreeVmRegistry::worktree_spec("wt-production");
        spec.image_or_root = ImageRef::new("llama_cpp");
        let handle = reg
            .ensure_worktree_vm_with_spec("wt-production", spec.clone())
            .await
            .expect("production spec spawn");
        assert_eq!(handle.sandbox_internal_id, "hsk-ch-persistent-1");
        assert!(reg.is_bound("wt-production").await);
        assert_eq!(obs.lock().unwrap().spawn_count, 1);

        spec.metadata.remove(SANDBOX_MODE_METADATA_KEY);
        let error = reg
            .ensure_worktree_vm_with_spec("wt-non-persistent", spec)
            .await
            .expect_err("non-persistent spec must fail before adapter spawn");
        assert!(matches!(error, WorktreeVmError::NonPersistentSpec { .. }));
        assert_eq!(obs.lock().unwrap().spawn_count, 1);
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

    #[tokio::test]
    async fn terminal_durable_binding_reconciles_exact_local_handle_and_memory() {
        let (reg, obs) = registry();
        let handle = reg
            .ensure_worktree_vm("wt-terminal-reconcile")
            .await
            .expect("create local VM handle");
        {
            let mut observation = obs.lock().unwrap();
            observation.status = Some(ProcessStatus::Running);
            observation.committed_memory_mib = 512;
        }
        let terminal = WorktreeVmBindingRecord {
            binding_id: handle.id,
            worktree_id: "wt-terminal-reconcile".to_string(),
            adapter_id: handle.adapter_id.as_str().to_string(),
            process_handle: handle.clone(),
            latest_snapshot: None,
            binding_state: WorktreeVmBindingState::Terminated,
            generation: 1,
            failure_reason: None,
        };

        let mut map = reg.persistent.lock().await;
        reg.reconcile_terminal_local_binding(&mut map, "wt-terminal-reconcile", &terminal)
            .await
            .expect("terminal durable row must reconcile exact local ownership");
        map.remove("wt-terminal-reconcile");
        drop(map);

        let observation = obs.lock().unwrap();
        assert!(observation.kill_called, "exact local handle must be killed");
        assert!(matches!(
            observation.status.as_ref(),
            Some(ProcessStatus::Killed {
                by_signal: Signal::Term
            })
        ));
        assert_eq!(
            observation.committed_memory_mib, 0,
            "terminal-row reconciliation must release adapter-local committed memory"
        );
    }

    #[tokio::test]
    async fn terminal_durable_binding_refuses_mismatched_local_successor() {
        let (reg, obs) = registry();
        let successor = reg
            .ensure_worktree_vm("wt-terminal-stale")
            .await
            .expect("create local successor");
        {
            let mut observation = obs.lock().unwrap();
            observation.status = Some(ProcessStatus::Running);
            observation.committed_memory_mib = 512;
        }
        let stale = WorktreeVmBindingRecord {
            binding_id: Uuid::now_v7(),
            worktree_id: "wt-terminal-stale".to_string(),
            adapter_id: successor.adapter_id.as_str().to_string(),
            process_handle: ProcessHandle::new(
                successor.adapter_id.clone(),
                None,
                "hsk-ch-persistent-stale",
            ),
            latest_snapshot: None,
            binding_state: WorktreeVmBindingState::Terminated,
            generation: 1,
            failure_reason: None,
        };

        let mut map = reg.persistent.lock().await;
        assert!(matches!(
            reg.reconcile_terminal_local_binding(&mut map, "wt-terminal-stale", &stale)
                .await,
            Err(WorktreeVmError::StaleBinding {
                operation: "terminal local reconciliation",
                ..
            })
        ));
        assert_eq!(map.get("wt-terminal-stale"), Some(&successor));
        drop(map);

        let observation = obs.lock().unwrap();
        assert!(
            !observation.kill_called,
            "stale row must not kill successor"
        );
        assert!(matches!(
            observation.status.as_ref(),
            Some(ProcessStatus::Running)
        ));
        assert_eq!(observation.committed_memory_mib, 512);
    }

    #[tokio::test]
    async fn fenced_transient_teardown_is_idempotent_after_owned_cleanup() {
        let (reg, _obs) = registry();
        let outcome = reg
            .ensure_worktree_vm_with_spec_outcome(
                "wt-fenced-idempotent",
                WorktreeVmRegistry::worktree_spec("wt-fenced-idempotent"),
            )
            .await
            .expect("create fenced transient binding");
        let identity = outcome
            .binding_identity
            .expect("transient create returns an ownership identity");
        reg.teardown_worktree_vm_if_current("wt-fenced-idempotent", &identity)
            .await
            .expect("first owned teardown succeeds");
        reg.teardown_worktree_vm_if_current("wt-fenced-idempotent", &identity)
            .await
            .expect("already-cleaned owned teardown is idempotent");
    }

    #[tokio::test]
    async fn stale_transient_teardown_identity_refuses_successor() {
        let (reg, _obs) = registry();
        let first = reg
            .ensure_worktree_vm_with_spec_outcome(
                "wt-fenced-successor",
                WorktreeVmRegistry::worktree_spec("wt-fenced-successor"),
            )
            .await
            .expect("create first transient binding");
        let stale_identity = first
            .binding_identity
            .expect("first create returns an ownership identity");
        reg.teardown_worktree_vm_if_current("wt-fenced-successor", &stale_identity)
            .await
            .expect("clean first transient binding");
        let successor = reg
            .ensure_worktree_vm_with_spec_outcome(
                "wt-fenced-successor",
                WorktreeVmRegistry::worktree_spec("wt-fenced-successor"),
            )
            .await
            .expect("create successor transient binding");

        assert!(matches!(
            reg.teardown_worktree_vm_if_current("wt-fenced-successor", &stale_identity)
                .await,
            Err(WorktreeVmError::StaleBinding {
                operation: "owned teardown",
                ..
            })
        ));
        assert_eq!(
            reg.resolve_worktree_vm("wt-fenced-successor")
                .await
                .expect("successor remains bound"),
            successor.handle
        );
        reg.teardown_worktree_vm_if_current(
            "wt-fenced-successor",
            &successor.binding_identity.expect("successor identity"),
        )
        .await
        .expect("clean successor transient binding");
    }
}
