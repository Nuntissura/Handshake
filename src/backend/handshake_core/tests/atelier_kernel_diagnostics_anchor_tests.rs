//! WP-KERNEL-005 MT-133 (Diagnostics Product Anchor Verification) live
//! PostgreSQL round-trip proof.
//!
//! No mocks: the test verifies the kernel-diagnostics anchor matrix against
//! the REAL product source tree (sessions, command catalog, Workflow Engine,
//! DCC, Locus, Flight Recorder, visual capture, build diagnostics), records it
//! through the real `AtelierStore` into live PostgreSQL, RE-READS it, and
//! asserts the canonical `atelier.source_evidence.matrix_recorded` EventLedger
//! family. A negative path proves the verification logic is real: against a
//! source tree without the product anchors, every anchor downgrades to
//! `BLOCKED_MISSING_ANCHOR` and the recorded EventLedger payload carries the
//! blocked count.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::source_evidence::{
    source_evidence_event_family, AnchorVerificationStatus, SourceMaturityStatus,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::diagnostics::product_anchor_matrix::{
    verify_kernel_diagnostics_anchor_matrix, KERNEL_DIAGNOSTICS_ANCHOR_VERIFICATION_MATRIX_ID,
    KERNEL_DIAGNOSTIC_SURFACES,
};
use handshake_core::kernel::KernelEventType;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

/// The repository root that contains `src/backend/handshake_core` and `app`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repository root above src/backend/handshake_core")
        .to_path_buf()
}

#[tokio::test]
async fn mt133_kernel_diagnostics_anchor_matrix_verifies_all_contract_surfaces() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt133_kernel_diagnostics_anchor_matrix_verifies_all_contract_surfaces: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    let root = repo_root();
    assert!(
        root.join("src/backend/handshake_core/src/lib.rs").exists(),
        "repo root resolution must land on the product source tree: {}",
        root.display()
    );

    let mut matrix =
        verify_kernel_diagnostics_anchor_matrix(&root, format!("test-run-{}", Uuid::new_v4()));
    matrix.matrix_id = format!(
        "{KERNEL_DIAGNOSTICS_ANCHOR_VERIFICATION_MATRIX_ID}-{}",
        Uuid::new_v4()
    );

    // Every surface the MT-133 contract names is covered and VERIFIED against
    // the real source tree before anything is persisted.
    assert_eq!(matrix.sources.len(), KERNEL_DIAGNOSTIC_SURFACES.len());
    assert_eq!(matrix.anchors.len(), KERNEL_DIAGNOSTIC_SURFACES.len());
    for surface in KERNEL_DIAGNOSTIC_SURFACES {
        let anchor_id = format!("ANCHOR-MT-133-{surface}");
        let anchor = matrix
            .anchors
            .iter()
            .find(|anchor| anchor.anchor_id == anchor_id)
            .unwrap_or_else(|| panic!("missing kernel diagnostics anchor {anchor_id}"));
        assert_eq!(
            anchor.verification_status,
            AnchorVerificationStatus::Verified,
            "{anchor_id} must verify against the live product source tree: {:?}",
            anchor.blocking_reason
        );
        assert!(
            !anchor.verified_product_paths.is_empty(),
            "{anchor_id} must cite verified product paths"
        );
        for ref_path in std::iter::once(&anchor.expected_product_path)
            .chain(anchor.verified_product_paths.iter())
        {
            assert!(
                root.join(ref_path).exists(),
                "{anchor_id} cites a product path that must exist: {ref_path}"
            );
        }
    }
    for source in &matrix.sources {
        assert_eq!(
            source.maturity_status,
            SourceMaturityStatus::Done,
            "{} must be DONE against the live source tree",
            source.source_id
        );
        assert!(source.gap_reason.is_none());
        for ref_path in source.evidence_refs.iter().chain(source.proof_refs.iter()) {
            assert!(
                root.join(ref_path).exists(),
                "{} cites a path that must exist: {ref_path}",
                source.source_id
            );
        }
    }

    // Persist through the real store, then RE-READ from PostgreSQL.
    let recorded = store
        .record_source_evidence_matrix(&matrix)
        .await
        .expect("record kernel diagnostics anchor matrix");
    assert_eq!(recorded.sources.len(), KERNEL_DIAGNOSTIC_SURFACES.len());
    assert_eq!(recorded.anchors.len(), KERNEL_DIAGNOSTIC_SURFACES.len());

    let reloaded = store
        .get_source_evidence_matrix(&matrix.matrix_id)
        .await
        .expect("reload kernel diagnostics anchor matrix");
    let reloaded_anchor_ids: HashSet<&str> = reloaded
        .anchors
        .iter()
        .map(|anchor| anchor.anchor_id.as_str())
        .collect();
    for surface in KERNEL_DIAGNOSTIC_SURFACES {
        let anchor_id = format!("ANCHOR-MT-133-{surface}");
        assert!(
            reloaded_anchor_ids.contains(anchor_id.as_str()),
            "reloaded matrix must round-trip anchor {anchor_id}"
        );
    }
    assert!(
        reloaded
            .anchors
            .iter()
            .all(|anchor| anchor.verification_status == AnchorVerificationStatus::Verified),
        "reloaded kernel diagnostics anchors must remain VERIFIED"
    );
    let reloaded_dcc = reloaded
        .anchors
        .iter()
        .find(|anchor| anchor.anchor_id == "ANCHOR-MT-133-dcc")
        .expect("DCC anchor round-trips");
    assert!(
        reloaded_dcc
            .verified_product_paths
            .iter()
            .any(|path| path.ends_with("KernelDccProjectionView.tsx")),
        "DCC anchor must cite the app DCC projection surface"
    );
    let reloaded_sessions = reloaded
        .sources
        .iter()
        .find(|source| source.source_id == "MT-133.sessions")
        .expect("sessions source row round-trips");
    assert!(
        reloaded_sessions
            .evidence_refs
            .iter()
            .any(|path| path.ends_with("kernel/session_broker.rs")),
        "sessions source row must cite the session broker product module"
    );

    // EventLedger proof for the recorded matrix.
    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_source_evidence_matrix", &matrix.matrix_id)
        .await
        .expect("list kernel diagnostics anchor matrix kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED
                && event.payload["atelier_payload"]["verified_anchor_count"]
                    == serde_json::json!(KERNEL_DIAGNOSTIC_SURFACES.len())
                && event.payload["atelier_payload"]["blocked_missing_anchor_count"]
                    == serde_json::json!(0)
        }),
        "recording the MT-133 anchor matrix must emit canonical EventLedger evidence"
    );
}

#[tokio::test]
async fn mt133_verification_records_blocked_anchors_for_missing_source_tree() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt133_verification_records_blocked_anchors_for_missing_source_tree: \
             PostgreSQL unavailable"
        );
        return;
    };
    let (store, database) = connected_store_with_ledger(&url).await;

    // A real (empty) directory that does not contain the product anchors: the
    // verification logic must downgrade every anchor instead of trusting the
    // declared matrix.
    let empty_root = tempfile::tempdir().expect("create empty verification root");
    let mut matrix = verify_kernel_diagnostics_anchor_matrix(
        empty_root.path(),
        format!("test-run-{}", Uuid::new_v4()),
    );
    matrix.matrix_id = format!(
        "{KERNEL_DIAGNOSTICS_ANCHOR_VERIFICATION_MATRIX_ID}-blocked-{}",
        Uuid::new_v4()
    );

    assert!(
        matrix.anchors.iter().all(|anchor| {
            anchor.verification_status == AnchorVerificationStatus::BlockedMissingAnchor
        }),
        "anchors must not stay VERIFIED against a source tree without the product paths"
    );
    for anchor in &matrix.anchors {
        let reason = anchor
            .blocking_reason
            .as_deref()
            .expect("blocked anchors carry a blocking_reason");
        assert!(
            reason.contains("missing kernel-diagnostics product anchors"),
            "blocking_reason must name the missing anchors: {reason}"
        );
    }
    assert!(matrix
        .sources
        .iter()
        .all(|source| source.maturity_status == SourceMaturityStatus::Blocked));

    let recorded = store
        .record_source_evidence_matrix(&matrix)
        .await
        .expect("record blocked kernel diagnostics anchor matrix");
    assert!(recorded.anchors.iter().all(|anchor| {
        anchor.verification_status == AnchorVerificationStatus::BlockedMissingAnchor
    }));

    let reloaded = store
        .get_source_evidence_matrix(&matrix.matrix_id)
        .await
        .expect("reload blocked kernel diagnostics anchor matrix");
    assert_eq!(reloaded.anchors.len(), KERNEL_DIAGNOSTIC_SURFACES.len());
    assert!(reloaded.anchors.iter().all(|anchor| {
        anchor.verification_status == AnchorVerificationStatus::BlockedMissingAnchor
    }));

    let kernel_events = database
        .list_kernel_events_for_aggregate("atelier_source_evidence_matrix", &matrix.matrix_id)
        .await
        .expect("list blocked kernel diagnostics matrix kernel events");
    assert!(
        kernel_events.iter().any(|event| {
            event.event_type == KernelEventType::AtelierDomainEventRecorded
                && event.payload["event_family"]
                    == source_evidence_event_family::SOURCE_EVIDENCE_MATRIX_RECORDED
                && event.payload["atelier_payload"]["blocked_missing_anchor_count"]
                    == serde_json::json!(KERNEL_DIAGNOSTIC_SURFACES.len())
                && event.payload["atelier_payload"]["verified_anchor_count"]
                    == serde_json::json!(0)
        }),
        "blocked MT-133 matrix must emit EventLedger evidence with the blocked anchor count"
    );
}
