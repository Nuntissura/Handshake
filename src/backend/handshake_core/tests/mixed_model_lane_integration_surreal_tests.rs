//! MT-018 mixed ModelLane/CRDT scope, restart, and atomic-denial proof.

mod surreal_test_store_support;

use handshake_core::kernel::crdt::{
    actor_site::{derive_knowledge_site_id, KnowledgeActorIdV1},
    state_vector::KnowledgeStateVectorV1,
};
use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneCrdtProposalDecision, ModelLaneCrdtUpdateAppendOutcome, ModelLaneStore,
    NewModelLaneCrdtProposal, NewModelLaneCrdtUpdate,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, ExactResourceScopeAttribution,
    OwnerAccountId, ResourceAccessContext, ResourceScope, WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use surreal_test_store_support::EmbeddedSurrealTestScope;

struct Harness {
    isolated: EmbeddedSurrealTestScope,
    storage: SurrealStorage,
    scope: ResourceScope,
    store: ModelLaneStore,
}

impl Harness {
    async fn create(label: &str) -> Self {
        let mut isolated = EmbeddedSurrealTestScope::create()
            .await
            .expect("allocate MT-018 mixed embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate shared production SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical embedded schema");
        let scope = exact_scope(label);
        let store = ModelLaneStore::new_scoped(storage.clone(), scope.clone());
        Self {
            isolated,
            storage,
            scope,
            store,
        }
    }

    async fn posture(&self, label: &str) -> SurrealAdmissibleCrdtPosture {
        build_surreal_admissible_crdt_posture(
            &self.store,
            self.scope
                .workspace
                .as_ref()
                .expect("exact workspace")
                .as_str(),
            label,
        )
        .await
        .expect("build production ModelLane/CRDT posture")
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("cleanup exact mixed embedded scope");
    }
}

#[tokio::test]
async fn mt018_distinct_diff_and_yjs_hashes_are_replay_stable() {
    let mut harness = Harness::create("mixed-restart").await;
    let posture = harness.posture("mixed-restart").await;
    assert_ne!(posture.approved_diff_sha256, posture.yjs_update_sha256);
    let message = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("admit mixed ModelLane/CRDT proposal");

    drop(harness.store);
    drop(harness.storage);
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close shared namespace/database");
    harness
        .isolated
        .reopen()
        .await
        .expect("reopen same namespace/database");
    let storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate shared storage");
    let store = ModelLaneStore::new_scoped(storage.clone(), harness.scope.clone());
    let proposal = store
        .crdt_proposal(&posture.proposal.proposal_id)
        .await
        .expect("replay proposal")
        .expect("proposal survives reopen");
    assert_eq!(proposal.diff_sha256, posture.approved_diff_sha256);
    assert_eq!(
        store
            .record_message(posture.message)
            .await
            .expect("idempotent message replay after reopen"),
        message
    );
    harness.store = store;
    harness.storage = storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn mt018_scope_stale_revoked_mixed_and_incomplete_rows_deny_without_mutation_or_leakage() {
    let harness = Harness::create("scope-denials").await;
    let posture = harness.posture("scope-denials").await;
    let baseline = harness
        .store
        .test_crdt_authority_counts()
        .await
        .expect("capture exact authority watermark");

    for (dimension, denied_scope) in one_field_mismatches(&harness.scope) {
        let denied = ModelLaneStore::new_scoped(harness.storage.clone(), denied_scope);
        assert!(
            denied
                .crdt_proposal(&posture.proposal.proposal_id)
                .await
                .expect("foreign exact read is non-leaking")
                .is_none(),
            "{dimension} mismatch must not reveal proposal metadata"
        );
        assert_eq!(
            denied
                .test_crdt_authority_counts()
                .await
                .expect("foreign scoped count"),
            Default::default(),
            "{dimension} mismatch must expose zero exact rows"
        );
        let mut message = posture.message.clone();
        message.message_id = format!("{}-{dimension}", message.message_id);
        message.idempotency_key = format!("{}-{dimension}", message.idempotency_key);
        assert!(
            denied.record_message(message).await.is_err(),
            "{dimension} mismatch must deny before mutation"
        );
    }

    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&harness.scope)
        .expect("complete exact scope");
    let revoked_writer = ModelLaneStore::new(
        harness.storage.clone(),
        ResourceAccessContext::for_exact_reader(exact),
    );
    assert!(revoked_writer
        .crdt_proposal(&posture.proposal.proposal_id)
        .await
        .expect("revoked write context keeps exact read")
        .is_some());
    let mut revoked_message = posture.message.clone();
    revoked_message.message_id.push_str("-revoked");
    revoked_message.idempotency_key.push_str("-revoked");
    assert!(
        revoked_writer
            .record_message(revoked_message)
            .await
            .is_err(),
        "read-only/revoked context must not regain write authority"
    );

    let incomplete = ResourceScope::new(
        harness.scope.owner_account_id,
        harness.scope.actor_principal_id,
    );
    let incomplete_store = ModelLaneStore::new_scoped(harness.storage.clone(), incomplete);
    assert!(
        incomplete_store
            .crdt_proposal(&posture.proposal.proposal_id)
            .await
            .is_err(),
        "incomplete attribution must fail before querying"
    );
    assert_eq!(
        harness
            .store
            .test_crdt_authority_counts()
            .await
            .expect("verify original watermark after all denials"),
        baseline
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn mt018_duplicate_conflict_and_binding_to_another_real_update_are_atomic_denials() {
    let harness = Harness::create("atomic-denials").await;
    let posture = harness.posture("atomic-denials").await;
    let actor = KnowledgeActorIdV1::parse(&posture.actor_id).expect("canonical CRDT actor");
    let workspace_id = harness
        .scope
        .workspace
        .as_ref()
        .expect("exact workspace")
        .as_str();
    let site = derive_knowledge_site_id(workspace_id, &posture.crdt_document_id, &actor);
    let mut vector = KnowledgeStateVectorV1::parse(&posture.update.state_vector_after)
        .expect("parse current state vector");
    let before = vector.encode();
    vector.increment(&site.site_id);
    let other_update = match harness
        .store
        .append_crdt_update(NewModelLaneCrdtUpdate {
            schema_id: "hsk.kernel.crdt_update@1".into(),
            document_id: posture.document_id.clone(),
            crdt_document_id: posture.crdt_document_id.clone(),
            update_id: "update-mt018-another-real".into(),
            update_seq: 3,
            update_bytes: posture
                .next_yjs_update_bytes("[mt018-another-real]")
                .expect("generate causally new real Yjs update"),
            actor_id: posture.actor_id.clone(),
            actor_kind: posture.actor_kind.clone(),
            session_id: posture.session_id.clone(),
            trace_id: posture.trace_id.clone(),
            state_vector_before: before,
            state_vector_after: vector.encode(),
            replay_order_key: "0003-another-real".into(),
            dependency_update_ids: vec![posture.update.update_id.clone()],
            site_id: site.site_id,
            kernel_task_run_id: posture.run_id.clone(),
            idempotency_key: "mt018-another-real-update".into(),
        })
        .await
        .expect("append another real Yjs row through production API")
    {
        ModelLaneCrdtUpdateAppendOutcome::Stored(row) => row,
        other => panic!("expected stored third update, got {other:?}"),
    };

    let proposal_input = NewModelLaneCrdtProposal {
        proposal_id: "proposal-mt018-other-binding".into(),
        document_id: posture.document_id.clone(),
        crdt_document_id: posture.crdt_document_id.clone(),
        base_update_seq: 1,
        base_state_vector: posture.update.state_vector_before.clone(),
        proposed_diff: json!({"op": "append_text", "value": "approved-other"}),
        source_span_citations: vec!["span://mt018/other-binding".into()],
        actor_id: posture.actor_id.clone(),
        actor_kind: posture.actor_kind.clone(),
        session_id: posture.session_id.clone(),
        correlation_id: posture.trace_id.clone(),
        lease_id: posture.lease.lease_id.clone(),
        kernel_task_run_id: posture.run_id.clone(),
        idempotency_key: "mt018-other-binding-record".into(),
    };
    let proposal = harness
        .store
        .record_crdt_proposal(proposal_input.clone())
        .await
        .expect("record second proposal");
    harness
        .store
        .decide_crdt_proposal(
            &proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Approved,
            "reviewer-mt018-other",
            None,
            &posture.run_id,
            "review-session-mt018-other",
            "mt018-other-binding-approve",
        )
        .await
        .expect("approve second proposal");
    let before_denial = harness
        .store
        .test_crdt_authority_counts()
        .await
        .expect("watermark before atomic denial");
    assert!(
        harness
            .store
            .bind_crdt_proposal_update(
                &proposal.proposal_id,
                &other_update.update_id,
                &posture.run_id,
                "mt018-other-binding-apply",
            )
            .await
            .is_err(),
        "a proposal based at seq 1 cannot bind a different real seq 3 update"
    );
    assert_eq!(
        harness
            .store
            .test_crdt_authority_counts()
            .await
            .expect("watermark after denied binding"),
        before_denial
    );

    let mut conflicting = proposal_input;
    conflicting.proposed_diff = json!({"op": "replace", "value": "conflict"});
    assert!(
        harness
            .store
            .record_crdt_proposal(conflicting)
            .await
            .is_err(),
        "duplicate proposal identity with different immutable diff must fail"
    );
    assert_eq!(
        harness
            .store
            .test_crdt_authority_counts()
            .await
            .expect("watermark after duplicate conflict"),
        before_denial
    );
    harness.cleanup().await;
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<(&'static str, ResourceScope)> {
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::mint();
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::mint();
    let mut session = scope.clone();
    session.authenticated_session = Some(AuthenticatedSessionRef::mint());
    let mut access_space = scope.clone();
    access_space.access_space = Some(AccessSpaceRef::mint());
    let mut workspace = scope.clone();
    workspace.workspace = Some(
        WorkspaceScopeRef::new("workspace-mt018-foreign").expect("nonblank foreign workspace"),
    );
    vec![
        ("owner_account_id", owner),
        ("actor_principal_id", actor),
        ("authenticated_session_id", session),
        ("access_space_id", access_space),
        ("workspace_id", workspace),
    ]
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt018-{label}"))
                .expect("nonblank exact workspace"),
        )
}
