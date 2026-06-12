//! WP-KERNEL-009 UserManualAndNoContextOps (MT-193..MT-208): the canonical
//! UserManual product surface.
//!
//! Master Spec anchors:
//! * 10.15.8 "UserManual migration bridge" — UserManual is the canonical
//!   product concept for operator and model operation guidance. Legacy
//!   ModelManual paths MAY remain only while they map deterministically to a
//!   UserManualRecord authority entry and emit a compatibility receipt when
//!   used.
//! * 2.3.13.11 `UserManualRecord` — a no-context operator/model manual entry
//!   tied to real product commands, routes, IPC channels, schemas, recovery
//!   paths, and visual-debug anchors. PostgreSQL + EventLedger is authority.
//! * 12.7 `PRIM-UserManual` — UserManual coverage is required for every new
//!   ProjectKnowledgeIndex route, rich editor command, backend navigation
//!   action, retrieval trace view, graph view, visual-debug action, and
//!   recovery path.
//!
//! Module law (MT-193 naming migration):
//! * `user_manual` is the CANONICAL module. New manual content, storage,
//!   routes, and registries live here and only here.
//! * `crate::model_manual` is a DECLARED DEPRECATED legacy shim. It stays
//!   compiling because the desktop app `#[path]`-includes
//!   `model_manual/mod.rs` (it must stay free of `crate::*` dependencies) and
//!   because WP-KERNEL-005 atelier surfaces (`atelier/model_manual_merge.rs`)
//!   still consume it. Every legacy symbol/channel is mapped in
//!   [`migration_plan::naming_migration_plan`] and seeded as
//!   `user_manual_legacy_aliases` authority rows; the mapping is enforced by
//!   tests (`tests/user_manual_storage_tests.rs`), so legacy and canonical
//!   surfaces can never silently diverge (no split-brain docs).
//! * The static [`crate::model_manual::MODEL_MANUAL`] manifest is the SEED
//!   SOURCE for the legacy tool catalog: [`seed`] imports it into
//!   `user_manual_tool_entries` rows so PostgreSQL holds the single canonical
//!   manual; the in-binary manifest is a deterministic projection input, not
//!   a second authority.
//!
//! Authority: PostgreSQL (`user_manual_*` tables, migration 0310) +
//! EventLedger (`KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` receipts). Rendered
//! markdown/HTML are projections only.

pub mod bundle_bridge;
pub mod fixtures;
pub mod freshness;
pub mod migration_plan;
pub mod projection;
pub mod registry;
pub mod seed;
pub mod spec_seed;
pub mod store;

pub use migration_plan::{
    naming_migration_plan, LegacyAlias, LegacyKind, MigrationPhase, NamingMigrationPlan,
    PlanRow, ShimState,
};
pub use store::{
    ManualSearchHit, NewUserManualPage, UserManualAnchor, UserManualFeatureEntry,
    UserManualPage, UserManualSection, UserManualStore, UserManualToolEntry,
    UserManualVersionRow,
};

/// Canonical UserManual corpus version. Independent from the legacy
/// `model_manual::MANUAL_VERSION` (1.x line): the 2.x line marks the
/// UserManual era where PostgreSQL rows are authority. Bump on any seed
/// content change — the freshness check (MT-204) compares stored
/// `content_hash` per page, and `user_manual_versions` records each seeded
/// version.
pub const USER_MANUAL_VERSION: &str = "2.0.0";

/// The canonical stuck-together product term (operator decision; constraint in
/// every MT-193..MT-208 contract). Route namespace, slugs, and citations all
/// derive from this.
pub const CANONICAL_TERM: &str = "UserManual";

/// Root of the canonical HTTP namespace (`src/api/user_manual.rs`).
pub const ROUTE_NAMESPACE: &str = "/usermanual";
