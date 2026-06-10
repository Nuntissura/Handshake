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
    error_taxonomy_catalog, prompt_response_matrix_catalog, DiagnosticsErrorClass,
    NewPromptResponseMatrixEntry, PromptResponseMatrixStatus,
};
use handshake_core::atelier::{AtelierError, AtelierStore};
use std::collections::HashMap;

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
#[tokio::test]
async fn mt166_reset_orphan_diagnostics_projection() {
    let Some(url) = database_url().await else {
        eprintln!("SKIP mt166_reset_orphan_diagnostics_projection: PostgreSQL unavailable");
        return;
    };
    let store = connected_store(&url).await;

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

    // Counts are non-negative and consistent with a fresh read.
    assert!(
        projection.orphaned_pending_count >= 0,
        "orphaned pending count must be non-negative"
    );
    assert!(
        projection.adopted_count >= 0,
        "adopted count must be non-negative"
    );

    // The read-only projection matches the recorded projection (same canonical
    // source rows).
    let read_again = store
        .list_reset_orphan_diagnostics()
        .await
        .expect("list reset/orphan diagnostics");
    assert_eq!(
        read_again.reset_modes, projection.reset_modes,
        "read-only and recorded projections must enumerate the same reset modes"
    );
    assert_eq!(
        read_again.resets.len(),
        projection.resets.len(),
        "read-only and recorded projections must surface the same reset rows"
    );
}
