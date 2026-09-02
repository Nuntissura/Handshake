//! MT-018 production proposal lifecycle over one embedded SurrealDB scope.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    ModelLaneCrdtProposalDecision, ModelLaneCrdtTestCorruption, ModelLaneStore,
    NewModelLaneCrdtProposal,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
};
use handshake_core::test_harness::crdt_workspace::{
    build_surreal_admissible_crdt_posture, SurrealAdmissibleCrdtPosture,
};
use serde_json::json;
use sha2::{Digest, Sha256};
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
            .expect("allocate exact MT-018 embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production SurrealStorage");
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
        .expect("build production-only admissible CRDT posture")
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated
            .cleanup()
            .await
            .expect("clean exact MT-018 embedded scope");
    }
}

#[tokio::test]
async fn mt018_proposal_kind_crdt_message_with_approved_applied_proposal_is_admitted() {
    let mut harness = Harness::create("positive-replay").await;
    let posture = harness.posture("positive-replay").await;

    assert_eq!(
        posture.approved_diff_sha256,
        sha256_hex(&posture.approved_diff_bytes)
    );
    assert_eq!(
        posture.yjs_update_sha256,
        sha256_hex(&posture.yjs_update_bytes)
    );
    assert_ne!(posture.approved_diff_sha256, posture.yjs_update_sha256);
    assert_eq!(
        posture.proposal.applied_update_id.as_deref(),
        Some(posture.update.update_id.as_str())
    );
    assert_eq!(
        posture.proposal.applied_update_sha256.as_deref(),
        Some(posture.approved_diff_sha256.as_str())
    );

    let stored = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("admit Proposal-kind message through production authority path");
    assert!(stored.crdt_authority_binding.is_some());
    let replay = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("identical message retry is stable");
    assert_eq!(stored, replay);

    let before_promotion = harness
        .store
        .test_crdt_authority_counts()
        .await
        .expect("count exact authority before atomic promotion");
    let promoted = harness
        .store
        .decide_crdt_proposal(
            &posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted,
            "promotion-gate-mt018",
            Some("accepted after approved Yjs binding".into()),
            &posture.run_id,
            "promotion-session-mt018",
            "mt018-positive-promotion",
        )
        .await
        .expect("promote applied proposal atomically")
        .expect("proposal remains present");
    assert_eq!(promoted.review_state, "promoted");
    assert_eq!(
        promoted.applied_update_sha256.as_deref(),
        Some(promoted.diff_sha256.as_str())
    );
    assert_ne!(
        promoted.promotion_requested_event_id,
        promoted.promotion_accepted_event_id
    );
    assert_eq!(
        promoted.last_transition_event_id,
        promoted
            .promotion_accepted_event_id
            .clone()
            .expect("promoted proposal carries accepted receipt")
    );
    let after_promotion = harness
        .store
        .test_crdt_authority_counts()
        .await
        .expect("count exact authority after atomic promotion");
    assert_eq!(
        after_promotion.proposal_rows,
        before_promotion.proposal_rows
    );
    assert_eq!(after_promotion.event_rows, before_promotion.event_rows + 2);
    let promotion_retry = harness
        .store
        .decide_crdt_proposal(
            &posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted,
            "promotion-gate-mt018",
            Some("accepted after approved Yjs binding".into()),
            &posture.run_id,
            "promotion-session-mt018",
            "mt018-positive-promotion",
        )
        .await
        .expect("retry atomic promotion")
        .expect("promoted proposal remains present");
    assert_eq!(promotion_retry, promoted);
    assert_eq!(
        harness
            .store
            .test_crdt_authority_counts()
            .await
            .expect("count exact authority after promotion retry"),
        after_promotion
    );

    drop(harness.store);
    drop(harness.storage);
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close production storage before same-scope reopen");
    harness
        .isolated
        .reopen()
        .await
        .expect("reopen exact namespace/database");
    let reopened_storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate same production storage");
    let reopened = ModelLaneStore::new_scoped(reopened_storage.clone(), harness.scope.clone());
    let replayed = reopened
        .crdt_proposal(&posture.proposal.proposal_id)
        .await
        .expect("read proposal after restart")
        .expect("proposal survives restart");
    assert_eq!(replayed.review_state, "promoted");
    assert_eq!(
        replayed.promotion_requested_event_id,
        promoted.promotion_requested_event_id
    );
    assert_eq!(
        replayed.promotion_accepted_event_id,
        promoted.promotion_accepted_event_id
    );
    assert_eq!(
        reopened
            .record_message(posture.message)
            .await
            .expect("message idempotency survives restart"),
        stored
    );
    harness.store = reopened;
    harness.storage = reopened_storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn mt018_proposal_admission_denies_missing_fabricated_stale_foreign_and_rejected_authority() {
    let harness = Harness::create("admission-negatives").await;
    let posture = harness.posture("admission-negatives").await;
    let baseline = harness
        .store
        .test_crdt_authority_counts()
        .await
        .expect("count exact authority before denials");

    let mut missing = posture.message.clone();
    missing.message_id.push_str("-missing");
    missing.idempotency_key.push_str("-missing");
    missing.crdt_proposal_ref = None;
    assert_denied(&harness.store, missing, "missing proposal binding").await;

    let mut fabricated = posture.message.clone();
    fabricated.message_id.push_str("-fabricated");
    fabricated.idempotency_key.push_str("-fabricated");
    fabricated.crdt_proposal_ref = Some("crdt-proposal://fabricated-mt018".into());
    assert_denied(&harness.store, fabricated, "fabricated proposal ref").await;

    let mut stale_vector = posture.message.clone();
    stale_vector.message_id.push_str("-stale-vector");
    stale_vector.idempotency_key.push_str("-stale-vector");
    stale_vector.crdt_state_vector = Some("hsk-sv1:stale-site=99".into());
    assert_denied(&harness.store, stale_vector, "stale state vector").await;

    let foreign = harness.posture("admission-foreign").await;
    let mut unrelated_snapshot = posture.message.clone();
    unrelated_snapshot.message_id.push_str("-snapshot");
    unrelated_snapshot.idempotency_key.push_str("-snapshot");
    unrelated_snapshot.crdt_base_snapshot_ref = foreign.message.crdt_base_snapshot_ref.clone();
    assert_denied(&harness.store, unrelated_snapshot, "unrelated snapshot").await;

    let mut foreign_update = posture.message.clone();
    foreign_update.message_id.push_str("-foreign-update");
    foreign_update.idempotency_key.push_str("-foreign-update");
    foreign_update.crdt_update_ref = foreign.message.crdt_update_ref.clone();
    assert_denied(
        &harness.store,
        foreign_update,
        "wrong actor/session/trace/document update",
    )
    .await;

    let mut wrong_trace = posture.message.clone();
    wrong_trace.message_id.push_str("-trace");
    wrong_trace.idempotency_key.push_str("-trace");
    wrong_trace.linked_span_contexts.clear();
    assert_denied(&harness.store, wrong_trace, "missing linked trace").await;

    let pending_id = "proposal-mt018-pending-negative";
    let pending = harness
        .store
        .record_crdt_proposal(NewModelLaneCrdtProposal {
            proposal_id: pending_id.into(),
            document_id: posture.document_id.clone(),
            crdt_document_id: posture.crdt_document_id.clone(),
            base_update_seq: 1,
            base_state_vector: posture.update.state_vector_before.clone(),
            proposed_diff: json!({"op": "replace", "value": "pending"}),
            source_span_citations: vec!["span://mt018/pending".into()],
            actor_id: posture.actor_id.clone(),
            actor_kind: posture.actor_kind.clone(),
            session_id: posture.session_id.clone(),
            correlation_id: posture.trace_id.clone(),
            lease_id: posture.lease.lease_id.clone(),
            kernel_task_run_id: posture.run_id.clone(),
            idempotency_key: "mt018-pending-record".into(),
        })
        .await
        .expect("record pending proposal through production API");
    let mut pending_message = posture.message.clone();
    pending_message.message_id.push_str("-pending");
    pending_message.idempotency_key.push_str("-pending");
    pending_message.crdt_proposal_ref = Some(format!("crdt-proposal://{}", pending.proposal_id));
    assert_denied(
        &harness.store,
        pending_message.clone(),
        "unapproved proposal",
    )
    .await;
    harness
        .store
        .decide_crdt_proposal(
            &pending.proposal_id,
            ModelLaneCrdtProposalDecision::Rejected,
            "reviewer-mt018-negative",
            Some("counterfactual rejection".into()),
            &posture.run_id,
            "review-session-mt018-negative",
            "mt018-pending-reject",
        )
        .await
        .expect("reject proposal through production API");
    pending_message.message_id.push_str("-rejected");
    pending_message.idempotency_key.push_str("-rejected");
    assert_denied(&harness.store, pending_message, "rejected proposal").await;

    let after = harness
        .store
        .test_crdt_authority_counts()
        .await
        .expect("count exact authority after denials");
    assert_eq!(after.update_rows, baseline.update_rows + 2);
    assert_eq!(after.snapshot_rows, baseline.snapshot_rows + 1);
    assert_eq!(after.lease_rows, baseline.lease_rows + 1);
    assert_eq!(after.proposal_rows, baseline.proposal_rows + 2);
    harness.cleanup().await;
}

#[tokio::test]
async fn mt018_hash_and_receipt_tampering_fail_closed_without_denied_write() {
    for (suffix, corruption) in [
        (
            "recorded-receipt",
            ModelLaneCrdtTestCorruption::RecordedReceiptAggregate,
        ),
        (
            "applied-receipt",
            ModelLaneCrdtTestCorruption::AppliedReceiptPayloadHash,
        ),
        ("diff-hash", ModelLaneCrdtTestCorruption::ProposalDiffHash),
        ("yjs-hash", ModelLaneCrdtTestCorruption::UpdateContentHash),
        (
            "incomplete",
            ModelLaneCrdtTestCorruption::ProposalIncompleteAttribution,
        ),
        (
            "mixed",
            ModelLaneCrdtTestCorruption::AppliedReceiptMixedScope,
        ),
        ("actor", ModelLaneCrdtTestCorruption::ProposalActorIdentity),
        (
            "session",
            ModelLaneCrdtTestCorruption::ProposalSessionIdentity,
        ),
        ("trace", ModelLaneCrdtTestCorruption::ProposalTraceIdentity),
        (
            "document",
            ModelLaneCrdtTestCorruption::ProposalDocumentIdentity,
        ),
    ] {
        let harness = Harness::create(suffix).await;
        let posture = harness.posture(suffix).await;
        harness
            .store
            .test_corrupt_crdt_proposal_authority(
                &posture.proposal.proposal_id,
                &posture.update.update_id,
                corruption,
            )
            .await
            .expect("apply enumerated corruption through narrow test seam");
        let before = harness
            .store
            .test_crdt_authority_counts()
            .await
            .expect("count rows after deliberate corruption");
        assert!(
            harness.store.record_message(posture.message).await.is_err(),
            "{suffix} corruption must deny admission"
        );
        let after = harness
            .store
            .test_crdt_authority_counts()
            .await
            .expect("count rows after denied admission");
        assert_eq!(before, after, "{suffix} denial must be non-mutating");
        harness.cleanup().await;
    }
}

#[tokio::test]
async fn mt018_promotion_receipt_pair_rejects_broken_causation_without_mutation() {
    let harness = Harness::create("promotion-causation").await;
    let posture = harness.posture("promotion-causation").await;
    harness
        .store
        .decide_crdt_proposal(
            &posture.proposal.proposal_id,
            ModelLaneCrdtProposalDecision::Promoted,
            "promotion-gate-mt018-causation",
            Some("counterfactual receipt proof".into()),
            &posture.run_id,
            "promotion-session-mt018-causation",
            "mt018-promotion-causation",
        )
        .await
        .expect("promote proposal before receipt counterfactual")
        .expect("promoted proposal remains present");
    harness
        .store
        .test_corrupt_crdt_proposal_authority(
            &posture.proposal.proposal_id,
            &posture.update.update_id,
            ModelLaneCrdtTestCorruption::PromotionAcceptedCausation,
        )
        .await
        .expect("break accepted receipt causation through enumerated seam");
    let before = harness
        .store
        .test_crdt_authority_counts()
        .await
        .expect("count authority after deliberate receipt corruption");
    assert!(
        harness
            .store
            .crdt_proposal(&posture.proposal.proposal_id)
            .await
            .is_err(),
        "broken promotion causation must fail closed"
    );
    assert!(
        harness.store.record_message(posture.message).await.is_err(),
        "broken promotion causation must deny Proposal-kind admission"
    );
    assert_eq!(
        harness
            .store
            .test_crdt_authority_counts()
            .await
            .expect("count authority after denied corrupt receipt reads"),
        before
    );
    harness.cleanup().await;
}

async fn assert_denied(
    store: &ModelLaneStore,
    message: handshake_core::swarm_orchestration::model_lane::NewModelLaneMessage,
    case: &str,
) {
    let before = store
        .test_crdt_authority_counts()
        .await
        .expect("count exact authority before denied admission");
    assert!(
        store.record_message(message).await.is_err(),
        "{case} must fail closed"
    );
    let after = store
        .test_crdt_authority_counts()
        .await
        .expect("count exact authority after denied admission");
    assert_eq!(
        before, after,
        "{case} denial must not mutate CRDT authority"
    );
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

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
