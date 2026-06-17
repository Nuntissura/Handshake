//! MT-207 UserManualSpecEnrichmentSeed: typed Master Spec wording + appendix
//! seed rows for the future UserManual spec enrichment pass.
//!
//! These rows are the PREPARED enrichment, not the enrichment itself: the
//! Master Spec bundle is operator-gated (DEC-005 authorized v02.193; the next
//! bundle lands through the spec-update workflow, not through product code).
//! Each row names the exact target module + anchor and carries proposed
//! wording grounded in what MT-193..MT-208 actually built, so a no-context
//! enrichment pass lifts them without re-deriving the implementation. The
//! rows are runtime-discoverable (`GET /usermanual/spec-enrichment-seed`) and
//! the content test asserts every `target_anchor` exists in the CURRENT spec
//! bundle — a seed row pointing at a vanished anchor is a defect.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpecEnrichmentSeedRow {
    pub seed_id: &'static str,
    /// Spec module file within the bundle's `spec-modules/`.
    pub target_module: &'static str,
    /// The existing anchor the wording attaches under (must exist today).
    pub target_anchor: &'static str,
    /// `amend` (extend the section) | `appendix` (12.x appendix entry).
    pub seed_kind: &'static str,
    pub proposed_wording_md: &'static str,
}

pub fn spec_enrichment_seed() -> &'static [SpecEnrichmentSeedRow] {
    SEED_ROWS
}

const SEED_ROWS: &[SpecEnrichmentSeedRow] = &[
    SpecEnrichmentSeedRow {
        seed_id: "UMSPEC-001",
        target_module: "10-product-surfaces.md",
        target_anchor: "10.15.8 UserManual migration bridge",
        seed_kind: "amend",
        proposed_wording_md: "UserManualRecord authority is implemented as the `user_manual_*` \
            PostgreSQL tables (pages, sections, anchors, tool entries, feature entries, \
            versions, legacy aliases; migration 0310) seeded idempotently from a compiled-in \
            corpus with `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` receipts per changed page. The \
            canonical read surface is `/usermanual/*`; anonymous reads are permitted because \
            the manual is the no-context bootstrap surface, and every page read returns a \
            synthesized bootstrap receipt id so anonymous discovery stays auditable. The \
            legacy bridge (`GET /usermanual/legacy/model-manual`) answers legacy callers with \
            the mapped canonical payload and emits the 10.15.8 compatibility receipt; the \
            deterministic legacy mapping is queryable at `/usermanual/legacy/aliases` and the \
            phased rename plan at `/usermanual/migration-plan`.",
    },
    SpecEnrichmentSeedRow {
        seed_id: "UMSPEC-002",
        target_module: "10-product-surfaces.md",
        target_anchor: "10.15.8 UserManual migration bridge",
        seed_kind: "amend",
        proposed_wording_md: "UserManual freshness is a product surface: \
            `GET /usermanual/freshness` compares the stored rows against the compiled-in seed \
            corpus AND the declared WP-009 surface registry, returning typed verdicts \
            (`current`, `stale_content`, `missing_page`, `uncovered_surface`, \
            `dangling_anchor`, `unseeded_version`, `missing_tool_entry`, \
            `stale_tool_entry`, `missing_feature_entry`, `stale_feature_entry`, \
            `missing_legacy_alias`, `stale_legacy_alias`). The build-update rule is \
            test-enforced: a registry surface without an `http_route` anchor on a seeded page \
            fails the suite; tool entries, feature entries, and legacy aliases are checked \
            against the seed corpus; and the doc-vs-runtime tests probe every documented route \
            against the real router so the manual can never describe a surface the product does \
            not serve.",
    },
    SpecEnrichmentSeedRow {
        seed_id: "UMSPEC-003",
        target_module: "02-system-architecture.md",
        target_anchor: "2.3.13.11",
        seed_kind: "amend",
        proposed_wording_md: "Context bundles cite UserManual pages through `user_manual_page` \
            knowledge entities (`ref_kind = entity`); the persisted item citation is \
            `usermanual:<slug>@<manual_version>#<section-anchor>@0-0@<content-hash-prefix>` so \
            a consumer carries the page version, the section source anchor, and a drift-\
            detectable content hash in one citeable string.",
    },
    SpecEnrichmentSeedRow {
        seed_id: "UMSPEC-004",
        target_module: "12-end-of-file-appendices.md",
        target_anchor: "PRIM-UserManual",
        seed_kind: "appendix",
        proposed_wording_md: "`PRIM-UserManual` concrete shape (WP-KERNEL-009): \
            `user_manual_pages` (slug-keyed UserManualRecord rows, sha256 content hash, \
            version metadata), `user_manual_sections` (typed ordered sections: purpose / \
            workflows / startup / run_commands / inputs_outputs / navigation / safety / \
            failure_modes / recovery / hooks / examples / schema), `user_manual_anchors` \
            (http_route / tauri_command / ipc_channel / cli_command / spec_anchor / test / \
            event_type / primitive / page_link joins to real surfaces), \
            `user_manual_tool_entries` + `user_manual_feature_entries` (machine-readable \
            catalog incl. the imported legacy ModelManual manifest), `user_manual_versions`, \
            `user_manual_legacy_aliases`. In-app access points (editor, Notes, retrieval \
            debug, diagnostics, command palette) are a typed registry served at \
            `/usermanual/access-points` with stable `hs-usermanual-*` element ids.",
    },
    SpecEnrichmentSeedRow {
        seed_id: "UMSPEC-005",
        target_module: "07-user-experience-and-development.md",
        target_anchor: "7.1.1.9",
        seed_kind: "amend",
        proposed_wording_md: "The unified work surface exposes the UserManual contextually: \
            editor help, Notes help, retrieval-debug help, the diagnostics manual tab, and \
            command-palette commands all resolve `/usermanual/pages/:slug` projections \
            (HTML with stable `data-hs-manual-*` selectors). Quickstart bundles \
            (`GET /usermanual/quickstarts/:area` for index / editor / loom / retrieval / \
            validation / state-recovery) give a no-context model a task-sized operating set \
            without reading the whole manual.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn seed_rows_are_unique_and_typed() {
        let mut ids = BTreeSet::new();
        for row in spec_enrichment_seed() {
            assert!(ids.insert(row.seed_id), "dup seed id {}", row.seed_id);
            assert!(matches!(row.seed_kind, "amend" | "appendix"));
            assert!(row.target_module.ends_with(".md"));
            assert!(!row.target_anchor.trim().is_empty());
            assert!(
                row.proposed_wording_md.len() > 80,
                "wording too thin to lift"
            );
        }
    }
}
