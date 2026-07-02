//! WP-KERNEL-009 CRDTAndConcurrencyCore bridge + API tests.
//!
//! Modules map 1:1 to microtasks:
//!   - mt_067_yjs_bridge: MT-067 TiptapYjsBridgeContract
//!   - mt_075_conflict_ui: MT-075 ConflictUiStateModel
//!
//! The HTTP tests serve the REAL axum router over a loopback TcpListener and
//! drive it with reqwest — in-process Handshake backend, real PostgreSQL,
//! no external relay (the MT-078 posture this surface must keep).

use base64::Engine;
use handshake_core::api::knowledge_crdt::{router_with_state, KnowledgeCrdtApiState};
use handshake_core::kernel::crdt::actor_site::{
    derive_knowledge_site_id, KnowledgeActorIdV1, KnowledgeActorKind,
};
use handshake_core::kernel::crdt::persistence::sha256_hex;
use handshake_core::kernel::crdt::state_vector::KnowledgeStateVectorV1;
use handshake_core::kernel::crdt::yjs_bridge::{
    YjsUpdateEnvelopeV1, YJS_UPDATE_ENCODING_V1, YJS_UPDATE_ENVELOPE_SCHEMA_ID,
};
use handshake_core::storage::tests::{postgres_backend_with_pool_from_env, PostgresTestBackend};
use handshake_core::storage::StorageError;

async fn backend_or_blocked() -> PostgresTestBackend {
    match postgres_backend_with_pool_from_env().await {
        Ok(backend) => backend,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

/// Serve the knowledge CRDT router on a loopback port; returns the base url.
async fn serve_knowledge_crdt(backend: &PostgresTestBackend) -> String {
    let app = router_with_state(KnowledgeCrdtApiState {
        db: backend.database.clone(),
        pool: backend.postgres_pool.clone(),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve router");
    });
    format!("http://{addr}")
}

#[allow(clippy::too_many_arguments)]
fn envelope(
    workspace_id: &str,
    document_id: &str,
    crdt_document_id: &str,
    update_id: &str,
    actor: &KnowledgeActorIdV1,
    session_id: &str,
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
        trace_id: format!("trace-{update_id}"),
        document_schema_id: "hsk.doc.rich_document@1".to_string(),
        update_b64: base64::engine::general_purpose::STANDARD.encode(bytes),
        update_sha256: sha256_hex(bytes),
        state_vector_before: before.encode(),
        state_vector_after: after.encode(),
        encoding: YJS_UPDATE_ENCODING_V1.to_string(),
    }
}

mod mt_067_yjs_bridge {
    use super::*;
    use handshake_core::kernel::crdt::yjs_bridge::{
        envelope_to_update_record, validate_yjs_update_envelope, YjsEnvelopeValidationError,
    };
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn envelope_serde_round_trip_is_lossless() {
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-mt067").expect("actor");
        let mut before = KnowledgeStateVectorV1::new();
        let site = derive_knowledge_site_id("ws-067", "crdt-067", &actor);
        let mut after = before.clone();
        after.increment(&site.site_id);
        let env = envelope(
            "ws-067",
            "doc-067",
            "crdt-067",
            "u-067",
            &actor,
            "sr-067",
            b"yjs-update-bytes",
            &before,
            &after,
        );
        let json_form = serde_json::to_string(&env).expect("serialize");
        let back: YjsUpdateEnvelopeV1 = serde_json::from_str(&json_form).expect("deserialize");
        assert_eq!(back, env);

        // Validation accepts and exposes typed parts.
        let validated = validate_yjs_update_envelope(&env).expect("valid envelope");
        assert_eq!(validated.update_bytes, b"yjs-update-bytes");
        assert_eq!(validated.actor, actor);
        assert_eq!(validated.before, before);
        assert_eq!(validated.after, after);

        // Record conversion carries every attribution + causal field.
        let record = envelope_to_update_record(&env, &validated, 1, "evt-067");
        assert_eq!(record.update_seq, 1);
        assert_eq!(record.actor_id, actor.canonical());
        assert_eq!(record.state_vector_after, after.encode());
        assert_eq!(record.update_sha256, env.update_sha256);
        assert!(record
            .update_bytes_ref
            .starts_with("postgres://kernel_crdt_updates/"));
        before.increment(&site.site_id);
    }

    #[test]
    fn envelope_validation_fails_closed() {
        let actor =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-mt067").expect("actor");
        let site = derive_knowledge_site_id("ws-067", "crdt-067", &actor);
        let before = KnowledgeStateVectorV1::new();
        let mut after = before.clone();
        after.increment(&site.site_id);
        let good = envelope(
            "ws-067", "doc-067", "crdt-067", "u-067", &actor, "sr-067", b"bytes", &before, &after,
        );

        // Hash mismatch.
        let mut bad_hash = good.clone();
        bad_hash.update_sha256 = sha256_hex(b"other-bytes");
        assert!(validate_yjs_update_envelope(&bad_hash)
            .expect_err("hash mismatch fails")
            .iter()
            .any(|error| matches!(error, YjsEnvelopeValidationError::UpdateHashMismatch { .. })));

        // Non-base64 payload.
        let mut bad_b64 = good.clone();
        bad_b64.update_b64 = "!!!not-base64!!!".to_string();
        assert!(validate_yjs_update_envelope(&bad_b64)
            .expect_err("bad base64 fails")
            .iter()
            .any(|error| matches!(
                error,
                YjsEnvelopeValidationError::UpdateBytesNotBase64 { .. }
            )));

        // Site id must match the deterministic derivation.
        let mut wrong_site = good.clone();
        wrong_site.site_id = "site-0000000000000000".to_string();
        assert!(validate_yjs_update_envelope(&wrong_site)
            .expect_err("foreign site id fails")
            .iter()
            .any(|error| matches!(error, YjsEnvelopeValidationError::SiteIdMismatch { .. })));

        // after must strictly dominate before AND advance the own site.
        let mut no_advance = good.clone();
        no_advance.state_vector_after = no_advance.state_vector_before.clone();
        assert!(validate_yjs_update_envelope(&no_advance)
            .expect_err("non-advancing vector fails")
            .iter()
            .any(|error| matches!(
                error,
                YjsEnvelopeValidationError::AfterDoesNotDominateBefore { .. }
            )));
    }

    /// Real HTTP push/pull cycle against PostgreSQL: push two updates from
    /// two actors, pull them back byte-identical and ordered, replay a
    /// duplicate push idempotently, and receive typed 409 denials for stale
    /// bases. Navigation receipts carry actor/session/correlation
    /// (spec 2.3.13.11 backend-navigation MUST).
    #[tokio::test]
    async fn http_push_pull_round_trip_with_navigation_receipts() {
        let backend = backend_or_blocked().await;
        let base_url = serve_knowledge_crdt(&backend).await;
        let client = reqwest::Client::new();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt067-{suffix}");
        let doc = format!("doc-mt067-{suffix}");
        let crdt_doc = format!("crdt-mt067-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-http").expect("actor");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "lm-http").expect("actor");
        let op_site = derive_knowledge_site_id(&ws, &crdt_doc, &operator);
        let lm_site = derive_knowledge_site_id(&ws, &crdt_doc, &model);

        // Push update 1 (operator).
        let empty = KnowledgeStateVectorV1::new();
        let mut sv1 = empty.clone();
        sv1.increment(&op_site.site_id);
        let env1 = envelope(
            &ws,
            &doc,
            &crdt_doc,
            "http-u1",
            &operator,
            "sr-op",
            b"op-bytes-1",
            &empty,
            &sv1,
        );
        let response = client
            .post(format!("{base_url}/knowledge/crdt/updates/push"))
            .json(&json!({ "envelope": env1 }))
            .send()
            .await
            .expect("push request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["result"]["outcome"], "stored");
        assert_eq!(body["result"]["update_seq"], 1);
        assert_eq!(body["receipt"]["actor_id"], operator.canonical());
        assert_eq!(body["receipt"]["operation"], "push_update");
        assert_eq!(
            body["receipt"]["target_authority_ref"],
            format!("postgres://kernel_crdt_updates/{crdt_doc}")
        );

        // Push update 2 (model) on the new head.
        let mut sv2 = sv1.clone();
        sv2.increment(&lm_site.site_id);
        let env2 = envelope(
            &ws,
            &doc,
            &crdt_doc,
            "http-u2",
            &model,
            "sr-lm",
            b"lm-bytes-2",
            &sv1,
            &sv2,
        );
        let response = client
            .post(format!("{base_url}/knowledge/crdt/updates/push"))
            .json(&json!({ "envelope": env2 }))
            .send()
            .await
            .expect("push request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        // Duplicate push replays idempotently.
        let response = client
            .post(format!("{base_url}/knowledge/crdt/updates/push"))
            .json(&json!({ "envelope": env2 }))
            .send()
            .await
            .expect("push request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["result"]["outcome"], "already_stored");

        // Stale base push is a typed 409, never silent.
        let mut stale_after = empty.clone();
        stale_after.increment(&lm_site.site_id);
        let stale = envelope(
            &ws,
            &doc,
            &crdt_doc,
            "http-u3",
            &model,
            "sr-lm",
            b"stale-bytes",
            &empty,
            &stale_after,
        );
        let response = client
            .post(format!("{base_url}/knowledge/crdt/updates/push"))
            .json(&json!({ "envelope": stale }))
            .send()
            .await
            .expect("push request");
        assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["result"]["outcome"], "denied");
        assert_eq!(body["result"]["denial"]["reason"]["code"], "stale_base");
        assert_eq!(body["result"]["denial"]["reason"]["head_update_seq"], 2);

        // Pull everything back: ordered, byte-identical, with head info.
        let response = client
            .get(format!("{base_url}/knowledge/crdt/updates/pull"))
            .query(&[
                ("workspace_id", ws.as_str()),
                ("document_id", doc.as_str()),
                ("crdt_document_id", crdt_doc.as_str()),
                ("since_update_seq", "0"),
                ("document_schema_id", "hsk.doc.rich_document@1"),
                ("actor_id", "local_model:lm-http"),
                ("session_id", "sr-lm"),
                ("correlation_id", "corr-pull-1"),
            ])
            .send()
            .await
            .expect("pull request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.expect("json body");
        let updates = body["result"]["updates"].as_array().expect("updates array");
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0]["update_id"], "http-u1");
        assert_eq!(updates[1]["update_id"], "http-u2");
        assert_eq!(
            updates[0]["update_b64"],
            base64::engine::general_purpose::STANDARD.encode(b"op-bytes-1")
        );
        assert_eq!(updates[1]["actor_id"], model.canonical());
        assert_eq!(body["result"]["head_update_seq"], 2);
        assert_eq!(body["result"]["head_state_vector"], sv2.encode());
        assert_eq!(body["receipt"]["correlation_id"], "corr-pull-1");

        // Incremental pull from seq 1 returns only the tail.
        let response = client
            .get(format!("{base_url}/knowledge/crdt/updates/pull"))
            .query(&[
                ("workspace_id", ws.as_str()),
                ("document_id", doc.as_str()),
                ("crdt_document_id", crdt_doc.as_str()),
                ("since_update_seq", "1"),
                ("document_schema_id", "hsk.doc.rich_document@1"),
                ("actor_id", "local_model:lm-http"),
                ("session_id", "sr-lm"),
                ("correlation_id", "corr-pull-2"),
            ])
            .send()
            .await
            .expect("pull request");
        let body: serde_json::Value = response.json().await.expect("json body");
        let updates = body["result"]["updates"].as_array().expect("updates array");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0]["update_id"], "http-u2");

        // Backend-navigation ids are REQUIRED (spec 2.3.13.11): missing
        // actor/session/correlation is a 400, not a silent read.
        let response = client
            .get(format!("{base_url}/knowledge/crdt/updates/pull"))
            .query(&[
                ("workspace_id", ws.as_str()),
                ("document_id", doc.as_str()),
                ("crdt_document_id", crdt_doc.as_str()),
                ("document_schema_id", "hsk.doc.rich_document@1"),
                ("actor_id", ""),
                ("session_id", "sr-lm"),
                ("correlation_id", "corr-pull-3"),
            ])
            .send()
            .await
            .expect("pull request");
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    }
}

mod mt_075_conflict_ui {
    use super::*;
    use handshake_core::kernel::crdt::save_semantics::save_rich_document_draft;
    use uuid::Uuid;

    /// Conflict UI state over HTTP: a stale save leaves a durable receipt;
    /// the endpoint renders it as a typed conflict entry with base/ours/
    /// theirs revisions, the conflicting actor, and resolution options.
    #[tokio::test]
    async fn conflict_state_endpoint_serves_typed_conflicts_from_receipts() {
        let backend = backend_or_blocked().await;
        let db = backend.database.clone();
        let pool = backend.postgres_pool.clone();
        let base_url = serve_knowledge_crdt(&backend).await;
        let client = reqwest::Client::new();
        let suffix = Uuid::now_v7().simple().to_string();
        let ws = format!("ws-mt075-{suffix}");
        let doc = format!("doc-mt075-{suffix}");
        let crdt_doc = format!("crdt-mt075-{suffix}");
        let operator =
            KnowledgeActorIdV1::new(KnowledgeActorKind::Operator, "op-ui").expect("actor");
        let model =
            KnowledgeActorIdV1::new(KnowledgeActorKind::LocalModel, "lm-ui").expect("actor");
        let op_site = derive_knowledge_site_id(&ws, &crdt_doc, &operator);
        let lm_site = derive_knowledge_site_id(&ws, &crdt_doc, &model);

        // Operator lands head; model collides from the same base.
        let empty = KnowledgeStateVectorV1::new();
        let mut head = empty.clone();
        head.increment(&op_site.site_id);
        save_rich_document_draft(
            db.as_ref(),
            &pool,
            &envelope(
                &ws, &doc, &crdt_doc, "ui-u1", &operator, "sr-op", b"op-ui-1", &empty, &head,
            ),
        )
        .await
        .expect("save flow");
        let mut model_after = empty.clone();
        model_after.increment(&lm_site.site_id);
        save_rich_document_draft(
            db.as_ref(),
            &pool,
            &envelope(
                &ws,
                &doc,
                &crdt_doc,
                "ui-u2",
                &model,
                "sr-lm",
                b"lm-ui-1",
                &empty,
                &model_after,
            ),
        )
        .await
        .expect("save flow");

        // The endpoint renders the durable conflict.
        let response = client
            .get(format!("{base_url}/knowledge/crdt/conflict_state"))
            .query(&[
                ("workspace_id", ws.as_str()),
                ("document_id", doc.as_str()),
                ("crdt_document_id", crdt_doc.as_str()),
                ("actor_id", "operator:op-ui"),
                ("session_id", "sr-op"),
                ("correlation_id", "corr-ui-1"),
            ])
            .send()
            .await
            .expect("conflict_state request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.expect("json body");
        let result = &body["result"];
        assert_eq!(
            result["schema_id"],
            "hsk.kernel.knowledge_conflict_ui_state@1"
        );
        assert_eq!(result["head_update_seq"], 1);
        assert_eq!(result["head_state_vector"], head.encode());

        let conflicts = result["conflicts"].as_array().expect("conflicts array");
        assert_eq!(conflicts.len(), 1);
        let conflict = &conflicts[0];
        assert_eq!(conflict["kind"], "stale_draft_save");
        assert_eq!(
            conflict["conflicting_actors"][0]["actor_id"],
            model.canonical()
        );
        assert_eq!(conflict["base"]["state_vector"], empty.encode());
        assert_eq!(conflict["ours"]["state_vector"], head.encode());
        assert_eq!(conflict["theirs"]["state_vector"], model_after.encode());
        assert_eq!(conflict["theirs"]["update_id"], "ui-u2");
        let options = conflict["resolution_options"]
            .as_array()
            .expect("options array");
        assert!(options
            .iter()
            .any(|option| option["option"] == "pull_merge_resubmit"));
        assert!(options
            .iter()
            .any(|option| option["option"] == "adopt_server_head"));
        assert!(!conflict["denial_receipt_id"].as_str().unwrap().is_empty());
        assert!(!conflict["event_ledger_event_id"]
            .as_str()
            .unwrap()
            .is_empty());

        // Navigation receipt echoes the caller identification.
        assert_eq!(body["receipt"]["operation"], "conflict_state");
        assert_eq!(body["receipt"]["actor_id"], "operator:op-ui");

        // A clean document reports an empty conflict list (not an error).
        let response = client
            .get(format!("{base_url}/knowledge/crdt/conflict_state"))
            .query(&[
                ("workspace_id", ws.as_str()),
                ("document_id", doc.as_str()),
                ("crdt_document_id", "crdt-clean-doc"),
                ("actor_id", "operator:op-ui"),
                ("session_id", "sr-op"),
                ("correlation_id", "corr-ui-2"),
            ])
            .send()
            .await
            .expect("conflict_state request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["result"]["conflicts"].as_array().unwrap().len(), 0);
    }
}
