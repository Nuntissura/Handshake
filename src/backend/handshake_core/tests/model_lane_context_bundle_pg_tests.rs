//! WP-1 MT-005: Dexterity ContextBundle handoff runtime proof.
//!
//! These tests use real PostgreSQL plus the kernel EventLedger. They prove that
//! cloud/local model handoffs move through replayable ContextBundle rows, not
//! hidden provider memory or prompt-only state.

mod knowledge_pg_support;

use std::sync::Arc;

use async_trait::async_trait;
use handshake_core::kernel::{DummyEchoModelAdapter, KernelActor};
use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
use handshake_core::swarm_orchestration::model_lane::{
    model_lane_context_bundle_id_for_handoff, LaunchAuthority, ModelLaneAuthority,
    ModelLaneContextBundleHandoffRecord, ModelLaneCrdtHandoffMetadata,
    ModelLaneHandoffSelectionState, ModelLaneHandoffSourceKind, ModelLaneKind,
    ModelLaneLocusBinding, ModelLaneLoomHandoffRef, ModelLaneMemoryPackHandoffRef,
    ModelLaneMessageKind, ModelLaneProviderKind, ModelLaneRecoveryState, ModelLaneRoutingMetadata,
    ModelLaneStatus, ModelLaneStore, ModelLaneTarget, NewModelLane,
    NewModelLaneContextBundleArtifactBinding, NewModelLaneContextBundleHandoff,
    NewModelLaneMessage, NewModelLaneRun, RuntimeBinding,
};
use handshake_core::swarm_orchestration::{
    LiveSession, ModelSessionFactory, RecordingSwarmSink, RunBudget, SpawnRequest, SwarmConfig,
    SwarmCoordinator, SwarmError, SwarmResult,
};
use serde_json::json;
use sha2::{Digest, Sha256};

#[tokio::test]
async fn model_lane_context_bundle_persists_selection_state_and_replays() {
    let (pool, store) = model_lane_store().await;
    seed_run_with_messages(&store).await;

    let mut selected = sample_handoff(
        "handoff-selected",
        "idem-handoff-selected",
        "msg-proposal-001",
        "lane-local",
        ModelLaneHandoffSourceKind::Proposal,
        artifact_payload_hash("msg-proposal-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    selected.crdt_payload = Some(sample_crdt_payload());
    selected.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&selected).expect("derive ContextBundle id");
    let mut handoffs = vec![
        selected,
        sample_handoff(
            "handoff-rejected",
            "idem-handoff-rejected",
            "msg-critique-001",
            "lane-cloud",
            ModelLaneHandoffSourceKind::Critique,
            artifact_payload_hash("msg-critique-001"),
            ModelLaneHandoffSelectionState::Rejected,
        ),
        sample_handoff(
            "handoff-unresolved",
            "idem-handoff-unresolved",
            "msg-status-001",
            "lane-local",
            ModelLaneHandoffSourceKind::Status,
            artifact_payload_hash("msg-status-001"),
            ModelLaneHandoffSelectionState::Unresolved,
        ),
        sample_handoff(
            "handoff-superseded",
            "idem-handoff-superseded",
            "msg-recovery-001",
            "lane-cloud",
            ModelLaneHandoffSourceKind::Recovery,
            artifact_payload_hash("msg-recovery-001"),
            ModelLaneHandoffSelectionState::Superseded,
        ),
    ];
    let context_bundle_id = handoffs[0].context_bundle_id.clone();
    assert!(
        handoffs
            .iter()
            .all(|handoff| handoff.context_bundle_id == context_bundle_id),
        "one ContextBundle must replay all handoff selection states together"
    );

    let mut stored = Vec::new();
    for handoff in handoffs.iter().cloned() {
        stored.push(
            store
                .record_context_bundle_handoff(handoff)
                .await
                .expect("record ContextBundle handoff"),
        );
    }

    assert!(stored.iter().all(|row| row.event_ledger_seq > 0));
    assert!(stored
        .iter()
        .all(|row| row.event_ledger_event_id.starts_with("KE-")));

    let ledger_row: (String, String, String) = sqlx::query_as(
        "SELECT event_type, aggregate_type, aggregate_id \
         FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&stored[0].event_ledger_event_id)
    .fetch_one(&pool)
    .await
    .expect("ContextBundle handoff EventLedger row");
    assert_eq!(ledger_row.0, "CONTEXT_BUNDLE_RECORDED");
    assert_eq!(ledger_row.1, "model_lane_context_bundle_handoff");
    assert_eq!(ledger_row.2, "handoff-selected");
    let ledger_payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&stored[0].event_ledger_event_id)
            .fetch_one(&pool)
            .await
            .expect("ContextBundle handoff EventLedger payload");
    assert_eq!(
        ledger_payload["record"]["event_ledger_event_id"],
        json!(stored[0].event_ledger_event_id),
        "EventLedger payload must contain the final persisted handoff event id"
    );
    assert_eq!(
        ledger_payload["record"]["event_ledger_seq"],
        json!(stored[0].event_ledger_seq),
        "EventLedger payload must contain the final persisted handoff sequence"
    );

    let artifact_ledger: (String, String, String, String, i64, serde_json::Value) = sqlx::query_as(
        "SELECT event_type, aggregate_type, aggregate_id, event_id, event_sequence, payload \
         FROM kernel_event_ledger \
         WHERE aggregate_type = 'model_lane_context_bundle_artifact' \
           AND aggregate_id = 'artifact-binding-msg-proposal-001'",
    )
    .fetch_one(&pool)
    .await
    .expect("ContextBundle artifact binding EventLedger row");
    assert_eq!(artifact_ledger.0, "ARTIFACT_STORED");
    assert_eq!(artifact_ledger.1, "model_lane_context_bundle_artifact");
    assert_eq!(artifact_ledger.2, "artifact-binding-msg-proposal-001");
    assert_eq!(
        artifact_ledger.5["record"]["event_ledger_event_id"],
        json!(artifact_ledger.3),
        "EventLedger payload must contain the final persisted artifact event id"
    );
    assert_eq!(
        artifact_ledger.5["record"]["event_ledger_seq"],
        json!(artifact_ledger.4),
        "EventLedger payload must contain the final persisted artifact sequence"
    );

    let replay = store
        .replay_context_bundle_handoffs("run-mt005", &context_bundle_id)
        .await
        .expect("replay ContextBundle handoffs");
    assert_eq!(replay.len(), 4);
    assert_replay_order_matches_eventledger(&replay);
    assert_eq!(
        replay
            .iter()
            .map(|row| row.selection_state)
            .collect::<Vec<_>>(),
        vec![
            ModelLaneHandoffSelectionState::Selected,
            ModelLaneHandoffSelectionState::Rejected,
            ModelLaneHandoffSelectionState::Unresolved,
            ModelLaneHandoffSelectionState::Superseded,
        ]
    );
    assert_eq!(
        replay[0].artifact_ref,
        artifact_store_payload_ref("msg-proposal-001")
    );
    assert_eq!(
        replay[0].artifact_sha256,
        artifact_payload_hash("msg-proposal-001")
    );

    let downstream = store
        .consume_context_bundle_for_downstream("run-mt005", &context_bundle_id, "lane-cloud")
        .await
        .expect("downstream lane consumes replayed ContextBundle");
    assert_eq!(downstream.records.len(), 4);
    assert_eq!(downstream.downstream_lane_id, "lane-cloud");
    assert_eq!(
        downstream.allowed_context["selected"]
            .as_array()
            .expect("selected array")
            .len(),
        1
    );
    let kernel_context_bundle = downstream
        .to_kernel_context_bundle()
        .expect("downstream handoff converts to kernel ContextBundle");
    assert_eq!(
        kernel_context_bundle.context_bundle_id,
        format!("CTX-{}", &downstream.context_hash[..16]),
        "kernel ContextBundle id must follow ContextBundle V1 CTX-<hash> identity"
    );
    assert_eq!(kernel_context_bundle.session_run_id, "lane-cloud");
    assert_eq!(kernel_context_bundle.context_hash, downstream.context_hash);

    let coordinator = coordinator_with_store(store.clone());
    let adapter = DummyEchoModelAdapter::new("dummy-echo-mt005");
    let adapter_output = coordinator
        .invoke_downstream_context_bundle(
            "run-mt005",
            &context_bundle_id,
            "lane-cloud",
            &adapter,
            KernelActor::ModelAdapter("lane-cloud".into()),
        )
        .await
        .expect("coordinator invokes adapter with downstream ContextBundle");
    assert_eq!(
        adapter_output.context_bundle_id, kernel_context_bundle.context_bundle_id,
        "coordinator must pass the kernel ContextBundle id to the adapter boundary"
    );
    assert!(
        adapter_output
            .response_text
            .contains(&kernel_context_bundle.context_hash),
        "adapter output must be derived from the replayed downstream ContextBundle hash"
    );

    let wrong_downstream = coordinator
        .context_bundle_for_downstream_lane("run-mt005", &context_bundle_id, "lane-missing")
        .await
        .expect_err("wrong downstream lane cannot consume another lane's ContextBundle");
    assert!(
        wrong_downstream.to_string().contains("downstream_lane_id"),
        "wrong downstream lane failure must be explicit: {wrong_downstream}"
    );

    let duplicate = store
        .record_context_bundle_handoff(handoffs.remove(0))
        .await
        .expect("same idempotency and same content returns existing handoff");
    assert_eq!(
        duplicate.event_ledger_event_id,
        stored[0].event_ledger_event_id
    );
    assert_eq!(duplicate.context_bundle_hash, stored[0].context_bundle_hash);

    let registry_rows = store
        .schema_registry_rows()
        .await
        .expect("schema registry rows");
    assert!(
        registry_rows
            .iter()
            .any(|row| row.schema_id == "hsk.model_lane_context_bundle_handoff@1"),
        "ContextBundle handoff schema must be registered for state recovery"
    );
    assert!(
        registry_rows
            .iter()
            .any(|row| row.schema_id == "hsk.model_lane_context_bundle_artifact@1"),
        "ContextBundle artifact binding schema must be registered for state recovery"
    );
}

#[tokio::test]
async fn model_lane_context_bundle_missing_artifact_ref_fails_closed() {
    let (_pool, store) = model_lane_store().await;
    seed_run_with_messages(&store).await;

    let proposal_message = advisory_message(
        "msg-proposal-001",
        "idem-artifact-boundary-message-copy",
        "lane-local",
        ModelLaneMessageKind::Proposal,
        artifact_payload_hash("msg-proposal-001"),
        "artifact binding boundary copy",
    );
    let mut wrong_payload_hash_binding = sample_artifact_binding_for_message(&proposal_message);
    wrong_payload_hash_binding.artifact_binding_id = "artifact-binding-wrong-payload-hash".into();
    wrong_payload_hash_binding.idempotency_key = "idem-artifact-binding-wrong-payload-hash".into();
    wrong_payload_hash_binding.content_hash = sample_sha256('e');
    wrong_payload_hash_binding.artifact_sha256 = sample_sha256('e');
    let wrong_payload_hash_err = store
        .record_context_bundle_artifact_binding(wrong_payload_hash_binding)
        .await
        .expect_err("artifact binding must hash payload_json before handoff");
    assert!(
        wrong_payload_hash_err
            .to_string()
            .contains("payload_json sha256"),
        "artifact binding payload hash failure must be explicit: {wrong_payload_hash_err}"
    );

    let mut wrong_payload_ref_binding = sample_artifact_binding_for_message(&proposal_message);
    wrong_payload_ref_binding.artifact_binding_id = "artifact-binding-wrong-payload-ref".into();
    wrong_payload_ref_binding.idempotency_key = "idem-artifact-binding-wrong-payload-ref".into();
    wrong_payload_ref_binding.artifact_payload_ref =
        "artifact-store://model-lane/mt005/msg-proposal-001/not-payload".into();
    let wrong_payload_ref_err = store
        .record_context_bundle_artifact_binding(wrong_payload_ref_binding)
        .await
        .expect_err("artifact binding payload ref must match artifact ref");
    assert!(
        wrong_payload_ref_err
            .to_string()
            .contains("artifact_payload_ref"),
        "artifact binding payload ref failure must be explicit: {wrong_payload_ref_err}"
    );

    let mut wrong_artifact = sample_handoff(
        "handoff-wrong-artifact",
        "idem-handoff-wrong-artifact",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    wrong_artifact.artifact_ref = "artifact://mt005/not-the-source".into();
    let wrong_artifact_err = store
        .record_context_bundle_handoff(wrong_artifact)
        .await
        .expect_err("artifact_ref mismatch must fail closed");
    assert!(
        wrong_artifact_err.to_string().contains("artifact_ref"),
        "artifact_ref mismatch must be explicit: {wrong_artifact_err}"
    );

    let mut wrong_hash = sample_handoff(
        "handoff-wrong-hash",
        "idem-handoff-wrong-hash",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    wrong_hash.artifact_sha256 = sample_sha256('f');
    wrong_hash.content_hash = sample_sha256('f');
    let wrong_hash_err = store
        .record_context_bundle_handoff(wrong_hash)
        .await
        .expect_err("artifact_sha256 mismatch must fail closed");
    assert!(
        wrong_hash_err.to_string().contains("artifact_sha256"),
        "artifact_sha256 mismatch must be explicit: {wrong_hash_err}"
    );

    let missing_source = sample_handoff(
        "handoff-missing-source",
        "idem-handoff-missing-source",
        "msg-missing-404",
        "lane-local",
        ModelLaneHandoffSourceKind::Proposal,
        artifact_payload_hash("msg-proposal-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    let missing_source_err = store
        .record_context_bundle_handoff(missing_source)
        .await
        .expect_err("missing source message must fail closed");
    assert!(
        missing_source_err.to_string().contains("not replayable"),
        "missing source must say not replayable: {missing_source_err}"
    );

    store
        .record_message(advisory_message(
            "msg-unbound-001",
            "idem-message-unbound-001",
            "lane-cloud",
            ModelLaneMessageKind::Critique,
            artifact_payload_hash("msg-unbound-001"),
            "cloud lane produces a message without ArtifactStore authority",
        ))
        .await
        .expect("record unbound source message");
    let unbound_artifact = sample_handoff(
        "handoff-unbound-artifact",
        "idem-handoff-unbound-artifact",
        "msg-unbound-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-unbound-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    let unbound_artifact_err = store
        .record_context_bundle_handoff(unbound_artifact)
        .await
        .expect_err("source message with no ArtifactStore authority must fail closed");
    assert!(
        unbound_artifact_err
            .to_string()
            .contains("ArtifactStore/EventLedger authority"),
        "unbound artifact failure must be explicit: {unbound_artifact_err}"
    );

    let mut unsafe_memory = sample_handoff(
        "handoff-unsafe-memory",
        "idem-handoff-unsafe-memory",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    unsafe_memory.memory_pack_refs[0].cloud_safe = false;
    let unsafe_memory_err = store
        .record_context_bundle_handoff(unsafe_memory)
        .await
        .expect_err("cloud downstream must reject non-cloud-safe memory packs");
    assert!(
        unsafe_memory_err.to_string().contains("cloud_safe"),
        "cloud memory-pack failure must be explicit: {unsafe_memory_err}"
    );

    let mut missing_downstream = sample_handoff(
        "handoff-missing-downstream",
        "idem-handoff-missing-downstream",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    missing_downstream.downstream_lane_id = String::new();
    missing_downstream.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&missing_downstream)
            .expect("derive ContextBundle id");
    let missing_downstream_err = store
        .record_context_bundle_handoff(missing_downstream)
        .await
        .expect_err("model-to-model handoff requires downstream lane id");
    assert!(
        missing_downstream_err
            .to_string()
            .contains("downstream_lane_id"),
        "missing downstream lane must fail closed: {missing_downstream_err}"
    );

    let mut hidden_memory = sample_handoff(
        "handoff-hidden-memory",
        "idem-handoff-hidden-memory",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    hidden_memory.memory_pack_refs[0].memory_pack_ref =
        "provider-session://openai/thread-hidden".into();
    let hidden_memory_err = store
        .record_context_bundle_handoff(hidden_memory)
        .await
        .expect_err("hidden provider/session memory cannot be authority");
    assert!(
        hidden_memory_err
            .to_string()
            .contains("hidden provider/session memory"),
        "hidden memory failure must be explicit: {hidden_memory_err}"
    );

    let mut unreviewed_memory = sample_handoff(
        "handoff-unreviewed-memory",
        "idem-handoff-unreviewed-memory",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    unreviewed_memory.memory_pack_refs[0].review_status = "draft".into();
    let unreviewed_memory_err = store
        .record_context_bundle_handoff(unreviewed_memory)
        .await
        .expect_err("unreviewed MemoryPack refs cannot be handoff authority");
    assert!(
        unreviewed_memory_err.to_string().contains("review_status"),
        "unreviewed memory failure must be explicit: {unreviewed_memory_err}"
    );

    let mut local_only_cloud_memory = sample_handoff(
        "handoff-local-only-cloud-memory",
        "idem-handoff-local-only-cloud-memory",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    local_only_cloud_memory.memory_pack_refs[0].classification = "local_only_context".into();
    let local_only_cloud_memory_err = store
        .record_context_bundle_handoff(local_only_cloud_memory)
        .await
        .expect_err("cloud lane must reject local_only_context memory");
    assert!(
        local_only_cloud_memory_err
            .to_string()
            .contains("local_only_context"),
        "local-only cloud memory failure must be explicit: {local_only_cloud_memory_err}"
    );

    let mut hidden_projection = sample_handoff(
        "handoff-hidden-projection",
        "idem-handoff-hidden-projection",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    hidden_projection.memory_pack_refs[0].projection_ref =
        Some("provider-session://openai/projection-hidden".into());
    let hidden_projection_err = store
        .record_context_bundle_handoff(hidden_projection)
        .await
        .expect_err("hidden projection refs cannot become MemoryPack authority");
    assert!(
        hidden_projection_err
            .to_string()
            .contains("hidden provider/session memory"),
        "hidden projection failure must be explicit: {hidden_projection_err}"
    );

    let mut normalized_hidden_memory = sample_handoff(
        "handoff-normalized-hidden-memory",
        "idem-handoff-normalized-hidden-memory",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    normalized_hidden_memory.memory_pack_refs[0].memory_pack_ref =
        "  PROVIDER-SESSION://openai/thread-hidden".into();
    let normalized_hidden_memory_err = store
        .record_context_bundle_handoff(normalized_hidden_memory)
        .await
        .expect_err("hidden memory URI checks must trim and normalize case");
    assert!(
        normalized_hidden_memory_err
            .to_string()
            .contains("hidden provider/session memory"),
        "normalized hidden memory failure must be explicit: {normalized_hidden_memory_err}"
    );

    let mut too_many_memory_refs = sample_handoff(
        "handoff-too-many-memory-refs",
        "idem-handoff-too-many-memory-refs",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    too_many_memory_refs.memory_pack_refs = (0..17)
        .map(|index| {
            let mut memory = sample_memory_pack(true);
            memory.memory_pack_ref = format!("memory-pack://mt005/cloud-review-{index}");
            memory.projection_ref = Some(format!("memory-projection://mt005/cloud-review-{index}"));
            memory.evidence_ref = format!("eventledger://mt005/memory-pack/cloud-review-{index}");
            memory
        })
        .collect();
    let too_many_memory_refs_err = store
        .record_context_bundle_handoff(too_many_memory_refs)
        .await
        .expect_err("MemoryPack refs must be bounded");
    assert!(
        too_many_memory_refs_err
            .to_string()
            .contains("bounded FEMS limit"),
        "too many MemoryPack refs failure must be explicit: {too_many_memory_refs_err}"
    );

    let mut too_many_loom_refs = sample_handoff(
        "handoff-too-many-loom-refs",
        "idem-handoff-too-many-loom-refs",
        "msg-critique-001",
        "lane-cloud",
        ModelLaneHandoffSourceKind::Critique,
        artifact_payload_hash("msg-critique-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    too_many_loom_refs.loom_refs = (0..65)
        .map(|index| {
            let mut loom = sample_loom_ref();
            loom.block_id = format!("loom-block-{index:03}");
            loom.event_ledger_evidence_ref = format!("eventledger://mt005/loom/block-{index:03}");
            loom.flight_recorder_evidence_ref =
                format!("flight-recorder://mt005/loom/block-{index:03}");
            loom
        })
        .collect();
    let too_many_loom_refs_err = store
        .record_context_bundle_handoff(too_many_loom_refs)
        .await
        .expect_err("Loom refs must be bounded");
    assert!(
        too_many_loom_refs_err
            .to_string()
            .contains("loom_refs exceeds bounded limit"),
        "too many Loom refs failure must be explicit: {too_many_loom_refs_err}"
    );
}

#[tokio::test]
async fn model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable() {
    let (_pool, store) = model_lane_store().await;
    seed_run_with_messages(&store).await;

    let mut handoff = sample_handoff(
        "handoff-crdt-loom",
        "idem-handoff-crdt-loom",
        "msg-proposal-001",
        "lane-local",
        ModelLaneHandoffSourceKind::Proposal,
        artifact_payload_hash("msg-proposal-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    handoff.crdt_payload = Some(sample_crdt_payload());
    handoff.loom_refs = vec![sample_loom_ref()];
    handoff.memory_pack_refs = vec![sample_memory_pack(true)];
    handoff.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&handoff).expect("derive ContextBundle id");

    let stored = store
        .record_context_bundle_handoff(handoff.clone())
        .await
        .expect("record CRDT/Loom ContextBundle handoff");
    let replay = store
        .replay_context_bundle_handoffs("run-mt005", &stored.context_bundle_id)
        .await
        .expect("replay CRDT/Loom handoff");
    assert_eq!(replay.len(), 1);
    let crdt = replay[0].crdt_payload.as_ref().expect("replayed CRDT");
    assert_eq!(crdt.schema_id, "hsk.model_lane_crdt_payload@1");
    assert_eq!(
        crdt.update_bytes_ref,
        "crdt-update://mt005/msg-proposal-001"
    );
    assert_eq!(crdt.state_vector, "sv:mt005:1");
    assert_eq!(crdt.base_snapshot_ref, "crdt-snapshot://mt005/base-v1");
    assert_eq!(crdt.authority_effect, "advisory_only");
    assert_eq!(
        crdt.replay_metadata["yjs_compatible"],
        json!(true),
        "CRDT payload must carry Yjs-compatible replay metadata"
    );
    assert_eq!(replay[0].loom_refs[0].workspace_id, "workspace-mt005");
    assert_eq!(
        replay[0].loom_refs[0].flight_recorder_evidence_ref,
        "flight-recorder://mt005/loom/block-001"
    );
    assert_eq!(replay[0].memory_pack_refs[0].cloud_safe, true);
    assert_eq!(
        replay[0].memory_pack_refs[0].memory_pack_hash,
        sample_sha256('a')
    );

    let mut missing_crdt = sample_handoff(
        "handoff-crdt-missing",
        "idem-handoff-crdt-missing",
        "msg-proposal-001",
        "lane-local",
        ModelLaneHandoffSourceKind::Proposal,
        artifact_payload_hash("msg-proposal-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    missing_crdt.crdt_payload = None;
    let missing_crdt_err = store
        .record_context_bundle_handoff(missing_crdt)
        .await
        .expect_err("CRDT source message requires CRDT handoff metadata");
    assert!(
        missing_crdt_err.to_string().contains("crdt_payload"),
        "missing CRDT metadata must fail closed: {missing_crdt_err}"
    );

    let mut wrong_update_ref = handoff;
    wrong_update_ref.handoff_id = "handoff-crdt-wrong-update-ref".into();
    wrong_update_ref.idempotency_key = "idem-handoff-crdt-wrong-update-ref".into();
    wrong_update_ref.replay_order_key = "00000052/handoff-crdt-wrong-update-ref".into();
    wrong_update_ref
        .crdt_payload
        .as_mut()
        .expect("CRDT payload")
        .update_bytes_ref = "crdt-update://mt005/not-the-source".into();
    let wrong_update_err = store
        .record_context_bundle_handoff(wrong_update_ref)
        .await
        .expect_err("CRDT update ref must match source message replay ref");
    assert!(
        wrong_update_err.to_string().contains("update_bytes_ref"),
        "CRDT update ref mismatch must be explicit: {wrong_update_err}"
    );

    let mut non_yjs = sample_handoff(
        "handoff-crdt-non-yjs",
        "idem-handoff-crdt-non-yjs",
        "msg-proposal-001",
        "lane-local",
        ModelLaneHandoffSourceKind::Proposal,
        artifact_payload_hash("msg-proposal-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    let mut non_yjs_payload = sample_crdt_payload();
    non_yjs_payload.replay_metadata = json!({
        "format": "opaque_patch",
        "yjs_compatible": false
    });
    non_yjs.crdt_payload = Some(non_yjs_payload);
    let non_yjs_err = store
        .record_context_bundle_handoff(non_yjs)
        .await
        .expect_err("non-Yjs CRDT replay metadata must fail closed");
    assert!(
        non_yjs_err.to_string().contains("Yjs-compatible"),
        "non-Yjs failure must be explicit: {non_yjs_err}"
    );
}

fn assert_replay_order_matches_eventledger(replay: &[ModelLaneContextBundleHandoffRecord]) {
    for pair in replay.windows(2) {
        assert!(
            pair[0].event_ledger_seq < pair[1].event_ledger_seq,
            "ContextBundle replay must be ordered by EventLedger sequence"
        );
    }
}

async fn model_lane_store() -> (sqlx::PgPool, ModelLaneStore) {
    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-005 proof");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated model-lane ContextBundle schema");
    let store = ModelLaneStore::new(pool.clone());
    (pool, store)
}

struct ContextBundleOnlyFactory;

#[async_trait]
impl ModelSessionFactory for ContextBundleOnlyFactory {
    async fn create(&self, _request: &SpawnRequest) -> SwarmResult<LiveSession> {
        Err(SwarmError::FactoryFailed(
            "MT-005 ContextBundle proof must not create a model session".into(),
        ))
    }
}

fn coordinator_with_store(store: ModelLaneStore) -> SwarmCoordinator {
    let (ledger, _drain) = LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 4096,
            ..LedgerBatcherConfig::default()
        },
        Arc::new(NoopOverflowSink),
    )
    .expect("manual process ledger");
    SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(1)),
        Arc::new(ContextBundleOnlyFactory),
        Arc::new(RecordingSwarmSink::new()),
        ledger,
        store,
    )
}

async fn seed_run_with_messages(store: &ModelLaneStore) {
    store
        .record_run(sample_run())
        .await
        .expect("record MT-005 run");
    for lane in [
        sample_lane(
            "lane-local",
            ModelLaneKind::LocalModel,
            RuntimeBinding::Local,
            LaunchAuthority::ModelRuntime,
            ModelLaneProviderKind::LocalRuntime,
        ),
        sample_lane(
            "lane-cloud",
            ModelLaneKind::CloudModel,
            RuntimeBinding::Cloud,
            LaunchAuthority::CloudLane,
            ModelLaneProviderKind::OpenAi,
        ),
    ] {
        store.record_lane(lane).await.expect("record MT-005 lane");
    }
    let messages = [
        advisory_message(
            "msg-proposal-001",
            "idem-message-proposal-001",
            "lane-local",
            ModelLaneMessageKind::Proposal,
            artifact_payload_hash("msg-proposal-001"),
            "local lane proposes CRDT edit for Loom block",
        ),
        advisory_message(
            "msg-critique-001",
            "idem-message-critique-001",
            "lane-cloud",
            ModelLaneMessageKind::Critique,
            artifact_payload_hash("msg-critique-001"),
            "cloud lane critiques the local edit",
        ),
        advisory_message(
            "msg-status-001",
            "idem-message-status-001",
            "lane-local",
            ModelLaneMessageKind::Status,
            artifact_payload_hash("msg-status-001"),
            "local lane reports pending validation status",
        ),
        advisory_message(
            "msg-recovery-001",
            "idem-message-recovery-001",
            "lane-cloud",
            ModelLaneMessageKind::Recovery,
            artifact_payload_hash("msg-recovery-001"),
            "cloud lane marks prior handoff superseded during recovery",
        ),
    ];
    for message in messages {
        store
            .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&message))
            .await
            .expect("record ArtifactStore/EventLedger binding for source message");
        store
            .record_message(message)
            .await
            .expect("record advisory MT-005 message");
    }
}

fn sample_run() -> NewModelLaneRun {
    NewModelLaneRun {
        run_id: "run-mt005".into(),
        trace_id: "trace-mt005".into(),
        run_span_id: "span-run-mt005".into(),
        coordinator_session_id: "coordinator-session-mt005".into(),
        routing_policy: "cloud_plan_local_execute".into(),
        context_bundle_id: "context-bundle://mt005/bootstrap".into(),
        lane_ids: vec!["lane-local".into(), "lane-cloud".into()],
        event_ledger_stream_id: "mlane-stream-run-mt005".into(),
        artifact_namespace: "artifact://model-lane/mt005".into(),
        projection_plan_ref: Some("projection-plan://mt005/cloud-review".into()),
        consent_receipt_ref: Some("consent://mt005/cloud-review".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-005".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT005".into(),
        idempotency_key: "idem-run-mt005".into(),
        replay_order_key: "00000000/run".into(),
        replay_after_event_ledger_seq: None,
        recovery_state: ModelLaneRecoveryState::Restartable,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-context-bundle-handoff#run".into()),
        locus_binding: Some(sample_locus("session-run", "model-session-run")),
        memory_pack_ref: "memory-pack://mt005".into(),
        memory_pack_hash: sample_sha256('a'),
        determinism_mode: "deterministic_replay".into(),
        budget_summary_ref: "budget://mt005".into(),
        selected_model_id: Some("model://mt005/local".into()),
        candidate_model_ids: vec!["model://mt005/local".into(), "model://mt005/cloud".into()],
        procedural_review_status: "runtime_context_bundle_preflight".into(),
        truncation_warning_ref: None,
        rejection_reason_refs: vec!["rejection://mt005/no-hidden-provider-memory".into()],
    }
}

fn sample_lane(
    lane_id: &str,
    kind: ModelLaneKind,
    runtime_binding: RuntimeBinding,
    launch_authority: LaunchAuthority,
    provider_kind: ModelLaneProviderKind,
) -> NewModelLane {
    let process_backed = !matches!(
        runtime_binding,
        RuntimeBinding::Human | RuntimeBinding::Subagent | RuntimeBinding::Validator
    );
    NewModelLane {
        lane_id: lane_id.into(),
        run_id: "run-mt005".into(),
        trace_id: "trace-mt005".into(),
        lane_span_id: format!("span-{lane_id}"),
        event_ledger_stream_id: "mlane-stream-run-mt005".into(),
        kind,
        role: format!("role-{lane_id}"),
        backend: format!("backend-{lane_id}"),
        model_id: Some(format!("model://mt005/{lane_id}")),
        session_id: format!("session-{lane_id}"),
        model_session_id: format!("model-session-{lane_id}"),
        adapter_id: format!("adapter-{lane_id}"),
        runtime_binding: runtime_binding.clone(),
        launch_authority,
        provider_kind,
        capability_token_ids: vec!["capability://mt005/read-context".into()],
        effective_capability_snapshot_ref: Some(format!("capability-snapshot://{lane_id}")),
        capability_negotiation_ref: Some(format!("capability-negotiation://{lane_id}")),
        provider_feature_profile_ref: Some(format!("provider-feature-profile://{lane_id}")),
        requested_execution_policy_ref: Some(format!("execution-policy://requested/{lane_id}")),
        effective_execution_policy_ref: Some(format!("execution-policy://effective/{lane_id}")),
        projection_plan_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some(format!("projection-plan://{lane_id}")),
        consent_receipt_ref: (runtime_binding == RuntimeBinding::Cloud)
            .then_some(format!("consent://{lane_id}")),
        tool_gate_decision_refs: vec!["toolgate://mt005/read-context".into()],
        status: ModelLaneStatus::Ready,
        recovery_state: ModelLaneRecoveryState::Restartable,
        heartbeat_at_utc: Some("2026-06-29T09:00:00Z".into()),
        lease_expires_at_utc: Some("2026-06-29T09:05:00Z".into()),
        reclaim_after_utc: Some("2026-06-29T09:06:00Z".into()),
        restart_generation: 0,
        cancellation_ref: Some(format!("cancel-token://{lane_id}")),
        reclaim_policy_ref: Some("reclaim-policy://mt005".into()),
        terminal_status_mapping_ref: Some("terminal-status://mt005".into()),
        process_ownership_ref: process_backed.then_some(format!("process-ledger://{lane_id}")),
        no_os_process_reason_ref: (!process_backed).then_some(format!("no-os://{lane_id}")),
        backpressure_ref: None,
        loop_counter_ref: Some("loop-counter://mt005".into()),
        last_runtime_status_ref: Some("runtime-status://ready".into()),
        last_recovery_event_ref: None,
        failstate_code: None,
        startup_failure_ref: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-context-bundle-handoff#lane".into()),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-005".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT005".into(),
        locus_binding: Some(sample_locus(
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
    }
}

fn advisory_message(
    message_id: &str,
    idempotency_key: &str,
    lane_id: &str,
    kind: ModelLaneMessageKind,
    payload_sha256: String,
    summary: &str,
) -> NewModelLaneMessage {
    let proposal_fields = kind == ModelLaneMessageKind::Proposal;
    NewModelLaneMessage {
        message_id: message_id.into(),
        run_id: "run-mt005".into(),
        trace_id: "trace-mt005".into(),
        message_span_id: format!("span-{message_id}"),
        parent_span_id: Some(format!("span-{lane_id}")),
        linked_span_contexts: vec!["span-coordinator-mt005".into()],
        from_lane_id: lane_id.into(),
        to_lane: ModelLaneTarget::Coordinator,
        routing: Some(sample_routing(
            &format!("corr-{message_id}"),
            "coordinator",
            "coordinator-session-mt005",
        )),
        kind,
        payload_ref: artifact_store_payload_ref(message_id),
        payload_sha256,
        event_ledger_stream_id: "mlane-stream-run-mt005".into(),
        summary: summary.into(),
        authority: ModelLaneAuthority::Advisory,
        promotion_decision_id: None,
        promotion_gate_ref: None,
        promotion_receipt_ref: None,
        validator_verdict_ref: None,
        operator_decision_ref: None,
        promoted_artifact_ref: None,
        promoted_artifact_sha256: None,
        promoted_artifact_version: None,
        tool_gate_decision_refs: vec!["toolgate://mt005/read-context".into()],
        coordinator_session_id: "coordinator-session-mt005".into(),
        work_packet_id: Some("WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into()),
        micro_task_id: Some("MT-005".into()),
        task_board_id: Some("task-board://wp-1".into()),
        owner_session: "KERNEL_BUILDER-MT005".into(),
        locus_binding: Some(sample_locus(
            &format!("session-{lane_id}"),
            &format!("model-session-{lane_id}"),
        )),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000010/{message_id}"),
        replay_after_event_ledger_seq: Some(1),
        proposal_ref: proposal_fields.then_some(format!("proposal://mt005/{message_id}")),
        crdt_update_ref: proposal_fields.then_some(format!("crdt-update://mt005/{message_id}")),
        crdt_base_snapshot_ref: proposal_fields.then_some("crdt-snapshot://mt005/base-v1".into()),
        crdt_state_vector: proposal_fields.then_some("sv:mt005:1".into()),
        crdt_proposal_ref: proposal_fields.then_some(format!("crdt-proposal://mt005/{message_id}")),
        crdt_stale_base_ref: None,
        failstate_code: None,
        reason_ref: None,
        recovery_hint_ref: Some("usermanual://model-lane-context-bundle-handoff#advisory".into()),
        created_at_utc: "2026-06-29T09:01:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger-backed source message",
            "locus": "locus://wp1/mt005/coordinator-session-mt005",
            "palmistry": "external watcher link expected when feature is available"
        }),
    }
}

fn sample_artifact_binding_for_message(
    message: &NewModelLaneMessage,
) -> NewModelLaneContextBundleArtifactBinding {
    NewModelLaneContextBundleArtifactBinding {
        artifact_binding_id: format!("artifact-binding-{}", message.message_id),
        run_id: message.run_id.clone(),
        trace_id: message.trace_id.clone(),
        artifact_ref: message.payload_ref.clone(),
        artifact_sha256: message.payload_sha256.clone(),
        content_hash: message.payload_sha256.clone(),
        artifact_kind: "model_lane_message_payload".into(),
        artifact_manifest_ref: format!(
            "artifact-store://model-lane/mt005/{}/artifact.json",
            message.message_id
        ),
        artifact_payload_ref: message.payload_ref.clone(),
        payload_json: artifact_payload_json(&message.message_id),
        event_ledger_stream_id: message.event_ledger_stream_id.clone(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-005".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-MT005".into(),
        idempotency_key: format!("idem-artifact-binding-{}", message.message_id),
        created_at_utc: "2026-06-29T09:00:30Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ArtifactStore/EventLedger binding for model-lane payload",
            "locus": "locus://wp1/mt005/coordinator-session-mt005",
            "palmistry": "external watcher link expected when feature is available"
        }),
    }
}

fn artifact_store_payload_ref(message_id: &str) -> String {
    format!("artifact-store://model-lane/mt005/{message_id}/payload")
}

fn artifact_payload_json(message_id: &str) -> serde_json::Value {
    json!({
        "schema_id": "hsk.model_lane_message_payload@1",
        "message_id": message_id,
        "body": format!("deterministic payload for {message_id}")
    })
}

fn artifact_payload_hash(message_id: &str) -> String {
    sha256_hex(&canonical_json_bytes(&artifact_payload_json(message_id)))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn canonical_json_bytes(value: &serde_json::Value) -> Vec<u8> {
    let mut output = String::new();
    write_canonical_json(&mut output, value);
    output.into_bytes()
}

fn write_canonical_json(output: &mut String, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => output.push_str("null"),
        serde_json::Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        serde_json::Value::Number(value) => output.push_str(&value.to_string()),
        serde_json::Value::String(value) => {
            output.push('"');
            for ch in value.chars() {
                match ch {
                    '"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    ch => output.push(ch),
                }
            }
            output.push('"');
        }
        serde_json::Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, item);
            }
            output.push(']');
        }
        serde_json::Value::Object(map) => {
            output.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical_json(output, &serde_json::Value::String((*key).clone()));
                output.push(':');
                if let Some(value) = map.get(*key) {
                    write_canonical_json(output, value);
                }
            }
            output.push('}');
        }
    }
}

fn sample_handoff(
    handoff_id: &str,
    idempotency_key: &str,
    source_message_id: &str,
    source_lane_id: &str,
    source_kind: ModelLaneHandoffSourceKind,
    artifact_sha256: String,
    selection_state: ModelLaneHandoffSelectionState,
) -> NewModelLaneContextBundleHandoff {
    let decided = selection_state != ModelLaneHandoffSelectionState::Unresolved;
    let mut handoff = NewModelLaneContextBundleHandoff {
        handoff_id: handoff_id.into(),
        context_bundle_id: "CTX-placeholder".into(),
        run_id: "run-mt005".into(),
        trace_id: "trace-mt005".into(),
        handoff_span_id: format!("span-{handoff_id}"),
        parent_span_id: Some("span-coordinator-mt005".into()),
        linked_span_contexts: vec![format!("span-{source_message_id}")],
        downstream_lane_id: "lane-cloud".into(),
        source_lane_id: source_lane_id.into(),
        source_message_id: source_message_id.into(),
        artifact_ref: artifact_store_payload_ref(source_message_id),
        artifact_sha256: artifact_sha256.clone(),
        content_hash: artifact_sha256,
        source_kind,
        authority_state: ModelLaneAuthority::Advisory,
        selection_state,
        reason_code: format!("reason://mt005/{handoff_id}"),
        decision_ref: decided.then_some("context-decision://mt005/select".into()),
        reviewer_ref: decided.then_some("validator://mt005/context-bundle".into()),
        replay_hint: format!("replay://mt005/{handoff_id}"),
        crdt_payload: None,
        loom_refs: Vec::new(),
        memory_pack_refs: vec![sample_memory_pack(true)],
        event_ledger_stream_id: "mlane-stream-run-mt005".into(),
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-005".into(),
        task_board_id: "task-board://wp-1".into(),
        owner_session: "KERNEL_BUILDER-MT005".into(),
        idempotency_key: idempotency_key.into(),
        replay_order_key: format!("00000050/{handoff_id}"),
        created_at_utc: "2026-06-29T09:02:00Z".into(),
        diagnostic_payload: json!({
            "flight_recorder": "ContextBundle handoff EventLedger receipt required",
            "locus": "locus://wp1/mt005/coordinator-session-mt005",
            "loom": "artifact refs replay through host-side Loom evidence when present",
            "fems": "MemoryPack refs are explicit and reviewable",
            "palmistry": "external watcher link expected when feature is available"
        }),
    };
    handoff.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&handoff).expect("derive ContextBundle id");
    handoff
}

fn sample_crdt_payload() -> ModelLaneCrdtHandoffMetadata {
    ModelLaneCrdtHandoffMetadata {
        schema_id: "hsk.model_lane_crdt_payload@1".into(),
        document_id: "doc-mt005".into(),
        workspace_id: "workspace-mt005".into(),
        actor_id: "actor-lane-local".into(),
        actor_kind: "local_model".into(),
        lane_id: "lane-local".into(),
        crdt_site_id: "site-lane-local".into(),
        update_seq: 1,
        update_bytes_ref: "crdt-update://mt005/msg-proposal-001".into(),
        update_sha256: sample_sha256('6'),
        state_vector: "sv:mt005:1".into(),
        base_snapshot_ref: "crdt-snapshot://mt005/base-v1".into(),
        materialized_projection_hash: sample_sha256('7'),
        replay_metadata: json!({
            "format": "yjs_update_v1",
            "yjs_compatible": true,
            "flight_recorder": "eventledger://mt005/crdt/msg-proposal-001"
        }),
        promotion_gate_ref: "promotion-gate://mt005/preflight".into(),
        promotion_receipt_ref: None,
        validation_runner_ref: "validation-runner://mt005/crdt".into(),
        authority_effect: "advisory_only".into(),
    }
}

fn sample_loom_ref() -> ModelLaneLoomHandoffRef {
    ModelLaneLoomHandoffRef {
        workspace_id: "workspace-mt005".into(),
        block_id: "loom-block-001".into(),
        source_block_id: Some("loom-block-source".into()),
        target_block_id: Some("loom-block-target".into()),
        artifact_ref: Some("loom-artifact://mt005/block-001".into()),
        content_hash: sample_sha256('8'),
        version: "1".into(),
        event_ledger_evidence_ref: "eventledger://mt005/loom/block-001".into(),
        flight_recorder_evidence_ref: "flight-recorder://mt005/loom/block-001".into(),
    }
}

fn sample_memory_pack(cloud_safe: bool) -> ModelLaneMemoryPackHandoffRef {
    ModelLaneMemoryPackHandoffRef {
        memory_pack_ref: "memory-pack://mt005/cloud-review".into(),
        memory_pack_hash: sample_sha256('a'),
        scope_tag: "wp1.mt005.context_bundle".into(),
        review_status: "reviewed".into(),
        cloud_safe,
        classification: "cloud_safe_context".into(),
        projection_ref: Some("memory-projection://mt005/cloud-review".into()),
        evidence_ref: "eventledger://mt005/memory-pack/cloud-review".into(),
    }
}

fn sample_locus(session_id: &str, model_session_id: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1".into(),
        micro_task_id: "MT-005".into(),
        task_board_id: Some("task-board://wp-1".into()),
        coordinator_session_id: "coordinator-session-mt005".into(),
        session_id: session_id.into(),
        model_session_id: model_session_id.into(),
        owner_session: "KERNEL_BUILDER-MT005".into(),
        locus_binding_ref: "locus://wp1/mt005/coordinator-session-mt005".into(),
    }
}

fn sample_routing(
    correlation_id: &str,
    target_role: &str,
    target_session: &str,
) -> ModelLaneRoutingMetadata {
    ModelLaneRoutingMetadata {
        target_role: target_role.into(),
        target_session: target_session.into(),
        correlation_id: correlation_id.into(),
        requires_ack: true,
        ack_for: None,
    }
}

fn sample_sha256(ch: char) -> String {
    std::iter::repeat(ch).take(64).collect()
}
