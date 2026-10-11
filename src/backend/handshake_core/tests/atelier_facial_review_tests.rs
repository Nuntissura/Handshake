use handshake_core::atelier::facial::{
    generate_facial_ingest_analysis, FacialIngestAnalysisItem, GenerateFacialIngestAnalysisRequest,
};
use handshake_core::atelier::facial_native::models::{
    ARCFACE_ENV_KEY, FRAMING_CLOSEUP_ENV_KEY, FRAMING_THREEQUARTER_ENV_KEY,
    IDENTITY_COUNT_THRESHOLD_ENV_KEY, IDENTITY_MARGIN_ENV_KEY, IDENTITY_THRESHOLD_ENV_KEY,
    LANDMARK_ENV_KEY, YUNET_ENV_KEY,
};
use handshake_core::atelier::facial_native::review::{
    build_review_export_manifest, build_review_montage, build_review_session, build_review_status,
    claim_review_shard, record_review_decision, BuildFacialReviewExportRequest,
    BuildFacialReviewMontageRequest, BuildFacialReviewSessionRequest, FacialReviewClaimRequest,
    FacialReviewDecisionRequest, FACIAL_REVIEW_EXPORT_SCHEMA_ID, FACIAL_REVIEW_MONTAGE_SCHEMA_ID,
    FACIAL_REVIEW_SESSION_SCHEMA_ID, FACIAL_REVIEW_STATUS_SCHEMA_ID,
};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::Mutex;

static IDENTITY_ENV_LOCK: Mutex<()> = Mutex::new(());

fn item(
    item_id: &str,
    file_name: &str,
    content_hash: Option<&str>,
    byte_len: i64,
) -> FacialIngestAnalysisItem {
    FacialIngestAnalysisItem {
        item_id: item_id.to_owned(),
        source_ref: format!("dataset://facial-review/{file_name}"),
        local_path_hint: None,
        file_name: file_name.to_owned(),
        byte_len,
        content_hash: content_hash.map(ToOwned::to_owned),
        lane: "pending".to_owned(),
    }
}

fn analysis() -> handshake_core::atelier::facial::FacialIngestAnalysisExport {
    with_identity_env_cleared(|| {
        generate_facial_ingest_analysis(GenerateFacialIngestAnalysisRequest {
            batch_id: "018f7848-1111-7000-9000-000000000028".to_owned(),
            profile: "quality+dedupe+identity+review".to_owned(),
            requested_by: "facial-agent-028".to_owned(),
            items: vec![
                item("item-a", "a.jpg", Some("same-content-hash"), 3_000_000),
                item("item-b", "b.jpg", Some("same-content-hash"), 1_000_000),
                item("item-c", "c.jpg", None, 2_000_000),
                item("item-d", "d.jpg", Some("unique-content-hash"), 4_000_000),
            ],
        })
        .expect("facial analysis")
    })
}

fn session() -> handshake_core::atelier::facial_native::review::FacialReviewSessionArtifact {
    let export = analysis();
    build_review_session(BuildFacialReviewSessionRequest {
        batch_id: export.batch_id,
        analysis_sha256: export.analysis_sha256,
        receipt_sha256: export.receipt_sha256,
        created_by: "review-orchestrator".to_owned(),
        created_at_utc: "2026-07-01T10:00:00Z".to_owned(),
        shard_count: 2,
        claim_ttl_seconds: Some(60),
        rows: export.rows,
    })
    .expect("review session")
}

fn with_identity_env_cleared<T, F: FnOnce() -> T>(f: F) -> T {
    let _guard = IDENTITY_ENV_LOCK.lock().expect("identity env lock");
    let keys = [
        ARCFACE_ENV_KEY,
        YUNET_ENV_KEY,
        LANDMARK_ENV_KEY,
        IDENTITY_THRESHOLD_ENV_KEY,
        IDENTITY_MARGIN_ENV_KEY,
        IDENTITY_COUNT_THRESHOLD_ENV_KEY,
        FRAMING_CLOSEUP_ENV_KEY,
        FRAMING_THREEQUARTER_ENV_KEY,
    ];
    let saved = keys
        .iter()
        .map(|key| (*key, std::env::var(key).ok()))
        .collect::<Vec<_>>();
    for key in keys {
        std::env::remove_var(key);
    }
    let result = catch_unwind(AssertUnwindSafe(f));
    for key in keys {
        std::env::remove_var(key);
    }
    for (key, value) in saved {
        if let Some(value) = value {
            std::env::set_var(key, value);
        }
    }
    match result {
        Ok(value) => value,
        Err(payload) => resume_unwind(payload),
    }
}

#[test]
fn facial_review_session_uses_stable_non_positional_candidate_ids() {
    let session = session();

    assert_eq!(session.schema_id, FACIAL_REVIEW_SESSION_SCHEMA_ID);
    assert_eq!(session.item_count, 4);
    assert_eq!(session.shard_count, 2);
    assert_eq!(session.items[0].id_basis, "content_hash_plus_source_ref");
    assert!(
        session
            .items
            .iter()
            .any(|item| item.id_basis == "source_ref"),
        "missing hashes must fall back to source_ref-based IDs"
    );
    let unique_ids = session
        .items
        .iter()
        .map(|item| item.stable_image_id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        unique_ids.len(),
        session.items.len(),
        "duplicate content hashes must not collapse separate review candidates"
    );
    assert!(session.lineage_refs["analysis_sha256"].as_str().is_some());
    assert!(session.session_sha256.len() >= 16);
}

#[test]
fn facial_review_claims_are_disjoint_and_expired_claims_are_recoverable() {
    let session = session();
    let claim_a = claim_review_shard(
        &session,
        &[],
        &[],
        FacialReviewClaimRequest {
            actor: "agent-a".to_owned(),
            claimed_at_utc: "2026-07-01T10:00:00Z".to_owned(),
            shard: Some(0),
            claim_ttl_seconds: Some(60),
            steal_expired: false,
        },
    )
    .expect("claim a");
    let claim_b = claim_review_shard(
        &session,
        &[claim_a.clone()],
        &[],
        FacialReviewClaimRequest {
            actor: "agent-b".to_owned(),
            claimed_at_utc: "2026-07-01T10:00:10Z".to_owned(),
            shard: Some(1),
            claim_ttl_seconds: Some(60),
            steal_expired: false,
        },
    )
    .expect("claim b");

    assert_ne!(claim_a.shard, claim_b.shard);
    assert!(
        claim_a
            .stable_image_ids
            .iter()
            .all(|id| !claim_b.stable_image_ids.contains(id)),
        "parallel shard claims must not overlap"
    );
    let active_err = claim_review_shard(
        &session,
        &[claim_a.clone()],
        &[],
        FacialReviewClaimRequest {
            actor: "agent-c".to_owned(),
            claimed_at_utc: "2026-07-01T10:00:20Z".to_owned(),
            shard: Some(0),
            claim_ttl_seconds: Some(60),
            steal_expired: false,
        },
    )
    .expect_err("active claim should block overlap");
    assert!(active_err.contains("actively claimed"));

    let expired_err = claim_review_shard(
        &session,
        &[claim_a.clone()],
        &[],
        FacialReviewClaimRequest {
            actor: "agent-c".to_owned(),
            claimed_at_utc: "2026-07-01T10:02:00Z".to_owned(),
            shard: Some(0),
            claim_ttl_seconds: Some(60),
            steal_expired: false,
        },
    )
    .expect_err("expired claim recovery should be explicit");
    assert!(expired_err.contains("expired claim"));

    let recovered = claim_review_shard(
        &session,
        &[claim_a.clone()],
        &[],
        FacialReviewClaimRequest {
            actor: "agent-c".to_owned(),
            claimed_at_utc: "2026-07-01T10:02:00Z".to_owned(),
            shard: Some(0),
            claim_ttl_seconds: Some(60),
            steal_expired: true,
        },
    )
    .expect("expired claim recovered");
    assert_eq!(recovered.recovered_from_claim_id, Some(claim_a.claim_id));
}

#[test]
fn facial_review_decisions_preserve_entered_vocabulary_and_replay_status() {
    let session = session();
    let claim = claim_review_shard(
        &session,
        &[],
        &[],
        FacialReviewClaimRequest {
            actor: "agent-a".to_owned(),
            claimed_at_utc: "2026-07-01T10:00:00Z".to_owned(),
            shard: Some(0),
            claim_ttl_seconds: Some(600),
            steal_expired: false,
        },
    )
    .expect("claim");
    let pass_receipt = record_review_decision(
        &session,
        &claim,
        FacialReviewDecisionRequest {
            actor: "agent-a".to_owned(),
            decided_at_utc: "2026-07-01T10:01:00Z".to_owned(),
            item_id: claim.item_ids[0].clone(),
            claim_id: claim.claim_id.clone(),
            decision: "pass".to_owned(),
            reason: "sharp usable training face".to_owned(),
            tags: vec!["lora".to_owned(), "pass".to_owned()],
            notes: Some("source kept read-only".to_owned()),
        },
    )
    .expect("pass decision");
    let unsure_receipt = record_review_decision(
        &session,
        &claim,
        FacialReviewDecisionRequest {
            actor: "agent-a".to_owned(),
            decided_at_utc: "2026-07-01T10:02:00Z".to_owned(),
            item_id: claim.item_ids[1].clone(),
            claim_id: claim.claim_id.clone(),
            decision: "unsure".to_owned(),
            reason: "needs operator look".to_owned(),
            tags: vec!["needs review".to_owned()],
            notes: None,
        },
    )
    .expect("unsure decision");

    assert_eq!(pass_receipt.entered_decision, "pass");
    assert_eq!(pass_receipt.canonical_decision, "accept");
    assert_eq!(unsure_receipt.entered_decision, "unsure");
    assert_eq!(unsure_receipt.canonical_decision, "hold");

    let status = build_review_status(
        &session,
        &[claim],
        &[pass_receipt, unsure_receipt],
        "2026-07-01T10:03:00Z",
    )
    .expect("review status");
    assert_eq!(status.schema_id, FACIAL_REVIEW_STATUS_SCHEMA_ID);
    assert_eq!(status.decided_count, 2);
    assert_eq!(status.accepted_count, 1);
    assert_eq!(status.hold_count, 1);
    assert_eq!(status.undecided_count, 2);
    assert_eq!(status.per_actor_decisions["agent-a"], 2);
}

#[test]
fn facial_review_montage_is_tile_map_with_argus_selectors() {
    let session = session();
    let montage = build_review_montage(
        &session,
        &[],
        BuildFacialReviewMontageRequest {
            requested_by: "argus-agent".to_owned(),
            page: 0,
            columns: 2,
            rows: 2,
            tile_width: 128,
            tile_height: 96,
            decision_filter: Some("undecided".to_owned()),
        },
    )
    .expect("montage");

    assert_eq!(montage.schema_id, FACIAL_REVIEW_MONTAGE_SCHEMA_ID);
    assert_eq!(montage.artifact_kind, "tile_map_manifest");
    assert_eq!(montage.tile_count, 4);
    assert_eq!(montage.tiles[0].x, 0);
    assert_eq!(montage.tiles[1].x, 128);
    assert_eq!(montage.tiles[2].y, 96);
    assert!(montage
        .tiles
        .iter()
        .all(|tile| tile.argus_selector.starts_with("argus://facial-review/")));
    assert!(montage
        .tiles
        .iter()
        .all(|tile| tile.stable_image_id.starts_with("facial-review-image-")));
}

#[test]
fn facial_review_export_manifest_records_lineage_and_never_mutates_sources() {
    let session = session();
    let exportable_item = session
        .items
        .iter()
        .find(|item| item.duplicate_role != "duplicate")
        .expect("exportable item")
        .clone();
    let claim = claim_review_shard(
        &session,
        &[],
        &[],
        FacialReviewClaimRequest {
            actor: "agent-a".to_owned(),
            claimed_at_utc: "2026-07-01T10:00:00Z".to_owned(),
            shard: Some(exportable_item.shard),
            claim_ttl_seconds: Some(600),
            steal_expired: false,
        },
    )
    .expect("claim");
    let accept = record_review_decision(
        &session,
        &claim,
        FacialReviewDecisionRequest {
            actor: "agent-a".to_owned(),
            decided_at_utc: "2026-07-01T10:01:00Z".to_owned(),
            item_id: exportable_item.item_id.clone(),
            claim_id: claim.claim_id.clone(),
            decision: "pass".to_owned(),
            reason: "best candidate".to_owned(),
            tags: vec!["train".to_owned()],
            notes: None,
        },
    )
    .expect("decision");

    let blocked = build_review_export_manifest(
        &session,
        std::slice::from_ref(&accept),
        BuildFacialReviewExportRequest {
            requested_by: "export-agent".to_owned(),
            exported_at_utc: "2026-07-01T10:05:00Z".to_owned(),
            dataset_name: "leeseo_lora".to_owned(),
            repeats: 12,
            allow_partial: false,
            output_root_ref: "artifact://atelier/facial/review-export".to_owned(),
        },
    )
    .expect_err("undecided rows should block full export");
    assert!(blocked.contains("undecided"));

    let manifest = build_review_export_manifest(
        &session,
        &[accept],
        BuildFacialReviewExportRequest {
            requested_by: "export-agent".to_owned(),
            exported_at_utc: "2026-07-01T10:05:00Z".to_owned(),
            dataset_name: "leeseo_lora".to_owned(),
            repeats: 12,
            allow_partial: true,
            output_root_ref: "artifact://atelier/facial/review-export".to_owned(),
        },
    )
    .expect("partial export manifest");

    assert_eq!(manifest.schema_id, FACIAL_REVIEW_EXPORT_SCHEMA_ID);
    assert!(!manifest.source_mutation);
    assert_eq!(manifest.copy_mode, "manifest_only_no_source_mutation");
    assert_eq!(manifest.funnel["source_candidates"].as_u64(), Some(4));
    assert_eq!(manifest.funnel["exported"].as_u64(), Some(1));
    assert_eq!(
        manifest.lineage["source"]["analysis_sha256"].as_str(),
        Some(session.analysis_sha256.as_str())
    );
    assert_eq!(manifest.exported_files.len(), 1);
    assert!(manifest.exported_files[0]
        .output_ref
        .contains("12_leeseo_lora"));
    assert!(manifest
        .problems
        .iter()
        .any(|problem| problem.problem == "undecided_skipped"));
}
