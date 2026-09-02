//! WP-1 MT-023: real worktree-scoped model-lane proof over embedded SurrealDB.
//!
//! This target is ignored by default because it boots Cloud Hypervisor/KVM,
//! loads a real GGUF through the packaged guest warm agent, and generates real
//! tokens. Embedded SurrealDB is the only durable database in the proof.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use futures::StreamExt;
use handshake_core::{
    model_runtime::{
        registry::RuntimeBinding as RuntimeAdapterBinding, CancellationToken, GenPrompt,
        GenerateRequest, ModelId, SamplingParams,
    },
    process_ledger::{
        drain_and_join_ledger_writer, LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome,
        NoopOverflowSink, SurrealProcessLedgerStore,
    },
    sandbox::{
        AdapterId, CloudHypervisorAdapter, CloudHypervisorConfig, Command, IsolationTier,
        ProcessStatus, SandboxAdapter, SandboxAdapterRegistry, Signal, CLOUD_HYPERVISOR_ADAPTER_ID,
    },
    swarm_orchestration::{
        build_production_swarm_coordinator_with_sandbox_registry,
        model_lane::{
            DexterityLaunchContract, ModelLaneStore, RuntimeBinding as ModelLaneRuntimeBinding,
        },
        resource_scope::{
            AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId,
            ResourceAccessContext, ResourceScope, WorkspaceScopeRef,
        },
        CloudLaneFactoryConfig, ModelInstanceId, SpawnRequest, WorktreeVmBindingState,
        WorktreeVmRegistry,
    },
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

mod surreal_test_store_support;

use surreal_test_store_support::EmbeddedSurrealTestScope;

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
        prompt: GenPrompt::new("MT-023 microVM proof: produce one short sentence."),
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
#[ignore = "requires WSL2 /dev/kvm, Cloud Hypervisor, packaged hsk-warm-agent + llama-server, and a real GGUF"]
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
        "hsk-warm-agent and llama-server must come from one validated package"
    );
    let gguf_sha =
        std::env::var(MT023_GGUF_SHA_ENV).unwrap_or_else(|_| sha256_file_hex(&gguf_path));
    assert_eq!(sha256_file_hex(&gguf_path), gguf_sha);

    let mut surreal_scope = EmbeddedSurrealTestScope::create()
        .await
        .expect("allocate exact MT-023 embedded SurrealDB test scope");
    let storage = surreal_scope
        .activate_storage()
        .await
        .expect("activate production SurrealStorage for MT-023 live proof");

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
    let model_lane_store = ModelLaneStore::new_scoped(storage.clone(), owner_scope.clone());

    let process_store = Arc::new(SurrealProcessLedgerStore::new(storage.clone()));
    let (ledger, ledger_writer) = LedgerBatcher::spawn(
        process_store,
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
        parent_session_id,
    )
    .with_local_artifact(gguf_path.display().to_string(), gguf_sha)
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
        .expect("prove baseline orphan set");
    assert_eq!(
        coordinator
            .spawn_session(request)
            .await
            .expect("spawn real MT-023 model lane"),
        instance_id
    );

    let durable_registry = WorktreeVmRegistry::new_durable(
        adapter_trait.clone(),
        storage.clone(),
        ResourceAccessContext::for_account(owner_scope.clone()),
    );
    let binding = durable_registry
        .durable_binding(&worktree_id)
        .await
        .expect("read durable worktree binding")
        .expect("durable worktree binding exists");
    assert_eq!(binding.binding_state, WorktreeVmBindingState::Snapshotted);
    assert!(binding.latest_snapshot.is_some());
    let handle = durable_registry
        .resolve_worktree_vm(&worktree_id)
        .await
        .expect("adopt exact live worktree VM");

    let exec = adapter
        .exec(
            &handle,
            Command {
                argv: vec![
                    "/bin/sh".to_owned(),
                    "-c".to_owned(),
                    "sha256sum /worktree/AGENTS.md && cat /worktree/.git".to_owned(),
                ],
                env_overlay: Default::default(),
                stdin: None,
                timeout_ms: Some(30_000),
            },
        )
        .await
        .expect("execute worktree identity probe inside VM");
    assert_eq!(
        exec.exit_code,
        0,
        "{}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let guest_identity = String::from_utf8_lossy(&exec.stdout);
    assert!(guest_identity.starts_with(&sha256_file_hex(&worktree_root.join("AGENTS.md"))));
    let host_git_pointer =
        std::fs::read_to_string(worktree_root.join(".git")).expect("read worktree .git pointer");
    assert!(guest_identity.contains(host_git_pointer.trim()));

    let replay = model_lane_store
        .replay_run(&launch.run_id)
        .await
        .expect("replay MT-023 ModelLane authority");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].lane_id, launch.lane_id);
    assert_eq!(
        replay.lanes[0].runtime_binding,
        ModelLaneRuntimeBinding::Local
    );

    let mut stream = coordinator
        .generate_session_managed(instance_id, generation_request())
        .expect("start coordinator-owned generation");
    let mut generated_text = String::new();
    let mut saw_terminal = false;
    while let Some(item) = tokio::time::timeout(Duration::from_secs(150), stream.next())
        .await
        .expect("receive real token frame")
    {
        let token = item.expect("real microVM token stream");
        if token.finish_reason.is_some() {
            saw_terminal = true;
        } else {
            generated_text.push_str(&token.text);
        }
    }
    assert!(!generated_text.trim().is_empty());
    assert!(saw_terminal);

    let other_scope = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(workspace);
    let other_registry = WorktreeVmRegistry::new_durable(
        adapter_trait,
        storage.clone(),
        ResourceAccessContext::for_account(other_scope),
    );
    assert!(other_registry
        .durable_binding(&worktree_id)
        .await
        .expect("cross-account lookup fails closed")
        .is_none());
    assert_eq!(
        adapter.status(&handle).await.expect("owner VM status"),
        ProcessStatus::Running
    );

    coordinator
        .cancel_session(instance_id, "mt023-live-proof-cleanup")
        .await
        .expect("operator-cancel live MT-023 lane");
    let terminal = durable_registry
        .durable_binding(&worktree_id)
        .await
        .expect("read terminal binding")
        .expect("terminal binding remains auditable");
    assert_eq!(terminal.binding_state, WorktreeVmBindingState::Terminated);
    assert!(matches!(
        adapter
            .status(&handle)
            .await
            .expect("terminal adapter status"),
        ProcessStatus::Killed {
            by_signal: Signal::Term | Signal::Kill
        }
    ));
    assert_eq!(
        adapter
            .discover_orphan_vm_dirs()
            .await
            .expect("final orphan set"),
        orphans_before
    );

    let drain =
        drain_and_join_ledger_writer(&ledger_close, ledger_writer, Duration::from_secs(15)).await;
    assert!(matches!(drain, LedgerDrainJoinOutcome::Flushed));
    assert!(!captured_flight_events
        .lock()
        .expect("Flight Recorder capture lock")
        .is_empty());

    drop(other_registry);
    drop(durable_registry);
    drop(coordinator);
    drop(model_lane_store);
    drop(storage);
    let cleanup = surreal_scope
        .cleanup()
        .await
        .expect("clean exact MT-023 embedded SurrealDB test scope");
    assert!(cleanup.database_absent);
    assert!(cleanup.namespace_absent_after_reopen);
}
