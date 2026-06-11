//! WP-KERNEL-005 MT-001/MT-002 source-evidence and anchor-verification proof.
//!
//! This test records a machine-readable product/runtime matrix in live
//! PostgreSQL and proves the EventLedger projection. It is not a governance note.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::AtelierStore;
use handshake_core::atelier::source_evidence::{
    AnchorVerificationStatus, CORE_DATA_SOURCE_EVIDENCE_MATRIX_ID, NewAnchorVerificationRecord,
    POSE_COMFY_SOURCE_EVIDENCE_MATRIX_ID, POSE_MEDIA_ANCHOR_VERIFICATION_MATRIX_ID,
    SourceMaturityStatus, core_data_source_evidence_matrix, pose_comfy_source_evidence_matrix,
    pose_media_anchor_verification_matrix, source_evidence_event_family,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{Database, postgres::PostgresDatabase};
use sqlx::postgres::PgPoolOptions;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

async fn connected_store_with_ledger(url: &str) -> (AtelierStore, Arc<dyn Database>) {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool.clone());
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    let database = database.into_arc();
    let store = AtelierStore::with_event_ledger(pool, database.clone());
    store.ensure_schema().await.expect("ensure atelier schema");
    (store, database)
}

fn source_tree_ref_path(ref_path: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    match ref_path.strip_prefix("src/backend/handshake_core/") {
        Some(relative_to_crate) => manifest_dir.join(relative_to_crate),
        None => PathBuf::from(ref_path),
    }
}

fn assert_source_tree_ref_exists(ref_path: &str) {
    let path = source_tree_ref_path(ref_path);
    assert!(
        path.exists(),
        "source evidence ref must exist in the product source tree: {ref_path}"
    );
}

#[tokio::test]
async fn source_evidence_matrix_records_maturity_and_anchor_verification() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP source_evidence_matrix_records_maturity_and_anchor_verification: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let mut matrix = core_data_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    matrix.matrix_id = format!("{CORE_DATA_SOURCE_EVIDENCE_MATRIX_ID}-{}", Uuid::new_v4());
    let recorded = store
        .record_source_evidence_matrix(&matrix)
        .await
        .expect("record source evidence matrix");

    assert_eq!(recorded.matrix_id, matrix.matrix_id);
    assert_eq!(
        recorded.sources.len(),
        matrix.sources.len(),
        "all source maturity rows are persisted"
    );
    assert!(
        recorded
            .sources
            .iter()
            .any(|source| source.maturity_status == SourceMaturityStatus::Done),
        "matrix must carry DONE maturity rows"
    );
    assert!(
        recorded
            .sources
            .iter()
            .any(|source| source.maturity_status == SourceMaturityStatus::Review),
        "matrix must carry REVIEW maturity rows"
    );
    let mt_018 = recorded
        .sources
        .iter()
        .find(|source| source.source_id == "MT-018.media-review-metadata")
        .expect("MT-018 source row recorded");
    assert_eq!(
        mt_018.maturity_status,
        SourceMaturityStatus::Review,
        "MT-018 should no longer be a missing-anchor source"
    );
    assert!(
        mt_018
            .evidence_refs
            .iter()
            .any(|path| path.ends_with("0043_atelier_media_review_metadata.sql")),
        "MT-018 source row must cite the review metadata migration"
    );
    assert!(mt_018.gap_reason.is_none());
    assert!(
        recorded.anchors.iter().any(|anchor| {
            anchor.verification_status == AnchorVerificationStatus::Verified
                && !anchor.verified_product_paths.is_empty()
        }),
        "verified anchors must cite product paths"
    );
    let mt_018_anchor = recorded
        .anchors
        .iter()
        .find(|anchor| anchor.anchor_id == "ANCHOR-MT-018-review-metadata")
        .expect("MT-018 anchor recorded");
    assert_eq!(
        mt_018_anchor.verification_status,
        AnchorVerificationStatus::Verified,
        "MT-018 review metadata anchor must be verified after product implementation"
    );
    assert!(
        mt_018_anchor
            .verified_product_paths
            .iter()
            .any(|path| path.ends_with("atelier_media_artifact_tests.rs")),
        "MT-018 anchor must cite its runtime proof"
    );

    let reloaded = store
        .get_source_evidence_matrix(&matrix.matrix_id)
        .await
        .expect("reload source evidence matrix");
    assert_eq!(reloaded.sources.len(), recorded.sources.len());
    assert_eq!(reloaded.anchors.len(), recorded.anchors.len());

    let mut second_matrix =
        core_data_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    second_matrix.matrix_id = format!("{CORE_DATA_SOURCE_EVIDENCE_MATRIX_ID}-{}", Uuid::new_v4());
    store
        .record_source_evidence_matrix(&second_matrix)
        .await
        .expect("record second matrix with same source_id/anchor_id values");
    let first_after_second = store
        .get_source_evidence_matrix(&matrix.matrix_id)
        .await
        .expect("first matrix remains reloadable after second matrix write");
    let second_reloaded = store
        .get_source_evidence_matrix(&second_matrix.matrix_id)
        .await
        .expect("second matrix reloads independently");
    assert_eq!(
        first_after_second.sources.len(),
        recorded.sources.len(),
        "matrix-scoped source rows must not be moved by a later matrix write"
    );
    assert_eq!(
        second_reloaded.sources.len(),
        second_matrix.sources.len(),
        "second matrix keeps its own source rows"
    );
    assert!(
        first_after_second
            .anchors
            .iter()
            .any(|anchor| anchor.anchor_id == "ANCHOR-MT-018-review-metadata"),
        "first matrix retains MT-018 anchor row"
    );
    assert!(
        second_reloaded
            .anchors
            .iter()
            .any(|anchor| anchor.anchor_id == "ANCHOR-MT-018-review-metadata"),
        "second matrix has its own MT-018 anchor row"
    );

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_source_evidence_matrix", &matrix.matrix_id)
        .await
        .expect("list source evidence matrix kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED
                && event.payload["atelier_payload"]["source_count"]
                    == serde_json::json!(matrix.sources.len())
                && event.payload["atelier_payload"]["blocked_missing_anchor_count"]
                    == serde_json::json!(0)
        }),
        "recording the matrix must emit canonical EventLedger evidence"
    );

    let second_kernel_events = database
        .list_kernel_events_for_aggregate(
            "atelier_source_evidence_matrix",
            &second_matrix.matrix_id,
        )
        .await
        .expect("list second source evidence matrix kernel events");
    assert!(
        second_kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED
                && event.payload["atelier_payload"]["source_count"]
                    == serde_json::json!(second_matrix.sources.len())
        }),
        "second matrix must emit a separate EventLedger aggregate"
    );
}

#[tokio::test]
async fn pose_comfy_source_evidence_matrix_records_required_adapter_sources() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP pose_comfy_source_evidence_matrix_records_required_adapter_sources: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let mut matrix = pose_comfy_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    matrix.matrix_id = format!("{POSE_COMFY_SOURCE_EVIDENCE_MATRIX_ID}-{}", Uuid::new_v4());
    let recorded = store
        .record_source_evidence_matrix(&matrix)
        .await
        .expect("record pose/comfy source evidence matrix");

    let expected_sources = [
        "MT-081.posekit",
        "MT-081.openpose",
        "MT-081.identity",
        "MT-081.comfyui",
        "MT-081.workflow-registry",
        "MT-081.image-sourcing-adapter",
    ];
    for source_id in expected_sources {
        let source = recorded
            .sources
            .iter()
            .find(|source| source.source_id == source_id)
            .unwrap_or_else(|| panic!("missing source row {source_id}"));
        assert_eq!(
            source.maturity_status,
            SourceMaturityStatus::Done,
            "{source_id} should be verified as a current product source"
        );
        assert!(
            source.gap_reason.is_none(),
            "{source_id} must not be carried as a missing-anchor placeholder"
        );
        assert!(
            !source.evidence_refs.is_empty() && !source.proof_refs.is_empty(),
            "{source_id} must cite product evidence and proof refs"
        );
    }

    assert_eq!(
        recorded
            .anchors
            .iter()
            .filter(|anchor| anchor.verification_status == AnchorVerificationStatus::Verified)
            .count(),
        expected_sources.len(),
        "each required Pose/ComfyUI source should have a verified product anchor"
    );
    let expected_source_set: HashSet<&str> = expected_sources.iter().copied().collect();
    let verified_anchor_sources = recorded
        .anchors
        .iter()
        .filter(|anchor| anchor.verification_status == AnchorVerificationStatus::Verified)
        .map(|anchor| anchor.source_id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(
        verified_anchor_sources, expected_source_set,
        "verified anchors must cover exactly the required Pose/ComfyUI sources"
    );
    for source in recorded.sources.iter() {
        for ref_path in source.evidence_refs.iter().chain(source.proof_refs.iter()) {
            assert_source_tree_ref_exists(ref_path);
        }
    }
    let expected_proof_refs = [
        (
            "MT-081.posekit",
            "src/backend/handshake_core/tests/atelier_pose_tests.rs",
        ),
        (
            "MT-081.openpose",
            "src/backend/handshake_core/tests/atelier_media_artifact_tests.rs",
        ),
        (
            "MT-081.identity",
            "src/backend/handshake_core/tests/atelier_pose_tests.rs",
        ),
        (
            "MT-081.comfyui",
            "src/backend/handshake_core/tests/atelier_comfy_tests.rs",
        ),
        (
            "MT-081.workflow-registry",
            "src/backend/handshake_core/tests/atelier_command_corpus_tests.rs",
        ),
        (
            "MT-081.image-sourcing-adapter",
            "src/backend/handshake_core/tests/atelier_sourcing_tests.rs",
        ),
    ];
    for (source_id, proof_ref) in expected_proof_refs {
        let source = recorded
            .sources
            .iter()
            .find(|source| source.source_id == source_id)
            .unwrap_or_else(|| panic!("missing source row {source_id}"));
        assert!(
            source.proof_refs.iter().any(|value| value == proof_ref),
            "{source_id} must cite exact runtime proof ref {proof_ref}"
        );
    }
    for anchor in recorded.anchors.iter() {
        assert_source_tree_ref_exists(&anchor.expected_product_path);
        for ref_path in &anchor.verified_product_paths {
            assert_source_tree_ref_exists(ref_path);
        }
    }
    assert!(
        recorded.anchors.iter().any(|anchor| {
            anchor.source_id == "MT-081.comfyui"
                && anchor
                    .verified_product_paths
                    .iter()
                    .any(|path| path.ends_with("atelier_comfy_tests.rs"))
        }),
        "ComfyUI anchor must cite its live PostgreSQL/EventLedger proof"
    );
    assert!(
        recorded.anchors.iter().any(|anchor| {
            anchor.source_id == "MT-081.image-sourcing-adapter"
                && anchor
                    .verified_product_paths
                    .iter()
                    .any(|path| path.ends_with("atelier_sourcing_tests.rs"))
        }),
        "image-sourcing adapter anchor must cite its live PostgreSQL/EventLedger proof"
    );

    let reloaded = store
        .get_source_evidence_matrix(&matrix.matrix_id)
        .await
        .expect("reload pose/comfy source evidence matrix");
    assert_eq!(reloaded.sources.len(), expected_sources.len());
    assert_eq!(reloaded.anchors.len(), expected_sources.len());

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_source_evidence_matrix", &matrix.matrix_id)
        .await
        .expect("list pose/comfy source evidence matrix kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED
                && event.payload["atelier_payload"]["source_count"]
                    == serde_json::json!(expected_sources.len())
                && event.payload["atelier_payload"]["blocked_missing_anchor_count"]
                    == serde_json::json!(0)
        }),
        "recording the Pose/ComfyUI matrix must emit canonical EventLedger evidence"
    );
}

#[tokio::test]
async fn mt082_pose_media_anchor_verification_records_required_anchors() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt082_pose_media_anchor_verification_records_required_anchors: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let mut matrix = pose_media_anchor_verification_matrix(format!("test-run-{}", Uuid::new_v4()));
    matrix.matrix_id = format!(
        "{POSE_MEDIA_ANCHOR_VERIFICATION_MATRIX_ID}-{}",
        Uuid::new_v4()
    );
    let recorded = store
        .record_source_evidence_matrix(&matrix)
        .await
        .expect("record pose/media anchor verification matrix");

    // Required pose/media product anchors named by the MT-082 contract:
    // pose, media, artifact, workflow, external tools, diagnostics.
    let required_anchors = [
        "ANCHOR-MT-082-pose",
        "ANCHOR-MT-082-media",
        "ANCHOR-MT-082-artifact",
        "ANCHOR-MT-082-workflow",
        "ANCHOR-MT-082-external-tools",
        "ANCHOR-MT-082-diagnostics",
    ];
    for anchor_id in required_anchors {
        let anchor = recorded
            .anchors
            .iter()
            .find(|anchor| anchor.anchor_id == anchor_id)
            .unwrap_or_else(|| panic!("missing required pose/media anchor {anchor_id}"));
        match anchor.verification_status {
            AnchorVerificationStatus::Verified => {
                assert!(
                    !anchor.verified_product_paths.is_empty(),
                    "{anchor_id} VERIFIED anchor must cite non-empty verified_product_paths"
                );
                assert_source_tree_ref_exists(&anchor.expected_product_path);
                for ref_path in &anchor.verified_product_paths {
                    assert_source_tree_ref_exists(ref_path);
                }
            }
            AnchorVerificationStatus::BlockedMissingAnchor => {
                assert!(
                    anchor
                        .blocking_reason
                        .as_deref()
                        .map(str::trim)
                        .map(|reason| !reason.is_empty())
                        .unwrap_or(false),
                    "{anchor_id} BLOCKED_MISSING_ANCHOR must carry a blocking_reason"
                );
            }
        }
    }

    // All MT-082 anchors back existing product surfaces, so all are VERIFIED.
    assert_eq!(
        recorded
            .anchors
            .iter()
            .filter(|anchor| anchor.verification_status == AnchorVerificationStatus::Verified)
            .count(),
        required_anchors.len(),
        "every required pose/media product anchor must be VERIFIED"
    );

    let reloaded = store
        .get_source_evidence_matrix(&matrix.matrix_id)
        .await
        .expect("reload pose/media anchor verification matrix");
    assert_eq!(reloaded.anchors.len(), required_anchors.len());
    let reloaded_anchor_ids: HashSet<&str> = reloaded
        .anchors
        .iter()
        .map(|anchor| anchor.anchor_id.as_str())
        .collect();
    for anchor_id in required_anchors {
        assert!(
            reloaded_anchor_ids.contains(anchor_id),
            "reloaded matrix must round-trip required anchor {anchor_id}"
        );
    }
    let reloaded_pose = reloaded
        .anchors
        .iter()
        .find(|anchor| anchor.anchor_id == "ANCHOR-MT-082-pose")
        .expect("pose anchor round-trips");
    assert_eq!(
        reloaded_pose.verification_status,
        AnchorVerificationStatus::Verified,
        "reloaded pose anchor must remain VERIFIED"
    );
    assert!(
        reloaded_pose
            .verified_product_paths
            .iter()
            .any(|path| path.ends_with("atelier/pose.rs")),
        "reloaded pose anchor must cite its product module path"
    );

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_source_evidence_matrix", &matrix.matrix_id)
        .await
        .expect("list pose/media anchor verification kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED
                && event.payload["atelier_payload"]["verified_anchor_count"]
                    == serde_json::json!(required_anchors.len())
                && event.payload["atelier_payload"]["blocked_missing_anchor_count"]
                    == serde_json::json!(0)
        }),
        "recording the MT-082 anchor matrix must emit canonical EventLedger evidence"
    );
}

#[tokio::test]
async fn source_evidence_matrix_rerecord_removes_omitted_rows() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP source_evidence_matrix_rerecord_removes_omitted_rows: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _) = connected_store_with_ledger(&url).await;

    let mut matrix = pose_comfy_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    matrix.matrix_id = format!("{POSE_COMFY_SOURCE_EVIDENCE_MATRIX_ID}-{}", Uuid::new_v4());
    store
        .record_source_evidence_matrix(&matrix)
        .await
        .expect("record full source evidence matrix");

    let removed_source_id = "MT-081.openpose";
    let removed_anchor_id = "ANCHOR-MT-081-openpose";
    let mut reduced_matrix = matrix.clone();
    reduced_matrix.recorded_by = format!("test-rerecord-{}", Uuid::new_v4());
    reduced_matrix
        .sources
        .retain(|source| source.source_id != removed_source_id);
    reduced_matrix
        .anchors
        .retain(|anchor| anchor.anchor_id != removed_anchor_id);

    let recorded = store
        .record_source_evidence_matrix(&reduced_matrix)
        .await
        .expect("re-record reduced source evidence matrix");
    assert_eq!(
        recorded.sources.len(),
        reduced_matrix.sources.len(),
        "re-recording a stable matrix must remove omitted source rows"
    );
    assert_eq!(
        recorded.anchors.len(),
        reduced_matrix.anchors.len(),
        "re-recording a stable matrix must remove omitted anchor rows"
    );
    assert!(
        recorded
            .sources
            .iter()
            .all(|source| source.source_id != removed_source_id),
        "stale source rows must not remain after a stable matrix re-record"
    );
    assert!(
        recorded
            .anchors
            .iter()
            .all(|anchor| anchor.anchor_id != removed_anchor_id),
        "stale anchor rows must not remain after a stable matrix re-record"
    );
}

#[tokio::test]
async fn source_evidence_matrix_rejects_candidate_and_invalid_anchor_rows() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP source_evidence_matrix_rejects_candidate_and_invalid_anchor_rows: PostgreSQL unavailable"
        );
        return;
    };
    let (store, _) = connected_store_with_ledger(&url).await;

    let mut candidate_matrix =
        core_data_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    candidate_matrix.matrix_id = format!("candidate-reject-{}", Uuid::new_v4());
    candidate_matrix.sources[0].evidence_refs = vec!["candidate::CoreParser".to_string()];
    let err = store
        .record_source_evidence_matrix(&candidate_matrix)
        .await
        .expect_err("candidate refs must not persist as verified product anchors");
    assert!(
        err.to_string().contains("candidate name"),
        "candidate-name rejection must be explicit: {err}"
    );
    assert!(
        store
            .get_source_evidence_matrix(&candidate_matrix.matrix_id)
            .await
            .is_err(),
        "invalid candidate matrix must not persist source rows"
    );

    let mut bad_verified = core_data_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    bad_verified.matrix_id = format!("bad-verified-{}", Uuid::new_v4());
    bad_verified.anchors.push(NewAnchorVerificationRecord {
        anchor_id: format!("ANCHOR-BAD-VERIFIED-{}", Uuid::new_v4()),
        source_id: bad_verified.sources[0].source_id.clone(),
        anchor_label: "Bad verified anchor".to_string(),
        expected_product_path: "src/backend/handshake_core/src/atelier/core.rs".to_string(),
        verification_status: AnchorVerificationStatus::Verified,
        verified_product_paths: vec![],
        blocking_reason: None,
    });
    let err = store
        .record_source_evidence_matrix(&bad_verified)
        .await
        .expect_err("verified anchors without product paths must be rejected");
    assert!(
        err.to_string().contains("VERIFIED anchors"),
        "verified-anchor rejection must be explicit: {err}"
    );

    let mut bad_blocked = core_data_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    bad_blocked.matrix_id = format!("bad-blocked-{}", Uuid::new_v4());
    bad_blocked.anchors.push(NewAnchorVerificationRecord {
        anchor_id: format!("ANCHOR-BAD-BLOCKED-{}", Uuid::new_v4()),
        source_id: bad_blocked.sources[0].source_id.clone(),
        anchor_label: "Bad blocked anchor".to_string(),
        expected_product_path: "src/backend/handshake_core/src/atelier/missing.rs".to_string(),
        verification_status: AnchorVerificationStatus::BlockedMissingAnchor,
        verified_product_paths: vec![],
        blocking_reason: None,
    });
    let err = store
        .record_source_evidence_matrix(&bad_blocked)
        .await
        .expect_err("blocked anchors without reason must be rejected");
    assert!(
        err.to_string().contains("BLOCKED_MISSING_ANCHOR"),
        "blocked-anchor rejection must be explicit: {err}"
    );
}

/// MT-002 v3 strengthening: a VERIFIED anchor row must mean more than "the
/// cited file exists on disk". Every migration a Core/Data anchor cites must
/// be APPLIED in the live Handshake-managed PostgreSQL (`_sqlx_migrations`),
/// and the representative anchors' migrations must actively shape the runtime
/// schema (their tables exist in `information_schema`). All cited refs are
/// taken from the matrix RE-READ from PostgreSQL, never from the in-memory
/// constant.
#[tokio::test]
async fn mt002_core_data_verified_anchor_migrations_are_applied_in_live_postgres() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt002_core_data_verified_anchor_migrations_are_applied_in_live_postgres: PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let mut matrix = core_data_source_evidence_matrix(format!("test-run-{}", Uuid::new_v4()));
    matrix.matrix_id = format!("{CORE_DATA_SOURCE_EVIDENCE_MATRIX_ID}-{}", Uuid::new_v4());
    store
        .record_source_evidence_matrix(&matrix)
        .await
        .expect("record core-data source evidence matrix");
    let reloaded = store
        .get_source_evidence_matrix(&matrix.matrix_id)
        .await
        .expect("re-read core-data source evidence matrix from PostgreSQL");

    // Collect every migration the persisted matrix cites as anchor evidence.
    let cited_refs = reloaded
        .sources
        .iter()
        .flat_map(|source| source.evidence_refs.iter().chain(source.proof_refs.iter()))
        .chain(
            reloaded
                .anchors
                .iter()
                .flat_map(|anchor| anchor.verified_product_paths.iter()),
        );
    let mut cited_migrations: HashSet<(i64, String)> = HashSet::new();
    for ref_path in cited_refs {
        let file_name = ref_path.rsplit('/').next().unwrap_or(ref_path);
        if !file_name.ends_with(".sql") {
            continue;
        }
        let version: i64 = file_name
            .split('_')
            .next()
            .and_then(|prefix| prefix.parse().ok())
            .unwrap_or_else(|| {
                panic!("cited migration {file_name} must carry a numeric version prefix")
            });
        cited_migrations.insert((version, ref_path.clone()));
    }
    assert!(
        !cited_migrations.is_empty(),
        "core-data matrix must cite at least one migration as anchor evidence"
    );

    // Behavioral upgrade over assert_source_tree_ref_exists: each cited
    // migration must be applied in the live database, not merely on disk.
    for (version, ref_path) in &cited_migrations {
        let applied: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(
                 SELECT 1 FROM _sqlx_migrations
                 WHERE version = $1 AND success = TRUE
               )"#,
        )
        .bind(version)
        .fetch_one(store.pool())
        .await
        .expect("query _sqlx_migrations for cited anchor migration");
        assert!(
            applied,
            "anchor evidence cites migration {ref_path}, which must be applied in live PostgreSQL"
        );
    }

    // Representative Core/Data anchors: the cited migrations actively shape
    // the runtime schema the claimed behavior runs on.
    let anchored_runtime_tables = [
        (
            "0043_atelier_media_review_metadata.sql",
            "atelier_media_review_metadata",
        ),
        (
            "0037_atelier_sheet_parser_ast.sql",
            "atelier_sheet_parse_snapshot",
        ),
    ];
    for (migration_suffix, runtime_table) in anchored_runtime_tables {
        assert!(
            reloaded.anchors.iter().any(|anchor| {
                anchor.verification_status == AnchorVerificationStatus::Verified
                    && anchor
                        .verified_product_paths
                        .iter()
                        .any(|path| path.ends_with(migration_suffix))
            }),
            "a VERIFIED core-data anchor must keep citing {migration_suffix}"
        );
        let table_exists: bool = sqlx::query_scalar(
            r#"SELECT EXISTS(
                 SELECT 1 FROM information_schema.tables
                 WHERE table_schema = 'public' AND table_name = $1
               )"#,
        )
        .bind(runtime_table)
        .fetch_one(store.pool())
        .await
        .expect("query information_schema for anchor runtime table");
        assert!(
            table_exists,
            "cited migration {migration_suffix} must create runtime table {runtime_table} in live PostgreSQL"
        );
    }

    // Canonical EventLedger evidence for this recording.
    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_source_evidence_matrix", &matrix.matrix_id)
        .await
        .expect("list core-data anchor verification kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED
                && event.payload["atelier_payload"]["blocked_missing_anchor_count"]
                    == serde_json::json!(0)
        }),
        "recording the MT-002 anchor matrix must emit canonical EventLedger evidence"
    );
}
