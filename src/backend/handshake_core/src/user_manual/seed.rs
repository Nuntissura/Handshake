//! The compiled-in UserManual seed corpus and the idempotent seeder.
//!
//! * MT-196 UserManualPurposeAndWorkflowPages — purpose, core workflows,
//!   startup/run commands, expected inputs/outputs, navigation paths.
//! * MT-197 UserManualToolPrimitiveCatalog — tools, primitives, APIs, backend
//!   navigation commands, visual-debug surfaces, safe usage (tool entries are
//!   generated from [`registry::wp009_surface_registry`] + the legacy static
//!   manifest so the catalog can never drift from the declared inventory).
//! * MT-198 UserManualFailureRecoveryPages — common failures, diagnostics,
//!   recovery steps, repair queues, stale state, and embedded-store behavior.
//! * MT-199 UserManualModelQuickstartBundles — per-area quickstart pages.
//! * MT-206 UserManualStateRecoveryGuide — session compaction, interrupted
//!   MTs, failed builds, validation reentry.
//!
//! ACCURACY IS LAW: every command, route, header, error code, permission
//! decision, and port documented here is exercised by the doc-vs-runtime
//! consistency tests (`tests/user_manual_content_tests.rs`,
//! `tests/user_manual_api_tests.rs`). A seed claim the product does not
//! honor is a test failure, not a doc nit.
//!
//! Seeding is idempotent: pages/tools short-circuit on content hash, receipts
//! (`KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED`) are appended only for changed
//! rows, and the corpus version lands in `user_manual_versions`.

use serde_json::json;

use super::migration_plan::naming_migration_plan;
use super::registry::{user_manual_access_points, wp009_surface_registry, SurfaceGroup};
use super::store::{
    sha256_hex, LegacyAliasRow, NewManualAnchor, NewManualSection, NewUserManualPage,
    UserManualFeatureEntry, UserManualStore, UserManualToolEntry,
};
use super::USER_MANUAL_VERSION;
use crate::kernel::model_manual::kernel002_no_context_model_manual;
use crate::model_manual::{model_manual, CommandStatus};
use crate::storage::surreal::SurrealDatabase;
use crate::storage::StorageResult;

/// Everything the seeder writes.
pub struct SeedCorpus {
    pub pages: Vec<NewUserManualPage>,
    pub tools: Vec<UserManualToolEntry>,
    pub features: Vec<UserManualFeatureEntry>,
    pub aliases: Vec<LegacyAliasRow>,
}

/// What one `ensure_seeded` run changed.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SeedReport {
    pub manual_version: String,
    pub seed_content_hash: String,
    pub pages_total: usize,
    pub pages_changed: usize,
    pub tools_total: usize,
    pub tools_changed: usize,
    pub features_total: usize,
    pub features_changed: usize,
    pub aliases_total: usize,
    pub aliases_changed: usize,
    pub version_receipt_event_id: Option<String>,
}

fn section(kind: &'static str, title: &str, body_md: &str) -> NewManualSection {
    NewManualSection {
        section_kind: kind,
        title: title.to_string(),
        body_md: body_md.to_string(),
        body_json: None,
    }
}

fn section_with_json(
    kind: &'static str,
    title: &str,
    body_md: &str,
    body_json: serde_json::Value,
) -> NewManualSection {
    NewManualSection {
        section_kind: kind,
        title: title.to_string(),
        body_md: body_md.to_string(),
        body_json: Some(body_json),
    }
}

fn route_anchor(method: &'static str, value: &str) -> NewManualAnchor {
    NewManualAnchor {
        anchor_kind: "http_route",
        anchor_value: value.to_string(),
        http_method: method,
    }
}

fn page_link(slug: &str) -> NewManualAnchor {
    NewManualAnchor {
        anchor_kind: "page_link",
        anchor_value: slug.to_string(),
        http_method: "",
    }
}

fn spec_anchor(value: &str) -> NewManualAnchor {
    NewManualAnchor {
        anchor_kind: "spec_anchor",
        anchor_value: value.to_string(),
        http_method: "",
    }
}

/// Route anchors for every registry surface in a group — the MT-195 coverage
/// gate joins these against the registry, so group pages stay complete by
/// construction.
fn group_route_anchors(group: SurfaceGroup) -> Vec<NewManualAnchor> {
    wp009_surface_registry()
        .iter()
        .filter(|s| s.group == group)
        .map(|s| route_anchor(s.method, s.route))
        .collect()
}

fn group_routes_md(group: SurfaceGroup) -> String {
    let mut out = String::new();
    for s in wp009_surface_registry().iter().filter(|s| s.group == group) {
        out.push_str(&format!("- `{} {}` — {}\n", s.method, s.route, s.summary));
    }
    out
}

// ---------------------------------------------------------------------------
// The corpus.
// ---------------------------------------------------------------------------

pub fn seed_corpus() -> SeedCorpus {
    SeedCorpus {
        pages: seed_pages(),
        tools: seed_tool_entries(),
        features: seed_feature_entries(),
        aliases: seed_aliases(),
    }
}

/// Stable hash over the full corpus (version metadata + MT-204 freshness).
pub fn corpus_hash(corpus: &SeedCorpus) -> String {
    let page_hashes: Vec<String> = corpus.pages.iter().map(|p| p.content_hash()).collect();
    let tool_hashes: Vec<&str> = corpus
        .tools
        .iter()
        .map(|t| t.content_hash.as_str())
        .collect();
    let feature_hashes: Vec<&str> = corpus
        .features
        .iter()
        .map(|f| f.content_hash.as_str())
        .collect();
    sha256_hex(
        &serde_json::to_string(&json!({
            "manual_version": USER_MANUAL_VERSION,
            "pages": page_hashes,
            "tools": tool_hashes,
            "features": feature_hashes,
            "aliases": corpus.aliases,
        }))
        .expect("corpus serializes"),
    )
}

fn seed_pages() -> Vec<NewUserManualPage> {
    let mut pages = vec![
        page_manual_toc(),
        page_product_purpose(),
        page_core_workflows(),
        page_startup_and_run_commands(),
        page_backend_navigation_and_identity(),
        page_permissions_and_safety(),
        page_atelier_storage_authority(),
        page_knowledge_index_surface(),
        page_notes_loom_surface(),
        page_rich_documents_surface(),
        page_retrieval_surface(),
        page_memory_surface(),
        page_crdt_surface(),
        page_usermanual_surface(),
        page_failure_modes_and_recovery(),
        page_repair_queues_and_staleness(),
        page_embedded_store_recovery(),
        page_surreal_swarm_concurrency_and_load(),
        page_state_recovery_guide(),
        page_kernel_write_governance(),
        page_legacy_bridge(),
    ];
    pages.extend(quickstart_pages());
    pages
}

fn page_manual_toc() -> NewUserManualPage {
    let all_slugs = [
        "handshake-product-purpose",
        "core-workflows",
        "startup-and-run-commands",
        "backend-navigation-and-identity",
        "permissions-and-safety",
        "atelier-storage-authority",
        "knowledge-index-surface",
        "notes-loom-surface",
        "rich-documents-surface",
        "retrieval-and-context-bundles-surface",
        "memory-and-claims-surface",
        "crdt-collaboration-surface",
        "usermanual-surface",
        "failure-modes-and-recovery",
        "repair-queues-and-staleness",
        "embedded-store-recovery",
        "surreal-swarm-concurrency-and-load",
        "state-recovery-guide",
        "kernel-write-governance",
        "legacy-model-manual-bridge",
        "quickstart-index",
        "quickstart-editor",
        "quickstart-loom",
        "quickstart-retrieval",
        "quickstart-validation",
        "quickstart-state-recovery",
    ];
    let mut anchors: Vec<NewManualAnchor> = all_slugs.iter().map(|s| page_link(s)).collect();
    anchors.push(route_anchor("GET", "/usermanual/pages"));
    NewUserManualPage {
        slug: "manual-toc".into(),
        title: "UserManual — Table of Contents".into(),
        page_kind: "navigation",
        audience: "model_and_operator",
        spec_anchors: vec!["10.15.8".into()],
        sections: vec![
            section(
                "navigation",
                "How to use this manual",
                "This is the Handshake UserManual: the built-in, no-context operating manual for \
                 models and operators. Every page is an embedded SurrealDB authority row served over \
                 `GET /usermanual/pages/:slug`. Start here with no prior context:\n\n\
                 1. `GET /usermanual/pages` — list all pages.\n\
                 2. `GET /usermanual/pages/handshake-product-purpose` — what Handshake is.\n\
                 3. `GET /usermanual/pages/startup-and-run-commands` — how to start and probe the product.\n\
                 4. `GET /usermanual/quickstarts/:area` — task-sized bundles \
                 (`index`, `editor`, `loom`, `retrieval`, `validation`, `state-recovery`).\n\
                 5. `GET /usermanual/search?q=<term>` — search pages, sections, and the tool catalog.\n\n\
                 Every page listed below is reachable from this TOC (the visual-navigation \
                 fixture asserts this; an orphan page is a defect).",
            ),
            section_with_json(
                "navigation",
                "All pages",
                &all_slugs
                    .iter()
                    .map(|s| format!("- [[{s}]]\n"))
                    .collect::<String>(),
                json!({ "pages": all_slugs }),
            ),
        ],
        anchors,
    }
}

fn page_product_purpose() -> NewUserManualPage {
    NewUserManualPage {
        slug: "handshake-product-purpose".into(),
        title: "Handshake — Product Purpose".into(),
        page_kind: "purpose",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.11".into(), "7.1.1.9".into(), "10.15.8".into()],
        sections: vec![
            section(
                "purpose",
                "What Handshake is",
                "Handshake is a local-first creative + execution workbench where operators and \
                 models co-author work over ONE authority substrate: embedded SurrealDB plus the \
                 EventLedger. WP-KERNEL-009 adds the Project Knowledge Index (typed knowledge \
                 about a project's sources, code symbols, claims, and media), a Tiptap/ProseMirror \
                 rich document editor with embedded Monaco code nodes, the Notes surface \
                 (operator name for the Loom engine: backlinks, graphs, tags, folders, wiki \
                 projections — an Obsidian-class replacement), retrieval with explainable \
                 context bundles, and this UserManual.\n\n\
                 The unified work surface law (spec 7.1.1.9): Notes + Loom engine + project wiki \
                 + rich editor are ONE surface over one substrate — operators work the GUI, \
                 models work the backend HTTP APIs documented here, and both observe the same \
                 canonical state.",
            ),
            section(
                "purpose",
                "Authority model",
                "Embedded SurrealDB + EventLedger is canonical for durable state, receipts, indexing \
                 evidence, and validation. Generated markdown, wiki pages, HTML exports, context \
                 bundles, debug reports, and UI projections are PROJECTIONS — useful, never \
                 authority. There is no alternate local database, no Docker dependency, and no database daemon: \
                 Handshake opens its own embedded SurrealDB store (see \
                 [[embedded-store-recovery]]).",
            ),
            section(
                "navigation",
                "Where to go next",
                "- Operate the product: [[startup-and-run-commands]]\n\
                 - Call backend APIs: [[backend-navigation-and-identity]]\n\
                 - Task-sized intros: [[quickstart-index]], [[quickstart-editor]], \
                 [[quickstart-loom]], [[quickstart-retrieval]]\n\
                 - When something breaks: [[failure-modes-and-recovery]]",
            ),
        ],
        anchors: vec![
            page_link("startup-and-run-commands"),
            page_link("backend-navigation-and-identity"),
            page_link("embedded-store-recovery"),
            page_link("failure-modes-and-recovery"),
            spec_anchor("2.3.13.11"),
            spec_anchor("7.1.1.9"),
            spec_anchor("10.15.8"),
        ],
    }
}

fn page_core_workflows() -> NewUserManualPage {
    NewUserManualPage {
        slug: "core-workflows".into(),
        title: "Core Workflows".into(),
        page_kind: "workflow",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.11".into(), "10.20".into()],
        sections: vec![
            section(
                "workflows",
                "Index a project",
                "1. Register/inspect roots: `GET /knowledge/ingestion/roots`.\n\
                 2. Start a run: `POST /knowledge/ingestion/runs` (identity headers required) — \
                 emits `KNOWLEDGE_INDEX_RUN_STARTED/COMPLETED/FAILED` receipts.\n\
                 3. Inspect extraction receipts per source: \
                 `GET /knowledge/ingestion/sources/:source_id/receipts`.\n\
                 4. Failed/partial extractions queue in `GET /knowledge/ingestion/repairs`; \
                 retry one with `POST /knowledge/ingestion/repairs/:repair_id/retry`.",
            ),
            section(
                "workflows",
                "Navigate indexed code (no external LSP)",
                "1. `GET /knowledge/code/symbols?workspace_id=&name=` — find a symbol.\n\
                 2. `GET /knowledge/code/symbols/:entity_id` — definition span + staleness \
                 verdict (`fresh` / `marked_stale` / fail-closed `unknown`; stale is FLAGGED, \
                 never served silently).\n\
                 3. `/references`, `/tests`, `/spans` sub-routes — callers/callees, validating \
                 tests, and the source-span citations behind every answer.\n\
                 4. `GET /knowledge/code/files/:path/lens` — the Monaco code-lens payload.",
            ),
            section(
                "workflows",
                "Author a rich document",
                "1. `POST /knowledge/documents` {workspace_id, title, content_json} — creates the \
                 authority row (doc_version 1).\n\
                 2. `PUT /knowledge/documents/:id/save` {expected_version, content_json} — \
                 optimistic concurrency; a stale expected_version returns 409 `conflict` (reload \
                 then merge, never blind-overwrite).\n\
                 3. `GET /knowledge/documents/:id/history?limit=&offset=` — paginated append-only \
                 revisions.\n\
                 4. `GET /knowledge/documents/:id/projection?format=markdown|html|plain_text|wiki_loom|context_bundle` \
                 — projections of the authority row.\n\
                 5. Import external content: `POST /knowledge/documents/import` \
                 (markdown | plain_text | html; HTML is sanitized fail-closed and unconvertible \
                 fragments land as typed `ImportedRaw` blocks, never silently dropped).",
            ),
            section(
                "workflows",
                "Work the Notes (Loom) surface",
                "Create blocks (`POST /workspaces/:ws/loom/blocks`), link them \
                 (`POST .../loom/edges`), then navigate: backlinks with context, unlinked \
                 mentions, breadcrumbs, tag hubs, folders with color labels, pinned grids, \
                 local/global graph views, bounded traversal, and full-text search. Compile a \
                 project wiki projection (`POST .../loom/wiki`) and regenerate it when stale. \
                 See [[notes-loom-surface]] for the full route list.",
            ),
            section(
                "workflows",
                "Retrieve cited context",
                "Compiled context bundles are bounded, cited, explainable, and replayable. Load a \
                 bundle (`GET /knowledge/retrieval/bundles/:bundle_id`), check its staleness \
                 verdict (`.../staleness` — per-item `ok` / missing-evidence / `source_stale`), \
                 repair a stale bundle (`POST .../repair`), and export the AI-ready evidence \
                 manifest (`.../export`).",
            ),
            section(
                "workflows",
                "Operate this manual",
                "List pages, read a page, follow its `page_link` anchors, search, and pull \
                 quickstart bundles — see [[usermanual-surface]]. The manual's freshness against \
                 the live surface registry is itself a product surface: \
                 `GET /usermanual/freshness`.",
            ),
        ],
        anchors: vec![
            page_link("notes-loom-surface"),
            page_link("usermanual-surface"),
            page_link("knowledge-index-surface"),
            page_link("rich-documents-surface"),
            page_link("retrieval-and-context-bundles-surface"),
            route_anchor("POST", "/knowledge/ingestion/runs"),
            route_anchor("GET", "/knowledge/code/symbols"),
            route_anchor("POST", "/knowledge/documents"),
            route_anchor("GET", "/usermanual/freshness"),
        ],
    }
}

fn page_startup_and_run_commands() -> NewUserManualPage {
    NewUserManualPage {
        slug: "startup-and-run-commands".into(),
        title: "Startup And Run Commands".into(),
        page_kind: "workflow",
        audience: "model_and_operator",
        spec_anchors: vec!["10.15.8".into()],
        sections: vec![
            section_with_json(
                "startup",
                "Start the backend",
                "From the repo root:\n\n\
                 ```\n\
                 cargo run -p handshake_core --bin handshake_core --features app-runtime\n\
                 ```\n\n\
                 The server binds `127.0.0.1:37501` and mounts every API both at `/` and under \
                 `/api` (e.g. `/usermanual/pages` and `/api/usermanual/pages` are the same \
                 surface). On startup Handshake opens its embedded SurrealDB store from \
                 `HANDSHAKE_DATA_DIR` when configured, otherwise from the platform-local application \
                 data directory. It starts no database server or daemon. Quiet by design: no \
                 foreground window is popped.",
                json!({
                    "run_command": "cargo run -p handshake_core --bin handshake_core --features app-runtime",
                    "listen_addr": "127.0.0.1:37501",
                    "api_mounts": ["/", "/api"],
                    "database_engine": "embedded_surrealdb_rocksdb",
                    "data_dir_override": "HANDSHAKE_DATA_DIR"
                }),
            ),
            section(
                "run_commands",
                "Probe health",
                "```\ncurl http://127.0.0.1:37501/health\n```\n\n\
                 `GET /health` answers when the server is up. If it does not answer, see \
                 [[embedded-store-recovery]] and [[state-recovery-guide]].",
            ),
            section(
                "run_commands",
                "Run scoped tests (the validation path)",
                "Always run SCOPED test targets, one cargo invocation at a time — never the full \
                 suite in shared worktrees:\n\n\
                 ```\n\
                 cargo test -p handshake_core --features test-utils --test user_manual_api_tests\n\
                 cargo test -p handshake_core --features test-utils --test knowledge_code_nav_api_tests\n\
                 cargo test -p handshake_core --lib user_manual\n\
                 ```\n\n\
                 The migrated MT-136 storage proofs and Surreal-backed fixture tests allocate an \
                 isolated store under `HANDSHAKE_ARTIFACTS_ROOT` and exercise the real embedded \
                 SurrealDB/RocksDB engine. They fail hard when the store cannot open; these scoped \
                 proofs have no alternate local database, in-memory, server, or mock fallback. Other integration-test \
                 targets may still be pending migration and must be inspected before use.",
            ),
            section(
                "inputs_outputs",
                "What every API speaks",
                "JSON in, JSON out (except asset content/thumbnail bytes). Errors are typed \
                 envelopes `{\"error\": \"<code>\", ...}` — never bare 500 strings. Knowledge \
                 surfaces additionally require identity headers; see \
                 [[backend-navigation-and-identity]].",
            ),
        ],
        anchors: vec![
            page_link("embedded-store-recovery"),
            page_link("state-recovery-guide"),
            page_link("backend-navigation-and-identity"),
            NewManualAnchor {
                anchor_kind: "cli_command",
                anchor_value: "cargo run -p handshake_core --bin handshake_core --features app-runtime"
                    .into(),
                http_method: "",
            },
            NewManualAnchor {
                anchor_kind: "cli_command",
                anchor_value: "cargo test -p handshake_core --features test-utils --test user_manual_api_tests"
                    .into(),
                http_method: "",
            },
        ],
    }
}

fn page_backend_navigation_and_identity() -> NewUserManualPage {
    NewUserManualPage {
        slug: "backend-navigation-and-identity".into(),
        title: "Backend Navigation And Identity Headers".into(),
        page_kind: "navigation",
        audience: "model",
        spec_anchors: vec!["2.3.13.11".into(), "10.20".into()],
        sections: vec![
            section_with_json(
                "navigation",
                "The identity header contract",
                "Backend navigation is attributable (spec 2.3.13.11): knowledge surfaces REQUIRE \
                 these headers and answer 400 `bad_request` without them:\n\n\
                 - `x-hsk-actor-id` — who acts\n\
                 - `x-hsk-kernel-task-run-id` — the kernel task run\n\
                 - `x-hsk-session-run-id` — the session run\n\n\
                 Optional:\n\n\
                 - `x-hsk-actor-kind` — see [[permissions-and-safety]] (defaults are \
                 surface-specific and FAIL CLOSED)\n\
                 - `x-hsk-correlation-id` — correlation chain\n\n\
                 Reads leave `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` receipts; writes leave their \
                 own typed receipts. The UserManual and Notes/Loom read surfaces accept \
                 anonymous calls (the manual is the bootstrap surface — it must be readable \
                 before identity is known); manual page reads synthesize and RETURN a bootstrap \
                 receipt so even anonymous discovery is auditable.",
                json!({
                    "required_headers": ["x-hsk-actor-id", "x-hsk-kernel-task-run-id", "x-hsk-session-run-id"],
                    "optional_headers": ["x-hsk-actor-kind", "x-hsk-correlation-id"],
                    "header_required_groups": ["knowledge_ingestion", "code_navigation", "rich_documents", "retrieval", "memory_claims", "crdt_collaboration"],
                    "anonymous_read_groups": ["notes_loom", "user_manual"]
                }),
            ),
            section(
                "navigation",
                "Route namespaces",
                "- `/knowledge/ingestion/*` — source roots, runs, receipts, repairs\n\
                 - `/knowledge/code/*` — symbol/code navigation (no external LSP)\n\
                 - `/knowledge/documents/*` — rich document authority\n\
                 - `/knowledge/retrieval/*` — context bundles + staleness + repair\n\
                 - `/knowledge/memory/*` — claims, facts, conflicts, neighborhood\n\
                 - `/knowledge/crdt/*` — draft sync (push/pull/conflict state)\n\
                 - `/workspaces/:ws/loom/*` + `/workspaces/:ws/assets/*` — Notes/Loom\n\
                 - `/usermanual/*` — this manual\n\n\
                 Everything is also mounted under `/api/...`. The complete machine-readable \
                 inventory: `GET /usermanual/tools` (every row carries method, route, expected \
                 input/output, errors, recovery).",
            ),
            section(
                "hooks",
                "Visual-debug hooks",
                "Diagnostics surfaces expose structured state for no-context models: \
                 `GET /knowledge/memory/visual-debug` (memory state projection) and the manual's \
                 HTML projection (`GET /usermanual/pages/:slug/projection?format=html`) with \
                 stable `data-hs-manual-*` selectors for DOM-level assertions.",
            ),
        ],
        anchors: {
            let mut a = vec![
                page_link("permissions-and-safety"),
                route_anchor("GET", "/usermanual/tools"),
                route_anchor("GET", "/knowledge/memory/visual-debug"),
            ];
            a.push(spec_anchor("2.3.13.11"));
            a
        },
    }
}

fn page_permissions_and_safety() -> NewUserManualPage {
    NewUserManualPage {
        slug: "permissions-and-safety".into(),
        title: "Permissions And Safety Constraints".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.11".into(), "10.15.8".into()],
        sections: vec![
            section_with_json(
                "safety",
                "Document actor kinds (rich documents)",
                "`x-hsk-actor-kind` on `/knowledge/documents/*` uses this vocabulary and is \
                 decided SERVER-SIDE per action (read / write / index):\n\n\
                 | actor kind | read | write | index |\n\
                 |---|---|---|---|\n\
                 | `operator` | yes | yes | yes |\n\
                 | `system` | yes | yes | yes |\n\
                 | `local_model` | yes | yes | yes |\n\
                 | `cloud_model` | yes | **DENIED** | yes |\n\
                 | `validator` | yes | DENIED | DENIED |\n\
                 | `unauthenticated` (absent header) | yes | DENIED | DENIED |\n\n\
                 Fail-closed rules: an ABSENT actor kind is the least-privileged \
                 `unauthenticated` actor (read-only); an UNKNOWN token is a 400 — privilege is \
                 asserted explicitly and validated, never inferred. Denials are 403 `forbidden` \
                 with a stable reason code (e.g. `cloud_model_write_denied`, \
                 `unauthenticated_write_denied`).",
                json!({
                    "actor_kinds": ["operator", "local_model", "cloud_model", "validator", "system", "unauthenticated"],
                    "decisions": {
                        "operator": {"read": true, "write": true, "index": true},
                        "system": {"read": true, "write": true, "index": true},
                        "local_model": {"read": true, "write": true, "index": true},
                        "cloud_model": {"read": true, "write": false, "index": true},
                        "validator": {"read": true, "write": false, "index": false},
                        "unauthenticated": {"read": true, "write": false, "index": false}
                    }
                }),
            ),
            section(
                "safety",
                "Safety constraints",
                "- Never treat projections (markdown exports, wiki pages, UI state, this page's \
                 rendered HTML) as authority; authority is the embedded SurrealDB row + EventLedger \
                 receipt.\n\
                 - Never invent write paths: if no documented route performs the mutation, stop \
                 and record the gap; do not poke tables directly.\n\
                 - Embeds in rich documents obey the embed-target law: artifact/media/source ids \
                 or http(s) URLs only — absolute filesystem paths and script-bearing URIs are \
                 rejected at construction (`empty`, `absolute path`, `non-http url`, `scheme not \
                 allowed for id` errors).\n\
                 - HTML import is sanitized fail-closed; unconvertible content becomes typed \
                 `ImportedRaw` blocks.\n\
                 - The UserManual resync surface (`POST /usermanual/resync`) is write-gated: \
                 `cloud_model` and `unauthenticated` are DENIED (403) — manual content comes \
                 from the compiled-in seed, so manual text can never be injected at runtime by \
                 an unprivileged caller.\n\
                 - List reads are bounded (caps around 500 rows); pagination is explicit \
                 (`limit`/`offset`) — never assume a list is the whole canonical set.",
            ),
        ],
        anchors: vec![
            page_link("rich-documents-surface"),
            route_anchor("POST", "/usermanual/resync"),
            spec_anchor("10.15.8"),
        ],
    }
}

/// WP-KERNEL-012 MT-138: the no-context storage and restart contract for the
/// Atelier domain after its embedded-store port.
fn page_atelier_storage_authority() -> NewUserManualPage {
    NewUserManualPage {
        slug: "atelier-storage-authority".into(),
        title: "Atelier — Embedded Storage Authority".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.0".into(), "6.0.0".into()],
        sections: vec![
            section(
                "purpose",
                "Atelier storage authority after the port",
                "Atelier reads and writes Handshake's single embedded SurrealDB authority through \
                 `AtelierStore`. There is no legacy server database, alternate local database, in-memory, mock, compatibility, or \
                 fallback database path. Domain mutations and their EventLedger evidence use the \
                 same embedded authority boundary.",
            ),
            section_with_json(
                "workflows",
                "Startup and restart behavior",
                "Before the backend serves routes, the global Surreal bootstrap verifies the \
                 compiled schema lineage and fingerprint. `AtelierStore::bootstrap_schema` then \
                 reuses a complete Atelier projection, applies the canonical bounded projection \
                 only when the Atelier surface is wholly absent, and refuses partial or full-count \
                 catalog divergence. Inspection and canonical DDL run under one shared bootstrap \
                 mutex, and DDL applies in one transaction. Exact-complete reuse verifies canonical \
                 fields, indexes, and events, not only table names. Its `AtelierStore::ensure_schema` \
                 readiness check also verifies every canonical `atelier_*` table and fails startup \
                 closed if any table is missing. After readiness succeeds, \
                 `bootstrap_builtin_command_corpus` projects the built-in CKC command corpus into \
                 the same store and removes obsolete builtin descriptors and blocked records. An \
                 unchanged restart performs no corpus mutation, timestamp refresh, or event append. \
                 Restarting reopens the same data directory, repeats both gates, and preserves \
                 existing Atelier rows.",
                json!({
                    "authority": "embedded_surrealdb",
                    "schema_bootstrap": "AtelierStore::bootstrap_schema",
                    "schema_atomicity": "shared_mutex_and_single_transaction",
                    "catalog_reuse_gate": ["tables", "fields", "indexes", "events"],
                    "readiness_gate": "AtelierStore::ensure_schema",
                    "startup_projection": "AtelierStore::bootstrap_builtin_command_corpus",
                    "obsolete_builtin_policy": "remove_descriptor_and_blocked_records",
                    "unchanged_restart_writes": 0,
                    "fallbacks": [],
                    "failure_mode": "fail_closed_before_routes"
                }),
            ),
            section_with_json(
                "schema",
                "Where the SurrealDB schema comes from",
                "`storage/surreal/schema.surql` is the sole declarative schema authority. Startup \
                 proves its exact bytes against a pinned SHA-256, parses the same source into a \
                 sorted semantic catalog, and verifies exact identities and counts for 282 tables, \
                 3,320 authored fields, 795 indexes, 19 events, two views, and two sequences. The \
                 live catalog is then read from the pinned SurrealDB 3.2 engine and compared with an \
                 exact structured fingerprint. A fresh embedded RocksDB bootstrap, adversarial \
                 catalog mutations, shutdown, reopen, and unchanged-fingerprint checks prove that \
                 source, applied state, and restarted state agree. Unknown, missing, or redefined \
                 objects fail closed; startup never substitutes another database or prunes ordinary \
                 application records. The only mutating reuse paths are two exact hash-allowlisted \
                 predecessors. For the retired-registry lineage, startup transactionally rewrites the 61 historically registered rows \
                 from the retired `migration_file` field to `schema_source`, adds the current-only \
                 0343 support-table registry row and the MT-142 `knowledge_rich_document_title_anchors` \
                 table with its registry row for a final exact 63-row registry, removes the \
                 retired field, preserves \
                 application records, and reports `upgraded_supported_predecessor`. Every other \
                 predecessor or divergent lineage is rejected. Registry value \
                 `storage/surreal/schema.surql` is relative to the crate source root \
                 `src/backend/handshake_core/src`, resolving repo-wide to \
                 `src/backend/handshake_core/src/storage/surreal/schema.surql`. Stable proof \
                 identifiers are `exact_source_bytes_sha256`, \
                 `parsed_declarative_catalog_sha256`, and `live_engine_catalog_sha256`; the engine \
                 binding is `surrealdb_3_2_0`, the restart proof is `close/reopen`, and the drift \
                 policy is `fail_closed`.",
                json!({
                    "schema_source": "storage/surreal/schema.surql",
                    "schema_source_base": "src/backend/handshake_core/src",
                    "schema_source_repo_path": "src/backend/handshake_core/src/storage/surreal/schema.surql",
                    "supported_predecessor_transition": {
                        "allowlist": "exact_state_schema_info_and_61_row_predecessor_registry_sha256",
                        "registry_rewrite": "61_historical_migration_file_rows_to_63_current_schema_source_rows",
                        "pre_mt142_lineage": "revision_157_stores_gain_knowledge_rich_document_title_anchors_in_place",
                        "ordinary_application_records": "preserved",
                        "reported_outcome": "upgraded_supported_predecessor",
                        "all_other_lineages": "rejected"
                    },
                    "proof_layers": [
                        "exact_source_bytes_sha256",
                        "parsed_declarative_catalog_sha256",
                        "live_engine_catalog_sha256"
                    ],
                    "catalog_counts": {
                        "tables": 282,
                        "authored_fields": 3320,
                        "indexes": 795,
                        "events": 19,
                        "views": 2,
                        "sequences": 2
                    },
                    "engine_binding": "surrealdb_3_2_0",
                    "restart_proof": "embedded_rocksdb_close_reopen",
                    "drift_policy": "fail_closed"
                }),
            ),
            section(
                "navigation",
                "Inspecting the live surface",
                "- `GET /atelier/overview` returns bounded counts for the curated Atelier tables.\n\
                 - `GET /atelier/command-corpus` reads the durable builtin-command projection.\n\
                 - Storage-originated Atelier route failures are never permission to read from an \
                 alternate database. Request validation, not-found, and conflict responses remain \
                 ordinary `400`, `404`, and `409` domain outcomes.",
            ),
            section(
                "recovery",
                "Recovery",
                "1. Preserve the configured `HANDSHAKE_DATA_DIR`; do not delete it as a repair.\n\
                 2. First inspect any global Surreal schema lineage, manifest, or fingerprint error; \
                 it occurs before the Atelier gate.\n\
                 3. If global bootstrap succeeds, inspect the Atelier error for a partial schema, \
                 full-count catalog mismatch, or the first missing `atelier_*` table. Divergent \
                 lineage is refused, not overwritten.\n\
                 4. Repair the canonical SurrealDB schema/open path, then restart the backend.\n\
                 5. Verify `GET /atelier/overview` and `GET /atelier/command-corpus`; never configure \
                 an alternate database or bypass the readiness gate.",
            ),
        ],
        anchors: vec![
            route_anchor("GET", "/atelier/overview"),
            route_anchor("GET", "/atelier/command-corpus"),
            page_link("startup-and-run-commands"),
            page_link("embedded-store-recovery"),
            spec_anchor("2.3.13.0"),
            spec_anchor("6.0.0"),
        ],
    }
}

/// WP-KERNEL-012 MT-072: the editor Settings & Preferences surface, authored as the canonical typed
/// PreferenceRecord authority (Master Spec §10.17). Lists every editor `preference_id`, its value type,
/// default, and the exact recovery/inspection routes so a no-context model/operator can set, reset, and
/// audit editor preferences without reading source.
fn page_editor_preferences() -> NewUserManualPage {
    NewUserManualPage {
        slug: "editor-preferences-surface".into(),
        title: "Editor Settings & Preferences (PreferenceRecord)".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["10.17".into()],
        sections: vec![
            section(
                "purpose",
                "What this surface is",
                "Editor settings (font size, tab size, word wrap, syntax palette, keybinding overrides, \
                 and the code-editor view toggles) persist as the canonical typed **PreferenceRecord** \
                 authority in embedded SurrealDB — NOT as an opaque workspace-settings JSON blob. Every editor \
                 preference has a stable `preference_id`, a declared `value_type`, a registry `default`, a \
                 `scope` (workspace), a `source` (`default`/`operator`/`migration`), a monotonically \
                 increasing `revision`, typed validation, reset-to-default semantics, a full change \
                 history, and recoverable EventLedger receipts. Authority is the embedded SurrealDB row + the \
                 `PREFERENCE_RECORD_CHANGED` EventLedger receipt — never a projection or the settings UI.",
            ),
            section_with_json(
                "workflows",
                "Routes and preference ids",
                "All routes are workspace-scoped under `/workspaces/{workspace_id}/preferences`:\n\n\
                 - `GET /workspaces/{workspace_id}/preferences` — the redacted projection (effective \
                 value, default, scope, source, revision for every editor preference).\n\
                 - `GET /workspaces/{workspace_id}/preferences/{preference_id}` — the resolved record; an \
                 unset defined preference resolves to its registry default (never null), `revision=0`.\n\
                 - `PUT /workspaces/{workspace_id}/preferences/{preference_id}` body `{\"value\": ...}` — \
                 set a typed value. Out-of-range / wrong-type / unknown-enum values are rejected with a \
                 structured HTTP 400 (`preference_validation_failed`), never coerced.\n\
                 - `POST /workspaces/{workspace_id}/preferences/{preference_id}/reset` — reset to the \
                 registry default (a mutation with `source=operator` and its own receipt; NOT a delete).\n\
                 - `GET /workspaces/{workspace_id}/preferences/{preference_id}/history` — the change \
                 receipts, newest first, each pointing at its EventLedger event id.\n\n\
                 Defined editor `preference_id`s (namespace `view-defaults`): `view-defaults.editor.font-size` \
                 (float 6..48, default 13.0), `view-defaults.editor.tab-size` (int 1..16, default 4), \
                 `view-defaults.editor.insert-spaces` (bool, default true), `view-defaults.editor.word-wrap` \
                 (enum off|on|bounded, default off), `view-defaults.editor.word-wrap-column` (int 20..400, \
                 default 80), `view-defaults.editor.render-whitespace` (enum none|boundary|all, default \
                 none), `view-defaults.editor.minimap-enabled`, `view-defaults.editor.sticky-scroll`, \
                 `view-defaults.editor.line-numbers` (bool, default true), `view-defaults.editor.line-height` \
                 (float 1.0..2.0, default 1.0), `view-defaults.editor.bracket-matching`, \
                 `view-defaults.editor.indent-guides` (bool, default true), \
                 `view-defaults.editor.reading-mode-default` (bool, default false), \
                 `view-defaults.editor.syntax-palette-mode` (enum muted|standard|custom, default standard), \
                 `view-defaults.editor.syntax-custom-colors` (json-object of scope->sRGBA), \
                 `view-defaults.editor.keybinding-overrides` (json-object of action->chord).",
                json!({
                    "namespace": "view-defaults",
                    "preference_ids": [
                        "view-defaults.editor.font-size",
                        "view-defaults.editor.tab-size",
                        "view-defaults.editor.insert-spaces",
                        "view-defaults.editor.word-wrap",
                        "view-defaults.editor.word-wrap-column",
                        "view-defaults.editor.render-whitespace",
                        "view-defaults.editor.minimap-enabled",
                        "view-defaults.editor.sticky-scroll",
                        "view-defaults.editor.line-numbers",
                        "view-defaults.editor.line-height",
                        "view-defaults.editor.bracket-matching",
                        "view-defaults.editor.indent-guides",
                        "view-defaults.editor.reading-mode-default",
                        "view-defaults.editor.syntax-palette-mode",
                        "view-defaults.editor.syntax-custom-colors",
                        "view-defaults.editor.keybinding-overrides"
                    ],
                    "event_ledger_type": "PREFERENCE_RECORD_CHANGED"
                }),
            ),
            section(
                "recovery",
                "Failure and recovery steps",
                "- A preference that seems 'stuck' on the wrong value: `GET \
                 .../preferences/{preference_id}` shows the effective value + `revision` + `source`. If \
                 `source=default` and `revision=0`, no explicit value is stored (the registry default is \
                 in force).\n\
                 - Undo a bad change: `POST .../preferences/{preference_id}/reset` restores the default \
                 and appends a receipt; the prior value is preserved in the change history (`GET \
                 .../history`), so a reset never loses provenance and can be re-applied by reading the old \
                 `new_value` from the history and PUTting it back.\n\
                 - Audit who changed what: `GET .../preferences/{preference_id}/history` returns every \
                 receipt (before/after revision, old/new value, actor, `event_ledger_event_id`). The durable \
                 receipt lives in the kernel event ledger, NOT the Flight-Recorder `/events` business-event \
                 projection (that projection does not carry preference records and rejects `KE-` ids); \
                 correlate the `event_ledger_event_id` (a `KE-...` id) via `GET \
                 /kernel/events/aggregates/preference_record/{scope}:{scope_ref}:{preference_id}` for the \
                 durable EventLedger receipt.\n\
                 - A rejected write (HTTP 400 `preference_validation_failed`) never persisted anything — \
                 re-read the record to confirm it is unchanged, then PUT an in-range value.\n\
                 - Recover a transient save failure: an `Unavailable` backend during a preference write \
                 surfaces a visible error and retains the edit; the settings dialog exposes a \
                 \"Retry saving preference\" affordance that re-dispatches the exact retained edit (a \
                 validation 400 is non-retryable and is not re-sent).\n\
                 - Diagnostic posture (HBR-INT-009): Tier 1 Flight Recorder = WIRED (the \
                 `PREFERENCE_RECORD_CHANGED` receipt is durable EventLedger evidence in the kernel event \
                 ledger, appended in the same embedded SurrealDB transaction as the record write, and recoverable \
                 via the kernel aggregate endpoint above rather than the FR `/events` projection). Tier 2 \
                 internal_diagnostics and Tier 3 Palmistry = DEFERRED (not yet shipped in this worktree; \
                 preference reads/writes surface through the standard backend request diagnostics until \
                 they land).",
            ),
        ],
        anchors: vec![
            page_link("rich-documents-surface"),
            page_link("failure-modes-and-recovery"),
            spec_anchor("10.17"),
        ],
    }
}

fn surface_page(
    slug: &str,
    title: &str,
    group: SurfaceGroup,
    intro_md: &str,
    extra_sections: Vec<NewManualSection>,
    mut extra_anchors: Vec<NewManualAnchor>,
    spec_anchors: Vec<String>,
) -> NewUserManualPage {
    let mut sections = vec![
        section("purpose", "What this surface is", intro_md),
        section_with_json(
            "navigation",
            "Routes",
            &group_routes_md(group),
            json!({
                "routes": wp009_surface_registry()
                    .iter()
                    .filter(|s| s.group == group)
                    .map(|s| json!({
                        "surface_id": s.surface_id,
                        "method": s.method,
                        "route": s.route,
                        "summary": s.summary,
                    }))
                    .collect::<Vec<_>>()
            }),
        ),
    ];
    sections.extend(extra_sections);
    let mut anchors = group_route_anchors(group);
    anchors.append(&mut extra_anchors);
    NewUserManualPage {
        slug: slug.into(),
        title: title.into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors,
        sections,
        anchors,
    }
}

fn page_knowledge_index_surface() -> NewUserManualPage {
    surface_page(
        "knowledge-index-surface",
        "Project Knowledge Index — Ingestion And Code Navigation",
        SurfaceGroup::KnowledgeIngestion,
        "The Project Knowledge Index turns configured project roots into typed embedded SurrealDB \
         knowledge: sources with content hashes, extraction receipts, entities, edges, evidence \
         spans, and code symbols. Ingestion routes manage roots/runs/repairs; the code-navigation \
         routes (listed below with the ingestion routes) answer symbol questions WITHOUT an \
         external LSP server.",
        vec![
            section_with_json(
                "navigation",
                "Code navigation routes",
                &group_routes_md(SurfaceGroup::CodeNavigation),
                json!({
                    "routes": wp009_surface_registry()
                        .iter()
                        .filter(|s| s.group == SurfaceGroup::CodeNavigation)
                        .map(|s| json!({
                            "surface_id": s.surface_id,
                            "method": s.method,
                            "route": s.route,
                            "summary": s.summary,
                        }))
                        .collect::<Vec<_>>()
                }),
            ),
            section(
                "inputs_outputs",
                "Inputs and outputs",
                "All routes require the identity headers (400 `bad_request` otherwise) and \
                 return JSON. Code-nav reads append a `KNOWLEDGE_RETRIEVAL_TRACE_RECORDED` \
                 receipt and return its event id, so who-navigated-to-what is auditable. Symbol \
                 answers carry a staleness verdict: `fresh`, `marked_stale` (the indexed file \
                 changed or parse partially failed), or a fail-closed non-fresh state when the \
                 staleness lookup itself fails — stale data is FLAGGED, never silent. Parse \
                 status vocabulary: `parsed` | `partial` | `failed`.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "- 400 `bad_request` — missing identity headers or malformed params.\n\
                 - 404 `not_found` — unknown root/source/symbol/repair id.\n\
                 - 409 `conflict` / `policy_denied` — ingestion policy (allowlist/secret) \
                 refused the operation.\n\
                 - `io_error` — source unreadable at extraction time (queues a repair, never a \
                 silent skip).\n\
                 - 500 `internal_error` / `storage_error` — embedded SurrealDB unavailable: fail-closed, \
                 no data is served (see [[embedded-store-recovery]]).",
            ),
            section(
                "recovery",
                "Recovery",
                "Work the repair queue: `GET /knowledge/ingestion/repairs` then \
                 `POST /knowledge/ingestion/repairs/:repair_id/retry`. Re-run indexing with \
                 `POST /knowledge/ingestion/runs` (idempotent on stable relationship ids — \
                 re-indexing the same content does not duplicate edges). Stale symbols heal on \
                 the next successful index run of the owning file.",
            ),
        ],
        {
            // The page documents BOTH groups: ingestion routes come from the
            // surface_page group; the code-navigation anchors are added here
            // so the MT-195 gate sees full coverage (this is also the MT-112
            // closure: /knowledge/code/* is manual-registered).
            let mut extra = group_route_anchors(SurfaceGroup::CodeNavigation);
            extra.push(page_link("embedded-store-recovery"));
            extra.push(page_link("repair-queues-and-staleness"));
            extra
        },
        vec!["2.3.13.11".into(), "10.20".into()],
    )
}

fn page_notes_loom_surface() -> NewUserManualPage {
    surface_page(
        "notes-loom-surface",
        "Notes (Loom) — Blocks, Links, Graphs, Folders, Tags, Wiki",
        SurfaceGroup::NotesLoom,
        "Notes is the operator-facing name of the Loom engine (DEC-001: 'Loom' stays the \
         engine/spec term). It is the Obsidian-class knowledge surface: LoomBlocks are the \
         atoms; typed LoomEdges link them; backlinks (with context), unlinked mentions, \
         breadcrumbs, tag hubs, folders with color labels, pinned grids, saved views, \
         local/global graphs, bounded traversal, markdown-vault import, media assets, and \
         compiled project-wiki projections sit on top. Every block resolves to a \
         ProjectKnowledgeIndex entity with an EventLedger receipt (the `/knowledge` bridge \
         route) — Loom is not a parallel store.",
        vec![
            section(
                "inputs_outputs",
                "Inputs and outputs",
                "Workspace-scoped JSON routes (`/workspaces/:workspace_id/...`). Create a \
                 workspace first (`POST /workspaces` {name}). Reads do not require identity \
                 headers on this surface. Errors are typed: `HSK-400-LOOM-VALIDATION` (bad \
                 payload), `workspace_not_found` / block-level `not_found` codes (404), \
                 `HSK-403-SILENT-EDIT` (a write the storage guard refuses), `HSK-500-LOOM` \
                 (internal). Graph traversal depth is capped at 8 (default 3). \
                 Graph-search block hits include `hsk.loom_retrieval_bias@1` metadata \
                 so models can see pin, tag, favorite, and backlink ranking influence.",
            ),
            section(
                "workflows",
                "Block Collection Views — table, Kanban, and calendar",
                "Open the mounted Block Collections pane with `menu.view.open-block-collections` \
                 / command `view.block-collections`, or open a Loom search result whose \
                 `block.content_type` is `view_def`. Create with `bcv.new-view`, set \
                 `bcv.new-view.title`, choose `bcv.new-view.kind.table`, \
                 `bcv.new-view.kind.kanban`, or `bcv.new-view.kind.calendar`, then activate \
                 `bcv.new-view.confirm` (cancel: `bcv.new-view.cancel`). The client sends one \
                 stable block id to `POST /workspaces/:workspace_id/loom/views/definitions` \
                 and retains that id across Retry, so response loss cannot create a second view. \
                 embedded SurrealDB inserts the final `view_def` block, search projection, knowledge \
                 bridge, EventLedger mutation receipt, and exact Flight Recorder outbox event in \
                 one transaction; same-id/same-definition retries converge, while a changed \
                 payload for that id returns 409.\n\
                 Table controls `bcv.table.sort.*` persist the typed sort with \
                 `PATCH .../loom/views/definitions/:block_id`, then the host re-queries \
                 `POST .../loom/views/definitions/:block_id/results`; rows are \
                 `bcv.table.row.*` and are never client-side re-sorted. Kanban lanes/cards are \
                 `bcv.kanban.lane.*` / `bcv.kanban.card.*`; a move writes real tag add/remove \
                 mutations, then performs the same authoritative results re-query instead of \
                 locally moving a card. Calendar inputs `bcv.calendar.date-from` and \
                 `bcv.calendar.date-to` accept `YYYY-MM-DD`; `bcv.calendar.apply-range` persists \
                 the definition and re-queries. Switch persisted kinds with \
                 `bcv.kind.table`, `bcv.kind.kanban`, and `bcv.kind.calendar`.\n\
                 Empty states are explicit: `No blocks match this view.`, `No Kanban lanes.`, \
                 and `No blocks in this date range.` A load or mutation failure stays visible as \
                 `View error: ...` at `bcv.status`; `bcv.retry` replays a retained create intent \
                 with the same id or reloads the same saved view with one bounded definition fetch \
                 and one bounded results query. Workspace/generation guards reject stale \
                 deliveries.\n\
                 Diagnostic posture: Tier 1 Flight Recorder is WIRED for create/update through \
                 the transactional embedded SurrealDB outbox and restart reconciler; query events are \
                 observational. Tier 2 internal_diagnostics is WIRED at the shared host/watchdog \
                 but has no collection-specific counter. Tier 3 Palmistry is WIRED at the shared \
                 out-of-process freeze/crash boundary; it has no collection-specific child. \
                 Canonical verification uses a fresh run id with \
                 `tests/run_mt027_argus_proof.ps1 -RunId <mt027-run-id>` and stores source-bound \
                 evidence only under the allocated external Handshake_Artifacts MT-027 root.",
            ),
            section(
                "workflows",
                "Load and diagnose a large tag hub",
                "Open the Tags panel and select a tag, or call `GET \
                 /workspaces/:workspace_id/loom/tags/:tag_block_id`. The target must be a \
                 `tag_hub` block. One response returns the complete tag block, direct \
                 `sub_tags`, direct `tagged_blocks`, and `backlink_count` across every incoming \
                 edge type. Duplicate semantic tag edges remain legal: member rows are \
                 de-duplicated while `backlink_count` counts physical incoming edges. The \
                 MT-136's `mt136_database_surface_proof_a` exercises the real embedded-Surreal \
                 tag-hub read path, including de-duplication and non-tag rejection. There is not yet \
                 a migrated 5,001-block/5,000-edge load proof or a Surreal-native replacement for \
                 the former legacy server database `EXPLAIN (ANALYZE, BUFFERS)` evidence, so the historical \
                 2,000 ms large-hub budget is pending revalidation and must not be claimed from the \
                 functional proof. Use the structured stage timings below for current diagnosis.\n\
                 Recovery: a non-tag target fails closed as `HSK-400-LOOM-VALIDATION`; an \
                 unavailable backend, request timeout, or partial response remains a typed \
                 request failure and must be retried after `/health` reports both service and \
                 database healthy. MT-045 proof failures retain the owned backend listen \
                 report, stdout/stderr, process exit state, health snapshot, and reqwest error \
                 chain under the external `Handshake_Artifacts/wp-kernel-012/mt-045` root.\n\
                 Diagnostic posture (HBR-INT-009): the backend emits structured \
                 `loom_tag_hub_stage_timing` tracing events for tag-block lookup, incoming-edge \
                 query, workspace lookup, LoomBlock mapping, sub-tag/tag assembly, backlink count, JSON \
                 serialization, and response construction; events contain only mechanical durations, \
                 row counts, edge type, and response bytes. Tier 1 Flight Recorder remains WIRED \
                 for Loom mutations but these read-stage timings use structured backend logs. \
                 Tier 2 internal_diagnostics and Tier 3 Palmistry remain WIRED at the shared \
                 host/process boundary; neither changes tag-hub response authority.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "- 404 `workspace_not_found` — the :workspace_id does not exist.\n\
                 - 400 `HSK-400-LOOM-VALIDATION` — malformed block/edge/folder payloads.\n\
                 - 403 `HSK-403-SILENT-EDIT` — silent-edit guard refused an unattributed write.\n\
                 - Stale wiki projections — wiki pages are projections; check \
                 `GET .../loom/wiki/:projection_id/stale` and regenerate.\n\
                 - Unresolvable embeds/assets render typed error states, never blank nodes \
                 (spec 7.1.1.9).",
            ),
            section(
                "recovery",
                "Recovery",
                "Regenerate stale wiki projections (`POST .../wiki/:projection_id/regenerate`). \
                 Recompute derived metrics per block or workspace-wide \
                 (`POST .../loom/metrics/recompute`). Re-run unlinked-mention scans after bulk \
                 imports. Deleted blocks cascade their bridge rows; knowledge entities are \
                 retired, not hard-deleted, so detection history survives.",
            ),
            section(
                "safety",
                "Durable folder/tag mutation receipts and diagnostic posture",
                "Folder-tree mutations (`POST/PATCH/DELETE .../loom/folders...` and folder \
                 membership `PUT/DELETE .../loom/folders/:folder_id/blocks/:block_id`) and \
                 tag-edge mutations (`POST/DELETE .../loom/edges` with `edge_type='tag'`) each \
                 append a durable EventLedger receipt in the SAME embedded SurrealDB transaction as the \
                 domain write. Event types: `KNOWLEDGE_LOOM_FOLDER_MUTATED` \
                 (aggregate_type=`loom_folder`, aggregate_id=folder_id) and \
                 `KNOWLEDGE_LOOM_TAG_MUTATED` (aggregate_type=`loom_edge`, aggregate_id=edge_id). \
                 The receipt id is stored on `loom_folders.event_ledger_event_id`, \
                 `loom_folder_members.event_ledger_event_id`, and \
                 `loom_edges.event_ledger_event_id` with a foreign key to `kernel_event_ledger`, \
                 so a committed mutation can never lack durable evidence and a failed receipt \
                 append rolls the whole mutation back — no partial folder/edge row survives a \
                 restart. Correlate the receipt on `GET .../events`.\n\
                 - Diagnostic posture (HBR-INT-009): Tier 1 Flight Recorder = WIRED (folder \
                 mutations emit `loom_folder_mutated`; tag-edge mutations emit \
                 `loom_edge_created`/`loom_edge_deleted`; these best-effort DuckDB mirrors are \
                 secondary to the durable atomic EventLedger receipt above). Tier 2 \
                 internal_diagnostics = WIRED for the hosting surface, DEFERRED for panel-specific \
                 counters: the native diag_ring event ring, frame-timing sampler, panic hook, and \
                 operation watchdog observe the editor surface that hosts the folder-tree and \
                 tags/tag-hub panels, but a dedicated folder/tag mutation-latency or fetch-failure \
                 counter is not yet emitted to diag_ring (that finer-grained panel telemetry is \
                 DEFERRED with follow-up, not silently skipped). Tier 3 Palmistry = WIRED (the \
                 external out-of-process watcher survives GUI freezes/crashes while these panels \
                 are open).",
            ),
        ],
        vec![page_link("quickstart-loom")],
        vec!["2.2.1.14".into(), "7.1.1.9".into(), "10.12".into()],
    )
}

fn page_rich_documents_surface() -> NewUserManualPage {
    surface_page(
        "rich-documents-surface",
        "Rich Documents — Authority, History, Projections, Embeds",
        SurfaceGroup::RichDocuments,
        "RichDocuments are versioned Tiptap/ProseMirror JSON authority rows in embedded SurrealDB with \
         EventLedger receipts on every save (`KNOWLEDGE_RICH_DOCUMENT_SAVED`). The editor (and \
         embedded Monaco code nodes) renders the typed block tree; saves are optimistic \
         (expected_version) so concurrent writers get a 409 instead of clobbering each other. \
         HTML is the primary export projection (spec 7.1.1.10); markdown export is deliberately \
         lossy.",
        vec![
            section(
                "inputs_outputs",
                "Inputs and outputs",
                "All routes REQUIRE identity headers (400 otherwise). Writes additionally pass \
                 the actor-kind permission boundary — see [[permissions-and-safety]] \
                 (`cloud_model` and `unauthenticated` cannot write). Key bodies:\n\n\
                 - create: `{workspace_id, title, content_json?}`\n\
                 - save: `{expected_version, content_json}` -> 409 `conflict` on stale version\n\
                 - save receipts include `reference_targets` extracted from the exact promoted block tree\n\
                 - EventLedger correlation: `GET /kernel/events/aggregates/knowledge_rich_document/:id`\n\
                 - import: `{workspace_id, title, format: markdown|plain_text|html, content}`\n\
                 - history: `?limit=&offset=` (paginated, newest first)\n\
                 - projection: `?format=markdown|html|plain_text|wiki_loom|context_bundle`",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "- 400 `bad_request` — missing identity headers, unknown actor-kind token, \
                 malformed content_json, or an embed violating the embed-target law (empty / \
                 absolute path / non-http url / scheme-bearing id).\n\
                 - 403 `forbidden` — permission denial with stable reason \
                 (`cloud_model_write_denied`, `validator_write_denied`, \
                 `unauthenticated_write_denied`).\n\
                 - 404 `not_found` — unknown document/revision/embed.\n\
                 - 409 `conflict` — expected_version does not match the stored doc_version.\n\
                 - `receipt_build_failed` / 500 `internal_error` — receipt or storage failure: \
                 the write does not happen without its receipt (fail-closed).",
            ),
            section(
                "recovery",
                "Recovery",
                "409 conflict: reload (`GET /knowledge/documents/:id`), merge, re-save with the \
                 fresh version. Broken embeds: list the typed queue \
                 (`GET .../embeds/broken`) and apply a repair action \
                 (`relink` | `reresolve` | `remove`) via `POST /knowledge/documents/embeds/:embed_id/repair`. \
                 Backlink drift after bulk edits: `POST .../backlinks` rebuilds the rows. \
                 Historical content is never lost — every revision is loadable via \
                 `GET .../history/:doc_version`. To audit a save, take its \
                 `save_receipt_event_id`, read the document aggregate through \
                 `GET /kernel/events/aggregates/knowledge_rich_document/:id`, and match the exact \
                 `event_id`; an empty list means that aggregate has no ledger events.\n\n\
                 WP-KERNEL-012 MT-120 — save receipt OWNERSHIP. The save route now authenticates an \
                 OPTIONAL native session: send `x-hsk-session-token` and the server derives the \
                 `handshake-native:{pid}:{fingerprint}` principal from the native-MCP binding and stamps \
                 it into the server-written receipt payload as `minted_by_principal`. Sending NO token is \
                 still accepted and behaves exactly as before — the save succeeds, but its receipt carries \
                 no anchor and is therefore UNCLAIMABLE by a `document_saved` Flight Recorder event. A \
                 token that is present but invalid or stale is 401 `HSK-401-DOC-SESSION`; it NEVER falls \
                 back to the header identity, because a silent downgrade would restore the forgeable path. \
                 A client may not put a `handshake-native:` value in `x-hsk-actor-id` unless it equals the \
                 principal the server just derived — otherwise 403 `HSK-403-DOC-ACTOR-SPOOF`, on every \
                 document route.\n\n\
                 What this does NOT change: the ledger `actor_id` COLUMN still carries the CLIENT-declared \
                 per-agent attribution, so two agents saving in one process remain distinguishable. \
                 Ownership lives in the payload field; attribution lives in the column. Do not conflate \
                 them. `kernel_task_run_id`, `session_run_id` and `correlation_id` are also unchanged — the \
                 Flight Recorder compares those against the client-supplied event payload, so rebinding \
                 them would replace one unsatisfiable clause with three.\n\n\
                 Why a save's `document_saved` event may be rejected with 400 `HSK-400-INVALID-EVENT`: the \
                 receipt has no `minted_by_principal` (the save was unauthenticated), or it was minted by a \
                 DIFFERENT principal than the one now claiming it — including the same app after a restart, \
                 because the fingerprint is derived from process birth identity. Recovery is to re-save \
                 under the current session rather than to re-issue the event.",
            ),
            section(
                "safety",
                "Native media-embed NodeViews — states and diagnostic posture (HBR-INT-009)",
                "The native editor renders the four CKC media embeds (image, slideshow, album, \
                 video) as interactive egui NodeViews dispatched from the `hsLink` atom by \
                 `refKind`. Every state is observable and typed, never blank and never a panic: \
                 `Resolving` shows a spinner (`author_id=embed-loading-{asset_id}`); a decoded \
                 image renders at its intrinsic aspect ratio (`author_id=embed-image-{asset_id}`, \
                 clickable to a full-size modal `embed-image-modal-{asset_id}`); slideshow/album \
                 expose prev/next/cell controls; and ANY failure — empty ref, absolute path, `..` \
                 traversal, disallowed scheme, missing asset, or an undecodable body — degrades to \
                 a VISIBLE typed error chip (`author_id=embed-error-{asset_id}`). Reference \
                 validation is fail-closed and runs BEFORE any HTTP call. Image bytes are decoded \
                 off the UI thread (tokio `spawn_blocking`) and only the RGBA bytes cross back to \
                 the egui thread for texture upload, so a large or corrupt asset can never freeze \
                 the frame loop.\n\
                 - Tier 1 Flight Recorder = WIRED at the backend embed-authority boundary: the \
                 rich-document save receipt (`KNOWLEDGE_RICH_DOCUMENT_SAVED`) records the exact \
                 `reference_targets` (embed ids), and the broken-embed queue \
                 (`GET .../embeds/broken`) plus the repair route persist embed-repair authority. \
                 The native render issues only read GETs (asset metadata/content/thumbnail); a \
                 client render is not a durable state change and emits no separate business event.\n\
                 - Tier 2 internal_diagnostics = WIRED for the hosting surface, DEFERRED for \
                 embed-specific counters: the native diag_ring event ring, frame-timing sampler, \
                 panic hook, and operation watchdog observe the editor surface that hosts the \
                 embed widgets (off-thread decode keeps the UI-thread frame budget). A dedicated \
                 embed decode-latency / decode-failure counter is not yet emitted to diag_ring — \
                 that finer-grained embed telemetry is DEFERRED with follow-up (no silent skip).\n\
                 - Tier 3 Palmistry = WIRED: the external out-of-process watcher survives GUI \
                 freezes/crashes while embed-bearing documents are open. Because decode is \
                 off-thread and every failure degrades to a typed error chip, a corrupt or missing \
                 asset cannot crash or hang the shell.",
            ),
        ],
        vec![
            route_anchor(
                "GET",
                "/kernel/events/aggregates/:aggregate_type/:aggregate_id",
            ),
            page_link("permissions-and-safety"),
            page_link("quickstart-editor"),
        ],
        vec!["2.3.13.11".into(), "7.1.1.8".into(), "7.1.1.10".into()],
    )
}

fn page_retrieval_surface() -> NewUserManualPage {
    surface_page(
        "retrieval-and-context-bundles-surface",
        "Retrieval — Context Bundles, Staleness, Repair",
        SurfaceGroup::Retrieval,
        "Retrieval compiles BOUNDED, CITED context bundles through an executed plan -> rank -> \
         budget -> snippet pipeline. Every build persists the kernel ContextBundle (id \
         `CTX-<hash>`), per-item decisions (`included` / `excluded_budget` / \
         `excluded_relevance` / `excluded_redacted`), and a replayable RetrievalTrace bound to \
         the bundle. Bundles can cite sources, spans, claims, passages, entities — including \
         UserManual pages (cited as `usermanual:<slug>@<version>` through the page's knowledge \
         entity).",
        vec![
            section(
                "inputs_outputs",
                "Inputs and outputs",
                "Identity headers required. `GET /knowledge/retrieval/bundles/:bundle_id` returns \
                 the bundle + items with citations and token accounting; `/export` returns the \
                 `ai_ready_evidence_export@1` manifest; `/staleness` returns per-item verdicts \
                 (`ok`, missing-evidence reasons like a span/source/claim that no longer exists, \
                 `source_stale` when the cited source changed since indexing) and a bundle-level \
                 `stale` flag; `POST .../repair` recompiles against current sources and returns \
                 the new bundle id.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "- 400 `bad_request` — missing identity headers.\n\
                 - 404 `not_found` — unknown bundle id.\n\
                 - Stale bundles — never consume a bundle without checking `/staleness` when \
                 freshness matters; the projection format served to a model (md/HTML/JSON) is \
                 recorded in the RetrievalTrace.\n\
                 - `receipt_build_failed` / 500 `internal_error` — fail-closed storage paths.",
            ),
            section(
                "recovery",
                "Recovery",
                "`POST /knowledge/retrieval/bundles/:bundle_id/repair` recompiles a stale bundle \
                 (old bundle stays for audit; the response names the replacement). If cited \
                 sources vanished, re-run ingestion first ([[knowledge-index-surface]]).",
            ),
        ],
        vec![
            page_link("knowledge-index-surface"),
            page_link("quickstart-retrieval"),
        ],
        vec!["2.3.13.11".into()],
    )
}

fn page_memory_surface() -> NewUserManualPage {
    surface_page(
        "memory-and-claims-surface",
        "Memory — Claims, Facts, Conflicts, Neighborhood",
        SurfaceGroup::MemoryClaims,
        "The native memory system stores typed claims with a lifecycle \
         (`probationary` -> `stable` / `rejected` / `superseded` / `conflicted`), evidence \
         spans, facts, and bridge edges. Contradictions are DETECTED and surfaced as conflict \
         rows — never silently overwritten.",
        vec![
            section(
                "failure_modes",
                "Failure modes",
                "- 400 `bad_request` — missing identity headers.\n\
                 - 404 `not_found` — unknown claim/fact/entity id.\n\
                 - Conflicted claims — a claim in `conflicted` state needs resolution before it \
                 ranks normally in retrieval.",
            ),
            section(
                "recovery",
                "Recovery",
                "List open conflicts (`GET /knowledge/memory/conflicts`), inspect both claims and \
                 their evidence spans, and resolve through the conflict-resolution flow (the \
                 resolution leaves a receipt). The visual-debug projection \
                 (`GET /knowledge/memory/visual-debug`) exposes the same state with stable \
                 selectors for no-context inspection.",
            ),
        ],
        vec![],
        vec!["2.3.13.11".into()],
    )
}

fn page_crdt_surface() -> NewUserManualPage {
    surface_page(
        "crdt-collaboration-surface",
        "CRDT Draft Collaboration — Push, Pull, Conflict State",
        SurfaceGroup::CrdtCollaboration,
        "Human/AI co-editing rides on Yjs-compatible CRDT updates as DRAFT evidence: push \
         updates, pull since a state vector, and inspect conflict/lease state. CRDT merge is \
         not authority — drafts become authority only through the validated document save / \
         promotion path with EventLedger receipts.",
        vec![
            section(
                "failure_modes",
                "Failure modes",
                "- 400 — malformed update payloads (`knowledge_crdt_push_failed` family \
                 envelopes carry the reason).\n\
                 - 409 — conflicting head / stale state vector: pull first, merge, re-push.\n\
                 - Expired leases — lease writes are denied \
                 (`KNOWLEDGE_CRDT_LEASE_WRITE_DENIED` receipts) until re-claimed.",
            ),
            section(
                "recovery",
                "Recovery",
                "Pull the current head (`GET /knowledge/crdt/updates/pull`), merge locally, \
                 re-push. Inspect `GET /knowledge/crdt/conflict_state` for lease holders and \
                 pending conflicts. Recovery receipts (`KNOWLEDGE_CRDT_RECOVERY_RECEIPT_RECORDED`) \
                 mark replays after interruption.",
            ),
        ],
        vec![],
        vec!["2.3.13.11".into()],
    )
}

fn page_usermanual_surface() -> NewUserManualPage {
    surface_page(
        "usermanual-surface",
        "UserManual — This Surface",
        SurfaceGroup::UserManual,
        "The UserManual is itself a product surface: pages/sections/anchors/tool entries are \
         embedded SurrealDB rows (migration 0310), seeded from a compiled-in corpus, receipted through \
         `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` events, and served read-only over \
         `/usermanual/*`. Anonymous reads are allowed (this is the bootstrap surface); the only \
         write surface is the gated `POST /usermanual/resync`.",
        vec![
            section(
                "inputs_outputs",
                "Inputs and outputs",
                "Reads return JSON rows; `GET /usermanual/pages/:slug` returns \
                 `{page, sections, anchors, bootstrap_receipt_event_id}`. The projection route \
                 renders HTML with stable `data-hs-manual-*` selectors (visual-debug law) or \
                 markdown with `<topic>` tags. `GET /usermanual/freshness` compares DB rows vs \
                 the compiled-in corpus vs the surface registry and returns typed verdicts: \
                 `current` | `stale_content` | `uncovered_surface` | `dangling_anchor` | \
                 `missing_page` | `unseeded_version` | `missing_tool_entry` | \
                 `stale_tool_entry` | `missing_feature_entry` | `stale_feature_entry` | \
                 `missing_legacy_alias` | `stale_legacy_alias`.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "- 404 `not_found` — unknown slug/tool/area/alias.\n\
                 - 400 `bad_request` — empty search query, bad format/area token.\n\
                 - 403 `forbidden` — resync attempted by `cloud_model`/`unauthenticated`.\n\
                 - `stale_content` freshness verdicts — the binary's seed changed but the DB \
                 was not resynced (or a page row was tampered): run the gated resync.\n\
                 - `missing_tool_entry` / `stale_tool_entry`, `missing_feature_entry` / \
                 `stale_feature_entry`, and `missing_legacy_alias` / `stale_legacy_alias` — \
                 non-page corpus rows drifted from the compiled seed: run the gated resync.",
            ),
            section(
                "recovery",
                "Recovery",
                "`POST /usermanual/resync` (operator/system/local_model) re-seeds idempotently — \
                 changed pages, tool entries, feature entries, and legacy aliases are written and \
                 receipted. The freshness route names exactly which page, anchor, surface, tool, \
                 feature, or alias is stale, uncovered, dangling, missing, or unseeded.",
            ),
        ],
        vec![page_link("manual-toc")],
        vec!["10.15.8".into(), "12.7".into()],
    )
}

fn page_failure_modes_and_recovery() -> NewUserManualPage {
    NewUserManualPage {
        slug: "failure-modes-and-recovery".into(),
        title: "Common Failure Modes And Recovery".into(),
        page_kind: "failure_recovery",
        audience: "model_and_operator",
        spec_anchors: vec!["10.15.8".into()],
        sections: vec![
            section_with_json(
                "failure_modes",
                "Typed error envelope vocabulary",
                "Every API answers errors as `{\"error\": \"<code>\", ...}`:\n\n\
                 | surface | codes |\n\
                 |---|---|\n\
                 | documents | `bad_request`, `forbidden`, `not_found`, `conflict` (409), `receipt_build_failed`, `internal_error` |\n\
                 | ingestion | `bad_request`, `conflict`, `policy_denied`, `io_error`, `not_found`, `internal_error` |\n\
                 | code-nav | `bad_request`, `not_found`, `serialize_failed`, `receipt_build_failed`, `internal_error` |\n\
                 | retrieval / memory | `bad_request`, `not_found`, `receipt_build_failed`, `internal_error` |\n\
                 | Notes/Loom | `HSK-400-LOOM-VALIDATION`, `workspace_not_found`/`not_found`, `HSK-403-SILENT-EDIT`, `HSK-500-LOOM` |\n\
                 | crdt | push/pull/head `*_failed` envelopes, 409 conflict |\n\
                 | usermanual | `bad_request`, `not_found`, `forbidden`, `internal_error` |",
                json!({
                    "documents": ["bad_request", "forbidden", "not_found", "conflict", "receipt_build_failed", "internal_error"],
                    "ingestion": ["bad_request", "conflict", "policy_denied", "io_error", "not_found", "internal_error"],
                    "code_nav": ["bad_request", "not_found", "serialize_failed", "receipt_build_failed", "internal_error"],
                    "retrieval": ["bad_request", "not_found", "receipt_build_failed", "internal_error"],
                    "memory": ["bad_request", "not_found", "receipt_build_failed", "internal_error"],
                    "loom": ["HSK-400-LOOM-VALIDATION", "workspace_not_found", "HSK-403-SILENT-EDIT", "HSK-500-LOOM"],
                    "usermanual": ["bad_request", "not_found", "forbidden", "internal_error"]
                }),
            ),
            section(
                "failure_modes",
                "The four failure families",
                "1. **Identity/permission** — 400 missing headers; 400 unknown actor-kind token; \
                 403 stable-reason denials (`cloud_model_write_denied` etc.). Fix the caller, \
                 not the server: assert the correct actor kind explicitly.\n\
                 2. **Concurrency** — 409 `conflict` on stale `expected_version` (documents) or \
                 stale state vector (CRDT). Reload/pull, merge, retry.\n\
                 3. **Content law** — embed-target violations (4 typed reasons: empty, absolute \
                 path, non-http URL, scheme-bearing id), unsanitizable HTML imports (typed error, \
                 never partial silent import), `ImportedRaw` blocks for unconvertible fragments.\n\
                 4. **Staleness** — flagged, never silent: symbol staleness verdicts \
                 (`marked_stale`), bundle item verdicts (`source_stale`, missing evidence), wiki \
                 projection `/stale` checks, manual `stale_content` verdicts.",
            ),
            section(
                "recovery",
                "Recovery map",
                "- Broken embeds -> `GET /knowledge/documents/:id/embeds/broken` + \
                 `POST /knowledge/documents/embeds/:embed_id/repair` (`relink`/`reresolve`/`remove`)\n\
                 - Failed extractions -> [[repair-queues-and-staleness]]\n\
                 - Stale bundles -> `POST /knowledge/retrieval/bundles/:id/repair`\n\
                 - Stale wiki -> `POST /workspaces/:ws/loom/wiki/:projection_id/regenerate`\n\
                 - Stale manual -> `POST /usermanual/resync`\n\
                 - Lost session state -> [[state-recovery-guide]]\n\
                 - Embedded store unavailable -> [[embedded-store-recovery]]",
            ),
        ],
        anchors: vec![
            page_link("repair-queues-and-staleness"),
            page_link("state-recovery-guide"),
            page_link("embedded-store-recovery"),
            route_anchor("POST", "/knowledge/documents/embeds/:embed_id/repair"),
        ],
    }
}

fn page_repair_queues_and_staleness() -> NewUserManualPage {
    NewUserManualPage {
        slug: "repair-queues-and-staleness".into(),
        title: "Repair Queues And Stale State".into(),
        page_kind: "failure_recovery",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.11".into()],
        sections: vec![
            section(
                "failure_modes",
                "Where stale/broken state queues",
                "Handshake never silently drops failed work; it queues typed repair rows:\n\n\
                 - **Ingestion repairs** — `GET /knowledge/ingestion/repairs`: failed/partial \
                 extractions with error class (`io_error`, parse failures, policy denials).\n\
                 - **Broken embeds** — `GET /knowledge/documents/:id/embeds/broken`: typed \
                 broken state with offered repair actions.\n\
                 - **Bundle staleness** — `GET /knowledge/retrieval/bundles/:id/staleness`: \
                 per-item missing-evidence / `source_stale` verdicts.\n\
                 - **Wiki staleness** — `GET /workspaces/:ws/loom/wiki/:projection_id/stale`.\n\
                 - **Manual freshness** — `GET /usermanual/freshness`.\n\
                 - **Memory conflicts** — `GET /knowledge/memory/conflicts`.",
            ),
            section(
                "recovery",
                "Working a queue",
                "Always: (1) list the queue, (2) inspect the typed reason, (3) apply the \
                 surface's repair action (`retry`, `repair`, `regenerate`, `resync`, resolve), \
                 (4) verify the row left the queue. Repairs leave receipts — cite the receipt \
                 id in handoffs so another model can verify without re-running.",
            ),
        ],
        anchors: vec![
            route_anchor("GET", "/knowledge/ingestion/repairs"),
            route_anchor("GET", "/usermanual/freshness"),
            page_link("failure-modes-and-recovery"),
        ],
    }
}

fn page_embedded_store_recovery() -> NewUserManualPage {
    NewUserManualPage {
        slug: "embedded-store-recovery".into(),
        title: "Missing Embedded Storage Behavior".into(),
        page_kind: "failure_recovery",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.11".into()],
        sections: vec![
            section(
                "failure_modes",
                "What happens without the database",
                "Embedded SurrealDB is the only authority store — there is NO alternate local database, in-memory, or mock \
                 fallback anywhere in the product. Behavior when it is unavailable:\n\n\
                 - **Product runtime**: knowledge routes FAIL CLOSED with 500 \
                 `internal_error`/`storage_error` envelopes; no fail-open path serves data when \
                 the store errors.\n\
                 - **Startup**: the server opens the configured `HANDSHAKE_DATA_DIR`, or its \
                 platform-local default, before serving. A locked, corrupt, or incompatible store \
                 fails closed.\n\
                 - **Scoped migrated tests**: MT-136 storage proofs and Surreal-backed fixtures \
                 allocate isolated stores under `HANDSHAKE_ARTIFACTS_ROOT`; when embedded SurrealDB \
                 is unavailable they fail hard. A green run for those targets therefore requires \
                 the real embedded engine, not alternate local database, mocks, or skipped proof. Inspect other \
                 integration-test targets before assuming they have completed the migration.",
            ),
            section(
                "recovery",
                "Recovery",
                "1. Probe: `curl http://127.0.0.1:37501/health` and inspect the backend storage error.\n\
                 2. Confirm `HANDSHAKE_DATA_DIR` resolves to the intended store, then restart the backend.\n\
                 3. If the data directory is locked or corrupt, preserve it: EventLedger and all \
                 manual/knowledge rows live in embedded SurrealDB, so never delete the store to \
                 'fix' startup without a backup.\n\
                 4. Re-run the smallest scoped test that exercises your surface to confirm \
                 recovery.",
            ),
            section(
                "workflows",
                "Process, session checkpoint, and Flight Recorder durability",
                "The process ledger writes `kernel_process_lifecycle` START/STOP state with one \
                 atomic SurrealDB merge per process. A replayed START cannot erase a concurrent \
                 STOP or reclaim result. The session checkpoint writer flushes queued \
                 `kernel_session_checkpoint` rows during bounded shutdown and reports sink or \
                 join failures. Every duplicate checkpoint id is ignored without replacing the \
                 original row or increasing the written-row count, including a duplicate carrying \
                 different content; later independent checkpoints in the same retained batch still \
                 persist. Flight Recorder envelopes \
                 receive a stable event id and idempotency key before their first write, use \
                 bounded retry, and surface durable-sink or shutdown failure instead of silently \
                 dropping accepted events. Startup reopens the same embedded store, runs \
                 restart-resume before serving, and the consumer timeline query reads the \
                 recovered `kernel_event_ledger` rows. There is no legacy server database compatibility or \
                 fallback path in any of these flows.",
            ),
        ],
        anchors: vec![
            page_link("startup-and-run-commands"),
            page_link("state-recovery-guide"),
        ],
    }
}

/// Slug of the MT-142 swarm page; shared with the in-crate consistency tests.
const SURREAL_SWARM_PAGE_SLUG: &str = "surreal-swarm-concurrency-and-load";

/// WP-KERNEL-012 MT-142 (AC-142-11, PT-142-10): embedded single-owner SurrealDB
/// topology, safe parallel swarm use, retry / retry-exhaustion / shutdown
/// behaviour, both load-profile runbooks, how to read
/// `hsk.surreal_swarm_load_report@1`, and the embedded-vs-remote proof
/// boundary. Every file:line cites the product tree or the pinned surrealdb
/// 3.2.0 sources recorded in the MT-142 research basis; the in-crate tests
/// `mt142_manual_covers_surreal_swarm_concurrency_and_load` and
/// `mt142_manual_runbook_targets_and_env_vars_exist` pin the page to the code.
fn page_surreal_swarm_concurrency_and_load() -> NewUserManualPage {
    const LOAD_TARGET: &str = "surreal_swarm_load_tests";
    const CI_TEST: &str = "ci_profile_16_workers_2000_operations_is_correct_and_bounded";
    const EXTENDED_TEST: &str = "extended_profile_64_workers_50000_operations";
    const SEMANTICS_TARGET: &str = "surreal_swarm_semantics_tests";
    const LIFECYCLE_TARGET: &str = "surreal_swarm_lifecycle_tests";
    let cargo_test = |target: &str, filter: &str| {
        let filter = if filter.is_empty() {
            String::new()
        } else {
            format!(" {filter}")
        };
        format!(
            "cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test {target} \
             --features surreal-test-support,test-utils{filter} -- --nocapture"
        )
    };
    let ci_command = cargo_test(LOAD_TARGET, CI_TEST);
    let extended_command = cargo_test(LOAD_TARGET, EXTENDED_TEST);
    let semantics_command = cargo_test(SEMANTICS_TARGET, "");
    let lifecycle_command = cargo_test(LIFECYCLE_TARGET, "");
    let load_profiles_md = format!(
        "Both profiles are integration targets under `src/backend/handshake_core/tests/` and need the real \
         embedded engine: cargo features `surreal-test-support,test-utils`; `HANDSHAKE_ARTIFACTS_ROOT` set to \
         an ABSOLUTE `_Artifacts` root (the fixture refuses relative paths and never falls back, \
         `storage/tests.rs:42-60`); `HANDSHAKE_WORKSPACE_ROOT` set to a run-scoped directory so artifact-writing \
         code never resolves the repo root from the manifest dir (`storage/mod.rs:687-696`); `CARGO_TARGET_DIR` \
         at the operator-designated shared target. Never run two cargo commands against that target at once. \
         Env assignments below are POSIX shell; in PowerShell write `$env:NAME='value';` before `cargo`.\n\n\
         CI deterministic profile (16 workers, 2000 operations, fixed seed; per-operation 5000 ms, per-worker \
         60000 ms, whole test 180000 ms; a timeout is a test failure, never an ignored test):\n\n\
         ```\n\
         HANDSHAKE_ARTIFACTS_ROOT=<absolute-artifacts-root> HANDSHAKE_WORKSPACE_ROOT=<run-dir> HANDSHAKE_SWARM_LOAD_REPORT_DIR=<report-dir> \\\n\
         {ci_command}\n\
         ```\n\n\
         It prints `SWARM_LOAD_REPORT=<path>`; the file is `swarm-load-<profile>-<run_id>.json` with profile \
         `ci`, i.e. `swarm-load-ci-<run_id>.json` (`tests/surreal_swarm_load_tests.rs:79`, `:1143`), under \
         `HANDSHAKE_SWARM_LOAD_REPORT_DIR` (fallback `<HANDSHAKE_ARTIFACTS_ROOT>/handshake-test/swarm-load/`, \
         `tests/swarm_support/mod.rs:666-681`).\n\n\
         Extended local profile (at least 64 workers, at least 50000 operations, at least 5000 dataset records, \
         seeded and repeatable, whole test 1800000 ms; runs only when `HANDSHAKE_SWARM_EXTENDED=1`, otherwise \
         the test prints `SWARM_EXTENDED=NOT_RUN_UNCONFIGURED` and passes without proving anything):\n\n\
         ```\n\
         HANDSHAKE_SWARM_EXTENDED=1 HANDSHAKE_SWARM_SEED=<u64> HANDSHAKE_ARTIFACTS_ROOT=<absolute-artifacts-root> HANDSHAKE_WORKSPACE_ROOT=<run-dir> HANDSHAKE_SWARM_LOAD_REPORT_DIR=<report-dir> \\\n\
         {extended_command}\n\
         ```\n\n\
         It writes `swarm-load-extended-<run_id>.json` plus an RSS record \
         `swarm-load-extended-<run_id>-memory.json` to the same directory (`:98`, `:1143`, `:1285`). \
         `HANDSHAKE_SWARM_SEED` (u64) \
         overrides the fixed workload seed for either profile; the seed used is printed and stored in \
         `workload_seed`. Companion proofs in the same tree:\n\
         - `{semantics_command}` - same-record expected-version race, idempotency convergence, \
         disjoint-record overlap, independent clients without a shared registry, opposite-order deadlock \
         freedom, registry reclamation.\n\
         - `{lifecycle_command}` - shutdown under load, reopen, acknowledged-write reconciliation.\n\n\
         Run-scoped stores live under `HANDSHAKE_ARTIFACTS_ROOT` and are removed by the fixture; keep only \
         the JSON reports."
    );
    NewUserManualPage {
        slug: SURREAL_SWARM_PAGE_SLUG.into(),
        title: "SurrealDB — Embedded Single-Owner Topology, Parallel Swarm Use, Retry, Shutdown, and Load Proof"
            .into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.0".into(), "2.3.13.11".into()],
        sections: vec![
            section_with_json(
                "purpose",
                "Embedded single-owner topology",
                "One `SurrealStorage` owns one embedded SurrealDB engine over one RocksDB store path: \
                 `SurrealStorage::open` calls `Surreal::new::<RocksDb>((path, engine_config))` exactly once and \
                 keeps that sole `Surreal<Db>` handle inside the wrapper (`storage/surreal.rs:874-941`, \
                 `:906-907`, `:158`, `:806-816`). The store path is `HANDSHAKE_DATA_DIR` (or the platform-local \
                 data dir) joined with `handshake-surreal` (`surreal.rs:143-146`, `:245-262`).\n\n\
                 Parallelism unit: clone the wrapper, never the engine. `SurrealStorage` and `SurrealDatabase` \
                 are cheap `Clone` values over one shared `Arc` (`surreal.rs:378-381`, \
                 `storage/surreal/database.rs:30-34`); every clone in every tokio task runs its operation under \
                 a shared lifecycle lease (`with_data_operation`/`with_lease`, `surreal.rs:951-963`, \
                 `:1091-1105`) against the same engine. The wrapper hands out no cloned SDK handle \
                 (`surreal.rs:1139-1141`). At the SDK level a cloned `Surreal<Db>` is a separate session over the \
                 same `Datastore` (surrealdb 3.2.0 `src/lib.rs:336-347`, `engine/local/native.rs:240`), which is \
                 why in-process sharing of cloned handles is the supported topology.\n\n\
                 A second process on the same store path is NOT supported, and a second embedded engine on \
                 the same path inside this process is NOT supported either: RocksDB takes an exclusive \
                 `<store>/LOCK` file at open and the second open fails with `IO error: Failed to create lock \
                 file` instead of becoming a second writer (RocksDB 11.0.0 `db/db_impl/db_impl_open.cc:439-442`, \
                 `port/win/env_win.cc:952-982`; RocksDB FAQ: multiple processes may not write to one RocksDB). \
                 Two wrappers over one engine are in-process sharing; never describe them, or any same-path \
                 second engine, as distributed concurrency or as a distributed proof.\n\n\
                 Engine guarantees this page relies on (surrealdb 3.2.0 pinned; research basis \
                 `MT-142/kb01/research/research_basis.json`):\n\
                 - Snapshot isolation per transaction with write-write conflict detection at commit: RocksDB \
                 `OptimisticTransactionDB` (`surrealdb-core-3.2.0/src/kvs/rocksdb/mod.rs:462-470`), snapshot \
                 pinned at transaction start (`mod.rs:757-766`), writes only buffered until commit \
                 (`mod.rs:2133-2138`; `optimistic_transaction.cc:365-368`).\n\
                 - Only WRITTEN keys are validated at commit; a plain read is not (no `SELECT ... FOR UPDATE` \
                 before SurrealDB 3.3.0). Every guard that decides on a record it read therefore also writes \
                 that record.\n\
                 - The engine never retries a user transaction (`dbs/executor.rs` has no retry loop; \
                 `kvs/ds.rs:1632-1681` wraps bootstrap only). Retry is Handshake's job: see the retry section.\n\
                 - A commit conflict renders as `Transaction conflict: ... This transaction can be retried` \
                 (`kvs/err.rs:47-49`; retryable predicate `:85-87`; RocksDB Busy/TryAgain mapping `:124-135`) \
                 and the losing transaction wrote nothing.\n\
                 - Every commit waits for a grouped WAL fsync (`SyncMode::Every`, `kvs/rocksdb/cnf.rs:598`; \
                 `commit_coordinator.rs:15-47`): an acknowledged `Ok` is durable; an unacknowledged in-flight \
                 commit is not.",
                json!({
                    "engine": "surrealdb 3.2.0 embedded kv-rocksdb (RocksDB OptimisticTransactionDB)",
                    "owner": "SurrealStorage::open -> Surreal::new::<RocksDb>((path, engine_config)); one engine per store path",
                    "parallelism_unit": "clone SurrealStorage / SurrealDatabase (Arc-shared); operations run under with_data_operation leases",
                    "second_process_same_path": "unsupported: RocksDB LOCK file; the second open fails",
                    "second_engine_same_path_same_process": "unsupported: same LOCK file",
                    "distributed_proof": "never claimed from same-path engines or in-process clones",
                    "isolation": "snapshot per transaction; write-write conflict detection at commit on written keys only",
                    "engine_internal_retry": false,
                    "durability": "acknowledged commit = grouped WAL fsync completed"
                }),
            ),
            section_with_json(
                "workflows",
                "Safe parallel swarm use",
                "Issue these concurrently from any number of tasks or model sessions. Each write is one \
                 `BEGIN TRANSACTION ... COMMIT TRANSACTION` query string with bound parameters and `THROW` \
                 guards, so a document save writes the document, its Loom projection, its search projection, \
                 its version row, its draft delete and (optionally) its idempotency claim atomically \
                 (`storage/surreal/knowledge.rs:2400-2410`):\n\
                 - Point reads and range/search queries: never wait on any lock; they read the transaction \
                 snapshot.\n\
                 - Creates on distinct records, deletes, and multi-record projection/ledger transactions on \
                 disjoint records: commit concurrently. RocksDB detects conflicts only between transactions \
                 that wrote an overlapping key, so disjoint records and different workspaces show at least two \
                 simultaneous in-flight writes (AC-142-2). MT-142 removed the process-global \
                 `RICH_DOCUMENT_MUTATION_LOCK` and `KNOWLEDGE_UPSERT_LOCK`; nothing serializes unrelated \
                 writes.\n\
                 - Same-record optimistic versioned update (expected-version race): the guarded \
                 `UPDATE ... WHERE doc_version = $expected_version` runs inside the transaction and otherwise \
                 executes `THROW 'HSK-KRD-SAVE-STALE'` (`knowledge.rs:2402`). Exactly one caller wins; every \
                 other caller gets the typed stale outcome `StorageError::Conflict(\"knowledge rich document \
                 version conflict: expected_version is stale\")` (`knowledge.rs:2383-2387`, `:2467-2490`), \
                 surfaced over HTTP as `409 {\"error\":\"conflict\",\"detail\":...}` \
                 (`api/knowledge_documents.rs:388-401`). There is no last-writer-wins. If two writers both pass \
                 the guard before either commits, the engine aborts one at commit with a retryable conflict; \
                 the CAS save is re-run only because the expected_version makes the re-run safe: the live read, \
                 the stale pre-check and the compare-and-set all repeat, so the re-run succeeds once or reports \
                 stale (`knowledge.rs:2351-2357`). A stale outcome is terminal and is never retried.\n\
                 - Identical idempotency-key replays: the claim statement creates the \
                 `knowledge_idempotency_keys` record in the same transaction or executes \
                 `THROW 'HSK-KIDEM-RACE'` (`knowledge.rs:2319`); the losing transaction aborts having written \
                 nothing and the caller re-reads the winner's committed result (`knowledge.rs:2351-2354`, \
                 `:2464-2466`). A replay with the same request hash returns the stored result reference; a \
                 different payload under the same key is a typed conflict (`knowledge.rs:2500-2501`). One key \
                 converges to exactly one durable effect.\n\
                 - `create_knowledge_rich_document_if_title_absent`: the transaction UPSERTs one \
                 `knowledge_rich_document_title_anchors` row per (workspace, normalized title) with a fresh \
                 claim nonce (`knowledge.rs:198-230`), so two concurrent creators write the same key and RocksDB \
                 admits exactly one commit; the loser's retry re-reads and returns the winner as the existing \
                 document. The anchor is a serialization device, not a uniqueness rule: duplicate titles created \
                 through the plain path stay legal.\n\
                 - Natural-key upserts (`upsert_knowledge_*`): IF-exists-UPDATE-ELSE-CREATE inside one \
                 transaction under `guarded_mutation` (`knowledge.rs:159-196`); a unique-index violation on the \
                 statement's OWN natural-key index is classified `RetryableSnapshotChange` and the re-run takes \
                 the UPDATE branch, while a violation on any other index is terminal \
                 (`classify_knowledge_error`, `knowledge.rs:111-128`).\n\n\
                 Keyed locks are optional contention shaping, never correctness. `KeyedLockRegistry` \
                 (`storage/surreal/keyed_lock.rs`) serializes only callers that hold the same `LockKey` \
                 (`Record { table, id }`, `NaturalKey { workspace_id, kind, key }`, `Workspace { workspace_id }`, \
                 `keyed_lock.rs:55-67`); `LockMode::Disabled` hands out no-op guards (`:97-102`, `:196-205`). One \
                 registry lives per `SurrealDatabase` value (`storage/surreal/database.rs:21-57`): \
                 `SurrealDatabase::new` creates a fresh `KeyedLockRegistry::keyed()`, \
                 `SurrealDatabase::with_lock_registry(storage, registry)` attaches an explicit one (pass \
                 `KeyedLockRegistry::disabled()` for the no-lock proof), `SurrealDatabase::lock_registry()` \
                 exposes it for measurement, and `Clone` shares it (clones are one logical wrapper). Knowledge \
                 writers take their keys through `guarded_mutation` (`knowledge.rs:159-196`): \
                 `acquire_many_with_deadline` with the statement timeout as deadline, then the bounded retry; a \
                 lock wait that outlives the statement timeout returns \
                 `StorageError::ConflictDetails { code: \"HSK-STORAGE-LOCK-WAIT-TIMEOUT\" }` \
                 (`LOCK_WAIT_TIMEOUT_CONFLICT_CODE`, `knowledge.rs:96-98`, `:152-157`, `:178-183`) instead of \
                 hanging. Multi-key acquisition sorts and \
                 dedups keys so opposite-order callers cannot deadlock (`:241-262`, `:281-285`); \
                 `acquire_with_deadline` returns a typed `LockWaitTimeout` instead of hanging (`:104-110`, \
                 `:209-239`); the registry reclaims every entry when its last guard drops, so its idle bound is \
                 0 entries (`:181-194`, `:287-305`); read paths never take a key (`:36-39`); the guard's \
                 `lock_wait()` feeds the report's `lock_wait_ms_p50_p95_p99` (`:329-333`). The \
                 independent-client proof runs two wrappers over one engine that share no registry (one in \
                 `LockMode::Disabled`) and every same-record, uniqueness, idempotency and ledger invariant \
                 holds identically, proving that the transactions own correctness (AC-142-8).\n\n\
                 Rules for a model driving parallel work:\n\
                 1. Take `expected_version` from your own last successful response; on `409` stale, re-read the \
                 document and rebase.\n\
                 2. Send an idempotency key when a request may be retried by the caller; identical replays are \
                 safe, divergent payloads under one key are rejected.\n\
                 3. Never start a second backend, test, or engine on the same `HANDSHAKE_DATA_DIR`; share the \
                 running backend's HTTP API instead.\n\
                 4. Treat `HSK-STORAGE-RETRY-EXHAUSTED` (next section) as contention to back off from, not as \
                 data loss.",
                json!({
                    "concurrent_safe": {
                        "point_read": "no lock; snapshot read",
                        "range_or_search_query": "no lock; snapshot read",
                        "create_disjoint": "commits concurrently",
                        "delete_disjoint": "commits concurrently",
                        "multi_record_transaction_disjoint": "one BEGIN..COMMIT; commits concurrently",
                        "same_record_expected_version": "one winner; losers HSK-KRD-SAVE-STALE -> StorageError::Conflict -> 409; never retried",
                        "same_idempotency_key": "one effect; losers HSK-KIDEM-RACE -> re-read winner; divergent payload -> conflict",
                        "natural_key_upsert": "UNIQUE index + IF-exists guard; index race replayed as RetryableSnapshotChange"
                    },
                    "removed_global_locks": ["RICH_DOCUMENT_MUTATION_LOCK", "KNOWLEDGE_UPSERT_LOCK"],
                    "keyed_lock": {
                        "registry": "KeyedLockRegistry per SurrealDatabase",
                        "modes": ["Keyed", "Disabled"],
                        "keys": ["Record", "NaturalKey", "Workspace"],
                        "multi_key_order": "sorted and deduplicated",
                        "idle_entry_bound": 0,
                        "read_paths_take_keys": false,
                        "lock_wait_deadline": "SurrealStorageConfig::statement_timeout",
                        "lock_wait_timeout_code": "HSK-STORAGE-LOCK-WAIT-TIMEOUT",
                        "correctness_dependency": "none; transactions and guards own correctness"
                    }
                }),
            ),
            section_with_json(
                "failure_modes",
                "Contention, retry, and retry exhaustion",
                "Retry lives in `storage/surreal/retry.rs` and wraps only replay-safe operations.\n\n\
                 Policy `RetryPolicy::CONTRACT` (`retry.rs:97-103`): 5 ms base delay, 250 ms cap, 8 attempts \
                 (the first attempt included), 2000 ms maximum elapsed, full jitter \
                 `sleep = random(0, min(cap, base * 2^n))` (`retry.rs:28-29`, `:524-525`). The 7 sleep upper \
                 bounds are 5, 10, 20, 40, 80, 160, 250 ms (worst case 565 ms of sleep, `retry.rs:1146-1150`). \
                 The effective deadline is the earlier of start + 2000 ms and the caller deadline (`:556-562`); \
                 no sleep starts that would end after it (`:568-575`); cancellation is observed before every \
                 attempt and during every sleep (`:489-495`, `:543-552`); an attempt already in flight is never \
                 abandoned, because dropping an acknowledged commit would misreport a durable write \
                 (`:31-38`).\n\n\
                 Retried (only under `Replay::Idempotent { key }`, `retry.rs:295-327`):\n\
                 - `RetryClass::RetryableTransient`: an engine commit conflict (`KvsError::TransactionConflict`) \
                 in any SDK-visible shape: typed `QueryError::TransactionConflict`, or a `NotExecuted`/`Internal` \
                 error whose message carries both `Transaction conflict:` and `This transaction can be retried`, \
                 or an unwrapped raw `Resource busy` / `Operation failed. Try again.` status (`retry.rs:608-673`).\n\
                 - `RetryClass::RetryableSnapshotChange`: a unique-index or IF-EXISTS race on an idempotent \
                 upsert; the integrating store decides it, `is_unique_index_violation` alone never does \
                 (`:336-339`, `:701-716`).\n\n\
                 Never retried: any operation declared `Replay::NotIdempotent` (its first error is returned \
                 as-is, `:321-326`, `:1070-1092`); every `RetryClass::Terminal` error: thrown guard codes such as \
                 `HSK-KRD-SAVE-STALE` and `HSK-KIDEM-RACE`, `AlreadyExists`, `TimedOut`, `Cancelled`, \
                 `NotExecuted` without the conflict markers, `Internal` without them (IO, corruption, `LOCK`, \
                 router closed), `Validation`, `NotAllowed`, `NotFound` (`:624-626`, `:1421-1444`). An \
                 expected-version mismatch is never a retry. A statement attempt that outlives the caller-side \
                 statement timeout (`SurrealStorageConfig::statement_timeout`, default 30 s) returns the terminal \
                 `SurrealStorageError::StatementTimeout { waited_ms }` and is never retried: dropping the SDK \
                 future does not abort the engine-side statement, so its outcome is unknown \
                 (`knowledge.rs:1282-1297`).\n\n\
                 Knowledge writes run under `RetryPolicy::CONTRACT` with `TokioClock`, one process-wide \
                 `SystemJitter` and the store's shutdown cancellation token \
                 (`RetryContext::unbounded().with_cancel(storage.cancellation_token())`, `knowledge.rs:101`, \
                 `:184-195`).\n\n\
                 Exhaustion and its code: when every attempt failed retryably and a bound is hit, `retry` returns \
                 `RetryError::Exhausted { attempts, elapsed, last, bound }` with `bound` = `max_attempts` or \
                 `max_elapsed` (`retry.rs:355-390`) and emits exactly one `warn` diagnostic \
                 `surreal retry exhausted` carrying `attempts`, `elapsed_ms`, `bound`, `replay_key` and \
                 `last_error` (`:584-606`). Store callers see \
                 `StorageError::ConflictDetails { code: \"HSK-STORAGE-RETRY-EXHAUSTED\", detail: \"attempts=.. \
                 elapsed_ms=.. bound=.. last=..\" }` (`RETRY_EXHAUSTED_CONFLICT_CODE`, `knowledge.rs:95`; \
                 `retry_error_to_storage`, `:130-150`), which the knowledge API maps to `409` with the code as \
                 `detail` (`api/knowledge_documents.rs:399-401`). Nothing was written by the exhausted attempts. \
                 Cancellation before an attempt or during a sleep returns \
                 `RetryError::Cancelled { attempts, elapsed }` and reaches callers as the closed-store error \
                 (`SurrealStorageError::Closed`, `embedded database is closed`; `knowledge.rs:107-109`, \
                 `:146-148`; `surreal.rs:184-185`). In the load report these appear as `conflict_count`, \
                 `retry_count`, `retry_exhaustion_count` and `failed_by_operation_and_class[..][retry_exhausted]`.",
                json!({
                    "retry_policy": {
                        "base_delay_ms": 5,
                        "maximum_delay_ms": 250,
                        "maximum_attempts": 8,
                        "maximum_elapsed_ms": 2000,
                        "jitter": "full",
                        "sleep_upper_bounds_ms": [5, 10, 20, 40, 80, 160, 250],
                        "worst_case_sleep_sum_ms": 565
                    },
                    "retried": ["RetryableTransient (engine commit conflict)", "RetryableSnapshotChange (idempotent upsert index race)"],
                    "retried_only_when": "Replay::Idempotent { key }",
                    "never_retried": ["Replay::NotIdempotent", "Terminal: thrown guard codes, AlreadyExists, TimedOut, Cancelled, NotExecuted/Internal without conflict markers, Validation, NotAllowed, NotFound", "expected-version mismatch (HSK-KRD-SAVE-STALE)", "SurrealStorageError::StatementTimeout (outcome unknown)"],
                    "exhaustion": {
                        "error": "RetryError::Exhausted { attempts, elapsed, last, bound }",
                        "bounds": ["max_attempts", "max_elapsed"],
                        "diagnostic": "warn `surreal retry exhausted` once per exhaustion",
                        "storage_error": "StorageError::ConflictDetails { code: HSK-STORAGE-RETRY-EXHAUSTED, detail: attempts=.. elapsed_ms=.. bound=.. last=.. }",
                        "http_status": 409
                    },
                    "cancellation": {
                        "error": "RetryError::Cancelled { attempts, elapsed }",
                        "storage_error": "SurrealStorageError::Closed"
                    }
                }),
            ),
            section_with_json(
                "workflows",
                "Shutdown under load and the ShutdownReport",
                "`SurrealStorage::shutdown` (`surreal.rs:1146-1194`) is idempotent across repeated and \
                 concurrent callers and is rejected with `ReentrantShutdown` from inside an operation \
                 (`:1147-1149`). Phases (`perform_shutdown`, `:1238-1273`):\n\
                 1. Admission stops: the lifecycle flips to CLOSING (`:1164-1166`) and every new `with_lease` \
                 call returns `SurrealStorageError::Closed` (`:1096-1102`, `embedded database is closed`), \
                 never a hang.\n\
                 2. Drain: the close waits up to `SurrealStorageConfig::drain_grace` (`DEFAULT_DRAIN_GRACE` \
                 5 s, `:148-150`; `with_drain_grace`, zero cancels immediately, `:301-306`) for every in-flight \
                 lease to finish (`:1252-1255`).\n\
                 3. Cancel: if the grace expires, the store-wide `CancellationToken` fires \
                 (`cancel_operations`, `:1256-1267`); `retry` sleeps and keyed-lock waits hold child tokens \
                 from `SurrealStorage::cancellation_token()` (`:1115-1120`) and return the closed-store error. \
                 Work already blocked inside the engine is not interruptible and is awaited instead \
                 (`:1238-1244`); the engine transaction timeout below caps that wait.\n\
                 4. Close: `close_client` runs a `RETURN true;` barrier under the client write lease, takes and \
                 drops the sole engine handle (`:1275-1297`), then on Windows proves RocksDB `LOCK` release by \
                 an exclusive-open probe with 5 ms to 250 ms doubling backoff (`:1300-1347`); non-Windows \
                 builds yield without that stronger proof (`:1349-1356`). Each caller waits at most \
                 `shutdown_wait` (`DEFAULT_SHUTDOWN_WAIT` 30 s, `:147`; `with_shutdown_wait_timeout`, zero \
                 rejected, `:286-299`) and otherwise gets `ShutdownStillInProgress { waited_ms }` while the \
                 close continues (`:1188-1193`). A retryable barrier failure reinstalls a fresh cancellation \
                 token and reopens the wrapper for another attempt; a terminal failure after the handle was \
                 dropped leaves it CLOSED and reports `Shutdown(error)` on every later call (`:1196-1236`).\n\
                 5. No single statement can hold a lease forever: \
                 `SurrealStorageConfig::with_engine_timeouts(query, transaction)` \
                 (`DEFAULT_ENGINE_QUERY_TIMEOUT` 30 s, `DEFAULT_ENGINE_TRANSACTION_TIMEOUT` 60 s, `:153-156`, \
                 `:323-343`; `None` disables one, zero is rejected) is handed to the embedded datastore at open \
                 through the SDK `Config::query_timeout`/`transaction_timeout` (`:894-907`; surrealdb-3.2.0 \
                 `engine/local/native.rs:131-133`, `opt/config.rs:16-17,53-61`; surrealdb-core-3.2.0 \
                 `dbs/executor.rs:1034-1049` cancels an expired write transaction and `kvs/ds.rs:3951-3953` \
                 makes the query timeout every query's context deadline). The caller-side `statement_timeout` \
                 (`DEFAULT_STATEMENT_TIMEOUT` 30 s, `:151-152`; `with_statement_timeout`, `:308-321`) bounds \
                 one statement attempt and every keyed-lock wait.\n\n\
                 Reading a `ShutdownReport { drained, cancelled, elapsed }` (`:794-804`; returned by \
                 `SurrealStorage::shutdown_with_report()`, `:1127-1135`, or read later with \
                 `last_shutdown_report()`, `:1122-1125`): `drained: bool` = every in-flight lease finished \
                 within the grace; `cancelled: bool` = the grace expired and the cancellation token fired before \
                 the remaining engine-bound leases were awaited; `elapsed: Duration` = wall time from the close \
                 attempt to engine release (the load report copies it to `shutdown_elapsed_ms`). \
                 `cancelled == true` is not an integrity failure: every acknowledged commit had already \
                 completed its grouped fsync and is present after reopen; every cancelled or unacknowledged \
                 transaction is absent as a whole (no partial document/version/projection/ledger state). \
                 `elapsed` far above `drain_grace`, or a `ShutdownStillInProgress` result, means engine-bound \
                 statements were still running: report it as a bound finding, do not retry blindly. After \
                 shutdown, reopen with `SurrealStorage::open` on the same data dir; the lifecycle proof \
                 (`surreal_swarm_lifecycle_tests`) reconciles acknowledged writes against the reopened store.",
                json!({
                    "phases": ["admission_stop", "drain_within_drain_grace", "cancel_cooperative_work", "barrier_drop_handle_prove_lock_release", "close"],
                    "shutdown_wait_default_ms": 30000,
                    "drain_grace_default_ms": 5000,
                    "statement_timeout_default_ms": 30000,
                    "engine_query_timeout_default_ms": 30000,
                    "engine_transaction_timeout_default_ms": 60000,
                    "report": {
                        "type": "ShutdownReport",
                        "fields": ["drained", "cancelled", "elapsed"],
                        "field_types": {"drained": "bool", "cancelled": "bool", "elapsed": "Duration"},
                        "accessors": ["SurrealStorage::shutdown_with_report", "SurrealStorage::last_shutdown_report"]
                    },
                    "post_shutdown_operation_error": "SurrealStorageError::Closed",
                    "runtime_symbols": [
                        "SurrealStorage::shutdown",
                        "SurrealStorage::shutdown_with_report",
                        "SurrealStorage::last_shutdown_report",
                        "SurrealStorage::cancellation_token",
                        "SurrealStorageError::Closed",
                        "SurrealStorageError::ReentrantShutdown",
                        "SurrealStorageError::ShutdownStillInProgress",
                        "SurrealStorageError::StatementTimeout",
                        "SurrealStorageConfig::with_shutdown_wait_timeout",
                        "SurrealStorageConfig::with_drain_grace",
                        "SurrealStorageConfig::with_statement_timeout",
                        "SurrealStorageConfig::with_engine_timeouts",
                        "DEFAULT_SHUTDOWN_WAIT",
                        "DEFAULT_DRAIN_GRACE",
                        "DEFAULT_STATEMENT_TIMEOUT",
                        "DEFAULT_ENGINE_QUERY_TIMEOUT",
                        "DEFAULT_ENGINE_TRANSACTION_TIMEOUT",
                        "ShutdownReport",
                        "SurrealDatabase::with_lock_registry",
                        "SurrealDatabase::lock_registry",
                        "RETRY_EXHAUSTED_CONFLICT_CODE",
                        "LOCK_WAIT_TIMEOUT_CONFLICT_CODE"
                    ]
                }),
            ),
            section_with_json(
                "workflows",
                "Running the deterministic CI profile and the extended local profile",
                &load_profiles_md,
                json!({
                    "features": "surreal-test-support,test-utils",
                    "commands": [
                        {"profile": "ci_deterministic", "target": LOAD_TARGET, "test_filter": CI_TEST, "command": ci_command,
                         "workers": 16, "operations": 2000, "timeouts_ms": {"per_operation": 5000, "per_worker": 60000, "whole_test": 180000},
                         "report_file": "swarm-load-ci-<run_id>.json"},
                        {"profile": "extended_local", "target": LOAD_TARGET, "test_filter": EXTENDED_TEST, "command": extended_command,
                         "workers_min": 64, "operations_min": 50000, "dataset_records_min": 5000, "timeouts_ms": {"whole_test": 1800000},
                         "gate": "HANDSHAKE_SWARM_EXTENDED=1", "not_run_marker": "SWARM_EXTENDED=NOT_RUN_UNCONFIGURED",
                         "report_file": "swarm-load-extended-<run_id>.json", "memory_file": "swarm-load-extended-<run_id>-memory.json"},
                        {"profile": "semantics", "target": SEMANTICS_TARGET, "test_filter": null, "command": semantics_command},
                        {"profile": "lifecycle", "target": LIFECYCLE_TARGET, "test_filter": null, "command": lifecycle_command}
                    ],
                    "env_vars": [
                        {"name": "HANDSHAKE_ARTIFACTS_ROOT", "value": "absolute _Artifacts root", "read_by_swarm_tests": true, "consumer": "src/storage/tests.rs"},
                        {"name": "HANDSHAKE_WORKSPACE_ROOT", "value": "run-scoped directory", "read_by_swarm_tests": false, "consumer": "src/storage/mod.rs"},
                        {"name": "HANDSHAKE_SWARM_LOAD_REPORT_DIR", "value": "report directory", "read_by_swarm_tests": true},
                        {"name": "HANDSHAKE_SWARM_SEED", "value": "u64 workload seed", "read_by_swarm_tests": true},
                        {"name": "HANDSHAKE_SWARM_EXTENDED", "value": "1 enables the extended profile", "read_by_swarm_tests": true}
                    ],
                    "printed_markers": ["SWARM_LOAD_REPORT=", "SWARM_EXTENDED=NOT_RUN_UNCONFIGURED", "swarm-load-", "swarm-load-extended-", "-memory.json"],
                    "report_dir_fallback": "<HANDSHAKE_ARTIFACTS_ROOT>/handshake-test/swarm-load/"
                }),
            ),
            section_with_json(
                "schema",
                "Reading hsk.surreal_swarm_load_report@1",
                "The report is the JSON serialization of `SwarmLoadReport` \
                 (`storage/surreal/swarm_load_report.rs:181-216`). First run `SwarmLoadReport::validate` \
                 (`:218-262`) or apply the same rules by hand; a report that fails them is not evidence. Then \
                 read the fields in this order:\n\
                 - `schema_id` must equal `hsk.surreal_swarm_load_report@1` (`:28`).\n\
                 - `integrity_verdict`: `pass` (valid only with non-empty `reopen_integrity_counts_and_hashes`, \
                 `:290-297`); `lost_write`, `duplicate_effect`, `partial_commit`, `dirty_read` (correctness \
                 failures); `retry_exhausted`, `timeout`, `cancelled` (bound failures); `not_run` (`:85-95`). \
                 Anything but `pass` is a failed proof.\n\
                 - `reopen_integrity_counts_and_hashes`: per table `{ row_count, content_hash }` measured after \
                 real shutdown and reopen (`:165-169`, `:211-212`).\n\
                 - `retry_exhaustion_count`, `timeout_count`, `cancellation_count`: non-zero values classify the \
                 run as retry-exhausted, timed out or cancelled even when the integrity counts reconcile; in the \
                 CI profile a timeout fails the test (contract `load_profiles.ci_deterministic.hard_bound`). \
                 Report them with their class counts; never fold them into a pass.\n\
                 - `failed_by_operation_and_class`: failures per operation class split into `terminal`, \
                 `retry_exhausted`, `timeout`, `cancelled`, `lock_wait_timeout` (`:57-65`).\n\
                 - `operation_mix`: every required class (`point_read`, `range_or_search_query`, `create`, \
                 `idempotent_upsert`, `optimistic_versioned_update`, `delete`, \
                 `multi_record_projection_or_ledger_transaction`, `:30-40`) with its `share` and `status` \
                 `run`/`not_run`; a class marked `run` must have `attempted_by_operation > 0` (`:264-287`). A \
                 class silently missing or at zero is a defective run, not a pass.\n\
                 - `attempted_by_operation`, `succeeded_by_operation`: counts per class; succeeded can never \
                 exceed attempted.\n\
                 - `conflict_count`, `conflict_rate`, `retry_count`, `retry_rate`: every rate is \
                 `{ numerator, denominator, rate }` with `denominator > 0` and `rate = numerator / denominator` \
                 (`:130-150`, `:331-343`); never read `rate` without its numerator and denominator.\n\
                 - `lock_wait_ms_p50_p95_p99` and `latency_ms_p50_p95_p99_by_operation`: either \
                 `{ \"status\": \"measured\", p50_ms, p95_ms, p99_ms, sample_count }` with `sample_count > 0`, or \
                 `{ \"status\": \"not_run\" }` (`:113-128`, `:317-329`). `not_run` means no samples; it is NOT a \
                 zero-latency pass, and `measured` with `sample_count` 0 is invalid. Percentiles are \
                 nearest-rank (`:383-403`).\n\
                 - `maximum_concurrent_operations`: in-flight high-water mark; a value below 2 means the \
                 workers were globally serialized and the run proves nothing about parallelism.\n\
                 - `throughput_operations_per_second`, `shutdown_elapsed_ms`: record; compare only against runs \
                 with the same `machine_context` and `engine_mode`.\n\
                 - `engine_mode`: `embedded_rocks_db` (the only shipped mode) or `remote` (`:67-74`).\n\
                 - `remote_proof_status`: `not_run_unconfigured`, `pass`, `fail` (`:97-103`). \
                 `not_run_unconfigured` is never a PASS and never counts as remote evidence; `pass` is valid \
                 only with `engine_mode` `remote` (`:298-302`).\n\
                 - `run_id`, `source_commit`, `surrealdb_version`, `sdk_version`, `workload_seed`, \
                 `worker_count`, `operation_count`, `dataset_cardinality` `{ records, workspaces }`, \
                 `contention_ratio` (0..=1): identify and reproduce the run.\n\
                 - `machine_context` `{ cpu_model, logical_cpus, total_memory_bytes, store_drive_kind \
                 (hdd|ssd|unknown), os }`: absolute latency is meaningful only on the same machine class.\n\
                 - No string may contain `C:\\Users`, `C:/Users`, `/home/`, `/Users/`, `password` or `token=`; \
                 validation rejects the report otherwise (`:42-43`, `:305-381`).\n\n\
                 Decision procedure for a no-context model: (1) validate; (2) `integrity_verdict == pass`; \
                 (3) read `retry_exhaustion_count`, `timeout_count`, `cancellation_count` and the \
                 `failed_by_operation_and_class` classes; (4) every required class attempted > 0 and no \
                 `not_run` percentile for a class that ran; (5) `maximum_concurrent_operations >= 2`; \
                 (6) `remote_proof_status` is `not_run_unconfigured` for every embedded run - report it as such, \
                 never as pass.",
                json!({
                    "schema_id": "hsk.surreal_swarm_load_report@1",
                    "validator": "SwarmLoadReport::validate",
                    "fields": [
                        "schema_id", "run_id", "source_commit", "surrealdb_version", "sdk_version", "engine_mode",
                        "workload_seed", "worker_count", "operation_count", "dataset_cardinality", "operation_mix",
                        "contention_ratio", "attempted_by_operation", "succeeded_by_operation",
                        "failed_by_operation_and_class", "conflict_count", "conflict_rate", "retry_count", "retry_rate",
                        "retry_exhaustion_count", "lock_wait_ms_p50_p95_p99", "latency_ms_p50_p95_p99_by_operation",
                        "throughput_operations_per_second", "maximum_concurrent_operations", "timeout_count",
                        "cancellation_count", "shutdown_elapsed_ms", "reopen_integrity_counts_and_hashes",
                        "integrity_verdict", "remote_proof_status", "machine_context"
                    ],
                    "integrity_verdict": ["pass", "lost_write", "duplicate_effect", "partial_commit", "dirty_read", "retry_exhausted", "timeout", "cancelled", "not_run"],
                    "remote_proof_status": ["not_run_unconfigured", "pass", "fail"],
                    "failure_classes": ["terminal", "retry_exhausted", "timeout", "cancelled", "lock_wait_timeout"],
                    "engine_mode": ["embedded_rocks_db", "remote"],
                    "percentile_status": ["measured", "not_run"],
                    "rules": [
                        "not_run percentile is not a zero-latency pass",
                        "rates carry numerator and denominator",
                        "not_run_unconfigured is never a PASS",
                        "no user-profile paths or credential-looking strings"
                    ]
                }),
            ),
            section_with_json(
                "purpose",
                "Current embedded proof versus future configured remote proof",
                "Current proof (every MT-142 run): `engine_mode` `embedded_rocks_db`, one in-process engine, \
                 workers are cloned wrappers, independent clients are two wrappers over that one engine \
                 without a shared keyed-lock registry. It proves transaction-owned correctness, bounded retry \
                 and bounded shutdown for the embedded single-owner topology and nothing about network \
                 clients, multiple processes or multiple nodes; `remote_proof_status` is \
                 `not_run_unconfigured`.\n\n\
                 Future remote proof (not run, not configured, no default): a configured remote WS/HTTP \
                 endpoint would receive the typed `QueryError::TransactionConflict` (wire code -32009) for \
                 commit conflicts, client-side transactions are unavailable over HTTP, and the \
                 `BEGIN ... COMMIT` query-string pattern and the typed-first classifier work unchanged \
                 (research basis `validation_plan.future_remote_behavior_distinguished`, labelled ASSUMPTION \
                 until a configured endpoint run exists). Only such a run may set `remote_proof_status` to \
                 `pass` or `fail`, and only with `engine_mode` `remote`. Do not derive any remote claim from an \
                 embedded report, and do not simulate a remote topology by opening the same store path twice.",
                json!({
                    "current": {"engine_mode": "embedded_rocks_db", "remote_proof_status": "not_run_unconfigured", "independent_clients": "two wrappers over one engine, no shared registry"},
                    "future_remote": {"engine_mode": "remote", "status_values": ["pass", "fail"], "requires": "configured endpoint run", "label": "ASSUMPTION until run"},
                    "forbidden": ["deriving remote claims from embedded reports", "opening the same store path twice as a remote simulation"]
                }),
            ),
            section(
                "recovery",
                "Recovery",
                "1. `409` with `detail` `HSK-STORAGE-RETRY-EXHAUSTED`: the write was not applied; read the \
                 record, back off, and retry from the caller with the same idempotency key or a fresh \
                 `expected_version`.\n\
                 2. `409` with `detail` `HSK-STORAGE-LOCK-WAIT-TIMEOUT`: a keyed-lock wait outlived the \
                 statement timeout; nothing was written; retry from the caller once the holder finishes.\n\
                 3. `StatementTimeout`: the attempt's outcome is unknown; read the record before deciding to \
                 resend.\n\
                 4. `409` `expected_version is stale`: re-read, rebase, resend; never loop without re-reading.\n\
                 5. `embedded database is closed`: the backend is shutting down or closed; wait for the \
                 restart, then re-issue.\n\
                 6. `ShutdownStillInProgress`: the close continues in the background; do not start another \
                 backend on the same `HANDSHAKE_DATA_DIR` until `SurrealStorage::open` succeeds on it.\n\
                 7. `IO error: Failed to create lock file`: another engine holds `<store>/LOCK`; stop that \
                 process instead of deleting the store or the `LOCK` file.\n\
                 8. `integrity_verdict` other than `pass`: preserve the run's store and report, record \
                 `run_id`, `workload_seed`, `source_commit`, and reproduce with the same `HANDSHAKE_SWARM_SEED` \
                 before changing code.",
            ),
        ],
        anchors: vec![
            page_link("embedded-store-recovery"),
            page_link("atelier-storage-authority"),
            page_link("startup-and-run-commands"),
            page_link("state-recovery-guide"),
        ],
    }
}

fn page_state_recovery_guide() -> NewUserManualPage {
    NewUserManualPage {
        slug: "state-recovery-guide".into(),
        title: "State Recovery — Compaction, Interruptions, Failed Builds, Validation Reentry"
            .into(),
        page_kind: "state_recovery",
        audience: "model",
        spec_anchors: vec!["10.15.8".into(), "2.3.13.11".into()],
        sections: vec![
            section(
                "recovery",
                "After session compaction (no chat memory)",
                "Chat history is NOT state. Recover from product authority:\n\n\
                 1. `GET /usermanual/pages/manual-toc` — re-learn the surface map (this manual \
                 is the bootstrap surface; anonymous reads allowed).\n\
                 2. Re-read your task contract (WP/MT JSON under `.GOV/task_packets/...`) — it \
                 is the binding scope, not your recollection.\n\
                 3. Replay your receipts: every write you made left an EventLedger receipt \
                 (`kernel_event_ledger`); correlation/session ids reconstruct what happened.\n\
                 4. Check repair queues ([[repair-queues-and-staleness]]) for work your \
                 interruption orphaned.",
            ),
            section(
                "recovery",
                "After an interrupted microtask",
                "1. `git -C <worktree> log --oneline -5` and `git status --short` — what landed \
                 vs what is uncommitted.\n\
                 2. Re-run the MT's scoped test target (named in the MT contract) — GREEN means \
                 the closure unit may already hold; RED names the next edit.\n\
                 3. Lifecycle evidence in the MT JSON records the last proven state — trust the \
                 recorded evidence over memory.\n\
                 4. Never re-do a write blindly: check for its receipt first (idempotency keys \
                 make safe re-runs explicit).",
            ),
            section_with_json(
                "recovery",
                "Parallel swarm operation and recovery",
                "Parallel local/cloud agents recover from the embedded SurrealDB/EventLedger swarm \
                 surface, not from chat history or UI state. Use the live runtime symbols as the \
                 recovery map:\n\n\
                 - `AgentLaneIdentity` names the lane, actor, provider attribution, and \
                 capability set.\n\
                 - `claim_work_surface` acquires or holds worktree/workspace/rich-document \
                 claims; expired claims are reclaimed before a new owner resumes.\n\
                 - `record_role_mailbox_handoff` records validator/operator handoff state \
                 (`progress`, `pass`, `fail`) with mailbox thread/message ids.\n\
                 - `resolve_backend_navigation_quiet` resolves backend navigation commands \
                 without foreground windows and records quiet background work.\n\
                 - `record_checkpoint` writes restartable recovery checkpoints; \
                 `recover_from_checkpoint` verifies the payload hash before emitting a recovery \
                 receipt.\n\
                 - `enqueue_indexing_lease` / `try_acquire_indexing_lease` serialize parallel \
                 index writers per scope; queued writers promote before newcomers after stale \
                 lease reclaim.\n\
                 - `record_quiet_background_work` records no-window/no-focus quiet work receipts.\n\
                 - `project_swarm_dashboard` projects claims, handoffs, checkpoints, recovery \
                 receipts, indexing leases, and quiet work into a bounded dashboard view.\n\
                 - `build_handoff_compression_template` creates a bounded resume template from \
                 existing checkpoint authority; it is a projection, not a second authority.\n\n\
                 Negative recovery proofs to cite before marking swarm work ready: \
                 `mt223_interrupted_indexing_start_failure_leaves_no_swarm_or_kir_receipts`, \
                 `mt223_quiet_receipt_failure_rolls_back_index_run_and_lease`, \
                 `mt223_stale_indexing_lease_enqueue_does_not_leapfrog_queued_writer`, and \
                 `mt223_restart_after_crash_reconstructs_swarm_state_from_surreal` (legacy test name). These \
                 prove false receipts are not emitted, queue order survives stale reclaim, and \
                 a fresh embedded SurrealDB store can reconstruct state from durable authority alone.",
                json!({
                    "runtime_symbols": [
                        "AgentLaneIdentity",
                        "claim_work_surface",
                        "record_role_mailbox_handoff",
                        "resolve_backend_navigation_quiet",
                        "record_checkpoint",
                        "recover_from_checkpoint",
                        "enqueue_indexing_lease",
                        "try_acquire_indexing_lease",
                        "record_quiet_background_work",
                        "project_swarm_dashboard",
                        "build_handoff_compression_template"
                    ],
                    "negative_case_tests": [
                        "mt223_interrupted_indexing_start_failure_leaves_no_swarm_or_kir_receipts",
                        "mt223_quiet_receipt_failure_rolls_back_index_run_and_lease",
                        "mt223_stale_indexing_lease_enqueue_does_not_leapfrog_queued_writer",
                        "mt223_restart_after_crash_reconstructs_swarm_state_from_surreal"
                    ],
                    "authority": [
                        "embedded SurrealDB",
                        "kernel_event_ledger",
                        "knowledge_agent_worktree_claims",
                        "knowledge_agent_role_mailbox_handoffs",
                        "knowledge_agent_state_recovery_checkpoints",
                        "knowledge_agent_recovery_receipts",
                        "knowledge_parallel_indexing_lease_queue",
                        "knowledge_agent_quiet_background_work"
                    ]
                }),
            ),
            section(
                "recovery",
                "After a failed build",
                "1. Re-run the SCOPED build: `cargo test -p handshake_core --features test-utils \
                 --test <target>` (one cargo invocation at a time; lock waits under a shared \
                 target dir are normal — never kill a peer's build).\n\
                 2. Read the FIRST compile error; later errors usually cascade.\n\
                 3. If the failure names a missing table, compare the exact declarative catalog in \
                 `storage/surreal/schema.surql` with the live structured catalog fingerprint.\n\
                 4. An embedded SurrealDB availability failure is not a pass — repair the configured store path.",
            ),
            section(
                "recovery",
                "Validation reentry",
                "1. `GET /usermanual/freshness` — the manual-vs-product drift verdicts.\n\
                 2. Re-run the surface's fixture tests (negative paths must stay red-on-defect).\n\
                 3. Cite receipts + test names + counts in the validation evidence; validator \
                 verdicts advance only on runtime proof, not status text (DEC-007).",
            ),
        ],
        anchors: vec![
            page_link("repair-queues-and-staleness"),
            page_link("manual-toc"),
            page_link("backend-navigation-and-identity"),
            page_link("quickstart-state-recovery"),
            route_anchor("GET", "/usermanual/freshness"),
        ],
    }
}

fn page_kernel_write_governance() -> NewUserManualPage {
    // Deterministic import of the kernel002 no-context manual topics
    // (UMMIG-002): the legacy typed struct remains the seed source until the
    // acceptance-run consumers migrate.
    let kernel_manual = kernel002_no_context_model_manual();
    let mut sections = vec![section(
        "purpose",
        "Why this page exists",
        "Models that WRITE through kernel-governed paths (write boxes, promotions, CRDT \
         workspaces, action catalog) follow the Kernel002 write-governance manual. This page is \
         the canonical UserManual home of those topics (imported deterministically from the \
         legacy `kernel002-no-context-model-manual-v1`; see [[legacy-model-manual-bridge]]).",
    )];
    for kernel_section in kernel_manual.sections {
        sections.push(section_with_json(
            "workflows",
            kernel_section.title,
            &kernel_section
                .instructions
                .iter()
                .map(|line| format!("- {line}\n"))
                .collect::<String>(),
            json!({ "instructions": kernel_section.instructions }),
        ));
    }
    NewUserManualPage {
        slug: "kernel-write-governance".into(),
        title: "Kernel Write Governance (Kernel002 Topics)".into(),
        page_kind: "workflow",
        audience: "model",
        spec_anchors: vec!["10.15.8".into()],
        sections,
        anchors: vec![page_link("legacy-model-manual-bridge")],
    }
}

fn page_legacy_bridge() -> NewUserManualPage {
    let plan = naming_migration_plan();
    NewUserManualPage {
        slug: "legacy-model-manual-bridge".into(),
        title: "Legacy ModelManual Bridge".into(),
        page_kind: "legacy_bridge",
        audience: "model_and_operator",
        spec_anchors: vec!["10.15.8".into()],
        sections: vec![
            section(
                "purpose",
                "The bridge law",
                "UserManual is the canonical term (operator decision; spec 10.15.8). Legacy \
                 `ModelManual` / `model_manual` paths remain ONLY while they map \
                 deterministically onto UserManual authority and emit a compatibility receipt \
                 when used. The mapping is queryable: `GET /usermanual/legacy/aliases`; the \
                 bridge route `GET /usermanual/legacy/model-manual` answers legacy callers with \
                 the canonical payload AND a `KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED` \
                 compatibility receipt.",
            ),
            section_with_json(
                "navigation",
                "Migration plan",
                "The full machine-readable plan: `GET /usermanual/migration-plan`. Summary of \
                 phases:\n\n\
                 - **P1 (this WP)**: canonical `user_manual` module + embedded SurrealDB authority + \
                 aliases + receipts (DONE by MT-193..MT-208).\n\
                 - **P2 (frontend lane)**: rename Tauri commands \
                 (`model_manual_get` -> canonical `/usermanual` routes), app help surface.\n\
                 - **P3 (later WP)**: retire the static legacy module files.",
                json!({
                    "rows": plan.rows
                        .iter()
                        .map(|r| json!({
                            "row_id": r.row_id,
                            "legacy_id": r.legacy_id,
                            "canonical_ref": r.canonical_ref,
                            "phase": r.phase.as_str(),
                            "shim_state": r.shim_state.as_str(),
                        }))
                        .collect::<Vec<_>>()
                }),
            ),
        ],
        anchors: vec![
            route_anchor("GET", "/usermanual/legacy/model-manual"),
            route_anchor("GET", "/usermanual/legacy/aliases"),
            route_anchor("GET", "/usermanual/migration-plan"),
            spec_anchor("10.15.8"),
        ],
    }
}

// ---------------------------------------------------------------------------
// MT-199 quickstart pages.
// ---------------------------------------------------------------------------

pub const QUICKSTART_AREAS: &[&str] = &[
    "index",
    "editor",
    "loom",
    "retrieval",
    "validation",
    "state-recovery",
];

fn quickstart(
    area: &str,
    title: &str,
    steps_md: &str,
    anchors: Vec<NewManualAnchor>,
) -> NewUserManualPage {
    NewUserManualPage {
        slug: format!("quickstart-{area}"),
        title: title.into(),
        page_kind: "quickstart",
        audience: "model",
        spec_anchors: vec!["10.15.8".into()],
        sections: vec![
            section(
                "startup",
                "Prerequisites",
                "Backend running on `127.0.0.1:37501` ([[startup-and-run-commands]]); knowledge \
                 surfaces need identity headers ([[backend-navigation-and-identity]]).",
            ),
            section("workflows", "Steps", steps_md),
        ],
        anchors,
    }
}

fn quickstart_pages() -> Vec<NewUserManualPage> {
    vec![
        quickstart(
            "index",
            "Quickstart — Index A Project",
            "1. `GET /knowledge/ingestion/roots` — see configured roots.\n\
             2. `POST /knowledge/ingestion/runs` — index; watch for \
             `KNOWLEDGE_INDEX_RUN_COMPLETED`.\n\
             3. `GET /knowledge/code/symbols?workspace_id=<ws>&name=<symbol>` — find symbols.\n\
             4. `GET /knowledge/code/symbols/:entity_id/references` — navigate the graph.\n\
             5. `GET /knowledge/ingestion/repairs` — confirm the queue is empty (or work it).",
            vec![
                page_link("knowledge-index-surface"),
                page_link("startup-and-run-commands"),
                page_link("backend-navigation-and-identity"),
            ],
        ),
        quickstart(
            "editor",
            "Quickstart — Rich Document Editing",
            "1. `POST /workspaces` {name} — get a workspace id.\n\
             2. `POST /knowledge/documents` {workspace_id, title, content_json} — doc_version 1.\n\
             3. `PUT /knowledge/documents/:id/save` {expected_version: 1, content_json} — \
             version 2; a 409 means reload + merge.\n\
             4. `GET /knowledge/documents/:id/history?limit=10&offset=0` — revisions.\n\
             5. `GET /knowledge/documents/:id/projection?format=html` — the primary export \
             projection.",
            vec![
                page_link("rich-documents-surface"),
                page_link("permissions-and-safety"),
                page_link("startup-and-run-commands"),
            ],
        ),
        quickstart(
            "loom",
            "Quickstart — Notes/Loom Navigation",
            "1. `POST /workspaces` {name} — workspace.\n\
             2. `POST /workspaces/:ws/loom/blocks` — create two blocks.\n\
             3. `POST /workspaces/:ws/loom/edges` — link them.\n\
             4. `GET /workspaces/:ws/loom/blocks/:id/backlinks` — backlinks with context.\n\
             5. `GET /workspaces/:ws/loom/graph/local?...` — the local graph.\n\
             6. `GET /workspaces/:ws/loom/graph-search?q=<term>` — search with \
             `hsk.loom_retrieval_bias@1` reasons on Loom block hits.\n\
             7. `GET /workspaces/:ws/loom/blocks/:id/knowledge` — the authority bridge row \
             (entity + receipt).",
            vec![
                page_link("notes-loom-surface"),
                page_link("startup-and-run-commands"),
            ],
        ),
        quickstart(
            "retrieval",
            "Quickstart — Retrieval And Context Bundles",
            "1. `GET /knowledge/retrieval/catalog` — modes and scopes.\n\
             2. Load a bundle: `GET /knowledge/retrieval/bundles/:bundle_id` (items carry \
             citations + decisions).\n\
             3. `GET .../staleness` — verify before consuming.\n\
             4. `POST .../repair` — recompile when stale.\n\
             5. `GET .../export` — the AI-ready evidence manifest.",
            vec![
                page_link("retrieval-and-context-bundles-surface"),
                page_link("backend-navigation-and-identity"),
            ],
        ),
        quickstart(
            "validation",
            "Quickstart — Validation",
            "1. For a migrated target named in the legacy-to-Surreal validation matrix, run its \
             SCOPED test target on real embedded SurrealDB ([[startup-and-run-commands]]). Before \
             running any other `<surface>_tests` target, inspect that target's current backend; \
             unmigrated legacy targets can still contain legacy server database setup or skip behavior.\n\
             2. For a migrated embedded target, SurrealDB availability failure is NOT a pass — \
             repair the configured store path.\n\
             3. Check negative fixtures stay red-on-defect (stale, missing, denied, conflict \
             paths).\n\
             4. `GET /usermanual/freshness` — manual-vs-product drift must be `current`.\n\
             5. Cite test names + counts + receipt ids in evidence; runtime proof only \
             (DEC-007: status text proves nothing).",
            vec![
                page_link("startup-and-run-commands"),
                route_anchor("GET", "/usermanual/freshness"),
            ],
        ),
        quickstart(
            "state-recovery",
            "Quickstart — State Recovery",
            "1. `GET /usermanual/pages/state-recovery-guide` — the full guide.\n\
             2. `curl http://127.0.0.1:37501/health` — is the product up?\n\
             3. Re-read your MT contract; replay your EventLedger receipts.\n\
             4. Work the repair queues ([[repair-queues-and-staleness]]).\n\
             5. Re-run the smallest scoped test for your surface.",
            vec![
                page_link("state-recovery-guide"),
                page_link("repair-queues-and-staleness"),
            ],
        ),
    ]
}

// ---------------------------------------------------------------------------
// MT-197 tool + feature entries.
// ---------------------------------------------------------------------------

fn group_common_errors(group: SurfaceGroup) -> Vec<String> {
    match group {
        SurfaceGroup::KnowledgeIngestion => vec![
            "400 bad_request (missing identity headers / malformed params)".into(),
            "404 not_found (unknown root/source/repair id)".into(),
            "409 conflict / policy_denied (allowlist or secret policy refused)".into(),
            "io_error (source unreadable; queues a repair)".into(),
            "500 internal_error (embedded SurrealDB unavailable; fail-closed)".into(),
        ],
        SurfaceGroup::CodeNavigation => vec![
            "400 bad_request (missing identity headers)".into(),
            "404 not_found (unknown symbol/file)".into(),
            "serialize_failed / receipt_build_failed".into(),
            "500 internal_error (fail-closed storage path)".into(),
        ],
        SurfaceGroup::RichDocuments => vec![
            "400 bad_request (missing headers, unknown actor-kind token, embed-target violation)".into(),
            "403 forbidden (cloud_model_write_denied / validator_write_denied / unauthenticated_write_denied)".into(),
            "404 not_found (unknown document/revision/embed)".into(),
            "409 conflict (stale expected_version)".into(),
            "500 internal_error / receipt_build_failed (fail-closed)".into(),
        ],
        SurfaceGroup::Retrieval => vec![
            "400 bad_request (missing identity headers)".into(),
            "404 not_found (unknown bundle)".into(),
            "stale bundle verdicts (source_stale / missing evidence) — check /staleness".into(),
            "500 internal_error (fail-closed)".into(),
        ],
        SurfaceGroup::MemoryClaims => vec![
            "400 bad_request (missing identity headers)".into(),
            "404 not_found (unknown claim/fact/entity)".into(),
            "500 internal_error (fail-closed)".into(),
        ],
        SurfaceGroup::CrdtCollaboration => vec![
            "400 malformed update payload".into(),
            "409 conflicting head / stale state vector".into(),
            "lease write denied (expired/foreign lease)".into(),
        ],
        SurfaceGroup::NotesLoom => vec![
            "400 HSK-400-LOOM-VALIDATION (malformed payload)".into(),
            "404 workspace_not_found / not_found".into(),
            "403 HSK-403-SILENT-EDIT (unattributed write refused)".into(),
            "500 HSK-500-LOOM".into(),
        ],
        SurfaceGroup::UserManual => vec![
            "400 bad_request (empty query / bad token)".into(),
            "404 not_found (unknown slug/tool/area)".into(),
            "403 forbidden (resync by cloud_model/unauthenticated)".into(),
        ],
    }
}

fn group_recovery_steps(group: SurfaceGroup) -> Vec<String> {
    match group {
        SurfaceGroup::KnowledgeIngestion => vec![
            "List the repair queue (GET /knowledge/ingestion/repairs) and retry rows".into(),
            "Re-run the index (POST /knowledge/ingestion/runs) — idempotent on relationship ids".into(),
        ],
        SurfaceGroup::CodeNavigation => vec![
            "Stale symbol verdicts heal on the next successful index run of the owning file".into(),
            "Missing symbols: confirm the file's root is registered and the run completed".into(),
        ],
        SurfaceGroup::RichDocuments => vec![
            "409: reload the document, merge, re-save with the fresh expected_version".into(),
            "Broken embeds: GET .../embeds/broken then POST embeds/:embed_id/repair (relink|reresolve|remove)".into(),
            "Backlink drift: POST /knowledge/documents/:id/backlinks rebuilds".into(),
        ],
        SurfaceGroup::Retrieval => vec![
            "POST /knowledge/retrieval/bundles/:id/repair recompiles a stale bundle".into(),
            "Re-ingest vanished sources first, then repair the bundle".into(),
        ],
        SurfaceGroup::MemoryClaims => vec![
            "Resolve conflicts via the conflict-resolution flow (receipted)".into(),
        ],
        SurfaceGroup::CrdtCollaboration => vec![
            "Pull current head, merge locally, re-push".into(),
            "Inspect conflict_state for lease holders before takeover".into(),
        ],
        SurfaceGroup::NotesLoom => vec![
            "Regenerate stale wiki projections (POST .../regenerate)".into(),
            "Recompute metrics (POST .../loom/metrics/recompute)".into(),
        ],
        SurfaceGroup::UserManual => vec![
            "POST /usermanual/resync (gated) re-seeds changed pages idempotently".into(),
            "GET /usermanual/freshness names the exact stale/uncovered/dangling item".into(),
        ],
    }
}

fn seed_tool_entries() -> Vec<UserManualToolEntry> {
    let mut tools = Vec::new();

    // WP-009 surfaces from the registry (origin wp009_surface). MT-197 +
    // closes the MT-112 deferred manual registration for /knowledge/code/*.
    for s in wp009_surface_registry() {
        let content_hash = sha256_hex(
            &serde_json::to_string(&json!({
                "surface_id": s.surface_id,
                "method": s.method,
                "route": s.route,
                "summary": s.summary,
                "expected_input": s.expected_input,
                "expected_output": s.expected_output,
                "manual_version": USER_MANUAL_VERSION,
            }))
            .expect("surface serializes"),
        );
        tools.push(UserManualToolEntry {
            tool_id: s.surface_id.to_string(),
            page_id: None,
            name: format!("{} {}", s.method, s.route),
            status: "wired".into(),
            ipc_channel: None,
            tauri_command: None,
            cli_flag: None,
            http_route: Some(s.route.to_string()),
            http_method: s.method.to_string(),
            description: s.summary.to_string(),
            expected_input: s.expected_input.to_string(),
            expected_output: s.expected_output.to_string(),
            schema_fields: Vec::new(),
            common_errors: group_common_errors(s.group),
            recovery_steps: group_recovery_steps(s.group),
            origin: "wp009_surface".into(),
            content_hash,
            manual_version: USER_MANUAL_VERSION.into(),
        });
    }

    // Legacy static manifest import (origin legacy_model_manual): the
    // deterministic 10.15.8 mapping — every legacy CommandReference becomes a
    // canonical tool entry, preserving content exactly.
    for command in model_manual().command_reference {
        let status = match command.status {
            CommandStatus::Wired => "wired",
            CommandStatus::Planned => "planned",
        };
        let content_hash = sha256_hex(
            &serde_json::to_string(&json!({
                "id": command.id,
                "name": command.name,
                "status": status,
                "ipc_channel": command.ipc_channel,
                "tauri_command": command.tauri_command,
                "cli_flag": command.cli_flag,
                "description": command.description,
                "expected_input": command.expected_input,
                "expected_output": command.expected_output,
                "schema_fields": command.schema_fields,
                "common_errors": command.common_errors,
                "recovery_steps": command.recovery_steps,
                "manual_version": USER_MANUAL_VERSION,
            }))
            .expect("command serializes"),
        );
        tools.push(UserManualToolEntry {
            tool_id: command.id.to_string(),
            page_id: None,
            name: command.name.to_string(),
            status: status.into(),
            ipc_channel: command.ipc_channel.map(str::to_string),
            tauri_command: command.tauri_command.map(str::to_string),
            cli_flag: command.cli_flag.map(str::to_string),
            http_route: None,
            http_method: String::new(),
            description: command.description.to_string(),
            expected_input: command.expected_input.to_string(),
            expected_output: command.expected_output.to_string(),
            schema_fields: command
                .schema_fields
                .iter()
                .map(|s| s.to_string())
                .collect(),
            common_errors: command
                .common_errors
                .iter()
                .map(|s| s.to_string())
                .collect(),
            recovery_steps: command
                .recovery_steps
                .iter()
                .map(|s| s.to_string())
                .collect(),
            origin: "legacy_model_manual".into(),
            content_hash,
            manual_version: USER_MANUAL_VERSION.into(),
        });
    }

    tools
}

fn seed_feature_entries() -> Vec<UserManualFeatureEntry> {
    let mut features = Vec::new();

    // One feature entry per WP-009 surface group.
    for group in [
        SurfaceGroup::KnowledgeIngestion,
        SurfaceGroup::CodeNavigation,
        SurfaceGroup::RichDocuments,
        SurfaceGroup::Retrieval,
        SurfaceGroup::MemoryClaims,
        SurfaceGroup::CrdtCollaboration,
        SurfaceGroup::NotesLoom,
        SurfaceGroup::UserManual,
    ] {
        let tool_ids: Vec<String> = wp009_surface_registry()
            .iter()
            .filter(|s| s.group == group)
            .map(|s| s.surface_id.to_string())
            .collect();
        let title = format!("WP-009 {}", group.as_str().replace('_', " "));
        let description = format!(
            "WP-KERNEL-009 {} surfaces; documented on UserManual page '{}'.",
            group.as_str().replace('_', " "),
            group.page_slug()
        );
        let content_hash = sha256_hex(
            &serde_json::to_string(&json!({
                "group": group.as_str(),
                "title": title,
                "description": description,
                "tool_ids": tool_ids,
                "manual_version": USER_MANUAL_VERSION,
            }))
            .expect("feature serializes"),
        );
        features.push(UserManualFeatureEntry {
            feature_id: format!("wp009.{}", group.as_str()),
            title,
            description,
            tool_ids,
            origin: "wp009_surface".into(),
            content_hash,
            manual_version: USER_MANUAL_VERSION.into(),
        });
    }

    // Legacy feature groups, imported deterministically.
    for group in model_manual().feature_groups {
        let tool_ids: Vec<String> = group.commands.iter().map(|c| c.to_string()).collect();
        let content_hash = sha256_hex(
            &serde_json::to_string(&json!({
                "id": group.id,
                "title": group.title,
                "description": group.description,
                "tool_ids": tool_ids,
                "manual_version": USER_MANUAL_VERSION,
            }))
            .expect("legacy feature serializes"),
        );
        features.push(UserManualFeatureEntry {
            feature_id: group.id.to_string(),
            title: group.title.to_string(),
            description: group.description.to_string(),
            tool_ids,
            origin: "legacy_model_manual".into(),
            content_hash,
            manual_version: USER_MANUAL_VERSION.into(),
        });
    }

    features
}

fn seed_aliases() -> Vec<LegacyAliasRow> {
    naming_migration_plan()
        .aliases
        .iter()
        .map(|a| LegacyAliasRow {
            alias: a.alias.to_string(),
            alias_kind: a.alias_kind.as_str().to_string(),
            canonical_kind: a.canonical_kind.to_string(),
            canonical_ref: a.canonical_ref.to_string(),
            deprecation_note: a.deprecation_note.to_string(),
            manual_version: USER_MANUAL_VERSION.to_string(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The idempotent seeder.
// ---------------------------------------------------------------------------

/// Seed (or re-sync) the UserManual corpus into embedded SurrealDB. Idempotent: rows
/// short-circuit on content hash; receipts are appended only for changed
/// pages plus one summary receipt when anything changed. Always records the
/// `user_manual_versions` row.
pub async fn ensure_seeded(db: &SurrealDatabase) -> StorageResult<SeedReport> {
    let store = UserManualStore::new(db);
    let corpus = seed_corpus();
    let seed_hash = corpus_hash(&corpus);

    let mut pages_changed = 0usize;
    for page in &corpus.pages {
        let (_, changed) = store
            .upsert_page(page, USER_MANUAL_VERSION, "current")
            .await?;
        if changed {
            pages_changed += 1;
        }
    }
    let mut tools_changed = 0usize;
    for tool in &corpus.tools {
        if store.upsert_tool_entry(tool).await? {
            tools_changed += 1;
        }
    }
    let mut features_changed = 0usize;
    for feature in &corpus.features {
        if store.upsert_feature_entry(feature).await? {
            features_changed += 1;
        }
    }
    let mut aliases_changed = 0usize;
    for alias in &corpus.aliases {
        if store.upsert_legacy_alias(alias).await? {
            aliases_changed += 1;
        }
    }

    let anything_changed = pages_changed + tools_changed + features_changed + aliases_changed > 0;
    let existing_version = store.get_version(USER_MANUAL_VERSION).await?;
    let version_receipt = if anything_changed || existing_version.is_none() {
        Some(
            store
                .record_version_with_receipt(
                    USER_MANUAL_VERSION,
                    &seed_hash,
                    corpus.pages.len() as i32,
                    corpus.tools.len() as i32,
                    corpus.features.len() as i32,
                    json!({
                        "seed_content_hash": seed_hash,
                        "pages_changed": pages_changed,
                        "tools_changed": tools_changed,
                        "features_changed": features_changed,
                        "aliases_changed": aliases_changed,
                    }),
                    "WP-KERNEL-009 MT-193..MT-208 built-in seed corpus",
                )
                .await?,
        )
    } else {
        None
    };

    Ok(SeedReport {
        manual_version: USER_MANUAL_VERSION.into(),
        seed_content_hash: seed_hash,
        pages_total: corpus.pages.len(),
        pages_changed,
        tools_total: corpus.tools.len(),
        tools_changed,
        features_total: corpus.features.len(),
        features_changed,
        aliases_total: corpus.aliases.len(),
        aliases_changed,
        version_receipt_event_id: version_receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn corpus_slugs_are_unique_and_kebab() {
        let corpus = seed_corpus();
        let mut slugs = BTreeSet::new();
        for page in &corpus.pages {
            assert!(slugs.insert(page.slug.clone()), "dup slug {}", page.slug);
            assert_eq!(page.slug, page.slug.to_lowercase());
            assert!(!page.slug.contains(' '));
            assert!(!page.sections.is_empty(), "{} has no sections", page.slug);
        }
    }

    #[test]
    fn every_section_body_json_matches_the_surreal_object_contract() {
        for page in seed_corpus().pages {
            for section in page.sections {
                if let Some(body_json) = section.body_json {
                    assert!(
                        body_json.is_object(),
                        "{}.{} body_json must be an object for the embedded SurrealDB schema",
                        page.slug,
                        section.title
                    );
                }
            }
        }
    }

    #[test]
    fn page_anchor_identities_are_unique_for_the_surreal_index() {
        for page in seed_corpus().pages {
            let mut identities = BTreeSet::new();
            for anchor in page.anchors {
                let identity = (
                    anchor.anchor_kind,
                    anchor.anchor_value.clone(),
                    anchor.http_method,
                );
                assert!(
                    identities.insert(identity),
                    "{} repeats anchor identity ({}, {}, {})",
                    page.slug,
                    anchor.anchor_kind,
                    anchor.anchor_value,
                    anchor.http_method
                );
            }
        }
    }

    #[test]
    fn toc_links_every_page_and_every_page_is_reachable() {
        let corpus = seed_corpus();
        let slugs: BTreeSet<String> = corpus.pages.iter().map(|p| p.slug.clone()).collect();
        // Every page_link anchor targets an existing page (no dangling links).
        for page in &corpus.pages {
            for anchor in &page.anchors {
                if anchor.anchor_kind == "page_link" {
                    assert!(
                        slugs.contains(&anchor.anchor_value),
                        "{} links to missing page {}",
                        page.slug,
                        anchor.anchor_value
                    );
                }
            }
        }
        // Every non-TOC page is reachable from manual-toc (BFS over page_link).
        let mut reachable = BTreeSet::new();
        let mut queue = vec!["manual-toc".to_string()];
        while let Some(slug) = queue.pop() {
            if !reachable.insert(slug.clone()) {
                continue;
            }
            if let Some(page) = corpus.pages.iter().find(|p| p.slug == slug) {
                for anchor in &page.anchors {
                    if anchor.anchor_kind == "page_link" {
                        queue.push(anchor.anchor_value.clone());
                    }
                }
            }
        }
        for slug in &slugs {
            assert!(
                reachable.contains(slug),
                "page {} is not reachable from manual-toc",
                slug
            );
        }
    }

    #[test]
    fn every_registry_surface_is_anchor_covered_in_the_corpus() {
        // The MT-195 build-update gate, compile-time edition: every registry
        // surface must be documented by an http_route anchor on some page.
        let corpus = seed_corpus();
        let mut covered = BTreeSet::new();
        for page in &corpus.pages {
            for anchor in &page.anchors {
                if anchor.anchor_kind == "http_route" {
                    covered.insert((anchor.http_method, anchor.anchor_value.clone()));
                }
            }
        }
        for s in wp009_surface_registry() {
            assert!(
                covered.contains(&(s.method, s.route.to_string())),
                "registry surface {} {} ({}) has NO UserManual route anchor — \
                 update the seed corpus in the same implementation unit (spec 10.15.8)",
                s.method,
                s.route,
                s.surface_id
            );
        }
    }

    #[test]
    fn tool_catalog_covers_registry_and_legacy_without_id_collisions() {
        let corpus = seed_corpus();
        let mut ids = BTreeSet::new();
        for tool in &corpus.tools {
            assert!(
                ids.insert(tool.tool_id.clone()),
                "dup tool id {}",
                tool.tool_id
            );
        }
        for s in wp009_surface_registry() {
            assert!(
                ids.contains(s.surface_id),
                "registry surface {} missing from tool catalog",
                s.surface_id
            );
        }
        for command in crate::model_manual::model_manual().command_reference {
            assert!(
                ids.contains(command.id),
                "legacy command {} missing from tool catalog",
                command.id
            );
        }
    }

    #[test]
    fn corpus_hash_is_deterministic() {
        assert_eq!(corpus_hash(&seed_corpus()), corpus_hash(&seed_corpus()));
    }

    #[test]
    fn quickstart_pages_cover_all_contract_areas() {
        let corpus = seed_corpus();
        for area in QUICKSTART_AREAS {
            let slug = format!("quickstart-{area}");
            assert!(
                corpus.pages.iter().any(|p| p.slug == slug),
                "missing quickstart page {slug}"
            );
        }
    }

    #[test]
    fn mt137_manual_covers_embedded_lifecycle_persistence() {
        let page = seed_corpus()
            .pages
            .into_iter()
            .find(|page| page.slug == "embedded-store-recovery")
            .expect("embedded storage behavior page");
        let body = page
            .sections
            .iter()
            .map(|section| section.body_md.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for required in [
            "kernel_process_lifecycle",
            "kernel_session_checkpoint",
            "kernel_event_ledger",
            "Every duplicate checkpoint id is ignored without replacing the original row",
            "later independent checkpoints in the same retained batch still persist",
            "stable event id and idempotency key",
            "no legacy server database compatibility or fallback path",
        ] {
            assert!(
                body.contains(required),
                "missing MT-137 manual text: {required}"
            );
        }
    }

    #[test]
    fn mt138_manual_covers_atelier_storage_authority_and_restart_gate() {
        let page = seed_corpus()
            .pages
            .into_iter()
            .find(|page| page.slug == "atelier-storage-authority")
            .expect("Atelier storage authority page");
        let body = page
            .sections
            .iter()
            .map(|section| section.body_md.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for required in [
            "global Surreal bootstrap",
            "AtelierStore::bootstrap_schema",
            "AtelierStore::ensure_schema",
            "every canonical `atelier_*` table",
            "fields, indexes, and events, not only table names",
            "one shared bootstrap mutex",
            "DDL applies in one transaction",
            "fails startup closed",
            "bootstrap_builtin_command_corpus",
            "removes obsolete builtin descriptors and blocked records",
            "no corpus mutation, timestamp refresh, or event append",
            "Restarting reopens the same data directory",
            "There is no legacy server database, alternate local database, in-memory, mock, compatibility, or fallback database path",
            "ordinary `400`, `404`, and `409` domain outcomes",
            "Divergent lineage is refused, not overwritten",
            "GET /atelier/overview",
            "GET /atelier/command-corpus",
        ] {
            assert!(
                body.contains(required),
                "missing MT-138 manual text: {required}"
            );
        }
    }

    #[test]
    fn mt139_manual_covers_declarative_schema_authority() {
        let page = seed_corpus()
            .pages
            .into_iter()
            .find(|page| page.slug == "atelier-storage-authority")
            .expect("Atelier storage authority page");
        let body = page
            .sections
            .iter()
            .map(|section| section.body_md.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let schema_authority = page
            .sections
            .iter()
            .find(|section| section.section_kind == "schema")
            .and_then(|section| section.body_json.as_ref())
            .expect("schema authority machine-readable contract");
        let predecessor_transition = &schema_authority["supported_predecessor_transition"];
        assert_eq!(
            predecessor_transition["allowlist"].as_str(),
            Some("exact_state_schema_info_and_61_row_predecessor_registry_sha256")
        );
        assert_eq!(
            predecessor_transition["registry_rewrite"].as_str(),
            // MT-142 re-pin: the title-anchor registry row makes the current registry 63 rows.
            Some("61_historical_migration_file_rows_to_63_current_schema_source_rows")
        );
        for required in [
            "schema.surql",
            "exact_source_bytes_sha256",
            "parsed_declarative_catalog_sha256",
            "live_engine_catalog_sha256",
            // MT-142 re-pin: knowledge_rich_document_title_anchors (+1 table, +7 fields, +2 indexes).
            "282 tables",
            "3,320 authored fields",
            "795 indexes",
            "19 events",
            "surrealdb_3_2_0",
            "close/reopen",
            "fail_closed",
        ] {
            assert!(
                body.contains(required),
                "missing MT-139 manual text: {required}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // MT-142 (AC-142-11, PT-142-10): the swarm page is pinned to the code.
    // -----------------------------------------------------------------------

    fn mt142_page() -> NewUserManualPage {
        seed_corpus()
            .pages
            .into_iter()
            .find(|page| page.slug == SURREAL_SWARM_PAGE_SLUG)
            .expect("MT-142 swarm concurrency page")
    }

    fn mt142_body(page: &NewUserManualPage) -> String {
        page.sections
            .iter()
            .map(|section| section.body_md.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn mt142_section_json<'a>(page: &'a NewUserManualPage, title: &str) -> &'a serde_json::Value {
        page.sections
            .iter()
            .find(|section| section.title == title)
            .and_then(|section| section.body_json.as_ref())
            .unwrap_or_else(|| panic!("MT-142 section '{title}' must carry body_json"))
    }

    fn mt142_crate_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn mt142_read(path: &std::path::Path) -> String {
        std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    }

    /// `pub <name>:` fields of the struct whose declaration line is `header`.
    fn mt142_struct_fields(source: &str, header: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut inside = false;
        for line in source.lines() {
            let trimmed = line.trim();
            if !inside {
                inside = trimmed == header;
                continue;
            }
            if trimmed == "}" {
                break;
            }
            if let Some((name, _)) = trimmed
                .strip_prefix("pub ")
                .and_then(|rest| rest.split_once(':'))
            {
                fields.push(name.trim().to_string());
            }
        }
        assert!(!fields.is_empty(), "no fields parsed under `{header}`");
        fields
    }

    /// Variant names of the enum whose declaration line is `header`.
    fn mt142_enum_variants(source: &str, header: &str) -> Vec<String> {
        let mut variants = Vec::new();
        let mut inside = false;
        for line in source.lines() {
            let trimmed = line.trim();
            if !inside {
                inside = trimmed == header;
                continue;
            }
            if trimmed == "}" {
                break;
            }
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[") {
                continue;
            }
            let name: String = trimmed
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                variants.push(name);
            }
        }
        assert!(!variants.is_empty(), "no variants parsed under `{header}`");
        variants
    }

    /// serde `rename_all = "snake_case"` spelling of a CamelCase variant name.
    fn mt142_snake(name: &str) -> String {
        let mut out = String::new();
        for (index, ch) in name.chars().enumerate() {
            if ch.is_ascii_uppercase() {
                if index > 0 {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push(ch);
            }
        }
        out
    }

    /// Lane B swarm test sources: `tests/surreal_swarm_*.rs`, everything under
    /// `tests/swarm_support/`, the modules those files include through
    /// `mod x;` / `#[path = ".."] mod x;` (transitively), and the canonical
    /// embedded fixture `src/storage/tests.rs` when the corpus opens it through
    /// `embedded_test_backend`.
    fn mt142_swarm_test_corpus() -> (Vec<std::path::PathBuf>, String) {
        fn canonical(path: &std::path::Path) -> std::path::PathBuf {
            dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        }
        fn push_rs_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            let mut paths: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    push_rs_files(&path, files);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let path = canonical(&path);
                    if !files.contains(&path) {
                        files.push(path);
                    }
                }
            }
        }

        let crate_root = mt142_crate_root();
        let tests_dir = crate_root.join("tests");
        let mut files: Vec<std::path::PathBuf> = Vec::new();
        let mut roots: Vec<_> = std::fs::read_dir(&tests_dir)
            .expect("read tests/")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| {
                            name.starts_with("surreal_swarm_") && name.ends_with(".rs")
                        })
            })
            .collect();
        roots.sort();
        for root in roots {
            files.push(canonical(&root));
        }
        push_rs_files(&tests_dir.join("swarm_support"), &mut files);

        let mut index = 0;
        while index < files.len() {
            let file = files[index].clone();
            let dir = file.parent().expect("test file parent").to_path_buf();
            let source = mt142_read(&file);
            let mut pending_path: Option<String> = None;
            for line in source.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("#[path = \"") {
                    pending_path = rest.split('"').next().map(str::to_string);
                    continue;
                }
                if trimmed.starts_with("#[") || trimmed.starts_with("#![") {
                    continue;
                }
                let declaration = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
                let Some(rest) = declaration.strip_prefix("mod ") else {
                    pending_path = None;
                    continue;
                };
                let name = rest.trim_end_matches(';').trim();
                if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    pending_path = None;
                    continue;
                }
                let candidates = match pending_path.take() {
                    Some(relative) => vec![dir.join(relative)],
                    None => vec![dir.join(format!("{name}.rs")), dir.join(name).join("mod.rs")],
                };
                for candidate in candidates {
                    if candidate.is_file() {
                        let candidate = canonical(&candidate);
                        if !files.contains(&candidate) {
                            files.push(candidate);
                        }
                    }
                }
            }
            index += 1;
        }

        let mut text = files
            .iter()
            .map(|file| mt142_read(file))
            .collect::<Vec<_>>()
            .join("\n");
        if text.contains("embedded_test_backend") {
            let fixture = canonical(&crate_root.join("src/storage/tests.rs"));
            text.push('\n');
            text.push_str(&mt142_read(&fixture));
            files.push(fixture);
        }
        (files, text)
    }

    /// MT-142 (AC-142-11, PT-142-10) self-consistency: the swarm page exists
    /// under its slug, names every `SwarmLoadReport` field and verdict
    /// vocabulary read from `storage/surreal/swarm_load_report.rs` (never a
    /// copied list), states `RetryPolicy::CONTRACT` and its schedule, both
    /// load-profile commands, the retry-exhaustion conflict code, the
    /// multi-process prohibition, and the shutdown defaults from the code.
    #[test]
    fn mt142_manual_covers_surreal_swarm_concurrency_and_load() {
        use crate::storage::surreal::keyed_lock::KeyedLockRegistry;
        use crate::storage::surreal::retry::RetryPolicy;
        use crate::storage::surreal::swarm_load_report::{
            EngineMode, RemoteProofStatus, REQUIRED_OPERATION_CLASSES, SWARM_LOAD_REPORT_SCHEMA_ID,
        };
        use crate::storage::surreal::{
            DEFAULT_DRAIN_GRACE, DEFAULT_ENGINE_QUERY_TIMEOUT, DEFAULT_ENGINE_TRANSACTION_TIMEOUT,
            DEFAULT_SHUTDOWN_WAIT, DEFAULT_STATEMENT_TIMEOUT,
        };

        let corpus = seed_corpus();
        let toc = corpus
            .pages
            .iter()
            .find(|page| page.slug == "manual-toc")
            .expect("manual-toc");
        assert!(
            toc.anchors.iter().any(|anchor| {
                anchor.anchor_kind == "page_link" && anchor.anchor_value == SURREAL_SWARM_PAGE_SLUG
            }),
            "manual-toc must link {SURREAL_SWARM_PAGE_SLUG}"
        );
        let page = mt142_page();
        assert_eq!(page.page_kind, "surface_guide");
        let body = mt142_body(&page);

        // Report fields come from the schema source so a field added, renamed
        // or removed by the storage lane fails this test.
        let report_source = mt142_read(&mt142_crate_root().join("src/storage/surreal/swarm_load_report.rs"));
        let fields = mt142_struct_fields(&report_source, "pub struct SwarmLoadReport {");
        assert!(
            fields.len() >= 31,
            "SwarmLoadReport has {} fields; the contract lists 30 plus machine_context",
            fields.len()
        );
        let report_json = mt142_section_json(&page, "Reading hsk.surreal_swarm_load_report@1");
        let documented: Vec<&str> = report_json["fields"]
            .as_array()
            .expect("fields array")
            .iter()
            .map(|value| value.as_str().expect("field name"))
            .collect();
        for field in &fields {
            assert!(
                body.contains(&format!("`{field}`")),
                "missing MT-142 manual text for report field: {field}"
            );
            assert!(
                documented.contains(&field.as_str()),
                "body_json fields list lacks report field {field}"
            );
        }
        for field in &documented {
            assert!(
                fields.iter().any(|actual| actual == field),
                "page documents phantom report field {field}"
            );
        }
        assert!(body.contains(SWARM_LOAD_REPORT_SCHEMA_ID));
        assert_eq!(report_json["schema_id"], SWARM_LOAD_REPORT_SCHEMA_ID);

        // Verdict vocabularies come from the enum sources; the snake_case
        // converter is pinned to serde's own rendering first.
        assert_eq!(
            serde_json::to_value(RemoteProofStatus::NotRunUnconfigured).expect("serializes"),
            mt142_snake("NotRunUnconfigured")
        );
        assert_eq!(
            serde_json::to_value(EngineMode::EmbeddedRocksDb).expect("serializes"),
            mt142_snake("EmbeddedRocksDb")
        );
        for (header, json_key) in [
            ("pub enum IntegrityVerdict {", "integrity_verdict"),
            ("pub enum RemoteProofStatus {", "remote_proof_status"),
            ("pub enum FailureClass {", "failure_classes"),
            ("pub enum EngineMode {", "engine_mode"),
        ] {
            let variants = mt142_enum_variants(&report_source, header);
            let listed = report_json[json_key].as_array().expect(json_key);
            assert_eq!(
                listed.len(),
                variants.len(),
                "body_json {json_key} drifted from `{header}`"
            );
            for variant in variants {
                let snake = mt142_snake(&variant);
                assert!(
                    body.contains(&format!("`{snake}`")),
                    "missing MT-142 manual text for {json_key} value: {snake}"
                );
                assert!(
                    listed.iter().any(|value| value.as_str() == Some(snake.as_str())),
                    "body_json {json_key} lacks {snake}"
                );
            }
        }
        for class in REQUIRED_OPERATION_CLASSES {
            let name = serde_json::to_value(class).expect("serializes");
            let name = name.as_str().expect("snake_case class name");
            assert!(
                body.contains(&format!("`{name}`")),
                "missing MT-142 manual text for required operation class: {name}"
            );
        }

        // Retry policy numbers and schedule come from the code constant.
        let policy = RetryPolicy::CONTRACT;
        let retry_json = &mt142_section_json(&page, "Contention, retry, and retry exhaustion")["retry_policy"];
        let millis = |duration: std::time::Duration| u64::try_from(duration.as_millis()).expect("fits u64");
        assert_eq!(retry_json["base_delay_ms"].as_u64(), Some(millis(policy.base_delay)));
        assert_eq!(retry_json["maximum_delay_ms"].as_u64(), Some(millis(policy.maximum_delay)));
        assert_eq!(retry_json["maximum_attempts"].as_u64(), Some(u64::from(policy.maximum_attempts)));
        assert_eq!(retry_json["maximum_elapsed_ms"].as_u64(), Some(millis(policy.maximum_elapsed)));
        let schedule: Vec<u64> = (0..policy.effective_maximum_attempts().saturating_sub(1))
            .map(|retry_index| millis(policy.backoff_upper_bound(retry_index)))
            .collect();
        let documented_schedule: Vec<u64> = retry_json["sleep_upper_bounds_ms"]
            .as_array()
            .expect("sleep_upper_bounds_ms")
            .iter()
            .map(|value| value.as_u64().expect("ms"))
            .collect();
        assert_eq!(documented_schedule, schedule, "documented sleep schedule drifted");
        assert_eq!(
            retry_json["worst_case_sleep_sum_ms"].as_u64(),
            Some(schedule.iter().sum::<u64>())
        );
        let schedule_text = schedule
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        for required in [
            format!("{} ms base delay", millis(policy.base_delay)),
            format!("{} ms cap", millis(policy.maximum_delay)),
            format!("{} attempts", policy.maximum_attempts),
            format!("{} ms maximum elapsed", millis(policy.maximum_elapsed)),
            format!("{schedule_text} ms"),
            format!("worst case {} ms of sleep", schedule.iter().sum::<u64>()),
            "full jitter".to_string(),
        ] {
            assert!(body.contains(&required), "missing MT-142 manual text: {required}");
        }

        // Codes, commands, prohibitions, report rules, shutdown symbols.
        for required in [
            "HSK-STORAGE-RETRY-EXHAUSTED",
            "HSK-KRD-SAVE-STALE",
            "HSK-KIDEM-RACE",
            "HSK-STORAGE-LOCK-WAIT-TIMEOUT",
            "StatementTimeout",
            "knowledge_rich_document_title_anchors",
            "surrealdb 3.2.0",
            "A second process on the same store path is NOT supported",
            "a second embedded engine on the same path inside this process is NOT supported",
            "never describe them, or any same-path second engine, as distributed concurrency or as a distributed proof",
            "do not simulate a remote topology by opening the same store path twice",
            "`not_run` means no samples; it is NOT a zero-latency pass",
            "`{ numerator, denominator, rate }`",
            "never read `rate` without its numerator and denominator",
            "`not_run_unconfigured` is never a PASS",
            "`measured`",
            "ShutdownReport { drained, cancelled, elapsed }",
            "RICH_DOCUMENT_MUTATION_LOCK",
            "KNOWLEDGE_UPSERT_LOCK",
            "HANDSHAKE_SWARM_EXTENDED=1",
            "HANDSHAKE_SWARM_SEED",
            "HANDSHAKE_SWARM_LOAD_REPORT_DIR",
            "HANDSHAKE_ARTIFACTS_ROOT",
            "HANDSHAKE_WORKSPACE_ROOT",
        ] {
            assert!(body.contains(required), "missing MT-142 manual text: {required}");
        }
        let runbook = mt142_section_json(
            &page,
            "Running the deterministic CI profile and the extended local profile",
        );
        let commands = runbook["commands"].as_array().expect("commands");
        for profile in ["ci_deterministic", "extended_local"] {
            assert!(
                commands.iter().any(|command| command["profile"] == profile),
                "missing MT-142 load-profile command: {profile}"
            );
        }
        for command in commands {
            let text = command["command"].as_str().expect("command string");
            assert!(text.starts_with("cargo test "), "{text}");
            assert!(text.contains("--features surreal-test-support,test-utils"), "{text}");
            assert!(body.contains(text), "command not in the page text verbatim: {text}");
        }
        // Shutdown defaults, the ShutdownReport shape and every runtime symbol the
        // page names come from the storage sources.
        let shutdown_json = mt142_section_json(&page, "Shutdown under load and the ShutdownReport");
        for (key, default, symbol) in [
            ("shutdown_wait_default_ms", DEFAULT_SHUTDOWN_WAIT, "DEFAULT_SHUTDOWN_WAIT"),
            ("drain_grace_default_ms", DEFAULT_DRAIN_GRACE, "DEFAULT_DRAIN_GRACE"),
            ("statement_timeout_default_ms", DEFAULT_STATEMENT_TIMEOUT, "DEFAULT_STATEMENT_TIMEOUT"),
            ("engine_query_timeout_default_ms", DEFAULT_ENGINE_QUERY_TIMEOUT, "DEFAULT_ENGINE_QUERY_TIMEOUT"),
            (
                "engine_transaction_timeout_default_ms",
                DEFAULT_ENGINE_TRANSACTION_TIMEOUT,
                "DEFAULT_ENGINE_TRANSACTION_TIMEOUT",
            ),
        ] {
            assert_eq!(shutdown_json[key].as_u64(), Some(millis(default)), "body_json {key} drifted");
            let required = format!("`{symbol}` {} s", default.as_secs());
            assert!(body.contains(&required), "missing MT-142 manual text: {required}");
        }
        let crate_root = mt142_crate_root();
        let surreal_source = mt142_read(&crate_root.join("src/storage/surreal.rs"));
        let report_fields = mt142_struct_fields(&surreal_source, "pub struct ShutdownReport {");
        let documented_report_fields: Vec<&str> = shutdown_json["report"]["fields"]
            .as_array()
            .expect("report fields")
            .iter()
            .map(|value| value.as_str().expect("field name"))
            .collect();
        assert_eq!(documented_report_fields, report_fields, "ShutdownReport fields drifted");
        for field in &report_fields {
            assert!(
                body.contains(&format!("`{field}")),
                "missing MT-142 manual text for ShutdownReport field: {field}"
            );
        }
        let storage_sources = [
            surreal_source,
            mt142_read(&crate_root.join("src/storage/surreal/database.rs")),
            mt142_read(&crate_root.join("src/storage/surreal/knowledge.rs")),
        ]
        .join("\n");
        for symbol in shutdown_json["runtime_symbols"].as_array().expect("runtime_symbols") {
            let symbol = symbol.as_str().expect("symbol");
            let leaf = symbol.rsplit("::").next().expect("symbol leaf");
            assert!(
                storage_sources.contains(leaf),
                "storage sources no longer define {symbol}; update the MT-142 manual"
            );
            assert!(body.contains(leaf), "manual text does not name runtime symbol {symbol}");
        }
        for code in ["HSK-STORAGE-RETRY-EXHAUSTED", "HSK-STORAGE-LOCK-WAIT-TIMEOUT"] {
            assert!(
                storage_sources.contains(&format!("\"{code}\"")),
                "storage sources no longer define conflict code {code}"
            );
        }
        let swarm_json = mt142_section_json(&page, "Safe parallel swarm use");
        assert_eq!(
            swarm_json["keyed_lock"]["idle_entry_bound"].as_u64(),
            Some(u64::try_from(KeyedLockRegistry::keyed().entry_count()).expect("fits u64"))
        );
    }

    /// MT-142 no-context runbook: every command the page documents names a
    /// `tests/<target>.rs` that exists (and, when it names a test, that test
    /// fn is defined there); every env var the page names is read by the swarm
    /// test sources (`read_by_swarm_tests`) or by its declared consumer file;
    /// every printed marker the page teaches a model to look for is emitted by
    /// those sources.
    #[test]
    fn mt142_manual_runbook_targets_and_env_vars_exist() {
        let page = mt142_page();
        let body = mt142_body(&page);
        let runbook = mt142_section_json(
            &page,
            "Running the deterministic CI profile and the extended local profile",
        );
        let crate_root = mt142_crate_root();
        let tests_dir = crate_root.join("tests");
        let (files, corpus) = mt142_swarm_test_corpus();
        assert!(
            !files.is_empty(),
            "no tests/surreal_swarm_*.rs target exists; the runbook documents nothing real"
        );

        for command in runbook["commands"].as_array().expect("commands") {
            let target = command["target"].as_str().expect("target");
            let text = command["command"].as_str().expect("command string");
            let target_file = tests_dir.join(format!("{target}.rs"));
            assert!(
                target_file.is_file(),
                "documented target {target} has no {}",
                target_file.display()
            );
            assert!(
                text.contains(&format!("--test {target} ")),
                "command does not run --test {target}: {text}"
            );
            assert!(body.contains(text), "command not in the page text verbatim: {text}");
            if let Some(filter) = command["test_filter"].as_str() {
                let source = mt142_read(&target_file);
                assert!(
                    source.contains(&format!("fn {filter}(")),
                    "{target}.rs defines no test fn {filter}"
                );
                assert!(
                    text.contains(&format!(" {filter} ")),
                    "command does not select {filter}: {text}"
                );
            }
        }

        for env in runbook["env_vars"].as_array().expect("env_vars") {
            let name = env["name"].as_str().expect("env var name");
            assert!(body.contains(name), "env var {name} not named in the page text");
            if env["read_by_swarm_tests"].as_bool() == Some(true) {
                assert!(
                    corpus.contains(name),
                    "{name} is documented as swarm-test input but no swarm test source reads it; sources: {files:?}"
                );
            }
            if let Some(consumer) = env["consumer"].as_str() {
                let consumer_source = mt142_read(&crate_root.join(consumer));
                assert!(
                    consumer_source.contains(name),
                    "documented consumer {consumer} does not read {name}"
                );
            }
        }

        for marker in runbook["printed_markers"].as_array().expect("printed_markers") {
            let marker = marker.as_str().expect("marker");
            assert!(body.contains(marker), "marker {marker} not in the page text");
            assert!(
                corpus.contains(marker),
                "swarm test sources neither print nor write documented marker {marker}; sources: {files:?}"
            );
        }
    }
}
