//! WP-1 MT-023: real worktree-scoped model-lane proof.
//!
//! This test is intentionally ignored by default because it boots the real
//! Cloud Hypervisor/KVM backend, loads a real GGUF through the packaged guest
//! warm agent, and writes to real PostgreSQL. It must be invoked explicitly
//! with the live environment named in the test's failure messages.

mod knowledge_pg_support;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use futures::StreamExt;
use handshake_core::{
    model_runtime::{
        registry::RuntimeBinding as RuntimeAdapterBinding, CancellationToken, GenPrompt,
        GenerateRequest, ModelId, SamplingParams,
    },
    process_ledger::{
        drain_and_join_ledger_writer, LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome,
        NoopOverflowSink, PostgresProcessLedgerStore,
    },
    sandbox::{
        AdapterId, CloudHypervisorAdapter, CloudHypervisorConfig, Command, DetachedProcessIdentity,
        IsolationTier, ProcessStatus, SandboxAdapter, SandboxAdapterRegistry, Signal,
        CLOUD_HYPERVISOR_ADAPTER_ID,
    },
    swarm_orchestration::{
        build_production_swarm_coordinator_with_sandbox_registry,
        model_lane::{
            DexterityLaunchContract, ModelLaneStore, RuntimeBinding as ModelLaneRuntimeBinding,
        },
        resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
            ResourceScope, WorkspaceScopeRef,
        },
        CloudLaneFactoryConfig, ModelInstanceId, SpawnRequest, WorktreeVmBindingState,
        WorktreeVmError, WorktreeVmRegistry,
    },
};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, Row};
use uuid::Uuid;

const MT023_LIVE_ENV: &str = "HANDSHAKE_MT023_LIVE";
const MT023_WORKTREE_ROOT_ENV: &str = "HANDSHAKE_MT023_WORKTREE_ROOT";
const MT023_GGUF_ENV: &str = "HANDSHAKE_SBX_GGUF";
const MT023_GGUF_SHA_ENV: &str = "HANDSHAKE_SBX_GGUF_SHA256";
const MT023_WARM_AGENT_ENV: &str = "HANDSHAKE_CH_WARM_AGENT_HOST_PATH";
const MT023_LLAMA_SERVER_ENV: &str = "HANDSHAKE_SANDBOX_LLAMA_CLI_HOST_PATH";

fn required_path(name: &str) -> PathBuf {
    let raw =
        std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set for MT-023 live proof"));
    let path = PathBuf::from(raw);
    assert!(
        path.exists(),
        "{name} path does not exist: {}",
        path.display()
    );
    path
}

fn sha256_file_hex(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|error| panic!("read {} for SHA-256: {error}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

fn generation_request() -> GenerateRequest {
    GenerateRequest {
        id: ModelId::new_v7(),
        prompt: GenPrompt::new("MT-023 worktree-scoped microVM proof: produce one short sentence."),
        sampling: SamplingParams::default(),
        lora_overrides: vec![],
        steering_overrides: vec![],
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 24,
        stop_sequences: vec![],
        speculative_mode: None,
        structured_decoding: None,
    }
}

#[tokio::test]
#[ignore = "requires real PostgreSQL, WSL2 /dev/kvm, Cloud Hypervisor, packaged hsk-warm-agent + llama-server, and a real GGUF"]
async fn mt023_real_worktree_model_lane_runs_inside_microvm() {
    assert_eq!(
        std::env::var(MT023_LIVE_ENV).ok().as_deref(),
        Some("1"),
        "{MT023_LIVE_ENV}=1 is required; the live proof must never pass by skipping"
    );

    let worktree_root = required_path(MT023_WORKTREE_ROOT_ENV)
        .canonicalize()
        .expect("canonicalize MT-023 worktree root");
    assert!(
        worktree_root.join(".git").is_file(),
        "MT-023 worktree root must be a linked Git worktree: {}",
        worktree_root.display()
    );
    let gguf_path = required_path(MT023_GGUF_ENV);
    let warm_agent_path = required_path(MT023_WARM_AGENT_ENV);
    let llama_server_path = required_path(MT023_LLAMA_SERVER_ENV);
    assert_eq!(
        warm_agent_path.parent(),
        llama_server_path.parent(),
        "hsk-warm-agent and llama-server must come from the same validated package"
    );
    let gguf_sha =
        std::env::var(MT023_GGUF_SHA_ENV).unwrap_or_else(|_| sha256_file_hex(&gguf_path));
    assert_eq!(
        sha256_file_hex(&gguf_path),
        gguf_sha,
        "the declared MT-023 GGUF hash must match the real artifact"
    );

    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("MT-023 live proof requires real PostgreSQL");
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&pg.schema_url)
        .await
        .expect("connect MT-023 isolated migrated PostgreSQL schema");

    let owner_account_id = OwnerAccountId::mint();
    let actor_principal_id = ActorPrincipalId::mint();
    let authenticated_session = AuthenticatedSessionRef::mint();
    let access_space = AccessSpaceRef::mint();
    let workspace = WorkspaceScopeRef::new(format!("mt023-live-{}", Uuid::now_v7()))
        .expect("valid MT-023 workspace id");
    let owner_scope = ResourceScope::new(owner_account_id, actor_principal_id)
        .with_session(authenticated_session)
        .with_access_space(access_space)
        .with_workspace(workspace.clone());
    let model_lane_store = ModelLaneStore::new_scoped(pool.clone(), owner_scope.clone());

    let (ledger, ledger_writer) = LedgerBatcher::spawn(
        Arc::new(PostgresProcessLedgerStore::new(pool.clone())),
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig::default(),
    );
    let ledger_close = ledger.clone();

    let adapter = Arc::new(
        CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default())
            .await
            .expect("construct real Cloud Hypervisor adapter for MT-023"),
    );
    let adapter_trait: Arc<dyn SandboxAdapter> = adapter.clone();
    let mut sandbox_registry =
        SandboxAdapterRegistry::new(AdapterId::new(CLOUD_HYPERVISOR_ADAPTER_ID));
    sandbox_registry.register(adapter_trait.clone());
    let sandbox_registry = Arc::new(sandbox_registry);

    let captured_flight_events = Arc::new(Mutex::new(Vec::new()));
    let flight_capture = Arc::clone(&captured_flight_events);
    let coordinator = build_production_swarm_coordinator_with_sandbox_registry(
        ledger,
        CloudLaneFactoryConfig::unconfigured(),
        model_lane_store.clone(),
        Some(sandbox_registry),
        Some(1),
        Uuid::now_v7(),
        move |event| {
            flight_capture
                .lock()
                .map_err(|_| "MT-023 Flight Recorder capture lock poisoned".to_owned())?
                .push(event);
            Ok(())
        },
    );

    let instance_id = ModelInstanceId::new(ModelId::new_v7(), 0);
    let parent_session_id = format!("mt023-live-parent-{}", Uuid::now_v7());
    let worktree_id = format!("mt023-live-worktree-{}", Uuid::now_v7());
    let request = SpawnRequest::new(
        instance_id,
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT023",
        parent_session_id.clone(),
    )
    .with_local_artifact(gguf_path.display().to_string(), gguf_sha.clone())
    .with_worktree(worktree_id.clone())
    .with_working_dir(worktree_root.display().to_string())
    .with_isolation_tier(IsolationTier::Tier3Microvm)
    .with_committed_memory_bytes(3 * 1024 * 1024 * 1024)
    .with_warm_vm_execution();
    let request = DexterityLaunchContract::attach_to_spawn_request(
        request,
        "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
        "MT-023",
    )
    .expect("construct MT-023 Dexterity launch contract");
    let launch = request
        .dexterity_launch
        .clone()
        .expect("attached MT-023 launch contract");

    let orphans_before = adapter
        .discover_orphan_vm_dirs()
        .await
        .expect("fail closed when baseline orphan discovery cannot be proven");
    let mut spawned = false;
    let mut observed_handle = None;
    let mut observed_snapshot = None;
    let mut failures = Vec::new();

    // AC-2/AC-3 fresh-process recovery proof. A reconstructed adapter cannot
    // adopt an in-memory warm transport, so resolve must fail closed; explicit
    // teardown must nevertheless reconstruct the exact durable VM identity,
    // reclaim it, and terminalize the canonical PostgreSQL binding.
    let restart_worktree_id = format!("mt023-restart-reclaim-worktree-{}", Uuid::now_v7());
    let restart_owner_registry = WorktreeVmRegistry::new_durable(
        adapter_trait.clone(),
        pool.clone(),
        model_lane_store.access().clone(),
    );
    match restart_owner_registry
        .ensure_worktree_vm(&restart_worktree_id)
        .await
    {
        Ok(restart_handle) => {
            let restart_binding = restart_owner_registry
                .durable_binding(&restart_worktree_id)
                .await
                .expect("read restart-reclaim source binding")
                .expect("restart-reclaim source binding exists");
            let restarted_adapter = Arc::new(
                CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default())
                    .await
                    .expect("construct restart-reclaim adapter B"),
            );
            let restarted_trait: Arc<dyn SandboxAdapter> = restarted_adapter.clone();
            let restarted_registry = WorktreeVmRegistry::new_durable(
                restarted_trait,
                pool.clone(),
                model_lane_store.access().clone(),
            );
            if !matches!(
                restarted_registry
                    .resolve_worktree_vm(&restart_worktree_id)
                    .await,
                Err(WorktreeVmError::DurableHandleUnavailable { .. })
            ) {
                failures.push(
                    "fresh adapter did not fail closed before explicit detached teardown"
                        .to_string(),
                );
            }
            if let Err(error) = restarted_registry
                .teardown_worktree_vm(&restart_worktree_id)
                .await
            {
                failures.push(format!(
                    "fresh adapter could not reclaim durable worktree VM: {error}"
                ));
            } else {
                match restarted_registry
                    .durable_binding(&restart_worktree_id)
                    .await
                {
                    Ok(Some(binding))
                        if binding.binding_state == WorktreeVmBindingState::Terminated => {}
                    other => failures.push(format!(
                        "fresh-adapter teardown did not terminalize durable binding: {other:?}"
                    )),
                }
                let identity = DetachedProcessIdentity {
                    process_uuid: restart_binding.binding_id,
                    handle: restart_handle,
                    executable_sha256: None,
                    os_creation_time_100ns: None,
                };
                match restarted_adapter.detached_status(&identity).await {
                    Ok(ProcessStatus::Orphaned) => {}
                    other => failures.push(format!(
                        "fresh-adapter teardown left detached VM live: {other:?}"
                    )),
                }
            }
        }
        Err(error) => failures.push(format!(
            "create restart-reclaim source worktree VM: {error}"
        )),
    }
    // Release adapter-A bookkeeping and committed-memory accounting even when
    // adapter B already killed the OS process/root; this is idempotent cleanup.
    if let Err(error) = restart_owner_registry
        .teardown_worktree_vm(&restart_worktree_id)
        .await
    {
        failures.push(format!(
            "cleanup restart-reclaim source adapter bookkeeping: {error}"
        ));
    }

    // AC-3 cancellation-during-create failure-state proof. Wait for the exact
    // dangerous interval measured by the adversarial review: durable VM binding
    // exists, but ProcessLedger START does not. Cancelling here must reclaim the
    // VM even though no LiveSession/teardown closure was ever returned.
    let cancel_instance_id = ModelInstanceId::new(ModelId::new_v7(), 1);
    let cancel_parent_session_id = format!("mt023-cancel-create-parent-{}", Uuid::now_v7());
    let cancel_worktree_id = format!("mt023-cancel-create-worktree-{}", Uuid::now_v7());
    let cancel_request = SpawnRequest::new(
        cancel_instance_id,
        RuntimeAdapterBinding::LlamaCpp,
        "KERNEL_BUILDER-MT023",
        cancel_parent_session_id.clone(),
    )
    .with_local_artifact(gguf_path.display().to_string(), gguf_sha.clone())
    .with_worktree(cancel_worktree_id.clone())
    .with_working_dir(worktree_root.display().to_string())
    .with_isolation_tier(IsolationTier::Tier3Microvm)
    .with_committed_memory_bytes(3 * 1024 * 1024 * 1024)
    .with_warm_vm_execution();
    let cancel_request = DexterityLaunchContract::attach_to_spawn_request(
        cancel_request,
        "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
        "MT-023",
    )
    .expect("construct MT-023 cancellation-during-create launch contract");
    let cancel_spawn = {
        let coordinator = coordinator.clone();
        tokio::spawn(async move { coordinator.spawn_session(cancel_request).await })
    };
    let cancel_registry = WorktreeVmRegistry::new_durable(
        adapter_trait.clone(),
        pool.clone(),
        model_lane_store.access().clone(),
    );
    let cancel_binding = {
        let deadline = Instant::now() + Duration::from_secs(150);
        loop {
            match cancel_registry.durable_binding(&cancel_worktree_id).await {
                Ok(Some(binding)) if binding.binding_state == WorktreeVmBindingState::Active => {
                    let start_count: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM kernel_process_lifecycle \
                         WHERE parent_session_id = $1",
                    )
                    .bind(&cancel_parent_session_id)
                    .fetch_one(&pool)
                    .await
                    .expect("query cancellation-window ProcessLedger START count");
                    if start_count == 0 {
                        break Some(binding);
                    }
                    failures.push(format!(
                        "cancellation probe missed the pre-START window: durable VM binding existed but ProcessLedger already had {start_count} row(s)"
                    ));
                    break Some(binding);
                }
                Ok(Some(_)) | Ok(None) => {}
                Err(error) => {
                    failures.push(format!(
                        "read cancellation-window durable VM binding: {error}"
                    ));
                    break None;
                }
            }
            if Instant::now() >= deadline {
                failures.push(
                    "timed out waiting for durable-binding-present / ProcessLedger-START-absent cancellation window"
                        .to_string(),
                );
                break None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    let cancel_result = coordinator
        .cancel_session(cancel_instance_id, "mt023-cancel-during-create")
        .await;
    let cancel_spawn_outcome = tokio::time::timeout(Duration::from_secs(30), cancel_spawn).await;
    let spawn_diagnostic = match &cancel_spawn_outcome {
        Ok(Ok(Err(error))) => format!("spawn_error={error}"),
        Ok(Ok(Ok(spawned_id))) => format!("spawned_id={spawned_id}"),
        Ok(Err(error)) => format!("join_error={error}"),
        Err(_) => "spawn_join_timeout=30s".to_string(),
    };
    if let Err(error) = cancel_result {
        failures.push(format!(
            "cancel pending warm VM factory creation failed: {error}; {spawn_diagnostic}"
        ));
    }
    match cancel_spawn_outcome {
        Ok(Ok(Err(_))) => {}
        Ok(Ok(Ok(spawned_id))) => failures.push(format!(
            "cancellation-during-create unexpectedly returned live session {spawned_id}"
        )),
        Ok(Err(error)) => failures.push(format!("join cancellation spawn task: {error}")),
        Err(_) => failures.push("cancellation-during-create spawn task did not stop".to_string()),
    }

    let cancel_terminal = {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let binding = cancel_registry
                .durable_binding(&cancel_worktree_id)
                .await
                .expect("read post-cancel durable binding");
            let adapter_status = match cancel_binding.as_ref() {
                Some(binding) => Some(
                    adapter
                        .status(&binding.process_handle)
                        .await
                        .expect("read post-cancel adapter status"),
                ),
                None => None,
            };
            let is_terminal = binding
                .as_ref()
                .map(|row| row.binding_state == WorktreeVmBindingState::Terminated)
                .unwrap_or(true)
                && adapter_status
                    .as_ref()
                    .map(|status| {
                        matches!(
                            status,
                            ProcessStatus::Killed {
                                by_signal: Signal::Term | Signal::Kill
                            }
                        )
                    })
                    .unwrap_or(true);
            if is_terminal {
                break true;
            }
            if Instant::now() >= deadline {
                failures.push(format!(
                    "cancel after durable binding but before ProcessLedger START left an ownerless VM: binding={binding:?}, adapter_status={adapter_status:?}"
                ));
                break false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    if !cancel_terminal {
        if let Err(error) = cancel_registry
            .teardown_worktree_vm(&cancel_worktree_id)
            .await
        {
            failures.push(format!(
                "cleanup cancellation-during-create RED-state VM: {error}"
            ));
        }
    }
    if let Some(snapshot) = cancel_binding.and_then(|binding| binding.latest_snapshot) {
        if let Err(error) = adapter.delete_snapshot(&snapshot).await {
            failures.push(format!(
                "delete cancellation-during-create snapshot after proof: {error}"
            ));
        }
    }

    let proof_result: Result<(), String> = async {
        let spawned_id = coordinator
            .spawn_session(request)
            .await
            .map_err(|error| format!("spawn through ModelLaneStore/SwarmCoordinator: {error}"))?;
        if spawned_id != instance_id {
            return Err(format!(
                "coordinator returned wrong instance: expected {instance_id}, got {spawned_id}"
            ));
        }
        spawned = true;

        let durable_registry = WorktreeVmRegistry::new_durable(
            adapter_trait.clone(),
            pool.clone(),
            model_lane_store.access().clone(),
        );
        let binding = durable_registry
            .durable_binding(&worktree_id)
            .await
            .map_err(|error| format!("read durable worktree binding: {error}"))?
            .ok_or_else(|| "durable worktree binding is missing after coordinator spawn".to_string())?;
        if binding.binding_state != WorktreeVmBindingState::Snapshotted {
            return Err(format!(
                "loaded warm VM must be snapshotted before lane readiness; got {:?}",
                binding.binding_state
            ));
        }
        if binding.latest_snapshot.is_none() {
            return Err("loaded warm VM binding has no durable snapshot".to_string());
        }
        observed_handle = Some(binding.process_handle.clone());
        observed_snapshot = binding.latest_snapshot.clone();

        let handle = durable_registry
            .resolve_worktree_vm(&worktree_id)
            .await
            .map_err(|error| format!("fresh durable registry cannot adopt exact live VM: {error}"))?;

        let restarted_adapter: Arc<dyn SandboxAdapter> = Arc::new(
            CloudHypervisorAdapter::try_new(CloudHypervisorConfig::default())
                .await
                .map_err(|error| {
                    format!("construct process-restart Cloud Hypervisor adapter: {error}")
                })?,
        );
        let restarted_registry = WorktreeVmRegistry::new_durable(
            restarted_adapter,
            pool.clone(),
            model_lane_store.access().clone(),
        );
        match restarted_registry.resolve_worktree_vm(&worktree_id).await {
            Err(WorktreeVmError::DurableHandleUnavailable {
                worktree_id: unavailable_worktree,
                adapter_id,
                reason,
            }) if unavailable_worktree == worktree_id
                && adapter_id == CLOUD_HYPERVISOR_ADAPTER_ID
                && reason == format!("sandbox process handle stale: {}", handle.id) => {}
            Ok(adopted) => {
                return Err(format!(
                    "a fresh process-local adapter falsely adopted the pre-restart VM handle: {adopted:?}"
                ));
            }
            Err(error) => {
                return Err(format!(
                    "process-restart recovery did not fail closed with the named durable-handle reason: {error}"
                ));
            }
        }

        let exec = adapter
            .exec(
                &handle,
                Command {
                    argv: vec![
                        "/bin/sh".to_string(),
                        "-c".to_string(),
                        "sha256sum /worktree/AGENTS.md && cat /worktree/.git".to_string(),
                    ],
                    env_overlay: Default::default(),
                    stdin: None,
                    timeout_ms: Some(30_000),
                },
            )
            .await
            .map_err(|error| format!("execute worktree identity probe inside live VM: {error}"))?;
        if exec.exit_code != 0 {
            return Err(format!(
                "in-guest /worktree probe failed rc={}: {}",
                exec.exit_code,
                String::from_utf8_lossy(&exec.stderr)
            ));
        }
        let guest_identity = String::from_utf8_lossy(&exec.stdout);
        let host_agents_sha = sha256_file_hex(&worktree_root.join("AGENTS.md"));
        if !guest_identity.starts_with(&host_agents_sha) {
            return Err(format!(
                "guest /worktree/AGENTS.md hash does not match bound host worktree; expected {host_agents_sha}, got {guest_identity:?}"
            ));
        }
        let host_git_pointer = std::fs::read_to_string(worktree_root.join(".git"))
            .map_err(|error| format!("read host linked-worktree .git pointer: {error}"))?;
        if !guest_identity.contains(host_git_pointer.trim()) {
            return Err(format!(
                "guest /worktree/.git does not identify the exact host worktree; expected {:?}, got {guest_identity:?}",
                host_git_pointer.trim()
            ));
        }

        let other_scope = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(workspace.clone());
        let other_store = ModelLaneStore::new_scoped(pool.clone(), other_scope);
        let other_registry = WorktreeVmRegistry::new_durable(
            adapter_trait.clone(),
            pool.clone(),
            other_store.access().clone(),
        );
        if other_registry
            .durable_binding(&worktree_id)
            .await
            .map_err(|error| format!("cross-account binding read failed unexpectedly: {error}"))?
            .is_some()
        {
            return Err("cross-account reader enumerated the owner worktree VM".to_string());
        }
        other_registry
            .teardown_worktree_vm(&worktree_id)
            .await
            .map_err(|error| format!("cross-account no-op teardown returned error: {error}"))?;
        if adapter
            .status(&handle)
            .await
            .map_err(|error| format!("read VM status after cross-account denial: {error}"))?
            != ProcessStatus::Running
        {
            return Err("cross-account teardown affected the owner VM".to_string());
        }

        let replay = model_lane_store
            .replay_run(&launch.run_id)
            .await
            .map_err(|error| format!("replay MT-023 ModelLane rows: {error}"))?;
        if replay.lanes.len() != 1 || replay.lanes[0].lane_id != launch.lane_id {
            return Err(format!(
                "ModelLaneStore replay did not return the coordinator lane: {:?}",
                replay.lanes
            ));
        }
        if replay.lanes[0].runtime_binding != ModelLaneRuntimeBinding::Local {
            return Err(format!(
                "ModelLane runtime binding is not llama_cpp: {:?}",
                replay.lanes[0].runtime_binding
            ));
        }

        let scope_row = sqlx::query(
            "SELECT owner_account_id, actor_principal_id, authenticated_session_id, \
                    access_space_id, workspace_id \
             FROM model_lanes WHERE lane_id = $1",
        )
        .bind(&launch.lane_id)
        .fetch_one(&pool)
        .await
        .map_err(|error| format!("read MT-023 ModelLane scope columns: {error}"))?;
        if scope_row.try_get::<Option<Uuid>, _>("owner_account_id").ok().flatten()
            != Some(owner_account_id.as_uuid())
            || scope_row
                .try_get::<Option<Uuid>, _>("actor_principal_id")
                .ok()
                .flatten()
                != Some(actor_principal_id.as_uuid())
            || scope_row
                .try_get::<Option<Uuid>, _>("authenticated_session_id")
                .ok()
                .flatten()
                != Some(authenticated_session.as_uuid())
            || scope_row
                .try_get::<Option<Uuid>, _>("access_space_id")
                .ok()
                .flatten()
                != Some(access_space.as_uuid())
            || scope_row
                .try_get::<Option<String>, _>("workspace_id")
                .ok()
                .flatten()
                .as_deref()
                != Some(workspace.as_str())
        {
            return Err("ModelLane row does not carry the exact five-field owner scope".to_string());
        }

        let mut stream = coordinator
            .generate_session_managed(instance_id, generation_request())
            .map_err(|error| format!("start coordinator-owned real generation: {error}"))?;
        let mut generated_text = String::new();
        let mut saw_terminal = false;
        loop {
            let item = tokio::time::timeout(Duration::from_secs(150), stream.next())
                .await
                .map_err(|_| "timed out waiting for real token frame from microVM".to_string())?;
            let Some(item) = item else { break };
            let token = item.map_err(|error| format!("real microVM token stream failed: {error}"))?;
            if token.finish_reason.is_some() {
                saw_terminal = true;
            } else {
                generated_text.push_str(&token.text);
            }
        }
        if generated_text.trim().is_empty() {
            return Err("real coordinator-routed microVM generation emitted no token text".to_string());
        }
        if !saw_terminal {
            return Err("real coordinator-routed microVM generation emitted no terminal frame".to_string());
        }
        Ok(())
    }
    .await;
    if let Err(error) = proof_result {
        failures.push(error);
    }

    if spawned && coordinator.session_state(instance_id).is_some() {
        if let Err(error) = coordinator
            .cancel_session(instance_id, "mt023-live-proof-cleanup")
            .await
        {
            failures.push(format!("operator-cancel live MT-023 lane: {error}"));
        }
    }

    let owner_registry = WorktreeVmRegistry::new_durable(
        adapter_trait,
        pool.clone(),
        model_lane_store.access().clone(),
    );
    match owner_registry.durable_binding(&worktree_id).await {
        Ok(Some(binding)) if spawned => {
            if binding.binding_state != WorktreeVmBindingState::Terminated {
                failures.push(format!(
                    "operator cancel did not terminalize durable VM binding: {:?}",
                    binding.binding_state
                ));
            }
        }
        Ok(None) if spawned => {
            failures.push("durable VM binding disappeared after cancel".to_string())
        }
        Ok(_) => {}
        Err(error) => failures.push(format!("read terminal durable binding: {error}")),
    }

    if let Some(handle) = observed_handle.as_ref() {
        match adapter.status(handle).await {
            Ok(ProcessStatus::Killed {
                by_signal: Signal::Term,
            }) => {}
            Ok(status) => failures.push(format!(
                "session cancel left VM in non-terminal adapter state: {status:?}"
            )),
            Err(error) => failures.push(format!("read VM adapter status after cancel: {error}")),
        }
    }
    let orphans_after = adapter
        .discover_orphan_vm_dirs()
        .await
        .expect("fail closed when final orphan discovery cannot be proven");
    let new_orphans: Vec<_> = orphans_after
        .iter()
        .filter(|path| !orphans_before.contains(path))
        .cloned()
        .collect();
    if !new_orphans.is_empty() {
        failures.push(format!(
            "MT-023 run introduced orphan VM directories: {new_orphans:?}"
        ));
    }

    let drain =
        drain_and_join_ledger_writer(&ledger_close, ledger_writer, Duration::from_secs(15)).await;
    if !matches!(drain, LedgerDrainJoinOutcome::Flushed) {
        failures.push(format!(
            "PostgreSQL process ledger did not drain: {drain:?}"
        ));
    }
    if spawned {
        {
            let flight_events = captured_flight_events
                .lock()
                .expect("MT-023 Flight Recorder capture lock");
            if flight_events.is_empty() {
                failures
                    .push("production coordinator emitted no Flight Recorder events".to_owned());
            }
            for event in flight_events.iter() {
                match serde_json::from_value::<
                    handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution,
                >(event.payload.clone()) {
                    Ok(scope)
                        if scope.owner_account_id == owner_account_id
                            && scope.actor_principal_id == actor_principal_id
                            && scope.authenticated_session_id == authenticated_session
                            && scope.access_space_id == access_space
                            && scope.workspace_id == workspace => {}
                    Ok(scope) => failures.push(format!(
                        "Flight Recorder event lost exact five-field scope: {scope:?}"
                    )),
                    Err(error) => failures.push(format!(
                        "Flight Recorder event has no exact five-field scope: {error}"
                    )),
                }
            }
        }
        match sqlx::query(
            "SELECT stopped_at, stop_reason, sandbox_adapter_id, owner_role, wp_id, mt_id, \
                    metadata_jsonb \
             FROM kernel_process_lifecycle \
             WHERE parent_session_id = $1",
        )
        .bind(&parent_session_id)
        .fetch_one(&pool)
        .await
        {
            Ok(row) => {
                if row
                    .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("stopped_at")
                    .ok()
                    .flatten()
                    .is_none()
                {
                    failures.push("VM process ledger row has START but no STOP".to_string());
                }
                if row
                    .try_get::<Option<String>, _>("stop_reason")
                    .ok()
                    .flatten()
                    .as_deref()
                    != Some("mt023-live-proof-cleanup")
                {
                    failures.push(
                        "VM process ledger STOP reason does not preserve operator cancel"
                            .to_string(),
                    );
                }
                if row
                    .try_get::<Option<String>, _>("sandbox_adapter_id")
                    .ok()
                    .flatten()
                    .as_deref()
                    != Some(CLOUD_HYPERVISOR_ADAPTER_ID)
                    || row.try_get::<String, _>("owner_role").ok().as_deref()
                        != Some("KERNEL_BUILDER-MT023")
                    || row
                        .try_get::<Option<String>, _>("wp_id")
                        .ok()
                        .flatten()
                        .as_deref()
                        != Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
                    || row
                        .try_get::<Option<String>, _>("mt_id")
                        .ok()
                        .flatten()
                        .as_deref()
                        != Some("MT-023")
                {
                    failures.push(
                        "VM process ledger row lost sandbox/owner/WP/MT metadata".to_string(),
                    );
                }
                match row.try_get::<serde_json::Value, _>("metadata_jsonb") {
                    Ok(metadata)
                        if metadata["checkout_lease_worktree_id"].as_str()
                            == Some(worktree_id.as_str())
                            && metadata["owner_account_id"]
                                == serde_json::json!(owner_account_id.as_uuid())
                            && metadata["actor_principal_id"]
                                == serde_json::json!(actor_principal_id.as_uuid())
                            && metadata["authenticated_session_id"]
                                == serde_json::json!(authenticated_session.as_uuid())
                            && metadata["access_space_id"]
                                == serde_json::json!(access_space.as_uuid())
                            && metadata["workspace_id"].as_str() == Some(workspace.as_str()) => {}
                    Ok(metadata) => failures.push(format!(
                        "VM process ledger metadata lost worktree or exact five-field scope identity: {metadata}"
                    )),
                    Err(error) => failures.push(format!("read VM process metadata JSON: {error}")),
                }
            }
            Err(error) => failures.push(format!("read VM START/STOP lifecycle row: {error}")),
        }
    }

    if let Some(snapshot) = observed_snapshot {
        if let Err(error) = adapter.delete_snapshot(&snapshot).await {
            failures.push(format!(
                "delete MT-023 proof snapshot after assertions: {error}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "MT-023 real worktree model-lane proof failed:\n{}",
        failures.join("\n")
    );
}
