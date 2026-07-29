//! WP-1 MT-005: Dexterity ContextBundle handoff runtime proof.
//!
//! These tests use real PostgreSQL plus the kernel EventLedger. They prove that
//! cloud/local model handoffs move through replayable ContextBundle rows, not
//! hidden provider memory or prompt-only state.

mod knowledge_pg_support;
mod model_lane_cloud_support;

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use base64::Engine;
use handshake_core::kernel::crdt::actor_site::{
    derive_knowledge_site_id, knowledge_crdt_identity, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::agent_lease::{
    claim_lease, expire_due_leases, release_lease, KnowledgeLeaseScopeKind, LeaseClaimOutcomeV1,
    LeaseClaimRequestV1,
};
use handshake_core::kernel::crdt::snapshot::{new_crdt_snapshot_record, CrdtSnapshotRecordInputV1};
use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
use handshake_core::kernel::crdt::yjs_bridge::{
    push_yjs_update, YjsPushOutcomeV1, YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1,
    YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::kernel::{DummyEchoModelAdapter, KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::process_ledger::{LedgerBatcher, LedgerBatcherConfig, NoopOverflowSink};
use handshake_core::storage::Database;
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
use yrs::updates::{decoder::Decode, encoder::Encode};
use yrs::{Doc, ReadTxn, StateVector, Text, Transact, Update};

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

    sqlx::query(
        r#"
        UPDATE model_lane_context_bundle_handoffs
        SET record_json = jsonb_set(record_json, '{context_bundle_hash}', '"forged-projection-hash"')
        WHERE handoff_id = $1
        "#,
    )
    .bind(&stored[0].handoff_id)
    .execute(&pool)
    .await
    .expect("tamper mutable ContextBundle projection for negative-path proof");
    let projection_tamper = store
        .replay_context_bundle_handoffs("run-mt005", &context_bundle_id)
        .await
        .expect_err("tampered ContextBundle projection must fail closed");
    assert!(
        projection_tamper
            .to_string()
            .contains("context_bundle_hash"),
        "projection tamper denial must identify hash authority: {projection_tamper}"
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
    const DOCUMENT_SCHEMA_ID: &str = "hsk.doc.rich_document@1";
    const SOURCE_MESSAGE_ID: &str = "msg-crdt-replayable-001";
    const UPDATE_ID: &str = "mt005-crdt-update-2";

    let kpg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("PostgreSQL/EventLedger is required for MT-005 CRDT replay proof");
    let workspace_id = kpg.create_workspace().await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&kpg.schema_url)
        .await
        .expect("connect isolated MT-005 CRDT schema");
    let store = ModelLaneStore::new(pool.clone());
    seed_run_with_messages(&store).await;

    let document_id = format!("doc-mt005-{workspace_id}");
    let crdt_document_id = format!("crdt-mt005-{workspace_id}");
    let actor = KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "mt005-lane-local")
        .expect("typed local actor");
    let site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &actor);
    let canonical = Doc::new();
    let mut state_vector = KnowledgeStateVectorV1::new();

    let base_update_bytes = mt005_append_yjs_text_update(
        &canonical,
        u64::from(site.yjs_client_id),
        "base snapshot text",
    );
    let base_update = mt005_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        "mt005-crdt-update-1",
        &actor,
        "session-mt005-bootstrap",
        &base_update_bytes,
        &state_vector,
        &site.site_id,
    );
    state_vector.increment(&site.site_id);
    assert!(matches!(
        push_yjs_update(&kpg.db, &base_update)
            .await
            .expect("persist base Yjs update"),
        YjsPushOutcomeV1::Stored { update_seq: 1, .. }
    ));

    let snapshot_state_vector = state_vector.encode();
    let snapshot_bytes = canonical
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let snapshot_identity = knowledge_crdt_identity(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        &actor,
        "trace-mt005-crdt-snapshot",
    );
    let snapshot_event = NewKernelEvent::builder(
        format!("KTR-MT005-CRDT-SNAPSHOT-{workspace_id}"),
        "session-lane-local".to_string(),
        KernelEventType::KnowledgeCrdtSnapshotRecorded,
        actor.to_kernel_actor(),
    )
    .aggregate("knowledge_crdt_document", crdt_document_id.clone())
    .idempotency_key(format!("mt005:{workspace_id}:snapshot"))
    .source_component("model_lane_context_bundle_pg_tests")
    .payload(json!({
        "covered_update_seq": 1,
        "state_vector": &snapshot_state_vector,
        "document_id": &document_id,
    }))
    .build()
    .expect("build CRDT snapshot event");
    let snapshot_event = kpg
        .db
        .append_kernel_event(snapshot_event)
        .await
        .expect("append CRDT snapshot event");
    let snapshot_ref =
        format!("postgres://kernel_crdt_snapshots/{crdt_document_id}/mt005-crdt-snapshot-1");
    let snapshot = new_crdt_snapshot_record(CrdtSnapshotRecordInputV1 {
        identity: &snapshot_identity,
        snapshot_id: "mt005-crdt-snapshot-1",
        covered_update_seq: 1,
        snapshot_bytes: &snapshot_bytes,
        snapshot_bytes_ref: &snapshot_ref,
        state_vector: &snapshot_state_vector,
        event_ledger_event_id: &snapshot_event.event_id,
        promotion_evidence_update_ids: &[],
    });
    kpg.db
        .append_kernel_crdt_snapshot(snapshot.clone(), snapshot_bytes)
        .await
        .expect("persist CRDT base snapshot");

    let update_bytes =
        mt005_append_yjs_text_update(&canonical, u64::from(site.yjs_client_id), " lane update");
    let update = mt005_yjs_envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        DOCUMENT_SCHEMA_ID,
        UPDATE_ID,
        &actor,
        "session-lane-local",
        &update_bytes,
        &state_vector,
        &site.site_id,
    );
    state_vector.increment(&site.site_id);
    assert!(matches!(
        push_yjs_update(&kpg.db, &update)
            .await
            .expect("persist lane Yjs update"),
        YjsPushOutcomeV1::Stored { update_seq: 2, .. }
    ));
    let records = kpg
        .db
        .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("list authoritative CRDT updates");
    let update_record = records
        .iter()
        .find(|record| record.update_id == UPDATE_ID)
        .expect("lane update receipt");
    let materialized_projection = canonical
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let materialized_projection_hash = sha256_hex(&materialized_projection);

    let mut source = advisory_message(
        SOURCE_MESSAGE_ID,
        "idem-message-crdt-replayable-001",
        "lane-local",
        ModelLaneMessageKind::Status,
        artifact_payload_hash(SOURCE_MESSAGE_ID),
        "local lane submits replayable CRDT state",
    );
    source.crdt_update_ref = Some(update_record.update_bytes_ref.clone());
    source.crdt_base_snapshot_ref = Some(snapshot.snapshot_bytes_ref.clone());
    source.crdt_state_vector = Some(update_record.state_vector_after.clone());
    source.linked_span_contexts.push(update.trace_id.clone());
    store
        .record_context_bundle_artifact_binding(sample_artifact_binding_for_message(&source))
        .await
        .expect("persist source ArtifactStore binding");
    let missing_lease_error = store
        .record_message(source.clone())
        .await
        .expect_err("CRDT actor without a persisted lane lease must fail closed");
    assert!(
        missing_lease_error
            .to_string()
            .contains("no persisted knowledge-agent lease binding"),
        "missing actor/lane lease denial must be explicit: {missing_lease_error}"
    );
    let authority_probe = |suffix: &str| {
        let mut probe = source.clone();
        probe.message_id = format!("msg-crdt-lease-{suffix}");
        probe.message_span_id = format!("span-crdt-lease-{suffix}");
        probe.idempotency_key = format!("idem-message-crdt-lease-{suffix}");
        probe
    };

    // A release on a second PostgreSQL connection must serialize behind the
    // same workspace/document authority lock held by ModelLane admission.
    let release_race_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim admission-vs-release race lease")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("admission-vs-release race lease must claim: {other:?}"),
    };
    let release_entered = Arc::new(tokio::sync::Notify::new());
    let release_admission = Arc::new(tokio::sync::Notify::new());
    let release_race_message = authority_probe("release-race");
    let admission_future = store.test_record_message_holding_crdt_authority_lock(
        release_race_message.clone(),
        release_entered.clone(),
        release_admission.clone(),
    );
    let release_future = async {
        release_entered.notified().await;
        let mut release = Box::pin(release_lease(
            &kpg.db,
            &pool,
            &release_race_lease.lease_id,
            &actor,
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut release)
                .await
                .is_err(),
            "lease release must block while ModelLane admission owns the CRDT authority lock"
        );
        release_admission.notify_one();
        release.await
    };
    let (admitted, released) = tokio::join!(admission_future, release_future);
    let admitted = admitted.expect("admission wins release race");
    assert_eq!(admitted.message_id, release_race_message.message_id);
    released
        .expect("release after admission")
        .expect("race lease remains releasable after admission commits");

    // A second covering workspace claim cannot appear as a phantom between
    // the admission query and the durable MODEL_RESPONSE_RECORDED append.
    let claim_race_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim admission-vs-second-claim race lease")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("admission-vs-second-claim race lease must claim: {other:?}"),
    };
    let claim_entered = Arc::new(tokio::sync::Notify::new());
    let claim_admission = Arc::new(tokio::sync::Notify::new());
    let claim_race_message = authority_probe("second-claim-race");
    let admission_future = store.test_record_message_holding_crdt_authority_lock(
        claim_race_message.clone(),
        claim_entered.clone(),
        claim_admission.clone(),
    );
    let second_claim_future = async {
        claim_entered.notified().await;
        let mut second_claim = Box::pin(claim_lease(
            &kpg.db,
            &pool,
            LeaseClaimRequestV1 {
                lane_id: "lane-local".into(),
                actor: actor.clone(),
                session_id: "session-lane-local".into(),
                correlation_id: update.trace_id.clone(),
                scope_kind: KnowledgeLeaseScopeKind::Workspace,
                scope_id: workspace_id.clone(),
                ttl_seconds: 3600,
            },
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut second_claim)
                .await
                .is_err(),
            "covering workspace claim must block while ModelLane admission owns the lock domain"
        );
        claim_admission.notify_one();
        second_claim.await
    };
    let (admitted, second_claim) = tokio::join!(admission_future, second_claim_future);
    let admitted = admitted.expect("admission wins second-covering-claim race");
    assert_eq!(admitted.message_id, claim_race_message.message_id);
    let workspace_race_lease = match second_claim.expect("second claim after admission") {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("workspace claim must complete after admission: {other:?}"),
    };
    for lease_id in [&claim_race_lease.lease_id, &workspace_race_lease.lease_id] {
        release_lease(&kpg.db, &pool, lease_id, &actor)
            .await
            .expect("release second-claim race lease")
            .expect("second-claim race lease exists");
    }

    // Natural expiry after the admission instant remains a valid immutable
    // receipt. The production sweep must wait for admission to commit, and a
    // later replay must revalidate the historical lease proof rather than
    // requiring the lease to still be active.
    let expiry_race_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 5,
        },
    )
    .await
    .expect("claim short-TTL admission-vs-sweep lease")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("short-TTL admission-vs-sweep lease must claim: {other:?}"),
    };
    let sweep_entered = Arc::new(tokio::sync::Notify::new());
    let sweep_admission = Arc::new(tokio::sync::Notify::new());
    let expiry_race_message = authority_probe("natural-expiry-sweep-race");
    let admission_future = store.test_record_message_holding_crdt_authority_lock(
        expiry_race_message.clone(),
        sweep_entered.clone(),
        sweep_admission.clone(),
    );
    let sweep_future = async {
        sweep_entered.notified().await;
        tokio::time::sleep(Duration::from_millis(5_200)).await;
        let mut sweep = Box::pin(expire_due_leases(&kpg.db, &pool));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut sweep)
                .await
                .is_err(),
            "expiry sweep must block while ModelLane admission owns the CRDT authority lock"
        );
        sweep_admission.notify_one();
        sweep.await
    };
    let (admitted, swept) = tokio::join!(admission_future, sweep_future);
    let admitted = admitted.expect("admission remains valid across natural expiry");
    assert_eq!(admitted.message_id, expiry_race_message.message_id);
    let swept = swept.expect("sweep after admission");
    assert!(
        swept
            .iter()
            .any(|lease| lease.lease_id == expiry_race_lease.lease_id),
        "production sweep must stamp the naturally expired admitted lease"
    );
    release_lease(&kpg.db, &pool, &expiry_race_lease.lease_id, &actor)
        .await
        .expect("release naturally expired lease")
        .expect("naturally expired lease exists");
    let replay_after_natural_expiry = store
        .replay_run("run-mt005")
        .await
        .expect("historically admitted CRDT message replays after natural lease expiry");
    assert!(replay_after_natural_expiry
        .messages
        .iter()
        .any(|message| message.message_id == expiry_race_message.message_id));

    let released_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim released-lease denial probe")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("released-lease denial probe must claim: {other:?}"),
    };
    release_lease(&kpg.db, &pool, &released_lease.lease_id, &actor)
        .await
        .expect("release denial-probe lease")
        .expect("released denial-probe lease exists");
    let released_error = store
        .record_message(authority_probe("released"))
        .await
        .expect_err("released CRDT lane lease must fail closed");
    assert!(released_error.to_string().contains("exact and active"));

    let expired_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim expired-lease denial probe")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("expired-lease denial probe must claim: {other:?}"),
    };
    sqlx::query(
        "UPDATE knowledge_crdt_agent_lane_leases SET expires_at_utc = claimed_at_utc + INTERVAL '1 microsecond' WHERE lease_id = $1",
    )
    .bind(&expired_lease.lease_id)
    .execute(&pool)
    .await
    .expect("move denial-probe lease expiry behind the database clock");
    let expired_error = store
        .record_message(authority_probe("expired"))
        .await
        .expect_err("expired CRDT lane lease must fail closed");
    assert!(expired_error.to_string().contains("exact and active"));
    release_lease(&kpg.db, &pool, &expired_lease.lease_id, &actor)
        .await
        .expect("release expired denial-probe lease")
        .expect("expired denial-probe lease exists");

    let correlation_mismatch_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: "trace-unrelated-crdt-update".into(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim correlation-mismatch denial probe")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("correlation-mismatch denial probe must claim: {other:?}"),
    };
    let correlation_error = store
        .record_message(authority_probe("correlation-mismatch"))
        .await
        .expect_err("foreign-correlation CRDT lane lease must fail closed");
    assert!(correlation_error.to_string().contains("exact and active"));
    release_lease(&kpg.db, &pool, &correlation_mismatch_lease.lease_id, &actor)
        .await
        .expect("release correlation-mismatch denial-probe lease")
        .expect("correlation-mismatch denial-probe lease exists");

    let scope_mismatch_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::IndexRun,
            scope_id: format!("unrelated-index-run-{workspace_id}"),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim scope-mismatch denial probe")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("scope-mismatch denial probe must claim: {other:?}"),
    };
    let scope_error = store
        .record_message(authority_probe("scope-mismatch"))
        .await
        .expect_err("unrelated-scope CRDT lane lease must fail closed");
    assert!(scope_error.to_string().contains("exact and active"));
    release_lease(&kpg.db, &pool, &scope_mismatch_lease.lease_id, &actor)
        .await
        .expect("release scope-mismatch denial-probe lease")
        .expect("scope-mismatch denial-probe lease exists");

    let workspace_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Workspace,
            scope_id: workspace_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim ambiguous workspace lease")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("ambiguous workspace lease must claim: {other:?}"),
    };
    let document_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim ambiguous document lease")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("ambiguous document lease must claim: {other:?}"),
    };
    let ambiguous_error = store
        .record_message(authority_probe("ambiguous"))
        .await
        .expect_err("multiple covering CRDT lane leases must fail closed");
    assert!(
        ambiguous_error.to_string().contains("ambiguous active"),
        "ambiguous lease denial must be explicit: {ambiguous_error}"
    );
    for lease_id in [&workspace_lease.lease_id, &document_lease.lease_id] {
        release_lease(&kpg.db, &pool, lease_id, &actor)
            .await
            .expect("release ambiguous denial-probe lease")
            .expect("ambiguous denial-probe lease exists");
    }

    let active_lease = match claim_lease(
        &kpg.db,
        &pool,
        LeaseClaimRequestV1 {
            lane_id: "lane-local".into(),
            actor: actor.clone(),
            session_id: "session-lane-local".into(),
            correlation_id: update.trace_id.clone(),
            scope_kind: KnowledgeLeaseScopeKind::Document,
            scope_id: crdt_document_id.clone(),
            ttl_seconds: 3600,
        },
    )
    .await
    .expect("claim MT-005 actor/lane binding lease")
    {
        LeaseClaimOutcomeV1::Claimed(lease) => lease,
        other => panic!("MT-005 actor/lane binding lease must claim: {other:?}"),
    };
    let stored_source = store
        .record_message(source.clone())
        .await
        .expect("persist lane-bound CRDT source message");
    let authority_binding = stored_source
        .crdt_authority_binding
        .as_ref()
        .expect("source message carries durable CRDT lane binding");
    assert_eq!(authority_binding.run_id, "run-mt005");
    assert_eq!(authority_binding.lane_id, "lane-local");
    assert_eq!(authority_binding.lease_id, active_lease.lease_id);
    assert_eq!(authority_binding.lease_correlation_id, update.trace_id);
    assert_eq!(authority_binding.lease_scope_kind, "document");
    assert_eq!(authority_binding.lease_scope_id, crdt_document_id);
    assert_eq!(
        authority_binding.lease_claimed_at_utc,
        active_lease.claimed_at_utc
    );
    assert_eq!(
        authority_binding.lease_expires_at_utc,
        active_lease.expires_at_utc
    );
    assert!(authority_binding.lease_admitted_at_utc >= active_lease.claimed_at_utc);
    assert!(authority_binding.lease_admitted_at_utc < active_lease.expires_at_utc);
    assert_eq!(
        authority_binding.materialized_projection_hash,
        materialized_projection_hash
    );
    assert_eq!(
        authority_binding.yjs_state_vector_b64,
        base64::engine::general_purpose::STANDARD
            .encode(canonical.transact().state_vector().encode_v1()),
        "stored CRDT authority must equal the Yjs state vector derived from persisted bytes"
    );
    let event_ledger_binding: serde_json::Value = sqlx::query_scalar(
        "SELECT payload->'crdt_authority_binding' FROM kernel_event_ledger WHERE event_id = $1",
    )
    .bind(&stored_source.event_ledger_event_id)
    .fetch_one(&pool)
    .await
    .expect("read immutable CRDT lease authority from EventLedger");
    assert_eq!(
        event_ledger_binding,
        serde_json::to_value(authority_binding).expect("serialize CRDT lease authority binding"),
        "projection and EventLedger must persist the same full CRDT lease authority proof"
    );
    release_lease(&kpg.db, &pool, &active_lease.lease_id, &actor)
        .await
        .expect("release admitted MT-005 actor/lane binding lease")
        .expect("admitted MT-005 actor/lane binding lease exists");
    let replay_after_release = store
        .replay_run("run-mt005")
        .await
        .expect("historical CRDT replay remains valid after its admitted lease is released");
    assert_eq!(
        replay_after_release
            .messages
            .iter()
            .find(|message| message.message_id == SOURCE_MESSAGE_ID)
            .and_then(|message| message.crdt_authority_binding.as_ref()),
        Some(authority_binding)
    );

    let mut proposal_without_crdt_proposal = source.clone();
    proposal_without_crdt_proposal.message_id = "msg-crdt-proposal-ref-required".into();
    proposal_without_crdt_proposal.idempotency_key =
        "idem-message-crdt-proposal-ref-required".into();
    proposal_without_crdt_proposal.kind = ModelLaneMessageKind::Proposal;
    let proposal_error = store
        .record_message(proposal_without_crdt_proposal)
        .await
        .expect_err("Proposal messages carrying CRDT updates require persisted proposal authority");
    assert!(
        proposal_error.to_string().contains("crdt_proposal_ref"),
        "missing CRDT proposal authority denial must be explicit: {proposal_error}"
    );

    let mut cross_lane = source.clone();
    cross_lane.message_id = "msg-crdt-cross-lane-denied".into();
    cross_lane.idempotency_key = "idem-message-crdt-cross-lane-denied".into();
    cross_lane.from_lane_id = "lane-cloud".into();
    cross_lane.parent_span_id = Some("span-lane-cloud".into());
    cross_lane.locus_binding = Some(sample_locus(
        "session-lane-cloud",
        "model-session-lane-cloud",
    ));
    let cross_lane_error = store
        .record_message(cross_lane)
        .await
        .expect_err("another lane cannot claim the local lane CRDT update");
    // Post-644dee55 fail-closed trace (see
    // src/backend/handshake_core/src/swarm_orchestration/model_lane.rs). Now that
    // model-authored CRDT authority is admissible, this cross-lane claim reaches
    // the durable CRDT authority path and is denied at the FIRST cross-lane guard:
    //   1. validate_message_crdt_authority_tx (~441) resolves the same local CRDT
    //      authority the source message carries: actor "session-lane-local".
    //   2. validate_crdt_lane_session_uniqueness_tx (~449) fires BEFORE both
    //      resolve_active_crdt_actor_lane_lease_tx (~450) and
    //      bind_crdt_authority_to_lane (~458). It proves the resolved CRDT session
    //      is uniquely owned by exactly one ModelLane (lane-local) and rejects any
    //      other source lane claiming it.
    // This IS the correct cross-lane guard: the local CRDT update is authored under
    // session "session-lane-local", which model_lanes proves belongs to lane-local.
    // A cloud-lane lease for that local session cannot legitimately exist (the
    // uniqueness query requires exactly one owning lane), so the attribution check
    // at model_lane.rs:12938 ("cannot be attributed") is unreachable for a genuine
    // cross-lane claim and asserting it here would assert a denial the resolver
    // never produces. We assert the real, stronger session-ownership denial instead
    // (fail-closed preserved: the message is still rejected, and the reason names
    // the true owning lane vs the offending source lane).
    let cross_lane_denial = cross_lane_error.to_string();
    assert!(
        cross_lane_denial
            .contains("crdt session session-lane-local belongs to run run-mt005 lane lane-local")
            && cross_lane_denial.contains("not source run run-mt005 lane lane-cloud"),
        "cross-lane CRDT claim must fail closed on session ownership (lane-local owns the update, lane-cloud cannot claim it): {cross_lane_denial}"
    );

    let mut handoff = sample_handoff(
        "handoff-crdt-replayable",
        "idem-handoff-crdt-replayable",
        SOURCE_MESSAGE_ID,
        "lane-local",
        ModelLaneHandoffSourceKind::Proposal,
        source.payload_sha256.clone(),
        ModelLaneHandoffSelectionState::Selected,
    );
    handoff.crdt_payload = Some(ModelLaneCrdtHandoffMetadata {
        schema_id: "hsk.model_lane_crdt_payload@1".into(),
        document_id: document_id.clone(),
        workspace_id: workspace_id.clone(),
        actor_id: actor.canonical(),
        actor_kind: KnowledgeActorKind::LocalModel.as_str().into(),
        lane_id: "lane-local".into(),
        crdt_site_id: site.site_id.clone(),
        update_seq: 2,
        update_bytes_ref: update_record.update_bytes_ref.clone(),
        update_sha256: update_record.update_sha256.clone(),
        state_vector: update_record.state_vector_after.clone(),
        base_snapshot_ref: snapshot.snapshot_bytes_ref.clone(),
        materialized_projection_hash: materialized_projection_hash.clone(),
        replay_metadata: json!({
            "format": "yjs_update_v1",
            "yjs_compatible": true,
            "replay_order_key": update_record.replay_metadata.replay_order_key,
            "dependency_update_ids": update_record.replay_metadata.dependency_update_ids,
            "schema_version": update_record.replay_metadata.schema_version,
        }),
        promotion_gate_ref: format!("promotion-gate://model-lane-message/{SOURCE_MESSAGE_ID}"),
        promotion_receipt_ref: None,
        validation_runner_ref: format!("eventledger://{}", update_record.event_ledger_event_id),
        authority_effect: "advisory_only".into(),
    });
    handoff.loom_refs = vec![sample_loom_ref()];
    handoff.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&handoff).expect("derive CRDT ContextBundle id");
    let context_bundle_id = handoff.context_bundle_id.clone();
    let mut forged_projection = handoff.clone();
    forged_projection.handoff_id = "handoff-crdt-forged-projection".into();
    forged_projection.idempotency_key = "idem-handoff-crdt-forged-projection".into();
    forged_projection
        .crdt_payload
        .as_mut()
        .expect("CRDT payload")
        .materialized_projection_hash = sample_sha256('f');
    forged_projection.context_bundle_id =
        model_lane_context_bundle_id_for_handoff(&forged_projection)
            .expect("derive forged projection ContextBundle id");
    let forged_projection_error = store
        .record_context_bundle_handoff(forged_projection)
        .await
        .expect_err("fabricated materialized projection hash must fail closed");
    assert!(forged_projection_error
        .to_string()
        .contains("materialized_projection_hash"));
    for (suffix, field, forged_value) in [
        ("order", "replay_order_key", json!("forged/replay/order")),
        ("schema", "schema_version", json!("forged-crdt-schema-v9")),
        (
            "dependencies",
            "dependency_update_ids",
            json!(["forged-missing-dependency"]),
        ),
    ] {
        let mut forged_replay = handoff.clone();
        forged_replay.handoff_id = format!("handoff-crdt-forged-replay-{suffix}");
        forged_replay.idempotency_key = format!("idem-handoff-crdt-forged-replay-{suffix}");
        forged_replay
            .crdt_payload
            .as_mut()
            .expect("CRDT payload")
            .replay_metadata[field] = forged_value;
        forged_replay.context_bundle_id = model_lane_context_bundle_id_for_handoff(&forged_replay)
            .expect("derive forged replay ContextBundle id");
        let forged_handoff_id = forged_replay.handoff_id.clone();
        let forged_replay_error = store
            .record_context_bundle_handoff(forged_replay)
            .await
            .expect_err("fabricated replay metadata must fail closed");
        assert!(forged_replay_error
            .to_string()
            .contains(&format!("replay_metadata.{field}")));
        let forged_replay_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM model_lane_context_bundle_handoffs WHERE handoff_id = $1",
        )
        .bind(&forged_handoff_id)
        .fetch_one(&pool)
        .await
        .expect("count rejected forged replay handoff rows");
        assert_eq!(
            forged_replay_rows, 0,
            "forged replay authority must leave no durable handoff row"
        );
    }
    store
        .record_context_bundle_handoff(handoff)
        .await
        .expect("persist authoritative CRDT ContextBundle handoff");

    let restarted = ModelLaneStore::new(pool.clone());
    let replay = restarted
        .replay_context_bundle_handoffs("run-mt005", &context_bundle_id)
        .await
        .expect("replay CRDT ContextBundle after store restart");
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].loom_refs, vec![sample_loom_ref()]);
    let replayed_crdt = replay[0]
        .crdt_payload
        .as_ref()
        .expect("replayed CRDT payload");
    assert_eq!(replayed_crdt.state_vector, update_record.state_vector_after);
    assert_eq!(
        replayed_crdt.materialized_projection_hash,
        materialized_projection_hash
    );
    let consumed = restarted
        .consume_context_bundle_for_downstream("run-mt005", &context_bundle_id, "lane-cloud")
        .await
        .expect("downstream consumes replayed CRDT ContextBundle");
    assert_eq!(consumed.run_id, "run-mt005");
    assert_eq!(consumed.context_bundle_id, context_bundle_id);
    assert_eq!(consumed.downstream_lane_id, "lane-cloud");
    assert_eq!(consumed.records.len(), 1);
    assert_eq!(consumed.records.as_slice(), replay.as_slice());

    let update_mutation = sqlx::query(
        "UPDATE kernel_crdt_updates SET trace_id = trace_id || '-forged' WHERE update_id = $1",
    )
    .bind(UPDATE_ID)
    .execute(&pool)
    .await
    .expect_err("persisted CRDT update authority must be immutable");
    assert!(update_mutation
        .to_string()
        .contains("append-only CRDT authority"));
    let snapshot_mutation = sqlx::query(
        "UPDATE kernel_crdt_snapshots SET actor_kind = 'system' WHERE snapshot_id = $1",
    )
    .bind(&snapshot.snapshot_id)
    .execute(&pool)
    .await
    .expect_err("persisted CRDT snapshot authority must be immutable");
    assert!(snapshot_mutation
        .to_string()
        .contains("append-only CRDT authority"));

    let stored_handoff = &replay[0];
    sqlx::query(
        r#"
        UPDATE kernel_event_ledger
        SET payload = jsonb_set(payload, '{record,context_bundle_hash}', '"forged-ledger-hash"')
        WHERE event_id = $1
        "#,
    )
    .bind(&stored_handoff.event_ledger_event_id)
    .execute(&pool)
    .await
    .expect("tamper ContextBundle EventLedger payload for negative-path proof");
    let ledger_tamper = restarted
        .replay_context_bundle_handoffs("run-mt005", &context_bundle_id)
        .await
        .expect_err("tampered CONTEXT_BUNDLE_RECORDED payload must fail closed");
    assert!(
        ledger_tamper
            .to_string()
            .contains("CONTEXT_BUNDLE_RECORDED"),
        "ledger tamper denial must identify ContextBundle EventLedger authority: {ledger_tamper}"
    );

    sqlx::query(
        "UPDATE model_lane_messages SET record_json = record_json - 'crdt_authority_binding' WHERE message_id = $1",
    )
    .bind(SOURCE_MESSAGE_ID)
    .execute(&pool)
    .await
    .expect("remove legacy CRDT binding for fail-closed replay proof");
    let missing_binding = restarted
        .replay_run("run-mt005")
        .await
        .expect_err("legacy CRDT message without full binding must fail closed");
    assert!(
        missing_binding
            .to_string()
            .contains("no persisted lease authority binding"),
        "missing binding denial must be explicit: {missing_binding}"
    );
}

#[tokio::test]
async fn model_lane_context_bundle_rejects_fabricated_crdt_authority_before_persistence() {
    let (_pool, store) = model_lane_store().await;
    seed_run_with_messages(&store).await;

    let mut fabricated = sample_handoff(
        "handoff-crdt-fabricated",
        "idem-handoff-crdt-fabricated",
        "msg-proposal-001",
        "lane-local",
        ModelLaneHandoffSourceKind::Proposal,
        artifact_payload_hash("msg-proposal-001"),
        ModelLaneHandoffSelectionState::Selected,
    );
    fabricated.crdt_payload = Some(sample_crdt_payload());
    fabricated.loom_refs = vec![sample_loom_ref()];
    fabricated.memory_pack_refs = vec![sample_memory_pack(true)];
    fabricated.context_bundle_id = model_lane_context_bundle_id_for_handoff(&fabricated)
        .expect("derive fabricated ContextBundle id");
    let bundle_id = fabricated.context_bundle_id.clone();

    let fabricated_err = store
        .record_context_bundle_handoff(fabricated)
        .await
        .expect_err("syntax-shaped CRDT metadata must not satisfy PostgreSQL/Yjs authority");
    assert!(
        fabricated_err
            .to_string()
            .contains("CRDT authority resolution failed"),
        "fabricated CRDT authority failure must be explicit: {fabricated_err}"
    );
    assert!(
        store
            .replay_context_bundle_handoffs("run-mt005", &bundle_id)
            .await
            .expect("replay after rejected handoff")
            .is_empty(),
        "rejected fabricated CRDT handoff must leave no durable row"
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

    // Cloud lanes fail closed unless durable ProjectionPlan/ConsentReceipt
    // authority already exists (spec 4.3.9.2.5, CX-MM-007). Seed the cloud
    // lane's authority before recording it, matching the identity that
    // `sample_lane("lane-cloud", Cloud, ...)` stamps.
    model_lane_cloud_support::seed_cloud_lane_authority(
        store,
        model_lane_cloud_support::CloudLaneAuthoritySpec {
            run_id: "run-mt005",
            lane_id: "lane-cloud",
            model_session_id: "model-session-lane-cloud",
            provider_kind: ModelLaneProviderKind::OpenAi.as_str(),
            requested_model_id: "model://mt005/lane-cloud",
            projection_plan_id: "projection-plan://lane-cloud",
            consent_receipt_id: "consent://lane-cloud",
            event_ledger_stream_id: "mlane-stream-run-mt005",
            work_packet_id: "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1",
            micro_task_id: "MT-005",
            task_board_id: "task-board://wp-1",
            owner_session: "KERNEL_BUILDER-MT005",
        },
    )
    .await;

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
        proposal_ref: None,
        crdt_update_ref: None,
        crdt_base_snapshot_ref: None,
        crdt_state_vector: None,
        crdt_proposal_ref: None,
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

fn mt005_append_yjs_text_update(canonical: &Doc, client_id: u64, text: &str) -> Vec<u8> {
    let canonical_state = canonical
        .transact()
        .encode_state_as_update_v1(&StateVector::default());
    let author = Doc::with_client_id(client_id);
    let author_text = author.get_or_insert_text("mt005-shared-document");
    author
        .transact_mut()
        .apply_update(Update::decode_v1(&canonical_state).expect("decode canonical Yjs state"))
        .expect("apply canonical Yjs state to author replica");
    let before = author.transact().state_vector();
    {
        let mut transaction = author.transact_mut();
        let offset = author_text.len(&transaction);
        author_text.insert(&mut transaction, offset, text);
    }
    let update = author.transact().encode_diff_v1(&before);
    canonical
        .transact_mut()
        .apply_update(Update::decode_v1(&update).expect("decode generated Yjs update"))
        .expect("apply generated Yjs update to canonical replica");
    update
}

#[allow(clippy::too_many_arguments)]
fn mt005_yjs_envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    document_schema_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
    update_bytes: &[u8],
    before: &KnowledgeStateVectorV1,
    site_id: &str,
) -> YjsUpdateEnvelopeV1 {
    let mut after = before.clone();
    after.increment(site_id);
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.into(),
        workspace_id: workspace_id.into(),
        document_id: document_id.into(),
        crdt_document_id: crdt_document_id.into(),
        update_id: update_id.into(),
        actor_id: actor.canonical(),
        site_id: site_id.into(),
        session_id: session_id.into(),
        trace_id: format!("trace-{update_id}"),
        document_schema_id: document_schema_id.into(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(update_bytes),
        update_sha256: sha256_hex(update_bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.into(),
    }
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
            "replay_order_key": "workspace-mt005/doc-mt005/00000000000000000001",
            "dependency_update_ids": [],
            "schema_version": "kernel-crdt-update-v1",
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
