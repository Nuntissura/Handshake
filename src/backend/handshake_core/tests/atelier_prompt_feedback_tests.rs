//! WP-CKC-posekit-overhaul MT-020 prompt-feedback kernel proof.
//!
//! Managed-PostgreSQL proof for the deterministic prompt-feedback kernel:
//! Leeseo i76 import -> PromptCases persist with all dimensions; operator/model/
//! subagent verdicts persist + emit EventLedger events; a fixed input + rule pack
//! yields a byte-stable rewrite with a populated rule trace; a JSONL export
//! becomes a hashed ArtifactStore artifact carrying the source case ids +
//! rule-pack id; and a `standard`-segment case rejects a prompt-stress mutation
//! (both at the engine level and as an identity-verdict rejection).
//!
//! Setup mirrors the passing WP-CKC backend tests (`atelier_settings_preference_tests`,
//! `atelier_pose_tests`): connect via `atelier_pg_support::database_url()` +
//! `AtelierStore::connect` + `ensure_schema()` (raw `include_str!`, no checksum
//! `_sqlx_migrations` state), so it is robust on the shared managed Postgres. It
//! does NOT call `PostgresDatabase::run_migrations()`. Emitted EventLedger rows are
//! read by SQL against `atelier_event` (as the settings test reads `payload`).
//!
//! Pure-engine unit tests (each of the 5 seed rules + a byte-identical rewrite
//! determinism test) live in `src/atelier/prompt_feedback/engine.rs` and run with
//! no database.

mod atelier_pg_support;

use atelier_pg_support::{database_url, test_artifact_workspace_root};
use handshake_core::atelier::prompt_feedback::adapter::{import_leeseo, CuippRow, LeeseoImportRequest};
use handshake_core::atelier::prompt_feedback::engine::{
    ActionKind, RULE_PROTECTED_EVAL, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION,
};
use handshake_core::atelier::prompt_feedback::model::{
    NewReviewVerdict, ReviewerKind, VerdictKind,
};
use handshake_core::atelier::prompt_feedback::PromptCaseFilter;
use handshake_core::atelier::AtelierStore;
use handshake_core::storage::artifacts::{read_file_artifact, ArtifactLayer};
use uuid::Uuid;

/// Connect + ensure schema against the shared managed Postgres. No checksum
/// migration state (`run_migrations`) is touched, so a concurrent cross-WP
/// session cannot break setup.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

/// Count the atelier EventLedger rows for one event family + aggregate id.
async fn event_count(store: &AtelierStore, event_family: &str, aggregate_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"SELECT count(*) FROM atelier_event
           WHERE event_family = $1 AND aggregate_id = $2"#,
    )
    .bind(event_family)
    .bind(aggregate_id)
    .fetch_one(store.pool())
    .await
    .expect("count atelier_event rows")
}

/// A small but real slice of the Leeseo i76 suite: one protected `standard`
/// no-detail close-up (with a leaked prompt-stress tail) and one prompt-stress
/// FaceDetailer+FaceID close-up.
///
/// The `case_id`s embed the run-unique `adapter_id` so the deterministic export
/// content-hash is fresh per run: on the shared, persistent managed Postgres, a
/// prior run's export row (same content-hash) would otherwise be reused (F6) but
/// its artifact lives in a now-deleted per-process tempdir.
fn i76_fixture_request(adapter_id: &str) -> LeeseoImportRequest {
    LeeseoImportRequest {
        project_id: "leeseo".to_string(),
        source_system: "leeseo".to_string(),
        adapter_id: adapter_id.to_string(),
        source_iteration_id: Some("i76".to_string()),
        imported_by: "seed".to_string(),
        rows: vec![
            CuippRow {
                case_id: format!("no_detail:0_closeup:1:{adapter_id}"),
                segment: Some("standard".to_string()),
                cell: Some("0_closeup".to_string()),
                render_key: Some("no_detail".to_string()),
                positive_prompt: Some("face-readable close-up, naked shoulders".to_string()),
                // A prompt-stress wardrobe tail leaking into a standard row.
                positive_tail: Some("open blouse no bra".to_string()),
                expected_failure: Some("vacant_face".to_string()),
                image_name: Some("closeup 01.png".to_string()),
                ..Default::default()
            },
            CuippRow {
                case_id: format!("with_detail_faceid:0_closeup:1:{adapter_id}"),
                segment: Some("prompt_stress".to_string()),
                cell: Some("0_closeup".to_string()),
                render_key: Some("FaceDetailer+FaceID".to_string()),
                // Claims an oral contact level but the positive prompt carries no
                // body-contact proof, so the contact rule must fire.
                contact_level: Some("oral".to_string()),
                body_target_terms: Some("face_closeup".to_string()),
                positive_prompt: Some("pov close-up looking into the lens".to_string()),
                expected_failure: Some("vacant_face".to_string()),
                ..Default::default()
            },
        ],
    }
}

#[tokio::test]
async fn i76_import_persists_prompt_cases_with_all_dimensions() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP i76_import_persists_prompt_cases_with_all_dimensions: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let adapter_id = format!("leeseo.cuipp.v1-{}", Uuid::new_v4());

    let request = i76_fixture_request(&adapter_id);
    let new_cases = import_leeseo(&request).expect("normalize i76 rows");
    let imported = store
        .import_prompt_cases(&new_cases)
        .await
        .expect("persist prompt cases");
    assert_eq!(imported.len(), 2);

    let standard = imported
        .iter()
        .find(|case| case.segment == "standard")
        .expect("standard case persisted");
    assert_eq!(standard.cell, "0_closeup");
    assert_eq!(standard.framing, "close-up");
    assert_eq!(standard.render_stack, "no_detail");
    assert!(standard.identity_judgement_allowed);
    assert_eq!(
        standard.image_artifact_ref.as_deref(),
        Some("dataset://leeseo/i76/closeup-01.png")
    );
    assert_eq!(
        standard.axes.prompt_stress_positive_tail.as_deref(),
        Some("open blouse no bra")
    );

    let stress = imported
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case persisted");
    assert_eq!(stress.render_stack, "FaceDetailer+FaceID");
    // Core invariant: a prompt-stress case is never identity evidence.
    assert!(!stress.identity_judgement_allowed);

    // A CASE_IMPORTED EventLedger event is emitted per case.
    assert!(
        event_count(
            &store,
            "atelier.prompt_feedback.case_imported",
            &standard.case_id.to_string()
        )
        .await
            >= 1,
        "import must emit a case_imported EventLedger row"
    );

    // Filtered listing groups by segment.
    let stress_only = store
        .list_prompt_cases(&PromptCaseFilter {
            segment: Some("prompt_stress".to_string()),
            ..Default::default()
        })
        .await
        .expect("list prompt-stress cases");
    assert!(stress_only.iter().all(|case| case.segment == "prompt_stress"));
}

#[tokio::test]
async fn verdicts_persist_for_operator_model_subagent_and_emit_events() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP verdicts_persist_for_operator_model_subagent_and_emit_events: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    let adapter_id = format!("leeseo.cuipp.v1-{}", Uuid::new_v4());
    let new_cases = import_leeseo(&i76_fixture_request(&adapter_id)).expect("normalize");
    let imported = store.import_prompt_cases(&new_cases).await.expect("import");
    let stress = imported
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case");

    for reviewer in [
        ReviewerKind::Operator,
        ReviewerKind::Model,
        ReviewerKind::Subagent,
    ] {
        let verdict = store
            .record_prompt_verdict(&NewReviewVerdict {
                case_id: stress.case_id,
                reviewer_kind: reviewer,
                reviewer_id: format!("{}-1", reviewer.as_token()),
                verdict_kind: VerdictKind::Failure,
                failure_class: Some("bland".to_string()),
                failure_tags: vec!["generic_nude".to_string()],
                is_identity_judgement: false,
                note: Some("prompt-quality readiness failure".to_string()),
            })
            .await
            .expect("record verdict");
        assert_eq!(
            event_count(
                &store,
                "atelier.prompt_feedback.verdict_recorded",
                &verdict.verdict_id.to_string()
            )
            .await,
            1,
            "each verdict must emit exactly one verdict_recorded EventLedger row"
        );
    }

    let verdicts = store
        .list_prompt_verdicts(stress.case_id)
        .await
        .expect("list verdicts");
    assert_eq!(verdicts.len(), 3);
}

#[tokio::test]
async fn deterministic_rewrite_is_byte_stable_with_populated_trace() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP deterministic_rewrite_is_byte_stable_with_populated_trace: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = format!("leeseo.cuipp.v1-{}", Uuid::new_v4());
    let new_cases = import_leeseo(&i76_fixture_request(&adapter_id)).expect("normalize");
    let imported = store.import_prompt_cases(&new_cases).await.expect("import");
    let stress = imported
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case");

    let first = store
        .plan_prompt_rewrite(stress.case_id, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION, "seed")
        .await
        .expect("plan rewrite");
    let second = store
        .plan_prompt_rewrite(stress.case_id, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION, "seed")
        .await
        .expect("re-plan rewrite");
    // Idempotent + byte-stable.
    assert_eq!(first.rewrite_id, second.rewrite_id);
    assert_eq!(first.output_hash, second.output_hash);
    assert_eq!(first.input_hash, second.input_hash);
    // The prompt-stress close-up claims oral contact without proof, so the
    // contact rule must fire with a populated trace.
    assert!(!first.outcome.trace.is_empty());
    assert!(first
        .outcome
        .trace
        .iter()
        .all(|entry| entry.rule_pack_id == SEED_RULE_PACK_ID && !entry.input_hash.is_empty()));
}

#[tokio::test]
async fn new_feedback_produces_a_distinct_rewrite_row() {
    // F3: a re-plan after new verdicts changes the output (contact rule flips to
    // a workflow-routing hint once contact-proof failure recurs) and must be a
    // DISTINCT rewrite row, not a silent overwrite.
    let Some(url) = database_url().await else {
        eprintln!("SKIP new_feedback_produces_a_distinct_rewrite_row: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = format!("leeseo.cuipp.v1-{}", Uuid::new_v4());
    let new_cases = import_leeseo(&i76_fixture_request(&adapter_id)).expect("normalize");
    let imported = store.import_prompt_cases(&new_cases).await.expect("import");
    let stress = imported
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case");

    let before = store
        .plan_prompt_rewrite(stress.case_id, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION, "seed")
        .await
        .expect("plan before feedback");

    // Two recurring contact-proof failures flip the recurring signal.
    for i in 0..2 {
        store
            .record_prompt_verdict(&NewReviewVerdict {
                case_id: stress.case_id,
                reviewer_kind: ReviewerKind::Model,
                reviewer_id: format!("contact-reviewer-{i}"),
                verdict_kind: VerdictKind::Failure,
                failure_class: Some("incoherence".to_string()),
                failure_tags: vec!["action_claim_without_contact_proof".to_string()],
                is_identity_judgement: false,
                note: None,
            })
            .await
            .expect("record contact verdict");
    }

    let after = store
        .plan_prompt_rewrite(stress.case_id, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION, "seed")
        .await
        .expect("plan after feedback");

    assert_ne!(
        before.rewrite_id, after.rewrite_id,
        "a different feedback state must be a distinct rewrite row (F3)"
    );
    assert_ne!(before.input_hash, after.input_hash);
    // The pure prompt-content hash is unchanged (feedback-independent provenance).
    assert_eq!(before.outcome.input_hash, after.outcome.input_hash);
}

#[tokio::test]
async fn unimplemented_rule_pack_is_rejected() {
    // F2: even if a pack row is registered, a rewrite against a non-seed pack is
    // rejected so a persisted trace can never misattribute a pack the engine did
    // not run.
    let Some(url) = database_url().await else {
        eprintln!("SKIP unimplemented_rule_pack_is_rejected: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let custom_pack = format!("custom.pack-{}", Uuid::new_v4());
    store
        .register_rule_pack(&custom_pack, 1, "Custom pack", None, &[], "seed")
        .await
        .expect("register custom pack");
    let adapter_id = format!("leeseo.cuipp.v1-{}", Uuid::new_v4());
    let new_cases = import_leeseo(&i76_fixture_request(&adapter_id)).expect("normalize");
    let imported = store.import_prompt_cases(&new_cases).await.expect("import");
    let case_id = imported[0].case_id;

    let err = store
        .plan_prompt_rewrite(case_id, &custom_pack, 1, "seed")
        .await
        .expect_err("a non-seed rule pack must be rejected");
    assert!(
        err.to_string().contains("not implemented"),
        "rejection must name the unimplemented pack: {err}"
    );
}

#[tokio::test]
async fn jsonl_export_is_a_hashed_artifact_store_artifact() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP jsonl_export_is_a_hashed_artifact_store_artifact: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = format!("leeseo.cuipp.v1-{}", Uuid::new_v4());
    let new_cases = import_leeseo(&i76_fixture_request(&adapter_id)).expect("normalize");
    let imported = store.import_prompt_cases(&new_cases).await.expect("import");
    let case_ids: Vec<Uuid> = imported.iter().map(|case| case.case_id).collect();
    let source_case_ids: Vec<String> =
        imported.iter().map(|case| case.source_case_id.clone()).collect();

    let workspace_root = test_artifact_workspace_root();
    let export = store
        .materialize_prompt_export(
            SEED_RULE_PACK_ID,
            SEED_RULE_PACK_VERSION,
            &case_ids,
            "seed",
            &workspace_root,
        )
        .await
        .expect("materialize export");

    assert!(export.artifact_ref.starts_with("artifact://"));
    assert!(export.artifact_ref.ends_with("/payload"));
    assert!(!export.content_hash.is_empty());
    assert!(export.byte_len > 0);
    assert_eq!(export.row_count as usize, imported.len());
    assert_eq!(export.rule_pack_id, SEED_RULE_PACK_ID);
    for source in &source_case_ids {
        assert!(export.source_case_ids.contains(source));
    }

    // The bytes are a real ArtifactStore artifact and match the content hash.
    let artifact_id = artifact_id_from_ref(&export.artifact_ref);
    let bytes = read_file_artifact(&workspace_root, ArtifactLayer::L1, artifact_id)
        .expect("read export artifact bytes");
    let jsonl = String::from_utf8(bytes).expect("utf-8 jsonl");
    assert!(jsonl.contains(SEED_RULE_PACK_ID));
    assert!(jsonl.contains("rule_trace"));
    assert!(jsonl.contains("original_prompt_hash"));
    for source in &source_case_ids {
        assert!(jsonl.contains(source));
    }

    // F6: re-exporting identical bytes reuses the same artifact, never orphaning
    // a new blob or repointing the row.
    let reexport = store
        .materialize_prompt_export(
            SEED_RULE_PACK_ID,
            SEED_RULE_PACK_VERSION,
            &case_ids,
            "seed",
            &workspace_root,
        )
        .await
        .expect("re-materialize export");
    assert_eq!(reexport.export_id, export.export_id);
    assert_eq!(reexport.artifact_ref, export.artifact_ref);
}

#[tokio::test]
async fn standard_case_rejects_prompt_stress_mutation_and_identity_verdict() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP standard_case_rejects_prompt_stress_mutation_and_identity_verdict: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = format!("leeseo.cuipp.v1-{}", Uuid::new_v4());
    let new_cases = import_leeseo(&i76_fixture_request(&adapter_id)).expect("normalize");
    let imported = store.import_prompt_cases(&new_cases).await.expect("import");
    let standard = imported
        .iter()
        .find(|case| case.segment == "standard")
        .expect("standard case");
    let stress = imported
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case");

    // Import-override closed: the prompt-stress case is not identity-eligible.
    assert!(!stress.identity_judgement_allowed);

    // Engine level: the leaked prompt-stress tail is hard-rejected and stripped.
    let plan = store
        .plan_prompt_rewrite(standard.case_id, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION, "seed")
        .await
        .expect("plan standard rewrite");
    let protected = plan
        .outcome
        .trace
        .iter()
        .find(|entry| entry.rule_id == RULE_PROTECTED_EVAL)
        .expect("protected eval rule must fire on a standard row with a leaked tail");
    assert_eq!(protected.action_kind, ActionKind::HardReject);
    assert!(plan.outcome.rewritten.prompt_stress_positive_tail.is_none());

    // Verdict level: an identity judgement on a prompt-stress case is rejected.
    let err = store
        .record_prompt_verdict(&NewReviewVerdict {
            case_id: stress.case_id,
            reviewer_kind: ReviewerKind::Model,
            reviewer_id: "model-identity".to_string(),
            verdict_kind: VerdictKind::Success,
            failure_class: None,
            failure_tags: Vec::new(),
            is_identity_judgement: true,
            note: None,
        })
        .await
        .expect_err("prompt-stress identity judgement must be rejected");
    assert!(
        err.to_string().contains("identity"),
        "rejection must name the identity constraint: {err}"
    );
}

/// Extract the artifact UUID from an `artifact://.../<uuid>/payload` ref.
fn artifact_id_from_ref(artifact_ref: &str) -> Uuid {
    let trimmed = artifact_ref
        .strip_suffix("/payload")
        .expect("artifact ref ends with /payload");
    let uuid_segment = trimmed
        .rsplit('/')
        .next()
        .expect("artifact ref has a uuid segment");
    Uuid::parse_str(uuid_segment).expect("artifact ref carries a uuid")
}
