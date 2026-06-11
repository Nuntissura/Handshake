//! WP-KERNEL-005 MT-132: Diagnostics source/evidence + status matrix,
//! assembled at runtime and proven against live Handshake-managed PostgreSQL.
//!
//! `model_manual_tests.rs::manual_covers_diagnostics_surfaces` proves the
//! MT-132 manual rows exist, but only as a static doc-manifest assertion.
//! This test closes the v2 concern by doing what the manual workflow
//! `diagnostics_source_evidence_matrix` actually instructs a no-context model
//! to do: assemble the source/evidence + status matrix from the LIVE typed
//! ModelManual command_reference plus the registered kernel action catalog
//! (never claiming Wired without a resolvable source), persist it through the
//! real source-evidence store into PostgreSQL, RE-READ it, and assert the
//! canonical EventLedger evidence. Nothing below is satisfiable by static
//! manual text alone: the persisted rows are recomputed from runtime data and
//! compared against the PostgreSQL re-read.
//!
//! Run, e.g.:
//!   cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
//!     --features test-utils --test diagnostics_source_evidence_matrix_pg_tests \
//!     --target-dir ../Handshake_Artifacts/handshake-cargo-target

mod atelier_pg_support;

use std::path::{Path, PathBuf};

use handshake_core::atelier::source_evidence::{
    AnchorVerificationStatus, NewAnchorVerificationRecord, NewSourceEvidenceMatrix,
    NewSourceEvidenceRecord, SourceMaturityStatus, SOURCE_EVIDENCE_MATRIX_RECORDED,
};
use handshake_core::atelier::AtelierStore;
use handshake_core::kernel::action_catalog::kernel002_action_catalog;
use handshake_core::model_manual::{model_manual, CommandStatus};
use uuid::Uuid;

async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    store
}

fn source_tree_path(ref_path: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    match ref_path.strip_prefix("src/backend/handshake_core/") {
        Some(relative_to_crate) => manifest_dir.join(relative_to_crate),
        None => PathBuf::from(ref_path),
    }
}

const MANUAL_CONTENT_PATH: &str = "src/backend/handshake_core/src/model_manual/content.rs";
const ACTION_CATALOG_PATH: &str = "src/backend/handshake_core/src/kernel/action_catalog.rs";

/// MT-132: the diagnostics status matrix is assembled from live runtime
/// sources (manual command_reference + registered action catalog), persisted
/// to PostgreSQL, re-read, and evidenced in the EventLedger.
#[tokio::test]
async fn mt132_diagnostics_status_matrix_assembles_from_runtime_and_round_trips_postgres() {
    let Some(url) = atelier_pg_support::database_url().await else {
        eprintln!(
            "SKIP mt132_diagnostics_status_matrix_assembles_from_runtime_and_round_trips_postgres: no PostgreSQL"
        );
        return;
    };
    let store = connected_store(&url).await;
    let manual = model_manual();
    let catalog = kernel002_action_catalog();

    // The MT-132 feature group enumerates the diagnostics surfaces the matrix
    // must cover; resolve each through the LIVE manual command_reference.
    let group = manual
        .feature_groups
        .iter()
        .find(|group| group.id == "diagnostics_source_evidence_matrix")
        .expect("MT-132 diagnostics feature group present");
    assert!(
        !group.commands.is_empty(),
        "the diagnostics matrix group must enumerate surfaces"
    );

    let mut sources = Vec::new();
    let mut anchors = Vec::new();
    for command_id in group.commands {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == *command_id)
            .unwrap_or_else(|| panic!("group surface {command_id} has no CommandReference row"));

        // The matrix's core invariant, enforced at assembly time against the
        // RUNTIME rows: never claim Wired without a resolvable source.
        let (implementation_status, maturity_status, evidence_pointer, gap_reason) =
            match command.status {
                CommandStatus::Wired => {
                    let channel = command.ipc_channel.unwrap_or_else(|| {
                        panic!(
                            "surface {command_id} is marked Wired without an ipc_channel; \
                             the matrix must refuse unresolvable Wired claims"
                        )
                    });
                    // A Wired channel must resolve in a REGISTERED runtime
                    // surface: an HTTP channel names a route; a
                    // `kernel.inspector.*` channel is a read-IPC command
                    // registered in the Tauri bridge (same registry that
                    // inspector_ipc_tests verifies); everything else must be
                    // a registered kernel action-catalog action.
                    if channel.starts_with("kernel.inspector.") {
                        let bridge = std::fs::read_to_string(
                            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                                .join("../../../app/src-tauri/src/inspector.rs"),
                        )
                        .expect("read Tauri inspector bridge source");
                        assert!(
                            bridge.contains(&format!("\"{channel}\"")),
                            "Wired surface {command_id} claims inspector IPC channel \
                             {channel} that is not registered in the Tauri bridge"
                        );
                    } else if !channel.starts_with('/') {
                        assert!(
                            catalog.action(channel).is_some(),
                            "Wired surface {command_id} claims catalog action {channel} \
                             that is not registered in kernel002_action_catalog"
                        );
                    }
                    (
                        "wired".to_string(),
                        SourceMaturityStatus::Done,
                        format!("ipc://{channel}"),
                        None,
                    )
                }
                CommandStatus::Planned => (
                    "planned".to_string(),
                    SourceMaturityStatus::Blocked,
                    format!("manual://command_reference/{command_id}"),
                    Some(format!(
                        "surface {command_id} has no registered action or route yet; \
                         marked as blocker instead of fabricating evidence"
                    )),
                ),
            };

        sources.push(NewSourceEvidenceRecord {
            source_id: (*command_id).to_string(),
            source_label: command.name.to_string(),
            source_ref: format!("manual://command_reference/{command_id}"),
            product_area: "kernel.diagnostics".to_string(),
            maturity_status,
            implementation_status,
            evidence_refs: vec![evidence_pointer, MANUAL_CONTENT_PATH.to_string()],
            proof_refs: vec![
                "src/backend/handshake_core/tests/diagnostics_source_evidence_matrix_pg_tests.rs"
                    .to_string(),
            ],
            gap_reason,
        });
    }

    // Anchors: the wired command-map surface and the manual assembly source
    // must both anchor to real product files (verified on disk, not prose).
    for (anchor_id, source_id, anchor_label, product_path) in [
        (
            "mt-132.action-catalog",
            "kernel_action_catalog_view",
            "Registered kernel action catalog (command map)",
            ACTION_CATALOG_PATH,
        ),
        (
            "mt-132.model-manual",
            "diagnostics_source_evidence_matrix",
            "Typed ModelManual command_reference (matrix assembly source)",
            MANUAL_CONTENT_PATH,
        ),
    ] {
        assert!(
            source_tree_path(product_path).exists(),
            "anchor product path must exist in the source tree: {product_path}"
        );
        anchors.push(NewAnchorVerificationRecord {
            anchor_id: anchor_id.to_string(),
            source_id: source_id.to_string(),
            anchor_label: anchor_label.to_string(),
            expected_product_path: product_path.to_string(),
            verification_status: AnchorVerificationStatus::Verified,
            verified_product_paths: vec![product_path.to_string()],
            blocking_reason: None,
        });
    }

    // The command-map surface the validator anchored MT-132 on must be the
    // REGISTERED catalog action, resolved at runtime (not quoted from prose).
    let catalog_view = manual
        .command_reference
        .iter()
        .find(|command| command.id == "kernel_action_catalog_view")
        .expect("kernel_action_catalog_view manual row present");
    assert_eq!(catalog_view.status, CommandStatus::Wired);
    assert!(
        catalog
            .action(catalog_view.ipc_channel.expect("wired row has a channel"))
            .is_some(),
        "kernel.action_catalog.view must be a registered catalog action"
    );

    // --- persist through the real store, RE-READ from PostgreSQL ---
    let matrix_id = format!(
        "wp-kernel-005.diagnostics.status-matrix@1-{}",
        Uuid::new_v4()
    );
    let recorded = store
        .record_source_evidence_matrix(&NewSourceEvidenceMatrix {
            matrix_id: matrix_id.clone(),
            recorded_by: format!("mt-132-proof-{}", Uuid::new_v4().simple()),
            sources: sources.clone(),
            anchors,
        })
        .await
        .expect("record diagnostics status matrix in PostgreSQL");
    assert_eq!(recorded.sources.len(), group.commands.len());

    let reloaded = store
        .get_source_evidence_matrix(&matrix_id)
        .await
        .expect("re-read diagnostics status matrix from PostgreSQL");
    assert_eq!(
        reloaded.sources.len(),
        group.commands.len(),
        "every diagnostics surface row round-trips through PostgreSQL"
    );

    // Every persisted row must agree with the LIVE manual status when
    // recomputed — the re-read is compared against runtime data, so a drifted
    // manual or a fabricated Wired claim fails here, not in prose.
    for command_id in group.commands {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == *command_id)
            .expect("command row still present");
        let row = reloaded
            .sources
            .iter()
            .find(|row| row.source_id == *command_id)
            .unwrap_or_else(|| panic!("persisted matrix lost surface {command_id}"));
        let expected_status = match command.status {
            CommandStatus::Wired => "wired",
            CommandStatus::Planned => "planned",
        };
        assert_eq!(
            row.implementation_status, expected_status,
            "persisted status for {command_id} must match the runtime manual"
        );
        if expected_status == "planned" {
            assert!(
                row.gap_reason
                    .as_deref()
                    .is_some_and(|reason| reason.contains("blocker")),
                "planned surface {command_id} must persist its blocker, not fake evidence"
            );
        } else {
            assert!(
                row.evidence_refs
                    .iter()
                    .any(|evidence| evidence.starts_with("ipc://")),
                "wired surface {command_id} must persist its resolvable source pointer"
            );
        }
    }

    // Both anchors round-trip as VERIFIED with their product paths.
    assert_eq!(reloaded.anchors.len(), 2);
    for anchor in &reloaded.anchors {
        assert_eq!(anchor.verification_status, AnchorVerificationStatus::Verified);
        assert!(
            anchor
                .verified_product_paths
                .iter()
                .all(|path| source_tree_path(path).exists()),
            "verified anchor paths must exist in the product source tree"
        );
    }

    // --- canonical EventLedger evidence for this recording ---
    let recorded_events = store
        .count_events_for_aggregate(
            SOURCE_EVIDENCE_MATRIX_RECORDED,
            "atelier_source_evidence_matrix",
            &matrix_id,
        )
        .await
        .expect("count matrix-recorded events");
    assert_eq!(
        recorded_events, 1,
        "recording the MT-132 matrix must emit exactly one EventLedger event"
    );
}
