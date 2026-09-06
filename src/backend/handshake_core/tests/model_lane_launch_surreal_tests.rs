//! WP-1 MT-003: launch-adapter normalization and launched-resource privacy
//! proof on embedded SurrealDB.
//!
//! This is the embedded-SurrealDB/EventLedger replacement for the superseded
//! PostgreSQL-era `model_lane_launch_tests` suite. The operator storage
//! correction of 2026-09-01 makes embedded SurrealDB the only permitted
//! MT-003 production and proof database [CX-503R], so the launch proof runs
//! against one exclusive embedded namespace/database per test and drives the
//! production `SwarmCoordinator` -> `ModelLaneStore` launch path. There is no
//! PostgreSQL, SQLite, mock-store, or in-memory authority fallback here.
//!
//! BYOK posture follows the operator correction of 2026-08-30: no
//! operator-provisioned or paid-provider credential is required. Cloud lanes
//! use a generated inert credential in an isolated in-process vault plus a
//! controlled no-egress recording provider, so "zero provider calls" is
//! observable rather than asserted.

mod surreal_test_store_support;

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream;
use futures::StreamExt;
use handshake_core::model_runtime::cloud::{InMemorySecretsVault, SecretsVault};
use handshake_core::model_runtime::registry::RuntimeBinding as RuntimeAdapterBinding;
use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenPrompt, GenerateRequest, KvCacheHandle, KvCachePolicy,
    LoadSpec, LoraStackHandle, ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError,
    ProviderKind, RuntimeKind, SamplingParams, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink, ProcessEngineKind,
    ProcessOwnershipRecordId, ProcessStart,
};
use handshake_core::storage::surreal::{
    bootstrap_schema, RowFilter, SurrealStorage, SurrealTestInspector,
};
use handshake_core::swarm_orchestration::model_lane::{
    dexterity_spawn_model_session_id, CloudExportDelegation, DexterityLaunchAdapterKind,
    DexterityLaunchAdapterRegistry, DexterityLaunchAdapterRequest, DexterityLaunchContract,
    DexterityNormalizedLaunch, ModelLaneCloudConsentReceiptStatus, ModelLaneCloudConsentScope,
    ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanStatus, ModelLaneCloudRetentionPolicy,
    ModelLaneError, ModelLaneNavigationLookup, ModelLaneRecord, ModelLaneRunRecord,
    ModelLaneStatus, ModelLaneStore, NewModelLaneCloudConsentReceipt,
    NewModelLaneCloudProjectionPlan, RuntimeBinding,
};
use handshake_core::swarm_orchestration::production_factory::{
    build_production_swarm_coordinator, CloudLaneFactoryConfig, CloudLiveRuntime,
    CloudRuntimeBuilder,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, AccountBoundAuthority, ActorPrincipalId, AuthenticatedSessionRef,
    ExactResourceScopeAttribution, OwnerAccountId, ResourceAccessContext,
    ResourceAccessLifecycleRegistry, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::swarm_orchestration::{
    ByokCloudProvider, LiveSession, ModelInstanceId, ModelSessionFactory, RecordingSwarmSink,
    RunBudget, SpawnRequest, SwarmConfig, SwarmCoordinator, SwarmError,
};
use serde_json::json;
use surreal_test_store_support::EmbeddedSurrealTestScope;

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-003";
const OWNER_SESSION: &str = "KERNEL_BUILDER-MT003";
/// Canonical embedded ModelLane EventLedger receipt prefix. The PostgreSQL-era
/// `KE-` prefix is superseded together with its database.
const MODEL_LANE_EVENT_ID_PREFIX: &str = "evt-model-lane-";
/// The five exact attribution columns, in canonical order, as stamped on both
/// `model_lane_authority` rows and their canonical `kernel_event_ledger` rows.
const SCOPE_FIELDS: [&str; 5] = [
    "owner_account_id",
    "actor_principal_id",
    "authenticated_session_id",
    "access_space_id",
    "workspace_id",
];

// ---------------------------------------------------------------------------
// Embedded harness
// ---------------------------------------------------------------------------

struct Harness {
    isolated: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    scope: ResourceScope,
    exact: ExactResourceScopeAttribution,
    lifecycle: ResourceAccessLifecycleRegistry,
    store: ModelLaneStore,
}

impl Harness {
    /// Every test owns an exclusive embedded store: sharing one database across
    /// concurrent tests was measured to fail with key-value commit conflicts.
    async fn create(label: &str) -> Self {
        let mut isolated = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate exact MT-003 launch embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical embedded schema");
        let scope = exact_scope(label);
        let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
            .expect("launch proof scope carries all five exact attribution fields");
        let lifecycle = ResourceAccessLifecycleRegistry::new();
        register_active_context(&lifecycle, &scope);
        let store = ModelLaneStore::new_scoped_with_lifecycle(
            storage.clone(),
            scope.clone(),
            lifecycle.clone(),
        );
        Self {
            isolated,
            storage,
            scope,
            exact,
            lifecycle,
            store,
        }
    }

    /// Read-only store for the exact launch owner.
    fn exact_reader(&self) -> ModelLaneStore {
        self.reader_for(self.exact.clone())
    }

    /// Registers `exact` ACTIVE before building its reader, so a denial proves
    /// five-field ResourceScope isolation rather than an unregistered lifecycle.
    fn reader_for(&self, exact: ExactResourceScopeAttribution) -> ModelLaneStore {
        self.lifecycle
            .register_active(exact.clone())
            .expect("register active foreign read context");
        ModelLaneStore::new(
            self.storage.clone(),
            ResourceAccessContext::for_exact_reader_with_lifecycle(exact, self.lifecycle.clone()),
        )
    }

    /// Writable store for a switched (foreign) exact context.
    fn writer_for(&self, exact: &ExactResourceScopeAttribution) -> ModelLaneStore {
        let scope = scope_from_exact(exact);
        register_active_context(&self.lifecycle, &scope);
        ModelLaneStore::new_scoped_with_lifecycle(
            self.storage.clone(),
            scope,
            self.lifecycle.clone(),
        )
    }

    fn inspector(&self) -> SurrealTestInspector {
        self.storage.test_inspector()
    }

    /// The five durably stamped attribution values for one authority aggregate.
    async fn persisted_scope(&self, aggregate_id: &str) -> Vec<String> {
        let inspector = self.inspector();
        let authority = inspector
            .table_selector("model_lane_authority")
            .await
            .expect("model_lane_authority is in the embedded catalog");
        let fields = SCOPE_FIELDS
            .iter()
            .map(|field| authority.field(field).expect("scope field selector"))
            .collect::<Vec<_>>();
        let rows = inspector
            .project(
                &authority,
                &fields,
                RowFilter::FieldEquals {
                    field: authority.field("aggregate_id").expect("aggregate_id"),
                    value: aggregate_id.to_owned().into(),
                },
            )
            .await
            .expect("project the launched authority row");
        assert_eq!(
            rows.len(),
            1,
            "exactly one durable authority row exists for {aggregate_id}"
        );
        SCOPE_FIELDS
            .iter()
            .map(|field| {
                rows[0].values[*field]
                    .as_str()
                    .unwrap_or_else(|| panic!("{field} is a durable string on {aggregate_id}"))
                    .to_owned()
            })
            .collect()
    }

    /// The five attribution values on the canonical EventLedger row that the
    /// authority aggregate references. A missing row is a hard failure: the
    /// reference is `ASSERT record::exists`, so an authority row cannot commit
    /// without its atomically appended kernel EventLedger event.
    async fn ledger_event_scope(&self, event_id: &str) -> Vec<String> {
        let inspector = self.inspector();
        let ledger = inspector
            .table_selector("kernel_event_ledger")
            .await
            .expect("kernel_event_ledger is in the embedded catalog");
        let fields = SCOPE_FIELDS
            .iter()
            .map(|field| ledger.field(field).expect("ledger scope field selector"))
            .collect::<Vec<_>>();
        let rows = inspector
            .project(
                &ledger,
                &fields,
                RowFilter::FieldEquals {
                    field: ledger.field("event_id").expect("event_id"),
                    value: event_id.to_owned().into(),
                },
            )
            .await
            .expect("project the canonical launch EventLedger row");
        assert_eq!(
            rows.len(),
            1,
            "the launch authority row references exactly one canonical EventLedger event"
        );
        SCOPE_FIELDS
            .iter()
            .map(|field| {
                rows[0].values[*field]
                    .as_str()
                    .unwrap_or_else(|| panic!("EventLedger {event_id} lost attribution {field}"))
                    .to_owned()
            })
            .collect()
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("clean exact MT-003 launch embedded scope");
    }
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt003-{label}")).expect("nonblank workspace"),
        )
}

fn scope_from_exact(exact: &ExactResourceScopeAttribution) -> ResourceScope {
    ResourceScope::new(exact.owner_account_id, exact.actor_principal_id)
        .with_session(exact.authenticated_session_id)
        .with_access_space(exact.access_space_id)
        .with_workspace(exact.workspace_id.clone())
}

fn register_active_context(lifecycle: &ResourceAccessLifecycleRegistry, scope: &ResourceScope) {
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(scope)
        .expect("proof scope must carry all five exact attribution fields");
    lifecycle
        .register_active(exact)
        .expect("register active authenticated resource-access context");
}

fn expected_scope_values(exact: &ExactResourceScopeAttribution) -> Vec<String> {
    vec![
        exact.owner_account_id.as_uuid().to_string(),
        exact.actor_principal_id.as_uuid().to_string(),
        exact.authenticated_session_id.as_uuid().to_string(),
        exact.access_space_id.as_uuid().to_string(),
        exact.workspace_id.as_str().to_string(),
    ]
}

/// One foreign context per attribution dimension. Each differs from `exact` in
/// exactly one field, so a denial proves the full five-field predicate rather
/// than an owner-only check.
fn foreign_exact_scopes(
    exact: &ExactResourceScopeAttribution,
) -> Vec<(&'static str, ExactResourceScopeAttribution)> {
    let mut owner = exact.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut principal = exact.clone();
    principal.actor_principal_id = ActorPrincipalId::mint();
    let mut session = exact.clone();
    session.authenticated_session_id = AuthenticatedSessionRef::mint();
    let mut access_space = exact.clone();
    access_space.access_space_id = AccessSpaceRef::mint();
    let mut workspace = exact.clone();
    workspace.workspace_id =
        WorkspaceScopeRef::new("workspace-mt003-foreign").expect("nonblank foreign workspace");
    vec![
        ("owner account", owner),
        ("actor principal", principal),
        ("authenticated session", session),
        ("AccessSpace", access_space),
        ("workspace", workspace),
    ]
}

fn assert_absent_without_scope_leak<T: std::fmt::Debug>(
    result: Result<T, ModelLaneError>,
    changed_dimension: &str,
    restricted: &ExactResourceScopeAttribution,
) -> String {
    let denied = result.expect_err("a one-dimension foreign scope must observe absence");
    assert!(
        matches!(&denied, ModelLaneError::NotFound(_)),
        "foreign {changed_dimension} denial must be indistinguishable from absence: {denied}"
    );
    let rendered = denied.to_string();
    for identifier in expected_scope_values(restricted) {
        assert!(
            !rendered.contains(&identifier),
            "foreign {changed_dimension} denial leaked restricted scope identifier {identifier}: {rendered}"
        );
    }
    rendered
}

/// Cloud consent authority denies a foreign context through the CX-MM-007
/// consent failstate rather than through row absence, so this asserts that
/// shape explicitly instead of coercing it into a `NotFound`.
fn assert_cloud_authority_denied_without_scope_leak<T: std::fmt::Debug>(
    result: Result<T, ModelLaneError>,
    changed_dimension: &str,
    restricted: &ExactResourceScopeAttribution,
) {
    let denied = result.expect_err("a one-dimension foreign scope must not revoke foreign consent");
    let rendered = denied.to_string();
    assert!(
        matches!(
            &denied,
            ModelLaneError::AuthorityDenied(_) | ModelLaneError::NotFound(_)
        ),
        "foreign {changed_dimension} consent revocation must fail closed: {denied}"
    );
    assert!(
        rendered.contains("CX-MM-007"),
        "foreign {changed_dimension} consent revocation must carry the CX-MM-007 failstate: {rendered}"
    );
    for identifier in expected_scope_values(restricted) {
        assert!(
            !rendered.contains(&identifier),
            "foreign {changed_dimension} consent denial leaked restricted scope identifier {identifier}: {rendered}"
        );
    }
}

// ---------------------------------------------------------------------------
// MT-003 acceptance: deterministic no-egress fail-closed cloud launch
// ---------------------------------------------------------------------------

/// AC: "Required BYOK proof MUST NOT require Operator-provisioned or
/// paid-provider credentials ... prove zero provider calls when provider
/// configuration or ProjectionPlan/ConsentReceipt authority is absent".
#[tokio::test]
async fn model_lane_cloud_launch_is_no_egress_and_fail_closed_without_authority() {
    let mut harness = Harness::create("cloud-no-egress").await;
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 128,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");

    // Generated inert credential in an isolated in-process vault. No operator
    // secret, no OS keychain entry, and no paid provider is involved.
    let vault: Arc<dyn SecretsVault> = Arc::new(InMemorySecretsVault::default());
    vault
        .put("mt003-inert-openai", "mt003-inert-openai-credential")
        .expect("store generated inert credential in isolated test vault");
    let cloud_builds = Arc::new(AtomicUsize::new(0));
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let cloud = CloudLaneFactoryConfig {
        openai: Some(Arc::new(NoEgressCloudBuilder::new(
            ProviderKind::ByokCloud,
            vault.clone(),
            "mt003-inert-openai",
            cloud_builds.clone(),
            provider_calls.clone(),
        ))),
        anthropic: None,
        official_cli: None,
        official_cli_by_provider: HashMap::new(),
    };
    let coordinator = build_production_swarm_coordinator(
        ledger,
        cloud,
        harness.store.clone(),
        Some(2),
        uuid::Uuid::now_v7(),
        |_event| Ok(()),
    );

    // 1. No ProjectionPlan / ConsentReceipt authority at all: fail closed with
    //    denial evidence and zero provider contact.
    let missing_normalized = DexterityLaunchAdapterRegistry::standard()
        .normalize(launch_request(
            DexterityLaunchAdapterKind::ByokCloudOpenAi,
            0,
            ModelLaneStatus::Ready,
        ))
        .expect("missing-authority cloud launch normalizes before durable preflight");
    let missing_error = coordinator
        .spawn_session(spawn_request_for_adapter(
            DexterityLaunchAdapterKind::ByokCloudOpenAi,
            0,
            spawn_contract_from_normalized(&missing_normalized),
        ))
        .await
        .expect_err("cloud launch without ProjectionPlan/ConsentReceipt must fail closed");
    assert!(
        missing_error.to_string().contains("CX-MM-007"),
        "missing cloud authority must be a consent denial: {missing_error}"
    );
    assert_eq!(
        cloud_builds.load(Ordering::SeqCst),
        0,
        "missing cloud authority must reject before the cloud builder opens a runtime"
    );
    assert_eq!(
        provider_calls.load(Ordering::SeqCst),
        0,
        "missing cloud authority must make zero provider calls"
    );
    assert!(
        harness
            .store
            .replay_run("run-mt003-0")
            .await
            .is_err(),
        "a fail-closed cloud launch must leave no durable launch authority behind"
    );

    // 2. Revoked authority is equally fail-closed and equally silent.
    let revoked_normalized = DexterityLaunchAdapterRegistry::standard()
        .normalize(launch_request(
            DexterityLaunchAdapterKind::ByokCloudOpenAi,
            1,
            ModelLaneStatus::Ready,
        ))
        .expect("revoked-authority cloud launch normalizes");
    let revoked_spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        1,
        spawn_contract_from_normalized(&revoked_normalized),
    );
    seed_cloud_launch_authority(
        &harness.store,
        &revoked_spawn,
        &DexterityLaunchAdapterKind::ByokCloudOpenAi,
        1,
    )
    .await;
    harness
        .store
        .test_commit_cloud_consent_revocation(
            "consent://mt003/1",
            "operator://mt003/no-egress-revoke",
            "revoked before any launch attempt",
        )
        .await
        .expect("revoke the seeded consent before launch");
    let revoked_error = coordinator
        .spawn_session(revoked_spawn)
        .await
        .expect_err("revoked consent must fail the cloud launch closed");
    assert!(
        revoked_error.to_string().contains("CX-MM-007"),
        "revoked cloud authority must be a consent denial: {revoked_error}"
    );
    assert_eq!(
        cloud_builds.load(Ordering::SeqCst),
        0,
        "revoked cloud authority must reject before the cloud builder opens a runtime"
    );
    assert_eq!(
        provider_calls.load(Ordering::SeqCst),
        0,
        "revoked cloud authority must make zero provider calls"
    );

    // 3. Configured inert credential plus valid authority: exactly one build and
    //    one recorded provider call through the no-egress recording provider.
    let valid_normalized = DexterityLaunchAdapterRegistry::standard()
        .normalize(launch_request(
            DexterityLaunchAdapterKind::ByokCloudOpenAi,
            2,
            ModelLaneStatus::Ready,
        ))
        .expect("valid cloud launch normalizes through the registry");
    let valid_spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        2,
        spawn_contract_from_normalized(&valid_normalized),
    );
    seed_cloud_launch_authority(
        &harness.store,
        &valid_spawn,
        &DexterityLaunchAdapterKind::ByokCloudOpenAi,
        2,
    )
    .await;
    let expected_model_session = dexterity_spawn_model_session_id(&valid_spawn);
    let valid_run_id = valid_spawn
        .dexterity_launch
        .as_ref()
        .expect("valid launch retains its Dexterity contract")
        .run_id
        .clone();
    let instance_id = valid_spawn.instance_id;
    coordinator
        .spawn_session(valid_spawn)
        .await
        .expect("valid inert-vault cloud launch uses the Rust registry");
    prove_generation(&coordinator, instance_id, "ByokCloudOpenAi").await;
    assert_eq!(cloud_builds.load(Ordering::SeqCst), 1);
    assert_eq!(provider_calls.load(Ordering::SeqCst), 1);

    // 4. The launch authority is durable: it survives a real component restart
    //    of the same embedded namespace/database, not just a process-local map.
    coordinator
        .drain_all()
        .await
        .expect("drain the no-egress cloud proof");
    drop(coordinator);
    drop(harness.store);
    harness
        .isolated
        .close_for_reopen()
        .await
        .expect("close the embedded store for a component restart");
    let storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reopen the same embedded namespace/database after restart");
    harness.storage = storage.clone();
    harness.store = ModelLaneStore::new_scoped_with_lifecycle(
        storage,
        harness.scope.clone(),
        harness.lifecycle.clone(),
    );
    let replay = harness
        .store
        .replay_cloud_consent_authority(&valid_run_id)
        .await
        .expect("replay durable MT-003 cloud authority after restart");
    assert_eq!(replay.projection_plans.len(), 1);
    assert_eq!(replay.consent_receipts.len(), 1);
    assert_eq!(
        replay.projection_plans[0].model_session_id.as_deref(),
        Some(expected_model_session.as_str()),
        "the restarted store must rebind the launched model session identity"
    );
    assert_eq!(
        provider_calls.load(Ordering::SeqCst),
        1,
        "restart replay must read durable authority without contacting a provider"
    );
    harness.cleanup().await;
}

// ---------------------------------------------------------------------------
// MT-003 acceptance: every lane kind carries owner + exact five-field scope
// ---------------------------------------------------------------------------

/// AC: "All local/cloud/CLI/human/subagent/validator lane launches MUST resolve
/// through the Rust backend SwarmCoordinator/ModelRuntime adapter registry" and
/// HBR-PRIV-001/002/005: every launched model/session resource carries its owner
/// and exact five-field scope, and no foreign context can observe it.
#[tokio::test]
async fn model_lane_launch_all_lane_kinds_carry_owner_and_exact_scope() {
    let harness = Harness::create("all-lane-kinds").await;
    let registry = DexterityLaunchAdapterRegistry::standard();
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 256,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        // The default 5-minute lease is shorter than this test body. Each case allocates its
        // own embedded store (~177s to bootstrap) and then walks every adapter kind, so the
        // no-OS authority session was expiring mid-test and failing with "lease is expired"
        // before the assertions ran. Lease expiry has its own dedicated coverage; what this
        // test proves is that every lane kind carries owner and exact five-field scope, so the
        // lease is widened here rather than weakening the product default.
        SwarmConfig::new(RunBudget::defaulted(8)).with_lease_ttl(Duration::from_secs(3600)),
        Arc::new(DexterityLaunchProofFactory {
            ledger: ledger.clone(),
            loads: loads.clone(),
            unloads: unloads.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        harness.store.clone(),
    );

    let expected_scope = expected_scope_values(&harness.exact);
    let exact_reader = harness.exact_reader();
    let mut authority_instance_id = None;
    let mut covered = Vec::new();

    for (idx, adapter_kind) in supported_adapters().into_iter().enumerate() {
        let raw_launch = launch_request(adapter_kind.clone(), idx, ModelLaneStatus::Ready);
        let launch = registry
            .normalize(raw_launch.clone())
            .expect("registered Dexterity adapter normalizes");

        let (run, lane): (ModelLaneRunRecord, ModelLaneRecord) =
            if adapter_uses_no_os_runtime(&adapter_kind) {
                let caller = coordinator
                    .authorize_no_os_model_lane(
                        &raw_launch,
                        authority_instance_id.expect("a process-backed authority session is live"),
                    )
                    .expect("live Dexterity authority session issues the no-OS caller receipt");
                coordinator
                    .launch_no_os_model_lane(raw_launch, caller)
                    .await
                    .expect("no-OS Dexterity lane launches through SwarmCoordinator")
            } else {
                let spawn = spawn_request_for_adapter(
                    adapter_kind.clone(),
                    idx,
                    spawn_contract_from_normalized(&launch),
                );
                seed_cloud_launch_authority(&harness.store, &spawn, &adapter_kind, idx).await;
                let instance_id = spawn.instance_id;
                let spawned = coordinator
                    .spawn_session(spawn)
                    .await
                    .expect("process-backed Dexterity lane launches through SwarmCoordinator");
                assert_eq!(spawned, instance_id);
                assert!(
                    coordinator.session_runtime(instance_id).is_some(),
                    "the launched runtime is exposed only after the launch record commits"
                );
                authority_instance_id.get_or_insert(instance_id);
                let replay = harness
                    .store
                    .replay_run(&launch.run_id)
                    .await
                    .expect("spawn_session launch replay exists");
                assert_eq!(replay.lanes.len(), 1);
                (replay.run, replay.lanes.into_iter().next().unwrap())
            };

        // --- normalized launch-adapter output contract, every lane kind ---
        assert!(
            lane.event_ledger_event_id
                .starts_with(MODEL_LANE_EVENT_ID_PREFIX),
            "adapter {adapter_kind:?} lane must carry a canonical embedded EventLedger receipt: {}",
            lane.event_ledger_event_id
        );
        assert!(lane.event_ledger_seq > 0);
        assert_eq!(run.event_ledger_stream_id, launch.event_ledger_stream_id);
        assert_eq!(lane.event_ledger_stream_id, launch.event_ledger_stream_id);
        assert!(lane.capability_negotiation_ref.is_some());
        assert!(lane.provider_feature_profile_ref.is_some());
        assert!(lane.requested_execution_policy_ref.is_some());
        assert!(lane.effective_execution_policy_ref.is_some());
        assert!(lane.cancellation_ref.is_some());
        assert!(lane.reclaim_policy_ref.is_some());
        assert!(lane.terminal_status_mapping_ref.is_some());
        assert_eq!(lane.owner_session, OWNER_SESSION);
        assert_eq!(lane.trace_id, format!("trace-mt003-{idx}"));
        if matches!(
            lane.runtime_binding,
            RuntimeBinding::Local | RuntimeBinding::Cloud | RuntimeBinding::CliBridge
        ) {
            assert!(lane
                .process_ownership_ref
                .as_deref()
                .expect("process-backed lane has an ownership ref")
                .starts_with("process-ledger://"));
            assert!(lane.no_os_process_reason_ref.is_none());
        } else {
            assert!(lane.process_ownership_ref.is_none());
            assert!(
                lane.no_os_process_reason_ref
                    .as_deref()
                    .expect("no-OS lane declares an explicit no-process equivalent")
                    .starts_with("no-os-process://"),
                "adapter {adapter_kind:?} must record an explicit no-OS-process equivalent"
            );
        }

        // --- HBR-PRIV-001/002: the launched resource is durably owner-stamped ---
        for aggregate_id in [run.run_id.as_str(), lane.lane_id.as_str()] {
            assert_eq!(
                harness.persisted_scope(aggregate_id).await,
                expected_scope,
                "adapter {adapter_kind:?} must stamp the server-owned five-field scope on {aggregate_id} before launch publication"
            );
        }
        // --- every scoped durable mutation appends the canonical EventLedger ---
        for (aggregate, event_id) in [
            ("ModelLaneRun", run.event_ledger_event_id.as_str()),
            ("ModelLane", lane.event_ledger_event_id.as_str()),
        ] {
            assert_eq!(
                harness.ledger_event_scope(event_id).await,
                expected_scope,
                "{aggregate} EventLedger event {event_id} lost exact launch attribution"
            );
        }
        let receipts = harness
            .store
            .test_scoped_authority_receipts(&launch.run_id, 16)
            .await
            .expect("scoped launch receipts");
        assert_eq!(
            receipts
                .iter()
                .map(|receipt| receipt.record_kind.as_str())
                .collect::<Vec<_>>(),
            vec!["run", "lane"],
            "adapter {adapter_kind:?} must append exactly one run and one lane event atomically"
        );

        // --- positive access: the exact launch owner sees its own lane ---
        exact_reader
            .replay_run(&launch.run_id)
            .await
            .expect("the exact launch owner must replay its own lane");
        exact_reader
            .navigation_by_lane(&lane.lane_id)
            .await
            .expect("the exact launch owner must navigate its own live lane");
        exact_reader
            .navigation_by_lookup(ModelLaneNavigationLookup {
                event_ledger_event_id: Some(lane.event_ledger_event_id.clone()),
                ..Default::default()
            })
            .await
            .expect("the exact launch owner must navigate its own EventLedger authority");

        // --- HBR-PRIV-005: one-dimension foreign contexts observe absence ---
        for (dimension, foreign) in foreign_exact_scopes(&harness.exact) {
            let foreign_store = harness.reader_for(foreign);
            assert_absent_without_scope_leak(
                foreign_store.replay_run(&launch.run_id).await,
                dimension,
                &harness.exact,
            );
            assert_absent_without_scope_leak(
                foreign_store.navigation_by_lane(&lane.lane_id).await,
                dimension,
                &harness.exact,
            );
            let denial = assert_absent_without_scope_leak(
                foreign_store
                    .navigation_by_lookup(ModelLaneNavigationLookup {
                        event_ledger_event_id: Some(lane.event_ledger_event_id.clone()),
                        ..Default::default()
                    })
                    .await,
                dimension,
                &harness.exact,
            );
            assert!(
                !denial.contains(&run.run_id) && !denial.contains(&lane.lane_id),
                "foreign {dimension} EventLedger lookup leaked hidden launch metadata: {denial}"
            );
        }
        covered.push(adapter_kind);
    }

    assert_eq!(
        covered,
        supported_adapters(),
        "every declared lane kind must be launched and proven in order"
    );
    coordinator
        .drain_all()
        .await
        .expect("drain the all-lane-kind proof");
    harness.cleanup().await;
}

// ---------------------------------------------------------------------------
// MT-003 acceptance: HBR-PRIV-006 revocation / context switch on a live launch
// ---------------------------------------------------------------------------

/// AC (V5 remediation, `still_owed`): "HBR-PRIV-006 revocation/context-switch
/// behaviour for a live launch". A foreign context switch must not retarget or
/// terminate a live launch, and an authorized revocation must terminate it
/// without mutating its immutable original attribution.
#[tokio::test]
async fn model_lane_launch_revocation_and_context_switch_fail_closed_on_a_live_launch() {
    let harness = Harness::create("live-revocation").await;
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 128,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(DexterityLaunchProofFactory {
            ledger: ledger.clone(),
            loads: Arc::new(AtomicUsize::new(0)),
            unloads: Arc::new(AtomicUsize::new(0)),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        harness.store.clone(),
    );

    let idx = 0usize;
    let raw = launch_request(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        idx,
        ModelLaneStatus::Ready,
    );
    let normalized = DexterityLaunchAdapterRegistry::standard()
        .normalize(raw)
        .expect("cloud launch normalizes");
    let spawn = spawn_request_for_adapter(
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        idx,
        spawn_contract_from_normalized(&normalized),
    );
    seed_cloud_launch_authority(
        &harness.store,
        &spawn,
        &DexterityLaunchAdapterKind::ByokCloudOpenAi,
        idx,
    )
    .await;
    let instance_id = spawn.instance_id;
    let contract = spawn
        .dexterity_launch
        .as_ref()
        .expect("cloud launch retains its contract")
        .clone();
    coordinator
        .spawn_session(spawn)
        .await
        .expect("live cloud launch commits");
    assert!(
        coordinator.session_runtime(instance_id).is_some(),
        "the launch under test must actually be live"
    );
    let scope_before = harness.persisted_scope(&contract.lane_id).await;
    assert_eq!(scope_before, expected_scope_values(&harness.exact));

    // A live context switch must fail closed: it can neither terminate the
    // lane, retarget its attribution, nor disturb the owner's live runtime.
    for (dimension, foreign) in foreign_exact_scopes(&harness.exact) {
        let switched = harness.writer_for(&foreign);
        assert_absent_without_scope_leak(
            switched
                .record_lane_terminal_status(
                    &contract.lane_id,
                    ModelLaneStatus::Cancelled,
                    "a foreign context must not terminate an exact-owner launch",
                )
                .await,
            dimension,
            &harness.exact,
        );
        assert_cloud_authority_denied_without_scope_leak(
            switched
                .test_commit_cloud_consent_revocation(
                    &contract
                        .consent_receipt_ref
                        .clone()
                        .expect("cloud launch has a consent receipt"),
                    "operator://mt003/foreign-revoke",
                    "a switched context must not revoke another account's consent",
                )
                .await,
            dimension,
            &harness.exact,
        );
        assert!(
            coordinator.session_runtime(instance_id).is_some(),
            "a live {dimension} switch must fail closed without cancelling the owner's runtime"
        );
        assert_eq!(
            harness.persisted_scope(&contract.lane_id).await,
            scope_before,
            "a rejected {dimension} switch must not retarget the live lane"
        );
    }

    // The authorized owner revokes: the live launch is terminated, its terminal
    // record stays readable to the owner, and its attribution is unchanged.
    let receipt_id = contract
        .consent_receipt_ref
        .clone()
        .expect("cloud launch has a consent receipt");
    let revoked = coordinator
        .revoke_cloud_consent_receipt(
            &receipt_id,
            "operator://mt003/scope-revocation",
            "MT-003 exact-scope live-launch revocation proof",
        )
        .await
        .expect("authorized revocation must cancel the exact live launch");
    assert_eq!(revoked.len(), 1, "single-lane consent revokes one launch");
    assert_eq!(revoked[0].status, ModelLaneStatus::Cancelled);
    assert!(
        coordinator.session_runtime(instance_id).is_none(),
        "a revoked launch must stop being usable"
    );
    assert_eq!(
        harness.persisted_scope(&contract.lane_id).await,
        scope_before,
        "revocation must terminate the launch without retargeting its immutable scope"
    );

    let exact_reader = harness.exact_reader();
    let replay = exact_reader
        .replay_run(&contract.run_id)
        .await
        .expect("the original exact owner retains its terminal audit record");
    assert_eq!(replay.lanes.len(), 1);
    assert_eq!(replay.lanes[0].status, ModelLaneStatus::Cancelled);
    let terminal_event_id = replay.lanes[0].event_ledger_event_id.clone();
    assert_eq!(
        harness.ledger_event_scope(&terminal_event_id).await,
        scope_before,
        "the terminal EventLedger event keeps the launch's exact attribution"
    );
    let consent = harness
        .store
        .replay_cloud_consent_authority(&contract.run_id)
        .await
        .expect("replay the revoked consent authority");
    assert_eq!(
        consent.consent_receipts[0].status,
        ModelLaneCloudConsentReceiptStatus::Revoked
    );
    assert!(!consent.consent_receipts[0].approved);

    // After revocation the terminal record is still owner-only.
    for (dimension, foreign) in foreign_exact_scopes(&harness.exact) {
        let foreign_store = harness.reader_for(foreign);
        assert_absent_without_scope_leak(
            foreign_store.replay_run(&contract.run_id).await,
            dimension,
            &harness.exact,
        );
        assert_absent_without_scope_leak(
            foreign_store
                .navigation_by_lookup(ModelLaneNavigationLookup {
                    event_ledger_event_id: Some(terminal_event_id.clone()),
                    ..Default::default()
                })
                .await,
            dimension,
            &harness.exact,
        );
    }
    harness.cleanup().await;
}

// ---------------------------------------------------------------------------
// MT-003 acceptance: launch-authority bypasses fail closed
// ---------------------------------------------------------------------------

/// AC: "Source/negative proof MUST fail closed for direct endpoint launch,
/// app/src/app-src-tauri launch authority, terminal-only launch state, and
/// unsupported tool capability." This gate is storage-free by construction: a
/// bypass must be rejected by the registry before any durable authority is
/// touched, so reaching the database at all would already be the defect.
#[tokio::test]
async fn model_lane_launch_rejects_direct_endpoint_frontend_tauri_and_terminal_bypass() {
    let registry = DexterityLaunchAdapterRegistry::standard();
    for adapter_kind in [
        DexterityLaunchAdapterKind::DirectEndpoint,
        DexterityLaunchAdapterKind::FrontendAppSrc,
        DexterityLaunchAdapterKind::AppSrcTauri,
        DexterityLaunchAdapterKind::TerminalOnly,
        DexterityLaunchAdapterKind::ExternalCompat,
    ] {
        let err = registry
            .normalize(launch_request(
                adapter_kind.clone(),
                90,
                ModelLaneStatus::Ready,
            ))
            .expect_err("bypass adapter must fail before persistence");
        assert!(
            err.to_string().contains("bypass") || err.to_string().contains("external_compat"),
            "unexpected bypass error for {adapter_kind:?}: {err}"
        );
    }

    let mut unsupported = launch_request(
        DexterityLaunchAdapterKind::LocalModelRuntime,
        91,
        ModelLaneStatus::Ready,
    );
    unsupported.requested_tool_capability_tokens =
        vec!["tool-capability://unsupported-shell".into()];
    let err = registry
        .normalize(unsupported)
        .expect_err("unsupported tool capability must fail closed");
    assert!(err.to_string().contains("unsupported tool capability"));

    // A Dexterity launch without durable ModelLane authority must fail before
    // the factory runs, so no lane can exist only in process memory.
    let (ledger, _drain) =
        LedgerBatcher::manual_for_tests(LedgerBatcherConfig::default(), Arc::new(NoopOverflowSink))
            .expect("manual process ledger");
    let calls = Arc::new(AtomicUsize::new(0));
    let coordinator = SwarmCoordinator::new(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(CountingFactory {
            calls: calls.clone(),
        }),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
    );
    let normalized = registry
        .normalize(launch_request(
            DexterityLaunchAdapterKind::LocalModelRuntime,
            92,
            ModelLaneStatus::Ready,
        ))
        .expect("a supported adapter normalizes");
    let err = coordinator
        .spawn_session(spawn_request_for_adapter(
            DexterityLaunchAdapterKind::LocalModelRuntime,
            92,
            spawn_contract_from_normalized(&normalized),
        ))
        .await
        .expect_err("a Dexterity launch without ModelLaneStore must fail before the factory");
    assert!(matches!(err, SwarmError::LedgerFailed(_)));
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "the factory must never run for a launch that has no durable authority"
    );
}

// ---------------------------------------------------------------------------
// Launch-adapter fixtures
// ---------------------------------------------------------------------------

fn supported_adapters() -> Vec<DexterityLaunchAdapterKind> {
    vec![
        DexterityLaunchAdapterKind::LocalModelRuntime,
        DexterityLaunchAdapterKind::ByokCloudOpenAi,
        DexterityLaunchAdapterKind::ByokCloudAnthropic,
        DexterityLaunchAdapterKind::OfficialCliBridge,
        DexterityLaunchAdapterKind::CliBridge,
        DexterityLaunchAdapterKind::HumanOperator,
        DexterityLaunchAdapterKind::Subagent,
        DexterityLaunchAdapterKind::Validator,
    ]
}

fn adapter_uses_no_os_runtime(adapter_kind: &DexterityLaunchAdapterKind) -> bool {
    matches!(
        adapter_kind,
        DexterityLaunchAdapterKind::HumanOperator
            | DexterityLaunchAdapterKind::Subagent
            | DexterityLaunchAdapterKind::Validator
    )
}

fn spawn_request_for_adapter(
    adapter_kind: DexterityLaunchAdapterKind,
    idx: usize,
    contract: DexterityLaunchContract,
) -> SpawnRequest {
    let request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 100 + idx as u32),
        RuntimeAdapterBinding::LlamaCpp,
        OWNER_SESSION,
        format!("coordinator-session-mt003-{idx}"),
    )
    .with_wp(WP_ID)
    .with_mt(MT_ID)
    .with_dexterity_launch(contract);
    match adapter_kind {
        DexterityLaunchAdapterKind::LocalModelRuntime => request,
        DexterityLaunchAdapterKind::ByokCloudOpenAi => request
            .with_cloud_provider(ProviderKind::ByokCloud, "gpt-4o")
            .with_byok_cloud_provider(ByokCloudProvider::OpenAi),
        DexterityLaunchAdapterKind::ByokCloudAnthropic => request
            .with_cloud_provider(ProviderKind::ByokCloud, "claude-sonnet-4")
            .with_byok_cloud_provider(ByokCloudProvider::Anthropic),
        DexterityLaunchAdapterKind::OfficialCliBridge => request
            .with_cloud_provider(ProviderKind::OfficialCli, "claude-sonnet")
            .with_sandbox_posture(
                handshake_core::sandbox::TrustClass::Trusted,
                handshake_core::sandbox::IsolationTier::Tier1Container,
                std::collections::BTreeSet::from([
                    handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
                ]),
                handshake_core::sandbox::NetPolicy::HostInherited,
                // Official-CLI / CLI-bridge lanes MUST carry a resolvable,
                // descriptor-matching requested execution-policy ref; the
                // preflight rejects unknown or stale refs.
                handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
            ),
        DexterityLaunchAdapterKind::CliBridge => request
            .with_cloud_provider(ProviderKind::OfficialCli, "generic-cli-bridge-model")
            .with_sandbox_posture(
                handshake_core::sandbox::TrustClass::Trusted,
                handshake_core::sandbox::IsolationTier::Tier1Container,
                std::collections::BTreeSet::from([
                    handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
                ]),
                handshake_core::sandbox::NetPolicy::HostInherited,
                handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
            ),
        other => panic!("adapter {other:?} is not process-backed"),
    }
}

fn launch_request(
    adapter_kind: DexterityLaunchAdapterKind,
    idx: usize,
    status: ModelLaneStatus,
) -> DexterityLaunchAdapterRequest {
    let process_backed = matches!(
        adapter_kind,
        DexterityLaunchAdapterKind::LocalModelRuntime
            | DexterityLaunchAdapterKind::ByokCloudOpenAi
            | DexterityLaunchAdapterKind::ByokCloudAnthropic
            | DexterityLaunchAdapterKind::OfficialCliBridge
            | DexterityLaunchAdapterKind::CliBridge
    );
    let cloud = matches!(
        adapter_kind,
        DexterityLaunchAdapterKind::ByokCloudOpenAi
            | DexterityLaunchAdapterKind::ByokCloudAnthropic
    );
    let candidate_model_id = match &adapter_kind {
        DexterityLaunchAdapterKind::ByokCloudOpenAi => "model://dexterity/byok_cloud/gpt-4o".into(),
        DexterityLaunchAdapterKind::ByokCloudAnthropic => {
            "model://dexterity/byok_cloud/claude-sonnet-4".into()
        }
        _ => format!("model://mt003/candidate/{idx}"),
    };
    DexterityLaunchAdapterRequest {
        adapter_kind,
        run_id: format!("run-mt003-{idx}"),
        lane_id: format!("lane-mt003-{idx}"),
        trace_id: format!("trace-mt003-{idx}"),
        run_span_id: format!("span-run-mt003-{idx}"),
        lane_span_id: format!("span-lane-mt003-{idx}"),
        coordinator_session_id: "coordinator-session-mt003".into(),
        routing_policy: "dexterity_registry_normalized".into(),
        context_bundle_id: format!("context-bundle://mt003/{idx}"),
        event_ledger_stream_id: format!("event-ledger://mt003/{idx}"),
        artifact_namespace: format!("artifact://mt003/{idx}"),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: OWNER_SESSION.into(),
        locus_binding_ref: format!("locus://wp1/mt003/{idx}"),
        role: format!("lane-role-{idx}"),
        backend: None,
        adapter_id: None,
        model_id: Some(format!("model://mt003/{idx}")),
        session_id: format!("session-mt003-{idx}"),
        model_session_id: format!("model-session-mt003-{idx}"),
        extra_capability_token_ids: vec![],
        requested_tool_capability_tokens: vec!["tool-capability://read-context".into()],
        effective_capability_snapshot_ref: None,
        capability_negotiation_ref: None,
        provider_feature_profile_ref: None,
        requested_execution_policy_ref: None,
        effective_execution_policy_ref: None,
        projection_plan_ref: cloud.then_some(format!("projection-plan://mt003/{idx}")),
        consent_receipt_ref: cloud.then_some(format!("consent://mt003/{idx}")),
        tool_gate_decision_refs: vec![format!("toolgate://mt003/{idx}/allow-read-context")],
        status: Some(status),
        heartbeat_at_utc: Some("2026-06-29T00:00:00Z".into()),
        lease_expires_at_utc: Some("2026-06-29T00:05:00Z".into()),
        reclaim_after_utc: Some("2026-06-29T00:06:00Z".into()),
        restart_generation: 0,
        cancellation_ref: None,
        reclaim_policy_ref: None,
        terminal_status_mapping_ref: None,
        process_ownership_ref: process_backed.then_some(format!("process-ledger://mt003/{idx}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some(format!("loop-counter://mt003/{idx}")),
        last_runtime_status_ref: Some(format!("runtime-status://mt003/{idx}")),
        last_recovery_event_ref: Some(format!("recovery://mt003/{idx}")),
        startup_failure_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        run_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#run".into()),
        lane_recovery_hint_ref: Some("usermanual://model-lane-launch-adapters#lane".into()),
        memory_pack_ref: "memory-pack://fems/mt003".into(),
        memory_pack_hash: sample_sha256(),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt003".into(),
        selected_model_id: None,
        candidate_model_ids: vec![candidate_model_id],
        procedural_review_status: "preflight_reviewed_and_registry_normalized".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec![],
    }
}

fn spawn_contract_from_normalized(launch: &DexterityNormalizedLaunch) -> DexterityLaunchContract {
    DexterityLaunchContract {
        run_id: launch.run_id.clone(),
        lane_id: launch.lane_id.clone(),
        restart_generation: launch.restart_generation,
        trace_id: launch.trace_id.clone(),
        run_span_id: launch.run_span_id.clone(),
        lane_span_id: launch.lane_span_id.clone(),
        routing_policy: launch.routing_policy.clone(),
        context_bundle_id: launch.context_bundle_id.clone(),
        event_ledger_stream_id: launch.event_ledger_stream_id.clone(),
        artifact_namespace: launch.artifact_namespace.clone(),
        task_board_id: launch
            .task_board_id
            .clone()
            .expect("MT-003 normalized launch includes task board"),
        locus_binding_ref: launch.locus_binding_ref.clone(),
        role: launch.role.clone(),
        backend: launch.backend.clone(),
        adapter_id: launch.adapter_id.clone(),
        capability_token_ids: launch.capability_token_ids.clone(),
        effective_capability_snapshot_ref: launch
            .effective_capability_snapshot_ref
            .clone()
            .expect("MT-003 normalized launch includes capability snapshot"),
        projection_plan_ref: launch.projection_plan_ref.clone(),
        consent_receipt_ref: launch.consent_receipt_ref.clone(),
        tool_gate_decision_refs: launch.tool_gate_decision_refs.clone(),
        memory_pack_ref: launch.memory_pack_ref.clone(),
        memory_pack_hash: launch.memory_pack_hash.clone(),
        determinism_mode: launch.determinism_mode.clone(),
        budget_summary_ref: launch.budget_summary_ref.clone(),
        candidate_model_ids: launch.candidate_model_ids.clone(),
        procedural_review_status: launch.procedural_review_status.clone(),
        truncation_warning_ref: launch.truncation_warning_ref.clone(),
        rejection_reason_refs: launch.rejection_reason_refs.clone(),
        run_recovery_hint_ref: launch.run_recovery_hint_ref.clone(),
        lane_recovery_hint_ref: launch.lane_recovery_hint_ref.clone(),
    }
}

fn sample_sha256() -> String {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()
}

/// Seeds the durable ProjectionPlan + ConsentReceipt authority a BYOK cloud
/// lane needs before it may launch. Non-cloud adapters need no authority and
/// return unchanged, which keeps the missing-authority denial meaningful.
async fn seed_cloud_launch_authority(
    store: &ModelLaneStore,
    spawn: &SpawnRequest,
    adapter_kind: &DexterityLaunchAdapterKind,
    idx: usize,
) {
    let (provider_kind, cloud_model_name) = match adapter_kind {
        DexterityLaunchAdapterKind::ByokCloudOpenAi => ("openai", "gpt-4o"),
        DexterityLaunchAdapterKind::ByokCloudAnthropic => ("anthropic", "claude-sonnet-4"),
        _ => return,
    };
    let contract = spawn
        .dexterity_launch
        .as_ref()
        .expect("cloud launch has a Dexterity contract");
    let projection_plan_id = contract
        .projection_plan_ref
        .clone()
        .expect("cloud launch has a ProjectionPlan ref");
    let consent_receipt_id = contract
        .consent_receipt_ref
        .clone()
        .expect("cloud launch has a ConsentReceipt ref");
    let model_session_id = dexterity_spawn_model_session_id(spawn);
    let requested_model_id = format!("model://dexterity/byok_cloud/{cloud_model_name}");
    let scope_hash = sample_sha256();
    let fan_out_targets = vec![format!("provider://{provider_kind}/byok")];

    let plan = store
        .record_cloud_projection_plan(NewModelLaneCloudProjectionPlan {
            projection_plan_id: projection_plan_id.clone(),
            run_id: contract.run_id.clone(),
            trace_id: contract.trace_id.clone(),
            lane_id: Some(contract.lane_id.clone()),
            model_session_id: Some(model_session_id.clone()),
            provider_kind: Some(provider_kind.into()),
            requested_model_id: Some(requested_model_id.clone()),
            scope_hash: scope_hash.clone(),
            source_artifact_refs: vec![format!("artifact-store://mt003/{idx}/cloud-context.json")],
            payload_artifact_ref: format!("artifact-store://mt003/{idx}/cloud-payload.json"),
            payload_sha256: sample_sha256(),
            redaction_policy_ref: "redaction-policy://mt003/cloud-safe".into(),
            redaction_summary: "workspace-local secrets and local-only memory are excluded".into(),
            retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
            export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
            provider_profile_ref: contract.adapter_id.clone(),
            fan_out_targets: fan_out_targets.clone(),
            export_delegation: CloudExportDelegation {
                audience_refs: fan_out_targets.clone(),
                source_scope: AccountBoundAuthority::from_access(store.access()),
                authorization_receipt_ref: None,
            },
            consent_scope: ModelLaneCloudConsentScope::SingleLane,
            target_bindings: vec![],
            status: ModelLaneCloudProjectionPlanStatus::Active,
            event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
            work_packet_id: spawn
                .wp_id
                .clone()
                .expect("cloud launch has a work packet binding"),
            micro_task_id: spawn
                .mt_id
                .clone()
                .expect("cloud launch has a micro-task binding"),
            task_board_id: contract.task_board_id.clone(),
            owner_session: spawn.owner_role.clone(),
            idempotency_key: format!("idem-projection-mt003-{idx}"),
            created_at_utc: "2026-09-04T00:00:00Z".into(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
            diagnostic_payload: json!({
                "flight_recorder": "EventLedger",
                "provider_call_attempted": false,
                "locus": contract.locus_binding_ref,
            }),
        })
        .await
        .expect("record MT-003 cloud ProjectionPlan authority");

    store
        .record_cloud_consent_receipt(NewModelLaneCloudConsentReceipt {
            consent_receipt_id,
            projection_plan_id,
            projection_plan_hash: plan.projection_plan_hash,
            run_id: contract.run_id.clone(),
            trace_id: contract.trace_id.clone(),
            lane_id: Some(contract.lane_id.clone()),
            model_session_id: Some(model_session_id),
            provider_kind: Some(provider_kind.into()),
            requested_model_id: Some(requested_model_id),
            scope_hash,
            consent_scope: ModelLaneCloudConsentScope::SingleLane,
            target_bindings: vec![],
            retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
            export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
            fan_out_targets,
            approved: true,
            approver: AccountBoundAuthority::from_access(store.access()),
            approved_by_ref: "operator://mt003/approval".into(),
            approved_at_utc: "2026-09-04T00:00:10Z".into(),
            valid_from_utc: "2026-01-01T00:00:00Z".into(),
            valid_until_utc: "2027-01-01T00:00:00Z".into(),
            revoked_at_utc: None,
            revocation_ref: None,
            revocation_input_hash: None,
            status: ModelLaneCloudConsentReceiptStatus::Approved,
            event_ledger_stream_id: contract.event_ledger_stream_id.clone(),
            work_packet_id: spawn
                .wp_id
                .clone()
                .expect("cloud launch has a work packet binding"),
            micro_task_id: spawn
                .mt_id
                .clone()
                .expect("cloud launch has a micro-task binding"),
            task_board_id: contract.task_board_id.clone(),
            owner_session: spawn.owner_role.clone(),
            idempotency_key: format!("idem-consent-mt003-{idx}"),
            created_at_utc: "2026-09-04T00:00:15Z".into(),
            user_manual_behavior_ref: "usermanual://model-lane-cloud-projection-consent#launch"
                .into(),
            diagnostic_payload: json!({
                "flight_recorder": "EventLedger",
                "provider_call_attempted": false,
                "locus": contract.locus_binding_ref,
            }),
        })
        .await
        .expect("record MT-003 cloud ConsentReceipt authority");
}

async fn prove_generation(
    coordinator: &SwarmCoordinator,
    instance_id: ModelInstanceId,
    lane_label: &str,
) {
    let model_id = coordinator
        .session_model_id(instance_id)
        .expect("the factory publishes its runtime-minted model identity");
    let request = GenerateRequest {
        id: model_id,
        prompt: GenPrompt::new("Reply with one short token."),
        sampling: SamplingParams::default(),
        lora_overrides: Vec::new(),
        steering_overrides: Vec::new(),
        kv_prefix_handle: None,
        cancel: CancellationToken::new(),
        max_tokens: 1,
        stop_sequences: Vec::new(),
        speculative_mode: None,
        structured_decoding: None,
    };
    let mut stream = coordinator
        .generate_session_managed(instance_id, request)
        .unwrap_or_else(|error| panic!("{lane_label} generation must start: {error}"));
    let mut observed_tokens = 0usize;
    while let Some(token) = stream.next().await {
        token.unwrap_or_else(|error| panic!("{lane_label} generation must complete: {error}"));
        observed_tokens += 1;
    }
    assert!(
        observed_tokens > 0,
        "{lane_label} generation returned no runtime output"
    );
}

// ---------------------------------------------------------------------------
// Deterministic no-egress runtime seams
// ---------------------------------------------------------------------------

struct DexterityLaunchProofFactory {
    ledger: LedgerBatcher,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for DexterityLaunchProofFactory {
    async fn create(&self, request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        let record_id = ProcessOwnershipRecordId::new_v7();
        let os_pid = 56000 + request.instance_id.instance;
        let start = ProcessStart::new(
            proof_process_engine_kind(request),
            request.owner_role.clone(),
            request.owner_wp.clone(),
        )
        .with_process_uuid(record_id.as_uuid())
        .with_os_pid(os_pid)
        .with_parent_session_id(request.parent_session_id.clone())
        .with_wp_id(request.wp_id.clone().unwrap_or_default())
        .with_mt_id(request.mt_id.clone().unwrap_or_default());
        self.ledger
            .record_start(start)
            .map_err(|err| SwarmError::LedgerFailed(err.to_string()))?;

        let mut owned_runtime =
            DexterityLaunchProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let model_id = owned_runtime
            .load(dexterity_load_spec_for_request(request))
            .await
            .map_err(|err| SwarmError::FactoryFailed(err.to_string()))?;
        let owned_runtime = Arc::new(tokio::sync::Mutex::new(owned_runtime));
        let shared_runtime =
            DexterityLaunchProofRuntime::new(self.loads.clone(), self.unloads.clone());
        let teardown: handshake_core::swarm_orchestration::SessionTeardown = Arc::new(move || {
            let owned_runtime = Arc::clone(&owned_runtime);
            Box::pin(async move {
                owned_runtime
                    .lock()
                    .await
                    .unload(model_id)
                    .await
                    .map_err(|err| SwarmError::Internal(err.to_string()))
            })
        });
        Ok(LiveSession::new(
            Arc::new(shared_runtime),
            model_id,
            CancellationToken::new(),
            teardown,
            record_id,
            os_pid,
        ))
    }
}

/// Records whether the session factory was reached at all. Every bypass and
/// missing-authority path must leave this at zero.
struct CountingFactory {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelSessionFactory for CountingFactory {
    async fn create(&self, _request: &SpawnRequest) -> Result<LiveSession, SwarmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(SwarmError::FactoryFailed(
            "CountingFactory must not be reached by a fail-closed launch preflight".into(),
        ))
    }
}

fn proof_process_engine_kind(request: &SpawnRequest) -> ProcessEngineKind {
    match request.provider {
        Some(ProviderKind::OfficialCli) => ProcessEngineKind::OfficialCliBridge,
        Some(ProviderKind::ByokCloud) => ProcessEngineKind::HelperSubprocess,
        Some(ProviderKind::ExternalCompat) => ProcessEngineKind::ExternalCompat,
        Some(ProviderKind::Local) | None => match request.runtime_binding {
            RuntimeAdapterBinding::Candle => ProcessEngineKind::Candle,
            RuntimeAdapterBinding::LlamaCpp => ProcessEngineKind::LlamaCpp,
        },
    }
}

/// Deterministic cloud seam. It reads only a generated inert credential from an
/// isolated in-process vault and returns a controllable local runtime; no HTTP
/// client, subprocess, or provider endpoint is ever constructed, so
/// `provider_calls` is a real observation rather than an assumption.
struct NoEgressCloudBuilder {
    provider: ProviderKind,
    vault: Arc<dyn SecretsVault>,
    lane: String,
    builds: Arc<AtomicUsize>,
    provider_calls: Arc<AtomicUsize>,
}

impl NoEgressCloudBuilder {
    fn new(
        provider: ProviderKind,
        vault: Arc<dyn SecretsVault>,
        lane: impl Into<String>,
        builds: Arc<AtomicUsize>,
        provider_calls: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            provider,
            vault,
            lane: lane.into(),
            builds,
            provider_calls,
        }
    }
}

#[async_trait]
impl CloudRuntimeBuilder for NoEgressCloudBuilder {
    fn provider(&self) -> ProviderKind {
        self.provider
    }

    async fn build_loaded(
        &self,
        model_name: &str,
        _invocation_context: Option<handshake_core::model_runtime::cloud::CliInvocationContext>,
        _working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        self.vault
            .get(&self.lane)
            .map_err(|error| format!("isolated inert test credential unavailable: {error}"))?;
        self.builds.fetch_add(1, Ordering::SeqCst);
        let mut runtime = DexterityLaunchProofRuntime::new_with_provider_calls(
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
            self.provider_calls.clone(),
        );
        let model_id = runtime
            .load(LoadSpec {
                artifact_path: std::path::PathBuf::new(),
                sha256_expected: String::new(),
                runtime_kind: RuntimeKind::LlamaCpp,
                sampling_defaults: SamplingParams::default(),
                kv_cache_policy: KvCachePolicy::default(),
                declared_capabilities: ModelCapabilities::default(),
                provider: self.provider,
                engine_origin: Some(model_name.to_string()),
                external_engine_import: None,
            })
            .await
            .map_err(|error| format!("no-egress cloud runtime load failed: {error}"))?;
        Ok(CloudLiveRuntime {
            runtime: Arc::new(runtime),
            model_id,
        })
    }
}

struct DexterityLaunchProofRuntime {
    capabilities: ModelCapabilities,
    kv: KvCacheHandle,
    lora: LoraStackHandle,
    steering: SteeringHookHandle,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
    provider_calls: Option<Arc<AtomicUsize>>,
}

impl DexterityLaunchProofRuntime {
    fn new(loads: Arc<AtomicUsize>, unloads: Arc<AtomicUsize>) -> Self {
        Self::new_with_optional_provider_calls(loads, unloads, None)
    }

    fn new_with_provider_calls(
        loads: Arc<AtomicUsize>,
        unloads: Arc<AtomicUsize>,
        provider_calls: Arc<AtomicUsize>,
    ) -> Self {
        Self::new_with_optional_provider_calls(loads, unloads, Some(provider_calls))
    }

    fn new_with_optional_provider_calls(
        loads: Arc<AtomicUsize>,
        unloads: Arc<AtomicUsize>,
        provider_calls: Option<Arc<AtomicUsize>>,
    ) -> Self {
        Self {
            capabilities: ModelCapabilities::default(),
            kv: KvCacheHandle::new("dexterity-mt003-kv"),
            lora: LoraStackHandle::new("dexterity-mt003-lora"),
            steering: SteeringHookHandle::new("dexterity-mt003-steering"),
            loads,
            unloads,
            provider_calls,
        }
    }
}

#[async_trait]
impl ModelRuntime for DexterityLaunchProofRuntime {
    async fn load(&mut self, _spec: LoadSpec) -> Result<ModelId, ModelRuntimeError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        self.unloads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn generate(&self, req: GenerateRequest) -> TokenStream {
        if let Some(provider_calls) = &self.provider_calls {
            provider_calls.fetch_add(1, Ordering::SeqCst);
        }
        let cancel = req.cancel.clone();
        let items = (0..req.max_tokens.min(2)).map(move |i| {
            if cancel.is_cancelled() {
                Err(ModelRuntimeError::Cancelled)
            } else {
                Ok(handshake_core::model_runtime::GeneratedToken {
                    token_id: i,
                    text: format!("dexterity-mt003-token-{i}"),
                    logprob: None,
                    finish_reason: None,
                })
            }
        });
        Box::pin(stream::iter(items.collect::<Vec<_>>()))
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: vec![],
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: vec![] })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Ok(&self.capabilities)
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Ok(self.kv.clone())
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Ok(self.lora.clone())
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Ok(self.steering.clone())
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}

fn dexterity_load_spec_for_request(request: &SpawnRequest) -> LoadSpec {
    LoadSpec {
        artifact_path: "mt003-proof-model.gguf".into(),
        sha256_expected: sample_sha256(),
        runtime_kind: RuntimeKind::LlamaCpp,
        sampling_defaults: SamplingParams::default(),
        kv_cache_policy: KvCachePolicy::default(),
        declared_capabilities: ModelCapabilities::default(),
        provider: request.provider.unwrap_or(ProviderKind::Local),
        engine_origin: Some("dexterity-mt003-proof-runtime".into()),
        external_engine_import: None,
    }
}
