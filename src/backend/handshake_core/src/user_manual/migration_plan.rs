//! MT-193 UserManualNamingMigrationPlan: the machine-readable, restartable
//! plan for renaming ModelManual / Model Manual / model_manual to UserManual.
//!
//! This is NOT prose. Every legacy surface is a typed [`PlanRow`] with its
//! canonical UserManual target, the migration phase that retires it, the shim
//! state it is in NOW, the lane that owns the rename, and the test that pins
//! the mapping. A no-context model continues the migration by filtering rows
//! on `shim_state != Migrated` for its phase/lane.
//!
//! Split-brain law (spec 10.15.8): a legacy path may exist ONLY while it maps
//! deterministically onto UserManual authority. The mapping here is seeded
//! into PostgreSQL (`user_manual_legacy_aliases`, migration 0310) by
//! [`crate::user_manual::seed`], and `tests/user_manual_storage_tests.rs`
//! enforces:
//! * every `*model_manual*` source file in this crate has a `PlanRow`
//!   (a NEW legacy file without a plan row fails the suite), and
//! * every legacy Tauri command / IPC channel has exactly one alias row whose
//!   canonical target resolves.

use serde::Serialize;

/// What kind of legacy surface a plan row describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyKind {
    /// A Rust module / source file carrying the legacy name.
    Module,
    /// A Tauri command symbol exposed to the desktop app.
    TauriCommand,
    /// A kernel IPC channel name.
    IpcChannel,
    /// A generated artifact (markdown projection, generator binary).
    Projection,
    /// A test target pinned to the legacy name.
    Test,
    /// A documentation/manifest constant (e.g. `MANUAL_VERSION`).
    Constant,
}

impl LegacyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::TauriCommand => "tauri_command",
            Self::IpcChannel => "ipc_channel",
            Self::Projection => "projection",
            Self::Test => "test",
            Self::Constant => "constant",
        }
    }
}

/// Where a legacy surface is in the rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShimState {
    /// Legacy path still answers callers, mapped 1:1 onto UserManual
    /// authority; compatibility receipts are emitted on the bridge route.
    ActiveShim,
    /// Legacy path is data only (seed source / generated artifact); the
    /// canonical surface is already UserManual.
    MappedSeedSource,
    /// Rename blocked on another lane (e.g. the desktop app `#[path]`
    /// include); tracked, not silent.
    PendingRename,
    /// Fully migrated; the legacy name no longer exists.
    Migrated,
}

impl ShimState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActiveShim => "active_shim",
            Self::MappedSeedSource => "mapped_seed_source",
            Self::PendingRename => "pending_rename",
            Self::Migrated => "migrated",
        }
    }
}

/// Which phase retires the row's legacy surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationPhase {
    /// THIS work packet (MT-193..MT-208): canonical `user_manual` module,
    /// PostgreSQL authority, aliases, receipts, freshness, fixtures.
    P1CanonicalAuthority,
    /// Frontend lane / follow-up: rename Tauri commands + app help surface to
    /// `/usermanual` routes; legacy command names become thin wrappers.
    P2AppSurfaceRename,
    /// Later WP: retire the static `model_manual` content module once the app
    /// include is migrated; physical file renames.
    P3FileRetirement,
}

impl MigrationPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P1CanonicalAuthority => "p1_canonical_authority",
            Self::P2AppSurfaceRename => "p2_app_surface_rename",
            Self::P3FileRetirement => "p3_file_retirement",
        }
    }
}

/// One legacy surface and its canonical UserManual target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PlanRow {
    /// Stable row id (`UMMIG-NNN`).
    pub row_id: &'static str,
    pub legacy_kind: LegacyKind,
    /// The legacy identifier exactly as it appears in code.
    pub legacy_id: &'static str,
    /// Repo-relative path of the legacy surface.
    pub legacy_path: &'static str,
    /// The canonical UserManual target (module path, route, table, or slug).
    pub canonical_ref: &'static str,
    pub phase: MigrationPhase,
    pub shim_state: ShimState,
    /// Which lane owns retiring this row (`backend` is this WP's lane).
    pub owner_lane: &'static str,
    /// The test that pins the mapping (file::test or file scope).
    pub migration_test: &'static str,
    /// Why the legacy surface cannot be deleted yet (empty when it can).
    pub blocker: &'static str,
}

/// A legacy callable name and the canonical surface it must resolve to.
/// Seeded into `user_manual_legacy_aliases` (migration 0310) so the mapping
/// is queryable authority, not source-only lore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LegacyAlias {
    /// The legacy callable (Tauri command, IPC channel, module symbol).
    pub alias: &'static str,
    pub alias_kind: LegacyKind,
    /// `page` | `tool` | `route` — what the canonical ref points at.
    pub canonical_kind: &'static str,
    /// Page slug, tool id, or route path.
    pub canonical_ref: &'static str,
    pub deprecation_note: &'static str,
}

/// The full MT-193 plan.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct NamingMigrationPlan {
    pub plan_id: &'static str,
    pub spec_anchor: &'static str,
    pub canonical_term: &'static str,
    pub rows: &'static [PlanRow],
    pub aliases: &'static [LegacyAlias],
}

/// The canonical, machine-readable naming migration plan.
pub fn naming_migration_plan() -> NamingMigrationPlan {
    NamingMigrationPlan {
        plan_id: "usermanual-naming-migration-v1",
        spec_anchor: "10.15.8 UserManual migration bridge [ADD v02.192]",
        canonical_term: super::CANONICAL_TERM,
        rows: PLAN_ROWS,
        aliases: LEGACY_ALIASES,
    }
}

const PLAN_ROWS: &[PlanRow] = &[
    PlanRow {
        row_id: "UMMIG-001",
        legacy_kind: LegacyKind::Module,
        legacy_id: "crate::model_manual (mod.rs/types.rs/content.rs/projection.rs)",
        legacy_path: "src/backend/handshake_core/src/model_manual/",
        canonical_ref: "crate::user_manual (PostgreSQL authority via user_manual::store)",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::MappedSeedSource,
        owner_lane: "backend_later_wp",
        migration_test: "tests/user_manual_storage_tests.rs::mt193_every_legacy_model_manual_file_is_plan_covered",
        blocker: "app/src-tauri/src/manual.rs #[path]-includes model_manual/mod.rs; the module must stay crate-dependency-free until the app reads /usermanual routes (P2)",
    },
    PlanRow {
        row_id: "UMMIG-002",
        legacy_kind: LegacyKind::Module,
        legacy_id: "crate::kernel::model_manual (KernelModelManualV1)",
        legacy_path: "src/backend/handshake_core/src/kernel/model_manual.rs",
        canonical_ref: "user_manual_pages rows seeded by user_manual::seed (kernel002 topics -> page sections)",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::MappedSeedSource,
        owner_lane: "backend_later_wp",
        migration_test: "tests/user_manual_content_tests.rs::mt196_kernel002_manual_topics_are_seeded_as_pages",
        blocker: "kernel/pre_use_kernel_acceptance_run.rs consumes the typed struct; retire after acceptance-run reads canonical rows",
    },
    PlanRow {
        row_id: "UMMIG-003",
        legacy_kind: LegacyKind::Module,
        legacy_id: "crate::atelier::model_manual_merge (merge + drift guard)",
        legacy_path: "src/backend/handshake_core/src/atelier/model_manual_merge.rs",
        canonical_ref: "user_manual::freshness (MT-204 verdicts over user_manual_* tables)",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::ActiveShim,
        owner_lane: "backend_later_wp",
        migration_test: "tests/atelier_model_manual_merge_tests.rs (existing, stays green during bridge)",
        blocker: "WP-KERNEL-005 atelier manual dataset still merges through it; fold into user_manual freshness after atelier rows are reseeded as user_manual_tool_entries",
    },
    PlanRow {
        row_id: "UMMIG-004",
        legacy_kind: LegacyKind::TauriCommand,
        legacy_id: "model_manual_get",
        legacy_path: "app/src-tauri/src/manual.rs",
        canonical_ref: "GET /usermanual/pages + GET /usermanual/legacy/model-manual (compat receipt)",
        phase: MigrationPhase::P2AppSurfaceRename,
        shim_state: ShimState::ActiveShim,
        owner_lane: "frontend",
        migration_test: "tests/user_manual_api_tests.rs::mt203_legacy_bridge_route_maps_and_emits_compat_receipt",
        blocker: "app/** is the concurrent frontend lane; backend exposes the canonical + bridge routes now",
    },
    PlanRow {
        row_id: "UMMIG-005",
        legacy_kind: LegacyKind::TauriCommand,
        legacy_id: "model_manual_list_commands",
        legacy_path: "app/src-tauri/src/manual.rs",
        canonical_ref: "GET /usermanual/tools",
        phase: MigrationPhase::P2AppSurfaceRename,
        shim_state: ShimState::ActiveShim,
        owner_lane: "frontend",
        migration_test: "tests/user_manual_api_tests.rs::mt201_tools_list_and_read_resolve",
        blocker: "app/** is the concurrent frontend lane",
    },
    PlanRow {
        row_id: "UMMIG-006",
        legacy_kind: LegacyKind::TauriCommand,
        legacy_id: "model_manual_search",
        legacy_path: "app/src-tauri/src/manual.rs",
        canonical_ref: "GET /usermanual/search",
        phase: MigrationPhase::P2AppSurfaceRename,
        shim_state: ShimState::ActiveShim,
        owner_lane: "frontend",
        migration_test: "tests/user_manual_api_tests.rs::mt201_search_finds_pages_and_tools",
        blocker: "app/** is the concurrent frontend lane",
    },
    PlanRow {
        row_id: "UMMIG-007",
        legacy_kind: LegacyKind::IpcChannel,
        legacy_id: "kernel.model_manual.get",
        legacy_path: "app/src-tauri/src/manual.rs",
        canonical_ref: "kernel.user_manual.get (P2) -> GET /usermanual/pages/:slug",
        phase: MigrationPhase::P2AppSurfaceRename,
        shim_state: ShimState::ActiveShim,
        owner_lane: "frontend",
        migration_test: "tests/user_manual_storage_tests.rs::mt193_every_legacy_alias_resolves_to_canonical",
        blocker: "channel constants live in the app include",
    },
    PlanRow {
        row_id: "UMMIG-008",
        legacy_kind: LegacyKind::IpcChannel,
        legacy_id: "kernel.model_manual.list_commands",
        legacy_path: "app/src-tauri/src/manual.rs",
        canonical_ref: "kernel.user_manual.list_commands (P2) -> GET /usermanual/tools",
        phase: MigrationPhase::P2AppSurfaceRename,
        shim_state: ShimState::ActiveShim,
        owner_lane: "frontend",
        migration_test: "tests/user_manual_storage_tests.rs::mt193_every_legacy_alias_resolves_to_canonical",
        blocker: "channel constants live in the app include",
    },
    PlanRow {
        row_id: "UMMIG-009",
        legacy_kind: LegacyKind::IpcChannel,
        legacy_id: "kernel.model_manual.search",
        legacy_path: "app/src-tauri/src/manual.rs",
        canonical_ref: "kernel.user_manual.search (P2) -> GET /usermanual/search",
        phase: MigrationPhase::P2AppSurfaceRename,
        shim_state: ShimState::ActiveShim,
        owner_lane: "frontend",
        migration_test: "tests/user_manual_storage_tests.rs::mt193_every_legacy_alias_resolves_to_canonical",
        blocker: "channel constants live in the app include",
    },
    PlanRow {
        row_id: "UMMIG-010",
        legacy_kind: LegacyKind::Projection,
        legacy_id: "app/MODEL_MANUAL.md (generated)",
        legacy_path: "app/MODEL_MANUAL.md",
        canonical_ref: "GET /usermanual/pages/:slug/projection?format=markdown|html",
        phase: MigrationPhase::P2AppSurfaceRename,
        shim_state: ShimState::MappedSeedSource,
        owner_lane: "frontend",
        migration_test: "tests/user_manual_api_tests.rs::mt205_projection_renders_readable_navigable_html",
        blocker: "generated artifact; regenerate from canonical rows once app reads /usermanual",
    },
    PlanRow {
        row_id: "UMMIG-011",
        legacy_kind: LegacyKind::Projection,
        legacy_id: "bin/model_manual_md_gen",
        legacy_path: "src/backend/handshake_core/src/bin/model_manual_md_gen.rs",
        canonical_ref: "user_manual::projection::render_page_markdown over PostgreSQL rows",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::ActiveShim,
        owner_lane: "backend_later_wp",
        migration_test: "tests/user_manual_api_tests.rs::mt205_projection_renders_readable_navigable_html",
        blocker: "operator tooling may still invoke the generator; retire with UMMIG-001",
    },
    PlanRow {
        row_id: "UMMIG-012",
        legacy_kind: LegacyKind::Test,
        legacy_id: "tests/model_manual_tests.rs",
        legacy_path: "src/backend/handshake_core/tests/model_manual_tests.rs",
        canonical_ref: "tests/user_manual_storage_tests.rs + tests/user_manual_content_tests.rs",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::ActiveShim,
        owner_lane: "backend_later_wp",
        migration_test: "tests/model_manual_tests.rs (stays green during bridge; retired with UMMIG-001)",
        blocker: "pins the legacy manifest invariants the seed import depends on",
    },
    PlanRow {
        row_id: "UMMIG-014",
        legacy_kind: LegacyKind::Module,
        legacy_id: "crate::kernel::dcc_kb003_model_manual_hints (DCC breadcrumb hints)",
        legacy_path: "src/backend/handshake_core/src/kernel/dcc_kb003_model_manual_hints.rs",
        canonical_ref: "rename to dcc_kb003_user_manual_hints once DCC IPC consumers migrate; hint CONTENT already names no legacy surface",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::PendingRename,
        owner_lane: "backend_later_wp",
        migration_test: "tests/user_manual_storage_tests.rs::mt193_every_legacy_model_manual_file_is_plan_covered",
        blocker: "WP-KERNEL-003 DCC projection consumers reference the module path; rename is mechanical but belongs with the UMMIG-001 file retirement pass",
    },
    PlanRow {
        row_id: "UMMIG-015",
        legacy_kind: LegacyKind::Test,
        legacy_id: "tests/atelier_model_manual_merge_tests.rs",
        legacy_path: "src/backend/handshake_core/tests/atelier_model_manual_merge_tests.rs",
        canonical_ref: "folds into user_manual freshness tests when UMMIG-003 retires the atelier merge module",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::ActiveShim,
        owner_lane: "backend_later_wp",
        migration_test: "tests/atelier_model_manual_merge_tests.rs (stays green during bridge)",
        blocker: "pins the WP-KERNEL-005 atelier merge + drift guard until UMMIG-003 lands",
    },
    PlanRow {
        row_id: "UMMIG-016",
        legacy_kind: LegacyKind::Test,
        legacy_id: "tests/kernel_model_manual_tests.rs",
        legacy_path: "src/backend/handshake_core/tests/kernel_model_manual_tests.rs",
        canonical_ref: "superseded by tests/user_manual_content_tests.rs::mt196_kernel002_manual_topics_are_seeded_as_pages once UMMIG-002 retires the kernel002 struct",
        phase: MigrationPhase::P3FileRetirement,
        shim_state: ShimState::ActiveShim,
        owner_lane: "backend_later_wp",
        migration_test: "tests/kernel_model_manual_tests.rs (stays green during bridge)",
        blocker: "pins the kernel002 manual struct the seed import maps from",
    },
    PlanRow {
        row_id: "UMMIG-013",
        legacy_kind: LegacyKind::Constant,
        legacy_id: "model_manual::MANUAL_VERSION (1.x line)",
        legacy_path: "src/backend/handshake_core/src/model_manual/mod.rs",
        canonical_ref: "user_manual::USER_MANUAL_VERSION (2.x line) + user_manual_versions rows",
        phase: MigrationPhase::P1CanonicalAuthority,
        shim_state: ShimState::ActiveShim,
        owner_lane: "backend",
        migration_test: "tests/user_manual_storage_tests.rs::mt194_seed_records_version_metadata",
        blocker: "atelier drift guard (HBR-MAN-001) still keys on the 1.x version for the legacy dataset",
    },
];

const LEGACY_ALIASES: &[LegacyAlias] = &[
    LegacyAlias {
        alias: "model_manual_get",
        alias_kind: LegacyKind::TauriCommand,
        canonical_kind: "route",
        canonical_ref: "/usermanual/legacy/model-manual",
        deprecation_note: "Deprecated since UserManual 2.0.0: use GET /usermanual/pages (list) and GET /usermanual/pages/:slug (read); the legacy bridge route returns the mapped payload and emits a compatibility receipt (spec 10.15.8).",
    },
    LegacyAlias {
        alias: "model_manual_list_commands",
        alias_kind: LegacyKind::TauriCommand,
        canonical_kind: "route",
        canonical_ref: "/usermanual/tools",
        deprecation_note: "Deprecated since UserManual 2.0.0: use GET /usermanual/tools.",
    },
    LegacyAlias {
        alias: "model_manual_search",
        alias_kind: LegacyKind::TauriCommand,
        canonical_kind: "route",
        canonical_ref: "/usermanual/search",
        deprecation_note: "Deprecated since UserManual 2.0.0: use GET /usermanual/search?q=.",
    },
    LegacyAlias {
        alias: "kernel.model_manual.get",
        alias_kind: LegacyKind::IpcChannel,
        canonical_kind: "route",
        canonical_ref: "/usermanual/legacy/model-manual",
        deprecation_note: "Deprecated since UserManual 2.0.0: canonical IPC name is kernel.user_manual.get (P2); backend canonical surface is /usermanual.",
    },
    LegacyAlias {
        alias: "kernel.model_manual.list_commands",
        alias_kind: LegacyKind::IpcChannel,
        canonical_kind: "route",
        canonical_ref: "/usermanual/tools",
        deprecation_note: "Deprecated since UserManual 2.0.0: canonical IPC name is kernel.user_manual.list_commands (P2).",
    },
    LegacyAlias {
        alias: "kernel.model_manual.search",
        alias_kind: LegacyKind::IpcChannel,
        canonical_kind: "route",
        canonical_ref: "/usermanual/search",
        deprecation_note: "Deprecated since UserManual 2.0.0: canonical IPC name is kernel.user_manual.search (P2).",
    },
    LegacyAlias {
        alias: "kernel002-no-context-model-manual-v1",
        alias_kind: LegacyKind::Module,
        canonical_kind: "page",
        canonical_ref: "kernel-write-governance",
        deprecation_note: "Deprecated since UserManual 2.0.0: the kernel002 no-context manual topics are seeded as the kernel-write-governance UserManual page.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn plan_row_ids_are_unique_and_fields_populated() {
        let plan = naming_migration_plan();
        let mut ids = BTreeSet::new();
        for row in plan.rows {
            assert!(ids.insert(row.row_id), "duplicate plan row {}", row.row_id);
            assert!(!row.legacy_id.trim().is_empty());
            assert!(!row.legacy_path.trim().is_empty());
            assert!(!row.canonical_ref.trim().is_empty());
            assert!(!row.migration_test.trim().is_empty());
            // A non-migrated row MUST name its blocker (no silent shims).
            if row.shim_state != ShimState::Migrated {
                assert!(
                    !row.blocker.trim().is_empty(),
                    "{} is a live shim without a named blocker",
                    row.row_id
                );
            }
        }
    }

    #[test]
    fn aliases_are_unique_and_carry_deprecation_notes() {
        let plan = naming_migration_plan();
        let mut names = BTreeSet::new();
        for alias in plan.aliases {
            assert!(names.insert(alias.alias), "duplicate alias {}", alias.alias);
            assert!(alias.deprecation_note.contains("Deprecated since UserManual"));
            assert!(matches!(alias.canonical_kind, "page" | "tool" | "route"));
            assert!(!alias.canonical_ref.trim().is_empty());
        }
    }

    #[test]
    fn legacy_tauri_and_ipc_surfaces_each_have_an_alias() {
        let plan = naming_migration_plan();
        let alias_names: BTreeSet<_> = plan.aliases.iter().map(|a| a.alias).collect();
        for expected in [
            "model_manual_get",
            "model_manual_list_commands",
            "model_manual_search",
            "kernel.model_manual.get",
            "kernel.model_manual.list_commands",
            "kernel.model_manual.search",
        ] {
            assert!(alias_names.contains(expected), "missing alias {expected}");
        }
    }
}
