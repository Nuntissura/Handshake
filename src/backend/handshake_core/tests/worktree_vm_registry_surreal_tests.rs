//! WP-1 MT-023: embedded-SurrealDB proof for durable worktree microVM ownership.
//!
//! The deterministic adapter isolates registry authority, recovery, exact scope,
//! generation fencing, and terminal state. The separate live target owns the
//! real Cloud Hypervisor/KVM/model boundary.

use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use handshake_core::{
    model_runtime::{WarmAgentGuestFrame, WARM_AGENT_PROTOCOL_ID, WARM_AGENT_PROTOCOL_VERSION},
    sandbox::{
        AdapterCapabilities, AdapterId, BindMode, Command, ExecResult, GpuPassthrough,
        IsolationStrength, IsolationTier, NetPolicy, ProcessHandle, ProcessSpec, ProcessStatus,
        SandboxAdapter, SandboxAdapterError, Signal, SnapshotRef, ThroughputClass,
    },
    storage::surreal::SurrealStorage,
    swarm_orchestration::{
        resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
            ResourceAccessContext, ResourceScope, ScopeDenied, WorkspaceScopeRef,
        },
        worktree_vm_registry::{WorktreeVmBindingState, WorktreeVmError, WorktreeVmRegistry},
    },
};
use sha2::{Digest, Sha256};
use surrealdb::types::SurrealValue;
use uuid::Uuid;

mod surreal_test_store_support;

use surreal_test_store_support::EmbeddedSurrealTestScope;

struct StoreHarness {
    scope: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
}

impl StoreHarness {
    async fn create(_slug: &str) -> Self {
        let mut scope = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate exact embedded SurrealDB test scope");
        let storage = scope
            .activate_storage()
            .await
            .expect("activate production SurrealStorage for MT-023");
        Self { scope, storage }
    }

    async fn restart_storage(&mut self) -> SurrealStorage {
        self.scope
            .shutdown_storage_for_reopen()
            .await
            .expect("close production SurrealStorage before restart");
        self.scope
            .reopen()
            .await
            .expect("reopen exact embedded SurrealDB test scope");
        self.storage = self
            .scope
            .activate_storage()
            .await
            .expect("reactivate production SurrealStorage after restart");
        self.storage.clone()
    }

    async fn close(self) {
        let Self { mut scope, storage } = self;
        drop(storage);
        let cleanup = scope
            .cleanup()
            .await
            .expect("clean exact embedded SurrealDB test scope");
        assert!(cleanup.database_absent);
        assert!(cleanup.namespace_absent_after_reopen);
    }
}

#[derive(Default)]
struct AdapterState {
    statuses: HashMap<Uuid, ProcessStatus>,
    handles: HashMap<Uuid, ProcessHandle>,
}

#[derive(Default)]
struct DurableAdapter {
    fail_next_spawn: AtomicBool,
    spawn_count: AtomicUsize,
    kill_count: AtomicUsize,
    snapshot_count: AtomicUsize,
    restore_count: AtomicUsize,
    ordinal: AtomicUsize,
    state: Mutex<AdapterState>,
}

impl DurableAdapter {
    fn fail_next_spawn(&self) {
        self.fail_next_spawn.store(true, Ordering::SeqCst);
    }

    fn create_running_handle(&self, label: &str) -> ProcessHandle {
        let ordinal = self.ordinal.fetch_add(1, Ordering::SeqCst) + 1;
        let handle = ProcessHandle::new(
            AdapterId::new("cloud_hypervisor"),
            None,
            format!("hsk-surreal-{label}-{ordinal}"),
        );
        let mut state = self.state.lock().expect("adapter state lock");
        state.statuses.insert(handle.id, ProcessStatus::Running);
        state.handles.insert(handle.id, handle.clone());
        handle
    }
}

#[async_trait]
impl SandboxAdapter for DurableAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        self.spawn_count.fetch_add(1, Ordering::SeqCst);
        if self.fail_next_spawn.swap(false, Ordering::SeqCst) {
            return Err(SandboxAdapterError::SpawnFailed {
                adapter_id: AdapterId::new("cloud_hypervisor"),
                reason: "MT-023 deterministic spawn failure".to_owned(),
            });
        }
        Ok(self.create_running_handle("spawn"))
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
        handle: &ProcessHandle,
        signal: Signal,
    ) -> Result<(), SandboxAdapterError> {
        self.kill_count.fetch_add(1, Ordering::SeqCst);
        let mut state = self.state.lock().expect("adapter state lock");
        let status =
            state
                .statuses
                .get_mut(&handle.id)
                .ok_or(SandboxAdapterError::ProcessHandleStale {
                    process_id: handle.id,
                })?;
        *status = ProcessStatus::Killed { by_signal: signal };
        Ok(())
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        self.state
            .lock()
            .expect("adapter state lock")
            .statuses
            .get(&handle.id)
            .cloned()
            .ok_or(SandboxAdapterError::ProcessHandleStale {
                process_id: handle.id,
            })
    }

    async fn exit_code(&self, handle: &ProcessHandle) -> Result<Option<i32>, SandboxAdapterError> {
        Ok(match self.status(handle).await? {
            ProcessStatus::Exited { code } => Some(code),
            _ => None,
        })
    }

    async fn snapshot(&self, handle: &ProcessHandle) -> Result<SnapshotRef, SandboxAdapterError> {
        self.snapshot_count.fetch_add(1, Ordering::SeqCst);
        if !matches!(self.status(handle).await?, ProcessStatus::Running) {
            return Err(SandboxAdapterError::SnapshotFailed {
                adapter_id: handle.adapter_id.clone(),
                reason: "source handle is not running".to_owned(),
            });
        }
        Ok(SnapshotRef::new(
            handle.adapter_id.clone(),
            format!("/mt023-snapshots/{}", handle.id),
        ))
    }

    async fn restore(&self, _snapshot: &SnapshotRef) -> Result<ProcessHandle, SandboxAdapterError> {
        self.restore_count.fetch_add(1, Ordering::SeqCst);
        Ok(self.create_running_handle("restore"))
    }

    async fn delete_snapshot(&self, _snapshot: &SnapshotRef) -> Result<(), SandboxAdapterError> {
        Ok(())
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

fn exact_scope(workspace: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(WorkspaceScopeRef::new(workspace).expect("valid workspace scope"))
}

fn ready_frame(model_hash: &str, model_guest_path: &str) -> WarmAgentGuestFrame {
    WarmAgentGuestFrame::Ready {
        protocol_id: WARM_AGENT_PROTOCOL_ID.to_owned(),
        protocol_version: WARM_AGENT_PROTOCOL_VERSION,
        agent_id: "mt023-surreal-warm-agent".to_owned(),
        ready_nonce: "mt023-surreal-ready".to_owned(),
        loaded_model_sha256: Some(model_hash.to_owned()),
        loaded_model_guest_path: Some(model_guest_path.to_owned()),
    }
}

#[derive(Debug, SurrealValue)]
struct ReceiptQueryBindings {
    worktree_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, SurrealValue)]
struct ReceiptProofRow {
    event_id: String,
    event_sequence: i64,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: String,
    payload_hash: String,
    source_component: String,
    transition_event_type: String,
    record_id: String,
    payload_worktree_id: String,
    binding_id: String,
    binding_state: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct EventCountRow {
    count: i64,
}

#[derive(Debug, SurrealValue)]
struct ReceiptTamperBindings {
    event_id: String,
    replacement_event_id: String,
    foreign_owner_account_id: String,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

fn tamper_bindings(
    scope: &ResourceScope,
    event_id: String,
    replacement_event_id: String,
) -> ReceiptTamperBindings {
    let exact = receipt_bindings(scope, "unused");
    ReceiptTamperBindings {
        event_id,
        replacement_event_id,
        foreign_owner_account_id: OwnerAccountId::mint().as_uuid().to_string(),
        owner_account_id: exact.owner_account_id,
        actor_principal_id: exact.actor_principal_id,
        authenticated_session_id: exact.authenticated_session_id,
        access_space_id: exact.access_space_id,
        workspace_id: exact.workspace_id,
    }
}

fn receipt_bindings(scope: &ResourceScope, worktree_id: &str) -> ReceiptQueryBindings {
    ReceiptQueryBindings {
        worktree_id: worktree_id.to_owned(),
        owner_account_id: scope.owner_account_id.as_uuid().to_string(),
        actor_principal_id: scope.actor_principal_id.as_uuid().to_string(),
        authenticated_session_id: scope
            .authenticated_session
            .expect("exact test scope has session")
            .as_uuid()
            .to_string(),
        access_space_id: scope
            .access_space
            .expect("exact test scope has AccessSpace")
            .as_uuid()
            .to_string(),
        workspace_id: scope
            .workspace
            .as_ref()
            .expect("exact test scope has workspace")
            .as_str()
            .to_owned(),
    }
}

async fn receipt_history(
    storage: &SurrealStorage,
    scope: &ResourceScope,
    worktree_id: &str,
) -> Vec<ReceiptProofRow> {
    let bindings = receipt_bindings(scope, worktree_id);
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<ReceiptProofRow, _>(
                        r#"
                        SELECT event_id, event_sequence, event_version,
                               kernel_task_run_id, session_run_id,
                               aggregate_type, aggregate_id, idempotency_key,
                               event_type, actor_kind, actor_id, causation_id,
                               correlation_id, payload_hash, source_component,
                               payload.transition_event_type AS transition_event_type,
                               payload.record_id AS record_id,
                               payload.worktree_id AS payload_worktree_id,
                               payload.binding_id AS binding_id,
                               payload.binding_state AS binding_state,
                               owner_account_id, actor_principal_id,
                               authenticated_session_id, access_space_id,
                               workspace_id
                        FROM kernel_event_ledger
                        WHERE payload.worktree_id = $worktree_id
                          AND aggregate_type = 'worktree_vm_binding'
                          AND source_component = 'worktree_vm_registry'
                          AND owner_account_id = $owner_account_id
                          AND actor_principal_id = $actor_principal_id
                          AND authenticated_session_id = $authenticated_session_id
                          AND access_space_id = $access_space_id
                          AND workspace_id = $workspace_id
                        ORDER BY event_sequence ASC;
                        "#,
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("read exact-scope canonical registry receipts");
    assert_receipt_attribution(&rows, scope, worktree_id);
    rows
}

fn assert_receipt_attribution(
    receipts: &[ReceiptProofRow],
    scope: &ResourceScope,
    worktree_id: &str,
) {
    let exact = receipt_bindings(scope, worktree_id);
    assert!(
        !receipts.is_empty(),
        "accepted transitions must emit receipts"
    );
    for (index, receipt) in receipts.iter().enumerate() {
        assert!(
            receipt.event_sequence > 0,
            "canonical sequence must be positive"
        );
        if let Some(previous) = index.checked_sub(1).map(|value| &receipts[value]) {
            assert!(
                receipt.event_sequence > previous.event_sequence,
                "canonical receipt sequence must increase"
            );
            assert_eq!(
                receipt.causation_id.as_deref(),
                Some(previous.event_id.as_str())
            );
        } else {
            assert_eq!(receipt.causation_id, None);
        }
        assert_eq!(receipt.event_version, "kernel_event_v1");
        assert_eq!(receipt.kernel_task_run_id, exact.workspace_id);
        assert_eq!(receipt.session_run_id, exact.authenticated_session_id);
        assert_eq!(receipt.aggregate_type, "worktree_vm_binding");
        assert_eq!(receipt.aggregate_id, receipt.record_id);
        assert_eq!(receipt.idempotency_key, receipt.event_id);
        assert_eq!(receipt.transition_event_type, receipt.event_type);
        assert_eq!(receipt.actor_kind, "operator");
        assert_eq!(receipt.actor_id, exact.actor_principal_id);
        assert_eq!(receipt.correlation_id, receipt.record_id);
        assert_eq!(receipt.source_component, "worktree_vm_registry");
        assert_eq!(receipt.payload_worktree_id, worktree_id);
        assert_eq!(receipt.owner_account_id, exact.owner_account_id);
        assert_eq!(receipt.actor_principal_id, exact.actor_principal_id);
        assert_eq!(
            receipt.authenticated_session_id,
            exact.authenticated_session_id
        );
        assert_eq!(receipt.access_space_id, exact.access_space_id);
        assert_eq!(receipt.workspace_id, exact.workspace_id);

        let payload = BTreeMap::from([
            ("binding_id", receipt.binding_id.as_str()),
            ("binding_state", receipt.binding_state.as_str()),
            ("record_id", receipt.record_id.as_str()),
            (
                "transition_event_type",
                receipt.transition_event_type.as_str(),
            ),
            ("worktree_id", receipt.payload_worktree_id.as_str()),
        ]);
        let payload_bytes = serde_json::to_vec(&payload).expect("canonical flat receipt payload");
        assert_eq!(
            receipt.payload_hash,
            hex::encode(Sha256::digest(payload_bytes))
        );
    }
}

async fn unscoped_event_count(storage: &SurrealStorage, worktree_id: &str) -> i64 {
    #[derive(Debug, SurrealValue)]
    struct WorktreeBinding {
        worktree_id: String,
    }

    let bindings = WorktreeBinding {
        worktree_id: worktree_id.to_owned(),
    };
    storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_first::<EventCountRow, _>(
                        "SELECT count() AS count FROM kernel_event_ledger WHERE payload.worktree_id = $worktree_id GROUP ALL;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("count raw canonical registry receipts")
        .map_or(0, |row| row.count)
}

fn registry_record_id(worktree_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"handshake.worktree-vm-binding.v1\0");
    digest.update(format!("worktree-vm:{worktree_id}").as_bytes());
    hex::encode(digest.finalize())
}

#[tokio::test]
async fn durable_binding_survives_registry_restart_and_terminalizes() {
    let mut harness = StoreHarness::create("restart").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-restart");
    let worktree_id = format!("wt-mt023-restart-{}", Uuid::now_v7());

    let owner = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    let handle = owner
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create durable SurrealDB binding");
    let before_restart = receipt_history(&harness.storage, &scope, &worktree_id).await;
    assert_eq!(
        before_restart
            .iter()
            .map(|receipt| receipt.event_type.as_str())
            .collect::<Vec<_>>(),
        ["TASK_INTENT_RECORDED", "SESSION_STARTED"]
    );
    let started_receipt = before_restart[1].event_id.clone();
    drop(owner);

    let restarted_storage = harness.restart_storage().await;

    let restarted = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        restarted_storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    assert_eq!(
        restarted
            .resolve_worktree_vm(&worktree_id)
            .await
            .expect("adopt durable binding after registry restart"),
        handle
    );
    let after_restart = receipt_history(&restarted_storage, &scope, &worktree_id).await;
    assert_eq!(after_restart, before_restart);
    assert_eq!(after_restart[1].event_id, started_receipt);
    assert_eq!(after_restart[1].binding_state, "active");
    assert_eq!(
        restarted
            .durable_binding(&worktree_id)
            .await
            .expect("verify receipt-linked generation after restart")
            .expect("active binding after restart")
            .generation,
        1
    );
    restarted
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("terminalize durable binding");
    let binding = restarted
        .durable_binding(&worktree_id)
        .await
        .expect("read terminal binding")
        .expect("terminal binding remains auditable");
    assert_eq!(binding.binding_state, WorktreeVmBindingState::Terminated);
    let after_teardown = receipt_history(&restarted_storage, &scope, &worktree_id).await;
    assert_eq!(
        after_teardown
            .iter()
            .map(|receipt| receipt.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "TASK_INTENT_RECORDED",
            "SESSION_STARTED",
            "SESSION_COMPLETED"
        ]
    );
    assert_eq!(adapter.kill_count.load(Ordering::SeqCst), 1);
    restarted
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("terminal retry is idempotent");
    assert_eq!(
        receipt_history(&restarted_storage, &scope, &worktree_id).await,
        after_teardown,
        "terminal retry must not append a duplicate receipt"
    );
    assert_eq!(adapter.kill_count.load(Ordering::SeqCst), 1);
    drop(restarted);
    drop(restarted_storage);
    harness.close().await;
}

#[tokio::test]
async fn snapshot_restore_receipts_are_atomic_and_restart_stable() {
    let mut harness = StoreHarness::create("restore-receipts").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-restore-receipts");
    let worktree_id = format!("wt-mt023-restore-receipts-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create warm source VM");
    let manifest = registry
        .snapshot_warm_model(
            &worktree_id,
            "mt023-model-sha256",
            "/models/mt023.gguf",
            &ready_frame("mt023-model-sha256", "/models/mt023.gguf"),
        )
        .await
        .expect("snapshot with canonical receipt");
    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("terminalize source generation");
    registry
        .restore_warm_model(&manifest, "mt023-model-sha256", "/models/mt023.gguf")
        .await
        .expect("restore with canonical receipt");

    let receipts = receipt_history(&harness.storage, &scope, &worktree_id).await;
    assert_eq!(
        receipts
            .iter()
            .map(|receipt| receipt.event_type.as_str())
            .collect::<Vec<_>>(),
        [
            "TASK_INTENT_RECORDED",
            "SESSION_STARTED",
            "ARTIFACT_STORED",
            "SESSION_COMPLETED",
            "TASK_INTENT_RECORDED",
            "TRACE_REPLAYED"
        ]
    );
    let restored_receipt = receipts.last().expect("restore receipt").event_id.clone();
    assert_eq!(
        registry
            .durable_binding(&worktree_id)
            .await
            .expect("verify restored receipt-linked generation")
            .expect("restored generation exists")
            .generation,
        2
    );
    drop(registry);

    let restarted_storage = harness.restart_storage().await;
    let restarted = WorktreeVmRegistry::new_durable(
        adapter,
        restarted_storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    let restored = restarted
        .durable_binding(&worktree_id)
        .await
        .expect("verify restored receipt after storage restart")
        .expect("restored binding remains durable");
    assert_eq!(restored.binding_state, WorktreeVmBindingState::Snapshotted);
    let after_restart = receipt_history(&restarted_storage, &scope, &worktree_id).await;
    assert_eq!(
        after_restart.last().expect("restart receipt").event_id,
        restored_receipt
    );
    restarted
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean restored generation");
    let terminal_history = receipt_history(&restarted_storage, &scope, &worktree_id).await;
    assert_eq!(terminal_history.len(), 7);
    restarted
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("restored terminal retry is idempotent");
    assert_eq!(
        receipt_history(&restarted_storage, &scope, &worktree_id).await,
        terminal_history
    );
    drop(restarted);
    drop(restarted_storage);
    harness.close().await;
}

#[tokio::test]
async fn failed_spawn_compensation_is_receipted_and_retry_advances_generation() {
    let harness = StoreHarness::create("compensation").await;
    let adapter = Arc::new(DurableAdapter::default());
    adapter.fail_next_spawn();
    let scope = exact_scope("mt023-compensation");
    let worktree_id = format!("wt-mt023-compensation-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    assert!(matches!(
        registry.ensure_worktree_vm(&worktree_id).await,
        Err(WorktreeVmError::Sandbox(
            SandboxAdapterError::SpawnFailed { .. }
        ))
    ));
    assert_eq!(
        registry
            .durable_binding(&worktree_id)
            .await
            .expect("compensated row is verified before non-live filtering"),
        None
    );
    let failed_receipts = receipt_history(&harness.storage, &scope, &worktree_id).await;
    assert_eq!(
        failed_receipts
            .iter()
            .map(|receipt| receipt.event_type.as_str())
            .collect::<Vec<_>>(),
        ["TASK_INTENT_RECORDED", "SESSION_FAILED"]
    );

    registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("retry creates a new receipted generation");
    let active = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read retry generation")
        .expect("retry generation exists");
    assert_eq!(active.generation, 2);
    assert_eq!(active.binding_state, WorktreeVmBindingState::Active);
    let before_idempotent_retry = receipt_history(&harness.storage, &scope, &worktree_id).await;
    assert_eq!(before_idempotent_retry.len(), 4);
    registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("active ensure retry is idempotent");
    assert_eq!(
        receipt_history(&harness.storage, &scope, &worktree_id).await,
        before_idempotent_retry,
        "accepted idempotent retry must not duplicate a canonical event"
    );
    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean retry generation");
    harness.close().await;
}

#[tokio::test]
async fn missing_or_foreign_event_receipt_fails_closed_without_vm_mutation() {
    let harness = StoreHarness::create("receipt-denial").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-receipt-denial");

    for foreign_scope in [false, true] {
        let worktree_id = format!("wt-mt023-receipt-denial-{}", Uuid::now_v7());
        let registry = WorktreeVmRegistry::new_durable(
            adapter.clone(),
            harness.storage.clone(),
            ResourceAccessContext::for_account(scope.clone()),
        );
        let handle = registry
            .ensure_worktree_vm(&worktree_id)
            .await
            .expect("create receipt-linked binding");
        let receipts = receipt_history(&harness.storage, &scope, &worktree_id).await;
        let event_id = receipts.last().expect("start receipt").event_id.clone();
        let bindings = tamper_bindings(&scope, event_id, String::new());
        let mutation = harness
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    if foreign_scope {
                        database
                            .execute_returning(
                                r#"
                                UPDATE type::record('kernel_event_ledger', $event_id)
                                SET owner_account_id = $foreign_owner_account_id
                                WHERE owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                RETURN AFTER;
                                "#,
                                bindings,
                            )
                            .await
                    } else {
                        database
                            .execute_returning(
                                r#"
                                DELETE type::record('kernel_event_ledger', $event_id)
                                WHERE owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                RETURN BEFORE;
                                "#,
                                bindings,
                            )
                            .await
                    }
                })
            })
            .await;
        if !foreign_scope {
            assert!(
                mutation.is_err(),
                "typed receipt reference must reject orphan creation"
            );
            assert!(registry
                .durable_binding(&worktree_id)
                .await
                .expect("rejected orphan mutation preserves readable binding")
                .is_some());
            continue;
        }
        assert_eq!(
            mutation.expect("apply exact-scope foreign-receipt counterfactual"),
            1
        );
        assert!(matches!(
            registry.durable_binding(&worktree_id).await,
            Err(WorktreeVmError::EventLedgerReceiptMissing)
                | Err(WorktreeVmError::EventLedgerReceiptMismatch)
        ));
        assert!(matches!(
            registry.ensure_worktree_vm(&worktree_id).await,
            Err(WorktreeVmError::EventLedgerReceiptMissing)
                | Err(WorktreeVmError::EventLedgerReceiptMismatch)
                | Err(WorktreeVmError::Storage(_))
        ));
        assert_eq!(
            adapter
                .status(&handle)
                .await
                .expect("original VM remains live"),
            ProcessStatus::Running
        );
    }
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 2);
    harness.close().await;
}

#[tokio::test]
async fn tampered_or_wrong_receipt_linkage_fails_closed_after_restart() {
    let mut harness = StoreHarness::create("receipt-tamper").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-receipt-tamper");

    for wrong_link in [false, true] {
        let worktree_id = format!("wt-mt023-receipt-tamper-{}", Uuid::now_v7());
        let registry = WorktreeVmRegistry::new_durable(
            adapter.clone(),
            harness.storage.clone(),
            ResourceAccessContext::for_account(scope.clone()),
        );
        let handle = registry
            .ensure_worktree_vm(&worktree_id)
            .await
            .expect("create receipt-linked binding");
        let receipts = receipt_history(&harness.storage, &scope, &worktree_id).await;
        let reserve_id = receipts[0].event_id.clone();
        let start_id = receipts[1].event_id.clone();
        let bindings = tamper_bindings(&scope, start_id, reserve_id);
        let affected = harness
            .storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    if wrong_link {
                        database
                            .execute_returning(
                                r#"
                                UPDATE worktree_vm_bindings
                                SET event_ledger_event_id = type::record('kernel_event_ledger', $replacement_event_id)
                                WHERE event_ledger_event_id = type::record('kernel_event_ledger', $event_id)
                                  AND owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                RETURN AFTER;
                                "#,
                                bindings,
                            )
                            .await
                    } else {
                        database
                            .execute_returning(
                                r#"
                                UPDATE type::record('kernel_event_ledger', $event_id)
                                SET payload.binding_state = 'terminated'
                                WHERE owner_account_id = $owner_account_id
                                  AND actor_principal_id = $actor_principal_id
                                  AND authenticated_session_id = $authenticated_session_id
                                  AND access_space_id = $access_space_id
                                  AND workspace_id = $workspace_id
                                RETURN AFTER;
                                "#,
                                bindings,
                            )
                            .await
                    }
                })
            })
            .await
            .expect("apply exact-scope receipt-link counterfactual");
        assert_eq!(affected, 1);
        assert!(matches!(
            registry.resolve_worktree_vm(&worktree_id).await,
            Err(WorktreeVmError::EventLedgerReceiptMissing)
                | Err(WorktreeVmError::EventLedgerReceiptMismatch)
        ));
        assert_eq!(
            adapter
                .status(&handle)
                .await
                .expect("receipt denial must not mutate the live VM"),
            ProcessStatus::Running
        );
        drop(registry);

        let restarted_storage = harness.restart_storage().await;
        let restarted = WorktreeVmRegistry::new_durable(
            adapter.clone(),
            restarted_storage,
            ResourceAccessContext::for_account(scope.clone()),
        );
        assert!(matches!(
            restarted.durable_binding(&worktree_id).await,
            Err(WorktreeVmError::EventLedgerReceiptMissing)
                | Err(WorktreeVmError::EventLedgerReceiptMismatch)
        ));
    }
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 2);
    harness.close().await;
}

#[tokio::test]
async fn duplicate_receipt_identity_is_rejected_without_linkage_mutation() {
    let harness = StoreHarness::create("receipt-duplicate").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-receipt-duplicate");
    let worktree_id = format!("wt-mt023-receipt-duplicate-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter,
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create receipt-linked binding");
    let receipts = receipt_history(&harness.storage, &scope, &worktree_id).await;
    let bindings = tamper_bindings(
        &scope,
        receipts[1].event_id.clone(),
        receipts[0].event_id.clone(),
    );
    let duplicate = harness
        .storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .execute_returning(
                        r#"
                        UPDATE type::record('kernel_event_ledger', $event_id)
                        SET idempotency_key = $replacement_event_id
                        WHERE owner_account_id = $owner_account_id
                          AND actor_principal_id = $actor_principal_id
                          AND authenticated_session_id = $authenticated_session_id
                          AND access_space_id = $access_space_id
                          AND workspace_id = $workspace_id
                        RETURN AFTER;
                        "#,
                        bindings,
                    )
                    .await
            })
        })
        .await;
    assert!(
        duplicate.is_err(),
        "duplicate receipt identity must fail closed"
    );
    assert!(registry
        .durable_binding(&worktree_id)
        .await
        .expect("rejected duplicate preserves canonical linkage")
        .is_some());
    assert_eq!(
        receipt_history(&harness.storage, &scope, &worktree_id)
            .await
            .len(),
        2
    );
    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean duplicate counterfactual binding");
    harness.close().await;
}

#[tokio::test]
async fn incomplete_scope_fails_before_storage_or_spawn() {
    let harness = StoreHarness::create("required-scope").await;
    let adapter = Arc::new(DurableAdapter::default());

    let missing_session = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(WorkspaceScopeRef::new("mt023-missing-session").unwrap());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(missing_session),
    );
    let missing_session_worktree = "wt-missing-session";
    assert!(matches!(
        registry.ensure_worktree_vm(missing_session_worktree).await,
        Err(WorktreeVmError::AuthenticatedSessionScopeRequired)
    ));
    assert_eq!(
        unscoped_event_count(&harness.storage, missing_session_worktree).await,
        0
    );

    let missing_access_space = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_workspace(WorkspaceScopeRef::new("mt023-missing-space").unwrap());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(missing_access_space),
    );
    let missing_space_worktree = "wt-missing-space";
    assert!(matches!(
        registry.ensure_worktree_vm(missing_space_worktree).await,
        Err(WorktreeVmError::AccessSpaceScopeRequired)
    ));
    assert_eq!(
        unscoped_event_count(&harness.storage, missing_space_worktree).await,
        0
    );

    let missing_workspace = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(missing_workspace),
    );
    let missing_workspace_worktree = "wt-missing-workspace";
    assert!(matches!(
        registry
            .ensure_worktree_vm(missing_workspace_worktree)
            .await,
        Err(WorktreeVmError::WorkspaceScopeRequired)
    ));
    assert_eq!(
        unscoped_event_count(&harness.storage, missing_workspace_worktree).await,
        0
    );
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 0);

    harness.close().await;
}

#[tokio::test]
async fn exact_scope_mismatch_fails_before_a_second_spawn() {
    let harness = StoreHarness::create("scope").await;
    let adapter = Arc::new(DurableAdapter::default());
    let owner_scope = exact_scope("mt023-scope");
    let worktree_id = format!("wt-mt023-scope-{}", Uuid::now_v7());
    let owner = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(owner_scope.clone()),
    );
    owner
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create owner binding");
    let original = owner
        .durable_binding(&worktree_id)
        .await
        .expect("read owner binding")
        .expect("owner binding exists");

    let owner_session = owner_scope.authenticated_session.expect("owner session");
    let owner_space = owner_scope.access_space.expect("owner AccessSpace");
    let owner_workspace = owner_scope.workspace.clone().expect("owner workspace");
    let denied_scopes = [
        ResourceScope::new(OwnerAccountId::mint(), owner_scope.actor_principal_id)
            .with_session(owner_session)
            .with_access_space(owner_space)
            .with_workspace(owner_workspace.clone()),
        ResourceScope::new(owner_scope.owner_account_id, ActorPrincipalId::mint())
            .with_session(owner_session)
            .with_access_space(owner_space)
            .with_workspace(owner_workspace.clone()),
        ResourceScope::new(owner_scope.owner_account_id, owner_scope.actor_principal_id)
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(owner_space)
            .with_workspace(owner_workspace.clone()),
        ResourceScope::new(owner_scope.owner_account_id, owner_scope.actor_principal_id)
            .with_session(owner_session)
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(owner_workspace.clone()),
        ResourceScope::new(owner_scope.owner_account_id, owner_scope.actor_principal_id)
            .with_session(owner_session)
            .with_access_space(owner_space)
            .with_workspace(
                WorkspaceScopeRef::new("mt023-other-workspace")
                    .expect("valid counterfactual workspace"),
            ),
    ];
    for denied_scope in denied_scopes {
        let event_count_before = unscoped_event_count(&harness.storage, &worktree_id).await;
        let denied = WorktreeVmRegistry::new_durable(
            adapter.clone(),
            harness.storage.clone(),
            ResourceAccessContext::for_account(denied_scope),
        );
        assert!(matches!(
            denied.ensure_worktree_vm(&worktree_id).await,
            Err(WorktreeVmError::ScopeDenied(
                ScopeDenied::ExactAttributionMismatch
            ))
        ));
        assert_eq!(
            denied
                .durable_binding(&worktree_id)
                .await
                .expect("denied scope read is non-disclosing"),
            None
        );
        assert!(matches!(
            denied.snapshot(&worktree_id).await,
            Err(WorktreeVmError::NotBound { .. })
        ));
        denied
            .teardown_worktree_vm(&worktree_id)
            .await
            .expect("denied teardown is non-disclosing and side-effect free");
        assert_eq!(adapter.snapshot_count.load(Ordering::SeqCst), 0);
        assert_eq!(
            unscoped_event_count(&harness.storage, &worktree_id).await,
            event_count_before,
            "one-field scope mismatch must emit no canonical event"
        );
        assert_eq!(
            adapter
                .status(&original.process_handle)
                .await
                .expect("owner handle remains observable after denied teardown"),
            ProcessStatus::Running
        );
        assert_eq!(
            owner
                .durable_binding(&worktree_id)
                .await
                .expect("read owner binding after denied attempt")
                .expect("owner binding remains present"),
            original
        );
    }
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 1);

    owner
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean owner binding");
    drop(owner);
    harness.close().await;
}

#[tokio::test]
async fn durable_raw_snapshot_restore_fails_before_adapter_restore() {
    let harness = StoreHarness::create("raw-restore").await;
    let adapter = Arc::new(DurableAdapter::default());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(exact_scope("mt023-raw-restore")),
    );
    let worktree_id = format!("wt-mt023-raw-restore-{}", Uuid::now_v7());
    let snapshot = SnapshotRef::new(
        AdapterId::new("cloud_hypervisor"),
        "/mt023-snapshots/unattributed",
    );

    assert!(matches!(
        registry.restore(&worktree_id, &snapshot).await,
        Err(WorktreeVmError::SnapshotSourceMissing)
    ));
    assert_eq!(adapter.restore_count.load(Ordering::SeqCst), 0);
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 0);
    assert_eq!(
        registry
            .durable_binding(&worktree_id)
            .await
            .expect("query exact durable scope after denied restore"),
        None
    );
    assert_eq!(
        unscoped_event_count(&harness.storage, &worktree_id).await,
        0
    );

    drop(registry);
    harness.close().await;
}

#[test]
fn every_durable_surreal_query_block_predicates_all_five_scope_fields() {
    let source = include_str!("../src/swarm_orchestration/worktree_vm_registry.rs");
    let query_anchors = [
        "LET $existing = (SELECT binding_id",
        "worktree VM compensation EventLedger receipt",
        "worktree VM bind EventLedger receipt",
        "worktree VM snapshot EventLedger receipt",
        "worktree VM teardown EventLedger receipt",
        "SELECT event_id, event_sequence\n                            FROM kernel_event_ledger",
        "FROM type::record('worktree_vm_bindings', $record_id)",
        "FROM worktree_vm_bindings",
    ];
    let predicates = [
        "owner_account_id = $owner_account_id",
        "actor_principal_id = $actor_principal_id",
        "authenticated_session_id = $authenticated_session_id",
        "access_space_id = $access_space_id",
        "workspace_id = $workspace_id",
    ];

    for anchor in query_anchors {
        let start = source
            .find(anchor)
            .unwrap_or_else(|| panic!("missing durable query anchor: {anchor}"));
        let tail = &source[start..];
        let end = tail
            .find("\"#,")
            .unwrap_or_else(|| panic!("unterminated durable query block: {anchor}"));
        let query = &tail[..end];
        for predicate in predicates {
            assert!(
                query.contains(predicate),
                "durable query `{anchor}` omits `{predicate}`"
            );
        }
    }

    let reservation = {
        let start = source
            .find("RETURN CREATE $record CONTENT")
            .expect("reservation create block");
        let tail = &source[start..];
        let end = tail.find("};").expect("reservation create terminator");
        &tail[..end]
    };
    for field in [
        "owner_account_id: $owner_account_id",
        "actor_principal_id: $actor_principal_id",
        "authenticated_session_id: $authenticated_session_id",
        "access_space_id: $access_space_id",
        "workspace_id: $workspace_id",
    ] {
        assert!(
            reservation.contains(field),
            "durable reservation omits `{field}`"
        );
    }

    for required in [
        "TYPE record<kernel_event_ledger>",
        "CREATE type::record('kernel_event_ledger', $event_id)",
        "event_ledger_event_id = type::record('kernel_event_ledger', $event_id)",
        "IF array::len($prior) != 0",
        "LIMIT 2",
        "EventLedgerReceiptMissing",
        "EventLedgerReceiptAmbiguous",
        "EventLedgerReceiptMismatch",
        "event_sequence > 0",
    ] {
        assert!(
            source.contains(required),
            "registry omits canonical receipt guard `{required}`"
        );
    }
}

#[test]
fn mt023_owned_sources_have_no_forbidden_database_backend_tokens() {
    let registry = include_str!("../src/swarm_orchestration/worktree_vm_registry.rs");
    let test = include_str!("worktree_vm_registry_surreal_tests.rs");
    let owned_sources = format!("{registry}\n{test}").to_ascii_lowercase();
    for (prefix, suffix) in [
        ("post", "gres"),
        ("sql", "ite"),
        ("sql", "x"),
        ("pg", "pool"),
    ] {
        let forbidden = format!("{prefix}{suffix}");
        assert!(
            !owned_sources.contains(&forbidden),
            "MT-023 owned source contains forbidden backend token `{forbidden}`"
        );
    }
}

#[tokio::test]
async fn concurrent_registries_share_one_durable_spawn() {
    let harness = StoreHarness::create("concurrent").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-concurrent");
    let worktree_id = format!("wt-mt023-concurrent-{}", Uuid::now_v7());
    let first = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    ));
    let second = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    ));
    let (first_result, second_result) = tokio::join!(
        first.ensure_worktree_vm(&worktree_id),
        second.ensure_worktree_vm(&worktree_id)
    );
    assert_eq!(
        first_result.expect("first ensure"),
        second_result.expect("second ensure")
    );
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        receipt_history(&harness.storage, &scope, &worktree_id)
            .await
            .len(),
        2
    );

    first
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean concurrent binding");
    drop(first);
    drop(second);
    harness.close().await;
}

#[tokio::test]
async fn snapshot_and_teardown_are_generation_fenced() {
    let harness = StoreHarness::create("snapshot").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-snapshot");
    let worktree_id = format!("wt-mt023-snapshot-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter,
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope),
    );
    registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create snapshot source binding");
    let snapshot = registry
        .snapshot(&worktree_id)
        .await
        .expect("record snapshot");
    let snapshotted = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read snapshotted binding")
        .expect("snapshotted binding exists");
    assert_eq!(
        snapshotted.binding_state,
        WorktreeVmBindingState::Snapshotted
    );
    assert_eq!(snapshotted.latest_snapshot.as_ref(), Some(&snapshot));

    registry
        .teardown_worktree_vm_if_current(
            &worktree_id,
            &handshake_core::swarm_orchestration::worktree_vm_registry::WorktreeVmBindingIdentity {
                binding_id: snapshotted.binding_id,
                generation: snapshotted.generation,
                process_handle: snapshotted.process_handle,
            },
        )
        .await
        .expect("generation-fenced teardown");
    drop(registry);
    harness.close().await;
}

#[tokio::test]
async fn stale_generation_cannot_teardown_successor() {
    let harness = StoreHarness::create("stale-generation").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-stale-generation");
    let worktree_id = format!("wt-mt023-stale-generation-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );

    registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create first generation");
    let first = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read first generation")
        .expect("first generation exists");
    let stale_identity =
        handshake_core::swarm_orchestration::worktree_vm_registry::WorktreeVmBindingIdentity {
            binding_id: first.binding_id,
            generation: first.generation,
            process_handle: first.process_handle,
        };
    registry
        .teardown_worktree_vm_if_current(&worktree_id, &stale_identity)
        .await
        .expect("terminalize first generation");

    let successor_handle = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create successor generation");
    let successor = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read successor generation")
        .expect("successor exists");
    assert!(successor.generation > stale_identity.generation);
    let receipts_before_stale_retry = receipt_history(&harness.storage, &scope, &worktree_id).await;
    assert!(matches!(
        registry
            .teardown_worktree_vm_if_current(&worktree_id, &stale_identity)
            .await,
        Err(WorktreeVmError::StaleBinding {
            operation: "owned teardown",
            ..
        })
    ));
    assert_eq!(
        adapter.status(&successor_handle).await.unwrap(),
        ProcessStatus::Running
    );
    assert_eq!(
        receipt_history(&harness.storage, &scope, &worktree_id).await,
        receipts_before_stale_retry,
        "stale generation must not append a canonical event"
    );

    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean successor generation");
    harness.close().await;
}

#[tokio::test]
async fn stale_process_identity_cannot_terminate_current_generation() {
    let harness = StoreHarness::create("stale-process").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-stale-process");
    let worktree_id = format!("wt-mt023-stale-process-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    let current_handle = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create current generation");
    let current = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read current generation")
        .expect("current generation exists");
    let stale_process =
        handshake_core::swarm_orchestration::worktree_vm_registry::WorktreeVmBindingIdentity {
            binding_id: current.binding_id,
            generation: current.generation,
            process_handle: ProcessHandle::new(
                AdapterId::new("cloud_hypervisor"),
                None,
                "mt023-forged-stale-process",
            ),
        };
    let receipts_before = receipt_history(&harness.storage, &scope, &worktree_id).await;

    assert!(matches!(
        registry
            .teardown_worktree_vm_if_current(&worktree_id, &stale_process)
            .await,
        Err(WorktreeVmError::StaleBinding {
            operation: "owned teardown",
            ..
        })
    ));
    assert_eq!(adapter.kill_count.load(Ordering::SeqCst), 0);
    assert_eq!(
        adapter.status(&current_handle).await.unwrap(),
        ProcessStatus::Running
    );
    assert_eq!(
        registry
            .durable_binding(&worktree_id)
            .await
            .expect("stale process denial preserves current row")
            .expect("current row remains"),
        current
    );
    assert_eq!(
        receipt_history(&harness.storage, &scope, &worktree_id).await,
        receipts_before
    );

    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean current generation");
    harness.close().await;
}

#[tokio::test]
async fn raw_and_manifest_cross_scope_restore_emit_no_event_or_adapter_side_effect() {
    let harness = StoreHarness::create("cross-scope-restore").await;
    let adapter = Arc::new(DurableAdapter::default());
    let owner_scope = exact_scope("mt023-cross-scope-restore");
    let worktree_id = format!("wt-mt023-cross-scope-restore-{}", Uuid::now_v7());
    let owner = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(owner_scope.clone()),
    );
    owner
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create snapshot source");
    let manifest = owner
        .snapshot_warm_model(
            &worktree_id,
            "mt023-cross-scope-model",
            "/models/mt023-cross-scope.gguf",
            &ready_frame("mt023-cross-scope-model", "/models/mt023-cross-scope.gguf"),
        )
        .await
        .expect("capture exact-scope manifest");
    owner
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("terminalize snapshot source");
    let event_count_before = unscoped_event_count(&harness.storage, &worktree_id).await;
    let foreign_scope = ResourceScope::new(OwnerAccountId::mint(), owner_scope.actor_principal_id)
        .with_session(owner_scope.authenticated_session.expect("owner session"))
        .with_access_space(owner_scope.access_space.expect("owner AccessSpace"))
        .with_workspace(owner_scope.workspace.clone().expect("owner workspace"));
    let denied = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(foreign_scope),
    );

    assert!(matches!(
        denied.restore(&worktree_id, &manifest.snapshot).await,
        Err(WorktreeVmError::SnapshotSourceMissing)
    ));
    assert!(matches!(
        denied
            .restore_warm_model(
                &manifest,
                "mt023-cross-scope-model",
                "/models/mt023-cross-scope.gguf",
            )
            .await,
        Err(WorktreeVmError::SnapshotScopeMismatch {
            dimension: "owner_account_id"
        })
    ));
    assert_eq!(adapter.restore_count.load(Ordering::SeqCst), 0);
    assert_eq!(
        unscoped_event_count(&harness.storage, &worktree_id).await,
        event_count_before
    );
    assert_eq!(
        denied
            .durable_binding(&worktree_id)
            .await
            .expect("cross-scope read is non-disclosing"),
        None
    );

    drop(denied);
    drop(owner);
    harness.close().await;
}

#[tokio::test]
async fn incomplete_legacy_row_denies_without_mutation_event_or_identifier_disclosure() {
    #[derive(Debug, SurrealValue)]
    struct LegacyBindings {
        record_id: String,
        binding_id: String,
        worktree_id: String,
        owner_account_id: String,
        actor_principal_id: String,
        authenticated_session_id: String,
        access_space_id: String,
    }

    #[derive(Debug, SurrealValue)]
    struct LegacyProofRow {
        binding_id: String,
        binding_state: String,
    }

    #[derive(Debug, SurrealValue)]
    struct RecordLookupBindings {
        record_id: String,
    }

    let harness = StoreHarness::create("legacy-incomplete").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-legacy-incomplete");
    let worktree_id = format!("wt-mt023-legacy-incomplete-{}", Uuid::now_v7());
    let exact = receipt_bindings(&scope, &worktree_id);
    let record_id = registry_record_id(&worktree_id);
    let legacy_binding_id = Uuid::now_v7().to_string();
    let bindings = LegacyBindings {
        record_id: record_id.clone(),
        binding_id: legacy_binding_id.clone(),
        worktree_id: worktree_id.clone(),
        owner_account_id: exact.owner_account_id,
        actor_principal_id: exact.actor_principal_id,
        authenticated_session_id: exact.authenticated_session_id,
        access_space_id: exact.access_space_id,
    };
    harness
        .storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<LegacyProofRow, _>(
                        r#"
                        CREATE type::record('worktree_vm_bindings', $record_id) CONTENT {
                            binding_id: $binding_id,
                            worktree_id: $worktree_id,
                            adapter_id: 'cloud_hypervisor',
                            process_handle_json: '{}',
                            latest_snapshot_json: NONE,
                            binding_state: 'active',
                            generation: 1,
                            failure_reason: NONE,
                            reservation_id: NONE,
                            owner_account_id: $owner_account_id,
                            actor_principal_id: $actor_principal_id,
                            authenticated_session_id: $authenticated_session_id,
                            access_space_id: $access_space_id,
                            updated_at_unix_ms: 0
                        } RETURN binding_id, binding_state;
                        "#,
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("seed incomplete legacy row");
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope),
    );

    assert!(matches!(
        registry.ensure_worktree_vm(&worktree_id).await,
        Err(WorktreeVmError::ScopeDenied(
            ScopeDenied::ExactAttributionMismatch
        ))
    ));
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 0);
    assert_eq!(
        unscoped_event_count(&harness.storage, &worktree_id).await,
        0
    );
    assert_eq!(
        registry
            .durable_binding(&worktree_id)
            .await
            .expect("legacy row is non-disclosing"),
        None
    );
    let proof = harness
        .storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_first::<LegacyProofRow, _>(
                        "SELECT binding_id, binding_state FROM type::record('worktree_vm_bindings', $record_id);",
                        RecordLookupBindings { record_id },
                    )
                    .await
            })
        })
        .await
        .expect("read raw legacy row")
        .expect("legacy row remains present");
    assert_eq!(proof.binding_id, legacy_binding_id);
    assert_eq!(proof.binding_state, "active");

    drop(registry);
    harness.close().await;
}

#[tokio::test]
async fn deterministic_failure_between_event_and_state_writes_rolls_back_both() {
    #[derive(Debug, SurrealValue)]
    struct AtomicFailureBindings {
        event_id: String,
        record_id: String,
        worktree_id: String,
        binding_id: String,
        owner_account_id: String,
        actor_principal_id: String,
        authenticated_session_id: String,
        access_space_id: String,
        workspace_id: String,
    }

    let harness = StoreHarness::create("atomic-rollback").await;
    let adapter = Arc::new(DurableAdapter::default());
    let scope = exact_scope("mt023-atomic-rollback");
    let worktree_id = format!("wt-mt023-atomic-rollback-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter,
        harness.storage.clone(),
        ResourceAccessContext::for_account(scope.clone()),
    );
    registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create rollback probe binding");
    let binding_before = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read rollback probe binding")
        .expect("rollback probe binding exists");
    let receipts_before = receipt_history(&harness.storage, &scope, &worktree_id).await;
    let exact = receipt_bindings(&scope, &worktree_id);
    let bindings = AtomicFailureBindings {
        event_id: format!("wvm-atomic-rollback-{}", Uuid::now_v7()),
        record_id: receipts_before[0].record_id.clone(),
        worktree_id: worktree_id.clone(),
        binding_id: binding_before.binding_id.to_string(),
        owner_account_id: exact.owner_account_id,
        actor_principal_id: exact.actor_principal_id,
        authenticated_session_id: exact.authenticated_session_id,
        access_space_id: exact.access_space_id,
        workspace_id: exact.workspace_id,
    };
    let failed = harness
        .storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<ReceiptProofRow, _>(
                        r#"
                        BEGIN TRANSACTION;
                        LET $current = (SELECT binding_id FROM type::record('worktree_vm_bindings', $record_id)
                            WHERE binding_id = $binding_id
                              AND owner_account_id = $owner_account_id
                              AND actor_principal_id = $actor_principal_id
                              AND authenticated_session_id = $authenticated_session_id
                              AND access_space_id = $access_space_id
                              AND workspace_id = $workspace_id LIMIT 1);
                        IF array::len($current) != 1 { THROW 'MT-023 atomic probe scope/fence missing'; };
                        CREATE type::record('kernel_event_ledger', $event_id) CONTENT {
                            event_id: $event_id, event_version: 'kernel_event_v1',
                            kernel_task_run_id: $workspace_id,
                            session_run_id: $authenticated_session_id,
                            aggregate_type: 'worktree_vm_binding', aggregate_id: $record_id,
                            idempotency_key: $event_id, event_type: 'SESSION_FAILED',
                            actor_kind: 'operator', actor_id: $actor_principal_id,
                            causation_id: NONE, correlation_id: $record_id,
                            payload_hash: 'deterministic-rollback-probe',
                            source_component: 'worktree_vm_registry',
                            payload: { transition_event_type: 'SESSION_FAILED',
                                record_id: $record_id, worktree_id: $worktree_id,
                                binding_id: $binding_id, binding_state: 'failed' },
                            owner_account_id: $owner_account_id,
                            actor_principal_id: $actor_principal_id,
                            authenticated_session_id: $authenticated_session_id,
                            access_space_id: $access_space_id,
                            workspace_id: $workspace_id,
                            created_at: time::now()
                        };
                        THROW 'MT-023 deterministic failure after event write';
                        UPDATE type::record('worktree_vm_bindings', $record_id)
                            SET binding_state = 'failed'
                            WHERE binding_id = $binding_id
                              AND owner_account_id = $owner_account_id
                              AND actor_principal_id = $actor_principal_id
                              AND authenticated_session_id = $authenticated_session_id
                              AND access_space_id = $access_space_id
                              AND workspace_id = $workspace_id;
                        COMMIT TRANSACTION;
                        "#,
                        bindings,
                    )
                    .await
            })
        })
        .await;
    assert!(failed.is_err(), "deterministic transaction probe must fail");
    assert_eq!(
        registry
            .durable_binding(&worktree_id)
            .await
            .expect("read binding after rollback")
            .expect("binding survives rollback"),
        binding_before
    );
    assert_eq!(
        receipt_history(&harness.storage, &scope, &worktree_id).await,
        receipts_before,
        "event creation must roll back with the rejected state mutation"
    );

    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean rollback probe binding");
    harness.close().await;
}
