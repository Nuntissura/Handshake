//! MT-141 typed retirement inventory for the severed migration corpus.
//!
//! MT-139 removed the historical up/down corpus and made
//! `storage/surreal/schema.surql` the sole declarative schema authority. These
//! entries distinguish behavior that no longer exists from the active catalog
//! and restart guarantees that supersede it.

#[path = "knowledge_ingestion_support.rs"]
mod knowledge_ingestion_support;

use handshake_core::storage::surreal::{
    bootstrap_schema, SchemaBootstrapOutcome, EXPECTED_SCHEMA_INFO_SHA256,
};
use knowledge_ingestion_support::open_embedded_store;

struct RetiredTestDisposition {
    retired_test: &'static str,
    removed_behavior: &'static str,
    superseding_proof: &'static str,
}

const RETIRED_TESTS: &[RetiredTestDisposition] = &[
    RetiredTestDisposition {
        retired_test: "every_knowledge_migration_ships_a_down_file",
        removed_behavior: "per-version reversible migration twins; the historical migration corpus was severed by the recorded MT-139 operator decision",
        superseding_proof: "MT-139 PT-139-3: no historical migration files remain under src; rollback-file coverage is intentionally no longer a product behavior",
    },
    RetiredTestDisposition {
        retired_test: "scratch_schema_apply_rollback_reapply",
        removed_behavior: "incremental apply, reverse rollback, routine cleanup, and re-apply repair over the retired migration lifecycle",
        superseding_proof: "MT-139 PT-139-2: exact declarative catalog identities/counts plus idempotent bootstrap and real close/reopen fingerprint preservation; it supersedes active-schema and restart proof, not historical rollback support",
    },
];

#[test]
fn mt141_knowledge_migration_retirements_are_exact_and_non_vacuous() {
    assert_eq!(RETIRED_TESTS.len(), 2);
    assert_eq!(
        RETIRED_TESTS
            .iter()
            .map(|entry| entry.retired_test)
            .collect::<Vec<_>>(),
        vec![
            "every_knowledge_migration_ships_a_down_file",
            "scratch_schema_apply_rollback_reapply",
        ]
    );
    for entry in RETIRED_TESTS {
        assert!(!entry.removed_behavior.is_empty());
        assert!(entry.superseding_proof.starts_with("MT-139 PT-139-"));
    }
    assert!(RETIRED_TESTS[0]
        .superseding_proof
        .contains("no longer a product behavior"));
    assert!(RETIRED_TESTS[1]
        .superseding_proof
        .contains("not historical rollback support"));
}

#[tokio::test]
async fn exact_current_embedded_schema_rebootstrap_preserves_catalog_fingerprint() {
    let store = open_embedded_store()
        .await
        .expect("open mandatory embedded migration replacement fixture");

    let first = bootstrap_schema(&store.storage)
        .await
        .expect("re-bootstrap exact-current embedded schema");
    let second = bootstrap_schema(&store.storage)
        .await
        .expect("repeat exact-current embedded schema bootstrap");

    assert_eq!(first.outcome, SchemaBootstrapOutcome::ReusedExactCurrent);
    assert_eq!(second.outcome, SchemaBootstrapOutcome::ReusedExactCurrent);
    assert_eq!(first.info_fingerprint_sha256, EXPECTED_SCHEMA_INFO_SHA256);
    assert_eq!(
        second.info_fingerprint_sha256,
        first.info_fingerprint_sha256
    );
    assert_eq!(second.table_names, first.table_names);
    assert!(first
        .table_names
        .iter()
        .any(|table| table == "knowledge_schema_registry"));

    store
        .close_and_remove()
        .await
        .expect("close embedded migration replacement fixture");
}
