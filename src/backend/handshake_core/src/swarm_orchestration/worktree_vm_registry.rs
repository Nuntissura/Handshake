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
use std::sync::{Arc, OnceLock, Weak};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use surrealdb::types::SurrealValue;
use tokio::sync::{Mutex, OnceCell};
use uuid::Uuid;

use crate::kernel::context_bundle::{canonical_json_bytes, sha256_hex};
use crate::model_runtime::{
    validate_ready_frame, WarmAgentGuestFrame, WarmAgentProtocolError, WarmVmSnapshotManifest,
    WarmVmSnapshotResourceScope,
};
use crate::sandbox::{
    AdapterId, DetachedProcessIdentity, DetachedReclaimOutcome, ImageRef, NetPolicy,
    ProcessHandle, ProcessSpec,
    ResourceLimits, SandboxAdapter, SandboxAdapterError, Signal, SnapshotRef, TrustClass,
    SANDBOX_MODE_METADATA_KEY, SANDBOX_MODE_PERSISTENT,
};
use crate::storage::surreal::{SurrealStorage, SurrealStorageError};
use crate::swarm_orchestration::resource_scope::{
    ResourceAccessContext, ResourceScope, ScopeDenied,
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
    Storage(#[from] SurrealStorageError),
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
    #[error("persisted worktree VM binding is malformed: {reason}")]
    InvalidPersistedBinding { reason: String },
    #[error("durable worktree VM binding has no valid canonical EventLedger receipt")]
    EventLedgerReceiptMissing,
    #[error("durable worktree VM binding has ambiguous canonical EventLedger authority")]
    EventLedgerReceiptAmbiguous,
    #[error("durable worktree VM binding canonical EventLedger receipt is inconsistent")]
    EventLedgerReceiptMismatch,
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
    storage: SurrealStorage,
    access: ResourceAccessContext,
    schema_ready: Arc<OnceCell<()>>,
}

enum RegistrySignalAttempt {
    NotAttempted,
    Attempted,
    Acknowledged,
    Uncertain(String),
}

struct RegistryTerminalState {
    signal_future: Option<std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<crate::sandbox::ProcessStatus>, SandboxAdapterError>> + Send>>>,
    signal: RegistrySignalAttempt,
    terminal: Option<crate::sandbox::ProcessStatus>,
    exit_code: Option<i32>,
    exit_code_observed: bool,
    cleanup_complete: bool,
}

struct RegistryTeardownOwner {
    adapter: Weak<dyn SandboxAdapter>,
    keepalive: std::sync::Mutex<Option<Arc<dyn SandboxAdapter>>>,
    binding_id: Uuid,
    generation: Option<i64>,
    handle: ProcessHandle,
    detached: Option<DetachedProcessIdentity>,
    store_identity: Option<Arc<OnceCell<()>>>,
    state: Mutex<RegistryTerminalState>,
}

static RETAINED_REGISTRY_TEARDOWNS:
    OnceLock<std::sync::Mutex<HashMap<String, Arc<RegistryTeardownOwner>>>> =
    OnceLock::new();

static DURABLE_SERIALIZERS: OnceLock<std::sync::Mutex<HashMap<String, Weak<Mutex<()>>>>> =
    OnceLock::new();

/// Binds `worktree_id` -> a persistent microVM handle, with snapshot/restore.
pub struct WorktreeVmRegistry {
    transient_identity: Arc<TransientRegistryIdentity>,
    transient_acquisitions: std::sync::Mutex<HashMap<String, Arc<RetainedTransientAcquisition>>>,
    transient_captures: std::sync::Mutex<HashMap<String, Arc<RetainedTransientCapture>>>,
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
            transient_identity: Arc::new(TransientRegistryIdentity),
            transient_acquisitions: std::sync::Mutex::new(HashMap::new()),
            transient_captures: std::sync::Mutex::new(HashMap::new()),
            adapter,
            persistent: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            durable: None,
        }
    }

    /// Construct a registry whose binding authority survives a registry/app
    /// component restart in Handshake's embedded SurrealDB. The store lazily
    /// bootstraps its MT-023 schema on first use; every durable read and
    /// mutation predicates the complete five-field scope and every returned
    /// row is authorized again after deserialization.
    pub fn new_durable(
        adapter: Arc<dyn SandboxAdapter>,
        storage: SurrealStorage,
        access: ResourceAccessContext,
    ) -> Self {
        Self {
            transient_identity: Arc::new(TransientRegistryIdentity),
            transient_acquisitions: std::sync::Mutex::new(HashMap::new()),
            transient_captures: std::sync::Mutex::new(HashMap::new()),
            adapter,
            persistent: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
            durable: Some(WorktreeVmDurableStore {
                storage,
                access,
                schema_ready: Arc::new(OnceCell::new()),
            }),
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
            self.reject_pending_transient_capture(worktree_id)?;
            if let Some(handle) = map.get(worktree_id) {
                return Ok(WorktreeVmEnsureOutcome {
                    handle: handle.clone(),
                    created: false,
                    binding_identity: Some(transient_binding_identity(handle)),
                });
            }
        }
        let pending_handle = self.pending.lock().await.get(worktree_id).cloned();
        if let Some(handle) = pending_handle {
            let mut resume_original = false;
            if let Some(durable) = &self.durable {
                let _serial = durable.serializer(worktree_id)?.lock_owned().await;
                if let Some((binding, reservation_id)) = durable
                    .reconcile_known_reservation_binding(worktree_id, &handle, &serde_json::to_string(&spec)?).await? {
                    let binding_id = binding.binding_id;
                    let generation = binding.generation;
                    let adopted = self.adopt_durable_binding(&mut map, binding).await?;
                    self.pending.lock().await.remove(worktree_id);
                    durable.finish_reservation_delivery(reservation_id);
                return Ok(WorktreeVmEnsureOutcome {
                    handle: adopted.clone(), created: false,
                    binding_identity: Some(WorktreeVmBindingIdentity {
                        binding_id, generation, process_handle: adopted,
                    }),
                });
                }
                if let Some(id) = durable.resume_reservation_for_source(
                    worktree_id, &serde_json::to_string(&spec)?, &self.adapter,
                ).await? {
                    let owner = durable.retained_reservation(id)?;
                    if owner.handle.lock().unwrap_or_else(|error| error.into_inner()).as_ref() != Some(&handle) {
                        return Err(WorktreeVmError::StaleBinding {
                            worktree_id: worktree_id.to_string(), operation: "original pending acquisition",
                        });
                    }
                    resume_original = true;
                }
            }
            if !resume_original {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(),
                adapter_id: handle.adapter_id.to_string(),
                reason: "adapter spawn completed but durable binding did not commit; original acquisition requires exact reconciliation before retry"
                    .to_string(),
            });
            }
        }

        // The process-local keyed mutex prevents duplicate side effects across
        // registry instances in this process. Embedded SurrealDB remains the
        // authority: a durable reservation record closes the cross-process
        // load -> spawn -> bind race and fails closed when another process owns
        // the reservation.
        let _durable_guard = if let Some(durable) = &self.durable {
            Some(durable.serializer(worktree_id)?.lock_owned().await)
        } else {
            None
        };
        let reservation_id = if let Some(durable) = &self.durable {
            if let Some(original) = durable.resume_reservation_for_source(
                worktree_id, &serde_json::to_string(&spec)?, &self.adapter,
            ).await? {
                Some(original)
            } else {
            if let Some(binding) = durable.load_physical_key(worktree_id, true).await? {
                let binding_id = binding.binding_id;
                let generation = binding.generation;
                let adopted = self.adopt_durable_binding(&mut map, binding).await?;
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
            {
                self.adapter.validate_spawn_acquisition()?;
                Some(durable.reserve(worktree_id, serde_json::to_string(&spec)?, &self.adapter).await?)
            }
            }
        } else {
            None
        };

        let transient_owner = if self.durable.is_none() {
            Some(self.begin_transient_acquisition(worktree_id, ReservationAcquisitionRequest::Spawn(spec.clone()))?)
        } else { None };
        let handle = match (&self.durable, reservation_id) {
            (Some(durable), Some(reservation_id)) => durable.observe_reservation_acquisition(
                reservation_id, Arc::clone(&self.adapter), ReservationAcquisitionRequest::Spawn(spec),
            ).await?,
            _ => self.observe_transient_acquisition(transient_owner.as_ref()
                .expect("original transient spawn owner")).await?,
        };
        if let (Some(durable), Some(reservation_id)) = (&self.durable, reservation_id) {
            durable.retain_reservation_handle(reservation_id, &handle)?;
        }
        self.pending
            .lock()
            .await
            .insert(worktree_id.to_string(), handle.clone());
        let mut binding_identity = None;
        if let (Some(durable), Some(reservation_id)) = (&self.durable, reservation_id) {
            match durable
                .persist_reserved_handle(
                    worktree_id,
                    &handle,
                    None,
                    WorktreeVmBindingState::Active,
                    reservation_id,
                )
                .await
            {
                Ok(identity) => binding_identity = Some(identity),
                Err(error) => return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_string(), adapter_id: handle.adapter_id.to_string(),
                    reason: format!("original binding outcome remains uncertain; handle and reservation retained: {error}"),
                }),
            }
        }
        self.pending.lock().await.remove(worktree_id);
        map.insert(worktree_id.to_string(), handle.clone());
        if let (Some(durable), Some(reservation_id)) = (&self.durable, reservation_id) {
            durable.finish_reservation_delivery(reservation_id);
        }
        if binding_identity.is_none() {
            binding_identity = Some(transient_binding_identity(&handle));
        }
        if let Some(owner) = &transient_owner {
            self.finish_transient_acquisition(owner, &handle)?;
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
        self.snapshot_fenced(worktree_id, None, None).await.map(|(snapshot, _, _)| snapshot)
    }

    /// Restore a previously captured snapshot into a fresh microVM and REBIND
    /// the worktree to the restored handle. Durable registries must use
    /// [`Self::restore_warm_model`] so the snapshot's persisted binding,
    /// generation, and complete resource scope are authorized before restore.
    /// The raw snapshot seam remains available only to non-durable registries.
    pub async fn restore(
        &self,
        worktree_id: &str,
        snapshot: &SnapshotRef,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        if self.durable.is_some() {
            return Err(WorktreeVmError::SnapshotSourceMissing);
        }
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
        if self.durable.is_none() { self.reject_pending_transient_capture(worktree_id)?; }
        if map.contains_key(worktree_id) {
            return Err(WorktreeVmError::AlreadyBound {
                worktree_id: worktree_id.to_string(),
            });
        }
        let pending_handle = self.pending.lock().await.get(worktree_id).cloned();
        if let Some(handle) = pending_handle {
            let mut resume_original = false;
            if let Some(durable) = &self.durable {
                let _serial = durable.serializer(worktree_id)?.lock_owned().await;
                if let Some((binding, reservation_id)) = durable
                    .reconcile_known_reservation_binding(worktree_id, &handle, &serde_json::to_string(snapshot)?).await? {
                    let binding_id = binding.binding_id;
                    let generation = binding.generation;
                    let adopted = self.adopt_durable_binding(&mut map, binding).await?;
                    self.pending.lock().await.remove(worktree_id);
                    durable.finish_reservation_delivery(reservation_id);
                return Ok(WorktreeVmRestoreOutcome {
                    handle: adopted.clone(),
                    binding_identity: Some(WorktreeVmBindingIdentity {
                        binding_id, generation, process_handle: adopted,
                    }),
                });
                }
                if let Some(id) = durable.resume_reservation_for_source(
                    worktree_id, &serde_json::to_string(snapshot)?, &self.adapter,
                ).await? {
                    let owner = durable.retained_reservation(id)?;
                    if owner.handle.lock().unwrap_or_else(|error| error.into_inner()).as_ref() != Some(&handle) {
                        return Err(WorktreeVmError::StaleBinding {
                            worktree_id: worktree_id.to_string(), operation: "original pending acquisition",
                        });
                    }
                    resume_original = true;
                }
            }
            if !resume_original {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(),
                adapter_id: handle.adapter_id.to_string(),
                reason: "adapter restore completed but durable binding did not commit; original acquisition requires exact reconciliation before retry".to_string(),
            });
            }
        }

        let _durable_guard = if let Some(durable) = &self.durable {
            Some(durable.serializer(worktree_id)?.lock_owned().await)
        } else {
            None
        };
        let reservation_id = if let Some(durable) = &self.durable {
            if let Some(manifest) = manifest {
                durable.authorize_snapshot_source(manifest).await?;
            }
            if let Some(original) = durable.resume_reservation_for_source(
                worktree_id, &serde_json::to_string(snapshot)?, &self.adapter,
            ).await? {
                Some(original)
            } else {
            if durable
                .load_physical_key(worktree_id, true)
                .await?
                .is_some()
            {
                return Err(WorktreeVmError::AlreadyBound {
                    worktree_id: worktree_id.to_string(),
                });
            }
            {
                if !self.adapter.capabilities().supports_snapshot {
                    return Err(SandboxAdapterError::SnapshotUnsupported {
                        adapter_id: self.adapter.capabilities().adapter_id,
                    }.into());
                }
                Some(durable.reserve(worktree_id, serde_json::to_string(snapshot)?, &self.adapter).await?)
            }
            }
        } else {
            None
        };

        let transient_owner = if self.durable.is_none() {
            Some(self.begin_transient_acquisition(worktree_id, ReservationAcquisitionRequest::Restore(snapshot.clone()))?)
        } else { None };
        let handle = match (&self.durable, reservation_id) {
            (Some(durable), Some(reservation_id)) => durable.observe_reservation_acquisition(
                reservation_id, Arc::clone(&self.adapter), ReservationAcquisitionRequest::Restore(snapshot.clone()),
            ).await?,
            _ => self.observe_transient_acquisition(transient_owner.as_ref()
                .expect("original transient restore owner")).await?,
        };
        if let (Some(durable), Some(reservation_id)) = (&self.durable, reservation_id) {
            durable.retain_reservation_handle(reservation_id, &handle)?;
        }
        self.pending
            .lock()
            .await
            .insert(worktree_id.to_string(), handle.clone());
        let mut binding_identity = None;
        if let (Some(durable), Some(reservation_id)) = (&self.durable, reservation_id) {
            match durable
                .persist_reserved_handle(
                    worktree_id,
                    &handle,
                    Some(snapshot),
                    WorktreeVmBindingState::Snapshotted,
                    reservation_id,
                )
                .await
            {
                Ok(identity) => binding_identity = Some(identity),
                Err(error) => return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_string(), adapter_id: handle.adapter_id.to_string(),
                    reason: format!("original binding outcome remains uncertain; handle and reservation retained: {error}"),
                }),
            }
        }
        self.pending.lock().await.remove(worktree_id);
        map.insert(worktree_id.to_string(), handle.clone());
        if let (Some(durable), Some(reservation_id)) = (&self.durable, reservation_id) {
            durable.finish_reservation_delivery(reservation_id);
        }
        if binding_identity.is_none() {
            binding_identity = Some(transient_binding_identity(&handle));
        }
        if let Some(owner) = &transient_owner {
            self.finish_transient_acquisition(owner, &handle)?;
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
        let warm = Self::selected_snapshot_readiness(model_artifact_sha256, model_guest_path, ready)?;
        let (_, manifest, _) = self.snapshot_fenced(worktree_id, Some(warm), None).await?;
        manifest.ok_or(WorktreeVmError::SnapshotSourceMissing)
    }

    fn selected_snapshot_readiness(
        model_artifact_sha256: &str,
        model_guest_path: &str,
        ready: &WarmAgentGuestFrame,
    ) -> Result<SnapshotWarmInput, WorktreeVmError> {
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
        Ok(SnapshotWarmInput {
            model_artifact_sha256: model_artifact_sha256.to_string(),
            model_guest_path: model_guest_path.to_string(),
            ready_nonce: ready_nonce.to_string(),
        })
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
        self.teardown_worktree_vm_inner(worktree_id, None, None).await
    }

    pub async fn teardown_worktree_vm_if_current(
        &self,
        worktree_id: &str,
        expected: &WorktreeVmBindingIdentity,
    ) -> Result<(), WorktreeVmError> {
        self.teardown_worktree_vm_inner(worktree_id, Some(expected), None)
            .await
    }

    pub(crate) async fn teardown_worktree_vm_if_current_observed(
        &self, worktree_id: &str, expected: &WorktreeVmBindingIdentity,
        handoff: &super::factory::NativeTerminalHandoff,
    ) -> Result<(), WorktreeVmError> {
        {
            let retained = handoff.lock().unwrap_or_else(|error| error.into_inner());
            if let Some(observed) = retained.as_ref() {
                if observed.handle == expected.process_handle { return Ok(()); }
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
        }
        self.teardown_worktree_vm_inner(worktree_id, Some(expected), Some(handoff)).await?;
        if handoff.lock().unwrap_or_else(|error| error.into_inner()).is_none() {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(), adapter_id: expected.process_handle.adapter_id.to_string(),
                reason: "no original native terminal observation accompanied the teardown handback".into(),
            });
        }
        Ok(())
    }

    /// Reconcile adapter-local ownership after another registry/process has
    /// already terminalized the canonical durable row. A terminal database row
    /// proves the external VM should be gone; it does not release an exact local
    /// adapter handle or its process-global committed-memory reservation.
    async fn finish_terminal_teardown(
        &self,
        worktree_id: &str,
        handle: &ProcessHandle,
        native_handoff: Option<&super::factory::NativeTerminalHandoff>,
    ) -> Result<(), WorktreeVmError> {
        let scope = self.durable.as_ref().map(|store| store.scope_strings()).transpose()?;
        let key = sha256_hex(serde_json::to_string(&(scope, worktree_id, handle))?.as_bytes());
        let owner = RETAINED_REGISTRY_TEARDOWNS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).get(&key).cloned();
        let Some(owner) = owner else { return Ok(()); };
        let state = owner.state.lock().await;
        if owner.handle != *handle || !state.cleanup_complete || state.signal_future.is_some() {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: worktree_id.to_string(), operation: "terminal teardown handback",
            });
        }
        if let Some(handoff) = native_handoff {
            if !state.exit_code_observed {
                return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_string(), adapter_id: handle.adapter_id.to_string(),
                    reason: "original native exit-code observation is unavailable".into(),
                });
            }
            let terminal = state.terminal.clone().ok_or(WorktreeVmError::SnapshotSourceMissing)?;
            if !matches!(terminal, crate::sandbox::ProcessStatus::Exited { .. } | crate::sandbox::ProcessStatus::Killed { .. }) {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            *handoff.lock().unwrap_or_else(|error| error.into_inner()) = Some(super::factory::NativeSessionTerminal {
                handle: handle.clone(), terminal, exit_code: state.exit_code,
            });
        }
        let mut owners = RETAINED_REGISTRY_TEARDOWNS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if owners.get(&key).is_some_and(|current| Arc::ptr_eq(current, &owner)) {
            owners.remove(&key);
            owner.keepalive.lock().unwrap_or_else(|error| error.into_inner()).take();
        }
        Ok(())
    }

    async fn observe_original_teardown_signal(
        state: &mut RegistryTerminalState,
    ) -> Result<(), WorktreeVmError> {
        let Some(original) = state.signal_future.as_mut() else { return Ok(()); };
        state.signal = RegistrySignalAttempt::Attempted;
        let result = original.as_mut().await;
        let error = match result {
            Ok(observed) => {
                state.signal = RegistrySignalAttempt::Acknowledged;
                if let Some(terminal @ (crate::sandbox::ProcessStatus::Exited { .. }
                    | crate::sandbox::ProcessStatus::Killed { .. })) = observed {
                    state.terminal = Some(terminal);
                }
                None
            }
            Err(error) => {
                state.signal = RegistrySignalAttempt::Uncertain(error.to_string());
                Some(error)
            }
        };
        // Retain the original result before retiring its completed future.
        state.signal_future.take();
        if let Some(error) = error { return Err(error.into()); }
        Ok(())
    }

    async fn observe_terminal_teardown(
        &self,
        worktree_id: &str,
        handle: &ProcessHandle,
        binding_id: Uuid,
        generation: Option<i64>,
        detached: bool,
        allow_signal: bool,
    ) -> Result<(), WorktreeVmError> {
        let scope = self.durable.as_ref().map(|store| store.scope_strings()).transpose()?;
        let key = sha256_hex(serde_json::to_string(&(
            scope, worktree_id, handle,
        ))?.as_bytes());
        let owner = {
            let mut owners = RETAINED_REGISTRY_TEARDOWNS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner());
            Arc::clone(owners.entry(key).or_insert_with(|| Arc::new(RegistryTeardownOwner {
                adapter: Arc::downgrade(&self.adapter),
                keepalive: std::sync::Mutex::new(Some(Arc::clone(&self.adapter))),
                binding_id, generation, handle: handle.clone(),
                detached: detached.then(|| DetachedProcessIdentity {
                    process_uuid: binding_id, handle: handle.clone(),
                    executable_sha256: None, os_creation_time_100ns: None,
                }),
                store_identity: self.durable.as_ref().map(|store| Arc::clone(&store.schema_ready)),
                state: Mutex::new(RegistryTerminalState {
                    signal_future: None,
                    signal: RegistrySignalAttempt::NotAttempted, terminal: None,
                    exit_code: None, exit_code_observed: false, cleanup_complete: false,
                }),
            })))
        };
        let same_store = match (&owner.store_identity, &self.durable) {
            (None, None) => true,
            (Some(original), Some(current)) => Arc::ptr_eq(original, &current.schema_ready),
            _ => false,
        };
        let adapter = owner.adapter.upgrade().ok_or_else(|| WorktreeVmError::DurableHandleUnavailable {
            worktree_id: worktree_id.to_string(), adapter_id: handle.adapter_id.to_string(),
            reason: "original teardown adapter is unavailable; terminal ownership cannot be substituted".into(),
        })?;
        let same_generation = match (owner.generation, generation) {
            (Some(original), Some(current)) => original == current,
            (Some(_), None) | (None, None) => true,
            (None, Some(_)) => false,
        };
        if owner.handle != *handle || owner.binding_id != binding_id || !same_generation
            || !Arc::ptr_eq(&adapter, &self.adapter) || !same_store {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: worktree_id.to_string(), operation: "original teardown owner",
            });
        }
        let mut state = owner.state.lock().await;
        Self::observe_original_teardown_signal(&mut state).await?;
        if state.cleanup_complete {
            return Ok(());
        }
        if state.terminal.is_none() {
            let observed = match &owner.detached {
                Some(identity) => adapter.detached_status(identity).await?,
                None => adapter.status(handle).await?,
            };
            match observed {
                terminal @ (crate::sandbox::ProcessStatus::Exited { .. }
                    | crate::sandbox::ProcessStatus::Killed { .. }) => state.terminal = Some(terminal),
                crate::sandbox::ProcessStatus::Running => {
                    if allow_signal && matches!(state.signal, RegistrySignalAttempt::NotAttempted) {
                        // Reservation precedes native entry; cancellation cannot resend it.
                        let original_adapter = Arc::clone(&adapter);
                        let original_handle = handle.clone();
                        let original_detached = owner.detached.clone();
                        let registry = crate::storage::surreal::configured_capability_registry();
                        state.signal_future = Some(Box::pin(async move {
                            let native = async move {
                                match original_detached {
                                    Some(identity) => original_adapter
                                        .reclaim_detached_observed(&identity, Signal::Term).await
                                        .map(|outcome| match outcome {
                                            DetachedReclaimOutcome::Observed(status) => Some(status),
                                            DetachedReclaimOutcome::Terminated
                                            | DetachedReclaimOutcome::AlreadyAbsent => None,
                                        })
                                        .map_err(SandboxAdapterError::from),
                                    None => original_adapter.kill(&original_handle, Signal::Term).await.map(|()| None),
                                }
                            };
                            crate::storage::surreal::with_captured_capability_registry(registry, native).await
                        }));
                        Self::observe_original_teardown_signal(&mut state).await?;

                    }
                    let observed = match state.terminal.clone() {
                        Some(terminal) => terminal,
                        None => match &owner.detached {
                            Some(identity) => adapter.detached_status(identity).await?,
                            None => adapter.status(handle).await?,
                        },
                    };
                    match observed {
                        terminal @ (crate::sandbox::ProcessStatus::Exited { .. }
                            | crate::sandbox::ProcessStatus::Killed { .. }) => state.terminal = Some(terminal),
                        other => {
                            let signal = match &state.signal {
                                RegistrySignalAttempt::NotAttempted => "not requested".to_string(),
                                RegistrySignalAttempt::Attempted => "original signal return unobserved".to_string(),
                                RegistrySignalAttempt::Acknowledged => "acknowledged without terminal proof".to_string(),
                                RegistrySignalAttempt::Uncertain(reason) => format!("uncertain: {reason}"),
                            };
                            return Err(WorktreeVmError::DurableHandleUnavailable {
                                worktree_id: worktree_id.to_string(),
                                adapter_id: handle.adapter_id.to_string(),
                                reason: format!("terminal teardown remains pending ({other:?}); signal {signal}"),
                            });
                        }
                    }
                }
                other => return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_string(), adapter_id: handle.adapter_id.to_string(),
                    reason: format!("adapter did not prove live or terminal ownership: {other:?}"),
                }),
            }
        }
        if !state.exit_code_observed {
            state.exit_code = match state.terminal.as_ref().expect("observed terminal kind") {
                crate::sandbox::ProcessStatus::Exited { code } => Some(*code),
                crate::sandbox::ProcessStatus::Killed { .. } => adapter.exit_code(handle).await?,
                _ => unreachable!("only actual terminal observations are retained"),
            };
            state.exit_code_observed = true;
        }
        adapter.cleanup_terminal(handle).await?;
        state.cleanup_complete = true;

        Ok(())
    }

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

        let local_owner = local.is_some() || pending.is_some();
        if !local_owner {
            let scope = self.durable.as_ref().map(|store| store.scope_strings()).transpose()?;
            let key = sha256_hex(serde_json::to_string(&(
                scope, worktree_id, &binding.process_handle,
            ))?.as_bytes());
            if !RETAINED_REGISTRY_TEARDOWNS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner()).contains_key(&key) {
                // This verified terminal row has no remaining local teardown work.
                return Ok(());
            }
        }
        self.observe_terminal_teardown(
            worktree_id, &binding.process_handle, binding.binding_id,
            Some(binding.generation), !local_owner, false,
        ).await?;
        if pending.as_ref() == Some(&binding.process_handle) {
            let mut pending = self.pending.lock().await;
            if pending.get(worktree_id) == Some(&binding.process_handle) {
                pending.remove(worktree_id);
            }
        }
        Ok(())
    }

    async fn teardown_worktree_vm_inner(
        &self,
        worktree_id: &str,
        expected: Option<&WorktreeVmBindingIdentity>,
        native_handoff: Option<&super::factory::NativeTerminalHandoff>,
    ) -> Result<(), WorktreeVmError> {
        let mut map = self.persistent.lock().await;
        if let Some(durable) = &self.durable {
            let _durable_guard = durable.serializer(worktree_id)?.lock_owned().await;
            let row = durable.select_scoped_row(worktree_id).await?;
            self.guard_teardown_snapshot_capture(durable, worktree_id, row.as_ref())?;
            let binding = match row {
                Some(row) => durable.load_verified_row(worktree_id, false, row).await?,
                None => None,
            };
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
                    self.observe_terminal_teardown(
                        worktree_id, handle, *process_uuid,
                        expected.map(|identity| identity.generation), false, true,
                    ).await?;
                }
                if let Some((handle, _)) = cleanup {
                    if map.get(worktree_id) == Some(&handle) {
                        map.remove(worktree_id);
                    }
                    let mut pending = self.pending.lock().await;
                    if pending.get(worktree_id) == Some(&handle) {
                        pending.remove(worktree_id);
                    }
                    drop(pending);
                    self.finish_terminal_teardown(worktree_id, &handle, native_handoff).await?;
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
                let local = map.get(worktree_id).cloned();
                let pending = self.pending.lock().await.get(worktree_id).cloned();
                for candidate in [local.as_ref(), pending.as_ref()].into_iter().flatten() {
                    if candidate != &binding.process_handle {
                        return Err(WorktreeVmError::StaleBinding {
                            worktree_id: worktree_id.to_string(), operation: "live binding teardown",
                        });
                    }
                }
                self.observe_terminal_teardown(
                    worktree_id, &binding.process_handle, binding.binding_id,
                    Some(binding.generation), local.is_none() && pending.is_none(), true,
                ).await?;
                durable.mark_terminated(&binding).await?;
                if map.get(worktree_id) == Some(&binding.process_handle) {
                    map.remove(worktree_id);
                }
                let mut pending = self.pending.lock().await;
                if pending.get(worktree_id) == Some(&binding.process_handle) {
                    pending.remove(worktree_id);
                }
                drop(pending);
                self.finish_terminal_teardown(worktree_id, &binding.process_handle, native_handoff).await?;
                return Ok(());
            }

            let binding = binding
                .as_ref()
                .expect("absent durable binding returned through the reconciliation branch");
            self.reconcile_terminal_local_binding(&mut map, worktree_id, binding)
                .await?;
            if map.get(worktree_id) == Some(&binding.process_handle) {
                map.remove(worktree_id);
            }
            self.finish_terminal_teardown(worktree_id, &binding.process_handle, native_handoff).await?;
            return Ok(());
        }

        self.reject_pending_transient_capture(worktree_id)?;
        {
            let local_owners = self.transient_acquisitions.lock().unwrap_or_else(|error| error.into_inner());
            let retained = RETAINED_TRANSIENT_ACQUISITIONS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner());
            let owns_local = local_owners.get(worktree_id).is_some_and(|owner|
                Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
                    && Arc::ptr_eq(&owner.adapter, &self.adapter) && owner.worktree_id == worktree_id);
            if owns_local || retained.values().any(|owner|
                owner.worktree_id == worktree_id && Arc::ptr_eq(&owner.adapter, &self.adapter)) {
                return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_string(), adapter_id: self.adapter.capabilities().adapter_id.to_string(),
                    reason: "original transient acquisition retains custody; teardown cannot infer no work or adopt its handle".into(),
                });
            }
        }
        let local = map.get(worktree_id).cloned();
        let pending = self.pending.lock().await.get(worktree_id).cloned();
        if matches!((&local, &pending), (Some(local), Some(pending)) if local != pending) {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: worktree_id.to_string(), operation: "transient teardown",
            });
        }
        let handle = local.or(pending);
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
        let identity = transient_binding_identity(&handle);
        self.observe_terminal_teardown(
            worktree_id, &handle, identity.binding_id, Some(identity.generation), false, true,
        ).await?;
        if map.get(worktree_id) == Some(&handle) {
            map.remove(worktree_id);
        }
        let mut pending = self.pending.lock().await;
        if pending.get(worktree_id) == Some(&handle) {
            pending.remove(worktree_id);
        }
        drop(pending);
        self.finish_terminal_teardown(worktree_id, &handle, native_handoff).await?;
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
        let Some(durable) = &self.durable else {
            return map
                .get(worktree_id)
                .cloned()
                .ok_or_else(|| WorktreeVmError::NotBound {
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
    /// diagnostics and independent embedded-SurrealDB verification.
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
        if map
            .get(&binding.worktree_id)
            .is_some_and(|local| local != &binding.process_handle)
        {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: binding.worktree_id,
                operation: "durable handle adoption",
            });
        }
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

const WORKTREE_VM_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS worktree_vm_bindings SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS binding_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS worktree_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS adapter_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS process_handle_json ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS latest_snapshot_json ON worktree_vm_bindings TYPE option<string>;
DEFINE FIELD IF NOT EXISTS snapshot_cleanup_json ON worktree_vm_bindings TYPE option<string>;
DEFINE FIELD IF NOT EXISTS reservation_intent_json ON worktree_vm_bindings TYPE option<string>;
DEFINE FIELD IF NOT EXISTS binding_state ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS generation ON worktree_vm_bindings TYPE int;
DEFINE FIELD IF NOT EXISTS failure_reason ON worktree_vm_bindings TYPE option<string>;
DEFINE FIELD IF NOT EXISTS reservation_id ON worktree_vm_bindings TYPE option<string>;
DEFINE FIELD IF NOT EXISTS owner_account_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS actor_principal_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS authenticated_session_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS access_space_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS workspace_id ON worktree_vm_bindings TYPE string;
DEFINE FIELD IF NOT EXISTS updated_at_unix_ms ON worktree_vm_bindings TYPE int;
DEFINE FIELD OVERWRITE event_ledger_event_id ON TABLE worktree_vm_bindings TYPE record<kernel_event_ledger> ASSERT record::exists($value) REFERENCE ON DELETE REJECT;
DEFINE FIELD OVERWRITE event_ledger_event_type ON TABLE worktree_vm_bindings TYPE string;
DEFINE FIELD OVERWRITE event_ledger_payload_hash ON TABLE worktree_vm_bindings TYPE string;
DEFINE INDEX IF NOT EXISTS worktree_vm_binding_id_unique ON worktree_vm_bindings FIELDS binding_id UNIQUE;
"#;

const EVENT_RESERVED: &str = "TASK_INTENT_RECORDED";
const EVENT_STARTED: &str = "SESSION_STARTED";
const EVENT_RESTORED: &str = "TRACE_REPLAYED";
const EVENT_SNAPSHOTTED: &str = "ARTIFACT_STORED";
const EVENT_TERMINATED: &str = "SESSION_COMPLETED";
const EVENT_COMPENSATED: &str = "SESSION_FAILED";

#[derive(Debug, SurrealValue)]
struct EmptySurrealBindings {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct DurableScopeStrings {
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct RecordIdBindings {
    record_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ReserveBindingBindings {
    expected_generation: i64,
    prior_generation: Option<i64>,
    prior_binding_id: Option<String>,
    prior_event_id: Option<String>,
    prior_snapshot_json: Option<String>,
    prior_snapshot_cleanup_json: Option<String>,
    reservation_intent_json: String,
    reservation_intent_sha256: String,
    operation_id: String,
    record_id: String,
    reservation_id: String,
    worktree_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    updated_at_unix_ms: i64,
    event_id: String,
    event_type: String,
    event_payload_hash: String,
}

#[derive(Debug, SurrealValue)]
struct PersistReservedBindingBindings {
    expected_generation: i64,
    reservation_intent_json: Option<String>,
    record_id: String,
    reservation_id: String,
    binding_id: String,
    adapter_id: String,
    process_handle_json: String,
    latest_snapshot_json: Option<String>,
    binding_state: String,
    updated_at_unix_ms: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    event_id: String,
    event_type: String,
    event_payload_hash: String,
}

#[derive(Debug, SurrealValue)]
struct FencedBindingMutationBindings {
    record_id: String,
    binding_id: String,
    generation: i64,
    latest_snapshot_json: Option<String>,
    updated_at_unix_ms: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    event_id: String,
    event_type: String,
    event_payload_hash: String,
}

#[derive(Debug, SurrealValue)]
struct ReleaseReservationBindings {
    record_id: String,
    reservation_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    updated_at_unix_ms: i64,
    event_id: String,
    event_type: String,
    event_payload_hash: String,
}

#[derive(Debug, SurrealValue)]
struct SnapshotSourceBindings {
    binding_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredBindingRow {
    binding_id: String,
    worktree_id: String,
    adapter_id: String,
    process_handle_json: String,
    latest_snapshot_json: Option<String>,
    snapshot_cleanup_json: Option<String>,
    reservation_intent_json: Option<String>,
    binding_state: String,
    generation: i64,
    failure_reason: Option<String>,
    reservation_id: Option<String>,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    updated_at_unix_ms: i64,
    event_ledger_event_id: String,
    event_ledger_event_type: String,
    event_ledger_payload_hash: String,
}

#[derive(Debug, SurrealValue)]
struct ReceiptBindings {
    event_id: String,
    record_id: String,
    worktree_id: String,
    binding_id: String,
    binding_state: String,
    event_type: String,
    event_payload_hash: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct StoredReceiptRow {
    event_id: String,
    event_sequence: i64,
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
        self.require_scope()?;
        Ok(format!("worktree-vm:{worktree_id}"))
    }

    fn serializer(&self, worktree_id: &str) -> Result<Arc<Mutex<()>>, WorktreeVmError> {
        let key = self.serialization_key(worktree_id)?;
        let serializers = DURABLE_SERIALIZERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
        let mut serializers = serializers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        serializers.retain(|_, serializer| serializer.strong_count() > 0);
        if let Some(serializer) = serializers.get(&key).and_then(Weak::upgrade) {
            return Ok(serializer);
        }
        let serializer = Arc::new(Mutex::new(()));
        serializers.insert(key, Arc::downgrade(&serializer));
        Ok(serializer)
    }

    fn scope_strings(&self) -> Result<DurableScopeStrings, WorktreeVmError> {
        let scope = self.require_scope()?;
        Ok(DurableScopeStrings {
            owner_account_id: scope.owner_account_id.as_uuid().to_string(),
            actor_principal_id: scope.actor_principal_id.as_uuid().to_string(),
            authenticated_session_id: scope
                .authenticated_session
                .ok_or(WorktreeVmError::AuthenticatedSessionScopeRequired)?
                .as_uuid()
                .to_string(),
            access_space_id: scope
                .access_space
                .ok_or(WorktreeVmError::AccessSpaceScopeRequired)?
                .as_uuid()
                .to_string(),
            workspace_id: scope
                .workspace
                .as_ref()
                .ok_or(WorktreeVmError::WorkspaceScopeRequired)?
                .as_str()
                .to_owned(),
        })
    }

    fn record_id(&self, worktree_id: &str) -> Result<String, WorktreeVmError> {
        let mut digest = Sha256::new();
        digest.update(b"handshake.worktree-vm-binding.v1\0");
        digest.update(self.serialization_key(worktree_id)?.as_bytes());
        Ok(hex::encode(digest.finalize()))
    }

    fn event_digest(&self, domain: &[u8], values: &[&str]) -> Result<String, WorktreeVmError> {
        let scope = self.scope_strings()?;
        let mut digest = Sha256::new();
        digest.update(domain);
        for value in [
            scope.owner_account_id.as_str(),
            scope.actor_principal_id.as_str(),
            scope.authenticated_session_id.as_str(),
            scope.access_space_id.as_str(),
            scope.workspace_id.as_str(),
        ]
        .into_iter()
        .chain(values.iter().copied())
        {
            digest.update([0]);
            digest.update(value.as_bytes());
        }
        Ok(hex::encode(digest.finalize()))
    }

    fn event_id(
        &self,
        record_id: &str,
        event_type: &str,
        operation_identity: &str,
    ) -> Result<String, WorktreeVmError> {
        Ok(format!(
            "wvm-{}",
            self.event_digest(
                b"handshake.worktree-vm-event-id.v1",
                &[record_id, event_type, operation_identity],
            )?
        ))
    }

    fn event_payload_hash(
        &self,
        record_id: &str,
        row: &StoredBindingRow,
    ) -> Result<String, WorktreeVmError> {
        if row.binding_state == "reserved" {
            if let Some(raw) = &row.reservation_intent_json {
                let intent: OriginalVmReservationIntent = serde_json::from_str(raw)?;
                return Self::reservation_intent_payload_hash(&intent);
            }
        }
        if row.binding_state == "failed" && row.failure_reason.as_deref() == Some("reservation_released")
            && row.process_handle_json.is_empty() {
            if let Some(raw) = &row.reservation_intent_json {
                let intent: OriginalVmReservationIntent = serde_json::from_str(raw)?;
                return Self::reservation_state_payload_hash(&intent, "failed", EVENT_COMPENSATED);
            }
        }
        Ok(Self::event_payload_hash_for_values(
            record_id,
            &row.worktree_id,
            &row.binding_id,
            &row.binding_state,
            &row.event_ledger_event_type,
        ))
    }

    fn event_payload_hash_for_values(
        record_id: &str,
        worktree_id: &str,
        binding_id: &str,
        binding_state: &str,
        event_type: &str,
    ) -> String {
        let payload = serde_json::json!({
            "binding_id": binding_id,
            "binding_state": binding_state,
            "record_id": record_id,
            "transition_event_type": event_type,
            "worktree_id": worktree_id,
        });
        sha256_hex(&canonical_json_bytes(&payload))
    }

    fn validate_receipt_type(row: &StoredBindingRow) -> Result<(), WorktreeVmError> {
        let valid = match row.binding_state.as_str() {
            "reserved" => row.event_ledger_event_type == EVENT_RESERVED,
            "active" => row.event_ledger_event_type == EVENT_STARTED,
            "snapshotted" => matches!(
                row.event_ledger_event_type.as_str(),
                EVENT_RESTORED | EVENT_SNAPSHOTTED
            ),
            "terminated" => row.event_ledger_event_type == EVENT_TERMINATED,
            "failed" => row.event_ledger_event_type == EVENT_COMPENSATED,
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(WorktreeVmError::EventLedgerReceiptMismatch)
        }
    }

    fn scoped_record_bindings(
        &self,
        worktree_id: &str,
    ) -> Result<RecordIdBindings, WorktreeVmError> {
        let scope = self.scope_strings()?;
        Ok(RecordIdBindings {
            record_id: self.record_id(worktree_id)?,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        })
    }

    async fn ensure_schema(&self) -> Result<(), WorktreeVmError> {
        self.schema_ready
            .get_or_try_init(|| async {
                self.storage
                    .with_data_operation(|database| {
                        Box::pin(async move {
                            let _ = database
                                .query_values::<surrealdb::types::Value, _>(
                                    WORKTREE_VM_SCHEMA,
                                    EmptySurrealBindings {},
                                )
                                .await?;
                            Ok(())
                        })
                    })
                    .await
            })
            .await?;
        Ok(())
    }

    async fn verify_receipt_linkage(
        &self,
        worktree_id: &str,
        row: &StoredBindingRow,
    ) -> Result<(), WorktreeVmError> {
        Self::validate_receipt_type(row)?;
        let record_id = self.record_id(worktree_id)?;
        let expected_hash = self.event_payload_hash(&record_id, row)?;
        if row.worktree_id != worktree_id || row.event_ledger_payload_hash != expected_hash {
            return Err(WorktreeVmError::EventLedgerReceiptMismatch);
        }
        let scope = self.scope_strings()?;
        let bindings = ReceiptBindings {
            event_id: row.event_ledger_event_id.clone(),
            record_id,
            worktree_id: worktree_id.to_owned(),
            binding_id: row.binding_id.clone(),
            binding_state: row.binding_state.clone(),
            event_type: row.event_ledger_event_type.clone(),
            event_payload_hash: expected_hash,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredReceiptRow, _>(
                            r#"
                            SELECT event_id, event_sequence
                            FROM kernel_event_ledger
                            WHERE id = type::record('kernel_event_ledger', $event_id)
                              AND event_id = $event_id
                              AND idempotency_key = $event_id
                              AND event_version = 'kernel_event_v1'
                              AND kernel_task_run_id = $workspace_id
                              AND session_run_id = $authenticated_session_id
                              AND aggregate_type = 'worktree_vm_binding'
                              AND aggregate_id = $record_id
                              AND event_type = $event_type
                              AND actor_kind = 'operator'
                              AND actor_id = $actor_principal_id
                              AND payload_hash = $event_payload_hash
                              AND source_component = 'worktree_vm_registry'
                              AND payload.transition_event_type = $event_type
                              AND payload.record_id = $record_id
                              AND payload.worktree_id = $worktree_id
                              AND payload.binding_id = $binding_id
                              AND payload.binding_state = $binding_state
                              AND owner_account_id = $owner_account_id
                              AND actor_principal_id = $actor_principal_id
                              AND authenticated_session_id = $authenticated_session_id
                              AND access_space_id = $access_space_id
                              AND workspace_id = $workspace_id
                              AND event_sequence > 0
                            LIMIT 2;
                            "#,
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        match rows.as_slice() {
            [receipt]
                if receipt.event_id == row.event_ledger_event_id && receipt.event_sequence > 0 =>
            {
                Ok(())
            }
            [] => Err(WorktreeVmError::EventLedgerReceiptMissing),
            [_] => Err(WorktreeVmError::EventLedgerReceiptMismatch),
            _ => Err(WorktreeVmError::EventLedgerReceiptAmbiguous),
        }
    }

    async fn reserve(
        &self, worktree_id: &str, source_json: String, adapter: &Arc<dyn SandboxAdapter>,
    ) -> Result<Uuid, WorktreeVmError> {
        self.ensure_schema().await?;
        let scope = self.scope_strings()?;
        let prior = self.select_scoped_row(worktree_id).await?;
        if let Some(raw) = prior.as_ref().and_then(|row| row.snapshot_cleanup_json.as_deref()) {
            let disposition: SnapshotDispositionRecord = serde_json::from_str(raw)?;
            if !matches!(disposition.phase, SnapshotDispositionPhase::Delivered
                | SnapshotDispositionPhase::Deleted | SnapshotDispositionPhase::Cancelled) {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
        }
        let retained = RETAINED_VM_RESERVATIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).values().find(|owner| {
                owner.intent.worktree_id == worktree_id
                    && Arc::ptr_eq(&owner.durable.schema_ready, &self.schema_ready)
            }).cloned();
        if let Some(owner) = retained.as_ref() {
            self.bind_original_warm_owner(owner)?;
            if !Arc::ptr_eq(&owner.durable.schema_ready, &self.schema_ready)
                || owner.intent.scope != scope {
                return Err(WorktreeVmError::ScopeDenied(ScopeDenied::ExactAttributionMismatch));
            }
            let acquisition = owner.acquisition.lock().await;
            if !acquisition.adapter.as_ref().is_some_and(|original| Arc::ptr_eq(original, adapter)) {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
        }
        if let Some(owner) = retained.as_ref() {
            if !owner.invoked.load(std::sync::atomic::Ordering::Acquire)
                && owner.intent.source_sha256 == sha256_hex(source_json.as_bytes()) {
                if let Some(row) = &prior {
                    if row.reservation_id.as_deref() == Some(owner.intent.reservation_id.to_string().as_str())
                        && row.generation == owner.intent.expected_generation
                        && row.reservation_intent_json.as_deref() == Some(serde_json::to_string(&owner.intent)?.as_str()) {
                        return Ok(owner.intent.reservation_id);
                    }
                }
            }
            let same_prior = match &prior {
                None => owner.intent.prior_generation.is_none(),
                Some(row) => matches!(row.binding_state.as_str(), "terminated" | "failed")
                    && owner.intent.prior_generation == Some(row.generation)
                    && owner.intent.prior_binding_id.as_ref() == Some(&row.binding_id)
                    && owner.intent.prior_event_id.as_ref() == Some(&row.event_ledger_event_id)
                    && owner.intent.prior_snapshot_json == row.latest_snapshot_json
                    && owner.intent.prior_snapshot_cleanup_json == row.snapshot_cleanup_json,
            };
            if owner.invoked.load(std::sync::atomic::Ordering::Acquire)
                || owner.intent.source_sha256 != sha256_hex(source_json.as_bytes())
                || !same_prior {
                return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_owned(), adapter_id: String::new(),
                    reason: "original reservation operation is still retained; no new native attempt is admitted".into(),
                });
            }
        }
        let reservation_id = retained.as_ref().map_or_else(Uuid::now_v7, |owner| owner.intent.reservation_id);
        let reservation_text = reservation_id.to_string();
        let record_id = self.record_id(worktree_id)?;
        let expected_generation = retained.as_ref().map(|owner| owner.intent.expected_generation)
            .or_else(|| prior.as_ref().map_or(Some(1), |row| row.generation.checked_add(1)))
            .filter(|generation| *generation > 0)
            .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
        let intent = if let Some(owner) = &retained {
            owner.intent.clone()
        } else { OriginalVmReservationIntent {
            version: 1, operation_id: Uuid::now_v7(), reservation_id,
            record_id: record_id.clone(), worktree_id: worktree_id.to_owned(),
            expected_generation, source_sha256: sha256_hex(source_json.as_bytes()), scope: scope.clone(),
            prior_generation: prior.as_ref().map(|row| row.generation),
            prior_binding_id: prior.as_ref().map(|row| row.binding_id.clone()),
            prior_event_id: prior.as_ref().map(|row| row.event_ledger_event_id.clone()),
            prior_snapshot_json: prior.as_ref().and_then(|row| row.latest_snapshot_json.clone()),
            prior_snapshot_cleanup_json: prior.as_ref().and_then(|row| row.snapshot_cleanup_json.clone()),
        } };
        let intent_json = serde_json::to_string(&intent)?;
        let owner = retained.unwrap_or_else(|| Arc::new(RetainedVmReservation {
            durable: self.clone(), intent: intent.clone(),
            invoked: std::sync::atomic::AtomicBool::new(false),
            handle: std::sync::Mutex::new(None),
            acquisition: Mutex::new(ReservationAcquisitionProgress {
                adapter: Some(Arc::clone(adapter)), original: None, failed: None,
            }),
            registry: crate::storage::surreal::configured_capability_registry(),
            warm_owner: ORIGINAL_WARM_RESERVATION_OWNER.try_with(Arc::downgrade).ok(),
            release_claimed: std::sync::atomic::AtomicBool::new(false),
        }));
        self.bind_original_warm_owner(&owner)?;
        RETAINED_VM_RESERVATIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).insert(reservation_id, owner);
        let event_type = EVENT_RESERVED.to_owned();
        let event_id = self.event_id(&record_id, &event_type, &reservation_text)?;
        let event_payload_hash = Self::reservation_intent_payload_hash(&intent)?;
        let bindings = ReserveBindingBindings {
            expected_generation,
            prior_generation: intent.prior_generation,
            prior_binding_id: intent.prior_binding_id.clone(),
            prior_event_id: intent.prior_event_id.clone(),
            prior_snapshot_json: intent.prior_snapshot_json.clone(),
            prior_snapshot_cleanup_json: intent.prior_snapshot_cleanup_json.clone(),
            reservation_intent_json: intent_json.clone(),
            reservation_intent_sha256: sha256_hex(intent_json.as_bytes()),
            operation_id: intent.operation_id.to_string(),
            record_id,
            reservation_id: reservation_text.clone(),
            worktree_id: worktree_id.to_owned(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            updated_at_unix_ms: chrono::Utc::now().timestamp_millis(),
            event_id,
            event_type,
            event_payload_hash,
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredBindingRow, _>(
                            r#"
                            BEGIN TRANSACTION;
                            LET $record = type::record('worktree_vm_bindings', $record_id);
                            LET $existing = (SELECT binding_id, worktree_id, adapter_id,
                                process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                generation, failure_reason, reservation_id, owner_account_id,
                                actor_principal_id, authenticated_session_id, access_space_id,
                                workspace_id, updated_at_unix_ms,
                                record::id(event_ledger_event_id) AS event_ledger_event_id,
                                event_ledger_event_type, event_ledger_payload_hash FROM $record
                                WHERE owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                  AND event_ledger_event_id.owner_account_id = $owner_account_id
                                  AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                  AND event_ledger_event_id.access_space_id = $access_space_id
                                  AND event_ledger_event_id.workspace_id = $workspace_id
                                  AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                  AND event_ledger_event_id.aggregate_id = $record_id
                                  AND event_ledger_event_id.event_type = event_ledger_event_type
                                  AND event_ledger_event_id.payload.transition_event_type = event_ledger_event_type
                                  AND event_ledger_event_id.payload_hash = event_ledger_payload_hash
                                  AND event_ledger_event_id.payload.record_id = $record_id
                                  AND event_ledger_event_id.payload.worktree_id = worktree_id
                                  AND event_ledger_event_id.payload.binding_id = binding_id
                                  AND event_ledger_event_id.payload.binding_state = binding_state);
                            IF (array::len($existing) = 0 AND $prior_generation = NONE)
                               OR (array::len($existing) = 1
                                 AND $existing[0].binding_state IN ['terminated', 'failed']
                                 AND $existing[0].generation = $prior_generation
                                 AND $existing[0].binding_id = $prior_binding_id
                                 AND $existing[0].event_ledger_event_id = $prior_event_id
                                 AND $existing[0].latest_snapshot_json = $prior_snapshot_json
                                 AND $existing[0].snapshot_cleanup_json = $prior_snapshot_cleanup_json) {
                                LET $next_generation = $expected_generation;
                                LET $prior = (SELECT event_id FROM kernel_event_ledger
                                    WHERE idempotency_key = $event_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id LIMIT 2);
                                IF array::len($prior) != 0 { THROW 'worktree VM reservation EventLedger receipt is orphaned or ambiguous'; };
                                LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                                    event_id: $event_id, event_version: 'kernel_event_v1',
                                    kernel_task_run_id: $workspace_id,
                                    session_run_id: $authenticated_session_id,
                                    aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                                    idempotency_key: $event_id, event_type: $event_type,
                                    actor_kind: 'operator', actor_id: $actor_principal_id,
                                    causation_id: IF array::len($existing) = 0 { NONE } ELSE { $existing[0].event_ledger_event_id },
                                    correlation_id: $record_id, payload_hash: $event_payload_hash,
                                    source_component: 'worktree_vm_registry',
                                    payload: { transition_event_type: $event_type,
                                        record_id: $record_id, worktree_id: $worktree_id,
                                        binding_id: $reservation_id,
                                        binding_state: 'reserved',
                                        operation_id: $operation_id,
                                        reservation_intent_sha256: $reservation_intent_sha256,
                                        expected_generation: $expected_generation },
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id, workspace_id: $workspace_id,
                                    created_at: time::now()
                                };
                                LET $stored = IF array::len($existing) = 0 {
                                  CREATE $record CONTENT {
                                    binding_id: $reservation_id, worktree_id: $worktree_id,
                                    adapter_id: '', process_handle_json: '',
                                    latest_snapshot_json: NONE, snapshot_cleanup_json: NONE, binding_state: 'reserved',
                                    reservation_intent_json: $reservation_intent_json,
                                    generation: $next_generation, failure_reason: NONE,
                                    reservation_id: $reservation_id,
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id,
                                    workspace_id: $workspace_id,
                                    updated_at_unix_ms: $updated_at_unix_ms,
                                    event_ledger_event_id: type::record('kernel_event_ledger', $event_id),
                                    event_ledger_event_type: $event_type,
                                    event_ledger_payload_hash: $event_payload_hash
                                  }
                                } ELSE {
                                  UPDATE $record CONTENT {
                                    binding_id: $reservation_id, worktree_id: $worktree_id,
                                    adapter_id: '', process_handle_json: '',
                                    latest_snapshot_json: NONE, snapshot_cleanup_json: NONE, binding_state: 'reserved',
                                    reservation_intent_json: $reservation_intent_json,
                                    generation: $next_generation,
                                    failure_reason: NONE, reservation_id: $reservation_id,
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id,
                                    workspace_id: $workspace_id,
                                    updated_at_unix_ms: $updated_at_unix_ms,
                                    event_ledger_event_id: type::record('kernel_event_ledger', $event_id),
                                    event_ledger_event_type: $event_type,
                                    event_ledger_payload_hash: $event_payload_hash
                                }
                                WHERE owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                };
                                LET $verified = (SELECT binding_id, worktree_id, adapter_id,
                                    process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                    generation, failure_reason, reservation_id, owner_account_id,
                                    actor_principal_id, authenticated_session_id, access_space_id,
                                    workspace_id, updated_at_unix_ms,
                                    record::id(event_ledger_event_id) AS event_ledger_event_id,
                                    event_ledger_event_type, event_ledger_payload_hash FROM $record
                                    WHERE owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id
                                      AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id)
                                      AND event_ledger_event_id.owner_account_id = $owner_account_id
                                      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                      AND event_ledger_event_id.access_space_id = $access_space_id
                                      AND event_ledger_event_id.workspace_id = $workspace_id
                                      AND event_ledger_event_id.event_id = $event_id
                                      AND event_ledger_event_id.idempotency_key = $event_id
                                      AND event_ledger_event_id.event_version = 'kernel_event_v1'
                                      AND event_ledger_event_id.kernel_task_run_id = $workspace_id
                                      AND event_ledger_event_id.session_run_id = $authenticated_session_id
                                      AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                      AND event_ledger_event_id.aggregate_id = $record_id
                                      AND event_ledger_event_id.event_type = $event_type
                                      AND event_ledger_event_id.payload.transition_event_type = $event_type
                                      AND event_ledger_event_id.actor_kind = 'operator'
                                      AND event_ledger_event_id.actor_id = $actor_principal_id
                                      AND event_ledger_event_id.source_component = 'worktree_vm_registry'
                                      AND event_ledger_event_id.payload_hash = $event_payload_hash
                                    LIMIT 2);
                                IF array::len($verified) != 1 { THROW 'worktree VM reservation receipt verification failed'; };
                                RETURN $verified;
                            } ELSE {
                                RETURN $existing;
                            };
                            COMMIT TRANSACTION;
                            "#,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await;
        let rows = match rows {
            Ok(rows) => rows,
            Err(SurrealStorageError::Database(error))
                if matches!(
                    error.already_exists_details(),
                    Some(surrealdb::types::AlreadyExistsError::Record { .. })
                ) =>
            {
                match self.select_scoped_row(worktree_id).await? {
                    Some(row) => vec![row],
                    None => {
                        return Err(WorktreeVmError::ScopeDenied(
                            ScopeDenied::ExactAttributionMismatch,
                        ));
                    }
                }
            }
            Err(error) => match self.select_scoped_row(worktree_id).await {
                Ok(Some(row)) if row.reservation_id.as_deref() == Some(reservation_text.as_str())
                    && row.binding_id == reservation_text
                    && row.generation == expected_generation
                    && row.reservation_intent_json.as_deref() == Some(intent_json.as_str()) => vec![row],
                _ => return Err(error.into()),
            },
        };
        let row = rows.into_iter().next().ok_or(WorktreeVmError::ScopeDenied(
            ScopeDenied::ExactAttributionMismatch,
        ))?;
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(worktree_id, &row).await?;
        if row.binding_state == "reserved"
            && row.reservation_id.as_deref() == Some(reservation_text.as_str())
            && row.generation == expected_generation
            && row.reservation_intent_json.as_deref() == Some(intent_json.as_str())
        {
            return Ok(reservation_id);
        }
        Err(WorktreeVmError::DurableHandleUnavailable {
            worktree_id: worktree_id.to_owned(),
            adapter_id: row.adapter_id,
            reason: format!(
                "embedded SurrealDB binding is already owned in state {}",
                row.binding_state
            ),
        })
    }

    async fn release_reservation(
        &self,
        worktree_id: &str,
        reservation_id: Uuid,
    ) -> Result<(), WorktreeVmError> {
        self.ensure_schema().await?;
        if let Some(row) = self.select_scoped_row(worktree_id).await? {
            if row.reservation_intent_json.is_some() {
                return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_owned(), adapter_id: row.adapter_id,
                    reason: "intent-backed reservation requires exact original no-effect/terminal reconciliation".into(),
                });
            }
        }
        let scope = self.scope_strings()?;
        let record_id = self.record_id(worktree_id)?;
        let reservation_text = reservation_id.to_string();
        let event_type = EVENT_COMPENSATED.to_owned();
        let event_id = self.event_id(&record_id, &event_type, &reservation_text)?;
        let event_payload_hash = Self::event_payload_hash_for_values(
            &record_id,
            worktree_id,
            &reservation_text,
            "failed",
            &event_type,
        );
        let bindings = ReleaseReservationBindings {
            record_id,
            reservation_id: reservation_text,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            updated_at_unix_ms: chrono::Utc::now().timestamp_millis(),
            event_id,
            event_type,
            event_payload_hash,
        };
        let rows = self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredBindingRow, _>(
                            r#"
                            BEGIN TRANSACTION;
                            LET $record = type::record('worktree_vm_bindings', $record_id);
                            LET $current = (SELECT binding_id, worktree_id, adapter_id,
                                process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                generation, failure_reason, reservation_id, owner_account_id,
                                actor_principal_id, authenticated_session_id, access_space_id,
                                workspace_id, updated_at_unix_ms,
                                record::id(event_ledger_event_id) AS event_ledger_event_id,
                                event_ledger_event_type, event_ledger_payload_hash FROM $record
                                WHERE owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                  AND event_ledger_event_id.owner_account_id = $owner_account_id
                                  AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                  AND event_ledger_event_id.access_space_id = $access_space_id
                                  AND event_ledger_event_id.workspace_id = $workspace_id
                                  AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                  AND event_ledger_event_id.aggregate_id = $record_id
                                  AND event_ledger_event_id.event_type = event_ledger_event_type
                                  AND event_ledger_event_id.payload_hash = event_ledger_payload_hash
                                  AND event_ledger_event_id.payload.record_id = $record_id
                                  AND event_ledger_event_id.payload.worktree_id = worktree_id
                                  AND event_ledger_event_id.payload.binding_id = binding_id
                                  AND event_ledger_event_id.payload.binding_state = binding_state);
                            IF array::len($current) = 1
                               AND $current[0].binding_state = 'failed'
                               AND $current[0].event_ledger_event_id = $event_id {
                                RETURN $current;
                            } ELSE IF array::len($current) = 1
                               AND $current[0].binding_state = 'reserved'
                               AND $current[0].reservation_id = $reservation_id
                               AND $current[0].reservation_intent_json = NONE {
                                LET $prior = (SELECT event_id FROM kernel_event_ledger
                                    WHERE idempotency_key = $event_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id LIMIT 2);
                                IF array::len($prior) != 0 { THROW 'worktree VM compensation EventLedger receipt is orphaned or ambiguous'; };
                                LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                                    event_id: $event_id, event_version: 'kernel_event_v1',
                                    kernel_task_run_id: $workspace_id,
                                    session_run_id: $authenticated_session_id,
                                    aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                                    idempotency_key: $event_id, event_type: $event_type,
                                    actor_kind: 'operator', actor_id: $actor_principal_id,
                                    causation_id: $current[0].event_ledger_event_id,
                                    correlation_id: $record_id, payload_hash: $event_payload_hash,
                                    source_component: 'worktree_vm_registry',
                                    payload: { transition_event_type: $event_type,
                                        record_id: $record_id, worktree_id: $current[0].worktree_id,
                                        binding_id: $reservation_id,
                                        binding_state: 'failed' },
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id, workspace_id: $workspace_id,
                                    created_at: time::now()
                                };
                                LET $stored = UPDATE $record SET
                                    binding_state = 'failed', failure_reason = 'reservation_released',
                                    reservation_id = NONE, updated_at_unix_ms = $updated_at_unix_ms,
                                    event_ledger_event_id = type::record('kernel_event_ledger', $event_id),
                                    event_ledger_event_type = $event_type,
                                    event_ledger_payload_hash = $event_payload_hash
                                    WHERE reservation_intent_json = NONE AND binding_state = 'reserved'
                                      AND reservation_id = $reservation_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id;
                                LET $verified = (SELECT binding_id, worktree_id, adapter_id,
                                    process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                    generation, failure_reason, reservation_id, owner_account_id,
                                    actor_principal_id, authenticated_session_id, access_space_id,
                                    workspace_id, updated_at_unix_ms,
                                    record::id(event_ledger_event_id) AS event_ledger_event_id,
                                    event_ledger_event_type, event_ledger_payload_hash FROM $record
                                    WHERE owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id
                                      AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id)
                                      AND event_ledger_event_id.owner_account_id = $owner_account_id
                                      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                      AND event_ledger_event_id.access_space_id = $access_space_id
                                      AND event_ledger_event_id.workspace_id = $workspace_id
                                      AND event_ledger_event_id.event_id = $event_id
                                      AND event_ledger_event_id.idempotency_key = $event_id
                                      AND event_ledger_event_id.event_version = 'kernel_event_v1'
                                      AND event_ledger_event_id.kernel_task_run_id = $workspace_id
                                      AND event_ledger_event_id.session_run_id = $authenticated_session_id
                                      AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                      AND event_ledger_event_id.aggregate_id = $record_id
                                      AND event_ledger_event_id.event_type = $event_type
                                      AND event_ledger_event_id.actor_kind = 'operator'
                                      AND event_ledger_event_id.actor_id = $actor_principal_id
                                      AND event_ledger_event_id.source_component = 'worktree_vm_registry'
                                      AND event_ledger_event_id.payload_hash = $event_payload_hash
                                    LIMIT 2);
                                IF array::len($verified) != 1 { THROW 'worktree VM compensation receipt verification failed'; };
                                RETURN $verified;
                            } ELSE {
                                RETURN [];
                            };
                            COMMIT TRANSACTION;
                            "#,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| WorktreeVmError::StaleBinding {
                worktree_id: worktree_id.to_owned(),
                operation: "reservation compensation",
            })?;
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(worktree_id, &row).await?;
        Ok(())
    }

    async fn persist_reserved_handle(
        &self,
        worktree_id: &str,
        handle: &ProcessHandle,
        snapshot: Option<&SnapshotRef>,
        state: WorktreeVmBindingState,
        reservation_id: Uuid,
    ) -> Result<WorktreeVmBindingIdentity, WorktreeVmError> {
        self.ensure_schema().await?;
        let scope = self.scope_strings()?;
        // The canonical binding identity is the immutable sandbox process UUID.
        // Replaying the same reservation/handle therefore reuses the same
        // mutation and EventLedger idempotency identity after an uncertain reply.
        let original_row = self.select_scoped_row(worktree_id).await?
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        let expected_generation = original_row.generation;
        let reservation_intent_json = original_row.reservation_intent_json.clone();
        if let Some(raw) = &reservation_intent_json {
            let intent: OriginalVmReservationIntent = serde_json::from_str(raw)?;
            if intent.reservation_id != reservation_id || intent.expected_generation != expected_generation {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
        }
        if original_row.binding_id == handle.id.to_string()
            && original_row.process_handle_json == serde_json::to_string(handle)?
            && matches!(original_row.binding_state.as_str(), "active" | "snapshotted") {
            if let Some(raw) = &reservation_intent_json {
                let intent: OriginalVmReservationIntent = serde_json::from_str(raw)?;
                let owner = self.retained_reservation(reservation_id)?;
                if serde_json::to_string(&owner.intent)? != *raw
                    || owner.handle.lock().unwrap_or_else(|error| error.into_inner()).as_ref() != Some(handle)
                    || intent.reservation_id != reservation_id {
                    return Err(WorktreeVmError::SnapshotSourceMismatch);
                }
            }
            return Ok(WorktreeVmBindingIdentity {
                binding_id: handle.id, generation: original_row.generation,
                process_handle: handle.clone(),
            });
        }
        let binding_id = handle.id;
        let binding_text = binding_id.to_string();
        let record_id = self.record_id(worktree_id)?;
        let process_handle_json = serde_json::to_string(handle)?;
        let latest_snapshot_json = snapshot.map(serde_json::to_string).transpose()?;
        let event_type = match state {
            WorktreeVmBindingState::Active => EVENT_STARTED,
            WorktreeVmBindingState::Snapshotted => EVENT_RESTORED,
            _ => {
                return Err(WorktreeVmError::InvalidPersistedBinding {
                    reason: "reserved binding may only transition to active or restored".to_owned(),
                });
            }
        }
        .to_owned();
        let event_id = self.event_id(&record_id, &event_type, &binding_text)?;
        let event_payload_hash = Self::event_payload_hash_for_values(
            &record_id,
            worktree_id,
            &binding_text,
            state.as_str(),
            &event_type,
        );
        let bindings = PersistReservedBindingBindings {
            expected_generation,
            reservation_intent_json: reservation_intent_json.clone(),
            record_id,
            reservation_id: reservation_id.to_string(),
            binding_id: binding_text,
            adapter_id: handle.adapter_id.as_str().to_owned(),
            process_handle_json,
            latest_snapshot_json,
            binding_state: state.as_str().to_owned(),
            updated_at_unix_ms: chrono::Utc::now().timestamp_millis(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            event_id,
            event_type,
            event_payload_hash,
        };
        let rows_result = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredBindingRow, _>(
                            r#"
                            BEGIN TRANSACTION;
                            LET $record = type::record('worktree_vm_bindings', $record_id);
                            LET $current = (SELECT binding_id, worktree_id, adapter_id,
                                process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                generation, failure_reason, reservation_id, owner_account_id,
                                actor_principal_id, authenticated_session_id, access_space_id,
                                workspace_id, updated_at_unix_ms,
                                record::id(event_ledger_event_id) AS event_ledger_event_id,
                                event_ledger_event_type, event_ledger_payload_hash FROM $record
                                WHERE owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                  AND event_ledger_event_id.owner_account_id = $owner_account_id
                                  AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                  AND event_ledger_event_id.access_space_id = $access_space_id
                                  AND event_ledger_event_id.workspace_id = $workspace_id
                                  AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                  AND event_ledger_event_id.aggregate_id = $record_id
                                  AND event_ledger_event_id.event_type = event_ledger_event_type
                                  AND event_ledger_event_id.payload_hash = event_ledger_payload_hash
                                  AND event_ledger_event_id.payload.record_id = $record_id
                                  AND event_ledger_event_id.payload.worktree_id = worktree_id
                                  AND event_ledger_event_id.payload.binding_id = binding_id
                                  AND event_ledger_event_id.payload.binding_state = binding_state);
                            IF array::len($current) = 1
                               AND $current[0].binding_state = $binding_state
                               AND $current[0].binding_id = $binding_id
                               AND $current[0].generation = $expected_generation
                               AND $current[0].reservation_intent_json = $reservation_intent_json
                               AND $current[0].process_handle_json = $process_handle_json
                               AND $current[0].latest_snapshot_json = $latest_snapshot_json
                               AND $current[0].event_ledger_event_id = $event_id {
                                RETURN $current;
                            } ELSE IF array::len($current) = 1
                               AND $current[0].binding_state = 'reserved'
                               AND $current[0].reservation_id = $reservation_id
                               AND $current[0].generation = $expected_generation
                               AND $current[0].reservation_intent_json = $reservation_intent_json {
                                LET $prior = (SELECT event_id FROM kernel_event_ledger
                                    WHERE idempotency_key = $event_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id LIMIT 2);
                                IF array::len($prior) != 0 { THROW 'worktree VM bind EventLedger receipt is orphaned or ambiguous'; };
                                LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                                    event_id: $event_id, event_version: 'kernel_event_v1',
                                    kernel_task_run_id: $workspace_id,
                                    session_run_id: $authenticated_session_id,
                                    aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                                    idempotency_key: $event_id, event_type: $event_type,
                                    actor_kind: 'operator', actor_id: $actor_principal_id,
                                    causation_id: $current[0].event_ledger_event_id,
                                    correlation_id: $record_id, payload_hash: $event_payload_hash,
                                    source_component: 'worktree_vm_registry',
                                    payload: { transition_event_type: $event_type,
                                        record_id: $record_id, worktree_id: $current[0].worktree_id,
                                        binding_id: $binding_id,
                                        binding_state: $binding_state },
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id, workspace_id: $workspace_id,
                                    created_at: time::now()
                                };
                                LET $stored = UPDATE $record SET
                                    binding_id = $binding_id, adapter_id = $adapter_id,
                                    process_handle_json = $process_handle_json,
                                    latest_snapshot_json = $latest_snapshot_json,
                                    binding_state = $binding_state, failure_reason = NONE,
                                    reservation_id = NONE, updated_at_unix_ms = $updated_at_unix_ms,
                                    event_ledger_event_id = type::record('kernel_event_ledger', $event_id),
                                    event_ledger_event_type = $event_type,
                                    event_ledger_payload_hash = $event_payload_hash
                                    WHERE generation = $expected_generation
                                      AND reservation_intent_json = $reservation_intent_json
                                      AND binding_state = 'reserved'
                                      AND reservation_id = $reservation_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id;
                                LET $verified = (SELECT binding_id, worktree_id, adapter_id,
                                    process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                    generation, failure_reason, reservation_id, owner_account_id,
                                    actor_principal_id, authenticated_session_id, access_space_id,
                                    workspace_id, updated_at_unix_ms,
                                    record::id(event_ledger_event_id) AS event_ledger_event_id,
                                    event_ledger_event_type, event_ledger_payload_hash FROM $record
                                    WHERE binding_id = $binding_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id
                                      AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id)
                                      AND event_ledger_event_id.owner_account_id = $owner_account_id
                                      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                      AND event_ledger_event_id.access_space_id = $access_space_id
                                      AND event_ledger_event_id.workspace_id = $workspace_id
                                      AND event_ledger_event_id.event_id = $event_id
                                      AND event_ledger_event_id.idempotency_key = $event_id
                                      AND event_ledger_event_id.event_version = 'kernel_event_v1'
                                      AND event_ledger_event_id.kernel_task_run_id = $workspace_id
                                      AND event_ledger_event_id.session_run_id = $authenticated_session_id
                                      AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                      AND event_ledger_event_id.aggregate_id = $record_id
                                      AND event_ledger_event_id.event_type = $event_type
                                      AND event_ledger_event_id.actor_kind = 'operator'
                                      AND event_ledger_event_id.actor_id = $actor_principal_id
                                      AND event_ledger_event_id.source_component = 'worktree_vm_registry'
                                      AND event_ledger_event_id.payload_hash = $event_payload_hash
                                    LIMIT 2);
                                IF array::len($verified) != 1 { THROW 'worktree VM bind receipt verification failed'; };
                                RETURN $verified;
                            } ELSE {
                                RETURN [];
                            };
                            COMMIT TRANSACTION;
                            "#,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await;
        let rows = match rows_result {
            Ok(rows) => rows,
            Err(primary) => match self.select_scoped_row(worktree_id).await {
                Ok(Some(row)) if row.binding_id == binding_id.to_string()
                    && row.generation == expected_generation
                    && row.binding_state == state.as_str()
                    && row.process_handle_json == serde_json::to_string(handle)?
                    && row.latest_snapshot_json == snapshot.map(serde_json::to_string).transpose()?
                    && row.reservation_intent_json == reservation_intent_json => vec![row],
                _ => return Err(primary.into()),
            },
        };
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| WorktreeVmError::StaleBinding {
                worktree_id: worktree_id.to_owned(),
                operation: "durable SurrealDB bind",
            })?;
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(worktree_id, &row).await?;
        Ok(WorktreeVmBindingIdentity {
            binding_id,
            generation: row.generation,
            process_handle: handle.clone(),
        })
    }
    async fn record_snapshot(
        &self,
        binding: &WorktreeVmBindingRecord,
        snapshot: &SnapshotRef,
    ) -> Result<(), WorktreeVmError> {
        self.ensure_schema().await?;
        let scope = self.scope_strings()?;
        let record_id = self.record_id(&binding.worktree_id)?;
        let binding_text = binding.binding_id.to_string();
        let latest_snapshot_json = serde_json::to_string(snapshot)?;
        let event_type = EVENT_SNAPSHOTTED.to_owned();
        let operation_identity = self.event_digest(
            b"handshake.worktree-vm-snapshot-operation.v1",
            &[binding_text.as_str(), latest_snapshot_json.as_str()],
        )?;
        let event_id = self.event_id(&record_id, &event_type, &operation_identity)?;
        let event_payload_hash = Self::event_payload_hash_for_values(
            &record_id,
            &binding.worktree_id,
            &binding_text,
            "snapshotted",
            &event_type,
        );
        let bindings = FencedBindingMutationBindings {
            record_id,
            binding_id: binding_text,
            generation: binding.generation,
            latest_snapshot_json: Some(latest_snapshot_json),
            updated_at_unix_ms: chrono::Utc::now().timestamp_millis(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            event_id,
            event_type,
            event_payload_hash,
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredBindingRow, _>(
                            r#"
                            BEGIN TRANSACTION;
                            LET $record = type::record('worktree_vm_bindings', $record_id);
                            LET $current = (SELECT binding_id, worktree_id, adapter_id,
                                process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                generation, failure_reason, reservation_id, owner_account_id,
                                actor_principal_id, authenticated_session_id, access_space_id,
                                workspace_id, updated_at_unix_ms,
                                record::id(event_ledger_event_id) AS event_ledger_event_id,
                                event_ledger_event_type, event_ledger_payload_hash FROM $record
                                WHERE binding_id = $binding_id
                                  AND generation = $generation
                                  AND binding_state IN ['active', 'snapshotted']
                                  AND owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                  AND event_ledger_event_id.owner_account_id = $owner_account_id
                                  AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                  AND event_ledger_event_id.access_space_id = $access_space_id
                                  AND event_ledger_event_id.workspace_id = $workspace_id
                                  AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                  AND event_ledger_event_id.aggregate_id = $record_id
                                  AND event_ledger_event_id.event_type = event_ledger_event_type
                                  AND event_ledger_event_id.payload_hash = event_ledger_payload_hash
                                  AND event_ledger_event_id.payload.record_id = $record_id
                                  AND event_ledger_event_id.payload.worktree_id = worktree_id
                                  AND event_ledger_event_id.payload.binding_id = binding_id
                                  AND event_ledger_event_id.payload.binding_state = binding_state);
                            IF array::len($current) = 1
                               AND $current[0].event_ledger_event_id = $event_id {
                                RETURN $current;
                            } ELSE IF array::len($current) = 1 {
                                LET $prior = (SELECT event_id FROM kernel_event_ledger
                                    WHERE idempotency_key = $event_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id LIMIT 2);
                                IF array::len($prior) != 0 { THROW 'worktree VM snapshot EventLedger receipt is orphaned or ambiguous'; };
                                LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                                    event_id: $event_id, event_version: 'kernel_event_v1',
                                    kernel_task_run_id: $workspace_id,
                                    session_run_id: $authenticated_session_id,
                                    aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                                    idempotency_key: $event_id, event_type: $event_type,
                                    actor_kind: 'operator', actor_id: $actor_principal_id,
                                    causation_id: $current[0].event_ledger_event_id,
                                    correlation_id: $record_id, payload_hash: $event_payload_hash,
                                    source_component: 'worktree_vm_registry',
                                    payload: { transition_event_type: $event_type,
                                        record_id: $record_id, worktree_id: $current[0].worktree_id,
                                        binding_id: $binding_id,
                                        binding_state: 'snapshotted' },
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id, workspace_id: $workspace_id,
                                    created_at: time::now()
                                };
                                LET $stored = UPDATE $record SET
                                    latest_snapshot_json = $latest_snapshot_json,
                                    binding_state = 'snapshotted',
                                    updated_at_unix_ms = $updated_at_unix_ms,
                                    event_ledger_event_id = type::record('kernel_event_ledger', $event_id),
                                    event_ledger_event_type = $event_type,
                                    event_ledger_payload_hash = $event_payload_hash
                                    WHERE binding_id = $binding_id
                                      AND generation = $generation
                                      AND binding_state IN ['active', 'snapshotted']
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id;
                                LET $verified = (SELECT binding_id, worktree_id, adapter_id,
                                    process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                    generation, failure_reason, reservation_id, owner_account_id,
                                    actor_principal_id, authenticated_session_id, access_space_id,
                                    workspace_id, updated_at_unix_ms,
                                    record::id(event_ledger_event_id) AS event_ledger_event_id,
                                    event_ledger_event_type, event_ledger_payload_hash FROM $record
                                    WHERE binding_id = $binding_id
                                      AND generation = $generation
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id
                                      AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id)
                                      AND event_ledger_event_id.owner_account_id = $owner_account_id
                                      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                      AND event_ledger_event_id.access_space_id = $access_space_id
                                      AND event_ledger_event_id.workspace_id = $workspace_id
                                      AND event_ledger_event_id.event_id = $event_id
                                      AND event_ledger_event_id.idempotency_key = $event_id
                                      AND event_ledger_event_id.event_version = 'kernel_event_v1'
                                      AND event_ledger_event_id.kernel_task_run_id = $workspace_id
                                      AND event_ledger_event_id.session_run_id = $authenticated_session_id
                                      AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                      AND event_ledger_event_id.aggregate_id = $record_id
                                      AND event_ledger_event_id.event_type = $event_type
                                      AND event_ledger_event_id.actor_kind = 'operator'
                                      AND event_ledger_event_id.actor_id = $actor_principal_id
                                      AND event_ledger_event_id.source_component = 'worktree_vm_registry'
                                      AND event_ledger_event_id.payload_hash = $event_payload_hash
                                    LIMIT 2);
                                IF array::len($verified) != 1 { THROW 'worktree VM snapshot receipt verification failed'; };
                                RETURN $verified;
                            } ELSE {
                                RETURN [];
                            };
                            COMMIT TRANSACTION;
                            "#,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| WorktreeVmError::StaleBinding {
                worktree_id: binding.worktree_id.clone(),
                operation: "snapshot",
            })?;
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(&binding.worktree_id, &row)
            .await?;
        Ok(())
    }
    async fn mark_terminated(
        &self,
        binding: &WorktreeVmBindingRecord,
    ) -> Result<(), WorktreeVmError> {
        self.ensure_schema().await?;
        let scope = self.scope_strings()?;
        let record_id = self.record_id(&binding.worktree_id)?;
        let binding_text = binding.binding_id.to_string();
        let latest_snapshot_json = binding
            .latest_snapshot
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let event_type = EVENT_TERMINATED.to_owned();
        let operation_identity = format!("{}:{}", binding.binding_id, binding.generation);
        let event_id = self.event_id(&record_id, &event_type, &operation_identity)?;
        let event_payload_hash = Self::event_payload_hash_for_values(
            &record_id,
            &binding.worktree_id,
            &binding_text,
            "terminated",
            &event_type,
        );
        let bindings = FencedBindingMutationBindings {
            record_id,
            binding_id: binding_text,
            generation: binding.generation,
            latest_snapshot_json,
            updated_at_unix_ms: chrono::Utc::now().timestamp_millis(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            event_id,
            event_type,
            event_payload_hash,
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredBindingRow, _>(
                            r#"
                            BEGIN TRANSACTION;
                            LET $record = type::record('worktree_vm_bindings', $record_id);
                            LET $current = (SELECT binding_id, worktree_id, adapter_id,
                                process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                generation, failure_reason, reservation_id, owner_account_id,
                                actor_principal_id, authenticated_session_id, access_space_id,
                                workspace_id, updated_at_unix_ms,
                                record::id(event_ledger_event_id) AS event_ledger_event_id,
                                event_ledger_event_type, event_ledger_payload_hash FROM $record
                                WHERE binding_id = $binding_id
                                  AND generation = $generation
                                  AND owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                  AND event_ledger_event_id.owner_account_id = $owner_account_id
                                  AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                  AND event_ledger_event_id.access_space_id = $access_space_id
                                  AND event_ledger_event_id.workspace_id = $workspace_id
                                  AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                  AND event_ledger_event_id.aggregate_id = $record_id
                                  AND event_ledger_event_id.event_type = event_ledger_event_type
                                  AND event_ledger_event_id.payload_hash = event_ledger_payload_hash
                                  AND event_ledger_event_id.payload.record_id = $record_id
                                  AND event_ledger_event_id.payload.worktree_id = worktree_id
                                  AND event_ledger_event_id.payload.binding_id = binding_id
                                  AND event_ledger_event_id.payload.binding_state = binding_state);
                            IF array::len($current) = 1
                               AND $current[0].binding_state = 'terminated'
                               AND $current[0].event_ledger_event_id = $event_id {
                                RETURN $current;
                            } ELSE IF array::len($current) = 1
                               AND $current[0].binding_state IN ['active', 'snapshotted'] {
                                LET $prior = (SELECT event_id FROM kernel_event_ledger
                                    WHERE idempotency_key = $event_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id LIMIT 2);
                                IF array::len($prior) != 0 { THROW 'worktree VM teardown EventLedger receipt is orphaned or ambiguous'; };
                                LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                                    event_id: $event_id, event_version: 'kernel_event_v1',
                                    kernel_task_run_id: $workspace_id,
                                    session_run_id: $authenticated_session_id,
                                    aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                                    idempotency_key: $event_id, event_type: $event_type,
                                    actor_kind: 'operator', actor_id: $actor_principal_id,
                                    causation_id: $current[0].event_ledger_event_id,
                                    correlation_id: $record_id, payload_hash: $event_payload_hash,
                                    source_component: 'worktree_vm_registry',
                                    payload: { transition_event_type: $event_type,
                                        record_id: $record_id, worktree_id: $current[0].worktree_id,
                                        binding_id: $binding_id,
                                        binding_state: 'terminated' },
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id, workspace_id: $workspace_id,
                                    created_at: time::now()
                                };
                                LET $stored = UPDATE $record SET
                                    binding_state = 'terminated', reservation_id = NONE,
                                    updated_at_unix_ms = $updated_at_unix_ms,
                                    event_ledger_event_id = type::record('kernel_event_ledger', $event_id),
                                    event_ledger_event_type = $event_type,
                                    event_ledger_payload_hash = $event_payload_hash
                                    WHERE binding_id = $binding_id
                                      AND generation = $generation
                                      AND binding_state IN ['active', 'snapshotted']
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id;
                                LET $verified = (SELECT binding_id, worktree_id, adapter_id,
                                    process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                    generation, failure_reason, reservation_id, owner_account_id,
                                    actor_principal_id, authenticated_session_id, access_space_id,
                                    workspace_id, updated_at_unix_ms,
                                    record::id(event_ledger_event_id) AS event_ledger_event_id,
                                    event_ledger_event_type, event_ledger_payload_hash FROM $record
                                    WHERE binding_id = $binding_id
                                      AND generation = $generation
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id
                                      AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id)
                                      AND event_ledger_event_id.owner_account_id = $owner_account_id
                                      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                      AND event_ledger_event_id.access_space_id = $access_space_id
                                      AND event_ledger_event_id.workspace_id = $workspace_id
                                      AND event_ledger_event_id.event_id = $event_id
                                      AND event_ledger_event_id.idempotency_key = $event_id
                                      AND event_ledger_event_id.event_version = 'kernel_event_v1'
                                      AND event_ledger_event_id.kernel_task_run_id = $workspace_id
                                      AND event_ledger_event_id.session_run_id = $authenticated_session_id
                                      AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                      AND event_ledger_event_id.aggregate_id = $record_id
                                      AND event_ledger_event_id.event_type = $event_type
                                      AND event_ledger_event_id.actor_kind = 'operator'
                                      AND event_ledger_event_id.actor_id = $actor_principal_id
                                      AND event_ledger_event_id.source_component = 'worktree_vm_registry'
                                      AND event_ledger_event_id.payload_hash = $event_payload_hash
                                    LIMIT 2);
                                IF array::len($verified) != 1 { THROW 'worktree VM teardown receipt verification failed'; };
                                RETURN $verified;
                            } ELSE {
                                RETURN [];
                            };
                            COMMIT TRANSACTION;
                            "#,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| WorktreeVmError::StaleBinding {
                worktree_id: binding.worktree_id.clone(),
                operation: "teardown",
            })?;
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(&binding.worktree_id, &row)
            .await?;
        Ok(())
    }
    async fn load(
        &self,
        worktree_id: &str,
        live_only: bool,
    ) -> Result<Option<WorktreeVmBindingRecord>, WorktreeVmError> {
        self.load_physical_key(worktree_id, live_only).await
    }

    async fn select_scoped_row(
        &self,
        worktree_id: &str,
    ) -> Result<Option<StoredBindingRow>, WorktreeVmError> {
        self.ensure_schema().await?;
        let bindings = self.scoped_record_bindings(worktree_id)?;
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredBindingRow, _>(
                            r#"
                            SELECT binding_id, worktree_id, adapter_id,
                                   process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json,
                                   binding_state, generation, failure_reason,
                                   reservation_id, owner_account_id,
                                   actor_principal_id, authenticated_session_id,
                                   access_space_id, workspace_id, updated_at_unix_ms,
                                   record::id(event_ledger_event_id) AS event_ledger_event_id,
                                   event_ledger_event_type, event_ledger_payload_hash
                            FROM type::record('worktree_vm_bindings', $record_id)
                            WHERE owner_account_id = $owner_account_id
                              AND actor_principal_id = $actor_principal_id
                              AND authenticated_session_id = $authenticated_session_id
                              AND access_space_id = $access_space_id
                              AND workspace_id = $workspace_id;
                            "#,
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        let row = rows.into_iter().next();
        if let Some(row) = row.as_ref() {
            self.authorize_row_scope(row)?;
            self.verify_receipt_linkage(worktree_id, row).await?;
            self.verify_snapshot_disposition_row(row).await?;
            self.verify_reservation_intent_row(row).await?;
        }
        Ok(row)
    }

    async fn load_physical_key(
        &self,
        worktree_id: &str,
        live_only: bool,
    ) -> Result<Option<WorktreeVmBindingRecord>, WorktreeVmError> {
        let Some(row) = self.select_scoped_row(worktree_id).await? else {
            return Ok(None);
        };
        self.load_verified_row(worktree_id, live_only, row).await
    }

    async fn load_verified_row(
        &self, worktree_id: &str, live_only: bool, row: StoredBindingRow,
    ) -> Result<Option<WorktreeVmBindingRecord>, WorktreeVmError> {
        if row.binding_state == "reserved" {
            if row.reservation_intent_json.is_some() {
                return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_owned(), adapter_id: row.adapter_id,
                    reason: "intent-backed reservation retains original acquisition; age is not no-effect proof".into(),
                });
            }
            let age_ms = chrono::Utc::now()
                .timestamp_millis()
                .saturating_sub(row.updated_at_unix_ms);
            if age_ms >= 10 * 60 * 1000 {
                let reservation_id = row
                    .reservation_id
                    .as_deref()
                    .ok_or_else(|| WorktreeVmError::InvalidPersistedBinding {
                        reason: "reserved row has no reservation_id".to_owned(),
                    })
                    .and_then(|value| {
                        Uuid::parse_str(value).map_err(|error| {
                            WorktreeVmError::InvalidPersistedBinding {
                                reason: format!("reservation_id is not a UUID: {error}"),
                            }
                        })
                    })?;
                self.release_reservation(worktree_id, reservation_id)
                    .await?;
                return Ok(None);
            }
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_owned(),
                adapter_id: row.adapter_id,
                reason:
                    "embedded SurrealDB binding is reserved by another create/restore operation"
                        .to_owned(),
            });
        }
        let state = WorktreeVmBindingState::parse(&row.binding_state)?;
        if live_only && !state.is_live() {
            return Ok(None);
        }
        if state == WorktreeVmBindingState::Failed && row.process_handle_json.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.row_to_record(row, state)?))
    }
    async fn authorize_snapshot_source(
        &self,
        manifest: &WarmVmSnapshotManifest,
    ) -> Result<(), WorktreeVmError> {
        self.ensure_schema().await?;
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
        let scope = self.scope_strings()?;
        let bindings = SnapshotSourceBindings {
            binding_id: binding_id.to_string(),
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        let rows = self
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values::<StoredBindingRow, _>(
                            r#"
                            SELECT binding_id, worktree_id, adapter_id,
                                   process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json,
                                   binding_state, generation, failure_reason,
                                   reservation_id, owner_account_id,
                                   actor_principal_id, authenticated_session_id,
                                   access_space_id, workspace_id, updated_at_unix_ms,
                                   record::id(event_ledger_event_id) AS event_ledger_event_id,
                                   event_ledger_event_type, event_ledger_payload_hash
                            FROM worktree_vm_bindings
                            WHERE binding_id = $binding_id
                              AND owner_account_id = $owner_account_id
                              AND actor_principal_id = $actor_principal_id
                              AND authenticated_session_id = $authenticated_session_id
                              AND access_space_id = $access_space_id
                              AND workspace_id = $workspace_id
                            LIMIT 1;
                            "#,
                            bindings,
                        )
                        .await
                })
            })
            .await?;
        let Some(row) = rows.into_iter().next() else {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        };
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(&row.worktree_id, &row).await?;
        self.verify_snapshot_disposition_row(&row).await?;
        let durable_scope = self.scope_strings()?;
        let pending_owner = RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default)
            .lock().unwrap_or_else(|error| error.into_inner()).values().any(|owner| {
                owner.intent.binding_id == binding_id
                    && owner.intent.generation == generation
                    && owner.intent.scope == durable_scope
                    && Arc::ptr_eq(&owner.durable.schema_ready, &self.schema_ready)
                    && owner.snapshot.lock().unwrap_or_else(|error| error.into_inner())
                        .as_ref() == Some(&manifest.snapshot)
            });
        if pending_owner {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if let Some(raw) = &row.snapshot_cleanup_json {
            let disposition: SnapshotDispositionRecord = serde_json::from_str(raw)?;
            if disposition.phase != SnapshotDispositionPhase::Delivered
                || disposition.snapshot.as_ref() != Some(&manifest.snapshot) {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
        }
        let stored_snapshot = row
            .latest_snapshot_json
            .as_deref()
            .map(serde_json::from_str::<SnapshotRef>)
            .transpose()?;
        if row.generation != generation || stored_snapshot.as_ref() != Some(&manifest.snapshot) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        Ok(())
    }

    fn authorize_row_scope(&self, row: &StoredBindingRow) -> Result<(), WorktreeVmError> {
        let expected = self.scope_strings()?;
        for (dimension, actual, expected) in [
            (
                "owner_account_id",
                row.owner_account_id.as_str(),
                expected.owner_account_id.as_str(),
            ),
            (
                "actor_principal_id",
                row.actor_principal_id.as_str(),
                expected.actor_principal_id.as_str(),
            ),
            (
                "authenticated_session_id",
                row.authenticated_session_id.as_str(),
                expected.authenticated_session_id.as_str(),
            ),
            (
                "access_space_id",
                row.access_space_id.as_str(),
                expected.access_space_id.as_str(),
            ),
            (
                "workspace_id",
                row.workspace_id.as_str(),
                expected.workspace_id.as_str(),
            ),
        ] {
            if actual != expected {
                return Err(WorktreeVmError::BindingScopeMismatch { dimension });
            }
        }
        Ok(())
    }

    fn row_to_record(
        &self,
        row: StoredBindingRow,
        state: WorktreeVmBindingState,
    ) -> Result<WorktreeVmBindingRecord, WorktreeVmError> {
        let binding_id = Uuid::parse_str(&row.binding_id).map_err(|error| {
            WorktreeVmError::InvalidPersistedBinding {
                reason: format!("binding_id is not a UUID: {error}"),
            }
        })?;
        let process_handle = serde_json::from_str(&row.process_handle_json)?;
        let latest_snapshot = row
            .latest_snapshot_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        Ok(WorktreeVmBindingRecord {
            binding_id,
            worktree_id: row.worktree_id,
            adapter_id: row.adapter_id,
            process_handle,
            latest_snapshot,
            binding_state: state,
            generation: row.generation,
            failure_reason: row.failure_reason,
        })
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

#[derive(Clone, PartialEq, Eq)]
struct SnapshotWarmInput {
    model_artifact_sha256: String,
    model_guest_path: String,
    ready_nonce: String,
}

struct RetainedSnapshotDisposition {
    adapter: Arc<dyn SandboxAdapter>,
    durable: WorktreeVmDurableStore,
    intent: SnapshotCaptureIntent,
    external_acceptance: bool,
    snapshot: std::sync::Mutex<Option<SnapshotRef>>,
    invoked: std::sync::atomic::AtomicBool,
    capture: Mutex<SnapshotCaptureProgress>,
    registry: Option<Arc<crate::capabilities::CapabilityRegistry>>,
    deletion: Mutex<SnapshotDeletion>,
}

#[derive(Default)]
struct SnapshotCaptureProgress {
    original: Option<std::pin::Pin<Box<dyn std::future::Future<Output = Result<SnapshotRef, SandboxAdapterError>> + Send>>>,
    failed: Option<String>,
}

#[derive(Default)]
struct SnapshotDeletion {
    original: Option<std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), SandboxAdapterError>> + Send>>>,
    completed: bool,
    last_error: Option<String>,
}

static RETAINED_SNAPSHOT_DISPOSITIONS:
    OnceLock<std::sync::Mutex<HashMap<Uuid, Arc<RetainedSnapshotDisposition>>>> =
    OnceLock::new();

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SnapshotDispositionPhase { Capturing, Available, Delivered, Quarantined, Deleted, Cancelled }

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotDispositionRecord {
    version: u8,
    intent: SnapshotCaptureIntent,
    snapshot: Option<SnapshotRef>,
    phase: SnapshotDispositionPhase,
    event_id: String,
    event_payload_hash: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotCaptureIntent {
    version: u8,
    capture_id: Uuid,
    request_owner_id: Uuid,
    worktree_id: String,
    binding_id: Uuid,
    generation: i64,
    process_handle_json: String,
    source_vm_event_id: String,
    source_vm_event_payload_hash: String,
    prior_latest_snapshot_json: Option<String>,
    prior_snapshot_disposition_json: Option<String>,
    ready_nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_artifact_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_guest_path: Option<String>,
    scope: DurableScopeStrings,
}

#[derive(SurrealValue)]
struct SnapshotTransitionBindings {
    record_id: String, binding_id: String, generation: i64, process_handle_json: String,
    expected_snapshot_json: Option<String>, expected_disposition_json: Option<String>,
    next_snapshot_json: Option<String>, next_disposition_json: Option<String>,
    phase: String, capture_id: String, request_owner_id: String, intent_sha256: String,
    snapshot_sha256: Option<String>, event_id: String, event_payload_hash: String,
    source_vm_event_id: String, source_vm_event_payload_hash: String, event_type: String,
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
    access_space_id: String, workspace_id: String,
}

impl SnapshotDispositionPhase {
    fn as_str(self) -> &'static str {
        match self { Self::Capturing => "capturing", Self::Available => "available", Self::Delivered => "delivered",
            Self::Quarantined => "quarantined", Self::Deleted => "deleted", Self::Cancelled => "cancelled" }
    }
}

impl WorktreeVmDurableStore {
    fn snapshot_disposition_record(&self, intent: &SnapshotCaptureIntent, phase: SnapshotDispositionPhase,
        snapshot: Option<SnapshotRef>) -> Result<SnapshotDispositionRecord, WorktreeVmError> {
        if intent.version != 1 || intent.scope != self.scope_strings()? || intent.capture_id.is_nil()
            || intent.request_owner_id.is_nil() || intent.generation <= 0 {
            return Err(WorktreeVmError::ScopeDenied(ScopeDenied::ExactAttributionMismatch));
        }
        let record_id = self.record_id(&intent.worktree_id)?;
        let intent_sha256 = sha256_hex(serde_json::to_string(intent)?.as_bytes());
        let snapshot_sha256 = snapshot.as_ref().map(serde_json::to_string).transpose()?
            .map(|json| sha256_hex(json.as_bytes()));
        let event_type = if phase == SnapshotDispositionPhase::Capturing { "ARTIFACT_PROPOSED" } else { EVENT_SNAPSHOTTED };
        let event_id = self.event_id(&record_id, event_type,
            &format!("snapshot:{}:{}", intent.capture_id, phase.as_str()))?;
        let payload = serde_json::json!({
            "record_id": record_id, "worktree_id": intent.worktree_id,
            "binding_id": intent.binding_id.to_string(), "generation": intent.generation,
            "capture_id": intent.capture_id.to_string(), "request_owner_id": intent.request_owner_id.to_string(),
            "snapshot_disposition": phase.as_str(), "snapshot_sha256": snapshot_sha256,
            "intent_sha256": intent_sha256, "transition_event_type": event_type,
        });
        Ok(SnapshotDispositionRecord { version: 1, intent: intent.clone(), snapshot, phase,
            event_id, event_payload_hash: sha256_hex(&canonical_json_bytes(&payload)) })
    }

    async fn transition_snapshot_disposition(
        &self, next: &SnapshotDispositionRecord, expected_snapshot_json: Option<String>,
        expected_disposition_json: Option<String>, next_snapshot_json: Option<String>,
        next_disposition_json: Option<String>,
    ) -> Result<StoredBindingRow, WorktreeVmError> {
        let intent = &next.intent;
        let canonical = self.snapshot_disposition_record(intent, next.phase, next.snapshot.clone())?;
        if canonical.event_id != next.event_id || canonical.event_payload_hash != next.event_payload_hash
            || next.version != 1 { return Err(WorktreeVmError::SnapshotSourceMismatch); }
        let scope = self.scope_strings()?;
        let record_id = self.record_id(&intent.worktree_id)?;
        let event_type = if next.phase == SnapshotDispositionPhase::Capturing { "ARTIFACT_PROPOSED" } else { EVENT_SNAPSHOTTED };
        let expected_next_snapshot = next_snapshot_json.clone();
        let expected_next_disposition = next_disposition_json.clone();
        let bindings = SnapshotTransitionBindings {
            record_id, binding_id: intent.binding_id.to_string(), generation: intent.generation,
            process_handle_json: intent.process_handle_json.clone(),
            expected_snapshot_json, expected_disposition_json, next_snapshot_json, next_disposition_json,
            phase: next.phase.as_str().into(), capture_id: intent.capture_id.to_string(),
            request_owner_id: intent.request_owner_id.to_string(),
            intent_sha256: sha256_hex(serde_json::to_string(intent)?.as_bytes()),
            snapshot_sha256: next.snapshot.as_ref().map(serde_json::to_string).transpose()?.map(|json| sha256_hex(json.as_bytes())),
            event_id: next.event_id.clone(), event_payload_hash: next.event_payload_hash.clone(),
            source_vm_event_id: intent.source_vm_event_id.clone(),
            source_vm_event_payload_hash: intent.source_vm_event_payload_hash.clone(), event_type: event_type.into(),
            owner_account_id: scope.owner_account_id, actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id, access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        let rows_result = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values_at::<StoredBindingRow, _>(r#"
                BEGIN TRANSACTION;
                LET $record = type::record('worktree_vm_bindings', $record_id);
                LET $current = (SELECT binding_id, worktree_id, adapter_id, process_handle_json,
                    latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                    generation, failure_reason, reservation_id, owner_account_id, actor_principal_id,
                    authenticated_session_id, access_space_id, workspace_id, updated_at_unix_ms,
                    record::id(event_ledger_event_id) AS event_ledger_event_id,
                    event_ledger_event_type, event_ledger_payload_hash FROM $record
                    WHERE binding_id = $binding_id AND generation = $generation
                      AND process_handle_json = $process_handle_json
                      AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                      AND authenticated_session_id = $authenticated_session_id
                      AND access_space_id = $access_space_id AND workspace_id = $workspace_id
                      AND event_ledger_event_id.owner_account_id = $owner_account_id
                      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                      AND event_ledger_event_id.access_space_id = $access_space_id
                      AND event_ledger_event_id.workspace_id = $workspace_id
                      AND event_ledger_event_id.payload_hash = event_ledger_payload_hash
                      AND event_ledger_event_id.payload.binding_state = binding_state);
                LET $receipt = (SELECT event_id FROM kernel_event_ledger
                    WHERE id = type::record('kernel_event_ledger', $event_id)
                      AND event_id = $event_id AND idempotency_key = $event_id
                      AND event_version = 'kernel_event_v1' AND payload_hash = $event_payload_hash
                      AND source_component = 'worktree_vm_registry'
                      AND aggregate_type = 'worktree_vm_binding' AND aggregate_id = $record_id
                      AND event_type = $event_type AND payload.capture_id = $capture_id
                      AND payload.request_owner_id = $request_owner_id AND payload.intent_sha256 = $intent_sha256
                      AND payload.snapshot_disposition = $phase AND payload.snapshot_sha256 = IF $snapshot_sha256 = NONE { NULL } ELSE { $snapshot_sha256 }
                      AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                      AND authenticated_session_id = $authenticated_session_id
                      AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2);
                IF array::len($current) = 1 AND array::len($receipt) = 1
                    AND $current[0].latest_snapshot_json = $next_snapshot_json
                    AND $current[0].snapshot_cleanup_json = $next_disposition_json
                    AND ($phase != 'delivered' OR $current[0].binding_state IN ['active', 'snapshotted']) {
                    RETURN $current;
                } ELSE IF array::len($current) = 1 AND array::len($receipt) = 0
                    AND $current[0].latest_snapshot_json = $expected_snapshot_json
                    AND $current[0].snapshot_cleanup_json = $expected_disposition_json
                    AND ($phase != 'delivered' OR $current[0].binding_state IN ['active', 'snapshotted'])
                    AND ($phase != 'capturing' OR (
                        $current[0].binding_state IN ['active', 'snapshotted']
                        AND $current[0].event_ledger_event_id = $source_vm_event_id
                        AND $current[0].event_ledger_payload_hash = $source_vm_event_payload_hash)) {
                    LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                        event_id: $event_id, event_version: 'kernel_event_v1',
                        kernel_task_run_id: $workspace_id, session_run_id: $authenticated_session_id,
                        aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                        idempotency_key: $event_id, event_type: $event_type,
                        actor_kind: 'operator', actor_id: $actor_principal_id,
                        causation_id: $source_vm_event_id, correlation_id: $capture_id,
                        source_component: 'worktree_vm_registry', payload_hash: $event_payload_hash,
                        payload: { record_id: $record_id, worktree_id: $current[0].worktree_id,
                            binding_id: $binding_id, generation: $generation, capture_id: $capture_id,
                            request_owner_id: $request_owner_id, intent_sha256: $intent_sha256,
                            snapshot_disposition: $phase, snapshot_sha256: IF $snapshot_sha256 = NONE { NULL } ELSE { $snapshot_sha256 },
                            transition_event_type: $event_type },
                        owner_account_id: $owner_account_id, actor_principal_id: $actor_principal_id,
                        authenticated_session_id: $authenticated_session_id,
                        access_space_id: $access_space_id, workspace_id: $workspace_id, created_at: time::now()
                    };
                    LET $stored = UPDATE $record SET latest_snapshot_json = $next_snapshot_json,
                        snapshot_cleanup_json = $next_disposition_json
                        WHERE binding_id = $binding_id AND generation = $generation
                          AND process_handle_json = $process_handle_json
                          AND latest_snapshot_json = $expected_snapshot_json
                          AND snapshot_cleanup_json = $expected_disposition_json
                          AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                          AND authenticated_session_id = $authenticated_session_id
                          AND access_space_id = $access_space_id AND workspace_id = $workspace_id;
                    LET $verified = (SELECT binding_id, worktree_id, adapter_id, process_handle_json,
                        latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                        generation, failure_reason, reservation_id, owner_account_id, actor_principal_id,
                        authenticated_session_id, access_space_id, workspace_id, updated_at_unix_ms,
                        record::id(event_ledger_event_id) AS event_ledger_event_id,
                        event_ledger_event_type, event_ledger_payload_hash FROM $record
                        WHERE binding_id = $binding_id AND generation = $generation
                          AND process_handle_json = $process_handle_json
                          AND latest_snapshot_json = $next_snapshot_json
                          AND snapshot_cleanup_json = $next_disposition_json
                          AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                          AND authenticated_session_id = $authenticated_session_id
                          AND access_space_id = $access_space_id AND workspace_id = $workspace_id);
                    IF array::len($verified) != 1 { THROW 'exact snapshot disposition CAS failed'; };
                    RETURN $verified;
                } ELSE { RETURN []; };
                COMMIT TRANSACTION;
            "#, bindings, 4).await
        })).await;
        let rows = match rows_result {
            Ok(rows) => rows,
            Err(primary) => {
                let reconciled = async {
                    let row = self.select_scoped_row(&intent.worktree_id).await?
                        .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
                    if row.binding_id != intent.binding_id.to_string()
                        || row.generation != intent.generation
                        || row.process_handle_json != intent.process_handle_json
                        || row.latest_snapshot_json != expected_next_snapshot
                        || row.snapshot_cleanup_json != expected_next_disposition {
                        return Err(WorktreeVmError::SnapshotSourceMismatch);
                    }
                    self.verify_snapshot_disposition_receipt(next).await?;
                    Ok::<_, WorktreeVmError>(row)
                }.await;
                return match reconciled {
                    Ok(row) => Ok(row),
                    Err(_) => Err(primary.into()),
                };
            }
        };
        let row = rows.into_iter().next().ok_or_else(|| WorktreeVmError::StaleBinding {
            worktree_id: intent.worktree_id.clone(), operation: "snapshot disposition exact CAS",
        })?;
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(&intent.worktree_id, &row).await?;
        self.verify_snapshot_disposition_receipt(next).await?;
        self.verify_snapshot_disposition_row(&row).await?;
        Ok(row)
    }
}

#[derive(SurrealValue)]
struct SnapshotDispositionReceiptBindings {
    event_id: String, event_type: String, event_payload_hash: String, record_id: String,
    capture_id: String, request_owner_id: String, intent_sha256: String, snapshot_sha256: Option<String>,
    generation: i64, binding_id: String, phase: String,
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
    access_space_id: String, workspace_id: String,
}

impl WorktreeVmDurableStore {
    async fn verify_snapshot_disposition_receipt(&self, record: &SnapshotDispositionRecord) -> Result<(), WorktreeVmError> {
        let canonical = self.snapshot_disposition_record(&record.intent, record.phase, record.snapshot.clone())?;
        if record.version != 1 || canonical.event_id != record.event_id
            || canonical.event_payload_hash != record.event_payload_hash { return Err(WorktreeVmError::SnapshotSourceMismatch); }
        let scope = self.scope_strings()?;
        let bindings = SnapshotDispositionReceiptBindings {
            event_id: record.event_id.clone(), event_type: if record.phase == SnapshotDispositionPhase::Capturing { "ARTIFACT_PROPOSED" } else { EVENT_SNAPSHOTTED }.into(),
            event_payload_hash: record.event_payload_hash.clone(), record_id: self.record_id(&record.intent.worktree_id)?,
            capture_id: record.intent.capture_id.to_string(), request_owner_id: record.intent.request_owner_id.to_string(),
            intent_sha256: sha256_hex(serde_json::to_string(&record.intent)?.as_bytes()),
            snapshot_sha256: record.snapshot.as_ref().map(serde_json::to_string).transpose()?.map(|json| sha256_hex(json.as_bytes())),
            generation: record.intent.generation, binding_id: record.intent.binding_id.to_string(), phase: record.phase.as_str().into(),
            owner_account_id: scope.owner_account_id, actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id, access_space_id: scope.access_space_id, workspace_id: scope.workspace_id,
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<StoredReceiptRow, _>(r#"
                SELECT event_id, event_sequence FROM kernel_event_ledger
                WHERE id = type::record('kernel_event_ledger', $event_id)
                  AND event_id = $event_id AND idempotency_key = $event_id AND event_version = 'kernel_event_v1'
                  AND event_type = $event_type AND payload_hash = $event_payload_hash AND event_sequence > 0
                  AND aggregate_type = 'worktree_vm_binding' AND aggregate_id = $record_id
                  AND source_component = 'worktree_vm_registry' AND actor_kind = 'operator' AND actor_id = $actor_principal_id
                  AND payload.capture_id = $capture_id AND payload.request_owner_id = $request_owner_id
                  AND payload.intent_sha256 = $intent_sha256 AND payload.snapshot_sha256 = IF $snapshot_sha256 = NONE { NULL } ELSE { $snapshot_sha256 }
                  AND payload.snapshot_disposition = $phase AND payload.binding_id = $binding_id AND payload.generation = $generation
                  AND kernel_task_run_id = $workspace_id AND session_run_id = $authenticated_session_id
                  AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                  AND authenticated_session_id = $authenticated_session_id
                  AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;
            "#, bindings).await
        })).await?;
        if rows.len() != 1 { return Err(WorktreeVmError::EventLedgerReceiptMissing); }
        Ok(())
    }

    async fn verify_snapshot_disposition_row(&self, row: &StoredBindingRow) -> Result<(), WorktreeVmError> {
        let Some(raw) = &row.snapshot_cleanup_json else { return Ok(()); };
        let record: SnapshotDispositionRecord = serde_json::from_str(raw)?;
        if record.intent.worktree_id != row.worktree_id || record.intent.binding_id.to_string() != row.binding_id
            || record.intent.generation != row.generation || record.intent.process_handle_json != row.process_handle_json {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let expected = match record.phase {
            SnapshotDispositionPhase::Capturing => record.intent.prior_latest_snapshot_json.clone(),
            SnapshotDispositionPhase::Available | SnapshotDispositionPhase::Delivered => record.snapshot.as_ref().map(serde_json::to_string).transpose()?,
            SnapshotDispositionPhase::Quarantined => {
                let captured = record.snapshot.as_ref().map(serde_json::to_string).transpose()?;
                if row.latest_snapshot_json != captured && row.latest_snapshot_json != record.intent.prior_latest_snapshot_json {
                    return Err(WorktreeVmError::SnapshotSourceMismatch);
                }
                row.latest_snapshot_json.clone()
            }
            SnapshotDispositionPhase::Deleted => record.intent.prior_latest_snapshot_json.clone(),
            SnapshotDispositionPhase::Cancelled => record.intent.prior_latest_snapshot_json.clone(),
        };
        if row.latest_snapshot_json != expected { return Err(WorktreeVmError::SnapshotSourceMismatch); }
        self.verify_snapshot_disposition_receipt(&record).await
    }
}


impl WorktreeVmRegistry {
    async fn snapshot_fenced(
        &self,
        worktree_id: &str,
        warm: Option<SnapshotWarmInput>,
        selected: Option<Arc<WarmSnapshotRequest>>,
    ) -> Result<(SnapshotRef, Option<WarmVmSnapshotManifest>, Option<Arc<RetainedSnapshotDisposition>>), WorktreeVmError> {
        if let Some(durable) = &self.durable {
            if let Some(row) = durable.select_scoped_row(worktree_id).await? {
                if let Some(raw) = &row.snapshot_cleanup_json {
                    let previous: SnapshotDispositionRecord = serde_json::from_str(raw)?;
                    let retained = RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                        .unwrap_or_else(|error| error.into_inner()).contains_key(&previous.intent.capture_id);
                    let externally_owned = RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                        .unwrap_or_else(|error| error.into_inner()).get(&previous.intent.capture_id)
                        .is_some_and(|owner| owner.external_acceptance);
                    if externally_owned || (selected.is_some() && previous.phase != SnapshotDispositionPhase::Delivered) {
                        return Err(WorktreeVmError::DurableHandleUnavailable {
                            worktree_id: worktree_id.to_string(), adapter_id: row.adapter_id,
                            reason: "selected snapshot request retains its own disposition".into(),
                        });
                    }
                    if previous.phase != SnapshotDispositionPhase::Delivered || retained {
                        self.retry_snapshot_cleanup(worktree_id, previous.intent.capture_id).await?;
                        return Err(WorktreeVmError::DurableHandleUnavailable {
                            worktree_id: worktree_id.to_string(), adapter_id: row.adapter_id,
                            reason: "original snapshot disposition reconciled; request a new capture explicitly".into(),
                        });
                    }
                }
            }
        }
        let mut map = self.persistent.lock().await;
        let Some(durable) = &self.durable else {
            self.reject_pending_transient_capture(worktree_id)?;
            let handle = map.get(worktree_id).cloned().ok_or_else(|| WorktreeVmError::NotBound {
                worktree_id: worktree_id.to_string(),
            })?;
            if let Some(request) = &selected {
                if request.worktree_id != worktree_id
                    || transient_binding_identity(&handle) != request.expected {
                    return Err(WorktreeVmError::StaleBinding {
                        worktree_id: worktree_id.to_string(), operation: "selected transient snapshot",
                    });
                }
            }
            let capture = self.begin_transient_capture(
                worktree_id, &handle, selected.as_ref().map(|request| request.owner_id),
            )?;
            if let Some(request) = &selected {
                *request.capture_id.lock().unwrap_or_else(|error| error.into_inner()) = Some(capture.operation_id);
                request.invoked.store(true, std::sync::atomic::Ordering::Release);
            }
            let snapshot = self.observe_transient_capture(&capture).await?;
            if let Some(request) = &selected {
                *request.snapshot.lock().unwrap_or_else(|error| error.into_inner()) = Some(snapshot.clone());
            }
            let manifest = warm.map(|warm| WarmVmSnapshotManifest::new(
                worktree_id, &warm.model_artifact_sha256, &warm.model_guest_path,
                &warm.ready_nonce, snapshot.clone(),
            ));
            if selected.is_none() {
                // Final public handoff is synchronous while the exact binding
                // map remains held. Selected factory captures await acceptance.
                self.finish_transient_capture(&capture, &snapshot)?;
            }
            return Ok((snapshot, manifest, None));
        };
        let _serial = durable.serializer(worktree_id)?.lock_owned().await;
        let binding = durable.load(worktree_id, true).await?
            .ok_or_else(|| WorktreeVmError::NotBound { worktree_id: worktree_id.to_string() })?;
        let row = durable.select_scoped_row(worktree_id).await?
            .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
        if row.binding_id != binding.binding_id.to_string()
            || row.generation != binding.generation
            || row.process_handle_json != serde_json::to_string(&binding.process_handle)? {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if let Some(request) = &selected {
            if request.worktree_id != worktree_id
                || request.expected.binding_id != binding.binding_id
                || request.expected.generation != binding.generation
                || request.expected.process_handle != binding.process_handle
                || map.get(worktree_id).is_some_and(|local| local != &binding.process_handle)
                || self.pending.lock().await.get(worktree_id)
                    .is_some_and(|pending| pending != &binding.process_handle) {
                return Err(WorktreeVmError::StaleBinding {
                    worktree_id: worktree_id.to_string(), operation: "selected durable snapshot",
                });
            }
        }
        if let Some(raw) = &row.snapshot_cleanup_json {
            let previous: SnapshotDispositionRecord = serde_json::from_str(raw)?;
            if previous.phase != SnapshotDispositionPhase::Delivered {
                return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_string(),
                    adapter_id: binding.adapter_id.clone(),
                    reason: "original snapshot disposition is pending or quarantined".to_string(),
                });
            }
        }
        let intent = SnapshotCaptureIntent {
            version: 1, capture_id: Uuid::now_v7(), request_owner_id: selected.as_ref().map_or_else(Uuid::now_v7, |request| request.owner_id),
            worktree_id: worktree_id.to_string(), binding_id: binding.binding_id,
            generation: binding.generation, process_handle_json: row.process_handle_json.clone(),
            source_vm_event_id: row.event_ledger_event_id.clone(),
            source_vm_event_payload_hash: row.event_ledger_payload_hash.clone(),
            prior_latest_snapshot_json: row.latest_snapshot_json.clone(),
            prior_snapshot_disposition_json: row.snapshot_cleanup_json.clone(),
            ready_nonce: warm.as_ref().map(|warm| warm.ready_nonce.clone()),
            model_artifact_sha256: warm.as_ref().map(|warm| warm.model_artifact_sha256.clone()),
            model_guest_path: warm.as_ref().map(|warm| warm.model_guest_path.clone()),
            scope: durable.scope_strings()?,
        };
        let owner = Arc::new(RetainedSnapshotDisposition {
            adapter: Arc::clone(&self.adapter), durable: durable.clone(), intent: intent.clone(),
            external_acceptance: selected.is_some(),
            snapshot: std::sync::Mutex::new(None), invoked: std::sync::atomic::AtomicBool::new(false), deletion: Mutex::new(SnapshotDeletion::default()),
            capture: Mutex::new(SnapshotCaptureProgress::default()),
            registry: crate::storage::surreal::configured_capability_registry(),
        });
        RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).insert(intent.capture_id, Arc::clone(&owner));
        if let Some(request) = &selected {
            *request.capture_id.lock().unwrap_or_else(|error| error.into_inner()) = Some(intent.capture_id);
        }
        let capturing = durable.snapshot_disposition_record(&intent, SnapshotDispositionPhase::Capturing, None)?;
        durable.transition_snapshot_disposition(
            &capturing, intent.prior_latest_snapshot_json.clone(),
            intent.prior_snapshot_disposition_json.clone(),
            intent.prior_latest_snapshot_json.clone(), Some(serde_json::to_string(&capturing)?),
        ).await?;
        if let Some(request) = &selected {
            request.invoked.store(true, std::sync::atomic::Ordering::Release);
        }
        let snapshot = self.observe_original_snapshot_capture(&owner).await?;
        if let Some(request) = &selected {
            *request.snapshot.lock().unwrap_or_else(|error| error.into_inner()) = Some(snapshot.clone());
        }
        reject_reused_snapshot_target(intent.prior_latest_snapshot_json.as_deref(), &snapshot)?;
        let available = durable.snapshot_disposition_record(
            &intent, SnapshotDispositionPhase::Available, Some(snapshot.clone()),
        )?;
        if let Err(primary) = durable.transition_snapshot_disposition(
            &available, intent.prior_latest_snapshot_json.clone(),
            Some(serde_json::to_string(&capturing)?),
            Some(serde_json::to_string(&snapshot)?), Some(serde_json::to_string(&available)?),
        ).await {
            return Err(match self.cleanup_snapshot_disposition_locked(&owner).await {
                Ok(()) => primary,
                Err(cleanup) => WorktreeVmError::CompensationFailed {
                    worktree_id: worktree_id.to_string(), operation: "snapshot persistence",
                    primary: primary.to_string(), cleanup: cleanup.to_string(),
                },
            });
        }
        // Provenance comes from this still-held original binding and scope.
        let manifest = warm.map(|warm| WarmVmSnapshotManifest::new(
            worktree_id, &warm.model_artifact_sha256, &warm.model_guest_path,
            &warm.ready_nonce, snapshot.clone(),
        ).with_durable_source(binding.binding_id, binding.generation,
            snapshot_resource_scope(durable.require_scope().expect("validated exact snapshot scope"))));
        if selected.is_some() {
            map.insert(worktree_id.to_string(), binding.process_handle);
            return Ok((snapshot, manifest, Some(owner)));
        }
        let delivered = durable.snapshot_disposition_record(
            &intent, SnapshotDispositionPhase::Delivered, Some(snapshot.clone()),
        )?;
        durable.transition_snapshot_disposition(
            &delivered, Some(serde_json::to_string(&snapshot)?),
            Some(serde_json::to_string(&available)?),
            Some(serde_json::to_string(&snapshot)?), Some(serde_json::to_string(&delivered)?),
        ).await?;
        map.insert(worktree_id.to_string(), binding.process_handle);
        RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).remove(&intent.capture_id);
        Ok((snapshot, manifest, None))
    }

    async fn observe_original_snapshot_capture(
        &self,
        owner: &RetainedSnapshotDisposition,
    ) -> Result<SnapshotRef, WorktreeVmError> {
        let mut capture = owner.capture.lock().await;
        if let Some(snapshot) = owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).clone() {
            return Ok(snapshot);
        }
        if let Some(reason) = &capture.failed {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: owner.intent.worktree_id.clone(),
                adapter_id: owner.adapter.capabilities().adapter_id.to_string(),
                reason: format!("original snapshot capture failed with unknown effect; custody retained: {reason}"),
            });
        }
        if capture.original.is_none() {
            let adapter = Arc::clone(&owner.adapter);
            let handle: ProcessHandle = serde_json::from_str(&owner.intent.process_handle_json)?;
            let registry = owner.registry.clone();
            capture.original = Some(Box::pin(async move {
                crate::storage::surreal::with_captured_capability_registry(
                    registry, adapter.snapshot(&handle),
                ).await
            }));
        }
        owner.invoked.store(true, std::sync::atomic::Ordering::Release);
        let observed = capture.original.as_mut().expect("retained original snapshot capture").as_mut().await;
        capture.original.take();
        match observed {
            Ok(snapshot) => {
                *owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()) = Some(snapshot.clone());
                Ok(snapshot)
            }
            Err(error) => {
                capture.failed = Some(error.to_string());
                Err(error.into())
            }
        }
    }

    async fn cleanup_snapshot_disposition_locked(
        &self,
        owner: &RetainedSnapshotDisposition,
    ) -> Result<(), WorktreeVmError> {
        let durable = &owner.durable;
        let intent = &owner.intent;
        let observed = owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).clone();
        if observed.is_none() && !owner.invoked.load(std::sync::atomic::Ordering::Acquire) {
            let row = durable.select_scoped_row(&intent.worktree_id).await?
                .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
            if row.binding_id != intent.binding_id.to_string()
                || row.generation != intent.generation
                || row.process_handle_json != intent.process_handle_json
                || row.latest_snapshot_json != intent.prior_latest_snapshot_json {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            if row.snapshot_cleanup_json == intent.prior_snapshot_disposition_json
                && row.event_ledger_event_id == intent.source_vm_event_id
                && row.event_ledger_payload_hash == intent.source_vm_event_payload_hash {
                return Ok(());
            }
            let capturing = durable.snapshot_disposition_record(intent, SnapshotDispositionPhase::Capturing, None)?;
            let cancelled = durable.snapshot_disposition_record(intent, SnapshotDispositionPhase::Cancelled, None)?;
            durable.transition_snapshot_disposition(
                &cancelled, intent.prior_latest_snapshot_json.clone(),
                Some(serde_json::to_string(&capturing)?),
                intent.prior_latest_snapshot_json.clone(), intent.prior_snapshot_disposition_json.clone(),
            ).await?;
            return Ok(());
        }
        let snapshot = owner.snapshot.lock().unwrap_or_else(|error| error.into_inner())
            .clone().ok_or_else(|| WorktreeVmError::DurableHandleUnavailable {
                worktree_id: intent.worktree_id.clone(),
                adapter_id: owner.adapter.capabilities().adapter_id.to_string(),
                reason: "snapshot effect has no confirmed original reference; custody retained".into(),
            })?;
        reject_reused_snapshot_target(intent.prior_latest_snapshot_json.as_deref(), &snapshot)?;
        let deleted = durable.snapshot_disposition_record(
            intent, SnapshotDispositionPhase::Deleted, Some(snapshot.clone()),
        )?;
        if durable.verify_snapshot_disposition_receipt(&deleted).await.is_ok() {
            let row = durable.select_scoped_row(&intent.worktree_id).await?
                .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
            if row.binding_id == intent.binding_id.to_string()
                && row.generation == intent.generation
                && row.process_handle_json == intent.process_handle_json
                && row.latest_snapshot_json == intent.prior_latest_snapshot_json
                && row.snapshot_cleanup_json == intent.prior_snapshot_disposition_json {
                return Ok(());
            }
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let row = durable.select_scoped_row(&intent.worktree_id).await?
            .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
        if row.binding_id != intent.binding_id.to_string()
            || row.generation != intent.generation
            || row.process_handle_json != intent.process_handle_json {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let current: SnapshotDispositionRecord = serde_json::from_str(
            row.snapshot_cleanup_json.as_deref().ok_or(WorktreeVmError::SnapshotSourceMissing)?
        )?;
        if current.intent.capture_id != intent.capture_id
            || current.intent.request_owner_id != intent.request_owner_id
            || current.intent.scope != intent.scope
            || current.phase == SnapshotDispositionPhase::Delivered {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let quarantined = durable.snapshot_disposition_record(
            intent, SnapshotDispositionPhase::Quarantined, Some(snapshot.clone()),
        )?;
        // Exact CAS detaches this reference before any physical delete.
        durable.transition_snapshot_disposition(
            &quarantined, row.latest_snapshot_json.clone(), row.snapshot_cleanup_json.clone(),
            intent.prior_latest_snapshot_json.clone(), Some(serde_json::to_string(&quarantined)?),
        ).await?;
        let mut deletion = owner.deletion.lock().await;
        if !deletion.completed {
            if deletion.original.is_none() {
                let adapter = Arc::clone(&owner.adapter);
                let original_snapshot = snapshot.clone();
                let registry = crate::storage::surreal::configured_capability_registry();
                deletion.original = Some(Box::pin(async move {
                    crate::storage::surreal::with_captured_capability_registry(
                        registry, adapter.delete_snapshot(&original_snapshot),
                    ).await
                }));
            }
            let result = deletion.original.as_mut().expect("retained original delete").as_mut().await;
            deletion.original.take();
            match result {
                Ok(()) => deletion.completed = true,
                Err(error) => {
                    deletion.last_error = Some(error.to_string());
                    return Err(error.into());
                }
            }
        }
        drop(deletion);
        durable.transition_snapshot_disposition(
            &deleted, intent.prior_latest_snapshot_json.clone(),
            Some(serde_json::to_string(&quarantined)?),
            intent.prior_latest_snapshot_json.clone(), intent.prior_snapshot_disposition_json.clone(),
        ).await?;
        Ok(())
    }

    pub(crate) async fn retry_snapshot_cleanup(
        &self, worktree_id: &str, capture_id: Uuid,
    ) -> Result<(), WorktreeVmError> {
        let owner = RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).get(&capture_id).cloned()
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if owner.external_acceptance {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let durable = self.durable.as_ref().ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if owner.intent.worktree_id != worktree_id
            || owner.intent.scope != durable.scope_strings()?
            || !Arc::ptr_eq(&owner.adapter, &self.adapter)
            || !Arc::ptr_eq(&owner.durable.schema_ready, &durable.schema_ready)
            || owner.durable.serialization_key(worktree_id)? != durable.serialization_key(worktree_id)? {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let _serial = durable.serializer(worktree_id)?.lock_owned().await;
        let row = durable.select_scoped_row(worktree_id).await?
            .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
        if row.binding_id != owner.intent.binding_id.to_string()
            || row.generation != owner.intent.generation
            || row.process_handle_json != owner.intent.process_handle_json {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if let Some(raw) = &row.snapshot_cleanup_json {
            let current: SnapshotDispositionRecord = serde_json::from_str(raw)?;
            if current.phase == SnapshotDispositionPhase::Delivered {
                if current.intent.capture_id != capture_id
                    || current.intent.request_owner_id != owner.intent.request_owner_id
                    || current.snapshot != *owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()) {
                    return Err(WorktreeVmError::SnapshotSourceMismatch);
                }
                durable.verify_snapshot_disposition_receipt(&current).await?;
                RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                    .unwrap_or_else(|error| error.into_inner()).remove(&capture_id);
                return Ok(());
            }
        }
        if owner.invoked.load(std::sync::atomic::Ordering::Acquire)
            && owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).is_none() {
            self.observe_original_snapshot_capture(&owner).await?;
        }
        self.cleanup_snapshot_disposition_locked(&owner).await?;
        RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).remove(&capture_id);
        Ok(())
    }
}



#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalVmReservationIntent {
    version: u8,
    operation_id: Uuid,
    reservation_id: Uuid,
    record_id: String,
    worktree_id: String,
    expected_generation: i64,
    source_sha256: String,
    scope: DurableScopeStrings,
    prior_generation: Option<i64>,
    prior_binding_id: Option<String>,
    prior_event_id: Option<String>,
    prior_snapshot_json: Option<String>,
    prior_snapshot_cleanup_json: Option<String>,
}

enum ReservationAcquisitionRequest {
    Spawn(ProcessSpec),
    Restore(SnapshotRef),
}

#[derive(Default)]
struct ReservationAcquisitionProgress {
    adapter: Option<Arc<dyn SandboxAdapter>>,
    original: Option<std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProcessHandle, SandboxAdapterError>> + Send>>>,
    failed: Option<String>,
}

struct RetainedVmReservation {
    durable: WorktreeVmDurableStore,
    intent: OriginalVmReservationIntent,
    invoked: std::sync::atomic::AtomicBool,
    handle: std::sync::Mutex<Option<ProcessHandle>>,
    acquisition: Mutex<ReservationAcquisitionProgress>,
    registry: Option<Arc<crate::capabilities::CapabilityRegistry>>,
    warm_owner: Option<std::sync::Weak<WarmReservationOwner>>,
    release_claimed: std::sync::atomic::AtomicBool,
}

static RETAINED_VM_RESERVATIONS:
    OnceLock<std::sync::Mutex<HashMap<Uuid, Arc<RetainedVmReservation>>>> = OnceLock::new();

#[derive(SurrealValue)]
struct ReservationIntentReceiptBindings {
    event_id: String,
    event_type: String,
    event_payload_hash: String,
    record_id: String,
    worktree_id: String,
    reservation_id: String,
    operation_id: String,
    intent_sha256: String,
    expected_generation: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

impl WorktreeVmDurableStore {
    fn reservation_intent_payload_hash(intent: &OriginalVmReservationIntent) -> Result<String, WorktreeVmError> {
        let payload = serde_json::json!({
            "binding_id": intent.reservation_id.to_string(),
            "binding_state": "reserved",
            "record_id": intent.record_id,
            "transition_event_type": EVENT_RESERVED,
            "worktree_id": intent.worktree_id,
            "operation_id": intent.operation_id.to_string(),
            "reservation_intent_sha256": sha256_hex(serde_json::to_string(intent)?.as_bytes()),
            "expected_generation": intent.expected_generation,
        });
        Ok(sha256_hex(&canonical_json_bytes(&payload)))
    }

    async fn verify_reservation_intent_row(&self, row: &StoredBindingRow) -> Result<(), WorktreeVmError> {
        let Some(raw) = &row.reservation_intent_json else { return Ok(()); };
        let intent: OriginalVmReservationIntent = serde_json::from_str(raw)?;
        let scope = self.scope_strings()?;
        let record_id = self.record_id(&row.worktree_id)?;
        if intent.version != 1 || intent.operation_id.is_nil() || intent.reservation_id.is_nil()
            || intent.scope != scope || intent.record_id != record_id
            || intent.worktree_id != row.worktree_id || intent.expected_generation != row.generation
            || (row.binding_state == "reserved"
                && (row.binding_id != intent.reservation_id.to_string()
                    || row.reservation_id.as_deref() != Some(intent.reservation_id.to_string().as_str()))) {
            return Err(WorktreeVmError::EventLedgerReceiptMismatch);
        }
        if row.binding_state == "failed" && row.failure_reason.as_deref() == Some("reservation_released")
            && row.process_handle_json.is_empty() {
            if !row.adapter_id.is_empty() || row.binding_id != intent.reservation_id.to_string()
                || row.reservation_id.is_some() {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            return self.verify_owned_reservation_receipts(&intent, true).await;
        }
        let bindings = ReservationIntentReceiptBindings {
            event_id: self.event_id(&record_id, EVENT_RESERVED, &intent.reservation_id.to_string())?,
            event_type: EVENT_RESERVED.to_owned(),
            event_payload_hash: Self::reservation_intent_payload_hash(&intent)?,
            record_id, worktree_id: row.worktree_id.clone(),
            reservation_id: intent.reservation_id.to_string(),
            operation_id: intent.operation_id.to_string(),
            intent_sha256: sha256_hex(serde_json::to_string(&intent)?.as_bytes()),
            expected_generation: intent.expected_generation,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
        };
        let rows = self.storage.with_data_operation(|database| Box::pin(async move {
            database.query_values::<StoredReceiptRow, _>(r#"
                SELECT event_id, event_sequence FROM kernel_event_ledger
                WHERE id = type::record('kernel_event_ledger', $event_id)
                  AND event_id = $event_id AND idempotency_key = $event_id
                  AND event_version = 'kernel_event_v1' AND event_sequence > 0
                  AND event_type = $event_type AND payload_hash = $event_payload_hash
                  AND aggregate_type = 'worktree_vm_binding' AND aggregate_id = $record_id
                  AND source_component = 'worktree_vm_registry'
                  AND actor_kind = 'operator' AND actor_id = $actor_principal_id
                  AND payload.record_id = $record_id AND payload.worktree_id = $worktree_id
                  AND payload.binding_id = $reservation_id AND payload.binding_state = 'reserved'
                  AND payload.operation_id = $operation_id
                  AND payload.reservation_intent_sha256 = $intent_sha256
                  AND payload.expected_generation = $expected_generation
                  AND kernel_task_run_id = $workspace_id AND session_run_id = $authenticated_session_id
                  AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                  AND authenticated_session_id = $authenticated_session_id
                  AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;
            "#, bindings).await
        })).await?;
        if rows.len() != 1 { return Err(WorktreeVmError::EventLedgerReceiptMissing); }
        Ok(())
    }

    fn retained_reservation(&self, id: Uuid) -> Result<Arc<RetainedVmReservation>, WorktreeVmError> {
        let owner = RETAINED_VM_RESERVATIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).get(&id).cloned()
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if !Arc::ptr_eq(&owner.durable.schema_ready, &self.schema_ready)
            || owner.intent.scope != self.scope_strings()? {
            return Err(WorktreeVmError::ScopeDenied(ScopeDenied::ExactAttributionMismatch));
        }
        Ok(owner)
    }

    fn mark_reservation_invoked(&self, id: Uuid) -> Result<(), WorktreeVmError> {
        let owner = self.retained_reservation(id)?;
        if owner.release_claimed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        owner.invoked.compare_exchange(false, true,
            std::sync::atomic::Ordering::AcqRel, std::sync::atomic::Ordering::Acquire)
            .map_err(|_| WorktreeVmError::DurableHandleUnavailable {
                worktree_id: owner.intent.worktree_id.clone(), adapter_id: String::new(),
                reason: "original reservation already crossed acquisition; no duplicate effect".into(),
            })?;
        Ok(())
    }

    fn retain_reservation_handle(&self, id: Uuid, handle: &ProcessHandle) -> Result<(), WorktreeVmError> {
        let owner = self.retained_reservation(id)?;
        let mut slot = owner.handle.lock().unwrap_or_else(|error| error.into_inner());
        if slot.as_ref().is_some_and(|original| original != handle) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        *slot = Some(handle.clone());
        Ok(())
    }

    fn finish_reservation_delivery(&self, id: Uuid) {
        if self.retained_reservation(id).is_ok() {
            RETAINED_VM_RESERVATIONS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner()).remove(&id);
        }
    }

    async fn reconcile_known_reservation_binding(
        &self, worktree_id: &str, handle: &ProcessHandle, source_json: &str,
    ) -> Result<Option<(WorktreeVmBindingRecord, Uuid)>, WorktreeVmError> {
        let Some(row) = self.select_scoped_row(worktree_id).await? else { return Ok(None); };
        let Some(raw) = &row.reservation_intent_json else { return Ok(None); };
        let intent: OriginalVmReservationIntent = serde_json::from_str(raw)?;
        let owner = self.retained_reservation(intent.reservation_id)?;
        if owner.intent.worktree_id != worktree_id
            || owner.intent.source_sha256 != sha256_hex(source_json.as_bytes())
            || serde_json::to_string(&owner.intent)? != *raw
            || owner.handle.lock().unwrap_or_else(|error| error.into_inner()).as_ref() != Some(handle)
            || row.process_handle_json != serde_json::to_string(handle)?
            || row.binding_id != handle.id.to_string()
            || row.generation != intent.expected_generation
            || !matches!(row.binding_state.as_str(), "active" | "snapshotted") {
            return Ok(None);
        }
        let state = WorktreeVmBindingState::parse(&row.binding_state)?;
        Ok(Some((self.row_to_record(row, state)?, intent.reservation_id)))
    }
}


impl WorktreeVmDurableStore {
    async fn resume_reservation_for_source(
        &self,
        worktree_id: &str,
        source_json: &str,
        adapter: &Arc<dyn SandboxAdapter>,
    ) -> Result<Option<Uuid>, WorktreeVmError> {
        let owner = RETAINED_VM_RESERVATIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).values().find(|owner| {
                owner.intent.worktree_id == worktree_id
                    && Arc::ptr_eq(&owner.durable.schema_ready, &self.schema_ready)
            }).cloned();
        let Some(owner) = owner else { return Ok(None); };
        self.bind_original_warm_owner(&owner)?;
        if owner.intent.scope != self.scope_strings()?
            || owner.intent.source_sha256 != sha256_hex(source_json.as_bytes()) {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_owned(), adapter_id: adapter.capabilities().adapter_id.to_string(),
                reason: "request does not match the original retained reservation intent".into(),
            });
        }
        {
            let acquisition = owner.acquisition.lock().await;
            if !acquisition.adapter.as_ref().is_some_and(|original| Arc::ptr_eq(original, adapter)) {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
        }
        let row = self.select_scoped_row(worktree_id).await?;
        let Some(row) = row else {
            return if owner.invoked.load(std::sync::atomic::Ordering::Acquire) {
                Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: worktree_id.to_owned(), adapter_id: adapter.capabilities().adapter_id.to_string(),
                    reason: "original acquisition was invoked but its binding row is missing".into(),
                })
            } else {
                Ok(None) // Only the exact same-ID unused reserve replay can proceed.
            };
        };
        if !owner.invoked.load(std::sync::atomic::Ordering::Acquire)
            && matches!(row.binding_state.as_str(), "terminated" | "failed")
            && owner.intent.prior_generation == Some(row.generation)
            && owner.intent.prior_binding_id.as_ref() == Some(&row.binding_id)
            && owner.intent.prior_event_id.as_ref() == Some(&row.event_ledger_event_id)
            && owner.intent.prior_snapshot_json == row.latest_snapshot_json
            && owner.intent.prior_snapshot_cleanup_json == row.snapshot_cleanup_json {
            return Ok(None);
        }
        if row.generation != owner.intent.expected_generation
            || row.reservation_intent_json.as_deref() != Some(serde_json::to_string(&owner.intent)?.as_str()) {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: worktree_id.to_owned(), operation: "original reservation acquisition",
            });
        }
        if row.binding_state == "reserved"
            && row.reservation_id.as_deref() == Some(owner.intent.reservation_id.to_string().as_str()) {
            return Ok(Some(owner.intent.reservation_id));
        }
        let handle = owner.handle.lock().unwrap_or_else(|error| error.into_inner()).clone();
        if let Some(handle) = handle {
            if row.binding_id == handle.id.to_string()
                && row.process_handle_json == serde_json::to_string(&handle)?
                && matches!(row.binding_state.as_str(), "active" | "snapshotted") {
                return Ok(Some(owner.intent.reservation_id));
            }
        }
        Err(WorktreeVmError::DurableHandleUnavailable {
            worktree_id: worktree_id.to_owned(), adapter_id: adapter.capabilities().adapter_id.to_string(),
            reason: "original reservation has no matching live or reserved outcome; custody retained".into(),
        })
    }

    async fn observe_reservation_acquisition(
        &self,
        id: Uuid,
        adapter: Arc<dyn SandboxAdapter>,
        request: ReservationAcquisitionRequest,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        let owner = self.retained_reservation(id)?;
        self.bind_original_warm_owner(&owner)?;
        let source_json = match &request {
            ReservationAcquisitionRequest::Spawn(spec) => serde_json::to_string(spec)?,
            ReservationAcquisitionRequest::Restore(snapshot) => serde_json::to_string(snapshot)?,
        };
        if owner.intent.source_sha256 != sha256_hex(source_json.as_bytes()) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let mut acquisition = owner.acquisition.lock().await;
        if owner.release_claimed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if !acquisition.adapter.as_ref().is_some_and(|original| Arc::ptr_eq(original, &adapter)) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if let Some(handle) = owner.handle.lock().unwrap_or_else(|error| error.into_inner()).clone() {
            return Ok(handle);
        }
        if let Some(reason) = &acquisition.failed {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: owner.intent.worktree_id.clone(),
                adapter_id: adapter.capabilities().adapter_id.to_string(),
                reason: format!("original acquisition failed with ambiguous native effect: {reason}"),
            });
        }
        if acquisition.original.is_none() {
            if owner.invoked.load(std::sync::atomic::Ordering::Acquire) {
                return Err(WorktreeVmError::DurableHandleUnavailable {
                    worktree_id: owner.intent.worktree_id.clone(),
                    adapter_id: adapter.capabilities().adapter_id.to_string(),
                    reason: "invoked acquisition has no retained original future; no replacement is admitted".into(),
                });
            }
            let registry = owner.registry.clone();
            acquisition.original = Some(Box::pin(async move {
                let native = async move {
                    match request {
                        ReservationAcquisitionRequest::Spawn(spec) => adapter.spawn(spec).await,
                        ReservationAcquisitionRequest::Restore(snapshot) => adapter.restore(&snapshot).await,
                    }
                };
                crate::storage::surreal::with_captured_capability_registry(registry, native).await
            }));
            self.mark_reservation_invoked(id)?;
        }
        let observed = acquisition.original.as_mut().expect("retained original VM acquisition").as_mut().await;
        acquisition.original.take();
        match observed {
            Ok(handle) => {
                // No await separates the real return from its owning result storage.
                self.retain_reservation_handle(id, &handle)?;
                Ok(handle)
            }
            Err(error) => {
                acquisition.failed = Some(error.to_string());
                Err(error.into())
            }
        }
    }
}


pub(crate) struct WarmSnapshotRequest {
    owner_id: Uuid,
    worktree_id: String,
    expected: WorktreeVmBindingIdentity,
    capture_id: std::sync::Mutex<Option<Uuid>>,
    snapshot: std::sync::Mutex<Option<SnapshotRef>>,
    ready: std::sync::Mutex<Option<SnapshotWarmInput>>,
    invoked: std::sync::atomic::AtomicBool,
    accepted: std::sync::atomic::AtomicBool,
    closed: std::sync::atomic::AtomicBool,
    deletion: Mutex<SnapshotDeletion>,
}

impl WarmSnapshotRequest {
    pub(crate) fn new(owner_id: Uuid, worktree_id: String, expected: WorktreeVmBindingIdentity) -> Arc<Self> {
        Arc::new(Self {
            owner_id, worktree_id, expected, capture_id: std::sync::Mutex::new(None),
            snapshot: std::sync::Mutex::new(None), ready: std::sync::Mutex::new(None),
            invoked: std::sync::atomic::AtomicBool::new(false),
            accepted: std::sync::atomic::AtomicBool::new(false),
            closed: std::sync::atomic::AtomicBool::new(false),
            deletion: Mutex::new(SnapshotDeletion::default()),
        })
    }
}

#[derive(Clone)]
pub(crate) struct WarmSnapshotTicket {
    pub(crate) manifest: WarmVmSnapshotManifest,
    request: Arc<WarmSnapshotRequest>,
}

impl WorktreeVmRegistry {
    pub(crate) async fn with_original_warm_context<T>(
        &self,
        captured: crate::storage::surreal::CapturedModelLaneOperation,
        registry: Option<Arc<crate::capabilities::CapabilityRegistry>>,
        operation: impl std::future::Future<Output = T>,
    ) -> T {
        match &self.durable {
            Some(store) => crate::storage::surreal::with_captured_model_lane_operation(
                &store.storage, captured, operation,
            ).await,
            None => crate::storage::surreal::with_captured_capability_registry(registry, operation).await,
        }
    }

    pub(crate) async fn capture_selected_warm_snapshot(
        &self,
        request: Arc<WarmSnapshotRequest>,
        model_hash: &str,
        guest_path: &str,
        ready: &WarmAgentGuestFrame,
    ) -> Result<WarmSnapshotTicket, WorktreeVmError> {
        let warm = Self::selected_snapshot_readiness(model_hash, guest_path, ready)?;
        {
            let mut original = request.ready.lock().unwrap_or_else(|error| error.into_inner());
            if original.as_ref().is_some_and(|previous| previous != &warm) {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            *original = Some(warm.clone());
        }
        let (_, manifest, _) = self.snapshot_fenced(
            &request.worktree_id, Some(warm), Some(Arc::clone(&request)),
        ).await?;
        Ok(WarmSnapshotTicket {
            manifest: manifest.ok_or(WorktreeVmError::SnapshotSourceMissing)?, request,
        })
    }

    fn verify_warm_capture_owner(
        &self, request: &WarmSnapshotRequest, owner: &RetainedSnapshotDisposition,
    ) -> Result<(), WorktreeVmError> {
        let durable = self.durable.as_ref().ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        let ready = request.ready.lock().unwrap_or_else(|error| error.into_inner()).clone()
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if owner.intent.request_owner_id != request.owner_id
            || owner.intent.worktree_id != request.worktree_id
            || owner.intent.binding_id != request.expected.binding_id
            || owner.intent.generation != request.expected.generation
            || owner.intent.process_handle_json != serde_json::to_string(&request.expected.process_handle)?
            || owner.intent.scope != durable.scope_strings()?
            || owner.intent.ready_nonce.as_deref() != Some(ready.ready_nonce.as_str())
            || owner.intent.model_artifact_sha256.as_deref() != Some(ready.model_artifact_sha256.as_str())
            || owner.intent.model_guest_path.as_deref() != Some(ready.model_guest_path.as_str())
            || !Arc::ptr_eq(&owner.adapter, &self.adapter)
            || !Arc::ptr_eq(&owner.durable.schema_ready, &durable.schema_ready) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        Ok(())
    }

    pub(crate) async fn accept_selected_warm_snapshot<T>(
        &self, ticket: &WarmSnapshotTicket, publish: impl FnOnce() -> T,
    ) -> Result<T, WorktreeVmError> {
        let request = &ticket.request;
        let map = self.persistent.lock().await;
        let ready = request.ready.lock().unwrap_or_else(|error| error.into_inner()).clone()
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if ticket.manifest.worktree_id != request.worktree_id
            || ticket.manifest.ready_nonce != ready.ready_nonce
            || ticket.manifest.model_artifact_sha256 != ready.model_artifact_sha256
            || ticket.manifest.model_guest_path != ready.model_guest_path
            || request.snapshot.lock().unwrap_or_else(|error| error.into_inner()).as_ref()
                != Some(&ticket.manifest.snapshot) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if map.get(&request.worktree_id).is_some_and(|handle| handle != &request.expected.process_handle)
            || self.pending.lock().await.get(&request.worktree_id)
                .is_some_and(|handle| handle != &request.expected.process_handle) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if let Some(durable) = &self.durable {
            let _serial = durable.serializer(&request.worktree_id)?.lock_owned().await;
            let id = request.capture_id.lock().unwrap_or_else(|error| error.into_inner())
                .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
            let owner = RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner()).get(&id).cloned()
                .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
            self.verify_warm_capture_owner(request, &owner)?;
            let available = durable.snapshot_disposition_record(
                &owner.intent, SnapshotDispositionPhase::Available, Some(ticket.manifest.snapshot.clone()))?;
            let delivered = durable.snapshot_disposition_record(
                &owner.intent, SnapshotDispositionPhase::Delivered, Some(ticket.manifest.snapshot.clone()))?;
            durable.transition_snapshot_disposition(
                &delivered, Some(serde_json::to_string(&ticket.manifest.snapshot)?),
                Some(serde_json::to_string(&available)?),
                Some(serde_json::to_string(&ticket.manifest.snapshot)?),
                Some(serde_json::to_string(&delivered)?),
            ).await?;
            request.accepted.store(true, std::sync::atomic::Ordering::Release);
            RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner()).remove(&id);
            // The selected VM cannot be torn down or replaced between durable
            // acceptance and the caller's synchronous session adoption.
            Ok(publish())
        } else {
            let handle = map.get(&request.worktree_id).ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
            if transient_binding_identity(handle) != request.expected {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            let capture = self.selected_transient_capture(request, &ticket.manifest.snapshot)?;
            self.finish_transient_capture(&capture, &ticket.manifest.snapshot)?;
            request.accepted.store(true, std::sync::atomic::Ordering::Release);
            Ok(publish())
        }
    }

    pub(crate) async fn cleanup_selected_warm_snapshot(
        &self, request: &WarmSnapshotRequest,
    ) -> Result<(), WorktreeVmError> {
        if request.closed.load(std::sync::atomic::Ordering::Acquire)
            || request.accepted.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(());
        }
        let map = self.persistent.lock().await;
        if map.get(&request.worktree_id).is_some_and(|handle| handle != &request.expected.process_handle)
            || self.pending.lock().await.get(&request.worktree_id)
                .is_some_and(|handle| handle != &request.expected.process_handle) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        if let Some(durable) = &self.durable {
            let _serial = durable.serializer(&request.worktree_id)?.lock_owned().await;
            let id = *request.capture_id.lock().unwrap_or_else(|error| error.into_inner());
            let Some(id) = id else {
                return if !request.invoked.load(std::sync::atomic::Ordering::Acquire) {
                    request.closed.store(true, std::sync::atomic::Ordering::Release); Ok(())
                } else { Err(WorktreeVmError::SnapshotSourceMissing) };
            };
            let row = durable.select_scoped_row(&request.worktree_id).await?
                .ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
            if row.binding_id != request.expected.binding_id.to_string()
                || row.generation != request.expected.generation
                || row.process_handle_json != serde_json::to_string(&request.expected.process_handle)? {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            if let Some(raw) = &row.snapshot_cleanup_json {
                let disposition: SnapshotDispositionRecord = serde_json::from_str(raw)?;
                if disposition.phase == SnapshotDispositionPhase::Delivered
                    && disposition.intent.capture_id == id
                    && disposition.intent.request_owner_id == request.owner_id {
                    durable.verify_snapshot_disposition_receipt(&disposition).await?;
                    request.accepted.store(true, std::sync::atomic::Ordering::Release);
                    RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                        .unwrap_or_else(|error| error.into_inner()).remove(&id);
                    return Ok(()); // Delivered references are protected, never deleted.
                }
            }
            let owner = RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner()).get(&id).cloned()
                .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
            self.verify_warm_capture_owner(request, &owner)?;
            if owner.invoked.load(std::sync::atomic::Ordering::Acquire)
                && owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).is_none() {
                self.observe_original_snapshot_capture(&owner).await?;
            }
            self.cleanup_snapshot_disposition_locked(&owner).await?;
            request.closed.store(true, std::sync::atomic::Ordering::Release);
            RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner()).remove(&id);
        } else {
            if !request.invoked.load(std::sync::atomic::Ordering::Acquire) {
                request.closed.store(true, std::sync::atomic::Ordering::Release);
                return Ok(());
            }
            let handle = map.get(&request.worktree_id).ok_or(WorktreeVmError::SnapshotSourceMismatch)?;
            if transient_binding_identity(handle) != request.expected {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            let id = (*request.capture_id.lock().unwrap_or_else(|error| error.into_inner()))
                .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
            let retained_capture = RETAINED_TRANSIENT_CAPTURES.get_or_init(Default::default).lock()
                .unwrap_or_else(|error| error.into_inner()).get(&id).cloned()
                .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
            if !Arc::ptr_eq(&retained_capture.registry_identity, &self.transient_identity)
                || !Arc::ptr_eq(&retained_capture.adapter, &self.adapter)
                || retained_capture.worktree_id != request.worktree_id
                || retained_capture.binding != request.expected
                || retained_capture.selected_owner != Some(request.owner_id) {
                return Err(WorktreeVmError::SnapshotSourceMismatch);
            }
            let snapshot = self.observe_transient_capture(&retained_capture).await?;
            *request.snapshot.lock().unwrap_or_else(|error| error.into_inner()) = Some(snapshot.clone());
            let capture = self.selected_transient_capture(request, &snapshot)?;
            let mut deletion = request.deletion.lock().await;
            if !deletion.completed {
                if deletion.original.is_none() {
                    let adapter = Arc::clone(&self.adapter);
                    let original_snapshot = snapshot.clone();
                    deletion.original = Some(Box::pin(async move { adapter.delete_snapshot(&original_snapshot).await }));
                }
                let result = deletion.original.as_mut().expect("original selected snapshot cleanup").as_mut().await;
                deletion.original.take();
                result?;
                deletion.completed = true;
            }
            self.finish_transient_capture(&capture, &snapshot)?;
            request.closed.store(true, std::sync::atomic::Ordering::Release);
        }
        Ok(())
    }
}



struct TransientRegistryIdentity;

struct RetainedTransientAcquisition {
    operation_id: Uuid,
    registry_identity: Arc<TransientRegistryIdentity>,
    adapter: Arc<dyn SandboxAdapter>,
    worktree_id: String,
    _original_context: crate::storage::surreal::CapturedModelLaneOperation,
    handle: std::sync::Mutex<Option<ProcessHandle>>,
    acquisition: Mutex<ReservationAcquisitionProgress>,
}

static RETAINED_TRANSIENT_ACQUISITIONS: OnceLock<
    std::sync::Mutex<HashMap<Uuid, Arc<RetainedTransientAcquisition>>>,
> = OnceLock::new();

impl WorktreeVmRegistry {
    fn begin_transient_acquisition(
        &self, worktree_id: &str, request: ReservationAcquisitionRequest,
    ) -> Result<Arc<RetainedTransientAcquisition>, WorktreeVmError> {
        match &request {
            ReservationAcquisitionRequest::Spawn(_) => self.adapter.validate_spawn_acquisition()?,
            ReservationAcquisitionRequest::Restore(_) if !self.adapter.capabilities().supports_snapshot => {
                return Err(SandboxAdapterError::SnapshotUnsupported {
                    adapter_id: self.adapter.capabilities().adapter_id,
                }.into());
            }
            _ => {}
        }
        let mut pending = self.transient_acquisitions.lock().unwrap_or_else(|error| error.into_inner());
        let mut retained = RETAINED_TRANSIENT_ACQUISITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        let captures = RETAINED_TRANSIENT_CAPTURES.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if captures.values().any(|owner| owner.worktree_id == worktree_id && Arc::ptr_eq(&owner.adapter, &self.adapter)) {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(), adapter_id: self.adapter.capabilities().adapter_id.to_string(),
                reason: "original transient snapshot capture excludes replacement acquisition".into(),
            });
        }
        if pending.contains_key(worktree_id) || retained.values().any(|owner|
            owner.worktree_id == worktree_id && Arc::ptr_eq(&owner.adapter, &self.adapter)) {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(), adapter_id: self.adapter.capabilities().adapter_id.to_string(),
                reason: "original transient operation retains acquisition custody; no replacement is admitted".into(),
            });
        }
        let adapter = Arc::clone(&self.adapter);
        let registry = crate::storage::surreal::configured_capability_registry();
        let owner = Arc::new(RetainedTransientAcquisition {
            operation_id: Uuid::now_v7(), registry_identity: Arc::clone(&self.transient_identity),
            adapter: Arc::clone(&adapter), worktree_id: worktree_id.to_string(),
            _original_context: crate::storage::surreal::capture_model_lane_operation(),
            handle: std::sync::Mutex::new(None),
            acquisition: Mutex::new(ReservationAcquisitionProgress {
                adapter: Some(Arc::clone(&adapter)), failed: None,
                original: Some(Box::pin(async move {
                    let native = async move {
                        match request {
                            ReservationAcquisitionRequest::Spawn(spec) => adapter.spawn(spec).await,
                            ReservationAcquisitionRequest::Restore(snapshot) => adapter.restore(&snapshot).await,
                        }
                    };
                    crate::storage::surreal::with_captured_capability_registry(registry, native).await
                })),
            }),
        });
        pending.insert(worktree_id.to_string(), Arc::clone(&owner));
        retained.insert(owner.operation_id, Arc::clone(&owner));
        Ok(owner)
    }

    async fn observe_transient_acquisition(
        &self, owner: &Arc<RetainedTransientAcquisition>,
    ) -> Result<ProcessHandle, WorktreeVmError> {
        if !Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
            || !Arc::ptr_eq(&owner.adapter, &self.adapter) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let mut progress = owner.acquisition.lock().await;
        if let Some(handle) = owner.handle.lock().unwrap_or_else(|error| error.into_inner()).clone() {
            return Ok(handle);
        }
        if let Some(reason) = &progress.failed {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: owner.worktree_id.clone(), adapter_id: owner.adapter.capabilities().adapter_id.to_string(),
                reason: format!("original transient acquisition outcome remains unknown: {reason}"),
            });
        }
        let original = progress.original.as_mut().ok_or_else(||
            WorktreeVmError::DurableHandleUnavailable {
                worktree_id: owner.worktree_id.clone(), adapter_id: owner.adapter.capabilities().adapter_id.to_string(),
                reason: "original transient acquisition future is unavailable".into(),
            })?;
        let observed = original.as_mut().await;
        match &observed {
            Ok(handle) => *owner.handle.lock().unwrap_or_else(|error| error.into_inner()) = Some(handle.clone()),
            Err(error) => progress.failed = Some(error.to_string()),
        }
        progress.original.take();
        observed.map_err(WorktreeVmError::from)
    }

    fn finish_transient_acquisition(
        &self, owner: &Arc<RetainedTransientAcquisition>, handle: &ProcessHandle,
    ) -> Result<(), WorktreeVmError> {
        if !Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
            || !Arc::ptr_eq(&owner.adapter, &self.adapter)
            || owner.handle.lock().unwrap_or_else(|error| error.into_inner()).as_ref() != Some(handle) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let mut pending = self.transient_acquisitions.lock().unwrap_or_else(|error| error.into_inner());
        let mut retained = RETAINED_TRANSIENT_ACQUISITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if !pending.get(&owner.worktree_id).is_some_and(|current| Arc::ptr_eq(current, owner))
            || !retained.get(&owner.operation_id).is_some_and(|current| Arc::ptr_eq(current, owner)) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        pending.remove(&owner.worktree_id);
        retained.remove(&owner.operation_id);
        Ok(())
    }
}



pub(crate) struct WarmReservationOwner {
    request_owner_id: Uuid,
    registry_identity: Arc<TransientRegistryIdentity>,
    adapter: Arc<dyn SandboxAdapter>,
    worktree_id: String,
    original: std::sync::Mutex<Option<Arc<RetainedVmReservation>>>,
}

pub(crate) enum WarmReservationRelease { ConfirmedBeforeNative }

tokio::task_local! {
    static ORIGINAL_WARM_RESERVATION_OWNER: Arc<WarmReservationOwner>;
}

impl WorktreeVmRegistry {
    pub(crate) fn warm_reservation_owner(
        &self, request_owner_id: Uuid, worktree_id: &str,
    ) -> Arc<WarmReservationOwner> {
        Arc::new(WarmReservationOwner {
            request_owner_id, registry_identity: Arc::clone(&self.transient_identity),
            adapter: Arc::clone(&self.adapter), worktree_id: worktree_id.to_string(),
            original: std::sync::Mutex::new(None),
        })
    }

    pub(crate) async fn with_warm_reservation_owner<T>(
        &self, owner: Arc<WarmReservationOwner>, work: impl std::future::Future<Output = T>,
    ) -> Result<T, WorktreeVmError> {
        if !Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
            || !Arc::ptr_eq(&owner.adapter, &self.adapter) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        Ok(ORIGINAL_WARM_RESERVATION_OWNER.scope(owner, work).await)
    }

    pub(crate) async fn release_owned_warm_reservation(
        &self, custody: &Arc<WarmReservationOwner>,
    ) -> Result<WarmReservationRelease, WorktreeVmError> {
        if !Arc::ptr_eq(&custody.registry_identity, &self.transient_identity)
            || !Arc::ptr_eq(&custody.adapter, &self.adapter) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let owner = custody.original.lock().unwrap_or_else(|error| error.into_inner()).clone()
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        let durable = self.durable.as_ref().ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        let original = owner.warm_owner.as_ref().and_then(std::sync::Weak::upgrade)
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if !Arc::ptr_eq(&original, custody)
            || owner.intent.worktree_id != custody.worktree_id
            || !Arc::ptr_eq(&owner.durable.schema_ready, &durable.schema_ready)
            || owner.intent.scope != durable.scope_strings()? {
            return Err(WorktreeVmError::ScopeDenied(ScopeDenied::ExactAttributionMismatch));
        }
        let _serial = durable.serializer(&custody.worktree_id)?.lock_owned().await;
        let acquisition = owner.acquisition.lock().await;
        if owner.invoked.load(std::sync::atomic::Ordering::Acquire)
            || acquisition.original.is_some() || acquisition.failed.is_some()
            || owner.handle.lock().unwrap_or_else(|error| error.into_inner()).is_some()
            || !acquisition.adapter.as_ref().is_some_and(|adapter| Arc::ptr_eq(adapter, &self.adapter)) {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: custody.worktree_id.clone(), adapter_id: self.adapter.capabilities().adapter_id.to_string(),
                reason: "original reservation is not proven pre-native; custody remains pending".into(),
            });
        }
        owner.release_claimed.store(true, std::sync::atomic::Ordering::Release);
        durable.reconcile_reserved_only(&owner.intent).await?;
        let mut retained = RETAINED_VM_RESERVATIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if retained.get(&owner.intent.reservation_id).is_some_and(|current| Arc::ptr_eq(current, &owner)) {
            retained.remove(&owner.intent.reservation_id);
        }
        Ok(WarmReservationRelease::ConfirmedBeforeNative)
    }
}

#[derive(Debug, SurrealValue)]
struct OwnedCompensationReceiptBindings {
    event_id: String, reserve_event_id: String, released: bool,
    event_type: String, event_payload_hash: String, binding_state: String,
    record_id: String, worktree_id: String, reservation_id: String,
    operation_id: String, intent_sha256: String, expected_generation: i64,
    owner_account_id: String, actor_principal_id: String, authenticated_session_id: String,
    access_space_id: String, workspace_id: String,
}

impl WorktreeVmDurableStore {
    fn validate_owned_reservation(&self, intent: &OriginalVmReservationIntent) -> Result<(), WorktreeVmError> {
        if intent.version != 1 || intent.operation_id.is_nil() || intent.reservation_id.is_nil()
            || intent.expected_generation <= 0 || intent.record_id != self.record_id(&intent.worktree_id)?
            || intent.scope != self.scope_strings()? {
            return Err(WorktreeVmError::ScopeDenied(ScopeDenied::ExactAttributionMismatch));
        }
        Ok(())
    }

    fn bind_original_warm_owner(&self, owner: &Arc<RetainedVmReservation>) -> Result<(), WorktreeVmError> {
        if owner.release_claimed.load(std::sync::atomic::Ordering::Acquire) {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: owner.intent.worktree_id.clone(), adapter_id: String::new(),
                reason: "original reservation release is claimed; native entry remains excluded".into(),
            });
        }
        let current = ORIGINAL_WARM_RESERVATION_OWNER.try_with(Arc::clone).ok();
        match (&owner.warm_owner, current) {
            (None, None) => Ok(()),
            (Some(original), Some(current)) => {
                let original = original.upgrade().ok_or(WorktreeVmError::SnapshotSourceMissing)?;
                if !Arc::ptr_eq(&original, &current) || current.request_owner_id.is_nil()
                    || owner.intent.worktree_id != current.worktree_id
                    || owner.intent.scope != self.scope_strings()?
                    || !Arc::ptr_eq(&owner.durable.schema_ready, &self.schema_ready) {
                    return Err(WorktreeVmError::SnapshotSourceMismatch);
                }
                {
                    let acquisition = owner.acquisition.try_lock().map_err(|_| WorktreeVmError::SnapshotSourceMismatch)?;
                    if !acquisition.adapter.as_ref().is_some_and(|adapter| Arc::ptr_eq(adapter, &current.adapter)) {
                        return Err(WorktreeVmError::SnapshotSourceMismatch);
                    }
                }
                let mut held = current.original.lock().unwrap_or_else(|error| error.into_inner());
                if held.as_ref().is_some_and(|previous| !Arc::ptr_eq(previous, owner)) {
                    return Err(WorktreeVmError::SnapshotSourceMismatch);
                }
                *held = Some(Arc::clone(owner));
                Ok(())
            }
            _ => Err(WorktreeVmError::SnapshotSourceMismatch),
        }
    }

    fn reservation_state_payload_hash(
        intent: &OriginalVmReservationIntent, state: &str, event_type: &str,
    ) -> Result<String, WorktreeVmError> {
        Ok(sha256_hex(&canonical_json_bytes(&serde_json::json!({
            "binding_id": intent.reservation_id.to_string(), "binding_state": state,
            "record_id": intent.record_id, "transition_event_type": event_type,
            "worktree_id": intent.worktree_id, "operation_id": intent.operation_id.to_string(),
            "reservation_intent_sha256": sha256_hex(serde_json::to_string(intent)?.as_bytes()),
            "expected_generation": intent.expected_generation,
        }))))
    }

    async fn verify_owned_reservation_receipts(
        &self, intent: &OriginalVmReservationIntent, compensated: bool,
    ) -> Result<(), WorktreeVmError> {
        self.validate_owned_reservation(intent)?;
        for released in [false, true].into_iter().take(if compensated { 2 } else { 1 }) {
            let scope = self.scope_strings()?;
            let event_type = if released { EVENT_COMPENSATED } else { EVENT_RESERVED };
            let state = if released { "failed" } else { "reserved" };
            let bindings = OwnedCompensationReceiptBindings {
                event_id: self.event_id(&intent.record_id, event_type, &intent.reservation_id.to_string())?,
                reserve_event_id: self.event_id(&intent.record_id, EVENT_RESERVED, &intent.reservation_id.to_string())?,
                released, event_type: event_type.to_string(), binding_state: state.to_string(),
                event_payload_hash: Self::reservation_state_payload_hash(intent, state, event_type)?,
                record_id: intent.record_id.clone(), worktree_id: intent.worktree_id.clone(),
                reservation_id: intent.reservation_id.to_string(), operation_id: intent.operation_id.to_string(),
                intent_sha256: sha256_hex(serde_json::to_string(intent)?.as_bytes()),
                expected_generation: intent.expected_generation,
                owner_account_id: scope.owner_account_id, actor_principal_id: scope.actor_principal_id,
                authenticated_session_id: scope.authenticated_session_id, access_space_id: scope.access_space_id,
                workspace_id: scope.workspace_id,
            };
            let rows = self.storage.with_data_operation(|database| Box::pin(async move {
                database.query_values::<StoredReceiptRow, _>(r#"
                    SELECT event_id, event_sequence FROM kernel_event_ledger
                    WHERE id = type::record('kernel_event_ledger', $event_id)
                      AND event_id = $event_id AND idempotency_key = $event_id
                      AND event_version = 'kernel_event_v1' AND event_sequence > 0
                      AND aggregate_type = 'worktree_vm_binding' AND aggregate_id = $record_id
                      AND event_type = $event_type AND payload_hash = $event_payload_hash
                      AND source_component = 'worktree_vm_registry'
                      AND actor_kind = 'operator' AND actor_id = $actor_principal_id
                      AND kernel_task_run_id = $workspace_id AND session_run_id = $authenticated_session_id
                      AND payload.record_id = $record_id AND payload.worktree_id = $worktree_id
                      AND payload.binding_id = $reservation_id AND payload.binding_state = $binding_state
                      AND payload.transition_event_type = $event_type
                      AND payload.operation_id = $operation_id
                      AND payload.reservation_intent_sha256 = $intent_sha256
                      AND payload.expected_generation = $expected_generation
                      AND ($released = false OR causation_id = $reserve_event_id)
                      AND owner_account_id = $owner_account_id AND actor_principal_id = $actor_principal_id
                      AND authenticated_session_id = $authenticated_session_id
                      AND access_space_id = $access_space_id AND workspace_id = $workspace_id LIMIT 2;
                "#, bindings).await
            })).await?;
            if rows.len() != 1 { return Err(WorktreeVmError::EventLedgerReceiptMissing); }
        }
        Ok(())
    }

    async fn reconcile_reserved_only(&self, intent: &OriginalVmReservationIntent) -> Result<(), WorktreeVmError> {
        self.validate_owned_reservation(intent)?;
        if self.verify_owned_reservation_receipts(intent, true).await.is_ok() { return Ok(()); }
        let row = self.select_scoped_row(&intent.worktree_id).await?
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if row.generation != intent.expected_generation || row.binding_id != intent.reservation_id.to_string()
            || row.binding_state != "reserved" || row.reservation_id.as_deref() != Some(intent.reservation_id.to_string().as_str())
            || row.reservation_intent_json.as_deref() != Some(serde_json::to_string(intent)?.as_str())
            || !row.process_handle_json.is_empty() || !row.adapter_id.is_empty() {
            return Err(WorktreeVmError::StaleBinding {
                worktree_id: intent.worktree_id.clone(), operation: "original pre-native reservation compensation",
            });
        }
        self.verify_owned_reservation_receipts(intent, false).await?;
        let release = self.release_reservation_with_intent(&intent.worktree_id, intent).await;
        match self.verify_owned_reservation_receipts(intent, true).await {
            Ok(()) => Ok(()),
            Err(proof) => match release {
                Err(primary) => Err(WorktreeVmError::CompensationFailed {
                    worktree_id: intent.worktree_id.clone(), operation: "original reservation compensation response",
                    primary: primary.to_string(), cleanup: proof.to_string(),
                }),
                Ok(()) => Err(proof),
            },
        }
    }
}

#[derive(Debug, SurrealValue)]
struct ReleaseReservationIntentBindings {
    reserve_event_id: String,
    reserve_payload_hash: String,
    expected_generation: i64,
    operation_id: String,
    reservation_intent_json: String,
    reservation_intent_sha256: String,
    record_id: String,
    reservation_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
    updated_at_unix_ms: i64,
    event_id: String,
    event_type: String,
    event_payload_hash: String,
}
impl WorktreeVmDurableStore {
    async fn release_reservation_with_intent(
        &self, worktree_id: &str, intent: &OriginalVmReservationIntent,
    ) -> Result<(), WorktreeVmError> {
        self.validate_owned_reservation(intent)?;
        if worktree_id != intent.worktree_id { return Err(WorktreeVmError::SnapshotSourceMismatch); }
        let reservation_id = intent.reservation_id;
        self.ensure_schema().await?;
        let scope = self.scope_strings()?;
        let record_id = self.record_id(worktree_id)?;
        let reservation_text = reservation_id.to_string();
        let event_type = EVENT_COMPENSATED.to_owned();
        let event_id = self.event_id(&record_id, &event_type, &reservation_text)?;
        let reservation_intent_json = serde_json::to_string(intent)?;
        let reservation_intent_sha256 = sha256_hex(reservation_intent_json.as_bytes());
        let event_payload_hash = Self::reservation_state_payload_hash(intent, "failed", &event_type)?;
        let reserve_event_id = self.event_id(&record_id, EVENT_RESERVED, &reservation_text)?;
        let reserve_payload_hash = Self::reservation_state_payload_hash(intent, "reserved", EVENT_RESERVED)?;
        let bindings = ReleaseReservationIntentBindings {
            reserve_event_id, reserve_payload_hash,
            expected_generation: intent.expected_generation,
            operation_id: intent.operation_id.to_string(),
            reservation_intent_json,
            reservation_intent_sha256,
            record_id,
            reservation_id: reservation_text,
            owner_account_id: scope.owner_account_id,
            actor_principal_id: scope.actor_principal_id,
            authenticated_session_id: scope.authenticated_session_id,
            access_space_id: scope.access_space_id,
            workspace_id: scope.workspace_id,
            updated_at_unix_ms: chrono::Utc::now().timestamp_millis(),
            event_id,
            event_type,
            event_payload_hash,
        };
        let rows = self.storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database
                        .query_values_at::<StoredBindingRow, _>(
                            r#"
                            BEGIN TRANSACTION;
                            LET $record = type::record('worktree_vm_bindings', $record_id);
                            LET $current = (SELECT binding_id, worktree_id, adapter_id,
                                process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                generation, failure_reason, reservation_id, owner_account_id,
                                actor_principal_id, authenticated_session_id, access_space_id,
                                workspace_id, updated_at_unix_ms,
                                record::id(event_ledger_event_id) AS event_ledger_event_id,
                                event_ledger_event_type, event_ledger_payload_hash FROM $record
                                WHERE generation = $expected_generation
                                  AND reservation_intent_json = $reservation_intent_json
                                  AND owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                  AND event_ledger_event_id.owner_account_id = $owner_account_id
                                  AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                  AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                  AND event_ledger_event_id.access_space_id = $access_space_id
                                  AND event_ledger_event_id.workspace_id = $workspace_id
                                  AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                  AND event_ledger_event_id.aggregate_id = $record_id
                                  AND event_ledger_event_id.event_type = event_ledger_event_type
                                  AND event_ledger_event_id.payload_hash = event_ledger_payload_hash
                                  AND event_ledger_event_id.payload.record_id = $record_id
                                  AND event_ledger_event_id.payload.worktree_id = worktree_id
                                  AND event_ledger_event_id.payload.binding_id = binding_id
                                  AND event_ledger_event_id.payload.binding_state = binding_state
                                  AND event_ledger_event_id.payload.operation_id = $operation_id
                                  AND event_ledger_event_id.payload.expected_generation = $expected_generation
                                  AND event_ledger_event_id.payload.reservation_intent_sha256 = $reservation_intent_sha256);
                            IF array::len($current) = 1
                               AND $current[0].binding_state = 'failed'
                               AND $current[0].event_ledger_event_id = $event_id
                               AND $current[0].event_ledger_payload_hash = $event_payload_hash {
                                RETURN $current;
                            } ELSE IF array::len($current) = 1
                               AND $current[0].binding_state = 'reserved'
                               AND $current[0].process_handle_json = ''
                               AND $current[0].adapter_id = ''
                               AND $current[0].reservation_id = $reservation_id
                               AND $current[0].binding_id = $reservation_id
                               AND $current[0].generation = $expected_generation
                               AND $current[0].reservation_intent_json = $reservation_intent_json
                               AND $current[0].event_ledger_event_id = $reserve_event_id
                               AND $current[0].event_ledger_payload_hash = $reserve_payload_hash {
                                LET $prior = (SELECT event_id FROM kernel_event_ledger
                                    WHERE idempotency_key = $event_id
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id LIMIT 2);
                                IF array::len($prior) != 0 { THROW 'worktree VM compensation EventLedger receipt is orphaned or ambiguous'; };
                                LET $ledger = CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                                    event_id: $event_id, event_version: 'kernel_event_v1',
                                    kernel_task_run_id: $workspace_id,
                                    session_run_id: $authenticated_session_id,
                                    aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                                    idempotency_key: $event_id, event_type: $event_type,
                                    actor_kind: 'operator', actor_id: $actor_principal_id,
                                    causation_id: $current[0].event_ledger_event_id,
                                    correlation_id: $record_id, payload_hash: $event_payload_hash,
                                    source_component: 'worktree_vm_registry',
                                    payload: { transition_event_type: $event_type,
                                        record_id: $record_id, worktree_id: $current[0].worktree_id,
                                        binding_id: $reservation_id,
                                        binding_state: 'failed', expected_generation: $expected_generation,
                                        operation_id: $operation_id, reservation_intent_sha256: $reservation_intent_sha256 },
                                    owner_account_id: $owner_account_id,
                                    actor_principal_id: $actor_principal_id,
                                    authenticated_session_id: $authenticated_session_id,
                                    access_space_id: $access_space_id, workspace_id: $workspace_id,
                                    created_at: time::now()
                                };
                                LET $stored = UPDATE $record SET
                                    binding_state = 'failed', failure_reason = 'reservation_released',
                                    reservation_id = NONE, updated_at_unix_ms = $updated_at_unix_ms,
                                    event_ledger_event_id = type::record('kernel_event_ledger', $event_id),
                                    event_ledger_event_type = $event_type,
                                    event_ledger_payload_hash = $event_payload_hash
                                    WHERE binding_state = 'reserved'
                                      AND process_handle_json = '' AND adapter_id = ''
                                      AND reservation_id = $reservation_id
                                      AND binding_id = $reservation_id
                                      AND generation = $expected_generation
                                      AND reservation_intent_json = $reservation_intent_json
                                      AND event_ledger_event_id = type::record('kernel_event_ledger', $reserve_event_id)
                                      AND event_ledger_payload_hash = $reserve_payload_hash
                                      AND owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id;
                                LET $verified = (SELECT binding_id, worktree_id, adapter_id,
                                    process_handle_json, latest_snapshot_json, snapshot_cleanup_json, reservation_intent_json, binding_state,
                                    generation, failure_reason, reservation_id, owner_account_id,
                                    actor_principal_id, authenticated_session_id, access_space_id,
                                    workspace_id, updated_at_unix_ms,
                                    record::id(event_ledger_event_id) AS event_ledger_event_id,
                                    event_ledger_event_type, event_ledger_payload_hash FROM $record
                                    WHERE owner_account_id = $owner_account_id
                                      AND actor_principal_id = $actor_principal_id
                                      AND authenticated_session_id = $authenticated_session_id
                                      AND access_space_id = $access_space_id
                                      AND workspace_id = $workspace_id
                                      AND event_ledger_event_id = type::record('kernel_event_ledger', $event_id)
                                      AND event_ledger_event_id.owner_account_id = $owner_account_id
                                      AND event_ledger_event_id.actor_principal_id = $actor_principal_id
                                      AND event_ledger_event_id.authenticated_session_id = $authenticated_session_id
                                      AND event_ledger_event_id.access_space_id = $access_space_id
                                      AND event_ledger_event_id.workspace_id = $workspace_id
                                      AND event_ledger_event_id.event_id = $event_id
                                      AND event_ledger_event_id.idempotency_key = $event_id
                                      AND event_ledger_event_id.event_version = 'kernel_event_v1'
                                      AND event_ledger_event_id.kernel_task_run_id = $workspace_id
                                      AND event_ledger_event_id.session_run_id = $authenticated_session_id
                                      AND event_ledger_event_id.aggregate_type = 'worktree_vm_binding'
                                      AND event_ledger_event_id.aggregate_id = $record_id
                                      AND event_ledger_event_id.event_type = $event_type
                                      AND event_ledger_event_id.actor_kind = 'operator'
                                      AND event_ledger_event_id.actor_id = $actor_principal_id
                                      AND event_ledger_event_id.source_component = 'worktree_vm_registry'
                                      AND event_ledger_event_id.payload_hash = $event_payload_hash
                                    LIMIT 2);
                                IF array::len($verified) != 1 { THROW 'worktree VM compensation receipt verification failed'; };
                                RETURN $verified;
                            } ELSE {
                                RETURN [];
                            };
                            COMMIT TRANSACTION;
                            "#,
                            bindings,
                            3,
                        )
                        .await
                })
            })
            .await?;
        let row = rows
            .into_iter()
            .next()
            .ok_or_else(|| WorktreeVmError::StaleBinding {
                worktree_id: worktree_id.to_owned(),
                operation: "reservation compensation",
            })?;
        self.authorize_row_scope(&row)?;
        self.verify_receipt_linkage(worktree_id, &row).await?;
        Ok(())
    }

}


struct RetainedTransientCapture {
    operation_id: Uuid,
    registry_identity: Arc<TransientRegistryIdentity>,
    adapter: Arc<dyn SandboxAdapter>,
    worktree_id: String,
    binding: WorktreeVmBindingIdentity,
    selected_owner: Option<Uuid>,
    _original_context: crate::storage::surreal::CapturedModelLaneOperation,
    snapshot: std::sync::Mutex<Option<SnapshotRef>>,
    capture: Mutex<SnapshotCaptureProgress>,
}

static RETAINED_TRANSIENT_CAPTURES: OnceLock<
    std::sync::Mutex<HashMap<Uuid, Arc<RetainedTransientCapture>>>,
> = OnceLock::new();

impl WorktreeVmRegistry {
    fn reject_pending_transient_capture(&self, worktree_id: &str) -> Result<(), WorktreeVmError> {
        let local = self.transient_captures.lock().unwrap_or_else(|error| error.into_inner());
        let retained = RETAINED_TRANSIENT_CAPTURES.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if local.get(worktree_id).is_some_and(|owner|
            Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
                && Arc::ptr_eq(&owner.adapter, &self.adapter))
            || retained.values().any(|owner| owner.worktree_id == worktree_id
                && Arc::ptr_eq(&owner.adapter, &self.adapter)) {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(), adapter_id: self.adapter.capabilities().adapter_id.to_string(),
                reason: "original transient snapshot capture retains custody".into(),
            });
        }
        Ok(())
    }

    fn begin_transient_capture(
        &self, worktree_id: &str, handle: &ProcessHandle, selected_owner: Option<Uuid>,
    ) -> Result<Arc<RetainedTransientCapture>, WorktreeVmError> {
        if !self.adapter.capabilities().supports_snapshot {
            return Err(SandboxAdapterError::SnapshotUnsupported {
                adapter_id: self.adapter.capabilities().adapter_id,
            }.into());
        }
        let mut local = self.transient_captures.lock().unwrap_or_else(|error| error.into_inner());
        let acquisitions = RETAINED_TRANSIENT_ACQUISITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut retained = RETAINED_TRANSIENT_CAPTURES.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if local.contains_key(worktree_id)
            || acquisitions.values().any(|owner| owner.worktree_id == worktree_id && Arc::ptr_eq(&owner.adapter, &self.adapter))
            || retained.values().any(|owner| owner.worktree_id == worktree_id && Arc::ptr_eq(&owner.adapter, &self.adapter)) {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: worktree_id.to_string(), adapter_id: self.adapter.capabilities().adapter_id.to_string(),
                reason: "original transient acquisition/capture retains this worktree".into(),
            });
        }
        let adapter = Arc::clone(&self.adapter);
        let original_handle = handle.clone();
        let registry = crate::storage::surreal::configured_capability_registry();
        let owner = Arc::new(RetainedTransientCapture {
            operation_id: Uuid::now_v7(), registry_identity: Arc::clone(&self.transient_identity),
            adapter: Arc::clone(&adapter), worktree_id: worktree_id.to_string(),
            binding: transient_binding_identity(handle), selected_owner,
            _original_context: crate::storage::surreal::capture_model_lane_operation(),
            snapshot: std::sync::Mutex::new(None),
            capture: Mutex::new(SnapshotCaptureProgress {
                failed: None,
                original: Some(Box::pin(async move {
                    crate::storage::surreal::with_captured_capability_registry(
                        registry, adapter.snapshot(&original_handle),
                    ).await
                })),
            }),
        });
        local.insert(worktree_id.to_string(), Arc::clone(&owner));
        retained.insert(owner.operation_id, Arc::clone(&owner));
        Ok(owner)
    }

    async fn observe_transient_capture(
        &self, owner: &Arc<RetainedTransientCapture>,
    ) -> Result<SnapshotRef, WorktreeVmError> {
        if !Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
            || !Arc::ptr_eq(&owner.adapter, &self.adapter) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let mut progress = owner.capture.lock().await;
        if let Some(snapshot) = owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).clone() {
            return Ok(snapshot);
        }
        if let Some(reason) = &progress.failed {
            return Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: owner.worktree_id.clone(), adapter_id: owner.adapter.capabilities().adapter_id.to_string(),
                reason: format!("original transient capture has an ambiguous outcome: {reason}"),
            });
        }
        let observed = progress.original.as_mut().ok_or(WorktreeVmError::SnapshotSourceMissing)?.as_mut().await;
        match &observed {
            Ok(snapshot) => *owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()) = Some(snapshot.clone()),
            Err(error) => progress.failed = Some(error.to_string()),
        }
        progress.original.take();
        observed.map_err(WorktreeVmError::from)
    }

    fn selected_transient_capture(
        &self, request: &WarmSnapshotRequest, snapshot: &SnapshotRef,
    ) -> Result<Arc<RetainedTransientCapture>, WorktreeVmError> {
        let id = (*request.capture_id.lock().unwrap_or_else(|error| error.into_inner()))
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        let owner = RETAINED_TRANSIENT_CAPTURES.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).get(&id).cloned()
            .ok_or(WorktreeVmError::SnapshotSourceMissing)?;
        if !Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
            || !Arc::ptr_eq(&owner.adapter, &self.adapter)
            || owner.worktree_id != request.worktree_id || owner.binding != request.expected
            || owner.selected_owner != Some(request.owner_id)
            || owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).as_ref() != Some(snapshot) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        Ok(owner)
    }

    fn finish_transient_capture(
        &self, owner: &Arc<RetainedTransientCapture>, snapshot: &SnapshotRef,
    ) -> Result<(), WorktreeVmError> {
        if !Arc::ptr_eq(&owner.registry_identity, &self.transient_identity)
            || !Arc::ptr_eq(&owner.adapter, &self.adapter)
            || owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).as_ref() != Some(snapshot) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let progress = owner.capture.try_lock().map_err(|_| WorktreeVmError::SnapshotSourceMismatch)?;
        if progress.original.is_some() || progress.failed.is_some() {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        let mut local = self.transient_captures.lock().unwrap_or_else(|error| error.into_inner());
        let mut retained = RETAINED_TRANSIENT_CAPTURES.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner());
        if !local.get(&owner.worktree_id).is_some_and(|current| Arc::ptr_eq(current, owner))
            || !retained.get(&owner.operation_id).is_some_and(|current| Arc::ptr_eq(current, owner)) {
            return Err(WorktreeVmError::SnapshotSourceMismatch);
        }
        local.remove(&owner.worktree_id);
        retained.remove(&owner.operation_id);
        Ok(())
    }
}



impl WorktreeVmRegistry {
    fn guard_teardown_snapshot_capture(
        &self, durable: &WorktreeVmDurableStore, worktree_id: &str,
        row: Option<&StoredBindingRow>,
    ) -> Result<(), WorktreeVmError> {
        // Called under the original worktree serializer, before any teardown
        // effect. Receipt validation authenticates intent, not native completion.
        let Some(row) = row else { return Ok(()); };
        let Some(raw) = &row.snapshot_cleanup_json else { return Ok(()); };
        let record: SnapshotDispositionRecord = serde_json::from_str(raw)?;
        if record.phase != SnapshotDispositionPhase::Capturing { return Ok(()); }
        let pending = || WorktreeVmError::DurableHandleUnavailable {
            worktree_id: worktree_id.to_string(), adapter_id: row.adapter_id.clone(),
            reason: "original durable snapshot capture has no confirmed native result; teardown remains pending".into(),
        };
        let owner = RETAINED_SNAPSHOT_DISPOSITIONS.get_or_init(Default::default).lock()
            .unwrap_or_else(|error| error.into_inner()).get(&record.intent.capture_id).cloned()
            .ok_or_else(pending)?;
        if !Arc::ptr_eq(&owner.adapter, &self.adapter)
            || !Arc::ptr_eq(&owner.durable.schema_ready, &durable.schema_ready)
            || owner.intent.scope != durable.scope_strings()?
            || owner.intent.worktree_id != worktree_id
            || row.worktree_id != worktree_id
            || row.adapter_id != self.adapter.capabilities().adapter_id.as_str()
            || row.binding_id != owner.intent.binding_id.to_string()
            || row.generation != owner.intent.generation
            || row.process_handle_json != owner.intent.process_handle_json
            || serde_json::to_string(&owner.intent)? != serde_json::to_string(&record.intent)?
            || !owner.invoked.load(std::sync::atomic::Ordering::Acquire) {
            return Err(pending());
        }
        let progress = owner.capture.try_lock().map_err(|_| pending())?;
        if progress.original.is_some() || progress.failed.is_some()
            || owner.snapshot.lock().unwrap_or_else(|error| error.into_inner()).is_none() {
            return Err(pending());
        }
        Ok(())
    }
}



fn reject_reused_snapshot_target(
    prior_json: Option<&str>, snapshot: &SnapshotRef,
) -> Result<(), WorktreeVmError> {
    let prior = prior_json.map(serde_json::from_str::<SnapshotRef>).transpose()?;
    if prior.as_ref().is_some_and(|prior|
        prior.id == snapshot.id
            || (prior.adapter_id == snapshot.adapter_id && prior.snapshot_dir == snapshot.snapshot_dir)) {
        return Err(WorktreeVmError::SnapshotSourceMismatch);
    }
    Ok(())
}

