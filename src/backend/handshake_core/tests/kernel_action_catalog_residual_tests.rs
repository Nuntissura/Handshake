#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! WP-KERNEL-012 MT-150: coverage restored from the suite deleted by 4f92cc25.
//!
//! Recovered from (pre-deletion):
//! `git show 4f92cc25^:src/backend/handshake_core/tests/kernel_postgres_control_plane_residual_tests.rs`
//!
//! Only one of that file's three tests is kept. The other two
//! (`postgres_residual_scope_preserves_folded_stubs_and_maps_without_reopening_bundle`,
//! `postgres_residual_scope_rejects_sqlite_as_authority_for_postgres_required_work`)
//! exercised `PostgresControlPlaneResidualScopeV1` /
//! `validate_postgres_control_plane_residual_scope` /
//! `project_postgres_control_plane_residual_scope`, which have zero
//! occurrences in `src/` (removed with PostgreSQL by 4f92cc25). Their
//! behaviour was removed, not merely relocated, so they are already
//! dispositioned as retired and are not restored here.

use handshake_core::kernel::{
    action_catalog::{kernel002_action_catalog, validate_kernel_action_catalog},
    action_envelope::AuthorityEffect,
};

/// The kernel action catalog still advertises `kernel.postgres_residual.project`
/// (a `ProjectionOnly` action with the `postgres_residual_mapping` validation
/// hook and `disposition` in its DCC preview's primary state fields) even
/// though the projection implementation it once fronted
/// (`PostgresControlPlaneResidualScopeV1` and friends) was physically removed
/// with PostgreSQL. This test documents that the catalog advertises an action
/// with no backing implementation left in `src/` — a real, live discrepancy
/// a model calling this action would hit, not a hypothetical one.
#[test]
fn kernel_action_catalog_exposes_residual_projection_action() {
    let catalog = kernel002_action_catalog();
    validate_kernel_action_catalog(&catalog).expect("catalog must validate");

    let action = catalog
        .action("kernel.postgres_residual.project")
        .expect("Postgres residual projection action must still be cataloged");

    assert_eq!(action.authority_effect, AuthorityEffect::ProjectionOnly);
    assert!(action
        .validation_hooks
        .iter()
        .any(|hook| hook.hook_id == "postgres_residual_mapping"));
    assert!(action
        .dcc_preview
        .primary_state_fields
        .contains(&"disposition".to_string()));
}
