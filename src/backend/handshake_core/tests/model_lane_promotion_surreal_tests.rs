//! MT-004 PromotionGate authority over one embedded SurrealDB namespace/database.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{
    RowFilter, ScalarValue, SurrealStorage, SurrealTestInspector,
};
use handshake_core::swarm_orchestration::model_lane::{
    LaunchAuthority, ModelLaneAuthority, ModelLaneAuthorityTestCorruption, ModelLaneKind,
    ModelLaneLocusBinding, ModelLaneMessageKind, ModelLanePromotionDenialReason,
    ModelLanePromotionOutcome, ModelLanePromotionState, ModelLaneProviderKind,
    ModelLaneRecoveryState, ModelLaneRoutingMetadata, ModelLaneRoutingPolicy, ModelLaneStatus,
    ModelLaneStore, ModelLaneTarget, NewModelLane, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneMessage, NewModelLanePromotionDecision, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceScope, WorkspaceScopeRef,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use surreal_test_store_support::IsolatedSurrealTestStore;

use std::{collections::BTreeMap, sync::Arc};
use handshake_core::{AccountBootstrapState, capabilities::CapabilityRegistry};
use handshake_core::storage::{Database, surreal::SurrealDatabase};
use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction};
use handshake_core::test_harness::{ac9_role_authority::Ac9RoleAuthorityFixture,
    crdt_workspace::AuthenticatedModelLaneTestOperation};
use handshake_core::swarm_orchestration::model_lane::{
    FinalPromotionAuthoritySelectors, ModelLaneResult, ModelLanePromotionDecisionRecord,
    ModelLaneMessageRecord, ModelLaneReplay, ModelLaneNavigationProjection,
};
use tokio::sync::Mutex;
use futures::FutureExt;
use uuid::Uuid;

type PromotionAuthorities = Arc<Mutex<BTreeMap<String, HeldPromotionAuthority>>>;

struct HeldPromotionAuthority {
    operations: BTreeMap<String, Uuid>,
    selectors: FinalPromotionAuthoritySelectors,
    runtime: Option<Ac9RoleAuthorityFixture>,
}

struct AuthenticatedPromotionStore {
    store: ModelLaneStore,
    bootstrap: AccountBootstrapState,
    authorization: AuthorizationRequest,
    authorities: PromotionAuthorities,
}

impl AuthenticatedPromotionStore {
    async fn operation(&self, action: ResourceAction) -> ModelLaneResult<AuthenticatedModelLaneTestOperation> {
        let mut request = self.authorization.clone();
        request.action = action;
        request.capability_id = if action == ResourceAction::Read { "fs.read" } else { "fs.write" }.into();
        AuthenticatedModelLaneTestOperation::authorize(self.bootstrap.surreal.clone(), request,
            self.store.access().exact_read_scope().expect("complete configured test scope").clone()).await
    }

    async fn record_promotion_decision(&self, mut input: NewModelLanePromotionDecision)
        -> ModelLaneResult<ModelLanePromotionDecisionRecord> {
        let mut authorities = self.authorities.lock().await;
        let held = authorities.get_mut(&input.run_id).expect("genuine roles acquired before input mutation");
        if self.store.access().exact_read_scope() != Some(&held.selectors.operator.expectation.receipt_scope) {
            return Err(handshake_core::swarm_orchestration::model_lane::ModelLaneError::AuthorityDenied(
                "configured promotion store differs from the retained genuine owner scope".into()));
        }
        let operation_id = *held.operations.entry(input.decision_id.clone()).or_insert_with(Uuid::now_v7);
        input.operator_authority_ref = Some(held.selectors.operator.evidence_ref.as_str().into());
        input.validator_authority_ref = Some(held.selectors.validator.evidence_ref.as_str().into());
        match held.runtime.as_ref() {
            Some(runtime) => self.operation(ResourceAction::Create).await?
                .run_with_configured_registry(self.bootstrap.capability_registry.clone(),
                    runtime.record_promotion_decision(&self.bootstrap, self.authorization.clone(), operation_id, input)).await,
            None => self.store.retry_committed_promotion_decision(&self.bootstrap, self.authorization.clone(),
                operation_id, held.selectors.clone(), input).await,
        }
    }

    async fn record_message(&self, mut input: NewModelLaneMessage) -> ModelLaneResult<ModelLaneMessageRecord> {
        if input.authority == ModelLaneAuthority::Promoted {
            let authorities = self.authorities.lock().await;
            let held = authorities.get(&input.run_id).expect("original canonical promotion roles");
            input.operator_decision_ref = Some(held.selectors.operator.evidence_ref.as_str().into());
            input.validator_verdict_ref = Some(held.selectors.validator.evidence_ref.as_str().into());
        }
        self.operation(ResourceAction::Create).await?.run(self.store.record_message(input)).await
    }

    async fn replay_promotion_decisions(&self, run: &str) -> ModelLaneResult<Vec<ModelLanePromotionDecisionRecord>> {
        self.operation(ResourceAction::Read).await?.run_with_configured_registry(
            self.bootstrap.capability_registry.clone(), self.store.replay_promotion_decisions(run)).await
    }
    async fn replay_run(&self, run: &str) -> ModelLaneResult<ModelLaneReplay> {
        self.operation(ResourceAction::Read).await?.run(self.store.replay_run(run)).await
    }
    async fn navigation_by_run(&self, run: &str) -> ModelLaneResult<ModelLaneNavigationProjection> {
        self.operation(ResourceAction::Read).await?.run(self.store.navigation_by_run(run)).await
    }
    async fn navigation_by_lane(&self, lane: &str) -> ModelLaneResult<ModelLaneNavigationProjection> {
        self.operation(ResourceAction::Read).await?.run(self.store.navigation_by_lane(lane)).await
    }
    async fn navigation_by_message(&self, message: &str) -> ModelLaneResult<ModelLaneNavigationProjection> {
        self.operation(ResourceAction::Read).await?.run(self.store.navigation_by_message(message)).await
    }
    async fn test_corrupt_scoped_authority(&self, kind: &str, id: &str,
        corruption: ModelLaneAuthorityTestCorruption) -> ModelLaneResult<()> {
        self.operation(ResourceAction::Create).await?.run_with_configured_registry(
            self.bootstrap.capability_registry.clone(), self.store.test_corrupt_scoped_authority(kind, id, corruption)).await
    }
}

struct OpenHandle<T>(Option<T>);
impl<T> std::ops::Deref for OpenHandle<T> {
    type Target = T;
    fn deref(&self) -> &T { self.0.as_ref().expect("owned open handle outside close/reopen") }
}

struct PromotionFixtureFailure {
    primary: Option<Box<dyn std::any::Any + Send>>,
    cleanup: Option<Box<dyn std::any::Any + Send>>,
}

struct Harness {
    fixture: authenticated_promotion_fixture::PublicPromotionFixture,
    storage: OpenHandle<SurrealStorage>,
    scope: ResourceScope,
    store: OpenHandle<AuthenticatedPromotionStore>,
    authorities: PromotionAuthorities,
}

impl Harness {
    async fn create(_label: &str) -> Self {
        let mut fixture = authenticated_promotion_fixture::PublicPromotionFixture::create().await;
        let authenticated = std::panic::AssertUnwindSafe(fixture.authenticate_workspace()).catch_unwind().await;
        if let Err(primary) = authenticated {
            let cleanup = std::panic::AssertUnwindSafe(fixture.cleanup()).catch_unwind().await.err();
            std::panic::resume_unwind(Box::new(PromotionFixtureFailure { primary: Some(primary), cleanup }));
        }
        let storage = fixture.isolated.storage().clone();
        let scope = fixture.scope.as_ref().expect("publicly created workspace/session").clone();
        let authorities = Arc::new(Mutex::new(BTreeMap::new()));
        let store = AuthenticatedPromotionStore { store: fixture.scoped_store(),
            bootstrap: fixture.bootstrap.as_ref().expect("original bootstrap").clone(),
            authorization: fixture.authorization(), authorities: authorities.clone() };
        Self { fixture, storage: OpenHandle(Some(storage)), scope, store: OpenHandle(Some(store)), authorities }
    }

    async fn iteration(&self, label: &str) -> AuthenticatedPromotionStore {
        let scope = self.fixture.additional_workspace(label).await;
        let mut store = self.store_for(scope.clone());
        store.authorization.external_resource_id = scope.workspace.as_ref().unwrap().as_str().into();
        store
    }

    fn store_for(&self, scope: ResourceScope) -> AuthenticatedPromotionStore {
        AuthenticatedPromotionStore {
            store: ModelLaneStore::new_scoped_with_lifecycle(self.storage.clone(), scope,
                self.storage.resource_access_lifecycle()),
            bootstrap: self.fixture.bootstrap.as_ref().expect("original bootstrap").clone(),
            authorization: self.fixture.authorization(), authorities: self.authorities.clone(),
        }
    }

    async fn shutdown_roles(authorities: &PromotionAuthorities) {
        let mut held = authorities.lock().await;
        let mut failures = Vec::new();
        for entry in held.values_mut() {
            if let Some(runtime) = entry.runtime.take() {
                if let Err(error) = runtime.shutdown().await { failures.push(error.to_string()); }
            }
        }
        assert!(failures.is_empty(), "genuine role runtimes must drain: {failures:?}");
    }

    async fn reopen(&mut self) {
        let capability_registry = self.fixture.bootstrap.as_ref().expect("actual original bootstrap")
            .capability_registry.clone();
        Self::shutdown_roles(&self.authorities).await;
        drop(self.store.0.take());
        drop(self.storage.0.take());
        drop(self.fixture.bootstrap.take());
        let identity = (self.fixture.isolated.store_path().to_owned(),
            self.fixture.isolated.namespace().to_owned(), self.fixture.isolated.database().to_owned());
        self.fixture.isolated.reopen().await.expect("reopen same scope after all original handles drain");
        assert_eq!(identity, (self.fixture.isolated.store_path().to_owned(),
            self.fixture.isolated.namespace().to_owned(), self.fixture.isolated.database().to_owned()));
        let storage = self.fixture.isolated.storage().clone();
        let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(storage.clone()));
        self.fixture.bootstrap = Some(AccountBootstrapState::new(database, storage.clone(),
            capability_registry));
        self.store.0 = Some(AuthenticatedPromotionStore {
            store: ModelLaneStore::new_scoped_with_lifecycle(storage.clone(), self.scope.clone(),
                storage.resource_access_lifecycle()),
            bootstrap: self.fixture.bootstrap.as_ref().unwrap().clone(),
            authorization: self.fixture.authorization(), authorities: self.authorities.clone(),
    });
        self.storage.0 = Some(storage);
    }

    async fn cleanup(&mut self) {
        Self::shutdown_roles(&self.authorities).await;
        drop(self.store.0.take());
        drop(self.storage.0.take());
        self.fixture.cleanup().await;
    }
}

struct Seeded {
    proposal_version: i64,
}

#[tokio::test]
async fn model_lane_promotion_appends_eventledger_and_replays_decision() {
    let mut harness = Harness::create("promotion-positive").await;
    let result = std::panic::AssertUnwindSafe(async {
        let seeded = seed_authority(&harness.store, "promotion-positive", true).await;
        let decision = sample_decision(
            "promotion-positive",
            "decision-approved",
            "idem-approved",
            seeded.proposal_version,
        );
        let stored = harness
            .store
            .record_promotion_decision(decision.clone())
            .await
            .expect("record canonical PromotionGate decision");
        assert_eq!(stored.outcome, ModelLanePromotionOutcome::Approved);
        assert_eq!(stored.final_state, ModelLanePromotionState::Executed);
        assert_eq!(
            stored.state_history,
            vec![
                ModelLanePromotionState::Advisory,
                ModelLanePromotionState::PromotionRequested,
                ModelLanePromotionState::PendingPolicy,
                ModelLanePromotionState::PendingApproval,
                ModelLanePromotionState::Approved,
                ModelLanePromotionState::Executing,
                ModelLanePromotionState::Executed,
            ]
        );
        assert!(stored.event_ledger_seq > 0);
        assert_eq!(
            harness
                .store
                .record_promotion_decision(decision)
                .await
                .expect("identical decision retry"),
            stored
        );
        assert_eq!(
            harness
                .store
                .replay_promotion_decisions("run-mt004-promotion-positive")
                .await
                .expect("replay decisions"),
            vec![stored.clone()]
        );

        harness.reopen().await;
        assert_eq!(harness.store.replay_promotion_decisions("run-mt004-promotion-positive").await
            .expect("decision survives restart with original authenticated session"), vec![stored]);
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

#[tokio::test]
async fn model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation() {
    let mut harness = Harness::create("promotion-denials").await;
    let result = std::panic::AssertUnwindSafe(async {
        let seeded = seed_authority(&harness.store, "promotion-denials", true).await;

        let mut stale = sample_decision(
            "promotion-denials",
            "decision-stale",
            "idem-stale",
            seeded.proposal_version,
        );
        stale.base_snapshot_ref = "snapshot://stale".into();
        let stale = harness
            .store
            .record_promotion_decision(stale)
            .await
            .expect("stale base is a durable denial");
        assert_eq!(stale.outcome, ModelLanePromotionOutcome::Denied);
        assert_eq!(
            stale.denial_reason,
            Some(ModelLanePromotionDenialReason::InputRefMismatch)
        );

        let mut schema = sample_decision(
            "promotion-denials",
            "decision-schema",
            "idem-schema",
            seeded.proposal_version,
        );
        schema.schema_id = "hsk.model_lane_message@999".into();
        let schema = harness
            .store
            .record_promotion_decision(schema)
            .await
            .expect("schema mismatch is a durable denial");
        assert_eq!(
            schema.denial_reason,
            Some(ModelLanePromotionDenialReason::SchemaMismatch)
        );

        let mut direct = sample_decision(
            "promotion-denials",
            "decision-direct",
            "idem-direct",
            seeded.proposal_version,
        );
        direct.direct_authority_mutation_attempt_ref = Some("mutation://forbidden".into());
        let direct = harness
            .store
            .record_promotion_decision(direct)
            .await
            .expect("direct mutation is a durable denial");
        assert_eq!(
            direct.denial_reason,
            Some(ModelLanePromotionDenialReason::DirectAuthorityMutation)
        );
        assert_eq!(
            harness
                .store
                .replay_promotion_decisions("run-mt004-promotion-denials")
                .await
                .expect("replay durable denials")
                .len(),
            3
        );
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

#[tokio::test]
async fn model_lane_promotion_reordered_inputs_keep_same_decision_hash() {
    let mut harness = Harness::create("promotion-order").await;
    let result = std::panic::AssertUnwindSafe(async {
        let seeded = seed_authority(&harness.store, "promotion-order", true).await;
        let first = harness
            .store
            .record_promotion_decision(sample_decision(
                "promotion-order",
                "decision-order-a",
                "idem-order-a",
                seeded.proposal_version,
            ))
            .await
            .expect("first canonical decision");
        let mut reordered = sample_decision(
            "promotion-order",
            "decision-order-b",
            "idem-order-b",
            seeded.proposal_version,
        );
        reordered.input_refs.reverse();
        let second = harness
            .store
            .record_promotion_decision(reordered)
            .await
            .expect("reordered canonical decision");
        assert_eq!(first.canonical_input_refs, second.canonical_input_refs);
        assert_eq!(first.canonical_hash_basis, second.canonical_hash_basis);
        assert_eq!(
            first.canonical_decision_hash,
            second.canonical_decision_hash
        );
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

#[tokio::test]
async fn model_lane_promotion_preserves_exact_scope_and_denies_foreign_or_mixed_sources() {
    let mut harness = Harness::create("promotion-scope").await;
    let result = std::panic::AssertUnwindSafe(async {
        let seeded = seed_authority(&harness.store, "promotion-scope", true).await;
        let owner = harness
            .store
            .record_promotion_decision(sample_decision(
                "promotion-scope",
                "decision-owner",
                "idem-owner",
                seeded.proposal_version,
            ))
            .await
            .expect("owner decision");
        assert_eq!(owner.outcome, ModelLanePromotionOutcome::Approved);

        let inspector = harness.storage.test_inspector();
        let authority_rows = table_count(&inspector, "model_lane_authority").await;
        let promotion_events = promotion_event_count(&inspector).await;
        for (index, foreign_scope) in one_field_mismatches(&harness.scope).into_iter().enumerate() {
            let foreign_metadata = [foreign_scope.owner_account_id.to_string(),
                foreign_scope.actor_principal_id.to_string(),
                foreign_scope.authenticated_session.unwrap().to_string(),
                foreign_scope.access_space.unwrap().to_string(),
                foreign_scope.workspace.as_ref().unwrap().as_str().to_owned()];
            let foreign = harness.store_for(foreign_scope);
            let denial = foreign.record_promotion_decision(sample_decision(
                "promotion-scope", &format!("decision-foreign-{index}"),
                &format!("idem-foreign-{index}"), seeded.proposal_version,
            )).await.expect_err("forged configured tuple must fail before promotion publication");
            assert!(matches!(&denial, handshake_core::swarm_orchestration::model_lane::ModelLaneError::AuthorityDenied(_)));
            let rendered = denial.to_string();
            assert!(foreign_metadata.iter().all(|value| !rendered.contains(value)),
                "denial must not disclose forged or actual identity metadata");
            assert_eq!(table_count(&inspector, "model_lane_authority").await, authority_rows,
                "forged configured tuple must not publish protected promotion authority");
            assert_eq!(promotion_event_count(&inspector).await, promotion_events,
                "forged configured tuple must not append promotion events; auth audit remains permitted");
            assert_eq!(harness.store.replay_promotion_decisions("run-mt004-promotion-scope").await.unwrap(),
                vec![owner.clone()], "owner decision stays unchanged after every rejected selector");
        }
        drop(inspector);
        assert_eq!(
            harness
                .store
                .replay_promotion_decisions("run-mt004-promotion-scope")
                .await
                .expect("owner decision unchanged"),
            vec![owner]
        );
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

#[tokio::test]
async fn promotion_same_scope_cross_run_input_is_a_durable_typed_denial() {
    let mut harness = Harness::create("same-scope-mixed-run").await;
    let result = std::panic::AssertUnwindSafe(async {
        let seeded = seed_authority(&harness.store, "mixed-run-a", true).await;
        let operation = harness.store.operation(ResourceAction::Create).await.unwrap();
        operation.run(async {
            Ok(seed_advisory_inputs(&harness.store.store, "mixed-run-b", true).await)
        }).await.expect("canonical B messages under the same genuine authenticated tuple");
        drop(operation);
        let before_b = harness.store.replay_run("run-mt004-mixed-run-b").await.unwrap();
        let mut candidate = sample_decision("mixed-run-a", "decision-mixed-run", "idem-mixed-run", seeded.proposal_version);
        let selected = "model-lane-message://message-mt004-mixed-run-b-proposal".to_owned();
        candidate.input_refs = vec![selected.clone(), "model-lane-message://message-mt004-mixed-run-a-critique".into()];
        candidate.selected_input_refs = vec![selected];
        let denied = harness.store.record_promotion_decision(candidate.clone()).await
            .expect("genuine A roles consume a mixed-run command and persist its typed denial");
        assert_eq!(denied.outcome, ModelLanePromotionOutcome::Denied);
        assert_eq!(denied.denial_reason, Some(ModelLanePromotionDenialReason::InputRefMismatch));
        assert!(denied.event_ledger_seq > 0);
        assert!(!denied.event_ledger_event_id.is_empty());
        let settled = promotion_settlement_sets(&harness.storage.test_inspector()).await;
        for key in ["model_lane_authority/promotion_decision", "model_lane_authority/role_evidence_consumption",
            "kernel_event_ledger/promotion_decision", "kernel_event_ledger/role_evidence_consumption"] {
            assert!(!settled[key].as_array().expect("complete canonical family set").is_empty(),
                "positive settled families make retry nonmutation controls nonvacuous");
        }
        let original_operation = {
            let held = harness.authorities.lock().await;
            held["run-mt004-mixed-run-a"].operations["decision-mixed-run"]
        };
        let original_selectors = {
            let held = harness.authorities.lock().await;
            held["run-mt004-mixed-run-a"].selectors.clone()
        };
        assert!(settled["model_lane_authority/role_evidence_consumption"].as_array().unwrap().iter().any(|row| {
            let body: serde_json::Value = serde_json::from_str(row["values"]["record_json"].as_str()
                .expect("actual canonical consumption JSON string")).expect("parse actual consumption body");
            body["schema_id"] == "handshake.role-promotion-consumption@1"
                && body["operation_id"] == original_operation.to_string()
                && body["decision_id"] == "decision-mixed-run"
        }), "positive canonical sets include the actual original CURRENT consumption, not only issuance lineage");
        let unchanged_retry_input = candidate.clone();
        let original_registry = harness.store.bootstrap.capability_registry.clone();
        assert_eq!(harness.store.record_promotion_decision(candidate.clone()).await.unwrap(), denied,
            "identical retry retains original operation and canonical denial");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled,
            "same live-owner committed retry cannot create another decision, consumption or ledger row");
        assert_eq!(harness.store.replay_promotion_decisions("run-mt004-mixed-run-a").await.unwrap(), vec![denied.clone()],
            "canonical replay validates the decision and EventLedger link");
        assert_eq!(harness.store.replay_run("run-mt004-mixed-run-b").await.unwrap(), before_b,
            "cross-run denial must not mutate B's canonical data");
        harness.reopen().await;
        assert!(Arc::ptr_eq(&original_registry, &harness.store.bootstrap.capability_registry),
            "canonical reopen retains the exact actual configured registry Arc");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled,
            "actual handle drain and canonical reopen retain the complete settlement sets");
        {
            let held = harness.authorities.lock().await;
            assert!(held["run-mt004-mixed-run-a"].runtime.is_none(), "original actual role owner was shut down before reopen");
            assert_eq!(held["run-mt004-mixed-run-a"].operations["decision-mixed-run"], original_operation);
            assert_eq!(held["run-mt004-mixed-run-a"].selectors, original_selectors);
        }
        assert_eq!(harness.store.replay_promotion_decisions("run-mt004-mixed-run-a").await.unwrap(), vec![denied.clone()]);
        assert_eq!(harness.store.record_promotion_decision(candidate).await.unwrap(), denied,
            "same original role selectors and operation survive canonical reopen");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled,
            "retired-owner retry returns the unchanged settled result without new decisions, consumptions or receipts");

        assert_eq!(harness.store.replay_run("run-mt004-mixed-run-b").await.unwrap(), before_b);
        let mut exact_retry_input = unchanged_retry_input;
        exact_retry_input.operator_authority_ref = Some(original_selectors.operator.evidence_ref.as_str().into());
        exact_retry_input.validator_authority_ref = Some(original_selectors.validator.evidence_ref.as_str().into());
        let mut changed_input = exact_retry_input.clone();
        changed_input.diagnostic_payload = json!({"changed_committed_retry_input": true});
        let changed = harness.store.store.retry_committed_promotion_decision(&harness.store.bootstrap,
            harness.store.authorization.clone(), original_operation, original_selectors.clone(), changed_input).await;
        assert!(matches!(changed, Err(handshake_core::swarm_orchestration::model_lane::ModelLaneError::IdempotencyConflict(_))),
            "same committed operation cannot accept a changed complete input");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled);
        assert!(harness.store.store.retry_committed_promotion_decision(&harness.store.bootstrap,
            harness.store.authorization.clone(), Uuid::now_v7(), original_selectors.clone(), exact_retry_input.clone()).await.is_err(),
            "an uncommitted operation cannot borrow another operation's settlement or fall back to first consumption");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled);
        let mut different_selectors = original_selectors.clone();
        different_selectors.operator.expectation.expected.aggregate_version += 1;
        assert!(harness.store.store.retry_committed_promotion_decision(&harness.store.bootstrap,
            harness.store.authorization.clone(), original_operation, different_selectors, exact_retry_input.clone()).await.is_err(),
            "committed retry must preserve the full original selector expectations");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled);
        let mut missing = harness.store.authorization.clone();
        missing.session_token.clear();
        let mut invalid = harness.store.authorization.clone();
        invalid.session_token = "0".repeat(64);
        let mut wrong_channel = harness.store.authorization.clone();
        wrong_channel.channel_binding_hash = Some("0".repeat(64));
        let mut read_only = harness.store.authorization.clone();
        read_only.action = ResourceAction::Read;
        read_only.capability_id = "fs.read".into();
        for authorization in [missing, invalid, wrong_channel, read_only] {
            assert!(harness.store.store.retry_committed_promotion_decision(&harness.store.bootstrap,
                authorization, original_operation, original_selectors.clone(), exact_retry_input.clone()).await.is_err(),
                "retired-owner retry must independently require current native Creator and channel authority");
            assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled);
        }
        for foreign_scope in one_field_mismatches(&harness.scope) {
            let foreign = harness.store_for(foreign_scope);
            assert!(foreign.store.retry_committed_promotion_decision(&foreign.bootstrap,
                foreign.authorization.clone(), original_operation, original_selectors.clone(), exact_retry_input.clone()).await.is_err(),
                "every configured scope field is checked at the genuine production retry boundary");
            assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, settled);
        }
        harness.store.test_corrupt_scoped_authority("promotion_decision", &denied.decision_id,
            ModelLaneAuthorityTestCorruption::ReceiptPayloadHash).await
            .expect("existing owned exact-scope corruption changes only the actual committed decision receipt hash");
        let corrupt = promotion_settlement_sets(&harness.storage.test_inspector()).await;
        assert_ne!(corrupt, settled, "the negative genuinely changes canonical settlement evidence");
        let corrupt_retry = harness.store.store.retry_committed_promotion_decision(&harness.store.bootstrap,
            harness.store.authorization.clone(), original_operation, original_selectors.clone(), exact_retry_input.clone()).await;
        assert!(matches!(corrupt_retry,
            Err(handshake_core::swarm_orchestration::model_lane::ModelLaneError::Surreal(
                handshake_core::storage::surreal::SurrealStorageError::InvalidModelLaneRecord { reason }))
                if matches!(reason, "original committed promotion consumption unavailable"
                    | "original committed promotion decision unavailable"
                    | "model-lane authority row lacks one exact-scope EventLedger envelope"
                    | "model-lane authority/EventLedger envelope mismatch")),
            "corrupt committed evidence must fail canonical envelope integrity, not a native authority gate");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, corrupt);
        let mut actual_read = harness.store.authorization.clone();
        actual_read.action = ResourceAction::Read;
        actual_read.capability_id = "fs.read".into();
        let read_grant = harness.storage.authorize_protected_resource(actual_read.clone()).await
            .expect("actual current Read authority before native revocation");
        let creator_grant = harness.storage.authorize_protected_resource(harness.store.authorization.clone()).await
            .expect("actual current Creator authority before native revocation");
        assert_eq!(read_grant.grant_id, creator_grant.grant_id,
            "the existing first-owner fixture uses one shared Read/Create grant; this is not isolated Read attenuation");
        harness.storage.revoke_grant(&read_grant.grant_id).await.expect("revoke the actual shared native grant");
        assert!(harness.storage.authorize_protected_resource(actual_read).await.is_err());
        assert!(harness.storage.authorize_protected_resource(harness.store.authorization.clone()).await.is_err());
        let grant_retry = harness.store.store.retry_committed_promotion_decision(&harness.store.bootstrap,
            harness.store.authorization.clone(), original_operation, original_selectors.clone(), exact_retry_input.clone()).await;
        assert!(matches!(grant_retry,
            Err(handshake_core::swarm_orchestration::model_lane::ModelLaneError::AuthorityDenied(ref reason))
                if reason == "resource authority denied"),
            "revoked shared Read/Create grant must fail native resource authorization, not earlier receipt corruption");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, corrupt);
        harness.fixture.revoke_owned_session_for_retry_denial().await;
        let session_retry = harness.store.store.retry_committed_promotion_decision(&harness.store.bootstrap,
            harness.store.authorization.clone(), original_operation, original_selectors, exact_retry_input).await;
        assert!(matches!(session_retry,
            Err(handshake_core::swarm_orchestration::model_lane::ModelLaneError::Surreal(
                handshake_core::storage::surreal::SurrealStorageError::Database(ref error)))
                if matches!(error.details(), surrealdb::types::ErrorDetails::NotFound(None))
                    && error.message() == "No record was returned"),
            "revoked native session must fail SIGNIN before grant lookup, not earlier grant denial or receipt corruption");
        assert_eq!(promotion_settlement_sets(&harness.storage.test_inspector()).await, corrupt);
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

#[tokio::test]
async fn promotion_projection_and_event_receipt_tamper_fail_every_consumer_closed() {
    let mut harness = Harness::create("promotion-tamper").await;
    let result = std::panic::AssertUnwindSafe(async {
        for (index, corruption) in [
            ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
            ModelLaneAuthorityTestCorruption::ProjectionScope,
            ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
            ModelLaneAuthorityTestCorruption::ReceiptScope,
        ]
        .into_iter()
        .enumerate()
        {
            let label = format!("promotion-tamper-{index}");
            let store = harness.iteration(&label).await;
            let seeded = seed_authority(&store, &label, true).await;
            let decision_id = format!("decision-tamper-{index}");
            let decision = sample_decision(
                &label,
                &decision_id,
                &format!("idem-tamper-{index}"),
                seeded.proposal_version,
            );
            store
                .record_promotion_decision(decision.clone())
                .await
                .expect("seed canonical promotion decision");
            store
                .test_corrupt_scoped_authority("promotion_decision", &decision_id, corruption)
                .await
                .expect("apply enumerated exact-scope corruption");

            let run_id = format!("run-mt004-{label}");
            assert!(store.replay_promotion_decisions(&run_id).await.is_err());
            assert!(store.replay_run(&run_id).await.is_err());
            assert!(store.navigation_by_run(&run_id).await.is_err());
            assert!(store.record_promotion_decision(decision).await.is_err());
            assert!(store
                .record_message(sample_promoted_message(&label, &decision_id, index))
                .await
                .is_err());
        }
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

#[tokio::test]
async fn navigation_rejects_tampered_run_lane_and_message_origins_before_redirect() {
    let mut harness = Harness::create("origin-matrix").await;
    let result = std::panic::AssertUnwindSafe(async {
        for (index, origin) in ["run", "lane", "message"].into_iter().enumerate() {
            let label = format!("origin-{origin}-{index}");
            let store = harness.iteration(&label).await;
            seed_authority(&store, &label, true).await;
            let run_id = format!("run-mt004-{label}");
            let lane_id = format!("lane-mt004-{label}");
            let message_id = format!("message-mt004-{label}-proposal");
            let aggregate_id = match origin {
                "run" => run_id.as_str(),
                "lane" => lane_id.as_str(),
                "message" => message_id.as_str(),
                _ => unreachable!(),
            };
            store
                .test_corrupt_scoped_authority(
                    origin,
                    aggregate_id,
                    ModelLaneAuthorityTestCorruption::ProjectionScope,
                )
                .await
                .expect("retarget typed origin projection outside its receipt scope");

            let denied = match origin {
                "run" => store.navigation_by_run(&run_id).await.is_err(),
                "lane" => store.navigation_by_lane(&lane_id).await.is_err(),
                "message" => store.navigation_by_message(&message_id).await.is_err(),
                _ => unreachable!(),
            };
            assert!(denied, "{origin} origin must validate before run redirect");
        }
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

/// Acceptance row 1 (all six coordinator routing policies reach a durable, EventLedger-backed
/// promotion decision) plus the two negative-proof classes the acceptance rows name that no other
/// case covers: mismatched expected aggregate version, and a duplicate idempotency key that
/// carries a different canonical decision.
#[tokio::test]
async fn promotion_covers_every_routing_policy_and_denies_version_and_idempotency_conflicts() {
    let mut harness = Harness::create("policy-matrix").await;
    let result = std::panic::AssertUnwindSafe(async {
        let seeded = seed_authority(&harness.store, "policy-matrix", true).await;

        for (index, policy) in ModelLaneRoutingPolicy::all().iter().copied().enumerate() {
            let mut decision = sample_decision(
                "policy-matrix",
                &format!("decision-policy-{index}"),
                &format!("idem-policy-{index}"),
                seeded.proposal_version,
            );
            decision.routing_policy = policy;
            // Each policy graph gates its stages on a different authority: the cloud stages need a
            // consent receipt, the validator lane needs a validator ref, the operator lane needs an
            // operator ref. Supply all three so the proof exercises the graph, not a missing field.
            decision.validator_authority_ref = Some("validator://mt004/policy-matrix".into());
            decision.diagnostic_payload = json!({
                "flight_recorder": "kernel_event_ledger",
                "operator_authority_ref": "operator://mt004/policy-matrix",
                "cloud_consent_receipt_ref": "consent://mt004/policy-matrix"
            });
            let stored = harness
                .store
                .record_promotion_decision(decision)
                .await
                .unwrap_or_else(|error| panic!("{policy:?} promotion decision: {error}"));
            assert_eq!(
                stored.outcome,
                ModelLanePromotionOutcome::Approved,
                "{policy:?} must reach an approved promotion"
            );
            assert_eq!(stored.inner.routing_policy, policy);
            assert_eq!(stored.final_state, ModelLanePromotionState::Executed);
            assert!(
                stored.event_ledger_seq > 0,
                "{policy:?} decision must be EventLedger-backed"
            );
        }
        assert_eq!(
            harness
                .store
                .replay_promotion_decisions("run-mt004-policy-matrix")
                .await
                .expect("replay every policy decision")
                .len(),
            ModelLaneRoutingPolicy::all().len()
        );

        let version = harness
            .store
            .record_promotion_decision(sample_decision(
                "policy-matrix",
                "decision-version",
                "idem-version",
                seeded.proposal_version + 41,
            ))
            .await
            .expect("aggregate version mismatch is a durable denial");
        assert_eq!(version.outcome, ModelLanePromotionOutcome::Denied);
        assert_eq!(
            version.denial_reason,
            Some(ModelLanePromotionDenialReason::AggregateVersionMismatch)
        );

        // `idem-policy-0` already belongs to the LocalFirst decision recorded above. This reuse
        // carries a different canonical decision under a policy whose authority gate is satisfied,
        // so the conflict is proven on idempotency, not on input validation.
        let conflict = sample_decision(
            "policy-matrix",
            "decision-conflict",
            "idem-policy-0",
            seeded.proposal_version,
        );
        assert!(
            harness
                .store
                .record_promotion_decision(conflict)
                .await
                .is_err(),
            "a reused idempotency key carrying a different canonical decision must conflict"
        );
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

/// Ports the fenced `model_lane_promotion_pg_tests` advisory-input authority matrix
/// (`model_lane_promotion_durably_denies_tampered_advisory_eventledger_input` and the advisory
/// scope arm of `model_lane_exact_scope_authority_rejects_message_run_lane_and_navigation_tamper`)
/// onto the embedded substrate. A promotion decision may never be Approved from an advisory input
/// whose durable projection or canonical EventLedger receipt has been tampered with.
#[tokio::test]
async fn promotion_denies_tampered_advisory_input_on_projection_and_receipt_authority() {
    let mut harness = Harness::create("advisory-tamper").await;
    let result = std::panic::AssertUnwindSafe(async {
        for (index, corruption) in [
            ModelLaneAuthorityTestCorruption::ProjectionScope,
            ModelLaneAuthorityTestCorruption::IncompleteAttribution,
            ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
            ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
            ModelLaneAuthorityTestCorruption::ReceiptScope,
        ]
        .into_iter()
        .enumerate()
        {
            let label = format!("advisory-tamper-{index}");
            let store = harness.iteration(&label).await;
            let seeded = seed_authority(&store, &label, true).await;
            let proposal_id = format!("message-mt004-{label}-proposal");
            store
                .test_corrupt_scoped_authority("message", &proposal_id, corruption)
                .await
                .expect("tamper the advisory input this decision will name");

            let run_id = format!("run-mt004-{label}");
            // Two distinct authority failures need two distinct expectations. A corruption that
            // rewrites the row's own scope removes it from this reader's exact scope, so replay
            // legitimately succeeds while no longer returning the message. A corruption that breaks
            // the projection/receipt link leaves the row in scope, so replay MUST fail closed.
            let replay = store.replay_run(&run_id).await;
            match corruption {
                ModelLaneAuthorityTestCorruption::ProjectionScope
                | ModelLaneAuthorityTestCorruption::IncompleteAttribution => {
                    let replay = replay.unwrap_or_else(|error| {
                        panic!("{corruption:?}: scope-removed message must not break replay: {error}")
                    });
                    assert!(
                        !replay
                            .messages
                            .iter()
                            .any(|message| message.message_id == proposal_id),
                        "{corruption:?}: replay returned a message outside this exact scope"
                    );
                }
                _ => assert!(
                    replay.is_err(),
                    "{corruption:?}: run replay accepted a broken advisory receipt link"
                ),
            }

            let decision = store
                .record_promotion_decision(sample_decision(
                    &label,
                    &format!("decision-advisory-{index}"),
                    &format!("idem-advisory-{index}"),
                    seeded.proposal_version,
                ))
                .await;
            match decision {
                Ok(record) => {
                    assert_eq!(
                        record.outcome,
                        ModelLanePromotionOutcome::Denied,
                        "{corruption:?}: tampered advisory input produced an approved promotion"
                    );
                    assert_eq!(
                        record.denial_reason,
                        Some(ModelLanePromotionDenialReason::InputRefMismatch),
                        "{corruption:?}: tampered advisory input must deny on input-ref authority"
                    );
                }
                Err(_) => {
                    // A fail-closed error is also an acceptable denial of authority, provided no
                    // approved decision became durable.
                    assert!(store
                        .replay_promotion_decisions(&run_id)
                        .await
                        .map(|records| records
                            .iter()
                            .all(|record| record.outcome != ModelLanePromotionOutcome::Approved))
                        .unwrap_or(true));
                }
            }
        }
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

/// Ports the fenced pg
/// `model_lane_promotion_rejects_message_projection_and_scope_tamper_on_replay_and_retry`:
/// once a promoted message is durable, tampering its projection or canonical receipt must fail
/// both run replay and the idempotent message retry.
#[tokio::test]
async fn promoted_message_projection_and_receipt_tamper_deny_replay_and_retry() {
    let mut harness = Harness::create("promoted-tamper").await;
    let result = std::panic::AssertUnwindSafe(async {
        for (index, corruption) in [
            ModelLaneAuthorityTestCorruption::ProjectionEventSequence,
            ModelLaneAuthorityTestCorruption::ProjectionScope,
            ModelLaneAuthorityTestCorruption::ReceiptPayloadHash,
            ModelLaneAuthorityTestCorruption::ReceiptScope,
        ]
        .into_iter()
        .enumerate()
        {
            let label = format!("promoted-tamper-{index}");
            let store = harness.iteration(&label).await;
            let seeded = seed_authority(&store, &label, true).await;
            let decision_id = format!("decision-promoted-{index}");
            store
                .record_promotion_decision(sample_decision(
                    &label,
                    &decision_id,
                    &format!("idem-promoted-{index}"),
                    seeded.proposal_version,
                ))
                .await
                .expect("seed canonical promotion decision");
            let promoted_input = sample_promoted_message(&label, &decision_id, index);
            let promoted = store
                .record_message(promoted_input.clone())
                .await
                .expect("record promoted message before tamper");
            store
                .test_corrupt_scoped_authority("message", &promoted.message_id, corruption)
                .await
                .expect("tamper the durable promoted message");

            let run_id = format!("run-mt004-{label}");
            // Same split as the advisory matrix: a rewritten row scope removes the promoted message
            // from this reader entirely, while a broken projection/receipt link must fail replay closed.
            let replay = store.replay_run(&run_id).await;
            if corruption == ModelLaneAuthorityTestCorruption::ProjectionScope {
                let replay = replay.unwrap_or_else(|error| {
                    panic!("{corruption:?}: scope-removed promoted message broke replay: {error}")
                });
                assert!(
                    !replay
                        .messages
                        .iter()
                        .any(|message| message.message_id == promoted.message_id),
                    "{corruption:?}: replay returned a promoted message outside this exact scope"
                );
            } else {
                assert!(
                    replay.is_err(),
                    "{corruption:?}: run replay accepted a tampered promoted message"
                );
            }
            // In every class the idempotent retry must refuse: it may neither silently re-create the
            // tampered promoted message nor return the corrupted row as a settled success.
            assert!(
                store.record_message(promoted_input).await.is_err(),
                "{corruption:?}: idempotent retry accepted a tampered promoted message"
            );
        }
    }).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(harness.cleanup()).catch_unwind().await;
    if result.is_err() || cleanup.is_err() {
        std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
            primary: result.err(), cleanup: cleanup.err(),
        }));
    }
}

/// Ports the substantive invariant of the fenced pg forged-projection arm of
/// `model_lane_exact_scope_authority_rejects_message_run_lane_and_navigation_tamper`: run/lane
/// authority held only in a foreign exact scope can never manufacture message-append authority
/// for the owner scope, and the denial must be non-disclosing and leave both the projection and
/// the EventLedger row counts unchanged.
#[tokio::test]
async fn foreign_scope_run_and_lane_cannot_manufacture_message_append_authority() {
    cross_account_append::run().await.expect("genuine cross-account append isolation and settled runtime");
}


async fn promotion_settlement_sets(inspector: &SurrealTestInspector) -> serde_json::Value {
    let mut sets = serde_json::Map::new();
    for (table_name, selector_name) in [
        ("model_lane_authority", "record_kind"), ("kernel_event_ledger", "aggregate_type"),
    ] {
        let catalog = inspector.table_catalog(table_name).await.expect("actual canonical settlement catalog");
        let table = inspector.table_selector(table_name).await.expect("actual canonical settlement table");
        let fields = catalog.fields.iter().filter(|field| field.name != "id"
            && field.name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
            .map(|field| table.field(&field.name)).collect::<Result<Vec<_>, _>>()
            .expect("all declared direct settlement fields are catalog-selected");
        assert!(!fields.is_empty());
        for family in ["promotion_decision", "role_evidence_consumption"] {
            let mut rows = inspector.project(&table, &fields, RowFilter::FieldEquals {
                field: table.field(selector_name).expect("actual canonical family selector"),
                value: ScalarValue::String(family.into()),
            }).await.expect("independent canonical full settlement family set");
            rows.sort_by_cached_key(|row| serde_json::to_string(&row.record_id).expect("canonical row identity"));
            sets.insert(format!("{table_name}/{family}"), serde_json::to_value(rows).expect("actual projected canonical rows"));
        }
    }
    serde_json::Value::Object(sets)
}
async fn promotion_event_count(inspector: &SurrealTestInspector) -> u64 {
    let table = inspector.table_selector("kernel_event_ledger").await.unwrap();
    let field = table.field("aggregate_type").unwrap();
    inspector.row_count(&table, RowFilter::FieldEquals { field,
        value: ScalarValue::String("promotion_decision".into()) }).await.unwrap()
}

async fn table_count(inspector: &SurrealTestInspector, table: &str) -> u64 {
    let table = inspector
        .table_selector(table)
        .await
        .unwrap_or_else(|error| panic!("select {table}: {error}"));
    inspector
        .row_count(&table, RowFilter::All)
        .await
        .unwrap_or_else(|error| panic!("count {}: {error}", table.name()))
}

async fn seed_authority(store: &AuthenticatedPromotionStore, label: &str, include_messages: bool) -> Seeded {
    let operation = store.operation(ResourceAction::Create).await.expect("genuine current Create authority");
    let (run, lane, artifact, version) = operation.run(async {
        let run = store.store.record_run(sample_run(label)).await?;
        let lane = store.store.record_lane(sample_lane(label)).await?;
        let artifact = store.store.record_context_bundle_artifact_binding(sample_artifact_binding(label)).await?;
        let version = if include_messages {
            let proposal = store.store.record_message(sample_message(label, "proposal")).await?;
            store.store.record_message(sample_message(label, "critique")).await?;
            proposal.event_stream_version
        } else { 1 };
        Ok((run, lane, artifact, version))
    }).await.expect("persist original promotion inputs under real authenticated Create boundary");
    drop(operation);
    if include_messages {
        let candidate = sample_decision(label, &format!("role-production-{label}"),
            &format!("role-production-{label}"), version);
        let policy = ".GOV/task_packets/WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1/MT-017.json#/ac9_current_surreal_two_author_source_admission_20261015/pure_role_authority_five_new_path_source_admission_20261015/actual_closed_handoff/exact_full_contract";
        let retention = handshake_role_authority::domain::RetentionBinding {
            policy_ref: policy.into(), authority_class: "Owner".into(),
            replay_policy_ref: policy.into(), artifact_retention_ttl_days: None,
        };
        let task = json!({"format_version":1,"task_id":format!("mt004-promotion-{label}"),
            "claimed_revision":1,"operations":[
                {"tool":"delegate_workspace_read","resource_kind":"workspace",
                 "external_resource_id":store.authorization.external_resource_id,
                 "action":"read","capability_id":"fs.read"},
                {"tool":"delegate_workspace_create","resource_kind":"workspace",
                 "external_resource_id":store.authorization.external_resource_id,
                 "action":"create","capability_id":"fs.write"}
            ]});
        let mut authorities = store.authorities.lock().await;
        assert!(!authorities.contains_key(&run.run_id), "one original role producer per persisted run");
        let authority = Ac9RoleAuthorityFixture::produce(&store.bootstrap, store.store.clone(),
            store.authorization.clone(), &run, &lane, &artifact, &candidate,
            retention.clone(), retention, task).await
            .expect("actual registered Validator review and authenticated Operator approval");
        let selectors = authority.selectors();
        authorities.insert(run.run_id.clone(), HeldPromotionAuthority {
            operations: BTreeMap::new(), selectors, runtime: Some(authority),
        });
    }
    Seeded { proposal_version: version }
}

async fn seed_advisory_inputs(store: &ModelLaneStore, label: &str, include_messages: bool) -> Seeded {
    store
        .record_run(sample_run(label))
        .await
        .expect("seed promotion run");
    store
        .record_lane(sample_lane(label))
        .await
        .expect("seed promotion lane");
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding(label))
        .await
        .expect("seed promoted artifact authority");
    if !include_messages {
        return Seeded {
            proposal_version: 1,
        };
    }
    let proposal = store
        .record_message(sample_message(label, "proposal"))
        .await
        .expect("seed proposal advisory");
    store
        .record_message(sample_message(label, "critique"))
        .await
        .expect("seed critique advisory");
    Seeded {
        proposal_version: proposal.event_stream_version,
    }
}

fn sample_decision(
    label: &str,
    decision_id: &str,
    idempotency_key: &str,
    expected_version: i64,
) -> NewModelLanePromotionDecision {
    NewModelLanePromotionDecision {
        decision_id: decision_id.into(),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        decision_span_id: format!("span-{decision_id}"),
        parent_span_id: Some(format!("span-lane-mt004-{label}")),
        linked_span_contexts: vec![
            format!("span-message-mt004-{label}-proposal"),
            format!("span-message-mt004-{label}-critique"),
        ],
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane,
        routing_launch_plan: Vec::new(),
        input_refs: vec![
            format!("model-lane-message://message-mt004-{label}-critique"),
            format!("model-lane-message://message-mt004-{label}-proposal"),
        ],
        selected_input_refs: vec![format!(
            "model-lane-message://message-mt004-{label}-proposal"
        )],
        rejected_input_refs: vec![format!(
            "model-lane-message://message-mt004-{label}-critique"
        )],
        validator_authority_ref: None,
        operator_authority_ref: Some(format!("operator://mt004/{label}")),
        expected_event_ledger_aggregate_type: "model_lane_message".into(),
        expected_event_ledger_aggregate_id: format!("message-mt004-{label}-proposal"),
        expected_event_ledger_version: expected_version,
        base_snapshot_ref: "not-applicable".into(),
        current_base_snapshot_ref: "not-applicable".into(),
        state_vector: "not-applicable".into(),
        current_state_vector: "not-applicable".into(),
        schema_id: "hsk.model_lane_message@1".into(),
        deterministic_tie_break_rule: "lexicographic_selected_ref_then_lowest_event_seq".into(),
        promotion_gate_ref: format!("promotion-gate://mt004/{label}"),
        promotion_receipt_ref: Some(format!("promotion-receipt://mt004/{label}")),
        promoted_artifact_ref: Some(format!("artifact://mt004/{label}/promoted")),
        promoted_artifact_sha256: Some(promoted_artifact_hash(label)),
        promoted_artifact_version: Some("1".into()),
        direct_authority_mutation_attempt_ref: None,
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("0020-{decision_id}"),
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:02:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "kernel_event_ledger",
            "operator_authority_ref": format!("operator://mt004/{label}")
        }),
    }
}

fn sample_artifact_binding(label: &str) -> NewModelLaneContextBundleArtifactBinding {
    let payload = promoted_artifact_payload(label);
    let hash = promoted_artifact_hash(label);
    let artifact_ref = format!("artifact://mt004/{label}/promoted");
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-mt004-{label}"),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        artifact_ref: artifact_ref.clone(),
        artifact_sha256: hash.clone(),
        content_hash: hash,
        artifact_kind: "model_lane_promoted_artifact".into(),
        artifact_manifest_ref: format!("artifact-manifest://mt004/{label}"),
        artifact_payload_ref: artifact_ref,
        payload_json: payload,
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: format!("artifact-binding-mt004-{label}"),
        created_at_utc: "2026-09-02T00:01:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn promoted_artifact_payload(label: &str) -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_promoted_artifact@1",
        "artifact_version": "1",
        "body": format!("deterministic promoted artifact {label}")
    })
}

fn promoted_artifact_hash(label: &str) -> String {
    hex::encode(Sha256::digest(
        serde_json::to_vec(&promoted_artifact_payload(label)).expect("serialize artifact"),
    ))
}

fn sample_run(label: &str) -> NewModelLaneRun {
    NewModelLaneRun {
        engine_profile: None,
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        run_span_id: format!("span-run-mt004-{label}"),
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        routing_policy: ModelLaneRoutingPolicy::OperatorLane.as_str().into(),
        context_bundle_id: format!("context-mt004-{label}"),
        lane_ids: vec![format!("lane-mt004-{label}")],
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        artifact_namespace: format!("artifact://mt004/{label}"),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        idempotency_key: format!("run-mt004-{label}"),
        replay_order_key: format!("0001-{label}"),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        locus_binding: Some(sample_locus(label)),
        memory_pack_ref: format!("memory-pack://mt004/{label}"),
        memory_pack_hash: "a".repeat(64),
        determinism_mode: "strict".into(),
        budget_summary_ref: format!("budget://mt004/{label}"),
        selected_model_id: Some("model://local/mt004".into()),
        candidate_model_ids: vec!["model://local/mt004".into()],
        procedural_review_status: "approved".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: Vec::new(),
    }
}

fn sample_lane(label: &str) -> NewModelLane {
    let session = format!("session-mt004-{label}");
    let model_session = format!("model-session-mt004-{label}");
    NewModelLane {
        engine_profile: None,
        lane_id: format!("lane-mt004-{label}"),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        lane_span_id: format!("span-lane-mt004-{label}"),
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        kind: ModelLaneKind::LocalModel,
        role: "proposal-author".into(),
        backend: "embedded-model-runtime".into(),
        model_id: Some("model://local/mt004".into()),
        session_id: session.clone(),
        model_session_id: model_session.clone(),
        adapter_id: "local-runtime".into(),
        runtime_binding: RuntimeBinding::Local,
        launch_authority: LaunchAuthority::ModelRuntime,
        provider_kind: ModelLaneProviderKind::LocalRuntime,
        capability_token_ids: vec!["capability://mt004/context".into()],
        effective_capability_snapshot_ref: Some("capability://mt004/snapshot".into()),
        capability_negotiation_ref: Some("capability://mt004/negotiation".into()),
        provider_feature_profile_ref: Some("provider://mt004/local".into()),
        requested_execution_policy_ref: Some("execution://mt004/requested".into()),
        effective_execution_policy_ref: Some("execution://mt004/effective".into()),
        projection_plan_ref: None,
        consent_receipt_ref: None,
        tool_gate_decision_refs: vec!["tool-gate://mt004/context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-09-02T00:00:00Z".into()),
        lease_expires_at_utc: Some("2099-09-02T00:00:00Z".into()),
        reclaim_after_utc: Some("2099-09-02T00:01:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel://mt004/{label}")),
        reclaim_policy_ref: Some("reclaim://mt004".into()),
        terminal_status_mapping_ref: Some("terminal://mt004".into()),
        process_ownership_ref: Some(format!("process://mt004/{label}")),
        no_os_process_reason_ref: None,
        backpressure_ref: None,
        loop_counter_ref: Some("loop://mt004".into()),
        last_runtime_status_ref: Some("runtime://mt004/ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding: Some(sample_locus_for(label, &session, &model_session)),
    }
}

fn sample_message(label: &str, kind: &str) -> NewModelLaneMessage {
    let message_id = format!("message-mt004-{label}-{kind}");
    NewModelLaneMessage {
        message_id: message_id.clone(),
        run_id: format!("run-mt004-{label}"),
        trace_id: format!("trace-mt004-{label}"),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-lane-mt004-{label}")),
        linked_span_contexts: vec![format!("trace-mt004-{label}")],
        from_lane_id: format!("lane-mt004-{label}"),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(ModelLaneRoutingMetadata {
            target_role: "coordinator".into(),
            target_session: format!("coordinator-mt004-{label}"),
            correlation_id: format!("correlation-{message_id}"),
            requires_ack: true,
            ack_for: None,
        }),
        kind: ModelLaneMessageKind::Proposal,
        payload_ref: format!("artifact://mt004/{label}/{kind}"),
        payload_sha256: if kind == "proposal" {
            "b".repeat(64)
        } else {
            "c".repeat(64)
        },
        event_ledger_stream_id: format!("model-lane://mt004/{label}"),
        summary: format!("{kind} advisory"),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["tool-gate://mt004/context".into()],
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-004".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding: Some(sample_locus(label)),
        idempotency_key: format!("message-mt004-{label}-{kind}"),
        replay_order_key: format!("0010-{kind}"),
        replay_after_event_ledger_seq: None,
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane/promotion".into()),
        created_at_utc: "2026-09-02T00:01:30Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn sample_promoted_message(label: &str, decision_id: &str, index: usize) -> NewModelLaneMessage {
    let artifact_ref = format!("artifact://mt004/{label}/promoted");
    let mut message = sample_message(label, &format!("promoted-{index}"));
    message.kind = ModelLaneMessageKind::PromotionRequest;
    message.authority = ModelLaneAuthority::Promoted;
    message.payload_ref = artifact_ref.clone();
    message.payload_sha256 = promoted_artifact_hash(label);
    message.promotion_decision_id = Some(decision_id.into());
    message.promotion_gate_ref = Some(format!("promotion-gate://mt004/{label}"));
    message.promotion_receipt_ref = Some(format!("promotion-receipt://mt004/{label}"));
    message.validator_verdict_ref = Some(format!("validator://mt004/{label}"));
    message.operator_decision_ref = Some(format!("operator://mt004/{label}"));
    message.promoted_artifact_ref = Some(artifact_ref);
    message.promoted_artifact_sha256 = Some(promoted_artifact_hash(label));
    message.promoted_artifact_version = Some("1".into());
    message
}

fn sample_locus(label: &str) -> ModelLaneLocusBinding {
    sample_locus_for(
        label,
        &format!("session-mt004-{label}"),
        &format!("model-session-mt004-{label}"),
    )
}

fn sample_locus_for(
    label: &str,
    session_id: &str,
    model_session_id: &str,
) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-004".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: format!("coordinator-mt004-{label}"),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: format!("owner-mt004-{label}"),
        locus_binding_ref: format!("locus://wp1/mt004/{label}"),
    }
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let workspace = scope.workspace.clone().expect("exact workspace");
    let session = scope.authenticated_session.expect("exact session");
    let access_space = scope.access_space.expect("exact access space");
    vec![
        ResourceScope::new(OwnerAccountId::mint(), scope.actor_principal_id)
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, ActorPrincipalId::mint())
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(AuthenticatedSessionRef::mint())
            .with_access_space(access_space)
            .with_workspace(workspace.clone()),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(session)
            .with_access_space(AccessSpaceRef::mint())
            .with_workspace(workspace),
        ResourceScope::new(scope.owner_account_id, scope.actor_principal_id)
            .with_session(session)
            .with_access_space(access_space)
            .with_workspace(
                WorkspaceScopeRef::new("workspace-mt004-foreign").expect("nonblank workspace"),
            ),
    ]
}

/// Public acquisition and cleanup shared by the promotion matrix and its independent prerequisite.
mod authenticated_promotion_fixture {
    use super::*;
    use std::path::Path;
    use std::sync::{Arc, Mutex, MutexGuard};
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use futures::FutureExt;
    use handshake_core::api::{authority, stage, workspaces};
    use handshake_core::capabilities::CapabilityRegistry;
    use handshake_core::storage::Database;
    use handshake_core::storage::surreal::resource_authority::{AuthorizationRequest, ResourceAction, ResourceKind};
    use handshake_core::test_harness::crdt_workspace::AuthenticatedModelLaneTestOperation;
    use handshake_core::AccountBootstrapState;
    use serde_json::Value;
    use tower::ServiceExt;
    use uuid::Uuid;

    static CHANNEL_LOCK: Mutex<()> = Mutex::new(());

    pub(super) struct NativeChannel {
        _lock: MutexGuard<'static, ()>,
        _directory: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
        pub(super) token: String,
    }

    impl NativeChannel {
        pub(super) fn create(root: &Path) -> Self {
            let lock = CHANNEL_LOCK.lock().unwrap_or_else(|error| error.into_inner());
            let directory = tempfile::tempdir_in(root).expect("own promotion native binding");
            let token = hex::encode(Sha256::digest(Uuid::now_v7().as_bytes()));
            let path = directory.path().join("native-binding.json");
            std::fs::write(&path, serde_json::to_vec(
                &stage::current_process_native_session_binding(&token),
            ).expect("serialize actual promotion process binding")).expect("write owned native binding");
            let previous = std::env::var_os("HANDSHAKE_STAGE_BINDING_FILE");
            std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", path);
            Self { _lock: lock, _directory: directory, previous, token }
        }
    }

    impl Drop for NativeChannel {
        fn drop(&mut self) {
            match &self.previous {
                Some(previous) => std::env::set_var("HANDSHAKE_STAGE_BINDING_FILE", previous),
                None => std::env::remove_var("HANDSHAKE_STAGE_BINDING_FILE"),
            }
        }
    }

    struct OwnedSession { token: String, identity: Value }

    pub(super) struct PublicPromotionFixture {
        pub(super) isolated: IsolatedSurrealTestStore,
        pub(super) bootstrap: Option<AccountBootstrapState>,
        channel: NativeChannel,
        session: Option<OwnedSession>,
        session_revoked_for_retry: bool,
        pub(super) scope: Option<ResourceScope>,
    }

    impl PublicPromotionFixture {
        pub(super) async fn create() -> Self {
            let root = surreal_test_store_support::canonical_artifacts_root_for_proof()
                .expect("verified promotion artifact root");
            let owner = root.join("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1")
                .join("MT-004").join("authenticated-promotion-fixture");
            assert!(owner.starts_with(&root));
            std::fs::create_dir_all(&owner).expect("create assigned promotion fixture descendant");
            let isolated = IsolatedSurrealTestStore::create_in(&owner).await.expect("isolated promotion database");
            let channel = NativeChannel::create(&owner);
            let registry = Arc::new(CapabilityRegistry::new());
            registry.profile_by_id("Operator").expect("actual loaded Operator profile");
            let database: Arc<dyn Database> = Arc::new(SurrealDatabase::new(isolated.storage().clone()));
            let bootstrap = AccountBootstrapState::new(database, isolated.storage().clone(), registry);
            Self { isolated, bootstrap: Some(bootstrap), channel, session: None, session_revoked_for_retry: false, scope: None }
        }

        fn router(&self) -> Router {
            let state = self.bootstrap.as_ref().expect("owned bootstrap").clone();
            let (activation, _receiver) = tokio::sync::mpsc::channel(1);
            authority::bootstrap_routes(state.clone(), activation).merge(workspaces::bootstrap_routes(state))
        }

        pub(super) async fn authenticate_workspace(&mut self) {
            assert!(self.session.is_none(), "one original session per fixture");
            let router = self.router();
            let (status, setup) = dispatch(&router, "GET", "/authority/setup", &self.channel.token, None, Value::Null).await;
            assert_eq!(status, StatusCode::OK, "read canonical setup state");
            let credentials = json!({"account_name": "Synthetic isolated promotion operator",
                "password": "synthetic isolated promotion fixture password without egress"});
            if setup["setup_required"].as_bool().expect("typed setup state") {
                let (status, _) = dispatch(&router, "POST", "/authority/setup", &self.channel.token, None, credentials.clone()).await;
                assert_eq!(status, StatusCode::OK, "actual one-time account setup");
            }
            let (status, credential) = dispatch(&router, "POST", "/authority/login", &self.channel.token, None, credentials).await;
            assert_eq!(status, StatusCode::OK, "actual password authentication");
            let (status, identity) = dispatch(&router, "POST", "/authority/session", &self.channel.token, None,
                json!({"account_id": field(&credential, "account_id"), "principal_id": field(&credential, "principal_id"),
                    "access_space_id": field(&credential, "access_space_id"), "authentication_token": field(&credential, "token")})).await;
            assert_eq!(status, StatusCode::OK, "actual session and vault exchange");
            let token = field(&identity, "session_token").to_owned();
            self.session = Some(OwnedSession { token: token.clone(), identity });
            let session = self.session.as_ref().expect("session cleanup custody installed");
            for key in ["account_id", "principal_id", "access_space_id"] {
                assert_eq!(field(&credential, key), field(&session.identity, key));
            }
            let (status, workspace) = dispatch(&router, "POST", "/workspaces", &self.channel.token, Some(&token),
                json!({"name": "Authenticated MT-004 promotion fixture"})).await;
            assert_eq!(status, StatusCode::CREATED, "real workspace/resource/Create grant producer");
            self.scope = Some(ResourceScope::new(
                OwnerAccountId::from_uuid(id(&session.identity, "account_id")),
                ActorPrincipalId::from_uuid(id(&session.identity, "principal_id")),
            ).with_session(AuthenticatedSessionRef::from_uuid(id(&session.identity, "session_id")))
                .with_access_space(AccessSpaceRef::from_uuid(id(&session.identity, "access_space_id")))
                .with_workspace(WorkspaceScopeRef::new(field(&workspace, "id")).expect("returned workspace id")));
        }

        pub(super) async fn additional_workspace(&self, label: &str) -> ResourceScope {
            let session = self.session.as_ref().expect("original authenticated session");
            let (status, workspace) = dispatch(&self.router(), "POST", "/workspaces", &self.channel.token,
                Some(&session.token), json!({"name":format!("Authenticated MT-004 {label}")})).await;
            assert_eq!(status, StatusCode::CREATED, "canonical additional workspace and owning grant");
            let mut scope = self.scope.as_ref().expect("original canonical account/session tuple").clone();
            scope.workspace = Some(WorkspaceScopeRef::new(field(&workspace, "id")).expect("real returned workspace"));
            scope
        }

        pub(super) fn authorization(&self) -> AuthorizationRequest {
            let session = self.session.as_ref().expect("actual authenticated session");
            let scope = self.scope.as_ref().expect("actual workspace scope");
            AuthorizationRequest { session_token: session.token.clone(),
                channel_binding_hash: Some(hex::encode(Sha256::digest(self.channel.token.as_bytes()))),
                capability_id: "fs.write".into(), resource_kind: ResourceKind::Workspace,
                external_resource_id: scope.workspace.as_ref().expect("actual workspace").as_str().to_owned(),
                action: ResourceAction::Create }
        }

        pub(super) fn scoped_store(&self) -> ModelLaneStore {
            ModelLaneStore::new_scoped_with_lifecycle(self.isolated.storage().clone(),
                self.scope.as_ref().expect("actual complete scope").clone(),
                self.isolated.storage().resource_access_lifecycle())
        }

        pub(super) async fn revoke_owned_session_for_retry_denial(&mut self) {
            let scope = self.scope.as_ref().expect("actual owned scope until cleanup");
            let session = scope.authenticated_session.as_ref().expect("actual original session").to_string();
            self.isolated.storage().revoke_session(&session).await.expect("native revocation of the actual original session");
            self.session_revoked_for_retry = true;
        }
        pub(super) async fn cleanup(&mut self) {
            if let Some(session) = self.session.as_ref() {
                let router = self.router();
                let (status, body) = dispatch(&router, "POST", "/authority/logout", &self.channel.token,
                    Some(&session.token), Value::Null).await;
                if self.session_revoked_for_retry {
                    assert_eq!(status, StatusCode::FORBIDDEN, "actual retired session invokes native secret retirement cleanup");
                    assert_eq!(body, json!({"error": "HSK-403-PROTECTED-RESOURCE"}));
                } else {
                    assert_eq!(status, StatusCode::OK, "revoke actual session and remove its vault secret");
                }
                drop(router);
                self.session.take();
            }
            drop(self.bootstrap.take());
            self.isolated.cleanup().await.expect("close actual promotion fixture database");
        }
    }

    async fn dispatch(router: &Router, method: &str, path: &str, channel: &str, token: Option<&str>, payload: Value)
        -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(path)
            .header("content-type", "application/json").header("x-hsk-channel-binding-token", channel);
        if let Some(token) = token { request = request.header("x-hsk-session-token", token); }
        let response = router.clone().oneshot(request.body(Body::from(serde_json::to_vec(&payload)
            .expect("serialize actual public fixture request"))).expect("public fixture request")).await
            .expect("dispatch actual bootstrap route");
        let status = response.status();
        let body = to_bytes(response.into_body(), 128 * 1024).await.expect("bounded public fixture response");
        (status, serde_json::from_slice(&body).expect("actual public response JSON"))
    }

    fn field<'a>(value: &'a Value, key: &str) -> &'a str {
        value.get(key).and_then(Value::as_str).filter(|value| !value.is_empty()).expect("canonical response field")
    }
    fn id(value: &Value, key: &str) -> Uuid { Uuid::parse_str(field(value, key)).expect("canonical response UUID") }

    #[tokio::test]
    async fn promotion_fixture_creates_canonical_inputs_with_public_authenticated_authority() {
        let mut fixture = PublicPromotionFixture::create().await;
        let result = std::panic::AssertUnwindSafe(async {
            fixture.authenticate_workspace().await;
            let scope = ExactResourceScopeAttribution::try_from_resource_scope(fixture.scope.as_ref().unwrap()).unwrap();
            let operation = AuthenticatedModelLaneTestOperation::authorize(
                fixture.isolated.storage().clone(), fixture.authorization(), scope.clone(),
            ).await.expect("real original authenticated protected-resource Create authority");
            let store = fixture.scoped_store();
            let seeded = operation.run(async { Ok(seed_advisory_inputs(&store, "authenticated-inputs", true).await) })
                .await.expect("actual canonical Create batch under original operation");
            assert!(seeded.proposal_version > 0);
            drop(operation);
            let mut read_authorization = fixture.authorization();
            read_authorization.capability_id = "fs.read".into();
            read_authorization.action = ResourceAction::Read;
            let read = AuthenticatedModelLaneTestOperation::authorize(
                fixture.isolated.storage().clone(), read_authorization, scope.clone(),
            ).await.expect("independent current authenticated protected-resource Read authority");
            read.run(async {
                let replay = store.replay_run("run-mt004-authenticated-inputs").await?;
                assert_eq!(replay.run.run_id, "run-mt004-authenticated-inputs");
                assert_eq!(replay.messages.len(), 2);
                assert_eq!(store.access().exact_read_scope(), Some(&scope));
                assert!(store.replay_promotion_decisions("run-mt004-authenticated-inputs").await?.is_empty(),
                    "fixture acquisition must not issue promotion authority");
                Ok(())
            }).await.expect("independent canonical reread under actual Read grant");
            drop(read);
            drop(store);
        }).catch_unwind().await;
        fixture.cleanup().await;
        if let Err(panic) = result { std::panic::resume_unwind(panic); }
    }
}

mod cross_account_append {
    use super::*;
    use std::{cell::RefCell, time::Duration};
    use serde_json::Value;
    use handshake_core::backend_runtime::{RuntimeConsumer, RuntimeHandle, RuntimeOptions};
    use handshake_core::storage::{ControlPlaneStorageConfig, ControlPlaneStorageMode};
    use handshake_core::storage::surreal::resource_authority::ResourceKind;

    type CaseResult<T> = Result<T, String>;
    type PanicPayload = Box<dyn std::any::Any + Send>;

    struct Session { token: String, identity: Value, workspace: String }
    impl Session {
        fn scope(&self) -> CaseResult<ResourceScope> {
            let id = |key| Uuid::parse_str(field(&self.identity, key)?).map_err(|e| e.to_string());
            Ok(ResourceScope::new(OwnerAccountId::from_uuid(id("account_id")?),
                ActorPrincipalId::from_uuid(id("principal_id")?))
                .with_session(AuthenticatedSessionRef::from_uuid(id("session_id")?))
                .with_access_space(AccessSpaceRef::from_uuid(id("access_space_id")?))
                .with_workspace(WorkspaceScopeRef::new(&self.workspace).map_err(|e| e.to_string())?))
        }
        fn authorization(&self, channel: &str, action: ResourceAction) -> AuthorizationRequest {
            AuthorizationRequest { session_token: self.token.clone(),
                channel_binding_hash: Some(hex::encode(Sha256::digest(channel.as_bytes()))),
                capability_id: if action == ResourceAction::Read { "fs.read" } else { "fs.write" }.into(),
                resource_kind: ResourceKind::Workspace, external_resource_id: self.workspace.clone(), action }
        }
    }
    fn field<'a>(value: &'a Value, key: &str) -> CaseResult<&'a str> {
        value.get(key).and_then(Value::as_str).filter(|v| !v.is_empty())
            .ok_or_else(|| format!("canonical response lacks {key}"))
    }
    fn require(condition: bool, failure: &str) -> CaseResult<()> {
        if condition { Ok(()) } else { Err(failure.into()) }
    }

    struct Consumer {
        owner: RefCell<Option<tokio::sync::oneshot::Receiver<Session>>>,
        member: RefCell<Option<tokio::sync::oneshot::Receiver<Session>>>,
        selector: RefCell<Option<tokio::sync::oneshot::Sender<CaseResult<String>>>>,
        result: RefCell<Option<tokio::sync::oneshot::Sender<CaseResult<()>>>>,
        panic: RefCell<Option<PanicPayload>>,
        confirmation: Uuid,
        channel: String,
    }
    impl Consumer {
        async fn observe_administration_resource(&self, state: &handshake_core::AppState, owner: &Session)
            -> CaseResult<String> {
            let inspector = state.surreal.test_inspector();
            let table = inspector.table_selector("kernel_event_ledger").await.map_err(|e| e.to_string())?;
            let names = ["source_component", "event_type", "aggregate_id", "payload"];
            let fields = names.iter().map(|name| table.field(name)).collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let rows = inspector.project(&table, &fields, RowFilter::FieldEquals {
                field: table.field("idempotency_key").map_err(|e| e.to_string())?,
                value: ScalarValue::String(format!("first-owner-confirmation:{}", self.confirmation)),
            }).await.map_err(|e| e.to_string())?;
            require(rows.len() == 1, "confirmation canonical event must be unique")?;
            let row = &rows[0].values;
            require(row.get("source_component").and_then(Value::as_str) == Some("local_account_authority")
                && row.get("event_type").and_then(Value::as_str) == Some("LOCAL_ACCOUNT_SETUP")
                && row.get("aggregate_id").and_then(Value::as_str) == Some(field(&owner.identity, "account_id")?),
                "confirmation event source/action/owner differs")?;
            let payload = row.get("payload").ok_or("canonical confirmation payload missing")?;
            require(field(payload, "phase")? == "confirmation" && field(payload, "result")? == "allow"
                && field(payload, "action")? == "LOCAL_ACCOUNT_SETUP"
                && field(payload, "operation_id")? == self.confirmation.to_string()
                && field(payload, "first_workspace_operation_id")? == owner.workspace,
                "confirmation payload does not identify the actual successful original operation")?;
            let actor = payload.get("actor_scope").ok_or("confirmation actor scope absent")?;
            for key in ["account_id", "principal_id", "session_id", "access_space_id"] {
                require(field(actor, key)? == field(&owner.identity, key)?, "confirmation actor provenance differs")?;
            }
            require(field(actor, "workspace_id")? == owner.workspace, "confirmation workspace differs")?;
            let resource = field(payload, "administration_resource_id")?;
            let refs = payload.get("resource_refs").and_then(Value::as_array).ok_or("confirmation refs absent")?;
            let actual: Vec<_> = refs.iter().filter(|r| r.get("kind").and_then(Value::as_str) == Some("account_administration")).collect();
            require(actual.len() == 1 && field(actual[0], "id")? == resource,
                "confirmation administration resource/ref disagree")?;
            Ok(resource.into()) // Selector only: the subsequent public issue still authenticates independently.
        }

        async fn observe_member_state(&self, state: &handshake_core::AppState, member: &Session,
            expected_sources: &BTreeMap<String, String>) -> CaseResult<(Vec<handshake_core::storage::surreal::ProjectedRow>, Vec<handshake_core::storage::surreal::ProjectedRow>)> {
            // Independent acceptance observation; this neither grants nor claims Member raw-content read access.
            let inspector = state.surreal.test_inspector();
            let owner = field(&member.identity, "account_id")?;
            let authority = inspector.table_selector("model_lane_authority").await.map_err(|e| e.to_string())?;
            let names = ["record_kind", "aggregate_id", "run_id", "idempotency_key", "record_json", "search_terms", "event_id", "event_ledger_event_id",
                "event_seq", "event_stream_version", "transaction_seq", "ordering_seq", "created_at", "owner_account_id", "actor_principal_id",
                "authenticated_session_id", "access_space_id", "workspace_id"];
            let fields = names.iter().map(|name| authority.field(name)).collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let mut rows = inspector.project(&authority, &fields, RowFilter::FieldEquals {
                field: authority.field("owner_account_id").map_err(|e| e.to_string())?,
                value: ScalarValue::String(owner.into()),
            }).await.map_err(|e| e.to_string())?;
            rows.retain(|row| row.values.get("workspace_id").and_then(Value::as_str) == Some(member.workspace.as_str())
                && row.values.get("run_id").and_then(Value::as_str) == Some("run-mt004-foreign-append"));
            require(rows.len() == 3, "canonical Member run/lane/artifact row set is incomplete or ambiguous")?;
            let mut kinds = std::collections::BTreeSet::new();
            let mut event_ids = std::collections::BTreeSet::new();
            let mut row_ids = std::collections::BTreeSet::new();
            for row in &rows {
                require(row.record_id.table == "model_lane_authority"
                    && row_ids.insert(row.record_id.key_string().ok_or("foreign canonical record key is not a string")?.to_owned()),
                    "foreign canonical row identity is invalid or duplicated")?;
                let kind = row.values.get("record_kind").and_then(Value::as_str).ok_or("canonical record kind absent")?;
                require(expected_sources.get(kind).map(String::as_str)
                    == row.values.get("aggregate_id").and_then(Value::as_str),
                    "foreign canonical aggregate identity differs from actual Create output")?;
                for (actual, canonical) in [("owner_account_id", "account_id"), ("actor_principal_id", "principal_id"),
                    ("authenticated_session_id", "session_id"), ("access_space_id", "access_space_id")] {
                    require(row.values.get(actual).and_then(Value::as_str) == Some(field(&member.identity, canonical)?),
                        "observed foreign source differs from actual Member session provenance")?;
                }
                kinds.insert(row.values.get("record_kind").and_then(Value::as_str).ok_or("canonical record kind absent")?.to_owned());
                event_ids.insert(row.values.get("event_id").and_then(Value::as_str).ok_or("canonical event identity absent")?.to_owned());
            }
            require(kinds == ["run", "lane", "context_artifact"].into_iter().map(str::to_owned).collect(),
                "canonical foreign source record families differ")?;
            require(event_ids.len() == rows.len(), "foreign source event identities are not unique")?;
            rows.sort_by_key(|row| row.record_id.key_string().expect("validated canonical string key").to_owned());
            let ledger = inspector.table_selector("kernel_event_ledger").await.map_err(|e| e.to_string())?;
            let names = ["event_id", "aggregate_type", "aggregate_id", "event_type", "source_component",
                "payload_hash", "payload", "owner_account_id", "actor_principal_id", "authenticated_session_id",
                "access_space_id", "workspace_id"];
            let fields = names.iter().map(|name| ledger.field(name)).collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            let mut events = inspector.project(&ledger, &fields, RowFilter::FieldEquals {
                field: ledger.field("owner_account_id").map_err(|e| e.to_string())?,
                value: ScalarValue::String(owner.into()),
            }).await.map_err(|e| e.to_string())?;
            events.retain(|row| row.values.get("workspace_id").and_then(Value::as_str) == Some(member.workspace.as_str())
                && row.values.get("event_id").and_then(Value::as_str).is_some_and(|id| event_ids.contains(id)));
            require(events.len() == rows.len(), "canonical foreign source EventLedger row set differs")?;
            let observed: std::collections::BTreeSet<_> = events.iter().filter_map(|row|
                row.values.get("event_id").and_then(Value::as_str).map(str::to_owned)).collect();
            require(observed == event_ids, "canonical foreign source event identities disagree")?;
            let mut ledger_ids = std::collections::BTreeSet::new();
            for event in &events {
                let key = event.record_id.key_string().ok_or("foreign canonical event key is not a string")?;
                require(event.record_id.table == "kernel_event_ledger" && ledger_ids.insert(key.to_owned())
                    && event.values.get("event_id").and_then(Value::as_str) == Some(key),
                    "foreign canonical ledger identity is invalid or duplicated")?;
                require(event.values.get("source_component").and_then(Value::as_str) == Some("model_lane"),
                    "canonical foreign source EventLedger producer differs")?;
                let payload = event.values.get("payload").ok_or("foreign source event payload absent")?;
                require(field(payload, "run_id")? == "run-mt004-foreign-append", "foreign source event run differs")?;
            }
            events.sort_by_key(|row| row.record_id.key_string().expect("validated canonical event string key").to_owned());
            Ok((rows, events))
        }

        async fn append_case(&self, state: &handshake_core::AppState, owner: &Session, member: &Session)
            -> CaseResult<()> {
            let foreign_owner = field(&member.identity, "account_id")?;
            require(foreign_owner != field(&owner.identity, "account_id")?, "Member must be a distinct canonical LocalAccount")?;
            let owner_scope = owner.scope()?;
            let member_scope = member.scope()?;
            let foreign = ModelLaneStore::new_scoped_with_lifecycle(state.surreal.clone(), member_scope.clone(),
                state.surreal.resource_access_lifecycle());
            let create_b = AuthenticatedModelLaneTestOperation::authorize(state.surreal.clone(),
                member.authorization(&self.channel, ResourceAction::Create),
                ExactResourceScopeAttribution::try_from_resource_scope(&member_scope).map_err(|e| e.to_string())?)
                .await.map_err(|e| e.to_string())?;
            let (run, lane, artifact) = create_b.run(async {
                let run = foreign.record_run(sample_run("foreign-append")).await?;
                let lane = foreign.record_lane(sample_lane("foreign-append")).await?;
                let artifact = foreign.record_context_bundle_artifact_binding(sample_artifact_binding("foreign-append")).await?;
                Ok((run, lane, artifact))
            }).await.map_err(|e| e.to_string())?;
            drop(create_b);
            let expected_sources = BTreeMap::from([
                ("run".into(), run.run_id.clone()), ("lane".into(), lane.lane_id.clone()),
                ("context_artifact".into(), artifact.artifact_binding_id.clone()),
            ]);
            let before = self.observe_member_state(state, member, &expected_sources).await?;
            let store = ModelLaneStore::new_scoped_with_lifecycle(state.surreal.clone(), owner_scope.clone(),
                state.surreal.resource_access_lifecycle());
            let create_a = AuthenticatedModelLaneTestOperation::authorize(state.surreal.clone(),
                owner.authorization(&self.channel, ResourceAction::Create),
                ExactResourceScopeAttribution::try_from_resource_scope(&owner_scope).map_err(|e| e.to_string())?)
                .await.map_err(|e| e.to_string())?;
            let inspector = state.surreal.test_inspector();
            let authority_rows = table_count(&inspector, "model_lane_authority").await;
            let ledger = inspector.table_selector("kernel_event_ledger").await.map_err(|e| e.to_string())?;
            let model_events = || RowFilter::FieldEquals {
                field: ledger.field("event_type").expect("schema declares canonical event type"),
                value: ScalarValue::String("MODEL_LANE_AUTHORITY_RECORDED".into()) };
            let ledger_rows = inspector.row_count(&ledger, model_events()).await.map_err(|e| e.to_string())?;
            let total_ledger_rows = inspector.row_count(&ledger, RowFilter::All).await.map_err(|e| e.to_string())?;
            let error = create_a.run(store.record_message(sample_message("foreign-append", "proposal"))).await
                .expect_err("foreign-scope run/lane must not authorize an owner-scope message append");
            let total_ledger_after = inspector.row_count(&ledger, RowFilter::All).await.map_err(|e| e.to_string())?;
            require(total_ledger_after == total_ledger_rows, "denied append reached the canonical EventLedger")?;
            require(!error.to_string().contains(foreign_owner), "denial disclosed the foreign owner account")?;
            require(table_count(&inspector, "model_lane_authority").await == authority_rows,
                "denied append mutated model-lane authority")?;
            require(inspector.row_count(&ledger, model_events()).await.map_err(|e| e.to_string())? == ledger_rows,
                "denied append appended a canonical model-lane EventLedger event")?;
            require(self.observe_member_state(state, member, &expected_sources).await? == before,
                "denied append changed canonical foreign run/lane/artifact data or linked events")?;
            Ok(())
        }
    }
    impl RuntimeConsumer for Consumer {
        fn consume<'a>(&'a self, state: &'a handshake_core::AppState,
            mut cancelled: tokio::sync::watch::Receiver<bool>) -> handshake_core::backend_runtime::ConsumerFuture<'a> {
            Box::pin(async move {
                let owner_rx = self.owner.borrow_mut().take().ok_or("owner input already consumed")?;
                let member_rx = self.member.borrow_mut().take().ok_or("Member input already consumed")?;
                let result = std::panic::AssertUnwindSafe(async {
                    let owner = tokio::select! { result = owner_rx => result.map_err(|e| e.to_string())?,
                        _ = cancelled.changed() => return Ok(()) };
                    let resource = self.observe_administration_resource(state, &owner).await;
                    let resource_failed = resource.is_err();
                    if let Some(sender) = self.selector.borrow_mut().take() { let _ = sender.send(resource); }
                    if resource_failed { return Err("canonical administration selector observation failed".into()); }
                    let member = tokio::select! { result = member_rx => result.map_err(|e| e.to_string())?,
                        _ = cancelled.changed() => return Ok(()) };
                    self.append_case(state, &owner, &member).await
                }).catch_unwind().await;
                let result = match result { Ok(result) => result, Err(payload) => {
                    self.panic.borrow_mut().replace(payload); Err("cross-account consumer assertion failed".into()) } };
                if let Some(sender) = self.result.borrow_mut().take() { let _ = sender.send(result.clone()); }
                // Keep the mounted owner available for real logout even when the proof failed.
                if !*cancelled.borrow() { let _ = cancelled.changed().await; }
                result
            })
        }
    }

    struct Http<'a> { client: reqwest::Client, base: String, channel: &'a str, deadline: tokio::time::Instant }
    impl Http<'_> {
        async fn request(&self, method: reqwest::Method, path: &str, token: Option<&str>, body: Value) -> CaseResult<Value> {
            let mut request = self.client.request(method, format!("{}{path}", self.base))
                .header("x-hsk-channel-binding-token", self.channel)
                .timeout(self.deadline.saturating_duration_since(tokio::time::Instant::now())).json(&body);
            if let Some(token) = token { request = request.header("x-hsk-session-token", token); }
            let response = request.send().await.map_err(|e| e.to_string())?;
            if !response.status().is_success() { return Err(format!("public {path} denied with {}", response.status())); }
            response.json().await.map_err(|e| e.to_string())
        }
        async fn exchange(&self, credential: Value, tokens: &mut Vec<String>) -> CaseResult<(String, Value)> {
            let identity = self.request(reqwest::Method::POST, "/authority/session", None, json!({
                "account_id":field(&credential,"account_id")?,"principal_id":field(&credential,"principal_id")?,
                "access_space_id":field(&credential,"access_space_id")?,"authentication_token":field(&credential,"token")?
            })).await?;
            let token = field(&identity, "session_token")?.to_owned();
            tokens.push(token.clone()); // Install cleanup custody immediately after actual exchange.
            Ok((token, identity))
        }
    }

    pub(super) async fn run() -> CaseResult<()> {
        let root = surreal_test_store_support::canonical_artifacts_root_for_proof().map_err(|e| e.to_string())?;
        let directory = tempfile::tempdir_in(root).map_err(|e| e.to_string())?;
        let channel = authenticated_promotion_fixture::NativeChannel::create(directory.path());
        let config = ControlPlaneStorageConfig { mode: ControlPlaneStorageMode::SurrealEmbedded,
            data_dir: Some(directory.path().join("cross-account-runtime")) };
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        require(address.ip().is_loopback(), "runtime proof listener is not owned loopback")?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
        let http = Http { client: reqwest::Client::new(), base: format!("http://{address}"), channel: &channel.token, deadline };
        let (owner_tx, owner_rx) = tokio::sync::oneshot::channel();
        let (member_tx, member_rx) = tokio::sync::oneshot::channel();
        let (selector_tx, selector_rx) = tokio::sync::oneshot::channel();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();
        let confirmation = Uuid::now_v7();
        let consumer = Arc::new(Consumer { owner: RefCell::new(Some(owner_rx)), member: RefCell::new(Some(member_rx)),
            selector: RefCell::new(Some(selector_tx)), result: RefCell::new(Some(result_tx)), panic: RefCell::new(None),
            confirmation, channel: channel.token.clone() });
        let local = tokio::task::LocalSet::new();
        local.run_until(async {
            let mut runtime = RuntimeHandle::start(RuntimeOptions { listener: Some(listener), storage: config }, Some(consumer.clone()));
            let mut tokens = Vec::new();
            let case = std::panic::AssertUnwindSafe(tokio::time::timeout_at(deadline, async {
                use reqwest::Method;
                loop {
                    let response = http.client.get(format!("{}/authority/setup", http.base))
                        .header("x-hsk-channel-binding-token", http.channel)
                        .timeout(deadline.saturating_duration_since(tokio::time::Instant::now())).send().await;
                    match response {
                        Ok(response) if response.status().is_success() => {
                            let setup: Value = response.json().await.map_err(|e| e.to_string())?;
                            require(setup["setup_required"] == true, "fresh runtime must require actual one-time setup")?;
                            break;
                        }
                        Ok(response) if response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE
                            && tokio::time::Instant::now() < deadline => {},
                        Ok(response) => return Err(format!("canonical setup readiness rejected with {}", response.status())),
                        Err(_) if tokio::time::Instant::now() < deadline => {},
                        Err(error) => return Err(error.to_string()),
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                let name = "MT004 canonical cross-account owner";
                let password = "isolated actual cross-account operator password";
                let credentials = json!({"account_name":name,"password":password});
                http.request(Method::POST, "/authority/setup", None, credentials.clone()).await?;
                let credential = http.request(Method::POST, "/authority/login", None, credentials.clone()).await?;
                let (owner_token, owner_identity) = http.exchange(credential, &mut tokens).await?;
                let workspace = http.request(Method::POST, "/workspaces", Some(&owner_token), json!({"name":"Actual cross-account Owner workspace"})).await?;
                let workspace_id = field(&workspace, "id")?.to_owned();
                Uuid::parse_str(&workspace_id).map_err(|e| e.to_string())?;
                http.request(Method::POST, "/authority/member-enrollment-administrator/confirmation", Some(&owner_token), json!({
                    "operation_id":confirmation,"first_workspace_operation_id":workspace_id,
                    "parent_workspace_external_id":workspace_id,"lifetime":{"mode":"no_expiry"},"password":password
                })).await?;
                // Publish only this genuinely authenticated owning workspace to activate the existing runtime.
                let activation = http.request(Method::POST, "/authority/session", Some(&owner_token),
                    json!({"workspace_id":workspace_id,"operation_id":Uuid::now_v7()})).await?;
                for (key, expected) in [("owner_account_id","account_id"),("actor_principal_id","principal_id"),
                    ("authenticated_session_id","session_id"),("access_space_id","access_space_id")] {
                    require(field(&activation,key)? == field(&owner_identity,expected)?, "runtime activation differs from original Owner session")?;
                }
                require(field(&activation,"workspace_id")? == workspace_id, "runtime activated a different workspace")?;
                owner_tx.send(Session { token: owner_token, identity: owner_identity, workspace: workspace_id.clone() })
                    .map_err(|_| "activated runtime Owner consumer unavailable".to_owned())?;
                let administration_resource = selector_rx.await.map_err(|e| e.to_string())??;
                let admin_credential = http.request(Method::POST, "/authority/login", None,
                    json!({"account_name":name,"password":password,"target":"member_enrollment_administrator"})).await?;
                let (admin_token, _) = http.exchange(admin_credential, &mut tokens).await?;
                let member_name = "MT004 enrolled distinct Member";
                let issued = http.request(Method::POST, "/authority/member-enrollments", Some(&admin_token), json!({
                    "request_id":Uuid::now_v7(),"account_name":member_name,"workspace_id":workspace_id,
                    "administrator_resource":administration_resource,"current_password":password
                })).await?;
                let operation = field(&issued,"operation_id")?;
                let member_password = "isolated actual enrolled Member password";
                http.request(Method::POST, &format!("/authority/member-enrollments/{operation}/redeem"), None,
                    json!({"invitation_secret":field(&issued,"invitation_secret")?,"password":member_password})).await?;
                let credential = http.request(Method::POST, "/authority/login", None,
                    json!({"account_name":member_name,"password":member_password})).await?;
                let (member_token, member_identity) = http.exchange(credential, &mut tokens).await?;
                let workspace = http.request(Method::POST, "/workspaces", Some(&member_token),
                    json!({"name":"Actual distinct Member private workspace"})).await?;
                member_tx.send(Session { token: member_token, identity: member_identity,
                    workspace: field(&workspace,"id")?.into() }).map_err(|_| "activated Member consumer unavailable".to_owned())?;
                result_rx.await.map_err(|e| e.to_string())?
            })).catch_unwind().await;
            // Session/vault cleanup runs before the production owner is asked to retire.
            let mut cleanup = Vec::new();
            for token in tokens.iter().rev() {
                if let Err(error) = http.request(reqwest::Method::POST, "/authority/logout", Some(token), Value::Null).await {
                    cleanup.push(error);
                } else {
                    match http.client.get(format!("{}/authority/session", http.base))
                        .header("x-hsk-channel-binding-token", http.channel)
                        .header("x-hsk-session-token", token)
                        .timeout(deadline.saturating_duration_since(tokio::time::Instant::now())).send().await {
                        Ok(response) if response.status() == reqwest::StatusCode::FORBIDDEN => {},
                        Ok(_) => cleanup.push("logged-out cross-account session remained readable".into()),
                        Err(error) => cleanup.push(error.to_string()),
                    }
                }
            }
            let shutdown = runtime.shutdown_at(deadline).await;
            if shutdown.is_err() || !runtime.retirement_settled() {
                // Keep this exact owner and LocalSet alive; timeout/error never proves retirement.
                if let Err(error) = runtime.join_retained_owner().await { cleanup.push(error); }
            }
            if let Err(error) = shutdown { cleanup.push(error); }
            if !runtime.retirement_settled() {
                eprintln!("cross-account proof retirement remains UNKNOWN; retaining the exact runtime, LocalSet and owned store");
                std::future::pending::<()>().await;
                unreachable!("unsettled production custody cannot be dropped");
            }
            if let Some(primary) = consumer.panic.borrow_mut().take() {
                std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
                    primary: Some(primary), cleanup: if cleanup.is_empty() { None } else { Some(Box::new(cleanup)) },
                }));
            }
            match case {
                Err(primary) => std::panic::resume_unwind(Box::new(PromotionFixtureFailure {
                    primary: Some(primary), cleanup: if cleanup.is_empty() { None } else { Some(Box::new(cleanup)) },
                })),
                Ok(Err(error)) => Err(format!("cross-account proof deadline elapsed; operation settlement UNKNOWN: {error}; cleanup={cleanup:?}")),
                Ok(Ok(result)) => {
                    if cleanup.is_empty() { result } else { Err(format!("case={result:?}; cleanup={cleanup:?}")) }
                }
            }
        }).await
    }
}
