//! WP-CKC-posekit-overhaul MT-020 prompt-feedback kernel proof (SurrealDB port).
//!
//! Embedded-SurrealDB proof for the deterministic prompt-feedback kernel: Leeseo
//! i76 import -> PromptCases persist with all dimensions; operator/model/subagent
//! verdicts persist + emit EventLedger events; a fixed input + rule pack yields a
//! byte-stable rewrite with a populated rule trace; a JSONL export becomes a
//! hashed ArtifactStore artifact carrying the source case ids + rule-pack id; a
//! `standard`-segment case rejects a prompt-stress mutation (both at the engine
//! level and as an identity-verdict rejection); and the HTTP lane router round-
//! trips import/cases/verdicts/rewrite/export/rulepacks.
//!
//! Every test opens its own isolated `AtelierSurrealHarness` (a fresh on-disk
//! embedded store with the canonical schema bootstrapped), so nothing is shared
//! across tests and nothing is skipped. Emitted events are read back through the
//! store's own `count_events_for_aggregate` and the kernel EventLedger
//! (`Database::list_kernel_events_for_aggregate`), which is where the atelier
//! projection payload lives.
//!
//! Pure-engine unit tests (each of the 5 seed rules + a byte-identical rewrite
//! determinism test) live in `src/atelier/prompt_feedback/engine.rs` and run with
//! no database.

mod atelier_surreal_support;

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use atelier_surreal_support::AtelierSurrealHarness;
use handshake_core::api::atelier_ckc_prompt_feedback as prompt_feedback_api;
use handshake_core::atelier::prompt_feedback::adapter::{
    import_leeseo, import_prompt_stress_csv_manifest, CuippRow, LeeseoImportRequest,
    PromptStressCsvImportRequest,
};
use handshake_core::atelier::prompt_feedback::engine::{
    ActionKind, RULE_PROTECTED_EVAL, SEED_RULE_PACK_ID, SEED_RULE_PACK_VERSION,
};
use handshake_core::atelier::prompt_feedback::model::{
    NewReviewVerdict, ReviewerKind, VerdictKind,
};
use handshake_core::atelier::prompt_feedback::PromptCaseFilter;
use handshake_core::atelier::AtelierStore;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::artifacts::{read_file_artifact, ArtifactLayer};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use uuid::Uuid;

const CASE_AGGREGATE: &str = "atelier_prompt_feedback_case";
const VERDICT_AGGREGATE: &str = "atelier_prompt_feedback_verdict";
const CASE_IMPORTED: &str = "atelier.prompt_feedback.case_imported";
const VERDICT_RECORDED: &str = "atelier.prompt_feedback.verdict_recorded";

/// Count the atelier EventLedger rows for one event family + aggregate.
async fn event_count(
    store: &AtelierStore,
    event_family: &str,
    aggregate_type: &str,
    aggregate_id: &str,
) -> i64 {
    store
        .count_events_for_aggregate(event_family, aggregate_type, aggregate_id)
        .await
        .expect("count atelier_event rows")
}

/// The atelier projection payload of the latest kernel EventLedger row for one
/// event family + aggregate (the kernel row wraps it under `atelier_payload`).
async fn latest_event_payload(
    harness: &AtelierSurrealHarness,
    event_family: &str,
    aggregate_type: &str,
    aggregate_id: &str,
) -> serde_json::Value {
    let events = harness
        .database
        .list_kernel_events_for_aggregate(aggregate_type, aggregate_id)
        .await
        .expect("list kernel events for aggregate");
    events
        .into_iter()
        .filter(|event| event.payload["event_family"] == event_family)
        .max_by_key(|event| event.event_sequence)
        .and_then(|event| event.payload.get("atelier_payload").cloned())
        .expect("latest atelier event payload for aggregate")
}

/// A small but real slice of the Leeseo i76 suite: one protected `standard`
/// no-detail close-up (with a leaked prompt-stress tail) and one prompt-stress
/// FaceDetailer+FaceID close-up.
///
/// The `case_id`s embed the run-unique `adapter_id` so the deterministic export
/// content-hash is fresh per run even if two tests ever shared a store.
///
/// `include_image_name` controls whether the standard row carries the reference
/// fixture's `image_name` (which the adapter turns into a portable `dataset://`
/// ref). The reference PostgreSQL schema accepted `dataset://` image refs; the
/// current SurrealDB `atelier_prompt_feedback_case.image_artifact_ref` ASSERT
/// only accepts `artifact://`, so the tests whose subject is NOT the image ref
/// leave it out and the two tests that assert the persisted ref keep it (they
/// document the schema gap until the ASSERT is widened).
fn i76_fixture_request_with_options(
    adapter_id: &str,
    include_image_name: bool,
) -> LeeseoImportRequest {
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
                image_name: include_image_name.then(|| "closeup 01.png".to_string()),
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

/// The reference fixture, verbatim (standard row carries `image_name`).
fn i76_fixture_request(adapter_id: &str) -> LeeseoImportRequest {
    i76_fixture_request_with_options(adapter_id, true)
}

/// The reference fixture minus the standard row's `image_name`.
fn i76_fixture_request_without_image_ref(adapter_id: &str) -> LeeseoImportRequest {
    i76_fixture_request_with_options(adapter_id, false)
}

fn prompt_stress_csv_request(adapter_id: &str, csv: &str) -> PromptStressCsvImportRequest {
    PromptStressCsvImportRequest {
        project_id: "leeseo".to_string(),
        source_system: "leeseo".to_string(),
        adapter_id: adapter_id.to_string(),
        source_iteration_id: Some("i76".to_string()),
        source_manifest_ref: Some("dataset://leeseo/i76/prompt_stress_manifest.csv".to_string()),
        imported_by: "seed".to_string(),
        csv: csv.to_string(),
    }
}

fn unique_adapter_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::now_v7())
}

#[tokio::test]
async fn prompt_stress_csv_manifest_imports_cases() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let adapter_id = unique_adapter_id("leeseo.prompt-stress.csv.v1");
    let csv = concat!(
        "\u{feff}case_id,cell,render_stack,positive_prompt,negative_prompt,image_name,contact_level,scene,notes\r\n",
        "with_detail_faceid:0_closeup:csv-1,0_closeup,FaceDetailer+FaceID,\"pov close-up, looking into the lens\",low quality,closeup 01.png,oral,\"wet room\",\"quoted, note\"\r\n",
        "no_detail:full:csv-2,full,no_detail,\"full body prompt\",bad hands,full.png,,studio,plain note\r\n",
    );

    let new_cases =
        import_prompt_stress_csv_manifest(&prompt_stress_csv_request(&adapter_id, csv))
            .expect("parse prompt-stress csv");
    assert_eq!(new_cases.len(), 2);
    assert_eq!(new_cases[0].source_case_id, "with_detail_faceid:0_closeup:csv-1");
    assert_eq!(new_cases[0].segment, "prompt_stress");
    assert!(!new_cases[0].identity_judgement_allowed);
    assert!(new_cases[0].prompt_quality_review_allowed);
    assert_eq!(new_cases[0].render_stack, "FaceDetailer+FaceID");
    assert_eq!(
        new_cases[0].image_artifact_ref.as_deref(),
        Some("dataset://leeseo/i76/closeup-01.png")
    );
    assert_eq!(
        new_cases[0]
            .hardcore_fields
            .pointer("/csv/source_format")
            .and_then(|value| value.as_str()),
        Some("prompt_stress_manifest.csv")
    );
    assert_eq!(
        new_cases[0]
            .hardcore_fields
            .pointer("/csv/row_number")
            .and_then(|value| value.as_u64()),
        Some(2)
    );
    assert_eq!(
        new_cases[0]
            .hardcore_fields
            .pointer("/csv/source_manifest_ref")
            .and_then(|value| value.as_str()),
        Some("dataset://leeseo/i76/prompt_stress_manifest.csv")
    );
    assert!(new_cases[0]
        .hardcore_fields
        .pointer("/csv/row_hash")
        .and_then(|value| value.as_str())
        .is_some_and(|hash| hash.starts_with("sha256:")));
    assert_eq!(
        new_cases[0]
            .hardcore_fields
            .pointer("/csv/unmapped/notes")
            .and_then(|value| value.as_str()),
        Some("quoted, note")
    );

    let imported = store
        .import_prompt_cases(&new_cases)
        .await
        .expect("persist csv prompt cases");
    assert_eq!(imported.len(), 2);
    assert!(imported.iter().all(|case| case.segment == "prompt_stress"));
    assert!(imported.iter().all(|case| !case.identity_judgement_allowed));

    let first = imported
        .iter()
        .find(|case| case.source_case_id == "with_detail_faceid:0_closeup:csv-1")
        .expect("persisted first csv case");
    let persisted_row_hash = first
        .hardcore_fields
        .pointer("/csv/row_hash")
        .and_then(|value| value.as_str())
        .expect("persisted csv row hash");
    assert!(persisted_row_hash.starts_with("sha256:"));
    assert_eq!(
        first
            .hardcore_fields
            .pointer("/csv/source_manifest_ref")
            .and_then(|value| value.as_str()),
        Some("dataset://leeseo/i76/prompt_stress_manifest.csv")
    );
    assert!(
        event_count(
            store,
            CASE_IMPORTED,
            CASE_AGGREGATE,
            &first.case_id.to_string()
        )
        .await
            >= 1,
        "CSV import must feed the normal prompt case persistence/EventLedger path"
    );
    let event_payload = latest_event_payload(
        &harness,
        CASE_IMPORTED,
        CASE_AGGREGATE,
        &first.case_id.to_string(),
    )
    .await;
    assert_eq!(
        event_payload
            .pointer("/csv/source_manifest_ref")
            .and_then(|value| value.as_str()),
        Some("dataset://leeseo/i76/prompt_stress_manifest.csv")
    );
    assert_eq!(
        event_payload
            .pointer("/csv/row_hash")
            .and_then(|value| value.as_str()),
        Some(persisted_row_hash)
    );

    let reordered_csv = concat!(
        "case_id,cell,render_stack,positive_prompt,negative_prompt,image_name\r\n",
        "no_detail:full:csv-2,full,no_detail,\"full body prompt\",bad hands,full.png\r\n",
        "with_detail_faceid:0_closeup:csv-1,0_closeup,FaceDetailer+FaceID,\"pov close-up, looking into the lens\",low quality,closeup 01.png\r\n",
    );
    let reordered =
        import_prompt_stress_csv_manifest(&prompt_stress_csv_request(&adapter_id, reordered_csv))
            .expect("parse reordered prompt-stress csv");
    let mut original_ids: Vec<String> =
        new_cases.iter().map(|case| case.source_case_id.clone()).collect();
    let mut reordered_ids: Vec<String> =
        reordered.iter().map(|case| case.source_case_id.clone()).collect();
    original_ids.sort();
    reordered_ids.sort();
    assert_eq!(
        original_ids, reordered_ids,
        "CSV source_case_id stability must not depend on row order"
    );
    harness.shutdown().await;
}

#[test]
fn prompt_stress_csv_manifest_rejects_malformed_rows() {
    let adapter_id = "leeseo.prompt-stress.csv.v1-malformed";
    let missing_case_id = concat!(
        "case_id,cell,render_stack,positive_prompt\r\n",
        "valid-case,0_closeup,no_detail,valid prompt\r\n",
        ",0_closeup,no_detail,missing case id\r\n",
    );
    let err =
        import_prompt_stress_csv_manifest(&prompt_stress_csv_request(adapter_id, missing_case_id))
            .expect_err("blank case_id must reject the whole csv");
    let err = err.to_string();
    assert!(err.contains("row 3"), "{err}");
    assert!(err.contains("case_id"), "{err}");

    let duplicate_header =
        "case_id,case_id,cell,render_stack,positive_prompt\r\none,two,0_closeup,no_detail,prompt\r\n";
    let err =
        import_prompt_stress_csv_manifest(&prompt_stress_csv_request(adapter_id, duplicate_header))
            .expect_err("duplicate header must reject the csv")
            .to_string();
    assert!(err.contains("duplicate header"), "{err}");
    assert!(err.contains("case_id"), "{err}");

    let non_portable_ref =
        "case_id,cell,render_stack,positive_prompt,image_ref\r\nportable,0_closeup,no_detail,prompt,D:\\bad\\raw.png\r\n";
    let err =
        import_prompt_stress_csv_manifest(&prompt_stress_csv_request(adapter_id, non_portable_ref))
            .expect_err("raw machine paths must reject csv image_ref")
            .to_string();
    assert!(err.contains("row 2"), "{err}");
    assert!(err.contains("image_ref"), "{err}");
    assert!(err.contains("portable"), "{err}");

    let prefixed_machine_path =
        "case_id,cell,render_stack,positive_prompt,image_ref\r\nportable,0_closeup,no_detail,prompt,artifact://D:\\bad\\raw.png\r\n";
    let err = import_prompt_stress_csv_manifest(&prompt_stress_csv_request(
        adapter_id,
        prefixed_machine_path,
    ))
    .expect_err("machine-local path hidden in an artifact ref must reject")
    .to_string();
    assert!(err.contains("row 2"), "{err}");
    assert!(err.contains("image_ref"), "{err}");
    assert!(err.contains("machine-local"), "{err}");

    let mut raw_manifest_ref_request = prompt_stress_csv_request(
        adapter_id,
        "case_id,cell,render_stack,positive_prompt\r\nraw-manifest,0_closeup,no_detail,prompt\r\n",
    );
    raw_manifest_ref_request.source_manifest_ref =
        Some("D:\\bad\\prompt_stress_manifest.csv".to_string());
    let err = import_prompt_stress_csv_manifest(&raw_manifest_ref_request)
        .expect_err("raw machine path source_manifest_ref must reject before persistence")
        .to_string();
    assert!(err.contains("source_manifest_ref"), "{err}");
    assert!(err.contains("portable"), "{err}");
    assert!(err.contains("machine-local"), "{err}");

    let trailing_quote_garbage =
        "case_id,cell,render_stack,positive_prompt\r\nbad-quote,0_closeup,no_detail,\"prompt\"garbage\r\n";
    let err = import_prompt_stress_csv_manifest(&prompt_stress_csv_request(
        adapter_id,
        trailing_quote_garbage,
    ))
    .expect_err("characters after a quoted field must reject")
    .to_string();
    assert!(err.contains("row 2"), "{err}");
    assert!(err.contains("column 4"), "{err}");
    assert!(err.contains("after quoted field"), "{err}");

    let conflicting_segment =
        "case_id,segment,cell,render_stack,positive_prompt\r\nnot-stress,standard,0_closeup,no_detail,prompt\r\n";
    let err = import_prompt_stress_csv_manifest(&prompt_stress_csv_request(
        adapter_id,
        conflicting_segment,
    ))
    .expect_err("non prompt-stress segment must reject csv")
    .to_string();
    assert!(err.contains("row 2"), "{err}");
    assert!(err.contains("segment"), "{err}");
    assert!(err.contains("prompt_stress"), "{err}");
}

#[test]
fn cuipp_json_import_still_maps_cases() {
    let request = i76_fixture_request("leeseo.cuipp.v1-json-regression");
    let cases = import_leeseo(&request).expect("normalize CUIPP json rows");
    assert_eq!(cases.len(), 2);
    let standard = cases
        .iter()
        .find(|case| case.segment == "standard")
        .expect("standard case");
    assert_eq!(
        standard.axes.prompt_stress_positive_tail.as_deref(),
        Some("open blouse no bra")
    );
    assert_eq!(
        standard.image_artifact_ref.as_deref(),
        Some("dataset://leeseo/i76/closeup-01.png")
    );
    let stress = cases
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case");
    assert!(!stress.identity_judgement_allowed);
    assert!(stress.prompt_quality_review_allowed);
    assert_eq!(stress.render_stack, "FaceDetailer+FaceID");
}

#[tokio::test]
async fn i76_import_persists_prompt_cases_with_all_dimensions() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");

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
            store,
            CASE_IMPORTED,
            CASE_AGGREGATE,
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
    assert_eq!(stress_only.len(), 1);
    assert!(stress_only.iter().all(|case| case.segment == "prompt_stress"));
    harness.shutdown().await;
}

#[tokio::test]
async fn reimport_updates_case_in_place_and_keeps_case_id() {
    // Idempotency on (adapter_id, source_case_id): a re-import of the same source
    // case updates the row, keeps its case_id and created_at_utc, and emits one
    // more case_imported event against the same aggregate.
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");
    let mut request = i76_fixture_request_without_image_ref(&adapter_id);
    let first = store
        .import_prompt_cases(&import_leeseo(&request).expect("normalize"))
        .await
        .expect("first import");
    let stress_before = first
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case")
        .clone();

    request.rows[1].positive_prompt =
        Some("pov close-up looking into the lens, wet lips parted".to_string());
    request.rows[1].micro_gate = Some("lips_readable".to_string());
    let second = store
        .import_prompt_cases(&import_leeseo(&request).expect("normalize again"))
        .await
        .expect("second import");
    let stress_after = second
        .iter()
        .find(|case| case.segment == "prompt_stress")
        .expect("prompt-stress case after re-import");

    assert_eq!(stress_after.case_id, stress_before.case_id);
    assert_eq!(stress_after.created_at_utc, stress_before.created_at_utc);
    assert_eq!(
        stress_after.positive_prompt,
        "pov close-up looking into the lens, wet lips parted"
    );
    assert_eq!(stress_after.micro_gate.as_deref(), Some("lips_readable"));
    assert_eq!(
        store
            .list_prompt_cases(&PromptCaseFilter {
                project_id: Some("leeseo".to_string()),
                ..Default::default()
            })
            .await
            .expect("list cases")
            .len(),
        2,
        "re-import must not create a second row for the same source case"
    );
    assert_eq!(
        event_count(
            store,
            CASE_IMPORTED,
            CASE_AGGREGATE,
            &stress_before.case_id.to_string()
        )
        .await,
        2,
        "each import (create and update) emits one case_imported event"
    );
    let fetched = store
        .get_prompt_case(stress_before.case_id)
        .await
        .expect("get updated case");
    assert_eq!(fetched.positive_prompt, stress_after.positive_prompt);
    harness.shutdown().await;
}

#[tokio::test]
async fn verdicts_persist_for_operator_model_subagent_and_emit_events() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");
    let new_cases =
        import_leeseo(&i76_fixture_request_without_image_ref(&adapter_id)).expect("normalize");
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
        assert_eq!(verdict.case_id, stress.case_id);
        assert_eq!(verdict.reviewer_kind, reviewer);
        assert_eq!(verdict.failure_tags, vec!["generic_nude".to_string()]);
        assert_eq!(
            event_count(
                store,
                VERDICT_RECORDED,
                VERDICT_AGGREGATE,
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
    harness.shutdown().await;
}

#[tokio::test]
async fn deterministic_rewrite_is_byte_stable_with_populated_trace() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");
    let new_cases =
        import_leeseo(&i76_fixture_request_without_image_ref(&adapter_id)).expect("normalize");
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
    assert_eq!(first.case_id, stress.case_id);
    assert_eq!(first.rule_pack_id, SEED_RULE_PACK_ID);
    assert_eq!(first.rule_pack_version, SEED_RULE_PACK_VERSION);
    // The prompt-stress close-up claims oral contact without proof, so the
    // contact rule must fire with a populated trace.
    assert!(!first.outcome.trace.is_empty());
    assert!(first
        .outcome
        .trace
        .iter()
        .all(|entry| entry.rule_pack_id == SEED_RULE_PACK_ID && !entry.input_hash.is_empty()));
    harness.shutdown().await;
}

#[tokio::test]
async fn new_feedback_produces_a_distinct_rewrite_row() {
    // F3: a re-plan after new verdicts changes the output (contact rule flips to
    // a workflow-routing hint once contact-proof failure recurs) and must be a
    // DISTINCT rewrite row, not a silent overwrite.
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");
    let new_cases =
        import_leeseo(&i76_fixture_request_without_image_ref(&adapter_id)).expect("normalize");
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
    let routed = after
        .outcome
        .trace
        .iter()
        .find(|entry| entry.rule_id == "contact_claim_without_contact_proof")
        .expect("contact rule fires after recurring feedback");
    assert_eq!(routed.action_kind, ActionKind::WorkflowRoutingHint);
    harness.shutdown().await;
}

#[tokio::test]
async fn unimplemented_rule_pack_is_rejected() {
    // F2: even if a pack row is registered, a rewrite against a non-seed pack is
    // rejected so a persisted trace can never misattribute a pack the engine did
    // not run.
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let custom_pack = format!("custom.pack-{}", Uuid::now_v7());
    let registered = store
        .register_rule_pack(&custom_pack, 1, "Custom pack", None, &[], "seed")
        .await
        .expect("register custom pack");
    assert_eq!(registered.rule_pack_id, custom_pack);
    assert_eq!(registered.version, 1);
    assert!(registered.rules.is_empty());
    let packs = store.list_rule_packs().await.expect("list rule packs");
    assert!(packs.iter().any(|pack| pack.rule_pack_id == custom_pack));
    assert!(packs
        .iter()
        .any(|pack| pack.rule_pack_id == SEED_RULE_PACK_ID && pack.rules.len() == 5));

    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");
    let new_cases =
        import_leeseo(&i76_fixture_request_without_image_ref(&adapter_id)).expect("normalize");
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
    harness.shutdown().await;
}

#[tokio::test]
async fn jsonl_export_is_a_hashed_artifact_store_artifact() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");
    let new_cases =
        import_leeseo(&i76_fixture_request_without_image_ref(&adapter_id)).expect("normalize");
    let imported = store.import_prompt_cases(&new_cases).await.expect("import");
    let case_ids: Vec<Uuid> = imported.iter().map(|case| case.case_id).collect();
    let source_case_ids: Vec<String> =
        imported.iter().map(|case| case.source_case_id.clone()).collect();

    let workspace = tempfile::tempdir().expect("isolated export workspace root");
    let workspace_root = workspace.path();
    let export = store
        .materialize_prompt_export(
            SEED_RULE_PACK_ID,
            SEED_RULE_PACK_VERSION,
            &case_ids,
            "seed",
            workspace_root,
        )
        .await
        .expect("materialize export");

    assert!(export.artifact_ref.starts_with("artifact://"));
    assert!(export.artifact_ref.ends_with("/payload"));
    assert!(export
        .manifest_ref
        .as_deref()
        .is_some_and(|value| value.ends_with("/artifact.json")));
    assert!(!export.content_hash.is_empty());
    assert!(export.byte_len > 0);
    assert_eq!(export.row_count as usize, imported.len());
    assert_eq!(export.rule_pack_id, SEED_RULE_PACK_ID);
    assert_eq!(export.rule_pack_version, SEED_RULE_PACK_VERSION);
    assert_eq!(export.rewrite_ids.len(), imported.len());
    for source in &source_case_ids {
        assert!(export.source_case_ids.contains(source));
    }

    // The bytes are a real ArtifactStore artifact and match the content hash.
    let artifact_id = artifact_id_from_ref(&export.artifact_ref);
    let bytes = read_file_artifact(workspace_root, ArtifactLayer::L1, artifact_id)
        .expect("read export artifact bytes");
    assert_eq!(bytes.len() as i64, export.byte_len);
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
            workspace_root,
        )
        .await
        .expect("re-materialize export");
    assert_eq!(reexport.export_id, export.export_id);
    assert_eq!(reexport.artifact_ref, export.artifact_ref);
    assert_eq!(
        event_count(
            store,
            "atelier.prompt_feedback.export_materialized",
            "atelier_prompt_feedback_export",
            &export.export_id.to_string()
        )
        .await,
        1,
        "one export row, one export_materialized event"
    );
    harness.shutdown().await;
}

#[tokio::test]
async fn standard_case_rejects_prompt_stress_mutation_and_identity_verdict() {
    let harness = AtelierSurrealHarness::create().await;
    let store = &harness.atelier;
    store.ensure_seed_rule_pack("seed").await.expect("seed rule pack");
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1");
    let new_cases =
        import_leeseo(&i76_fixture_request_without_image_ref(&adapter_id)).expect("normalize");
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
    assert!(
        store
            .list_prompt_verdicts(stress.case_id)
            .await
            .expect("list verdicts")
            .is_empty(),
        "a rejected identity verdict must not be persisted"
    );
    harness.shutdown().await;
}

// --- HTTP lane router ---------------------------------------------------------

/// One workspace root for the whole test binary. `HANDSHAKE_WORKSPACE_ROOT` is
/// process-global and tests run on parallel threads, so the HTTP export test
/// writes its artifact (distinct UUID) into this single root instead of racing on
/// the env var. Store-level export tests pass their own explicit roots.
fn shared_workspace_root() -> &'static PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let root = tempfile::tempdir()
            .expect("create isolated prompt-feedback workspace root")
            .keep();
        std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", &root);
        root
    })
}

#[derive(Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn app_state(harness: &AtelierSurrealHarness) -> AppState {
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage: harness.database.clone(),
        surreal: harness.storage.clone(),
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("mt020-prompt-feedback-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

async fn serve(state: AppState) -> (String, reqwest::Client, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        axum::serve(listener, prompt_feedback_api::routes(state))
            .await
            .expect("prompt-feedback lane API server");
    });
    (format!("http://{addr}"), reqwest::Client::new(), server)
}

const ACTOR_HEADER: &str = "x-hsk-actor-id";

#[tokio::test]
async fn prompt_feedback_api_round_trips_import_verdict_rewrite_export_and_rulepacks() {
    let harness = AtelierSurrealHarness::create().await;
    let workspace_root: &Path = shared_workspace_root();
    let (base, client, server) = serve(app_state(&harness)).await;
    let adapter_id = unique_adapter_id("leeseo.cuipp.v1-http");
    let fixture = i76_fixture_request_without_image_ref(&adapter_id);

    // Missing actor header is a 400 before any write.
    let unauthenticated = client
        .post(format!("{base}/atelier/prompt-feedback/import"))
        .json(&serde_json::json!({
            "project_id": fixture.project_id,
            "source_system": fixture.source_system,
            "adapter_id": fixture.adapter_id,
            "rows": [],
        }))
        .send()
        .await
        .expect("send import without actor");
    assert_eq!(unauthenticated.status().as_u16(), 400);

    // Import.
    let response = client
        .post(format!("{base}/atelier/prompt-feedback/import"))
        .header(ACTOR_HEADER, "http-operator")
        .json(&serde_json::json!({
            "project_id": fixture.project_id,
            "source_system": fixture.source_system,
            "adapter_id": fixture.adapter_id,
            "source_iteration_id": fixture.source_iteration_id,
            "rows": fixture.rows,
        }))
        .send()
        .await
        .expect("send import");
    assert_eq!(response.status().as_u16(), 201, "import must be 201 Created");
    let imported: serde_json::Value = response.json().await.expect("import json");
    assert_eq!(imported["imported_count"], 2);
    assert_eq!(imported["seed_rule_pack"]["rule_pack_id"], SEED_RULE_PACK_ID);
    assert_eq!(imported["seed_rule_pack"]["version"], SEED_RULE_PACK_VERSION);
    let cases = imported["cases"].as_array().expect("cases array");
    assert!(cases
        .iter()
        .all(|case| case["imported_by"] == "http-operator"));
    let stress = cases
        .iter()
        .find(|case| case["segment"] == "prompt_stress")
        .expect("prompt-stress case in import response");
    let standard = cases
        .iter()
        .find(|case| case["segment"] == "standard")
        .expect("standard case in import response");
    let stress_id: Uuid =
        serde_json::from_value(stress["case_id"].clone()).expect("stress case_id uuid");
    let standard_id: Uuid =
        serde_json::from_value(standard["case_id"].clone()).expect("standard case_id uuid");
    assert_eq!(stress["identity_judgement_allowed"], false);

    // Cases, filtered by segment.
    let listed: Vec<serde_json::Value> = client
        .get(format!(
            "{base}/atelier/prompt-feedback/cases?project_id=leeseo&segment=prompt_stress"
        ))
        .send()
        .await
        .expect("send list cases")
        .json()
        .await
        .expect("cases json");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["case_id"], stress["case_id"]);

    // Verdict: identity judgement on the prompt-stress case is a 400.
    let rejected = client
        .post(format!("{base}/atelier/prompt-feedback/verdicts"))
        .header(ACTOR_HEADER, "http-model")
        .json(&serde_json::json!({
            "case_id": stress_id,
            "reviewer_kind": "model",
            "verdict_kind": "success",
            "is_identity_judgement": true,
        }))
        .send()
        .await
        .expect("send rejected verdict");
    assert_eq!(rejected.status().as_u16(), 400);
    let rejected_body: serde_json::Value = rejected.json().await.expect("error json");
    assert_eq!(rejected_body["error"], "bad_request");

    // Verdict: a prompt-quality failure is recorded, reviewer_id defaults to the actor.
    let verdict = client
        .post(format!("{base}/atelier/prompt-feedback/verdicts"))
        .header(ACTOR_HEADER, "http-model")
        .json(&serde_json::json!({
            "case_id": stress_id,
            "reviewer_kind": "model",
            "verdict_kind": "failure",
            "failure_class": "incoherence",
            "failure_tags": ["action_claim_without_contact_proof"],
            "note": "no contact proof",
        }))
        .send()
        .await
        .expect("send verdict");
    assert_eq!(verdict.status().as_u16(), 201);
    let verdict: serde_json::Value = verdict.json().await.expect("verdict json");
    assert_eq!(verdict["reviewer_id"], "http-model");
    assert_eq!(verdict["reviewer_kind"], "model");
    assert_eq!(verdict["verdict_kind"], "failure");
    assert_eq!(verdict["case_id"], stress["case_id"]);

    // Unknown case is a 404.
    let missing = client
        .post(format!("{base}/atelier/prompt-feedback/verdicts"))
        .header(ACTOR_HEADER, "http-model")
        .json(&serde_json::json!({
            "case_id": Uuid::now_v7(),
            "reviewer_kind": "model",
            "verdict_kind": "failure",
        }))
        .send()
        .await
        .expect("send verdict for missing case");
    assert_eq!(missing.status().as_u16(), 404);

    // Rewrite preview (default rule pack version = seed v1) on the standard row
    // hard-rejects the leaked prompt-stress tail.
    let rewrite = client
        .post(format!("{base}/atelier/prompt-feedback/rewrite"))
        .header(ACTOR_HEADER, "http-operator")
        .json(&serde_json::json!({
            "case_id": standard_id,
            "rule_pack_id": SEED_RULE_PACK_ID,
        }))
        .send()
        .await
        .expect("send rewrite");
    assert_eq!(rewrite.status().as_u16(), 200);
    let rewrite: serde_json::Value = rewrite.json().await.expect("rewrite json");
    assert_eq!(rewrite["case_id"], standard["case_id"]);
    assert_eq!(rewrite["rule_pack_id"], SEED_RULE_PACK_ID);
    assert_eq!(rewrite["planned_by"], "http-operator");
    let trace = rewrite["outcome"]["trace"].as_array().expect("trace array");
    assert!(trace
        .iter()
        .any(|entry| entry["rule_id"] == RULE_PROTECTED_EVAL && entry["action_kind"] == "hard_reject"));
    assert!(rewrite["outcome"]["rewritten"]["prompt_stress_positive_tail"].is_null());

    // Rewrite with a blank rule pack id is a 400; a non-seed pack is a 400 (F2).
    let blank = client
        .post(format!("{base}/atelier/prompt-feedback/rewrite"))
        .header(ACTOR_HEADER, "http-operator")
        .json(&serde_json::json!({ "case_id": standard_id, "rule_pack_id": "  " }))
        .send()
        .await
        .expect("send blank rewrite");
    assert_eq!(blank.status().as_u16(), 400);
    let other_pack = client
        .post(format!("{base}/atelier/prompt-feedback/rewrite"))
        .header(ACTOR_HEADER, "http-operator")
        .json(&serde_json::json!({ "case_id": standard_id, "rule_pack_id": "custom.pack", "rule_pack_version": 3 }))
        .send()
        .await
        .expect("send non-seed rewrite");
    assert_eq!(other_pack.status().as_u16(), 400);

    // Rule packs list the seed pack with its five rules.
    let packs: Vec<serde_json::Value> = client
        .get(format!("{base}/atelier/prompt-feedback/rulepacks"))
        .send()
        .await
        .expect("send rulepacks")
        .json()
        .await
        .expect("rulepacks json");
    let seed = packs
        .iter()
        .find(|pack| pack["rule_pack_id"] == SEED_RULE_PACK_ID)
        .expect("seed pack listed");
    assert_eq!(seed["rules"].as_array().map(Vec::len), Some(5));
    assert!(seed["content_hash"]
        .as_str()
        .is_some_and(|hash| hash.starts_with("sha256:")));

    // Export: a hashed ArtifactStore artifact under HANDSHAKE_WORKSPACE_ROOT.
    let export = client
        .post(format!("{base}/atelier/prompt-feedback/export"))
        .header(ACTOR_HEADER, "http-operator")
        .json(&serde_json::json!({
            "rule_pack_id": SEED_RULE_PACK_ID,
            "case_ids": [standard_id, stress_id],
        }))
        .send()
        .await
        .expect("send export");
    assert_eq!(export.status().as_u16(), 201, "export must be 201 Created");
    let export: serde_json::Value = export.json().await.expect("export json");
    assert_eq!(export["row_count"], 2);
    assert_eq!(export["exported_by"], "http-operator");
    let artifact_ref = export["artifact_ref"].as_str().expect("artifact_ref");
    let artifact_id = artifact_id_from_ref(artifact_ref);
    let bytes = read_file_artifact(workspace_root, ArtifactLayer::L1, artifact_id)
        .expect("export artifact bytes exist under the shared workspace root");
    assert_eq!(bytes.len() as u64, export["byte_len"].as_u64().expect("byte_len"));
    let jsonl = String::from_utf8(bytes).expect("utf-8 jsonl");
    assert_eq!(jsonl.lines().count(), 2);
    assert!(jsonl.contains(&format!("no_detail:0_closeup:1:{adapter_id}")));

    // F6 over HTTP: an identical export returns the same receipt.
    let again: serde_json::Value = client
        .post(format!("{base}/atelier/prompt-feedback/export"))
        .header(ACTOR_HEADER, "http-operator")
        .json(&serde_json::json!({
            "rule_pack_id": SEED_RULE_PACK_ID,
            "case_ids": [stress_id, standard_id],
        }))
        .send()
        .await
        .expect("send export again")
        .json()
        .await
        .expect("export again json");
    assert_eq!(again["export_id"], export["export_id"]);
    assert_eq!(again["artifact_ref"], export["artifact_ref"]);

    // Export with no case ids is a 400.
    let empty = client
        .post(format!("{base}/atelier/prompt-feedback/export"))
        .header(ACTOR_HEADER, "http-operator")
        .json(&serde_json::json!({ "rule_pack_id": SEED_RULE_PACK_ID, "case_ids": [] }))
        .send()
        .await
        .expect("send empty export");
    assert_eq!(empty.status().as_u16(), 400);

    server.abort();
    harness.shutdown().await;
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
