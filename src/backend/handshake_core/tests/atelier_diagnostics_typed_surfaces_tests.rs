//! WP-KERNEL-005 MT-140 / MT-207 / MT-166: real PostgreSQL round-trip proofs
//! for the typed Model-Workflow-Diagnostics runtime surfaces.
//!
//! These MTs are TYPED RUNTIME surfaces (Postgres rows + EventLedger events),
//! never governance markdown:
//!   * MT-140 -- structured ErrorTaxonomy: 10 canonical error classes, each with
//!     a recovery hint.
//!   * MT-207 -- CKC WP-0118 prompt-response matrix preserved as a DEFERRED
//!     contract (prompt set + expected-response shape + scoring schema), no live
//!     scoring.
//!   * MT-166 -- Installer Reset And Orphan Evidence Projection over the existing
//!     reset/orphan tables (migration 0089).
//!
//! Gated on `atelier_pg_support::database_url()`: when no PostgreSQL is
//! available the test prints SKIP and returns (never SQLite).
//!
//! NOTE: migration 0112 is not yet wired into `ensure_schema` (the orchestrator
//! wires it after this MT lands). The shared preamble therefore applies the
//! 0112 migration itself; `CREATE TABLE IF NOT EXISTS` makes this idempotent and
//! safe once the orchestrator has wired it in.

mod atelier_pg_support;

use atelier_pg_support::database_url;
use handshake_core::atelier::command_corpus::{
    diagnostics_event_family, error_taxonomy_catalog, prompt_response_matrix_catalog,
    DiagnosticsErrorClass, NewPromptResponseMatrixEntry, PromptResponseMatrixStatus,
};
use handshake_core::atelier::intake::{
    AtelierResetMode, AtelierResetRequest, OrphanAdoptionRequest, OrphanAdoptionStatus,
};
use handshake_core::atelier::{AtelierError, AtelierStore, NewMediaAsset};
use std::collections::HashMap;
use uuid::Uuid;

/// Connect, ensure the wired schema, then apply the (not-yet-wired) 0112
/// diagnostics migration. Idempotent.
async fn connected_store(url: &str) -> AtelierStore {
    let store = AtelierStore::connect(url)
        .await
        .expect("connect to PostgreSQL");
    store.ensure_schema().await.expect("ensure atelier schema");
    sqlx::raw_sql(include_str!(
        "../migrations/0112_atelier_diagnostics_typed_surfaces.sql"
    ))
    .execute(store.pool())
    .await
    .expect("apply 0112 diagnostics migration");
    store
}

/// MT-140: the error-taxonomy records all 10 classes, each with a non-empty
/// recovery hint, and round-trips through Postgres.
#[tokio::test]
async fn mt140_error_taxonomy_covers_ten_classes_with_recovery() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt140_error_taxonomy_covers_ten_classes_with_recovery: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    // The catalog is exactly the 10 required classes.
    let catalog = error_taxonomy_catalog();
    assert_eq!(
        catalog.len(),
        10,
        "error taxonomy must define exactly 10 classes"
    );

    let recorded = store
        .record_error_taxonomy_catalog()
        .await
        .expect("record error taxonomy catalog");
    assert_eq!(recorded.len(), 10, "all 10 classes must persist");

    let reloaded = store
        .list_diagnostics_error_taxonomy()
        .await
        .expect("list error taxonomy");
    let by_token: HashMap<String, _> = reloaded
        .into_iter()
        .map(|e| (e.class.as_token().to_string(), e))
        .collect();

    let expected_tokens = [
        "validation",
        "capability_denied",
        "missing_state",
        "stale_lease",
        "timeout",
        "artifact_missing",
        "parse",
        "visual_mismatch",
        "package_guard",
        "stale_docs",
    ];
    assert_eq!(
        by_token.len(),
        expected_tokens.len(),
        "exactly 10 distinct classes must persist"
    );
    for token in expected_tokens {
        let entry = by_token
            .get(token)
            .unwrap_or_else(|| panic!("MT-140 class {token} must be present"));
        assert!(
            !entry.recovery_hint.trim().is_empty(),
            "MT-140 class {token} must carry a non-empty recovery hint"
        );
        assert!(
            !entry.description.trim().is_empty(),
            "MT-140 class {token} must carry a non-empty description"
        );
        // The persisted recovery hint matches the typed enum's hint.
        let class = DiagnosticsErrorClass::from_token(token).expect("known token");
        assert_eq!(
            entry.recovery_hint,
            class.recovery_hint(),
            "MT-140 persisted recovery hint must match the typed enum hint for {token}"
        );
    }
}

/// MT-140 negative: an unknown class token is rejected.
#[tokio::test]
async fn mt140_unknown_error_class_token_is_rejected() {
    let err = DiagnosticsErrorClass::from_token("not_a_real_class")
        .expect_err("unknown error class token must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "unknown error class token must produce a Validation error, got {err:?}"
    );
}

/// MT-207: the WP-0118 prompt-response matrix is preserved as a DEFERRED
/// contract -- prompt set + expected-response shape + scoring schema -- and
/// round-trips through Postgres with status DEFERRED.
#[tokio::test]
async fn mt207_prompt_response_matrix_preserved_as_deferred_contract() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt207_prompt_response_matrix_preserved_as_deferred_contract: PostgreSQL unavailable"
        );
        return;
    };
    let store = connected_store(&url).await;

    let catalog = prompt_response_matrix_catalog();
    assert!(
        !catalog.is_empty(),
        "WP-0118 prompt-response matrix catalog must preserve real entries"
    );
    for new in &catalog {
        assert_eq!(
            new.status,
            PromptResponseMatrixStatus::Deferred,
            "every preserved WP-0118 entry must default to DEFERRED (no live scoring)"
        );
    }

    store
        .record_prompt_response_matrix_catalog()
        .await
        .expect("record prompt-response matrix catalog");

    let reloaded = store
        .list_prompt_response_matrix()
        .await
        .expect("list prompt-response matrix");
    assert_eq!(
        reloaded.len(),
        catalog.len(),
        "all preserved matrix entries must persist"
    );
    for entry in &reloaded {
        assert_eq!(
            entry.status,
            PromptResponseMatrixStatus::Deferred,
            "preserved matrix entry {} must remain DEFERRED",
            entry.entry_id
        );
        assert!(
            !entry.prompt_text.trim().is_empty(),
            "matrix entry {} must preserve a prompt",
            entry.entry_id
        );
        // expected-response shape + scoring schema preserved as JSON.
        assert!(
            entry.expected_response_shape.is_object() || entry.expected_response_shape.is_array(),
            "matrix entry {} must preserve an expected-response shape",
            entry.entry_id
        );
        assert!(
            entry.scoring_schema.is_object(),
            "matrix entry {} must preserve a scoring schema",
            entry.entry_id
        );
    }
}

/// MT-207 negative: a scoring schema that is not a JSON object is rejected (the
/// scoring schema is a contract descriptor, never a scalar or live score).
#[tokio::test]
async fn mt207_non_object_scoring_schema_is_rejected() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt207_non_object_scoring_schema_is_rejected: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    let bad = NewPromptResponseMatrixEntry {
        entry_id: "wp-0118.bad-scoring-probe".to_string(),
        prompt_text: "probe".to_string(),
        expected_response_shape: serde_json::json!({ "type": "object" }),
        scoring_schema: serde_json::json!(0.5),
        status: PromptResponseMatrixStatus::Deferred,
    };
    let err = store
        .record_prompt_response_matrix_entry(&bad)
        .await
        .expect_err("non-object scoring schema must be rejected");
    assert!(
        matches!(err, AtelierError::Validation(_)),
        "non-object scoring schema must produce a Validation error, got {err:?}"
    );
}

/// MT-166: the reset/orphan evidence projection reads the existing reset/orphan
/// tables, enumerates the reset modes, surfaces orphan-pending/adopted counts,
/// and emits a projection event.
///
/// Non-trivial seeding: a real full-preserve reset is recorded first (creating
/// an `atelier_reset_operation` row plus `atelier_orphan_manifest_item` rows),
/// one orphan is adopted, and the projection is asserted to surface exactly
/// those canonical rows — never just `>= 0` counts.
#[tokio::test]
async fn mt166_reset_orphan_diagnostics_projection() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt166_reset_orphan_diagnostics_projection: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

    const PROJECTION_AGGREGATE: (&str, &str) = (
        "atelier_diagnostics_reset_orphan_projection",
        "reset-orphan-evidence",
    );
    let baseline_events = store
        .count_events_for_aggregate(
            diagnostics_event_family::DIAGNOSTICS_RESET_ORPHAN_PROJECTED,
            PROJECTION_AGGREGATE.0,
            PROJECTION_AGGREGATE.1,
        )
        .await
        .expect("count projection events before recording");

    // --- Seed: an original media asset, then a full-preserve reset. -------
    // The reset writes a real atelier_reset_operation row and one orphaned
    // atelier_orphan_manifest_item per preserved original.
    let marker = format!("mt-166-reset-orphan-{}", Uuid::new_v4());
    let artifact =
        atelier_pg_support::write_native_media_artifact(format!("{marker}-original").as_bytes());
    let asset = store
        .materialize_media_asset(&NewMediaAsset {
            content_hash: artifact.content_hash.clone(),
            mime: "image/png".to_string(),
            byte_len: artifact.byte_len,
            source_provenance: Some(format!("test-source:{marker}")),
            artifact_ref: artifact.artifact_ref.clone(),
        })
        .await
        .expect("materialize original media before reset");
    let reset = store
        .record_atelier_reset(&AtelierResetRequest {
            mode: AtelierResetMode::FullPreserveOriginalMedia,
            requested_by: "mt-166-diagnostics".to_string(),
            reason: format!("{marker}-full-preserve"),
        })
        .await
        .expect("record full-preserve reset");
    assert!(
        reset.original_media_preserved_count >= 1,
        "the seeded original must be preserved into the orphan manifest"
    );
    let manifest_id = reset
        .orphan_manifest_id
        .expect("full-preserve reset creates an orphan manifest");

    // --- Record the projection and prove it surfaces the seeded rows. -----
    let projection = store
        .record_reset_orphan_diagnostics_projection()
        .await
        .expect("record reset/orphan diagnostics projection");

    // Reset modes are enumerated for model diagnostics.
    assert!(
        projection
            .reset_modes
            .contains(&"preferences_only".to_string()),
        "projection must enumerate the preferences_only reset mode"
    );
    assert!(
        projection
            .reset_modes
            .contains(&"full_preserve_original_media".to_string()),
        "projection must enumerate the full_preserve_original_media reset mode"
    );

    // The seeded reset operation is surfaced with its canonical columns.
    let surfaced = projection
        .resets
        .iter()
        .find(|row| row.reset_id == reset.reset_id)
        .expect("the seeded reset operation must be surfaced by the projection");
    assert_eq!(surfaced.mode, "full_preserve_original_media");
    assert_eq!(surfaced.requested_by, "mt-166-diagnostics");
    assert_eq!(surfaced.reason, format!("{marker}-full-preserve"));
    assert_eq!(surfaced.orphan_manifest_id, Some(manifest_id));
    assert_eq!(
        surfaced.original_media_preserved_count, reset.original_media_preserved_count,
        "projection must mirror the canonical preserved-original count"
    );

    // The seeded orphaned item is counted as pending (non-zero, not >= 0).
    assert!(
        projection.orphaned_pending_count >= 1,
        "the seeded orphan manifest item must be counted as pending, got {}",
        projection.orphaned_pending_count
    );

    // Recording the projection emitted DIAGNOSTICS_RESET_ORPHAN_PROJECTED.
    let events_after_record = store
        .count_events_for_aggregate(
            diagnostics_event_family::DIAGNOSTICS_RESET_ORPHAN_PROJECTED,
            PROJECTION_AGGREGATE.0,
            PROJECTION_AGGREGATE.1,
        )
        .await
        .expect("count projection events after recording");
    assert_eq!(
        events_after_record,
        baseline_events + 1,
        "recording the projection must emit exactly one projection event"
    );

    // --- Adopt the seeded orphan: pending shrinks, adopted grows. ----------
    let items = store
        .list_orphan_manifest_items(manifest_id)
        .await
        .expect("list orphan manifest items");
    let seeded_item = items
        .iter()
        .find(|item| item.asset_id == asset.asset_id)
        .expect("manifest contains the seeded original asset");
    assert_eq!(seeded_item.adoption_status, OrphanAdoptionStatus::Orphaned);
    store
        .adopt_orphan_manifest_item(&OrphanAdoptionRequest {
            manifest_item_id: seeded_item.manifest_item_id,
            requested_by: "mt-166-adopt".to_string(),
        })
        .await
        .expect("adopt the seeded orphan manifest item");

    let after_adoption = store
        .list_reset_orphan_diagnostics()
        .await
        .expect("list reset/orphan diagnostics after adoption");
    assert_eq!(
        after_adoption.adopted_count,
        projection.adopted_count + 1,
        "adoption must move exactly one item into the adopted count"
    );
    assert_eq!(
        after_adoption.orphaned_pending_count,
        projection.orphaned_pending_count - 1,
        "adoption must remove exactly one item from the pending count"
    );

    // The read-only list path never emits the projection event.
    let events_after_list = store
        .count_events_for_aggregate(
            diagnostics_event_family::DIAGNOSTICS_RESET_ORPHAN_PROJECTED,
            PROJECTION_AGGREGATE.0,
            PROJECTION_AGGREGATE.1,
        )
        .await
        .expect("count projection events after read-only list");
    assert_eq!(
        events_after_list, events_after_record,
        "list_reset_orphan_diagnostics is read-only and must not emit events"
    );
}
