//! WP-KERNEL-009 MT-237 ParallelWriteConflictFixture.
//!
//! Real PostgreSQL/EventLedger proof that concurrent operator/model/validator
//! CRDT draft writes do not silently overwrite or drop state. Denied model and
//! validator writes leave durable denial receipts, render through
//! `/knowledge/crdt/conflict_state`, and repair by pull/merge/resubmit.

#[allow(dead_code)]
mod knowledge_pg_support;

use std::{collections::BTreeSet, sync::Arc};

use base64::Engine;
use handshake_core::api::knowledge_crdt::{router_with_state, KnowledgeCrdtApiState};
use handshake_core::kernel::crdt::actor_site::{
    derive_knowledge_site_id, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::persistence::sha256_hex;
use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
use handshake_core::kernel::crdt::yjs_bridge::{
    pull_yjs_updates, read_draft_head, YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1,
    YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::knowledge_crdt::list_denial_receipts_for_document;
use handshake_core::storage::Database;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Barrier;
use uuid::Uuid;

#[derive(Clone)]
struct InitialWriteAttempt {
    envelope: YjsUpdateEnvelopeV1,
    repair_update_id: &'static str,
    repair_bytes: &'static [u8],
}

#[allow(clippy::too_many_arguments)]
fn envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
    correlation_id: &str,
    bytes: &[u8],
    before: &KnowledgeStateVectorV1,
    after: &KnowledgeStateVectorV1,
) -> YjsUpdateEnvelopeV1 {
    let site = derive_knowledge_site_id(workspace_id, crdt_document_id, actor);
    YjsUpdateEnvelopeV1 {
        schema_id: YJS_UPDATE_ENVELOPE_SCHEMA_ID.to_string(),
        workspace_id: workspace_id.to_string(),
        document_id: document_id.to_string(),
        crdt_document_id: crdt_document_id.to_string(),
        update_id: update_id.to_string(),
        actor_id: actor.canonical(),
        site_id: site.site_id,
        session_id: session_id.to_string(),
        trace_id: correlation_id.to_string(),
        document_schema_id: "hsk.doc.rich_document@1".to_string(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(bytes),
        update_sha256: sha256_hex(bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    }
}

async fn serve_knowledge_crdt(db: Arc<dyn Database>, pool: sqlx::PgPool) -> String {
    let app = router_with_state(KnowledgeCrdtApiState { db, pool });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback CRDT fixture listener");
    let addr = listener.local_addr().expect("fixture listener addr");
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve CRDT fixture router");
    });
    format!("http://{addr}")
}

async fn push_update_http(
    client: &reqwest::Client,
    base_url: &str,
    envelope: &YjsUpdateEnvelopeV1,
) -> (reqwest::StatusCode, Value) {
    let response = client
        .post(format!("{base_url}/knowledge/crdt/updates/push"))
        .json(&json!({ "envelope": envelope }))
        .send()
        .await
        .expect("push update request");
    let status = response.status();
    let body = response.json().await.expect("push update json body");
    (status, body)
}

fn assert_stored_push(
    body: &Value,
    envelope: &YjsUpdateEnvelopeV1,
    update_seq: u64,
    head_state_vector: &str,
) {
    assert_eq!(body["result"]["outcome"].as_str(), Some("stored"));
    assert_eq!(body["result"]["update_seq"].as_u64(), Some(update_seq));
    assert_eq!(
        body["result"]["update_id"].as_str(),
        Some(envelope.update_id.as_str())
    );
    assert_eq!(
        body["result"]["head_state_vector"].as_str(),
        Some(head_state_vector)
    );
    assert_eq!(body["receipt"]["operation"].as_str(), Some("push_update"));
    assert_eq!(
        body["receipt"]["actor_id"].as_str(),
        Some(envelope.actor_id.as_str())
    );
    assert_eq!(
        body["receipt"]["session_id"].as_str(),
        Some(envelope.session_id.as_str())
    );
    assert_eq!(
        body["receipt"]["correlation_id"].as_str(),
        Some(envelope.trace_id.as_str())
    );
}

fn assert_stale_push_denied(
    body: &Value,
    envelope: &YjsUpdateEnvelopeV1,
    head_update_seq: u64,
    head_state_vector: &str,
) {
    assert_eq!(body["result"]["outcome"].as_str(), Some("denied"));
    assert_eq!(
        body["result"]["denial"]["crdt_document_id"].as_str(),
        Some(envelope.crdt_document_id.as_str())
    );
    assert_eq!(
        body["result"]["denial"]["update_id"].as_str(),
        Some(envelope.update_id.as_str())
    );
    assert_eq!(
        body["result"]["denial"]["actor_id"].as_str(),
        Some(envelope.actor_id.as_str())
    );
    assert_eq!(
        body["result"]["denial"]["reason"]["code"].as_str(),
        Some("stale_base")
    );
    assert_eq!(
        body["result"]["denial"]["reason"]["head_update_seq"].as_u64(),
        Some(head_update_seq)
    );
    assert_eq!(
        body["result"]["denial"]["reason"]["head_state_vector"].as_str(),
        Some(head_state_vector)
    );
    assert_eq!(body["receipt"]["operation"].as_str(), Some("push_update"));
    assert_eq!(
        body["receipt"]["actor_id"].as_str(),
        Some(envelope.actor_id.as_str())
    );
    assert_eq!(
        body["receipt"]["session_id"].as_str(),
        Some(envelope.session_id.as_str())
    );
    assert_eq!(
        body["receipt"]["correlation_id"].as_str(),
        Some(envelope.trace_id.as_str())
    );
}

fn ids_from_pull(updates: &[YjsUpdateEnvelopeV1]) -> BTreeSet<String> {
    updates
        .iter()
        .map(|update| update.update_id.clone())
        .collect()
}

fn decoded_update_text(update: &YjsUpdateEnvelopeV1) -> String {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(update.update_b64.as_bytes())
        .expect("pull update bytes decode from base64");
    String::from_utf8(bytes).expect("MT-237 sentinel update bytes are UTF-8")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mt237_parallel_model_validator_conflicts_leave_repairable_state() {
    let Some(pg) = knowledge_pg_support::knowledge_pg().await else {
        panic!(
            "ENVIRONMENT_BLOCKED: MT-237 requires real PostgreSQL or Handshake-managed PostgreSQL"
        );
    };
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect pool to MT-237 isolated schema");
    let db: Arc<dyn Database> = Arc::new(pg.db);
    let base_url = serve_knowledge_crdt(db.clone(), pool.clone()).await;
    let client = reqwest::Client::new();

    let suffix = Uuid::now_v7().simple().to_string();
    let workspace_id = format!("ws-mt237-{suffix}");
    let document_id = format!("doc-mt237-{suffix}");
    let crdt_document_id = format!("crdt-mt237-{suffix}");
    let operator =
        KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "mt237-op").expect("operator");
    let model =
        KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "mt237-model").expect("model");
    let validator = KnowledgeActorIdV1::new(KnowledgeActorKind::Validator, "mt237-validator")
        .expect("validator");
    let operator_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &operator);
    let model_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &model);
    let validator_site = derive_knowledge_site_id(&workspace_id, &crdt_document_id, &validator);

    let empty = KnowledgeStateVectorV1::new();
    let mut operator_initial_after = empty.clone();
    operator_initial_after.increment(&operator_site.site_id);
    let operator_update = envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "mt237-operator-initial",
        &operator,
        "sr-mt237-op",
        "corr-mt237-operator-initial",
        b"MT237-INITIAL-OPERATOR-SENTINEL",
        &empty,
        &operator_initial_after,
    );

    let mut model_initial_after = empty.clone();
    model_initial_after.increment(&model_site.site_id);
    let model_update = envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "mt237-model-initial",
        &model,
        "sr-mt237-model",
        "corr-mt237-model-initial",
        b"MT237-INITIAL-MODEL-SENTINEL",
        &empty,
        &model_initial_after,
    );
    let mut validator_initial_after = empty.clone();
    validator_initial_after.increment(&validator_site.site_id);
    let validator_update = envelope(
        &workspace_id,
        &document_id,
        &crdt_document_id,
        "mt237-validator-initial",
        &validator,
        "sr-mt237-validator",
        "corr-mt237-validator-initial",
        b"MT237-INITIAL-VALIDATOR-SENTINEL",
        &empty,
        &validator_initial_after,
    );

    let attempts = vec![
        InitialWriteAttempt {
            envelope: operator_update,
            repair_update_id: "mt237-operator-rebased",
            repair_bytes: b"MT237-REPAIRED-OPERATOR-SENTINEL",
        },
        InitialWriteAttempt {
            envelope: model_update,
            repair_update_id: "mt237-model-rebased",
            repair_bytes: b"MT237-REPAIRED-MODEL-SENTINEL",
        },
        InitialWriteAttempt {
            envelope: validator_update,
            repair_update_id: "mt237-validator-rebased",
            repair_bytes: b"MT237-REPAIRED-VALIDATOR-SENTINEL",
        },
    ];

    let barrier = Arc::new(Barrier::new(attempts.len()));
    let mut tasks = Vec::new();
    for attempt in attempts {
        let client = client.clone();
        let base_url = base_url.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let (status, body) = push_update_http(&client, &base_url, &attempt.envelope).await;
            (attempt, status, body)
        }));
    }
    let mut results = Vec::new();
    for task in tasks {
        results.push(task.await.expect("initial same-base push task"));
    }
    let accepted_indices: Vec<usize> = results
        .iter()
        .enumerate()
        .filter_map(|(index, (_, status, _))| (*status == reqwest::StatusCode::OK).then_some(index))
        .collect();
    assert_eq!(
        accepted_indices.len(),
        1,
        "exactly one same-base writer wins the first CRDT sequence slot"
    );
    let accepted_attempt = results[accepted_indices[0]].0.clone();
    let accepted_after =
        KnowledgeStateVectorV1::parse(&accepted_attempt.envelope.state_vector_after)
            .expect("accepted initial state vector parses");
    let accepted_after_encoded = accepted_after.encode();
    assert_stored_push(
        &results[accepted_indices[0]].2,
        &accepted_attempt.envelope,
        1,
        &accepted_after_encoded,
    );
    let denied_results: Vec<_> = results
        .iter()
        .filter(|(_, status, _)| *status != reqwest::StatusCode::OK)
        .collect();
    assert_eq!(
        denied_results.len(),
        2,
        "the two non-winning same-base writers must be typed stale conflicts"
    );
    for (attempt, status, body) in &denied_results {
        assert_eq!(
            *status,
            reqwest::StatusCode::CONFLICT,
            "losing writer {} must receive HTTP 409",
            attempt.envelope.update_id
        );
        assert_stale_push_denied(body, &attempt.envelope, 1, &accepted_after_encoded);
    }
    let denied_update_ids: BTreeSet<String> = denied_results
        .iter()
        .map(|(attempt, _, _)| attempt.envelope.update_id.clone())
        .collect();
    let denied_actor_ids: BTreeSet<String> = denied_results
        .iter()
        .map(|(attempt, _, _)| attempt.envelope.actor_id.clone())
        .collect();
    let mut denied_attempts_sorted: Vec<InitialWriteAttempt> = denied_results
        .iter()
        .map(|(attempt, _, _)| (*attempt).clone())
        .collect();
    denied_attempts_sorted
        .sort_by(|left, right| left.envelope.update_id.cmp(&right.envelope.update_id));

    let empty_encoded = empty.encode();
    let receipts = list_denial_receipts_for_document(&pool, &crdt_document_id)
        .await
        .expect("list MT-237 denial receipts");
    assert_eq!(receipts.len(), 2, "each denied writer leaves a receipt");
    let receipt_actors: BTreeSet<String> = receipts
        .iter()
        .map(|receipt| receipt.actor_id.clone())
        .collect();
    assert_eq!(receipt_actors, denied_actor_ids);
    let receipt_denied_update_ids: BTreeSet<String> = receipts
        .iter()
        .map(|receipt| {
            receipt.denial_payload["denied_update_id"]
                .as_str()
                .expect("receipt denied_update_id")
                .to_string()
        })
        .collect();
    assert_eq!(receipt_denied_update_ids, denied_update_ids);
    for receipt in &receipts {
        let denied_attempt = denied_attempts_sorted
            .iter()
            .find(|attempt| {
                receipt.denial_payload["denied_update_id"].as_str()
                    == Some(attempt.envelope.update_id.as_str())
            })
            .expect("receipt maps to a denied HTTP push");
        assert_eq!(receipt.receipt_kind, "stale_draft_save");
        assert_eq!(receipt.workspace_id, workspace_id);
        assert_eq!(receipt.document_id.as_deref(), Some(document_id.as_str()));
        assert_eq!(
            receipt.crdt_document_id.as_deref(),
            Some(crdt_document_id.as_str())
        );
        assert_eq!(
            receipt.denial_payload["head_update_seq"].as_u64(),
            Some(1),
            "receipt records the head that blocked the stale write"
        );
        assert_eq!(
            receipt.denial_payload["denied_update_sha256"].as_str(),
            Some(denied_attempt.envelope.update_sha256.as_str())
        );
        assert_eq!(
            receipt.denial_payload["attempted_state_vector"].as_str(),
            Some(denied_attempt.envelope.state_vector_after.as_str())
        );
        assert!(!receipt.event_ledger_event_id.is_empty());
    }

    let head_after_denials =
        read_draft_head(db.as_ref(), &workspace_id, &document_id, &crdt_document_id)
            .await
            .expect("read head after denied writes");
    assert_eq!(head_after_denials.head_update_seq, 1);
    assert_eq!(head_after_denials.head_state_vector, accepted_after_encoded);
    let records_after_denials = db
        .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("list CRDT rows after denials");
    assert_eq!(records_after_denials.len(), 1);
    assert_eq!(
        records_after_denials[0].update_id,
        accepted_attempt.envelope.update_id
    );

    let validator_actor = validator.canonical();
    let conflict_response = client
        .get(format!("{base_url}/knowledge/crdt/conflict_state"))
        .query(&[
            ("workspace_id", workspace_id.as_str()),
            ("document_id", document_id.as_str()),
            ("crdt_document_id", crdt_document_id.as_str()),
            ("actor_id", validator_actor.as_str()),
            ("session_id", "sr-mt237-validator"),
            ("correlation_id", "corr-mt237-conflict-state"),
        ])
        .send()
        .await
        .expect("conflict_state request");
    assert_eq!(conflict_response.status(), reqwest::StatusCode::OK);
    let conflict_body: Value = conflict_response.json().await.expect("json");
    assert_eq!(conflict_body["receipt"]["operation"], "conflict_state");
    assert_eq!(conflict_body["result"]["head_update_seq"], 1);
    let conflicts = conflict_body["result"]["conflicts"]
        .as_array()
        .expect("conflicts array");
    assert_eq!(conflicts.len(), 2);
    let conflict_actors: BTreeSet<String> = conflicts
        .iter()
        .map(|entry| {
            entry["conflicting_actors"][0]["actor_id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(conflict_actors, receipt_actors);
    for entry in conflicts {
        assert_eq!(entry["kind"].as_str(), Some("stale_draft_save"));
        assert_eq!(
            entry["base"]["state_vector"].as_str(),
            Some(empty_encoded.as_str())
        );
        assert_eq!(
            entry["ours"]["state_vector"].as_str(),
            Some(accepted_after_encoded.as_str())
        );
        let options = entry["resolution_options"]
            .as_array()
            .expect("resolution options");
        assert!(options
            .iter()
            .any(|option| option["option"] == "pull_merge_resubmit"));
        assert!(options
            .iter()
            .any(|option| option["option"] == "adopt_server_head"));
    }

    let pulled_for_repair = pull_yjs_updates(
        db.as_ref(),
        &workspace_id,
        &document_id,
        &crdt_document_id,
        0,
        "hsk.doc.rich_document@1",
    )
    .await
    .expect("pull for repair");
    assert_eq!(pulled_for_repair.updates.len(), 1);
    assert_eq!(
        pulled_for_repair.updates[0].update_id,
        accepted_attempt.envelope.update_id
    );

    let mut current_head = accepted_after.clone();
    let mut expected_update_ids = BTreeSet::from([accepted_attempt.envelope.update_id.clone()]);
    let mut repair_envelopes = Vec::new();
    let mut next_update_seq = 2;
    for attempt in &denied_attempts_sorted {
        let actor = KnowledgeActorIdV1::parse(&attempt.envelope.actor_id)
            .expect("denied attempt actor parses");
        let attempted_after = KnowledgeStateVectorV1::parse(&attempt.envelope.state_vector_after)
            .expect("denied attempt state vector parses");
        let repaired_after = current_head.merge(&attempted_after);
        let repair_correlation_id = format!("corr-{}", attempt.repair_update_id);
        let repair = envelope(
            &workspace_id,
            &document_id,
            &crdt_document_id,
            attempt.repair_update_id,
            &actor,
            &attempt.envelope.session_id,
            &repair_correlation_id,
            attempt.repair_bytes,
            &current_head,
            &repaired_after,
        );
        let (repair_status, repair_body) = push_update_http(&client, &base_url, &repair).await;
        assert_eq!(
            repair_status,
            reqwest::StatusCode::OK,
            "repair write {} must store through HTTP",
            repair.update_id
        );
        assert_stored_push(
            &repair_body,
            &repair,
            next_update_seq,
            &repaired_after.encode(),
        );
        expected_update_ids.insert(repair.update_id.clone());
        repair_envelopes.push(repair);
        current_head = repaired_after;
        next_update_seq += 1;
    }

    let pulled_after_repair = pull_yjs_updates(
        db.as_ref(),
        &workspace_id,
        &document_id,
        &crdt_document_id,
        0,
        "hsk.doc.rich_document@1",
    )
    .await
    .expect("pull after repair");
    assert_eq!(pulled_after_repair.head_update_seq, 3);
    assert_eq!(pulled_after_repair.head_state_vector, current_head.encode());
    assert_eq!(
        ids_from_pull(&pulled_after_repair.updates),
        expected_update_ids
    );
    let pulled_update_texts: BTreeSet<String> = pulled_after_repair
        .updates
        .iter()
        .map(decoded_update_text)
        .collect();
    assert!(
        pulled_update_texts.contains(&decoded_update_text(&accepted_attempt.envelope)),
        "accepted first write bytes must replay"
    );
    for repair in &repair_envelopes {
        assert!(
            pulled_update_texts.contains(&decoded_update_text(repair)),
            "repair write {} bytes must replay",
            repair.update_id
        );
    }
    for denied_attempt in &denied_attempts_sorted {
        let denied_text = decoded_update_text(&denied_attempt.envelope);
        assert!(
            !pulled_update_texts.contains(&denied_text),
            "denied write {} bytes must not replay",
            denied_attempt.envelope.update_id
        );
    }

    let records_after_repair = db
        .list_kernel_crdt_updates(&workspace_id, &document_id, &crdt_document_id)
        .await
        .expect("list CRDT rows after repair");
    assert_eq!(records_after_repair.len(), 3);
    let mut stored_update_texts = BTreeSet::new();
    for record in &records_after_repair {
        let bytes = db
            .read_kernel_crdt_update_bytes(&record.update_bytes_ref)
            .await
            .expect("read persisted update bytes");
        stored_update_texts.insert(String::from_utf8(bytes).expect("stored sentinel bytes"));
    }
    assert_eq!(
        stored_update_texts, pulled_update_texts,
        "pull feed bytes must come from the same accepted PostgreSQL rows"
    );
    for denied_attempt in &denied_attempts_sorted {
        assert!(
            !stored_update_texts.contains(&decoded_update_text(&denied_attempt.envelope)),
            "denied write {} bytes must not be persisted",
            denied_attempt.envelope.update_id
        );
    }

    let events = db
        .list_kernel_events_for_aggregate("knowledge_crdt_document", &crdt_document_id)
        .await
        .expect("event ledger rows");
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == KernelEventType::KnowledgeCrdtUpdateRecorded)
            .count(),
        3,
        "only accepted operator/repaired writes create update events"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == KernelEventType::KnowledgeCrdtConflictDetected)
            .count(),
        2,
        "each denied write creates a conflict event"
    );
    for receipt in list_denial_receipts_for_document(&pool, &crdt_document_id)
        .await
        .expect("receipts after repair")
    {
        let event = events
            .iter()
            .find(|event| event.event_id == receipt.event_ledger_event_id)
            .expect("receipt must link to a real EventLedger conflict row");
        assert_eq!(
            event.event_type,
            KernelEventType::KnowledgeCrdtConflictDetected
        );
        assert_eq!(event.aggregate_type, "knowledge_crdt_document");
        assert_eq!(event.aggregate_id, crdt_document_id);
        assert_eq!(event.actor.actor_id(), receipt.actor_id.as_str());
        assert_eq!(
            event.correlation_id.as_deref(),
            Some(receipt.correlation_id.as_str())
        );
        assert_eq!(event.source_component, "knowledge_crdt_save_semantics");
        assert_eq!(
            event.payload["denied_update_id"].as_str(),
            receipt.denial_payload["denied_update_id"].as_str()
        );
        assert_eq!(
            event.payload["head_update_seq"].as_u64(),
            receipt.denial_payload["head_update_seq"].as_u64()
        );
        assert_eq!(
            event.payload["base_state_vector"].as_str(),
            Some(empty_encoded.as_str())
        );
        assert_eq!(
            event.payload["attempted_state_vector"].as_str(),
            receipt.denial_payload["attempted_state_vector"].as_str()
        );
        assert_eq!(
            event.payload["decision"],
            receipt.denial_payload["decision"]
        );
    }

    assert_eq!(
        pulled_after_repair.updates.len(),
        3,
        "fixture intentionally leaves only accepted updates in the replay feed"
    );
}
