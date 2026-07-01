//! The compiled-in UserManual seed corpus and the idempotent seeder.
//!
//! * MT-196 UserManualPurposeAndWorkflowPages — purpose, core workflows,
//!   startup/run commands, expected inputs/outputs, navigation paths.
//! * MT-197 UserManualToolPrimitiveCatalog — tools, primitives, APIs, backend
//!   navigation commands, visual-debug surfaces, safe usage (tool entries are
//!   generated from [`registry::wp009_surface_registry`] + the legacy static
//!   manifest so the catalog can never drift from the declared inventory).
//! * MT-198 UserManualFailureRecoveryPages — common failures, diagnostics,
//!   recovery steps, repair queues, stale state, missing-Postgres behavior.
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
use crate::storage::postgres::PostgresDatabase;
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
        page_argus_visual_inspection(),
        page_knowledge_index_surface(),
        page_notes_loom_surface(),
        page_rich_documents_surface(),
        page_retrieval_surface(),
        page_memory_surface(),
        page_crdt_surface(),
        page_usermanual_surface(),
        page_failure_modes_and_recovery(),
        page_repair_queues_and_staleness(),
        page_missing_postgres_behavior(),
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
        "argus-visual-inspection",
        "knowledge-index-surface",
        "notes-loom-surface",
        "rich-documents-surface",
        "retrieval-and-context-bundles-surface",
        "memory-and-claims-surface",
        "crdt-collaboration-surface",
        "usermanual-surface",
        "failure-modes-and-recovery",
        "repair-queues-and-staleness",
        "missing-postgres-behavior",
        "state-recovery-guide",
        "kernel-write-governance",
        "legacy-model-manual-bridge",
        "quickstart-index",
        "quickstart-editor",
        "quickstart-loom",
        "quickstart-atelier-ckc-posekit",
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
                 models and operators. Every page is a PostgreSQL authority row served over \
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
                json!(all_slugs),
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
                 models co-author work over ONE authority substrate: PostgreSQL plus the \
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
                "PostgreSQL + EventLedger is canonical for durable state, receipts, indexing \
                 evidence, and validation. Generated markdown, wiki pages, HTML exports, context \
                 bundles, debug reports, and UI projections are PROJECTIONS — useful, never \
                 authority. There is no SQLite, no Docker dependency, no external daemon: \
                 Handshake manages its own PostgreSQL cluster (see \
                 [[missing-postgres-behavior]]).",
            ),
            section(
                "navigation",
                "Where to go next",
                "- Operate the product: [[startup-and-run-commands]]\n\
                 - Call backend APIs: [[backend-navigation-and-identity]]\n\
                 - Task-sized intros: [[quickstart-index]], [[quickstart-editor]], \
                 [[quickstart-loom]], [[quickstart-atelier-ckc-posekit]], \
                 [[quickstart-retrieval]]\n\
                 - When something breaks: [[failure-modes-and-recovery]]",
            ),
        ],
        anchors: vec![
            page_link("startup-and-run-commands"),
            page_link("backend-navigation-and-identity"),
            page_link("missing-postgres-behavior"),
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
                 surface). On startup Handshake ensures its own managed PostgreSQL cluster is \
                 running (default port 5544, data dir `Handshake_Artifacts/managed_pgdata` in the \
                 shared `Handshake_Artifacts` root beside the repo — the worktrees' sibling, not \
                 inside the worktree) — no Docker, no external daemon. Quiet by design: no foreground \
                 window is popped.",
                json!({
                    "run_command": "cargo run -p handshake_core --bin handshake_core --features app-runtime",
                    "listen_addr": "127.0.0.1:37501",
                    "api_mounts": ["/", "/api"],
                    "managed_postgres_port": 5544,
                    "managed_postgres_data_dir": "Handshake_Artifacts/managed_pgdata"
                }),
            ),
            section(
                "run_commands",
                "Probe health",
                "```\ncurl http://127.0.0.1:37501/health\n```\n\n\
                 `GET /health` answers when the server is up. If it does not answer, see \
                 [[missing-postgres-behavior]] and [[state-recovery-guide]].",
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
                 Integration tests provision an isolated schema per test on the real cluster \
                 (`POSTGRES_TEST_URL` > `DATABASE_URL` > managed cluster) and fail hard when \
                 PostgreSQL is unavailable. There is no SQLite or mock fallback.",
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
            page_link("missing-postgres-behavior"),
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
                 rendered HTML) as authority; authority is the PostgreSQL row + EventLedger \
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

fn page_argus_visual_inspection() -> NewUserManualPage {
    NewUserManualPage {
        slug: "argus-visual-inspection".into(),
        title: "Argus Visual Inspection".into(),
        page_kind: "diagnostics",
        audience: "model_and_operator",
        spec_anchors: vec!["10.15.8".into()],
        sections: vec![
            section(
                "purpose",
                "What Argus is",
                "Argus is the named Handshake visual inspection capability for operators and \
                 models. It is the eyes of future model work across Handshake products: use Argus \
                 to inspect panels, tabs, buttons, labels, bounds, enabled/disabled state, \
                 screenshots, and structured GUI state before claiming visual or behavioral work \
                 is done.\n\n\
                 Argus must be Rust-native and deeply integrated with Handshake. Its first \
                 implementation layer is the native shell's AccessKit UI-tree snapshot, the MCP \
                 swarm tool surface (`list_widgets`, `click_widget`, `set_value`, `screenshot`), \
                 stable author_id addressing, and the existing inspector/visual-debug posture. Do \
                 not replace Argus with a loose external screenshot script as the primary path.",
            ),
            section(
                "workflow",
                "How models use Argus",
                "1. Use `list_widgets` first to read the current AccessKit tree: author_id, role, \
                 label, value, actions, disabled state, bounds, and children.\n\
                 2. Use stable author_id selectors for assertions and steering. If a control has \
                 no stable author_id, treat that as a product defect for model operation.\n\
                 3. Use `screenshot` only as the visual companion to the structured tree, so a \
                 model can compare layout, overlap, readability, and actual rendered pixels.\n\
                 4. For parallel agents, prefer shared reads of the Argus snapshot and explicit \
                 MCP leases/receipts for mutations. Parallel agents must not coordinate through \
                 fragile screen position guesses.\n\
                 5. Record the Argus path used in validation evidence: snapshot route/tool, \
                 target author_id values, screenshot artifact when available, and the observed \
                 pass/fail result.",
            ),
            section(
                "safety",
                "Non-intrusive operation",
                "Argus must be quiet. It must not bring Handshake to the foreground, must not \
                 steal keyboard focus, must not steal mouse input, must not move the cursor, and \
                 must not require the operator to make the Handshake window active. Foreground \
                 desktop automation is not an acceptable substitute for Argus.\n\n\
                 If Argus cannot inspect a surface, state `not inspected`, keep any verdict \
                 unclaimed, and fall back to the closest non-intrusive proof path: headless \
                 egui/AccessKit tests, backend diagnostic snapshots, or a bounded operator-visible \
                 artifact that does not steal attention.",
            ),
            section(
                "recovery",
                "Failure and recovery",
                "Common failure modes: stale snapshots after a frame did not publish, missing \
                 author_id values, GPU-gated screenshots on a headless host, an MCP binding file \
                 that is not present yet, or a surface rendered outside the native AccessKit tree. \
                 Recover by rerunning a frame, querying `list_widgets` again, using the structured \
                 snapshot before the screenshot, checking MCP binding state, and filing the \
                 missing author_id or missing Argus coverage as product work instead of forcing \
                 foreground automation.",
            ),
        ],
        anchors: vec![
            page_link("quickstart-validation"),
            page_link("startup-and-run-commands"),
            page_link("permissions-and-safety"),
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
            json!(wp009_surface_registry()
                .iter()
                .filter(|s| s.group == group)
                .map(|s| json!({
                    "surface_id": s.surface_id,
                    "method": s.method,
                    "route": s.route,
                    "summary": s.summary,
                }))
                .collect::<Vec<_>>()),
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
        "The Project Knowledge Index turns configured project roots into typed PostgreSQL \
         knowledge: sources with content hashes, extraction receipts, entities, edges, evidence \
         spans, and code symbols. Ingestion routes manage roots/runs/repairs; the code-navigation \
         routes (listed below with the ingestion routes) answer symbol questions WITHOUT an \
         external LSP server.",
        vec![
            section_with_json(
                "navigation",
                "Code navigation routes",
                &group_routes_md(SurfaceGroup::CodeNavigation),
                json!(wp009_surface_registry()
                    .iter()
                    .filter(|s| s.group == SurfaceGroup::CodeNavigation)
                    .map(|s| json!({
                        "surface_id": s.surface_id,
                        "method": s.method,
                        "route": s.route,
                        "summary": s.summary,
                    }))
                    .collect::<Vec<_>>()),
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
                 - 500 `internal_error` / `storage_error` — PostgreSQL unavailable: fail-closed, \
                 no data is served (see [[missing-postgres-behavior]]).",
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
            extra.push(page_link("missing-postgres-behavior"));
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
        ],
        vec![
            route_anchor("POST", "/workspaces"),
            page_link("quickstart-loom"),
        ],
        vec!["2.2.1.14".into(), "7.1.1.9".into(), "10.12".into()],
    )
}

fn page_rich_documents_surface() -> NewUserManualPage {
    surface_page(
        "rich-documents-surface",
        "Rich Documents — Authority, History, Projections, Embeds",
        SurfaceGroup::RichDocuments,
        "RichDocuments are versioned Tiptap/ProseMirror JSON authority rows in PostgreSQL with \
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
                 `GET .../history/:doc_version`.",
            ),
        ],
        vec![
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
         PostgreSQL rows (migration 0310), seeded from a compiled-in corpus, receipted through \
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
                 - DB down -> [[missing-postgres-behavior]]",
            ),
        ],
        anchors: vec![
            page_link("repair-queues-and-staleness"),
            page_link("state-recovery-guide"),
            page_link("missing-postgres-behavior"),
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

fn page_missing_postgres_behavior() -> NewUserManualPage {
    NewUserManualPage {
        slug: "missing-postgres-behavior".into(),
        title: "Missing PostgreSQL Behavior".into(),
        page_kind: "failure_recovery",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.11".into()],
        sections: vec![
            section(
                "failure_modes",
                "What happens without the database",
                "PostgreSQL is the only authority store — there is NO SQLite, in-memory, or mock \
                 fallback anywhere in the product. Behavior when it is unavailable:\n\n\
                 - **Product runtime**: knowledge routes FAIL CLOSED with 500 \
                 `internal_error`/`storage_error` envelopes; no fail-open path serves data when \
                 the store errors.\n\
                 - **Startup**: the server ensures the Handshake-managed cluster \
                 (default `127.0.0.1:5544`, data dir `Handshake_Artifacts/managed_pgdata`) is \
                 running before serving; an adopted external cluster is left untouched at \
                 shutdown.\n\
                 - **Tests**: integration tests resolve `POSTGRES_TEST_URL` > `DATABASE_URL` > \
                 managed cluster; when PostgreSQL is unavailable they fail hard. A green run \
                 therefore requires real PostgreSQL, not SQLite, mocks, or skipped proof.",
            ),
            section(
                "recovery",
                "Recovery",
                "1. Probe: `curl http://127.0.0.1:37501/health` and check the cluster port 5544.\n\
                 2. Restart the backend — startup re-ensures the managed cluster.\n\
                 3. If the data dir is corrupt, the managed cluster logs name the failure; the \
                 EventLedger and all manual/knowledge rows live IN PostgreSQL, so never delete \
                 `Handshake_Artifacts/managed_pgdata` to 'fix' a startup error without a backup.\n\
                 4. Re-run the smallest scoped test that exercises your surface to confirm \
                 recovery.",
            ),
        ],
        anchors: vec![
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
                "Parallel local/cloud agents recover from the PostgreSQL/EventLedger swarm \
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
                 `mt223_restart_after_crash_reconstructs_swarm_state_from_postgres`. These \
                 prove false receipts are not emitted, queue order survives stale reclaim, and \
                 a fresh store can reconstruct state from Postgres alone.",
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
                        "mt223_restart_after_crash_reconstructs_swarm_state_from_postgres"
                    ],
                    "authority": [
                        "PostgreSQL",
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
                 3. If the failure names a missing table, the migration chain is behind: \
                 migrations run automatically per isolated test schema; check the migration file \
                 numbering for collisions.\n\
                 4. A PostgreSQL availability failure is not a pass — provision the cluster.",
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
            json!(kernel_section.instructions),
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
                 - **P1 (this WP)**: canonical `user_manual` module + PostgreSQL authority + \
                 aliases + receipts (DONE by MT-193..MT-208).\n\
                 - **P2 (frontend lane)**: rename Tauri commands \
                 (`model_manual_get` -> canonical `/usermanual` routes), app help surface.\n\
                 - **P3 (later WP)**: retire the static legacy module files.",
                json!(plan
                    .rows
                    .iter()
                    .map(|r| json!({
                        "row_id": r.row_id,
                        "legacy_id": r.legacy_id,
                        "canonical_ref": r.canonical_ref,
                        "phase": r.phase.as_str(),
                        "shim_state": r.shim_state.as_str(),
                    }))
                    .collect::<Vec<_>>()),
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
    "atelier-ckc-posekit",
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
            "atelier-ckc-posekit",
            "Quickstart — Atelier CKC/PoseKit",
            "1. Start from native Handshake Atelier: CKC/PoseKit are built-in Handshake-native \
             tools. The old CastKit app is behavior reference, not the runtime to port.\n\
             2. Bring images into the governed Atelier graph through existing media intake or \
             `POST /atelier/image-import/url`; include `x-hsk-actor-id`, an idempotency key, \
             and capability profile/grant refs.\n\
             3. Use PoseKit as Atelier pose state: set pose context, ingest validated OpenPose \
             rigs, list rigs, inspect one rig, and keep calibration explicit. If calibration is \
             unresolved, record BLOCKED with a block reason instead of inventing values.\n\
             4. Attach production artifacts as typed sidecars: OpenPose JSON, OpenPose PNG, and \
             conditioning PNG. These are exported/inspectable pose assets and stay hidden from \
             normal galleries unless a projection says otherwise.\n\
             5. Maintain character identity through append-only identity profiles and 512x512 \
             identity crop artifacts, with provenance by reference and no raw secrets.\n\
             6. For ComfyUI, record workflow receipts, replay history, SaveImage fallback \
             reasons, and retryable output-registration failures so saved outputs are not lost.\n\
             7. For Facial-backed LoRA mining ingest, run \
             `POST /atelier/intake/batches/:batch_id/facial/analyze` after the intake batch has \
             source items. Outputs are read-only analysis artifacts: rows include \
             `quality_source_family`, `quality_feature_id`, `quality_metrics`, `ofiq_quality`, \
             `dedupe_record`, `identity_record`, optional `identity_model_sha256`, detector and \
             landmark SHA fields, and exact content-hash duplicate groups; summary \
             `identity_provenance` and `native_feature_outputs` record `facet`, `python-ofiq`, \
             `imagededup`, identity-gate config/provenance, and unavailable `ediffiqa` features. \
             Missing content hashes stay singletons. A configured model hash is provenance, not \
             proof: `identity_source` becomes real only after a build with the \
             `facial-onnx-runtime` feature loads the native ArcFace tract/ONNX runtime and \
             scores a local image. Real rows use \
             `arcface_onnx_resize_112_v1`, emit an embedding digest/dimensions, and keep the \
             verdict `unsure` until a reference identity exists; no-model rows stay \
             `handshake_proxy_no_model` / `proxy_unverified`, and configured-but-unloaded or \
             dataset-only rows stay `handshake_identity_model_unavailable` / \
             `model_unavailable`; the summary uses `runtime_feature_disabled` when an ArcFace \
             model is configured in a build that does not include `facial-onnx-runtime`, and \
             `runtime_error_counts` records redacted failure buckets for load, image, and \
             inference failures. Do not treat metadata-only quality as OFIQ/eDifFIQA model parity or as \
             `handshake_native_proxy_v1`.\n\
             8. For Facial review queues, build a `hsk.atelier.facial_review.session@1` \
             artifact from the Facial analysis rows. Stable candidate IDs use \
             `content_hash_plus_source_ref` when a content hash exists and `source_ref` when it \
             does not, so duplicate files do not collapse into one review item. Parallel agents \
             claim disjoint shards with `hsk.atelier.facial_review.claim@1`; expired claims must \
             be explicitly recovered. Review decisions are append-only \
             `hsk.atelier.facial_review.decision@1` receipts with actor, timestamp, reason, \
             tags, and notes. Operator wording maps `pass` to canonical `accept`, `reject` to \
             `reject`, and `unsure` to canonical `hold`; keep both entered and canonical values. \
             `hsk.atelier.facial_review.montage@1` is a contact-sheet tile-map artifact with \
             `tile_id`, row/column coordinates, `stable_image_id`, source refs, current decision, \
             and `argus://facial-review/` selectors for headless visual inspection. \
             `hsk.atelier.facial_review.export@1` is a non-destructive Kohya/LoRA export \
             manifest: it records accepted candidates, decision receipts, analysis/receipt \
             SHA-256 lineage, dedupe/identity/quality fields, output refs, skipped rejects/holds, \
             and `source_mutation=false`; undecided items block export unless partial export is \
             explicit.\n\
             9. Configure Facial identity only through environment/config keys such as \
             `HANDSHAKE_FACIAL_ARCFACE_ONNX`, `HANDSHAKE_FACIAL_YUNET_ONNX`, \
             `HANDSHAKE_FACIAL_LANDMARK_MODEL`, and `HANDSHAKE_FACIAL_IDENTITY_THRESHOLD`; never \
             hardcode machine-local model paths in product code, tests, notes, or agent \
             instructions.\n\
             10. For visual/behavioral polish, use this native state map: source image strip, \
             open rig tabs, OpenPose sidecar strip, identity crop review, Comfy history/replay, \
             and deferred-feature list. Compare screens to the original CKC app for behavior, \
             but keep Handshake as the implementation authority.\n\
             11. Before changing UI behavior, read the relevant MT contract and the \
             Pose/Comfy deferred features. Do not fake detector execution, calibration, \
             route wiring, or external bridge authority just to make a screen look complete.",
            vec![
                page_link("legacy-model-manual-bridge"),
                page_link("quickstart-validation"),
                page_link("state-recovery-guide"),
                route_anchor("POST", "/atelier/image-import/url"),
                route_anchor("POST", "/atelier/intake/batches/:batch_id/facial/analyze"),
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
            "1. Run the surface's SCOPED test target on real PostgreSQL \
             ([[startup-and-run-commands]]): `cargo test -p handshake_core --features \
             test-utils --test <surface>_tests`.\n\
             2. A PostgreSQL availability failure is NOT a pass — provision PostgreSQL.\n\
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
            "500 internal_error (PostgreSQL unavailable; fail-closed)".into(),
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

/// Seed (or re-sync) the UserManual corpus into PostgreSQL. Idempotent: rows
/// short-circuit on content hash; receipts are appended only for changed
/// pages plus one summary receipt when anything changed. Always records the
/// `user_manual_versions` row.
pub async fn ensure_seeded(db: &PostgresDatabase) -> StorageResult<SeedReport> {
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
}
