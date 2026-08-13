//! WP-1 MT-023: real-PostgreSQL proof for durable worktree microVM ownership.
//!
//! The adapter is deliberately deterministic here so this test can isolate the
//! PostgreSQL ownership, restart-adoption, scope, and terminal-state contract.
//! AC-1/AC-6 still require a separate real Cloud Hypervisor + KVM coordinator
//! proof; this file does not claim to replace that boundary.

mod knowledge_pg_support;

use std::{
    collections::HashMap,
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
    swarm_orchestration::{
        resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
            ResourceAccessContext, ResourceScope, WorkspaceScopeRef,
        },
        worktree_vm_registry::{
            WorktreeVmBindingIdentity, WorktreeVmBindingState, WorktreeVmError, WorktreeVmRegistry,
        },
    },
};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Barrier;
use uuid::Uuid;

#[derive(Default)]
struct DurableAdapterState {
    statuses: HashMap<Uuid, ProcessStatus>,
    handles: HashMap<Uuid, ProcessHandle>,
    killed_handles: Vec<Uuid>,
}

#[derive(Clone)]
struct BlockingHook {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    armed: Arc<AtomicBool>,
}

impl BlockingHook {
    fn new() -> Self {
        Self {
            entered: Arc::new(Barrier::new(2)),
            release: Arc::new(Barrier::new(2)),
            armed: Arc::new(AtomicBool::new(true)),
        }
    }

    async fn block(&self) {
        if !self.armed.swap(false, Ordering::SeqCst) {
            return;
        }
        self.entered.wait().await;
        self.release.wait().await;
    }
}

#[derive(Default)]
struct DurableAdapter {
    spawn_count: AtomicUsize,
    restore_count: AtomicUsize,
    handle_ordinal: AtomicUsize,
    state: Mutex<DurableAdapterState>,
    spawn_barrier: Option<Arc<Barrier>>,
    restore_hook: Option<BlockingHook>,
    snapshot_hook: Option<BlockingHook>,
    kill_hook: Option<BlockingHook>,
    fail_kill: AtomicBool,
    next_spawn_handle: Mutex<Option<ProcessHandle>>,
}

impl DurableAdapter {
    fn status_for(&self, handle: &ProcessHandle) -> Option<ProcessStatus> {
        self.state
            .lock()
            .expect("adapter state lock")
            .statuses
            .get(&handle.id)
            .cloned()
    }

    fn running_handles(&self) -> Vec<ProcessHandle> {
        let state = self.state.lock().expect("adapter state lock");
        state
            .statuses
            .iter()
            .filter(|(_, status)| matches!(status, ProcessStatus::Running))
            .filter_map(|(id, _)| state.handles.get(id).cloned())
            .collect()
    }

    fn create_running_handle(&self, label: &str) -> ProcessHandle {
        let ordinal = self.handle_ordinal.fetch_add(1, Ordering::SeqCst) + 1;
        let handle = ProcessHandle::new(
            AdapterId::new("cloud_hypervisor"),
            None,
            format!("hsk-pg-{label}-{ordinal}"),
        );
        let mut state = self.state.lock().expect("adapter state lock");
        state.statuses.insert(handle.id, ProcessStatus::Running);
        state.handles.insert(handle.id, handle.clone());
        handle
    }

    fn reuse_on_next_spawn(&self, handle: ProcessHandle) {
        *self
            .next_spawn_handle
            .lock()
            .expect("next spawn handle lock") = Some(handle);
    }

    fn kill_count_for(&self, handle: &ProcessHandle) -> usize {
        self.state
            .lock()
            .expect("adapter state lock")
            .killed_handles
            .iter()
            .filter(|id| **id == handle.id)
            .count()
    }
}

#[async_trait]
impl SandboxAdapter for DurableAdapter {
    async fn spawn(&self, _spec: ProcessSpec) -> Result<ProcessHandle, SandboxAdapterError> {
        self.spawn_count.fetch_add(1, Ordering::SeqCst);
        if let Some(barrier) = &self.spawn_barrier {
            // Before the durable serialization fix, both registries reach this
            // barrier and deterministically expose the double-spawn. After the
            // fix, the first registry must be able to proceed alone while the
            // second waits on PostgreSQL and later adopts its handle.
            let _ =
                tokio::time::timeout(std::time::Duration::from_millis(500), barrier.wait()).await;
        }
        if let Some(handle) = self
            .next_spawn_handle
            .lock()
            .expect("next spawn handle lock")
            .take()
        {
            return Ok(handle);
        }
        Ok(self.create_running_handle("durable"))
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
        if let Some(hook) = &self.kill_hook {
            hook.block().await;
        }
        if self.fail_kill.load(Ordering::SeqCst) {
            return Err(SandboxAdapterError::AdapterUnavailable {
                adapter_id: handle.adapter_id.clone(),
                reason: "injected rollback cleanup failure".to_string(),
            });
        }
        let mut state = self.state.lock().expect("adapter state lock");
        let status =
            state
                .statuses
                .get_mut(&handle.id)
                .ok_or(SandboxAdapterError::ProcessHandleStale {
                    process_id: handle.id,
                })?;
        *status = ProcessStatus::Killed { by_signal: signal };
        state.killed_handles.push(handle.id);
        Ok(())
    }

    async fn status(&self, handle: &ProcessHandle) -> Result<ProcessStatus, SandboxAdapterError> {
        self.status_for(handle)
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
        if let Some(hook) = &self.snapshot_hook {
            hook.block().await;
        }
        match self.status(handle).await? {
            ProcessStatus::Running => Ok(SnapshotRef::new(
                AdapterId::new("cloud_hypervisor"),
                format!("/durable-snapshots/{}", handle.id),
            )),
            status => Err(SandboxAdapterError::SnapshotFailed {
                adapter_id: AdapterId::new("cloud_hypervisor"),
                reason: format!("source handle is not running: {status:?}"),
            }),
        }
    }

    async fn restore(&self, _snapshot: &SnapshotRef) -> Result<ProcessHandle, SandboxAdapterError> {
        self.restore_count.fetch_add(1, Ordering::SeqCst);
        if let Some(hook) = &self.restore_hook {
            hook.block().await;
        }
        Ok(self.create_running_handle("restored"))
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

#[tokio::test]
async fn durable_ensure_uses_one_connection_for_lock_load_and_upsert() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 single-connection proof");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&kpg.schema_url)
        .await
        .expect("connect single-connection worktree VM registry pool");

    let adapter = Arc::new(DurableAdapter::default());
    let worktree_id = format!("wt-mt023-single-connection-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(account_scope("wt-registry-single-connection")),
    );

    let handle = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        registry.ensure_worktree_vm(&worktree_id),
    )
    .await
    .expect("durable ensure must not self-deadlock while its advisory-lock transaction owns the only pool connection")
    .expect("single-connection durable ensure succeeds");
    let persisted_handle: serde_json::Value = sqlx::query_scalar(
        "SELECT process_handle FROM worktree_vm_bindings WHERE worktree_id = $1",
    )
    .bind(&worktree_id)
    .fetch_one(&pool)
    .await
    .expect("read binding committed by the single-connection transaction");
    assert_eq!(persisted_handle, serde_json::to_value(&handle).unwrap());
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 1);

    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean single-connection worktree VM");
}

#[tokio::test]
async fn durable_ensure_rejects_missing_workspace_before_spawn() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 workspace-scope proof");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&kpg.schema_url)
        .await
        .expect("connect missing-workspace worktree VM registry pool");

    let adapter = Arc::new(DurableAdapter::default());
    let incomplete_scope = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint());
    let worktree_id = format!("wt-mt023-missing-workspace-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(incomplete_scope),
    );

    let error = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect_err("a durable worktree VM requires exact workspace attribution");
    assert!(matches!(error, WorktreeVmError::WorkspaceScopeRequired));
    assert_eq!(
        adapter.spawn_count.load(Ordering::SeqCst),
        0,
        "scope rejection must happen before the VM side-effect boundary"
    );
    let row_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worktree_vm_bindings WHERE worktree_id = $1")
            .bind(&worktree_id)
            .fetch_one(&pool)
            .await
            .expect("verify no incomplete durable binding was written");
    assert_eq!(row_count, 0);
}

#[tokio::test]
async fn durable_ensure_rejects_missing_session_and_access_space_before_spawn() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 required-scope proof");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&kpg.schema_url)
        .await
        .expect("connect required-scope worktree VM registry pool");
    let adapter = Arc::new(DurableAdapter::default());

    let missing_session = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new("wt-registry-missing-session").expect("workspace scope"),
        );
    let session_registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(missing_session),
    );
    assert!(matches!(
        session_registry
            .ensure_worktree_vm("wt-mt023-missing-session")
            .await,
        Err(WorktreeVmError::AuthenticatedSessionScopeRequired)
    ));

    let missing_access_space = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new("wt-registry-missing-access-space").expect("workspace scope"),
        );
    let access_space_registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool,
        ResourceAccessContext::for_account(missing_access_space),
    );
    assert!(matches!(
        access_space_registry
            .ensure_worktree_vm("wt-mt023-missing-access-space")
            .await,
        Err(WorktreeVmError::AccessSpaceScopeRequired)
    ));
    assert_eq!(
        adapter.spawn_count.load(Ordering::SeqCst),
        0,
        "all missing-context rejections must precede the VM side-effect boundary"
    );
}

#[tokio::test]
async fn durable_registry_serializes_concurrent_ensure_across_registry_instances() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 concurrency proof");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated concurrent worktree VM registry schema");

    let adapter = Arc::new(DurableAdapter {
        spawn_barrier: Some(Arc::new(Barrier::new(2))),
        ..DurableAdapter::default()
    });
    let scope = account_scope("wt-registry-concurrent");
    let access = ResourceAccessContext::for_account(scope);
    let worktree_id = format!("wt-mt023-concurrent-{}", Uuid::now_v7());
    let first = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        access.clone(),
    ));
    let second = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool,
        access,
    ));

    let first_task = {
        let registry = first.clone();
        let worktree_id = worktree_id.clone();
        tokio::spawn(async move { registry.ensure_worktree_vm(&worktree_id).await })
    };
    let second_task = {
        let registry = second.clone();
        let worktree_id = worktree_id.clone();
        tokio::spawn(async move { registry.ensure_worktree_vm(&worktree_id).await })
    };
    let first_handle = first_task
        .await
        .expect("join first ensure")
        .expect("first ensure succeeds");
    let second_handle = second_task
        .await
        .expect("join second ensure")
        .expect("second ensure succeeds");

    let observed_spawn_count = adapter.spawn_count.load(Ordering::SeqCst);
    let observed_running = adapter.running_handles();
    for handle in &observed_running {
        adapter
            .kill(handle, Signal::Kill)
            .await
            .expect("clean every synthetic VM after observation");
    }

    assert_eq!(
        observed_spawn_count, 1,
        "two process-local registries passed durable load before either persisted; PostgreSQL must serialize load -> spawn -> bind so a last-writer-wins upsert cannot orphan the losing VM"
    );
    assert_eq!(
        first_handle, second_handle,
        "both ensures must adopt one VM"
    );
    assert_eq!(
        observed_running.len(),
        1,
        "one worktree scope must have exactly one live VM"
    );
}

#[tokio::test]
async fn aborted_durable_insert_remains_teardown_recoverable() {
    const INSERT_FENCE: i64 = 23_023_365;

    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 cancellation proof");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated cancelled-insert worktree VM registry schema");

    sqlx::query(
        r#"
        CREATE FUNCTION hsk_mt023_block_worktree_vm_insert()
        RETURNS trigger
        LANGUAGE plpgsql
        AS $$
        BEGIN
            PERFORM pg_advisory_xact_lock(23023365);
            RETURN NEW;
        END
        $$
        "#,
    )
    .execute(&pool)
    .await
    .expect("install deterministic post-spawn insert fence function");
    sqlx::query(
        r#"
        CREATE TRIGGER hsk_mt023_block_worktree_vm_insert
        BEFORE INSERT ON worktree_vm_bindings
        FOR EACH ROW
        EXECUTE FUNCTION hsk_mt023_block_worktree_vm_insert()
        "#,
    )
    .execute(&pool)
    .await
    .expect("install deterministic post-spawn insert fence trigger");

    let mut blocker = pool
        .acquire()
        .await
        .expect("acquire dedicated insert-fence connection");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(INSERT_FENCE)
        .execute(&mut *blocker)
        .await
        .expect("hold insert fence before starting VM ensure");

    let adapter = Arc::new(DurableAdapter::default());
    let worktree_id = format!("wt-mt023-aborted-insert-{}", Uuid::now_v7());
    let registry = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(account_scope("wt-registry-aborted-insert")),
    ));
    let ensure = {
        let registry = registry.clone();
        let worktree_id = worktree_id.clone();
        tokio::spawn(async move { registry.ensure_worktree_vm(&worktree_id).await })
    };

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while adapter.spawn_count.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("VM spawn must cross its side-effect boundary before cancellation");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        !ensure.is_finished(),
        "insert fence must hold ensure after the adapter has spawned the VM"
    );
    let row_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worktree_vm_bindings WHERE worktree_id = $1")
            .bind(&worktree_id)
            .fetch_one(&pool)
            .await
            .expect("read canonical binding before abort");
    assert_eq!(
        row_count, 0,
        "the failure state must be after VM spawn but before durable binding commit"
    );

    ensure.abort();
    let join_error = ensure
        .await
        .expect_err("aborted ensure must report task cancellation");
    assert!(join_error.is_cancelled());
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(INSERT_FENCE)
        .execute(&mut *blocker)
        .await
        .expect("release deterministic insert fence");

    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        registry.teardown_worktree_vm(&worktree_id),
    )
    .await
    .expect("pending post-spawn teardown must not self-deadlock while reacquiring the pending map")
    .expect("pending post-spawn VM must remain teardown-recoverable after future abort");
    assert!(
        adapter.running_handles().is_empty(),
        "aborting between adapter spawn and durable INSERT must not orphan the VM"
    );
}

#[tokio::test]
async fn durable_warm_snapshot_cannot_attach_under_another_account_scope() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 snapshot-scope proof");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated snapshot-scope worktree VM registry schema");

    let adapter = Arc::new(DurableAdapter::default());
    let owner_scope = account_scope("wt-registry-snapshot-owner");
    let other_scope = account_scope("wt-registry-snapshot-other");
    let worktree_id = format!("wt-mt023-snapshot-{}", Uuid::now_v7());
    let owner = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(owner_scope),
    );
    owner
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create owner VM");
    let ready = WarmAgentGuestFrame::Ready {
        protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
        protocol_version: WARM_AGENT_PROTOCOL_VERSION,
        agent_id: "owner-warm-agent".to_string(),
        ready_nonce: "owner-ready-nonce".to_string(),
        loaded_model_sha256: Some("ab".repeat(32)),
        loaded_model_guest_path: Some("/models/model.gguf".to_string()),
    };
    let manifest = owner
        .snapshot_warm_model(&worktree_id, &"ab".repeat(32), "/models/model.gguf", &ready)
        .await
        .expect("capture owner-scoped warm snapshot manifest");
    let other = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool,
        ResourceAccessContext::for_account(other_scope),
    );

    let attach = other
        .restore_warm_model(&manifest, &"ab".repeat(32), "/models/model.gguf")
        .await;
    if attach.is_ok() {
        other
            .teardown_worktree_vm(&worktree_id)
            .await
            .expect("clean leaked cross-account restored VM before failing");
    }
    owner
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean owner VM");

    assert!(
        attach.is_err(),
        "a serialized snapshot manifest is a derived artifact and must carry the owner/actor/session/access-space/workspace scope; another account must not attach it"
    );
}

#[tokio::test]
async fn durable_warm_snapshot_restores_under_the_exact_same_scope() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 same-scope restore proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated same-scope restore registry schema");

    let adapter = Arc::new(DurableAdapter::default());
    let scope = account_scope("wt-registry-same-scope-restore");
    let access = ResourceAccessContext::for_account(scope);
    let worktree_id = format!("wt-mt023-same-scope-restore-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(adapter.clone(), pool.clone(), access.clone());
    let original = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create source VM");
    let model_sha256 = "cd".repeat(32);
    let model_guest_path = "/models/same-scope.gguf";
    let ready = WarmAgentGuestFrame::Ready {
        protocol_id: WARM_AGENT_PROTOCOL_ID.to_string(),
        protocol_version: WARM_AGENT_PROTOCOL_VERSION,
        agent_id: "same-scope-warm-agent".to_string(),
        ready_nonce: "same-scope-ready-nonce".to_string(),
        loaded_model_sha256: Some(model_sha256.clone()),
        loaded_model_guest_path: Some(model_guest_path.to_string()),
    };
    let manifest = registry
        .snapshot_warm_model(&worktree_id, &model_sha256, model_guest_path, &ready)
        .await
        .expect("capture same-scope durable warm snapshot");
    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("terminalize source VM before restore");

    let restarted = WorktreeVmRegistry::new_durable(adapter.clone(), pool, access);
    let restored = restarted
        .restore_warm_model(&manifest, &model_sha256, model_guest_path)
        .await
        .expect("the exact same durable scope must authorize and restore its snapshot");
    assert_ne!(restored, original);
    let rebound = restarted
        .durable_binding(&worktree_id)
        .await
        .expect("read rebound durable binding")
        .expect("rebound binding exists");
    assert_eq!(rebound.process_handle, restored);
    assert_eq!(rebound.binding_state, WorktreeVmBindingState::Snapshotted);

    restarted
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean restored same-scope VM");
}

#[tokio::test]
async fn durable_restore_serializes_different_snapshots_across_registries() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 restore serialization proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&kpg.schema_url)
        .await
        .expect("connect restore serialization registry schema");
    let hook = BlockingHook::new();
    let adapter = Arc::new(DurableAdapter {
        restore_hook: Some(hook.clone()),
        ..DurableAdapter::default()
    });
    let access = ResourceAccessContext::for_account(account_scope("wt-registry-restore-race"));
    let worktree_id = format!("wt-mt023-restore-race-{}", Uuid::now_v7());
    let first = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        access.clone(),
    ));
    let second = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool,
        access,
    ));
    let snapshot_a = SnapshotRef::new(
        AdapterId::new("cloud_hypervisor"),
        "/durable-snapshots/restore-race-a",
    );
    let snapshot_b = SnapshotRef::new(
        AdapterId::new("cloud_hypervisor"),
        "/durable-snapshots/restore-race-b",
    );

    let first_task = {
        let registry = first.clone();
        let worktree_id = worktree_id.clone();
        tokio::spawn(async move { registry.restore(&worktree_id, &snapshot_a).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), hook.entered.wait())
        .await
        .expect("first restore reaches the adapter while holding durable serialization");
    let second_task = {
        let registry = second.clone();
        let worktree_id = worktree_id.clone();
        tokio::spawn(async move { registry.restore(&worktree_id, &snapshot_b).await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(adapter.restore_count.load(Ordering::SeqCst), 1);
    assert!(
        !second_task.is_finished(),
        "the second registry must wait on PostgreSQL before crossing the restore side-effect boundary"
    );
    hook.release.wait().await;

    let restored = first_task
        .await
        .expect("join first restore")
        .expect("first restore succeeds");
    assert!(matches!(
        second_task.await.expect("join second restore"),
        Err(WorktreeVmError::AlreadyBound { .. })
    ));
    assert_eq!(adapter.restore_count.load(Ordering::SeqCst), 1);
    assert_eq!(adapter.running_handles(), vec![restored.clone()]);
    first
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean serialized restored VM");
}

#[tokio::test]
async fn stale_teardown_cannot_kill_or_terminalize_a_successor_generation() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 teardown CAS proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&kpg.schema_url)
        .await
        .expect("connect stale teardown registry schema");
    let hook = BlockingHook::new();
    let adapter = Arc::new(DurableAdapter {
        kill_hook: Some(hook.clone()),
        ..DurableAdapter::default()
    });
    let access = ResourceAccessContext::for_account(account_scope("wt-registry-stale-teardown"));
    let worktree_id = format!("wt-mt023-stale-teardown-{}", Uuid::now_v7());
    let registry = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        access.clone(),
    ));
    let original = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create original binding");
    let original_binding = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read original binding")
        .expect("original binding exists");

    let teardown = {
        let registry = registry.clone();
        let worktree_id = worktree_id.clone();
        tokio::spawn(async move { registry.teardown_worktree_vm(&worktree_id).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), hook.entered.wait())
        .await
        .expect("teardown reaches kill after capturing the original generation");
    let successor = adapter.create_running_handle("teardown-successor");
    let successor_binding_id = Uuid::now_v7();
    sqlx::query(
        "UPDATE worktree_vm_bindings SET binding_id = $2, process_handle = $3, generation = $4, binding_state = 'active', updated_at = NOW() WHERE worktree_id = $1",
    )
    .bind(&worktree_id)
    .bind(successor_binding_id)
    .bind(serde_json::to_value(&successor).unwrap())
    .bind(original_binding.generation + 1)
    .execute(&pool)
    .await
    .expect("install successor while stale teardown is paused outside PostgreSQL");
    hook.release.wait().await;

    assert!(matches!(
        teardown.await.expect("join stale teardown"),
        Err(WorktreeVmError::StaleBinding {
            operation: "teardown",
            ..
        })
    ));
    let row: (Uuid, serde_json::Value, String, i64) = sqlx::query_as(
        "SELECT binding_id, process_handle, binding_state, generation FROM worktree_vm_bindings WHERE worktree_id = $1",
    )
    .bind(&worktree_id)
    .fetch_one(&pool)
    .await
    .expect("read successor after stale teardown");
    assert_eq!(row.0, successor_binding_id);
    assert_eq!(row.1, serde_json::to_value(&successor).unwrap());
    assert_eq!(row.2, "active");
    assert_eq!(row.3, original_binding.generation + 1);
    assert!(matches!(
        adapter.status_for(&original),
        Some(ProcessStatus::Killed { .. })
    ));
    assert_eq!(adapter.status_for(&successor), Some(ProcessStatus::Running));

    let cleanup = WorktreeVmRegistry::new_durable(adapter, pool, access);
    cleanup
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean successor after stale teardown proof");
}

#[tokio::test]
async fn fenced_teardown_reclaims_exact_local_handle_when_durable_row_is_absent() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 absent-row cleanup proof");
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&kpg.schema_url)
        .await
        .expect("connect absent-row cleanup registry schema");
    let adapter = Arc::new(DurableAdapter::default());
    let access = ResourceAccessContext::for_account(account_scope("wt-registry-absent-row"));
    let worktree_id = format!("wt-mt023-absent-row-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(adapter.clone(), pool.clone(), access);
    let handle = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create local handle with durable binding");
    let binding = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read durable binding identity")
        .expect("durable binding exists");
    let identity = WorktreeVmBindingIdentity {
        binding_id: binding.binding_id,
        generation: binding.generation,
        process_handle: handle.clone(),
    };
    sqlx::query("DELETE FROM worktree_vm_bindings WHERE binding_id = $1")
        .bind(binding.binding_id)
        .execute(&pool)
        .await
        .expect("remove canonical row to model committed cleanup with retained local ownership");

    registry
        .teardown_worktree_vm_if_current(&worktree_id, &identity)
        .await
        .expect(
            "exact retained handle must be reclaimed before the absent row is treated idempotently",
        );
    assert!(matches!(
        adapter.status_for(&handle),
        Some(ProcessStatus::Killed {
            by_signal: Signal::Term
        })
    ));
    assert!(adapter.running_handles().is_empty());
    registry
        .teardown_worktree_vm_if_current(&worktree_id, &identity)
        .await
        .expect("a second fenced cleanup is idempotent after the exact handle is gone");
}

#[tokio::test]
async fn absent_row_teardown_validates_both_local_and_pending_before_side_effects() {
    const INSERT_FENCE: i64 = 23_023_366;

    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 absent-row candidate proof");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&kpg.schema_url)
        .await
        .expect("connect absent-row candidate registry schema");
    let adapter = Arc::new(DurableAdapter::default());
    let registry = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(account_scope("wt-registry-absent-candidates")),
    ));
    let exact_worktree = format!("wt-mt023-absent-exact-{}", Uuid::now_v7());
    let mixed_worktree = format!("wt-mt023-absent-mixed-{}", Uuid::now_v7());
    let exact_local = registry
        .ensure_worktree_vm(&exact_worktree)
        .await
        .expect("create exact local candidate");
    let mixed_local = registry
        .ensure_worktree_vm(&mixed_worktree)
        .await
        .expect("create mixed local candidate");
    let exact_binding = registry
        .durable_binding(&exact_worktree)
        .await
        .expect("read exact binding")
        .expect("exact binding exists");
    let mixed_binding = registry
        .durable_binding(&mixed_worktree)
        .await
        .expect("read mixed binding")
        .expect("mixed binding exists");
    let exact_identity = WorktreeVmBindingIdentity {
        binding_id: exact_binding.binding_id,
        generation: exact_binding.generation,
        process_handle: exact_local.clone(),
    };
    let mixed_identity = WorktreeVmBindingIdentity {
        binding_id: mixed_binding.binding_id,
        generation: mixed_binding.generation,
        process_handle: mixed_local.clone(),
    };
    sqlx::query("DELETE FROM worktree_vm_bindings WHERE binding_id = ANY($1)")
        .bind(vec![exact_binding.binding_id, mixed_binding.binding_id])
        .execute(&pool)
        .await
        .expect("remove canonical rows before replacement cancellation");

    sqlx::query(
        r#"
        CREATE FUNCTION hsk_mt023_block_absent_candidate_insert()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            PERFORM pg_advisory_xact_lock(23023366);
            RETURN NEW;
        END
        $$
        "#,
    )
    .execute(&pool)
    .await
    .expect("install absent-candidate insert fence function");
    sqlx::query(
        r#"
        CREATE TRIGGER hsk_mt023_block_absent_candidate_insert
        BEFORE INSERT ON worktree_vm_bindings
        FOR EACH ROW EXECUTE FUNCTION hsk_mt023_block_absent_candidate_insert()
        "#,
    )
    .execute(&pool)
    .await
    .expect("install absent-candidate insert fence trigger");

    // Reusing the exact same adapter handle models both maps retaining the same
    // ownership identity. Teardown must kill it exactly once and clear both.
    adapter.reuse_on_next_spawn(exact_local.clone());
    let mut blocker = pool.acquire().await.expect("acquire exact insert blocker");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(INSERT_FENCE)
        .execute(&mut *blocker)
        .await
        .expect("hold exact insert fence");
    let before_exact_spawn = adapter.spawn_count.load(Ordering::SeqCst);
    let exact_ensure = {
        let registry = registry.clone();
        let worktree_id = exact_worktree.clone();
        tokio::spawn(async move { registry.ensure_worktree_vm(&worktree_id).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while adapter.spawn_count.load(Ordering::SeqCst) == before_exact_spawn {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("exact replacement crosses spawn boundary");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        !exact_ensure.is_finished(),
        "exact replacement must remain blocked after installing pending ownership"
    );
    exact_ensure.abort();
    assert!(exact_ensure
        .await
        .expect_err("exact replacement is cancelled")
        .is_cancelled());
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(INSERT_FENCE)
        .execute(&mut *blocker)
        .await
        .expect("release exact insert fence");
    drop(blocker);

    registry
        .teardown_worktree_vm_if_current(&exact_worktree, &exact_identity)
        .await
        .expect("equal local+pending ownership is reclaimed once");
    assert_eq!(adapter.kill_count_for(&exact_local), 1);
    registry
        .teardown_worktree_vm_if_current(&exact_worktree, &exact_identity)
        .await
        .expect("equal-candidate cleanup retry is idempotent");
    assert_eq!(adapter.kill_count_for(&exact_local), 1);

    // A different pending replacement is an ABA successor. Both fenced and
    // unfenced teardown must fail before killing either candidate.
    let mut blocker = pool.acquire().await.expect("acquire mixed insert blocker");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(INSERT_FENCE)
        .execute(&mut *blocker)
        .await
        .expect("hold mixed insert fence");
    let before_mixed_spawn = adapter.spawn_count.load(Ordering::SeqCst);
    let mixed_ensure = {
        let registry = registry.clone();
        let worktree_id = mixed_worktree.clone();
        tokio::spawn(async move { registry.ensure_worktree_vm(&worktree_id).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while adapter.spawn_count.load(Ordering::SeqCst) == before_mixed_spawn {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("mixed replacement crosses spawn boundary");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        !mixed_ensure.is_finished(),
        "mixed replacement must remain blocked after installing pending ownership"
    );
    mixed_ensure.abort();
    assert!(mixed_ensure
        .await
        .expect_err("mixed replacement is cancelled")
        .is_cancelled());
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(INSERT_FENCE)
        .execute(&mut *blocker)
        .await
        .expect("release mixed insert fence");
    drop(blocker);
    let mixed_pending = adapter
        .running_handles()
        .into_iter()
        .find(|handle| handle != &mixed_local)
        .expect("different pending replacement remains running");

    assert!(matches!(
        registry
            .teardown_worktree_vm_if_current(&mixed_worktree, &mixed_identity)
            .await,
        Err(WorktreeVmError::StaleBinding {
            operation: "owned teardown",
            ..
        })
    ));
    assert!(matches!(
        registry.teardown_worktree_vm(&mixed_worktree).await,
        Err(WorktreeVmError::StaleBinding {
            operation: "absent-row teardown",
            ..
        })
    ));
    assert_eq!(
        adapter.status_for(&mixed_local),
        Some(ProcessStatus::Running)
    );
    assert_eq!(
        adapter.status_for(&mixed_pending),
        Some(ProcessStatus::Running)
    );
    assert_eq!(adapter.kill_count_for(&mixed_local), 0);
    assert_eq!(adapter.kill_count_for(&mixed_pending), 0);

    adapter
        .kill(&mixed_local, Signal::Kill)
        .await
        .expect("clean owned mixed local fixture");
    adapter
        .kill(&mixed_pending, Signal::Kill)
        .await
        .expect("clean owned mixed pending fixture");
}

#[tokio::test]
async fn stale_snapshot_cannot_stamp_a_successor_generation() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 snapshot CAS proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&kpg.schema_url)
        .await
        .expect("connect stale snapshot registry schema");
    let hook = BlockingHook::new();
    let adapter = Arc::new(DurableAdapter {
        snapshot_hook: Some(hook.clone()),
        ..DurableAdapter::default()
    });
    let access = ResourceAccessContext::for_account(account_scope("wt-registry-stale-snapshot"));
    let worktree_id = format!("wt-mt023-stale-snapshot-{}", Uuid::now_v7());
    let registry = Arc::new(WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        access.clone(),
    ));
    let original = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create original snapshot binding");
    let original_binding = registry
        .durable_binding(&worktree_id)
        .await
        .expect("read original snapshot binding")
        .expect("original snapshot binding exists");
    let snapshot_task = {
        let registry = registry.clone();
        let worktree_id = worktree_id.clone();
        tokio::spawn(async move { registry.snapshot(&worktree_id).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), hook.entered.wait())
        .await
        .expect("snapshot captures original generation before adapter side effect");
    let successor = adapter.create_running_handle("snapshot-successor");
    let successor_binding_id = Uuid::now_v7();
    sqlx::query(
        "UPDATE worktree_vm_bindings SET binding_id = $2, process_handle = $3, latest_snapshot = NULL, generation = $4, binding_state = 'active', updated_at = NOW() WHERE worktree_id = $1",
    )
    .bind(&worktree_id)
    .bind(successor_binding_id)
    .bind(serde_json::to_value(&successor).unwrap())
    .bind(original_binding.generation + 1)
    .execute(&pool)
    .await
    .expect("install successor while stale snapshot is paused");
    hook.release.wait().await;

    assert!(matches!(
        snapshot_task.await.expect("join stale snapshot"),
        Err(WorktreeVmError::StaleBinding {
            operation: "snapshot",
            ..
        })
    ));
    let row: (Uuid, serde_json::Value, Option<serde_json::Value>, String, i64) =
        sqlx::query_as(
            "SELECT binding_id, process_handle, latest_snapshot, binding_state, generation FROM worktree_vm_bindings WHERE worktree_id = $1",
        )
        .bind(&worktree_id)
        .fetch_one(&pool)
        .await
        .expect("read successor after stale snapshot");
    assert_eq!(row.0, successor_binding_id);
    assert_eq!(row.1, serde_json::to_value(&successor).unwrap());
    assert_eq!(row.2, None);
    assert_eq!(row.3, "active");
    assert_eq!(row.4, original_binding.generation + 1);

    let cleanup = WorktreeVmRegistry::new_durable(adapter.clone(), pool, access);
    cleanup
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean successor after stale snapshot proof");
    adapter
        .kill(&original, Signal::Kill)
        .await
        .expect("clean original VM left intentionally live by the stale snapshot probe");
}

#[tokio::test]
async fn failed_restore_cleanup_preserves_combined_error_and_pending_discoverability() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 compensation proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&kpg.schema_url)
        .await
        .expect("connect restore compensation registry schema");
    sqlx::query(
        r#"
        CREATE FUNCTION hsk_mt023_reject_restore_binding()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'injected durable restore persistence failure';
        END
        $$
        "#,
    )
    .execute(&pool)
    .await
    .expect("install restore persistence failure function");
    sqlx::query(
        r#"
        CREATE TRIGGER hsk_mt023_reject_restore_binding
        BEFORE INSERT ON worktree_vm_bindings
        FOR EACH ROW EXECUTE FUNCTION hsk_mt023_reject_restore_binding()
        "#,
    )
    .execute(&pool)
    .await
    .expect("install restore persistence failure trigger");

    let adapter = Arc::new(DurableAdapter::default());
    adapter.fail_kill.store(true, Ordering::SeqCst);
    let access =
        ResourceAccessContext::for_account(account_scope("wt-registry-restore-compensation"));
    let worktree_id = format!("wt-mt023-restore-compensation-{}", Uuid::now_v7());
    let registry = WorktreeVmRegistry::new_durable(adapter.clone(), pool.clone(), access);
    let snapshot = SnapshotRef::new(
        AdapterId::new("cloud_hypervisor"),
        "/durable-snapshots/restore-compensation",
    );
    let error = registry
        .restore(&worktree_id, &snapshot)
        .await
        .expect_err("durable write and rollback cleanup are both injected to fail");
    assert!(matches!(
        error,
        WorktreeVmError::CompensationFailed {
            operation: "restore persistence",
            ..
        }
    ));
    assert_eq!(adapter.running_handles().len(), 1);
    let rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worktree_vm_bindings WHERE worktree_id = $1")
            .bind(&worktree_id)
            .fetch_one(&pool)
            .await
            .expect("verify failed durable transaction left no binding row");
    assert_eq!(rows, 0);

    adapter.fail_kill.store(false, Ordering::SeqCst);
    registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("pending restored handle remains discoverable for a later cleanup retry");
    assert!(adapter.running_handles().is_empty());
}

#[tokio::test]
async fn same_account_mismatched_principal_session_access_space_and_workspace_fail_closed() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 exact-scope negative proof");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect exact-scope negative registry schema");
    let adapter = Arc::new(DurableAdapter::default());
    let owner_scope = account_scope("wt-registry-exact-scope");
    let worktree_id = format!("wt-mt023-exact-scope-{}", Uuid::now_v7());
    let owner = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(owner_scope.clone()),
    );
    owner
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("create exact-scope owner binding");
    let canonical_before: (serde_json::Value, Uuid, Uuid, Uuid, i64) = sqlx::query_as(
        "SELECT process_handle, actor_principal_id, authenticated_session_id, access_space_id, generation FROM worktree_vm_bindings WHERE worktree_id = $1",
    )
    .bind(&worktree_id)
    .fetch_one(&pool)
    .await
    .expect("read canonical owner binding before collision probes");
    let foreign_snapshot = SnapshotRef::new(
        AdapterId::new("cloud_hypervisor"),
        "/durable-snapshots/foreign-exact-scope",
    );

    let mismatched_actor =
        ResourceScope::new(owner_scope.owner_account_id, ActorPrincipalId::mint())
            .with_session(owner_scope.authenticated_session.unwrap())
            .with_access_space(owner_scope.access_space.unwrap())
            .with_workspace(owner_scope.workspace.clone().unwrap());
    let actor_reader = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(mismatched_actor),
    );
    assert!(actor_reader
        .durable_binding(&worktree_id)
        .await
        .expect("exact actor-mismatched read must not disclose row metadata")
        .is_none());
    assert!(matches!(
        actor_reader.ensure_worktree_vm(&worktree_id).await,
        Err(WorktreeVmError::BindingScopeMismatch {
            dimension: "actor_principal_id"
        })
    ));
    assert!(matches!(
        actor_reader.restore(&worktree_id, &foreign_snapshot).await,
        Err(WorktreeVmError::BindingScopeMismatch {
            dimension: "actor_principal_id"
        })
    ));

    let mismatched_session =
        ResourceScope::new(owner_scope.owner_account_id, owner_scope.actor_principal_id)
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(owner_scope.access_space.unwrap())
            .with_workspace(owner_scope.workspace.clone().unwrap());
    let session_reader = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(mismatched_session),
    );
    assert!(session_reader
        .durable_binding(&worktree_id)
        .await
        .expect("exact session-mismatched read must not disclose row metadata")
        .is_none());
    assert!(matches!(
        session_reader.ensure_worktree_vm(&worktree_id).await,
        Err(WorktreeVmError::BindingScopeMismatch {
            dimension: "authenticated_session_id"
        })
    ));
    assert!(matches!(
        session_reader
            .restore(&worktree_id, &foreign_snapshot)
            .await,
        Err(WorktreeVmError::BindingScopeMismatch {
            dimension: "authenticated_session_id"
        })
    ));

    let mismatched_access_space =
        ResourceScope::new(owner_scope.owner_account_id, owner_scope.actor_principal_id)
            .with_session(owner_scope.authenticated_session.unwrap())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(owner_scope.workspace.clone().unwrap());
    let access_space_reader = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(mismatched_access_space),
    );
    assert!(access_space_reader
        .durable_binding(&worktree_id)
        .await
        .expect("exact AccessSpace-mismatched read must not disclose row metadata")
        .is_none());
    assert!(matches!(
        access_space_reader.ensure_worktree_vm(&worktree_id).await,
        Err(WorktreeVmError::BindingScopeMismatch {
            dimension: "access_space_id"
        })
    ));
    assert!(matches!(
        access_space_reader
            .restore(&worktree_id, &foreign_snapshot)
            .await,
        Err(WorktreeVmError::BindingScopeMismatch {
            dimension: "access_space_id"
        })
    ));
    assert_eq!(
        adapter.spawn_count.load(Ordering::SeqCst),
        1,
        "exact-scope collisions must be rejected before a second VM spawn"
    );
    assert_eq!(
        adapter.restore_count.load(Ordering::SeqCst),
        0,
        "exact-scope collisions must be rejected before a VM restore side effect"
    );
    let canonical_after: (serde_json::Value, Uuid, Uuid, Uuid, i64) = sqlx::query_as(
        "SELECT process_handle, actor_principal_id, authenticated_session_id, access_space_id, generation FROM worktree_vm_bindings WHERE worktree_id = $1",
    )
    .bind(&worktree_id)
    .fetch_one(&pool)
    .await
    .expect("read canonical owner binding after collision probes");
    assert_eq!(
        canonical_after, canonical_before,
        "rejected collisions must not overwrite the handle, exact scope, or generation"
    );
    assert_eq!(
        adapter.running_handles().len(),
        1,
        "rejected collision attempts must not orphan a second live VM"
    );

    let mismatched_workspace =
        ResourceScope::new(owner_scope.owner_account_id, owner_scope.actor_principal_id)
            .with_session(owner_scope.authenticated_session.unwrap())
            .with_access_space(owner_scope.access_space.unwrap())
            .with_workspace(
                WorkspaceScopeRef::new("wt-registry-other-workspace").expect("workspace scope"),
            );
    let workspace_reader = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool,
        ResourceAccessContext::for_account(mismatched_workspace),
    );
    assert!(workspace_reader
        .durable_binding(&worktree_id)
        .await
        .expect("workspace SQL predicate must fail closed without exposing row metadata")
        .is_none());

    owner
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean exact-scope negative proof VM");
}

fn account_scope(workspace: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(WorkspaceScopeRef::new(workspace).expect("non-empty workspace scope"))
}

#[tokio::test]
async fn durable_registry_survives_restart_enforces_scope_and_terminalizes_binding() {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real PostgreSQL is required for the MT-023 durable registry proof");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated worktree VM registry schema");

    let adapter = Arc::new(DurableAdapter::default());
    let owner_scope = account_scope("wt-registry-owner");
    let other_scope = account_scope("wt-registry-other");
    let worktree_id = format!("wt-mt023-{}", Uuid::now_v7());

    let registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(owner_scope.clone()),
    );
    let first_handle = registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("spawn and persist first worktree VM");
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 1);

    let canonical_row: (
        serde_json::Value,
        String,
        i64,
        Uuid,
        Uuid,
        Uuid,
        Uuid,
        String,
    ) = sqlx::query_as(
        "SELECT process_handle, binding_state, generation, owner_account_id, \
         actor_principal_id, authenticated_session_id, access_space_id, workspace_id \
         FROM worktree_vm_bindings WHERE worktree_id = $1",
    )
    .bind(&worktree_id)
    .fetch_one(&pool)
    .await
    .expect("fresh canonical binding row");
    assert_eq!(
        canonical_row.0,
        serde_json::to_value(&first_handle).unwrap()
    );
    assert_eq!(canonical_row.1, "active");
    assert_eq!(canonical_row.2, 1);
    assert_eq!(canonical_row.3, owner_scope.owner_account_id.as_uuid());
    assert_eq!(canonical_row.4, owner_scope.actor_principal_id.as_uuid());
    assert_eq!(
        canonical_row.5,
        owner_scope.authenticated_session.unwrap().as_uuid()
    );
    assert_eq!(canonical_row.6, owner_scope.access_space.unwrap().as_uuid());
    assert_eq!(
        canonical_row.7,
        owner_scope.workspace.as_ref().unwrap().as_str()
    );

    drop(registry);
    let restarted = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(owner_scope.clone()),
    );
    let adopted = restarted
        .resolve_worktree_vm(&worktree_id)
        .await
        .expect("fresh registry must adopt the exact running durable handle");
    assert_eq!(adopted, first_handle);
    assert_eq!(
        adapter.spawn_count.load(Ordering::SeqCst),
        1,
        "component restart must not spawn a replacement VM"
    );

    let other_account = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(other_scope),
    );
    assert!(
        other_account
            .durable_binding(&worktree_id)
            .await
            .expect("scoped durable read")
            .is_none(),
        "another account must not discover the owning account's binding"
    );
    assert!(matches!(
        other_account.resolve_worktree_vm(&worktree_id).await,
        Err(WorktreeVmError::NotBound { .. })
    ));
    other_account
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("cross-account teardown must be an idempotent no-op");
    assert_eq!(
        adapter.status_for(&first_handle),
        Some(ProcessStatus::Running)
    );

    let snapshot = restarted
        .snapshot(&worktree_id)
        .await
        .expect("snapshot durable worktree VM");
    let snapshotted = restarted
        .durable_binding(&worktree_id)
        .await
        .expect("read snapshotted binding")
        .expect("owner binding exists");
    assert_eq!(
        snapshotted.binding_state,
        WorktreeVmBindingState::Snapshotted
    );
    assert_eq!(snapshotted.latest_snapshot.as_ref(), Some(&snapshot));

    restarted
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("terminalize durable worktree VM");
    assert!(matches!(
        adapter.status_for(&first_handle),
        Some(ProcessStatus::Killed {
            by_signal: Signal::Term
        })
    ));
    let terminal = restarted
        .durable_binding(&worktree_id)
        .await
        .expect("read terminal binding")
        .expect("terminal row is retained for recovery evidence");
    assert_eq!(terminal.binding_state, WorktreeVmBindingState::Terminated);

    drop(restarted);
    let rebound_registry = WorktreeVmRegistry::new_durable(
        adapter.clone(),
        pool.clone(),
        ResourceAccessContext::for_account(owner_scope),
    );
    assert!(matches!(
        rebound_registry.resolve_worktree_vm(&worktree_id).await,
        Err(WorktreeVmError::NotBound { .. })
    ));
    let rebound_handle = rebound_registry
        .ensure_worktree_vm(&worktree_id)
        .await
        .expect("a terminal binding may be explicitly rebound");
    assert_ne!(rebound_handle, first_handle);
    assert_eq!(adapter.spawn_count.load(Ordering::SeqCst), 2);
    let rebound = rebound_registry
        .durable_binding(&worktree_id)
        .await
        .expect("read rebound durable row")
        .expect("rebound row exists");
    assert_eq!(rebound.binding_state, WorktreeVmBindingState::Active);
    assert_eq!(rebound.generation, 2);

    rebound_registry
        .teardown_worktree_vm(&worktree_id)
        .await
        .expect("clean up rebound worktree VM");
}
