//! MT-005 ContextBundle handoff authority over embedded SurrealDB.

mod surreal_test_store_support;

use handshake_core::storage::surreal::{bootstrap_schema, SurrealStorage};
use handshake_core::swarm_orchestration::model_lane::{
    model_lane_context_bundle_id_for_handoff, ModelLaneAuthority, ModelLaneCrdtHandoffMetadata,
    ModelLaneHandoffSelectionState, ModelLaneHandoffSourceKind, ModelLaneLoomHandoffRef,
    ModelLaneMemoryPackHandoffRef, ModelLaneStore, NewModelLaneContextBundleArtifactBinding,
    NewModelLaneContextBundleHandoff,
};
use handshake_core::swarm_orchestration::resource_scope::{
    AccessSpaceRef, ActorPrincipalId, AuthenticatedSessionRef, OwnerAccountId, ResourceScope,
    WorkspaceScopeRef,
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
            .expect("allocate MT-005 embedded scope");
        let storage = isolated
            .activate_storage()
            .await
            .expect("activate production SurrealStorage");
        bootstrap_schema(&storage)
            .await
            .expect("bootstrap canonical schema");
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
        .expect("build production CRDT posture")
    }

    async fn cleanup(mut self) {
        drop(self.store);
        drop(self.storage);
        self.isolated.cleanup().await.expect("clean MT-005 scope");
    }
}

#[tokio::test]
async fn model_lane_context_bundle_persists_selection_state_and_replays() {
    let mut harness = Harness::create("handoff-replay").await;
    let posture = harness.posture("handoff-replay").await;
    let source = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record authoritative source message");
    let downstream = record_downstream_lane(&harness.store, &posture).await;
    harness
        .store
        .record_context_bundle_artifact_binding(artifact_binding(&posture, &source))
        .await
        .expect("record exact artifact binding");

    let mut expected = Vec::new();
    for (index, state) in [
        ModelLaneHandoffSelectionState::Selected,
        ModelLaneHandoffSelectionState::Rejected,
        ModelLaneHandoffSelectionState::Unresolved,
        ModelLaneHandoffSelectionState::Superseded,
    ]
    .into_iter()
    .enumerate()
    {
        let input = handoff(
            &posture,
            &source,
            &downstream,
            &format!("selection-{index}"),
            state,
            None,
        );
        let stored = harness
            .store
            .record_context_bundle_handoff(input.clone())
            .await
            .expect("record typed selection handoff");
        assert_eq!(
            harness
                .store
                .record_context_bundle_handoff(input)
                .await
                .expect("identical handoff retry"),
            stored
        );
        expected.push(stored);
    }
    let context_bundle_id = expected[0].context_bundle_id.clone();
    let replay = harness
        .store
        .replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id)
        .await
        .expect("replay exact context handoffs");
    assert_eq!(replay, vec![expected[0].clone()]);
    let consumed = harness
        .store
        .consume_context_bundle_for_downstream(&posture.run_id, &context_bundle_id, &downstream)
        .await
        .expect("consume without raw prompt replay");
    assert_eq!(consumed.records, replay);
    let owner_receipts = harness
        .store
        .test_scoped_authority_receipts(&posture.run_id, 128)
        .await
        .expect("owner authority watermark");
    for foreign in one_field_mismatches(&harness.scope) {
        let foreign_store = ModelLaneStore::new_scoped(harness.storage.clone(), foreign);
        assert!(foreign_store
            .replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id)
            .await
            .expect("foreign replay is non-leaking")
            .is_empty());
        assert!(
            foreign_store
                .consume_context_bundle_for_downstream(
                    &posture.run_id,
                    &context_bundle_id,
                    &downstream,
                )
                .await
                .is_err()
        );
    }
    assert_eq!(
        harness
            .store
            .test_scoped_authority_receipts(&posture.run_id, 128)
            .await
            .expect("foreign denials are non-mutating"),
        owner_receipts
    );

    drop(harness.store);
    drop(harness.storage);
    harness
        .isolated
        .shutdown_storage_for_reopen()
        .await
        .expect("close before restart");
    harness.isolated.reopen().await.expect("reopen same scope");
    let storage = harness
        .isolated
        .activate_storage()
        .await
        .expect("reactivate same namespace/database");
    let reopened = ModelLaneStore::new_scoped(storage.clone(), harness.scope.clone());
    assert_eq!(
        reopened
            .replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id)
            .await
            .expect("handoff survives restart"),
        replay
    );
    harness.store = reopened;
    harness.storage = storage;
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_context_bundle_missing_artifact_ref_fails_closed() {
    let harness = Harness::create("handoff-denial").await;
    let posture = harness.posture("handoff-denial").await;
    let source = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record source message");
    let downstream = record_downstream_lane(&harness.store, &posture).await;
    let input = handoff(
        &posture,
        &source,
        &downstream,
        "missing-artifact",
        ModelLaneHandoffSelectionState::Selected,
        None,
    );
    let denied = harness
        .store
        .record_context_bundle_handoff(input)
        .await
        .expect_err("missing exact-scope artifact authority must fail");
    assert!(denied.to_string().contains("ArtifactStore/EventLedger"));
    assert!(harness
        .store
        .replay_context_bundle_handoffs(&posture.run_id, "CTX-does-not-exist")
        .await
        .expect("denial leaves no handoff rows")
        .is_empty());

    for foreign in one_field_mismatches(&harness.scope) {
        let foreign_store = ModelLaneStore::new_scoped(harness.storage.clone(), foreign);
        assert!(foreign_store
            .consume_context_bundle_for_downstream(
                &posture.run_id,
                "CTX-does-not-exist",
                &downstream,
            )
            .await
            .is_err());
    }
    assert_eq!(
        harness
            .store
            .replay_run(&posture.run_id)
            .await
            .expect("owner authority survives all denials")
            .messages,
        vec![source]
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable() {
    let harness = Harness::create("handoff-crdt").await;
    let posture = harness.posture("handoff-crdt").await;
    let source = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record CRDT-bearing source message");
    let downstream = record_downstream_lane(&harness.store, &posture).await;
    harness
        .store
        .record_context_bundle_artifact_binding(artifact_binding(&posture, &source))
        .await
        .expect("record source artifact binding");
    let binding = source
        .crdt_authority_binding
        .as_ref()
        .expect("source has exact CRDT binding");
    let crdt = crdt_metadata(&posture, &source);
    let input = handoff(
        &posture,
        &source,
        &downstream,
        "crdt-loom",
        ModelLaneHandoffSelectionState::Selected,
        Some(crdt),
    );
    let context_bundle_id = input.context_bundle_id.clone();
    let stored = harness
        .store
        .record_context_bundle_handoff(input)
        .await
        .expect("record CRDT and Loom handoff");
    let replay = harness
        .store
        .replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id)
        .await
        .expect("replay CRDT and Loom authority");
    assert_eq!(replay, vec![stored]);
    assert_eq!(
        replay[0]
            .crdt_payload
            .as_ref()
            .expect("CRDT metadata")
            .state_vector,
        binding.state_vector
    );
    assert_eq!(replay[0].loom_refs.len(), 1);
    harness.cleanup().await;
}

#[tokio::test]
async fn context_bundle_concurrent_record_and_consume_are_bounded_and_replay_identical() {
    let harness = Harness::create("handoff-concurrent").await;
    let posture = harness.posture("handoff-concurrent").await;
    let source = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record source message");
    let downstream = record_downstream_lane(&harness.store, &posture).await;
    harness
        .store
        .record_context_bundle_artifact_binding(artifact_binding(&posture, &source))
        .await
        .expect("record exact artifact binding");
    let first = handoff(
        &posture,
        &source,
        &downstream,
        "concurrent-first",
        ModelLaneHandoffSelectionState::Selected,
        None,
    );
    let context_bundle_id = first.context_bundle_id.clone();
    harness
        .store
        .record_context_bundle_handoff(first)
        .await
        .expect("seed consumable handoff");

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let writer_store = harness.store.clone();
    let writer_barrier = barrier.clone();
    let second = handoff(
        &posture,
        &source,
        &downstream,
        "concurrent-second",
        ModelLaneHandoffSelectionState::Rejected,
        None,
    );
    let writer = tokio::spawn(async move {
        writer_barrier.wait().await;
        writer_store.record_context_bundle_handoff(second).await
    });
    let reader_store = harness.store.clone();
    let reader_barrier = barrier.clone();
    let reader_run = posture.run_id.clone();
    let reader_bundle = context_bundle_id.clone();
    let reader_lane = downstream.clone();
    let reader = tokio::spawn(async move {
        reader_barrier.wait().await;
        reader_store
            .consume_context_bundle_for_downstream(&reader_run, &reader_bundle, &reader_lane)
            .await
    });
    barrier.wait().await;
    let (recorded, consumed) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(writer, reader)
    })
    .await
    .expect("record/consume lock ordering must not deadlock");
    recorded
        .expect("writer joins")
        .expect("concurrent handoff persists");
    let consumed = consumed
        .expect("reader joins")
        .expect("concurrent consumer returns canonical prefix");
    assert_eq!(
        consumed.records,
        harness
            .store
            .replay_context_bundle_handoffs(&posture.run_id, &context_bundle_id)
            .await
            .expect("canonical replay after concurrency")
    );
    harness.cleanup().await;
}

#[tokio::test]
async fn context_bundle_rejects_derivation_drift_and_fabricated_crdt_authority() {
    let harness = Harness::create("handoff-fabricated").await;
    let posture = harness.posture("handoff-fabricated").await;
    let source = harness
        .store
        .record_message(posture.message.clone())
        .await
        .expect("record source message");
    let downstream = record_downstream_lane(&harness.store, &posture).await;
    harness
        .store
        .record_context_bundle_artifact_binding(artifact_binding(&posture, &source))
        .await
        .expect("record exact artifact binding");

    let mut drifted = handoff(
        &posture,
        &source,
        &downstream,
        "drifted-source",
        ModelLaneHandoffSelectionState::Selected,
        None,
    );
    drifted.source_lane_id.push_str("-foreign");
    drifted.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&drifted).expect("derive drifted id");
    assert!(harness
        .store
        .record_context_bundle_handoff(drifted)
        .await
        .is_err());

    let mut fabricated_crdt = crdt_metadata(&posture, &source);
    fabricated_crdt.update_bytes_ref.push_str("-fabricated");
    let fabricated = handoff(
        &posture,
        &source,
        &downstream,
        "fabricated-crdt",
        ModelLaneHandoffSelectionState::Selected,
        Some(fabricated_crdt),
    );
    assert!(harness
        .store
        .record_context_bundle_handoff(fabricated)
        .await
        .is_err());

    let owner_records = harness
        .store
        .replay_context_bundle_handoffs(&posture.run_id, "CTX-does-not-exist")
        .await
        .expect("denials leave authority readable");
    assert!(owner_records.is_empty());
    harness.cleanup().await;
}

async fn record_downstream_lane(
    store: &ModelLaneStore,
    posture: &SurrealAdmissibleCrdtPosture,
) -> String {
    let replay = store
        .replay_run(&posture.run_id)
        .await
        .expect("read source lane for downstream derivation");
    let mut lane = replay.lanes[0].inner.clone();
    lane.lane_id = format!("{}-downstream", posture.lane_id);
    lane.role = "context-consumer".into();
    lane.lane_span_id = format!("{}-downstream", lane.lane_span_id);
    lane.session_id = format!("{}-downstream", lane.session_id);
    lane.model_session_id = format!("{}-downstream", lane.model_session_id);
    lane.adapter_id = "local-runtime-downstream".into();
    lane.process_ownership_ref = Some(format!("process://mt005/{}/downstream", posture.run_id));
    if let Some(locus) = lane.locus_binding.as_mut() {
        locus.session_id = lane.session_id.clone();
        locus.model_session_id = lane.model_session_id.clone();
        locus.locus_binding_ref = format!("locus://wp1/mt005/{}/downstream", posture.run_id);
    }
    let lane_id = lane.lane_id.clone();
    store
        .record_lane(lane)
        .await
        .expect("record downstream lane through production façade");
    lane_id
}

fn artifact_binding(
    posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> NewModelLaneContextBundleArtifactBinding {
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-{}", source.message_id),
        run_id: posture.run_id.clone(),
        trace_id: posture.trace_id.clone(),
        artifact_ref: source.payload_ref.clone(),
        artifact_sha256: source.payload_sha256.clone(),
        content_hash: source.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!("artifact-manifest://{}", source.message_id),
        artifact_payload_ref: source.payload_ref.clone(),
        payload_json: json!({
            "schema_id": "hsk.model_lane_context_payload@1",
            "message_id": source.message_id,
            "summary": source.summary,
        }),
        event_ledger_stream_id: source.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-005".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: source.owner_session.clone(),
        idempotency_key: format!("artifact-binding-{}", source.message_id),
        created_at_utc: "2026-09-02T00:03:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    }
}

fn crdt_metadata(
    posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
) -> ModelLaneCrdtHandoffMetadata {
    let binding = source
        .crdt_authority_binding
        .as_ref()
        .expect("source has exact CRDT binding");
    ModelLaneCrdtHandoffMetadata {
        schema_id: "hsk.model_lane_crdt_payload@1".into(),
        document_id: binding.document_id.clone(),
        workspace_id: binding.workspace_id.clone(),
        actor_id: binding.actor_id.clone(),
        actor_kind: binding.actor_kind.clone(),
        lane_id: binding.lane_id.clone(),
        crdt_site_id: binding.crdt_site_id.clone(),
        update_seq: binding.update_seq,
        update_bytes_ref: binding.update_bytes_ref.clone(),
        update_sha256: posture.update.update_sha256.clone(),
        state_vector: binding.state_vector.clone(),
        base_snapshot_ref: binding.base_snapshot_ref.clone(),
        materialized_projection_hash: binding.materialized_projection_hash.clone(),
        replay_metadata: json!({
            "format": "yjs_update_v1",
            "yjs_compatible": true,
            "replay_order_key": posture.update.replay_order_key,
            "dependency_update_ids": posture.update.dependency_update_ids,
            "schema_version": posture.update.replay_schema_version,
        }),
        promotion_gate_ref: format!("promotion-gate://model-lane-message/{}", source.message_id),
        promotion_receipt_ref: None,
        validation_runner_ref: format!("eventledger://{}", posture.update.event_ledger_event_id),
        authority_effect: "advisory_only".into(),
    }
}

fn handoff(
    posture: &SurrealAdmissibleCrdtPosture,
    source: &handshake_core::swarm_orchestration::model_lane::ModelLaneMessageRecord,
    downstream_lane_id: &str,
    suffix: &str,
    selection_state: ModelLaneHandoffSelectionState,
    crdt_payload: Option<ModelLaneCrdtHandoffMetadata>,
) -> NewModelLaneContextBundleHandoff {
    let mut input = NewModelLaneContextBundleHandoff {
        handoff_id: format!("handoff-{}-{suffix}", source.message_id),
        context_bundle_id: "pending-derived-id".into(),
        run_id: posture.run_id.clone(),
        trace_id: posture.trace_id.clone(),
        handoff_span_id: format!("span-handoff-{}-{suffix}", source.message_id),
        parent_span_id: Some(source.message_span_id.clone()),
        linked_span_contexts: vec![source.trace_id.clone()],
        downstream_lane_id: downstream_lane_id.into(),
        source_lane_id: source.from_lane_id.clone(),
        source_message_id: source.message_id.clone(),
        artifact_ref: source.payload_ref.clone(),
        artifact_sha256: source.payload_sha256.clone(),
        content_hash: source.payload_sha256.clone(),
        source_kind: ModelLaneHandoffSourceKind::Proposal,
        authority_state: ModelLaneAuthority::PromotionCandidate,
        selection_state,
        reason_code: format!("mt005_{suffix}"),
        decision_ref: Some(format!("decision://mt005/{suffix}")),
        reviewer_ref: Some(format!("reviewer://mt005/{suffix}")),
        replay_hint: "load typed handoff and exact artifact ref".into(),
        crdt_payload,
        loom_refs: vec![ModelLaneLoomHandoffRef {
            workspace_id: source
                .crdt_authority_binding
                .as_ref()
                .map(|binding| binding.workspace_id.clone())
                .expect("ContextBundle source has exact CRDT workspace authority"),
            block_id: format!("loom-block-{}", source.message_id),
            source_block_id: Some(format!("loom-source-{}", source.message_id)),
            target_block_id: Some(format!("loom-target-{}", source.message_id)),
            artifact_ref: Some(source.payload_ref.clone()),
            content_hash: source.payload_sha256.clone(),
            version: "1".into(),
            event_ledger_evidence_ref: format!("eventledger://{}", source.event_ledger_event_id),
            flight_recorder_evidence_ref: format!("flight-recorder://{}", source.message_id),
        }],
        memory_pack_refs: vec![ModelLaneMemoryPackHandoffRef {
            memory_pack_ref: format!("memory-pack://mt005/{}", source.message_id),
            memory_pack_hash: "d".repeat(64),
            scope_tag: "exact_resource_scope".into(),
            review_status: "reviewed".into(),
            cloud_safe: false,
            classification: "local_only_context".into(),
            projection_ref: None,
            evidence_ref: format!("eventledger://{}", source.event_ledger_event_id),
        }],
        event_ledger_stream_id: source.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-005".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: source.owner_session.clone(),
        idempotency_key: format!("handoff-{}-{suffix}", source.message_id),
        replay_order_key: format!("0040-{suffix}"),
        created_at_utc: "2026-09-02T00:04:00Z".into(),
        diagnostic_payload: json!({"flight_recorder": "kernel_event_ledger"}),
    };
    input.context_bundle_id = model_lane_context_bundle_id_for_handoff(&input)
        .expect("derive canonical ContextBundle id");
    input
}

fn exact_scope(label: &str) -> ResourceScope {
    ResourceScope {
        owner_account_id: OwnerAccountId::from_uuid(label_uuid(&(format!("account-{label}")))),
        actor_principal_id: ActorPrincipalId::from_uuid(label_uuid(&(format!("actor-{label}")))),
        authenticated_session: Some(AuthenticatedSessionRef::from_uuid(label_uuid(&(format!("session-{label}"))))),
        access_space: Some(AccessSpaceRef::from_uuid(label_uuid(&(format!("access-{label}"))))),
        workspace: Some(WorkspaceScopeRef::new(format!("workspace-{label}")).expect("workspace")),
    }
}

fn one_field_mismatches(scope: &ResourceScope) -> Vec<ResourceScope> {
    let mut owner = scope.clone();
    owner.owner_account_id = OwnerAccountId::from_uuid(label_uuid(&("account-foreign")));
    let mut actor = scope.clone();
    actor.actor_principal_id = ActorPrincipalId::from_uuid(label_uuid(&("actor-foreign")));
    let mut session = scope.clone();
    session.authenticated_session =
        Some(AuthenticatedSessionRef::from_uuid(label_uuid(&("session-foreign"))));
    let mut access = scope.clone();
    access.access_space = Some(AccessSpaceRef::from_uuid(label_uuid(&("access-foreign"))));
    let mut workspace = scope.clone();
    workspace.workspace =
        Some(WorkspaceScopeRef::new("workspace-foreign").expect("foreign workspace"));
    vec![owner, actor, session, access, workspace]
}

/// Deterministic identifier for a test label so the same label resolves to the
/// same exact scope across reopen phases of one proof.
fn label_uuid(label: &str) -> uuid::Uuid {
    let digest = <sha2::Sha256 as sha2::Digest>::digest(label.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    uuid::Uuid::from_bytes(bytes)
}
