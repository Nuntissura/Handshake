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
use super::registry::{wp009_surface_registry, SurfaceGroup};
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
        page_knowledge_index_surface(),
        page_notes_loom_surface(),
        page_rich_documents_surface(),
        page_retrieval_surface(),
        page_memory_surface(),
        page_crdt_surface(),
        page_model_lane_schema(),
        page_model_runtime_registry_and_loom_degrade(),
        page_model_lane_launch_adapters(),
        page_model_lane_promotion(),
        page_model_lane_context_bundle_handoff(),
        page_model_lane_cloud_projection_consent(),
        page_cloud_model_access(),
        page_model_lane_recovery(),
        page_model_lane_diagnostics(),
        page_model_lane_navigation(),
        page_model_lane_validation_harness(),
        page_embedded_model_lifecycle_ledger(),
        page_dedicated_embedding_model_routing(),
        page_operator_chat_launch(),
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
        "knowledge-index-surface",
        "notes-loom-surface",
        "rich-documents-surface",
        "retrieval-and-context-bundles-surface",
        "memory-and-claims-surface",
        "crdt-collaboration-surface",
        "model-lane-schema",
        "model-runtime-registry-and-loom-degrade",
        "model-lane-launch-adapters",
        "model-lane-promotion",
        "model-lane-context-bundle-handoff",
        "model-lane-cloud-projection-consent",
        "cloud-model-access",
        "model-lane-recovery",
        "model-lane-diagnostics",
        "model-lane-navigation",
        "model-lane-validation-harness",
        "embedded-model-lifecycle-ledger",
        "dedicated-embedding-model-routing",
        "operator-chat-launch",
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
                json!(all_slugs.to_vec()),
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
                 [[quickstart-loom]], [[quickstart-retrieval]]\n\
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

fn page_cloud_model_access() -> NewUserManualPage {
    NewUserManualPage {
        slug: "cloud-model-access".into(),
        title: "Cloud Model Access — Subscription Plans And BYOK Keys".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["2.3.13.11".into(), "10.15.8".into()],
        sections: vec![
            section(
                "purpose",
                "What this surface is",
                "The operator configures cloud model access from Settings > Cloud Models (MT-015). Two \
                 paths exist:\n\n\
                 - SUBSCRIPTION PLAN (primary): log in with a provider's OWN official CLI (Claude Code, \
                 GPT/Codex) via the official CLI bridge. Handshake stores NO credential for this path — \
                 the session lives in the provider's CLI. On Windows, status runs only against the same \
                 pinned, launchable executable graph through the attached Job-contained sandbox. Only \
                 recognized provider grammar from a successful status command can produce `logged_in` \
                 or `logged_out`; unsupported expiry detection, \
                 API-key/agent auth modes, a missing/non-launchable CLI, timeout, or unrecognized output \
                 reports `unavailable` rather than guessing. Non-Windows probing is fail-closed \
                 `unavailable` until an equivalent process-tree-contained adapter exists. The row provides \
                 an operator-initiated 'Log in…' button. The first click opens an in-app \
                 confirmation that discloses the new foreground terminal; only `Start login` calls the \
                 backend-owned launch endpoint. The backend uses the same pinned executable graph and \
                 fixed `claude auth login` or `codex login` argv, returning only an opaque pid handle. \
                 The GUI performs no shell or PATH resolution, and provider response data is never \
                 interpolated into a command.\n\
                 - BYOK (available, not required): paste an Anthropic or OpenAI API key. The key is \
                 stored ONLY in the OS keychain (Windows Credential Manager / macOS Keychain / Linux \
                 Secret Service). It is NEVER written to logs, the Flight Recorder, the EventLedger, the \
                 workspace-settings blob, or any plaintext store. A Remove control rotates/clears it.\n\n\
                 Gemini is NOT offered (its CLI is being discontinued).",
            ),
            section(
                "inputs_outputs",
                "Backend routes (models)",
                "A model configures the same access over HTTP:\n\n\
                 - `GET /model-access/providers` — non-secret enumeration: each provider's \
                 `configured` / `unavailable` status (a missing key is `unavailable`, never an error), \
                 each CLI bridge's typed `logged_in` / `logged_out` / `expired` / `unavailable` auth \
                 status, the CLI-bridge login commands, and the explicit `excluded: [\"gemini\"]` list. This is \
                 what the operator model-picker lists. `expired` remains a typed presentation state for a \
                 future exact provider signal; production never infers it from free text. The picker \
                 enables a CLI row only when the exact same provider has both a registered launch builder \
                 and `logged_in`; every other state remains visible but disabled.\n\
                 - `PUT /model-access/byok/{provider}/key` with body `{\"api_key\": \"…\"}` — store a \
                 BYOK key in the OS keychain. The response carries only non-secret status; the key is \
                 never echoed. `{provider}` is `anthropic` or `openai`; any other id (including \
                 `gemini`) returns 404 `provider_not_offered`.\n\
                 - `DELETE /model-access/byok/{provider}/key` — remove / rotate a key (idempotent).\n\n\
                 - `POST /model-access/cli-bridge/{provider}/login` — after operator confirmation, \
                 ask the backend to launch the same provider's already-pinned foreground login graph. \
                 The response contains only the provider id and pid launch handle.\n\n\
                 There is no route to read a stored key back out over HTTP.",
            ),
            section(
                "navigation",
                "Reaching the surface: modal or detached window (Argus targeting)",
                "Settings is opened from `HELP` then `Open Settings…` (`menu.help.settings`), from the \
                 command palette action `settings.open`, or by setting `settings_open = true`. The \
                 surface then renders in ONE of two mutually exclusive hosts — never both at once, so a \
                 driver never has to disambiguate a double UI:\n\n\
                 1. DOCKED (default): a modal in the MAIN window. Argus `window_id` is `main`; the \
                 surface root is the AccessKit node `settings.dialog` (`Role::Dialog`, modal). All \
                 controls listed above (`settings.cloud.*`, `settings.theme`, `settings.search`, \
                 `settings.list`, `settings.section.*`, `settings.close`) live in this window.\n\
                 2. DETACHED: its own OS window, entered by clicking `settings.popout` in the modal \
                 header. Argus `window_id` is `popout-settings`; the OS title is `Handshake – Settings` \
                 (en dash); the window root is the AccessKit node `popout-window-settings` \
                 (`Role::Window`). The detached window is enumerated by `argus.list_windows` from the \
                 moment it is created (before its first published snapshot, `revision` 0), so a driver \
                 polls one canonical surface instead of guessing viewport timing. `argus.list_widgets`, \
                 `argus.click`, `argus.set_value`, and `screenshot` all take that `window_id`; the \
                 shell records the detached window's OS window handle under it, so a capture grabs THAT \
                 exact window instead of matching by title.\n\n\
                 The SAME sections and the SAME control author_ids render in both hosts (one render \
                 path), so an inspect/set/click script written against the modal works unchanged \
                 against the detached window once the `window_id` is switched. The one deliberate \
                 difference is the root node, and it is the signal for WHICH host is live: while \
                 detached, `settings.dialog` is ABSENT and `popout-window-settings` is present; while \
                 docked, the reverse. The detached header adds `settings.redock` (return to the modal, \
                 settings stays open); its `settings.close` control and the window's OS close button \
                 close settings outright, after which a re-open comes back as the modal.\n\n\
                 Quiet-mode posture (HBR-QUIET-001): the detached window is created with \
                 `with_active(false)`, so popping Settings out never raises the window to the \
                 foreground or steals keyboard focus from the operator or from another agent's window. \
                 Recovery: if `popout-settings` is not listed, the surface is docked (or closed) — \
                 target `main` and, if needed, click `settings.popout` to detach it again; a stale \
                 `popout-settings` handle can never be captured, because re-dock/close unregister the \
                 window and forget its recorded OS handle in the same step.",
            ),
            section(
                "safety",
                "Consent boundary + failure modes",
                "- Saving a BYOK key creates NO ConsentReceipt and NO ConsentGate approval. Configuring \
                 access is not consenting to a cloud send: the FIRST cloud lane launch still hits the \
                 fail-closed per-session consent gate (see [[model-lane-cloud-projection-consent]]).\n\
                 - The key round-trips OUT of the keychain only for the cloud backend to use it as the \
                 provider's Authorization bearer token; it appears nowhere else.\n\
                 - 400 `empty_api_key` — a blank key is rejected and not stored.\n\
                 - 404 `provider_not_offered` — an unknown or excluded provider id (e.g. `gemini`).\n\
                 - 503 `keychain_unavailable` — the OS keychain feature is disabled; Handshake REFUSES \
                 to persist a cloud key rather than fall back to any plaintext store.",
            ),
            section(
                "run_commands",
                "Behavior matrix + proof targets",
                "MT-015 cloud model access coverage is tracked by \
                 `cloud_model_access_behavior_coverage_matrix()` and verified by \
                 `cloud_model_access_behaviors_have_manual_coverage`. The behavior matrix rows are \
                 `wp1.cloud_access.providers_enumeration`, `wp1.cloud_access.byok_store`, \
                 `wp1.cloud_access.byok_delete`, `wp1.cloud_access.secret_leak_guard`, \
                 `wp1.cloud_access.settings_argus`, and `wp1.cloud_access.cli_bridge_login`. \
                 These rows are `NOT_APPLICABLE-with-reason` for internal_diagnostics and Palmistry \
                 because Settings/keychain configuration is not a ModelLane runtime tier; the \
                 authority is the model-access HTTP route, the OS keychain leak proof, and the native \
                 Argus AccessKit tree.\n\n\
                 Exact backend route proof targets: `model_access_route_tests::put_store_returns_200_and_never_echoes_the_key`; \
                 `model_access_route_tests::delete_byok_key_is_idempotent_and_updates_status`; \
                 `model_access_route_tests::get_providers_reflects_configured_and_excludes_gemini`; \
                 `model_access_route_tests::cli_bridge_typed_status_wire_mapping_excludes_account_fields_and_gemini`; \
                 `model_access_route_tests::cli_login_route_returns_only_backend_owned_launch_handle`; \
                 `model_access_route_tests::put_empty_key_is_400`; \
                 `model_access_route_tests::put_gemini_is_404_excluded`; \
                 `model_access_route_tests::keychain_unavailable_is_503`. These cover \
                 `GET /model-access/providers`, `PUT /model-access/byok/{provider}/key`, \
                 and `DELETE /model-access/byok/{provider}/key` without touching the host keychain. \
                 The route auth-status test proves typed wire reduction only. Production parser/process \
                 proof is separate: `access_config::tests::official_auth_status_parsing_uses_exact_subscription_grammar_and_never_returns_output`, \
                 `official_cli_bridge::tests::auxiliary_auth_status_runner_is_job_contained_bounded_and_zeroizes_canary_output` \
                 (Windows), and `operator_chat_route_tests::logged_in_cli_requires_matching_registered_launch_builder`.\n\n\
                 Exact backend leak proof target: \
                 `cloud_byok_access_config_leak_tests::byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain`. \
                 It uses a real `OsKeychainSecretsVault`, proves the key round-trips only for provider \
                 use, and checks logs / Flight Recorder-adjacent tracing / audit rows / Debug output / \
                 HTTP bodies for the canary.\n\n\
                 Exact native Argus proof targets: \
                 `test_cloud_models_settings_argus::cloud_models_controls_are_addressable_and_gemini_is_never_offered`; \
                 `test_cloud_models_settings_argus::typing_and_saving_a_byok_key_clears_the_ui_buffer`; \
                 `test_cloud_models_settings_argus::cloud_models_key_entry_renders_when_backend_unreachable`; \
                 `test_cloud_models_settings_argus::typed_byok_key_is_wiped_from_egui_memory_after_close`; \
                 `test_cloud_models_settings_argus::cli_bridge_auth_status_renders_all_three_states_for_claude_and_codex`; \
                 `test_cloud_models_settings_argus::cli_bridge_login_records_the_official_command_without_stealing_focus`. \
                 These prove stable AccessKit IDs and rendering for the three typed UI states; they do not \
                 claim live provider expiry detection. They also prove no Gemini row, static BYOK fallback when the backend \
                 is unreachable, UI key-buffer wiping, an addressable foreground-launch confirmation, \
                 and fixed provider-owned CLI login command vectors without terminal launch in the \
                 headless test shell.\n\n\
                 Exact detached-Settings-window proof targets (native, same-shell AccessKit): \
                 `test_settings_dialog::settings_popout_control_detaches_into_its_own_argus_window_and_hides_the_modal`; \
                 `test_settings_dialog::re_docking_the_detached_settings_window_restores_the_modal`; \
                 `test_settings_dialog::closing_the_detached_settings_window_restores_modal_availability`; \
                 `test_settings_dialog::open_settings_while_detached_keeps_exactly_one_settings_host` \
                 (exact command: `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target \
                 --manifest-path src/frontend/handshake_native/Cargo.toml --test test_settings_dialog \
                 settings_popout_control_detaches_into_its_own_argus_window_and_hides_the_modal -- --exact`). \
                 They prove `settings.popout` is addressable, that detaching publishes the \
                 `popout-window-settings` (`Role::Window`, title `Handshake – Settings`) root and \
                 registers the `popout-settings` Argus window while the modal's `settings.dialog` root \
                 stops rendering, that every settings section (including `settings.cloud.*`) stays \
                 addressable in the detached window, and that re-dock / close / a repeated \
                 `OpenSettings` always leave exactly one settings host with no stale Argus \
                 registration. Honest headless scope: kittest embeds immediate viewports \
                 (`embed_viewports() == true`), so these prove the content, the window-root node, the \
                 mutual exclusion, and the registry lifecycle in-process; a real second top-level OS \
                 window and a native title-bar close still require the live wgpu/winit backend and are \
                 not simulated.",
            ),
        ],
        anchors: vec![
            page_link("model-lane-cloud-projection-consent"),
            page_link("permissions-and-safety"),
            route_anchor("GET", "/model-access/providers"),
            route_anchor("PUT", "/model-access/byok/:provider/key"),
            route_anchor("DELETE", "/model-access/byok/:provider/key"),
            route_anchor("POST", "/model-access/cli-bridge/:provider/login"),
            spec_anchor("2.3.13.11"),
            spec_anchor("10.15.8"),
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

fn page_model_lane_schema() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-schema".into(),
        title: "Dexterity Model-Lane Schema".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into()],
        sections: vec![
            section(
                "purpose",
                "What Dexterity records",
                "Dexterity is the internal kernel for model switching and model launching. It \
                 records every launchable or switchable participant as ModelLaneRun, ModelLane, \
                 and ModelLaneMessage rows in PostgreSQL. Cloud, local, CLI, human, subagent, \
                 and validator lanes do not speak through hidden peer chat authority: models \
                 propose typed messages and artifacts, while Handshake performs deterministic \
                 storage, EventLedger append, validation, promotion, and replay.",
            ),
            section_with_json(
                "schema",
                "Runtime schema",
                "The stable machine schema IDs are `hsk.model_lane_run@1`, \
                 `hsk.model_lane@1`, and `hsk.model_lane_message@1`; Dexterity is the \
                 display/kernel name, not a schema rename. Required fields include \
                 `locus_binding_ref`, `event_ledger_seq`, `payload_sha256`, \
                 `replay_order_key`, `recovery_state`, and `promotion_receipt_ref`. \
                 ModelLaneRun also carries FEMS posture fields: `memory_pack_ref`, \
                 `memory_pack_hash`, `determinism_mode`, `budget_summary_ref`, \
                 `selected_model_id`, `candidate_model_ids`, `procedural_review_status`, \
                 `truncation_warning_ref`, and `rejection_reason_refs`. \
                 Payloads live by ArtifactStore reference, shared edits carry CRDT refs and \
                 state vectors, and every persisted row has EventLedger evidence.",
                json!({
                    "kernel_name": "Dexterity",
                    "schemas": [
                        "ModelLaneRun",
                        "ModelLane",
                        "ModelLaneMessage"
                    ],
                    "schema_ids": [
                        "hsk.model_lane_run@1",
                        "hsk.model_lane@1",
                        "hsk.model_lane_message@1"
                    ],
                    "required_fields": [
                        "locus_binding_ref",
                        "event_ledger_seq",
                        "payload_sha256",
                        "replay_order_key",
                        "recovery_state",
                        "promotion_receipt_ref",
                        "memory_pack_ref",
                        "memory_pack_hash",
                        "determinism_mode",
                        "budget_summary_ref",
                        "selected_model_id",
                        "candidate_model_ids",
                        "procedural_review_status",
                        "truncation_warning_ref",
                        "rejection_reason_refs"
                    ],
                    "runtime_entrypoint": "SwarmCoordinator::spawn_session + SpawnRequest::with_dexterity_launch",
                    "fail_closed": "Dexterity launch contract requires ModelLaneStore; failed recording cancels/unloads the LiveSession before spawn success"
                }),
            ),
            section(
                "workflows",
                "Operator and model workflow",
                "Create or resume a Dexterity run through `SwarmCoordinator::spawn_session` with \
                 `SpawnRequest::with_dexterity_launch`, record each lane with its launch authority \
                 and runtime binding, then write lane messages as typed proposals, critiques, \
                 status updates, tool results, promotion requests, or recovery messages. \
                 `record_message` is idempotent by `idempotency_key`: same key and same \
                 `payload_sha256` returns the existing message; same key with a different \
                 payload fails closed. Same-key write races serialize through PostgreSQL \
                 transaction-scoped advisory locks before EventLedger append. Replay uses \
                 `event_ledger_seq`, not timestamps.",
            ),
            section(
                "recovery",
                "Recovery and diagnostics",
                "Recovery starts from PostgreSQL plus EventLedger: reload ModelLaneRun, lanes, \
                 and messages ordered by `event_ledger_seq`; inspect `recovery_state`, \
                 failstate refs, lease/reclaim fields, and Locus ownership before relaunch. \
                 HBR-INT-009 posture: Flight Recorder/EventLedger is WIRED through \
                 `dexterity_model_lane` EventLedger rows; internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer and must observe these records without becoming \
                 Dexterity authority.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact ModelLane schema proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests model_lane_schema_persists_and_replays_eventledger_rows -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests dexterity_launch_records_real_swarm_spawn_session_runtime_path -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests model_lane_schema_serializes_competing_terminal_updates -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests model_lane_schema_rejects_missing_locus_binding_and_idempotency_conflict -- --exact`. \
                 These exercise real PostgreSQL, EventLedger, schema registry rows, \
                 SwarmCoordinator runtime launch wiring, ContextBundle, ToolGate, ArtifactStore, \
                 Locus validation, capability validation, idempotency, and replay. There is no \
                 SQLite, mock, or structs-only fallback for Dexterity proof.",
            ),
        ],
        anchors: vec![
            spec_anchor("4.3.9.2.5"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "model_lane_schema_pg_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_model_runtime_registry_and_loom_degrade() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-runtime-registry-and-loom-degrade".into(),
        title: "Model Runtime Registry and Loom Semantic Degrade".into(),
        page_kind: "workflow",
        audience: "model_and_operator",
        spec_anchors: vec!["4.2.3".into(), "4.3.9".into(), "10.13".into()],
        sections: vec![
            section(
                "workflows",
                "Inspect the durable model runtime registry",
                "In the Rust-native app choose `MODELS` then `Open Model Runtime` \
                 (`menu.models.model-runtime`), choose `STUDIO` then `Model Runtime`, or choose \
                 `Settings` then `Model Runtime` then `Open Model Runtime` \
                 (`settings.model-runtime.open`). The pane reads the production \
                 `GET /model-runtime/registry` projection off the frame thread. Refresh re-reads \
                 PostgreSQL/EventLedger authority. A row is `LIVE / READY` only when its current \
                 runtime UUID and label equal the atomically committed last observation; unloaded \
                 rows remain `DORMANT` and expose no current UUID. Inspect model id, canonical artifact \
                 path plus SHA-256, adapter/runtime state, KV bytes/cap/hit rate/quantization, ordered \
                 LoRA ids/strengths, typed steering availability, ProcessOwnershipLedger link, tokens/s, \
                 VRAM, last call plus computed elapsed time, and the expandable typed engine-internals \
                 projection before launching or diagnosing a model lane. `Open in Flight Recorder` \
                 performs in-app navigation and carries the canonical ProcessOwnershipLedger reference; \
                 it does not hand a custom URI to the OS. `Inspect engine internals` is a read-only \
                 native drilldown and is enabled only when the typed engine-internals projection is \
                 available. `Quiesce model` posts schema-versioned `POST /model-runtime/control` for a \
                 current READY runtime and accepts success only from the matching typed receipt. \
                 Quiesce stops new admission to that model without claiming unload, process STOP, \
                 registry mutation, or selection rebound. Unload is enabled only when the row is \
                 READY, is not `application/default`, has matching embedded lifecycle authority, and \
                 has no other READY model sharing its adapter; the request carries the projected \
                 catalog revision and success requires quiesce, unload, catalog update, and durable \
                 ProcessOwnershipLedger STOP in the typed receipt. Compatible-adapter swap is enabled \
                 only with lifecycle, runtime-ledger, embedded-runtime, durable selection-rebind \
                 authority, and no READY sibling sharing the source adapter. It sends the opposite \
                 compatible adapter plus projected catalog and selection revisions; the native button \
                 names that target explicitly as `Swap to CandleRuntime` or `Swap to LlamaCppRuntime` \
                 and stays disabled if no compatible target exists. Success means the \
                 target adapter loaded and durably STARTed, the source quiesced/unloaded and durably \
                 STOPped, PostgreSQL adapter selection rebound, the catalog replacement published, \
                 application selection rebound when applicable, and `result_model_id` names the new \
                 boot UUID. Any capability, compatibility, CAS, lifecycle, or receipt mismatch stays \
                 fail-closed and Refresh re-observes authority. \
                 PostgreSQL owns `application/default` and \
                 `embeddings/default`; boot restores both by stable artifact SHA-256 before exposing \
                 routing. The `ACTIVE DEFAULT MODEL` row owns new application default-routed calls. \
                 Only a READY completion-role row with `default_selectable = true` exposes `Switch to …`; \
                 an embedding-role row may own `embeddings/default` but is not eligible for the \
                 application/default switch, and Operator Chat omits it from its default-model picker. That action \
                 posts `POST /model-runtime/selection`, serializes against concurrent swaps, \
                 prevalidates projection integrity, and rejects stale, non-READY, or embedding-role \
                 targets before mutation. It appends the active-selection EventLedger event and \
                 PostgreSQL compare-and-set in one transaction, then publishes the committed selection \
                 to the current router projection and cancels prior-default requests. Success returns \
                 `selection_receipt_ref`. Invalid input, stale target, embedding-role target, integrity \
                 failure, timeout, audit failure, or PostgreSQL revision conflict leaves the prior \
                 durable model selected and shows typed recovery guidance. Database or authority \
                 failure returns `503 MODEL_RUNTIME_REGISTRY_UNAVAILABLE`; restore authority, then \
                 Refresh to re-observe the durable projection.",
            ),
            section(
                "inputs_outputs",
                "Registry and catalog contract",
                "Inputs are the persisted artifact identity, runtime binding, declared \
                 capabilities, explicit persisted `completion` or `embedding` runtime role, and causation-linked selection history. Outputs are stable \
                 registry rows and `ModelCatalog` entries containing the per-boot model UUID, \
                 display/base-model label, artifact SHA-256, runtime binding, embedding \
                 capability/dimension, runtime role, `default_selectable`, READY state, and PostgreSQL active-purpose markers/revisions. The selector \
                 changes only `application/default`; `embeddings/default` is restored independently. Durable artifact-to-adapter rebinding remains \
                 a separate governed operation and is not performed by this panel. Unknown model lookup returns the explicit \
                 `unknown model` sentinel; an empty registry returns an empty list.",
            ),
            section(
                "recovery",
                "Loom dimension mismatch and recovery",
                "If the selected embedding output dimension is not 768, Loom reindex and search \
                 degrade to keyword/trigram instead of returning a hard error. The response carries \
                 `semantic_unavailable_reason = DimMismatch{expected, actual}` and the runtime emits \
                 `FR-EVT-LOOM-SEMANTIC-DEGRADED`. Recover by configuring the dedicated embedding \
                 model documented in [[dedicated-embedding-model-routing]] with the required \
                 dimension, then retry. Missing migrations, PostgreSQL failure, malformed rows, \
                 duplicate artifact hashes, adapter/role conflicts, or an invalid EventLedger selection \
                 chain fail closed; restore the current migration/database authority and the \
                 persisted SHA/binding rather than editing durable rows or revisions. HBR-INT-009 \
                 posture is explicit here: PostgreSQL/EventLedger plus Tier-1 Flight Recorder are \
                 WIRED; native `internal_diagnostics` is WIRED through its producer and Problems \
                 projection; Palmistry is WIRED through its authenticated watcher and survivor \
                 recovery importer.",
            ),
        ],
        anchors: vec![
            route_anchor("GET", "/model-runtime/registry"),
            route_anchor("POST", "/model-runtime/selection"),
            route_anchor("POST", "/model-runtime/control"),
            route_anchor("GET", "/usermanual/features"),
            page_link("dedicated-embedding-model-routing"),
            page_link("model-lane-schema"),
        ],
    }
}

fn page_embedded_model_lifecycle_ledger() -> NewUserManualPage {
    NewUserManualPage {
        slug: "embedded-model-lifecycle-ledger".into(),
        title: "Embedded Model Lifecycle Ledger & Fail-Closed Observability".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["3.6.2".into(), "4.6.1".into(), "4.2.3.2".into()],
        sections: vec![
            section(
                "purpose",
                "What this surface is",
                "When a local model is configured, the default LlmClient loads the selected proven \
                 Candle or opt-in llama.cpp model in-process through the embedded ModelRuntime. \
                 The llama.cpp path captures one configured GGUF source handle into a private \
                 retained stage, validates a GGUF-specific exact-byte integrity receipt, and gives \
                 only that staged path to the native loader. A binding whose runtime feature or \
                 exact-byte proof is unavailable still fails closed without READY exposure. This library load \
                 spawns no OS process, so master-spec §3.6.2 clause (1) \
                 (child of a SandboxAdapter, no bare std::process::Command) is satisfied \
                 vacuously; the ENFORCED obligation is clause (2): a ProcessOwnershipLedger START \
                 row on load and a matching STOP row at the explicit logical-shutdown boundary \
                 accepted by the MT-013 contract review, written to the same \
                 `kernel_process_lifecycle` table the swarm factory uses. Because there is no OS \
                 process, the START row is honestly pid-less (`os_pid = NULL`); on the valid path \
                 `process_uuid` equals the model's minted UUIDv7. If a runtime returns a non-v7 or \
                 duplicate model id, boot emits a distinct UUIDv7 quarantine START first and records \
                 the reported id plus `identity_contract_violation` in metadata, preventing row \
                 aliasing while keeping every successful load observable. A synthetic pid is forbidden.",
            ),
            section(
                "workflows",
                "Load, shutdown, and the STOP seam",
                "Before any artifact access, boot validates the persistent selection authority. It then \
                 reserves one START and one future STOP \
                 queue permit for every configured primary/embedding runtime as an all-or-none set. \
                 Each selected runtime must return exact-byte artifact integrity proof before its \
                 reserved START transition or any READY exposure. \
                 Candle opens each behavior-bearing source once: weights are copied into an unnamed \
                 private staging file and loaded from its immutable read-only mapping, while captured \
                 config and optional tokenizer bytes are parsed directly. The runtime computes a \
                 path-independent canonical receipt containing bundle, weights, config, and tokenizer \
                 SHA-256 values plus exact lengths. For llama.cpp, a bounded single-open copy creates \
                 a private retained `model.gguf`; digest, GGUF magic, single-file/split rejection, and \
                 tokenizer metadata are validated only from that stage. Linux/Android use a sealed \
                 anonymous memfd and bind every later open to its `/proc/self/fd` descriptor path; \
                 Windows retains a deny-write/delete file handle. Other Unix targets fail closed until \
                 a fresh-offset sealed descriptor path is proven. Native model construction and \
                 tokenizer loading use only that bound path, mmap is forced off, and the staged bytes are \
                 re-hashed and re-parsed before publication. Its format-specific receipt contains the \
                 canonical `model.gguf` bundle digest plus raw GGUF digest and exact length without \
                 fabricating Candle config or tokenizer components. Boot validates either receipt \
                 against the configured primary artifact digest. After each real load, \
                 `EmbeddedModelProcess::record_reserved_load_with_durable_ack` \
                 consumes its START permit; boot waits for the canonical PostgreSQL transaction to commit \
                 with `synchronous_commit=on` before registration or READY exposure. The ownership handle \
                 writes `model_artifact_sha256` from the verified receipt and embeds the complete \
                 `artifact_integrity_receipt` in bounded START metadata, so the durable row names the \
                 exact bytes the selected runtime consumed rather than merely repeating configuration. \
                 The ownership handle is then held by the LlmClient. The runtime is held behind \
                 `Arc<dyn ModelRuntime>`. Normal app shutdown uses the proven ordered seam below. \
                 A ModelRuntime control unload or compatible-adapter swap may instead take unique \
                 runtime ownership, call `unload(&mut self)`, update catalog/selection authority as \
                 required, and emit the matching reserved STOP only after the unload is proven; a \
                 swap durably STARTs its replacement before quiescing and unloading the source. \
                 In the normal app-shutdown seam, \
                 `LlmClient::shutdown_gracefully` first closes runtime admission, cancels active \
                 work, and waits for worker-owned generate/score/embed guards; only after every \
                 actual thread or blocking task exits does `EmbeddedModelProcess::shutdown` emit \
                 STOP. The synchronous `shutdown` seam requests cancellation but never claims \
                 STOP. Dropping an unquiesced client also emits no STOP and leaves START open for \
                 liveness reconciliation. The pre-reserved STOP cannot be lost to later queue \
                 saturation, and concurrent shutdown callers serialize on the same permit. The \
                 binary wires `axum::serve(...).with_graceful_ \
                 shutdown(...)` on Ctrl-C/SIGTERM, gives accepted connections at most 30 seconds \
                 to drain, then runs an ordered teardown: cancel/join background AppState owners \
                 -> close runtime admission and quiesce actual workers -> emit STOP only on \
                 proven idle -> drop the final runtime-owning AppState -> bounded ledger \
                 drain-and-join -> stop the managed cluster -> finish remaining shutdown checks \
                 -> release the OS-owned runtime lease immediately before backend return. \
                 The writer stops receiving \
                 when a retained failed batch reaches capacity, applies channel backpressure, and \
                 retries instead of discarding an already-accepted reserved STOP. Without the \
                 graceful-shutdown handler the process would be OS-killed and the STOP would never \
                 fire. If accepted connections exceed their drain deadline, reserved STOP permits \
                 are relinquished before runtime quiescence is attempted; even an idle runtime \
                 cannot emit STOP while Axum connection tasks still retain AppState. The writer \
                 drains the open START evidence and the process exits nonzero with the AppState and \
                 OS lease intentionally retained until process death. If any runtime worker \
                 misses the quiescence deadline on the normally drained path, no STOP is emitted: \
                 reserved STOP permits are \
                 relinquished, the writer drains the open START evidence, and the process exits \
                 nonzero without explicitly releasing the OS lease. OS process death then releases \
                 the lease and the next boot reconciles the surviving START. The durable graceful STOP \
                 reason is `llm-client-shutdown`; no more specific shutdown-trigger attribution is \
                 persisted. Candle and llama.cpp token streams use a bounded 64-slot channel: \
                 nonterminal data may occupy at most 63 slots, preserving one slot for a terminal \
                 token or error. A saturated producer applies cancellation-aware backpressure; \
                 cancellation inserts `FinishReason::Cancelled` into the reserved slot before the \
                 worker exits, while a dropped consumer closes the worker path instead of letting \
                 an unbounded queue retain generated tokens.",
            ),
            section(
                "recovery",
                "Hard-crash orphan reconcile",
                "A kill -9 / power-loss still leaves the START row open (`stopped_at IS NULL`). \
                 That orphan is session-less (`parent_session_id IS NULL`) and pid-less \
                 (`os_pid IS NULL`), so neither the session-scoped restart-resume reclaim nor the \
                 swarm reclaim (both filter on `parent_session_id = $session`) can EVER match it. \
                 Each backend that resolves an actually configured embedded local lane holds an \
                 exclusive OS-owned loopback UDP lease and stamps the exact versioned descriptor \
                 into every embedded START: instance UUID, host-scope id, \
                 protocol, loopback address, and port. PostgreSQL connection loss or restart does \
                 not release that lease. On the next boot, \
                 `reclaim_pidless_embedded_orphans` strictly decodes each prior descriptor. It \
                 ignores foreign-host rows, while incomplete, malformed, conflicting, ambiguous, \
                 or internally terminal rows stay open and make the typed report deferred/incomplete. For \
                 a same-host candidate it tries to bind the exact endpoint: address-in-use protects \
                 a live owner (or safe port reuse), while a successful exclusive claim is held \
                 through a short transaction-scoped PostgreSQL mutex and the exact descriptor \
                 update. Transaction-local two-second lock and three-second statement deadlines \
                 leave contended rows open and report them as deferred instead of hanging boot. \
                 Each boot examines at most 16 eligible runtime-instance groups through a durable \
                 per-host cyclic keyset cursor. The cursor advances across live/protected and \
                 malformed-ID groups so a fixed leading page cannot starve later stale instances; \
                 a separate bounded unsafe-row probe prevents prefilter-excluded corrupt rows from \
                 being reported as complete. A whole-table \
                 conflict check prevents a conflicting descriptor outside that bounded batch from \
                 being reclaimed, and a typed report marks when another boot sweep is required. \
                 The cutoff bounds the candidate scan but is never treated as liveness \
                 proof. Confirmed stale rows are closed by exact descriptor, setting `stopped_at`, a \
                 sentinel `exit_code`, and `stop_reason = 'orphan_reclaim_pidless_embedded_boot'`. \
                 Unverifiable metadata remains open rather than risking closure of a live owner. \
                 An open row that already carries `exit_code` or `stop_reason` is also treated as \
                 internally inconsistent and left open for operator inspection; reconciliation \
                 never preserves or overwrites that malformed terminal metadata while claiming a \
                 successful automatic STOP. \
                 Session-scoped process reclaim reserves a lossless STOP queue permit before kill, \
                 renews a UUIDv7-plus-generation fenced claim during slow termination, and reports \
                 STOP only after PostgreSQL store acknowledgement. A post-kill write failure remains \
                 `kill_succeeded_pending_stop`; retry persists STOP without killing again, while stale \
                 claimant tokens cannot renew, release, or finalize a newer claim. Before kill, the \
                 claimant durably enters `reclaim_kill_in_progress`; that phase is never lease-taken \
                 over, so a second backend cannot double-kill a blocked process. The stable \
                 `kill_operation_uuid` is the sandbox adapter idempotency key across retries. If the \
                 backend crashes during that phase, run the bounded session recovery sweep: adapter \
                 `succeeded` evidence advances to pending STOP without another kill; `failed` or \
                 `not_started` evidence releases the claim and retries the same operation UUID; \
                 `in_progress`, `unknown`, status-query errors, and transition errors remain truthfully \
                 open as typed per-operation outcomes while later independent operations continue. \
                 Malformed recovery rows likewise remain open with their raw process/operation \
                 identity and a typed repair error; they never panic or poison later rows in the \
                 bounded sweep. \
                 Re-run the sweep after adapter recovery; never fabricate STOP from unknown evidence. \
                  `HANDSHAKE_HOST_SCOPE_ID`, when set, must be stable and globally unique per OS \
                  or network namespace and must never be copied to another host. It is mandatory \
                  for non-loopback PostgreSQL and for a loopback URL that reaches PostgreSQL \
                  through an SSH tunnel, port forward, container, WSL, or another network \
                  namespace. Automatic loopback derivation is allowed only when this backend \
                  process started the exact managed PostgreSQL endpoint; an adopted, external, \
                  forwarded, or otherwise unproven loopback endpoint fails closed without the \
                  explicit value. This provenance gate prevents two hosts connected through \
                  identical localhost tunnel URLs from sharing an inferred host scope. Legacy \
                  endpoint-only scopes are deliberately not adopted or reconciled under the new \
                  identity because their originating host is ambiguous; inspect and close those \
                  rows manually after confirming the original process is gone. \
                  Duplicating the value across physical hosts can let one host mistake \
                 another host's live row for a local orphan because their UDP loopback namespaces \
                 are independent. If duplication is suspected, stop the affected backends, assign \
                 a different stable value to each namespace, inspect the old open ledger rows, and \
                 restart; do not rely on automatic reconciliation of rows stamped with the \
                 duplicated scope. Cloud-only and unconfigured-local boots do not acquire a \
                 model-runtime lease. For a configured local lane, an unavailable host scope or \
                 lease disables local inference before artifact access without aborting the rest \
                 of the backend. A successful typed reconciliation report may leave protected or \
                 bounded deferred rows open and warns that a later sweep is required; an actual \
                 reconciliation error is logged as an error and disables configured local inference \
                 before artifact access for that boot. \
                 HBR-INT-009 posture: Flight Recorder / EventLedger is WIRED (ProcessOwnershipLedger \
                  rows + per-call Flight Recorder events); internal_diagnostics is WIRED and Palmistry is WIRED through the native diagnostics and survivor-recovery paths; they observe these records without becoming their \
                 authority).",
            ),
            section(
                "safety",
                "Fail-closed and embedding Flight Recorder events",
                "Master-spec §4.2.3.2(3): every LlmClient call emits a Flight Recorder event — \
                 the error/disabled paths too. On the default lane a fail-closed \
                 `DisabledLlmClient::completion` emits a zeroed-usage `llm_inference` FR event \
                 (error_kind `llm_disabled`) at CALL TIME, never at construction. The embedding \
                 lane is likewise Flight-Recorder-correlatable: \
                 `LocalModelRuntimeLlmClient::embedding` emits `data_embedding_computed` on \
                 success and an error FR event on failure, and the previously-silent \
                 `DisabledLlmClient::embedding` fallback now emits a call-time FR event \
                 (error_kind `embedding_disabled`) before returning EmbeddingUnsupported. \
                 Embeddings carry no TokenUsage, so the embedding FR event is a product extension \
                 of the §4.2.3.2(3)/§11.5 correlatable-call discipline, not a literal MUST.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact Rust proof targets: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test embedded_model_ledger_tests` \
                  (a valid embedded load emits a pid-less START keyed on the minted UUIDv7, while \
                   invalid/duplicate returned identities receive distinct quarantine START rows; the graceful- \
                  shutdown sequence flushes the STOP through the background writer; the hard-crash \
                  orphan-reconcile sweep closes a stale pid-less embedded START; both the primary \
                  chat model and optional dedicated embedding model get START/STOP rows; supplied \
                  ledger START failure fails closed instead of leaving an active unledgered client); \
                  `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test llm_client_local_routing_tests` \
                  (fail-closed and embedding Flight Recorder events on every call path, including \
                  DisabledLlmClient::embedding, with `data_embedding_computed` using the validated \
                  Flight Recorder payload shape); \
                  `cargo test -j 1 --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test process_ledger_writer_tests` \
                  (reserved STOP capacity survives retained-batch store failure/recovery and \
                  invalid batch sizing fails closed); \
                  `cargo test -j 1 --manifest-path src/backend/handshake_core/Cargo.toml --features \"test-utils,candle-runtime-engine\" --lib candle_stream_reserves_terminal_slot_under_saturated_cancellation` \
                  and `cargo test -j 1 --manifest-path src/backend/handshake_core/Cargo.toml --features \"test-utils,llama-cpp-runtime-engine\" --lib llama_stream_reserves_terminal_slot_under_saturated_cancellation` \
                  prove saturated cancellation preserves the explicit terminal outcome. These are supporting deterministic proofs; MT-013 \
                  READY_FOR_VALIDATION additionally requires the live real-load proof command \
                  `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features \"test-utils,candle-runtime-engine\" --test candle_e2e_smoke mt013_real_candle_default_load_emits_process_ledger_start_stop -- --ignored --exact --nocapture` \
                  with `HANDSHAKE_TEST_CANDLE_MODEL_DIR` pointing at real Candle weights and output \
                  containing `[MT-013_REAL_CANDLE_LEDGER_DUMP]` with matching START/STOP rows. The \
                  deterministic suite exercises real PostgreSQL/EventLedger for the orphan-reconcile \
                  leg; the live Candle command fails loudly when the real model directory is absent; \
                  there is no SQLite or mock fallback.",
            ),
        ],
        anchors: vec![
            page_link("manual-toc"),
            spec_anchor("3.6.2"),
            spec_anchor("4.6.1"),
            spec_anchor("4.2.3.2"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "embedded_model_ledger_tests".into(),
                http_method: "",
            },
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "llm_client_local_routing_tests".into(),
                http_method: "",
            },
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "process_ledger_writer_tests".into(),
                http_method: "",
            },
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "candle_e2e_smoke::mt013_real_candle_default_load_emits_process_ledger_start_stop".into(),
                http_method: "",
            },
        ],
    }
}

fn page_dedicated_embedding_model_routing() -> NewUserManualPage {
    NewUserManualPage {
        slug: "dedicated-embedding-model-routing".into(),
        title: "Dedicated Embedding Model Routing".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.2.3.2".into(), "4.6.3".into()],
        sections: vec![
            section(
                "purpose",
                "What this surface is",
                "LoomSearchV2 semantic indexing/search uses a dedicated local embedding model when one is configured. The default chat/completion model remains the LlmClient profile identity, but embedding calls are routed through `ModelCatalog::embedding_model_for_dim(768)` and require a READY local registration with `supports_embedding=true` and `embedding_dimension=768`. A chat-only model is never used as an embedding fallback.",
            ),
            section(
                "startup",
                "Configure the second local model",
                "The primary chat model uses `HANDSHAKE_LOCAL_MODEL_PATH`, `HANDSHAKE_LOCAL_MODEL_SHA256`, `HANDSHAKE_LOCAL_MODEL_BINDING`, and `HANDSHAKE_LOCAL_MODEL_NAME`. The optional embedding model uses the parallel `HANDSHAKE_LOCAL_EMBEDDING_MODEL_PATH`, `HANDSHAKE_LOCAL_EMBEDDING_MODEL_SHA256`, `HANDSHAKE_LOCAL_EMBEDDING_MODEL_BINDING`, `HANDSHAKE_LOCAL_EMBEDDING_MODEL_NAME`, and `HANDSHAKE_LOCAL_EMBEDDING_MODEL_DIMENSION` variables. The embedding dimension defaults to 768, matching `LOOM_SEARCH_EMBEDDING_DIM` and the `loom_block_search_index.embedding vector(768)` contract.",
            ),
            section(
                "workflows",
                "Reindex and search",
                "On reindex, LoomSearchV2 resolves the READY 768-dimensional embedding registration and calls `LlmClient::embedding` with that registration's per-boot UUIDv7. The durable search row stores a stable embedding-space key in `loom_block_search_index.embedding_model` (`embedspace:<artifact_sha256>:dim:<dimension>`), not the per-boot routing UUID. On search, the query vector carries the same `query_embedding_model` embedding-space key; PostgreSQL computes vector similarity only against rows whose stored `embedding_model` matches, preventing cross-model vector-space contamination while preserving same-model scoring across restart.",
            ),
            section(
                "failure_modes",
                "Fallback and recovery",
                "If no READY 768-dimensional embedding-capable registration exists, LoomSearchV2 degrades to keyword/trigram search with `semantic_available=false` and `SemanticUnavailableReason::NoModel`; it does not call the chat model as an embedding fallback. If a selected embedding runtime returns a vector whose length is not 768, LoomSearchV2 emits `FR-EVT-LOOM-SEMANTIC-DEGRADED`, returns keyword/trigram results, and surfaces `SemanticUnavailableReason::DimMismatch`. Repair by configuring a model whose declared and actual embedding dimension are both 768, then reindex affected blocks.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact Rust proof targets: `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_registry_tests mt016_model_capabilities_declare_embedding_dimension_and_validate_consistency -- --exact`; `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_catalog_tests mt016_catalog_selects_ready_embedding_capable_model_distinct_from_chat -- --exact`; `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test llm_default_boot_resolution_tests mt016_default_boot_registers_distinct_embedding_model_when_configured -- --exact`; `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test loom_search_v2_tests mt016_loom_search_routes_reindex_and_search_to_registry_embedding_model -- --exact`; `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test loom_search_v2_tests mt016_loom_search_no_embedding_model_degrades_without_chat_embedding_call -- --exact`; `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_behavior_coverage_tests dedicated_embedding_model_behaviors_have_manual_coverage -- --exact`.",
            ),
        ],
        anchors: vec![
            page_link("manual-toc"),
            page_link("notes-loom-surface"),
            page_link("embedded-model-lifecycle-ledger"),
            spec_anchor("4.2.3.2"),
            spec_anchor("4.6.3"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "dedicated_embedding_model_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_operator_chat_launch() -> NewUserManualPage {
    NewUserManualPage {
        slug: "operator-chat-launch".into(),
        title: "Operator Chat / Launch Work-Surface".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into(), "4.3.9.4.4".into()],
        sections: vec![
            section(
                "purpose",
                "What this surface is",
                "The operator chat/launch pane (native egui, PaneType::OperatorChatLaunch, opened \
                 from the MODELS menu leaf `menu.models.operator-chat`) lets the operator pick a model \
                 lane (LOCAL / BYOK cloud / official CLI / SUBAGENT), pick a folder/worktree as the session's \
                 working directory, type a prompt, and launch an interactive lane session. \
                 The surface is also available as a persisted Settings > Swarm default via \
                 `settings.swarm-operator-chat-default-open`. The selected lane is structured as \
                 `lane_kind` + `model_id`, with \
                 `cloud_provider` or `cli_provider` attached when the selected row needs a provider. \
                 SUBAGENT rows are no-OS Dexterity lanes owned by the SubagentManager; they carry \
                 no process id and use `no_os_process_reason_ref` instead. \
                 The launched conversation, the model's exposed reasoning/thought, and its tool \
                 calls are captured as typed ModelLaneMessage records under a live ModelLaneRun \
                 and mirrored to the Flight Recorder. `GET /operator-chat/models` is the canonical \
                 picker inventory: it returns local, BYOK, official-CLI, subagent, excluded, and \
                 governed session rows without exposing secret key material.",
            ),
            section(
                "workflows",
                "Pick a model + folder, launch, and watch the transcript",
                "1. The picker enumerates LOCAL models via the MT-014 `ModelCatalog::list()`, \
                 BYOK cloud providers via the live cloud access registry, official CLI rows \
                 from provider-specific CLI bridge configs that are enabled only when the provider \
                 executable is found on PATH, and a SUBAGENT row for SubagentManager-owned lanes. \
                 An unconfigured cloud or CLI provider degrades to \
                 `unavailable`, never a mock. \
                 2. Selecting a model records an auditable selection decision \
                 (`ModelCatalog::record_selection_decision_with_context` -> \
                 `FR-EVT-MODEL-SELECTION-RECORDED`) with typed `selection_context` fields for \
                 `lane_kind`, `model_id`, `cloud_provider`, `cli_provider`, and the selected \
                 working directory when available, \
                 distinct from launch (spec 4.3.9.4.4). \
                 3. The operator selects an `owner_session_id` from the backend inventory. The \
                 backend `SessionRegistry` requires that exact owner session to be active, derives \
                 `parent_session_id` from its registered lineage, requires the parent to be active, \
                 and verifies `owner.spawn_depth == parent.spawn_depth + 1`; the client cannot \
                 supply or override the parent. Unregistered, inactive, missing, or inconsistent \
                 lineage returns `invalid_owner_session` before launch. \
                 4. The Settings Swarm section has a persisted Operator Chat default-open checkbox; \
                 when enabled, startup navigation opens the Operator Chat pane through the same \
                 runtime tab path as the MODELS menu. \
                 5. The operator selects a folder/worktree; that path is plumbed \
                 `SpawnRequest.working_dir -> CliBridgeConfig.working_dir` so the CLI subprocess \
                 truly runs in that directory. \
                 6. Process-backed launch resolves ONLY through `SwarmCoordinator::spawn_session` \
                 (never a frontend/app-src/direct-endpoint/terminal authority). SUBAGENT launch \
                 resolves through `SwarmCoordinator::launch_operator_subagent_model_lane`, \
                 normalizes via the Dexterity registry, and persists a no-OS ModelLaneRun/ModelLane; \
                 a missing ModelLaneStore fails closed. \
                 The operator's own prompt is persisted as a HUMAN_OPERATOR ModelLane message \
                 (launch_authority=Operator, runtime_binding=HUMAN).",
            ),
            section_with_json(
                "workflows",
                "Six executable policies and durable lifecycle routes",
                "Routing is a distinct, durable production lifecycle, not picker metadata. The six \
                 persisted policies compile to different `hsk.model_lane_routing_graph@1` DAGs: \
                 `local_first` runs `local-attempt` then cloud-consented `cloud-escalation` only \
                 after failure; `cloud_review` runs `local-candidate` then cloud-consented \
                 `cloud-review` after success; `cloud_plan_local_execute` runs cloud-consented \
                 `cloud-plan` then `local-execute`; `parallel_debate` runs `debate-local` and \
                 cloud-consented `debate-cloud` in parallel then `debate-join`; `validator_lane` \
                 runs `validation-candidate` then authority-gated `validator-verdict`; and \
                 `operator_lane` runs `operator-candidate` then authority-gated \
                 `operator-decision`. `POST /operator-chat/routing/lifecycle` executes the graph; \
                 `/routing/recover` reloads and resumes the durable execution; `/routing/authority` \
                 supplies the exact execution/stage/message authority and resumes; and \
                 `/routing/cancel` cancels with a durable reason. Each stage request binds \
                 `stage_id`, optional `lane_id` plus model `selection`, and optional \
                 `authority_lane_id`. The lifecycle request binds `execution_id`, \
                 `selecting_decision_id`, three independent authority refs, canonical run context, \
                 and the stage list. The returned `hsk.model_lane_routing_execution@5` state exposes \
                 status (`running|awaiting_authority|succeeded|failed|cancelled`) and stage state \
                 (`scheduled|claimed|in_flight|awaiting_authority|succeeded|failed|joined|cancelled|compensated`), \
                 attempts, expected run/lane/model/provider, fencing lease, output refs/hashes, \
                 authority refs, and EventLedger identity. Recovery reuses the original canonical \
                 context and launch plan; context, graph, execution-id, or authority mismatch fails \
                 closed rather than dispatching a changed graph.",
                json!({
                    "graph_schema": "hsk.model_lane_routing_graph@1",
                    "execution_schema": "hsk.model_lane_routing_execution@5",
                    "policies": [
                        "local_first",
                        "cloud_review",
                        "cloud_plan_local_execute",
                        "parallel_debate",
                        "validator_lane",
                        "operator_lane"
                    ],
                    "routes": [
                        "POST /operator-chat/routing/lifecycle",
                        "POST /operator-chat/routing/recover",
                        "POST /operator-chat/routing/authority",
                        "POST /operator-chat/routing/cancel"
                    ],
                    "context_fields": [
                        "run_id", "trace_id", "run_span_id", "coordinator_session_id",
                        "locus_ref", "work_packet_id", "micro_task_id", "task_board_id",
                        "owner_session", "initial_input_ref", "initial_input_sha256"
                    ],
                    "authority_fields": [
                        "cloud_consent_receipt_ref", "validator_authority_ref",
                        "operator_authority_ref"
                    ]
                }),
            ),
            section(
                "inputs_outputs",
                "Where the transcript, thought, and tool calls land",
                "The official CLI bridge is forced into stream JSON output so activities are TYPED. Each \
                 COMPLETED activity block from `parse_agent_activity_line` becomes ONE \
                 ModelLaneMessage via `ModelLaneStore::record_message`: a ToolCall -> ToolRequest, \
                 a rendered tool_result -> ToolResult, and the operator prompt / model answer / \
                 exposed thought -> Status messages discriminated by \
                 `diagnostic_payload.activity_kind` (`tool_call|thinking|text|other`). A Flight \
                 Recorder `FR-EVT-AGENT-*` event is emitted alongside each message. `launch()` DRIVES \
                 the launched runtime (`SwarmCoordinator::session_runtime`) and re-homes its REAL \
                 stdout through `capture_cli_stream`, so the persisted messages originate from the \
                 launched session's output. The pane then fetches those rows via \
                 `GET /operator-chat/transcript/:run_id` and RENDERS them. SUBAGENT launches persist \
                 the operator prompt and a ready subagent lane, but do not fabricate model stdout. \
                 Each control carries a \
                 stable AccessKit author_id (`operator-chat.surface`, `operator-chat.picker.model`, \
                 `operator-chat.session.<session_id>` for each real governed session row, \
                 `operator-chat.model.<lane>.<provider>.<model>`, `operator-chat.picker.folder`, \
                 `operator-chat.input.prompt`, `operator-chat.action.refresh-models`, \
                 `operator-chat.action.launch`, `operator-chat.launch.status`, `operator-chat.error`, \
                 `operator-chat.transcript`, `operator-chat.transcript.message.<message_id>`, \
                 `operator-chat.routing.request`, `operator-chat.routing.lifecycle`, \
                 `operator-chat.routing.recover`, `operator-chat.routing.authority`, \
                 `operator-chat.routing.cancel`, `operator-chat.routing.status`, \
                 with `operator-chat.transcript.row.<n>` only as an index fallback). Launch status \
                 is rendered outside the transcript so it cannot masquerade as a fetched message.",
            ),
            section(
                "recovery",
                "Fail-closed and HBR-INT-009 posture",
                "If the coordinator has no ModelLaneStore/PostgreSQL authority the launch is torn \
                 down and returns a LedgerFailed error (the route surfaces `launch_failed_closed`); \
                 no partial lane authority is created. The SHIPPED route is wired to a live \
                 `SwarmCoordinator` + `ModelLaneStore` from `AppState` (via \
                 `build_operator_chat_launch_service`), so a real launch runs; a deployment with no \
                 launch service wired returns `503 launch_not_wired` (and the transcript route \
                 `503 transcript_not_wired`); routing without the production coordinator returns \
                 `503 routing_not_wired`. Invalid launch inputs and routing authority/context \
                 mismatches return `400 bad_request`; governed owner-lineage failures return \
                 `400 invalid_owner_session` with a stable code; coordinator failures return \
                 `500 launch_failed_closed`; ModelLane and recorder failures return \
                 `500 model_lane_error` and `500 recorder_error`. A selection audit can fail \
                 independently with `500 selection_audit_failed` when \
                 `ModelCatalog::record_selection_decision_with_context` or its Flight Recorder \
                 write fails; keep the selection unchanged, restore the catalog/recorder path, \
                 and retry `POST /operator-chat/selection` before launch so selection evidence is \
                 never silently skipped. The official-CLI lane selects a provider-specific \
                 `CliBridgeConfig` from `cli_provider` and fails closed with `ProviderNotConfigured` \
                 until the matching provider executable/config is available; Local lanes resolve the \
                 selected `ModelCatalog` entry to its artifact path/hash and launch through the real \
                 candle/llama path. SUBAGENT lanes must record `RuntimeBinding::Subagent`, \
                 `LaunchAuthority::SubagentManager`, and no `process_ownership_ref`; routing them \
                 through a process factory is a failure. HBR-INT-009 posture: Tier-1 Flight Recorder / \
                 EventLedger is WIRED (agent-activity events + ModelLaneMessage authority + the \
                 selection-decision event); Tier-2 internal_diagnostics is WIRED through the native \
                 producer and Problems projection, and Tier-3 Palmistry is WIRED through the \
                 authenticated watcher and survivor recovery importer; both observe these records \
                 without becoming their authority.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact Rust proof targets use \
                 `CARGO_TARGET_DIR=..\\Handshake_Artifacts\\handshake-cargo-target`. Backend: \
                 `operator_chat_launch_drives_runtime_and_captures_one_message_per_completed_block` \
                 proves launch drives the loopback CLI runtime, persists the HUMAN_OPERATOR prompt, \
                 binds every non-status payload to artifact records, and re-homes REAL stdout into \
                 one ModelLaneMessage per completed activity block; \
                 `operator_chat_launch_stream_error_preserves_partial_capture_and_reclaims_session` \
                 proves partial stdout survives runtime failure and the session is reclaimed; \
                 `operator_chat_subagent_selection_launches_no_os_subagent_lane` proves SUBAGENT \
                 selection persists a no-OS SubagentManager lane and does not call the runtime factory; \
                 `operator_chat_route_tests` proves the launch/models/selection routes; \
                 `build_spawn_request_local_*` proves local catalog/artifact resolution; \
                 `official_cli_provider_selection_honors_requested_provider` proves provider-specific \
                 official CLI dispatch; `mt014_catalog_enumerates_and_labels_configured_model` proves \
                 catalog artifact path exposure. Native: `operator_chat_model_select_fires_audit_and_launch_renders_fetched_transcript` \
                 proves structured selection audit, fetched transcript rows, message-id author_ids, \
                 and launch status outside transcript; `operator_chat_launch_argus_opens_picks_types_and_launches` \
                 proves the pane opens, stable controls are addressable, and the launch button fails \
                 closed on the offline backend; `run_menu_opens_operator_chat_launch` proves MODELS > \
                 Open Operator Chat opens the native tab; `swarm_accessible_actions_listed` \
                 proves `menu.models.operator-chat` is swarm-accessible; `operator_chat_swarm_setting_persists` \
                 and `persisted_swarm_defaults_open_runtime_tabs` prove the Settings Swarm default \
                 is addressable, persisted, and opens the runtime pane.",
            ),
        ],
        anchors: vec![
            page_link("manual-toc"),
            page_link("model-lane-launch-adapters"),
            spec_anchor("4.3.9.2.5"),
            spec_anchor("4.3.9.4.4"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "operator_chat_capture_tests".into(),
                http_method: "",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/models".into(),
                http_method: "GET",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/launch".into(),
                http_method: "POST",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/transcript/:run_id".into(),
                http_method: "GET",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/selection".into(),
                http_method: "POST",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/routing/lifecycle".into(),
                http_method: "POST",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/routing/recover".into(),
                http_method: "POST",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/routing/authority".into(),
                http_method: "POST",
            },
            NewManualAnchor {
                anchor_kind: "http_route",
                anchor_value: "/operator-chat/routing/cancel".into(),
                http_method: "POST",
            },
        ],
    }
}

fn page_model_lane_launch_adapters() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-launch-adapters".into(),
        title: "Dexterity Launch Adapters".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into()],
        sections: vec![
            section(
                "purpose",
                "What Dexterity launches",
                "Dexterity is the internal kernel for model switching and launching. MT-003 \
                 normalizes local, BYOK cloud, official CLI, CLI bridge, human/operator, \
                 subagent, and validator lanes through Rust backend authority. The runtime \
                 entrypoints are `DexterityLaunchAdapterRegistry`, `DexterityNormalizedLaunch`, \
                 `SwarmCoordinator::spawn_session`, `ModelRuntime`, `CloudLane/BYOK`, \
                 `CliBridge`, `Operator`, `SubagentManager`, and `ValidatorRunner`. Official CLI \
                 process lifecycle authority is compile-anchored at `LiveCliSpawner::spawn` and \
                 `HandshakeNativeSandboxAdapter::spawn_attached_with_stdio`; every terminal path \
                 converges on `GuardedCliChild::terminate_and_collect`. Models \
                 propose edits and messages; Handshake performs deterministic validation, \
                 PostgreSQL storage, EventLedger append, replay, promotion, and recovery.",
            ),
            section_with_json(
                "schema",
                "Lane matrix",
                "Each lane declares a runtime binding, launch authority, provider feature \
                 profile, requested/effective execution policy, owner_session, trace/span, \
                 cancellation token boundary, reclaim policy, terminal status mapping, and \
                 either `process_ownership_ref` ProcessOwnershipLedger ownership or an explicit \
                 no-OS-process equivalent. No-OS caller receipts are minted from a live \
                 Ready/Generating authority session and launch rechecks that the authority lease \
                 is still live; they are not offline bearer tokens.",
                json!({
                    "kernel": "Dexterity",
                    "registry": "DexterityLaunchAdapterRegistry",
                    "normalized_record": "DexterityNormalizedLaunch",
                    "lanes": [
                        {"kind": "local", "authority": "ModelRuntime", "backend": "LlmClient -> ModelRuntime adapter; llama.cpp and Candle are adapter backends only"},
                        {"kind": "BYOK cloud OpenAI/Anthropic", "authority": "CloudLane/BYOK", "requires": ["projection_plan_ref", "consent_receipt_ref"]},
                        {"kind": "official CLI", "authority": "HandshakeNative attached-sandbox -> CliBridge", "process_engine_kind": "official_cli_bridge", "ownership": "ProcessOwnershipLedger START/STOP", "cleanup_order": "terminate/reap-before-STOP"},
                        {"kind": "CLI bridge", "authority": "HandshakeNative attached-sandbox -> CliBridge", "process_engine_kind": "official_cli_bridge", "ownership": "ProcessOwnershipLedger START/STOP", "cleanup_order": "terminate/reap-before-STOP"},
                        {"kind": "human/operator", "authority": "Operator", "no_os_process_reason_ref": "required"},
                        {"kind": "subagent", "authority": "SubagentManager", "no_os_process_reason_ref": "required"},
                        {"kind": "validator", "authority": "ValidatorRunner", "no_os_process_reason_ref": "required"}
                    ],
                    "forbidden_launch_authority": [
                        "direct endpoint",
                        "app/src",
                        "app/src-tauri",
                        "frontend IPC",
                        "terminal-only",
                        "unmanaged external model-server proof"
                    ],
                    "reference_only_non_authority": [
                        "docs/model-manual",
                        "app/MODEL_MANUAL.md",
                        "npm/JavaScript proof"
                    ]
                }),
            ),
            section(
                "navigation",
                "Product entrypoints",
                "Tauri IPC (`kernel_swarm_spawn_session`) and scheduled spin-up are request \
                 sources only, not launch authority. The live app bootstraps \
                 `SwarmRuntimeState` with a PostgreSQL `ModelLaneStore`; if that store is \
                 unavailable, model launch startup fails closed instead of constructing a \
                 no-store coordinator. Manual IPC spawns and calendar scheduled spin-ups attach \
                 a core-generated Dexterity contract through \
                 `DexterityLaunchContract::attach_to_spawn_request`, which sets \
                 `SpawnRequest::with_dexterity_launch` plus the WP/MT lineage before \
                 `SwarmCoordinator::spawn_session`. BYOK scheduled spin-ups must persist \
                 `byok_cloud_provider` so OpenAI/Anthropic attribution, projection, consent, \
                 and Flight Recorder/EventLedger records stay deterministic.",
            ),
            section(
                "failure_modes",
                "Failure and recovery",
                "A Dexterity launch fails closed when `ModelLaneStore` is absent, a \
                 ModelLaneStore-backed coordinator is called without \
                 `SpawnRequest::with_dexterity_launch`, a BYOK provider lacks explicit \
                 provider/projection/consent refs, a direct endpoint or frontend/Tauri/terminal-only \
                 launch bypass is requested, cancellation or reclaim metadata is missing, no \
                 process/no-OS ownership boundary exists, or an unsupported tool capability is \
                 requested. Startup failure records carry \
                 `startup_failure_code`, `startup_failure_ref`, `reason_ref`, `recovery_state`, \
                 owner_session, trace/span, terminal status mapping, and EventLedger evidence. \
                 Terminal ModelLane/EventLedger state is written before runtime teardown so a \
                 failed terminal write leaves the handle in `Cancelling` with a durable \
                 `cleanup_pending` receipt and an already-cancelled generation token. \
                 `SwarmCoordinator::retry_pending_session_cleanups` retries the original terminal \
                 intent after the persistence fault clears, without double teardown or duplicate \
                 ProcessOwnershipLedger STOP; the durable receipt advances through \
                 `teardown_succeeded` to `completed`. Terminal writes serialize \
                 per lane before any competing completed/failed/cancelled status can append. A \
                 runtime terminal Failed state records a `terminal-failure://dexterity/<lane_id>` \
                 failure ref instead of leaving the ModelLane failure shape incomplete. Official \
                 CLI execution uses the HandshakeNative attached-sandbox contract: the sandbox \
                 creates the ProcessOwnershipLedger START authority, owns the child for its full \
                 lifetime, and on success, failure, timeout, cancellation, or unwind performs \
                 terminate/reap-before-STOP. If termination, reap, or durable STOP recording fails, \
                 recovery keeps the lifecycle open or cleanup-pending and retries reconciliation; \
                 it never reports a terminal STOP merely because a queue accepted a row.",
            ),
            section(
                "hooks",
                "Tools and diagnostics",
                "Tool-capable lanes must pass capability checks before execution; unsupported \
                 tool capabilities fail before persistence. MT-003 records launch-time ToolGate \
                 decision refs and capability snapshots, but full cross-lane tool execution, \
                 projection fanout, and consent revocation behavior remain in later MTs. \
                 HBR-INT-009 posture: Flight Recorder/EventLedger is WIRED through \
                  `dexterity_model_lane` rows; internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer and observes these records without \
                 becoming launch authority.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-003 proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_all_lane_kinds_through_rust_registry -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_rejects_direct_endpoint_frontend_tauri_and_terminal_bypass -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_cancellation_reclaim_contracts_all_lane_kinds -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_records_factory_failure_through_swarm_coordinator -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests production_builder_wires_model_lane_store_for_failed_dexterity_launch -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_rejects_ready_transition_before_persistence_commit -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_cancel_session_records_terminal_model_lane_state -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_reaper_records_terminal_state_before_teardown -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --lib model_runtime::cloud::official_cli_bridge::tests::explicit_failed_terminate_leaves_start_open_without_stop -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_official_cli_bridge_tests failed_termination_with_never_eof_pipe_returns_within_cleanup_deadline -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_official_cli_bridge_tests continuous_output_cannot_starve_live_timeout_polling -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_official_cli_bridge_tests continuous_output_cannot_starve_live_cancellation_polling -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests dexterity_launch_records_real_swarm_spawn_session_runtime_path -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_launch_user_manual_entry_is_current -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_schema_user_manual_entry_is_current -- --exact`. \
                 These exercise real Rust backend registry normalization, SwarmCoordinator \
                 preflight, coordinator-owned no-OS launch records, ModelRuntime load/unload, \
                 PostgreSQL/EventLedger stream rows, production builder store wiring, factory \
                 failure persistence, fail-closed bypass rejection, no Ready/runtime exposure \
                 before ModelLane persistence, cancellation boundaries, durable cancellation \
                 terminal state, retryable terminal intent before runtime teardown, per-lane \
                 terminal serialization, \
                 reclaim policy, terminal status mapping, startup failure rows, manual parity, \
                 and no-OS-process equivalents. `docs/model-manual`, `app/MODEL_MANUAL.md`, \
                 and npm/JavaScript proof are reference-only and never launch authority.",
            ),
        ],
        anchors: vec![
            spec_anchor("4.3.9.2.5"),
            page_link("model-lane-schema"),
            page_link("model-lane-promotion"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "model_lane_launch_tests".into(),
                http_method: "",
            },
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "official_cli_attached_lifecycle_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_model_lane_promotion() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-promotion".into(),
        title: "Dexterity Routing and Promotion".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into()],
        sections: vec![
            section(
                "purpose",
                "What promotion does",
                "Dexterity keeps model output advisory until Handshake records an explicit \
                 `ModelLanePromotionDecision`. Local models, cloud models, CLI lanes, human \
                 operator lanes, subagents, and validator lanes can propose or critique, but \
                 only the Rust backend writes authority after deterministic checks. The stable \
                 machine schema is `hsk.model_lane_promotion_decision@1`; the EventLedger \
                 aggregate is `model_lane_promotion_decision` and the source component is \
                 `dexterity_model_lane`. The Rust input record is \
                 `NewModelLanePromotionDecision`; replay returns \
                 `ModelLanePromotionDecisionRecord`.",
            ),
            section_with_json(
                "schema",
                "Routing policies and decision fields",
                "MT-004 routing policies are typed Rust data: `local_first`, `cloud_review`, \
                 `cloud_plan_local_execute`, `parallel_debate`, `validator_lane`, and \
                 `operator_lane`. A promotion decision records stable sorted `input_refs`, \
                 `selected_input_refs`, `rejected_input_refs`, validator/operator authority \
                 refs, expected EventLedger aggregate/version, DB-derived current CRDT \
                 `current_base_snapshot_ref` and `current_state_vector`, schema guard, \
                 deterministic tie-break rule, `promotion_gate_ref`, optional \
                 `promotion_receipt_ref`, promoted artifact `ref`/`sha256`/`version`, \
                 trace/span links, idempotency key, Locus WP/MT/task-board ownership, \
                 `final_state`, and a canonical decision hash that is stable across input-ref \
                 ordering. New `ModelLaneMessage` writes also carry typed routing metadata: \
                 `target_role`, `target_session`, `correlation_id`, `requires_ack`, and \
                 optional `ack_for`.",
                json!({
                    "kernel": "Dexterity",
                    "schema_id": "hsk.model_lane_promotion_decision@1",
                    "aggregate_type": "model_lane_promotion_decision",
                    "state_machine": [
                        "advisory",
                        "promotion_requested",
                        "pending_policy",
                        "pending_approval",
                        "approved",
                        "denied",
                        "expired",
                        "executing",
                        "executed",
                        "skipped",
                        "unsupported"
                    ],
                    "routing_policies": [
                        "local_first",
                        "cloud_review",
                        "cloud_plan_local_execute",
                        "parallel_debate",
                        "validator_lane",
                        "operator_lane"
                    ],
                    "required_guards": [
                        "expected_event_ledger_aggregate_type",
                        "expected_event_ledger_aggregate_id",
                        "expected_event_ledger_version",
                        "base_snapshot_ref",
                        "state_vector",
                        "schema_id",
                        "deterministic_tie_break_rule",
                        "validator_authority_ref_or_operator_authority_ref",
                        "promotion_gate_ref",
                        "promotion_receipt_ref",
                        "promoted_artifact_ref",
                        "promoted_artifact_sha256",
                        "promoted_artifact_version",
                        "idempotency_key"
                    ],
                    "message_routing_fields": [
                        "target_role",
                        "target_session",
                        "correlation_id",
                        "requires_ack",
                        "ack_for"
                    ],
                    "canonical_hash_excludes": [
                        "decision row id",
                        "idempotency_key",
                        "timestamps",
                        "EventLedger event id"
                    ]
                }),
            ),
            section(
                "workflows",
                "Promotion workflow",
                "Write advisory `ModelLaneMessage` rows first. Then call \
                 `ModelLaneStore::record_promotion_decision` with all candidate refs and the \
                 expected CRDT/EventLedger/schema state. Dexterity resolves every \
                 `model-lane-message://...` ref from PostgreSQL, derives current CRDT \
                 base/state from the selected advisory rows, and rejects phantom, cross-run, or \
                 non-advisory refs. Approved decisions walk the deterministic \
                 state path `advisory -> promotion_requested -> pending_policy -> \
                 pending_approval -> approved -> executing -> executed`. Denied decisions walk \
                 `advisory -> promotion_requested -> pending_policy -> denied`. Only after an \
                 approved decision exists can `record_message` accept a \
                 `ModelLaneAuthority::Promoted` message for the matching \
                 `promotion_decision_id`, `promotion_gate_ref`, `promotion_receipt_ref`, \
                 `promoted_artifact_ref`, `promoted_artifact_sha256`, and \
                 `promoted_artifact_version`; direct promoted messages fail closed with \
                 PromotionGate resolution wording.",
            ),
            section(
                "workflows",
                "Disagreement and stalled progress",
                "For `parallel_debate`, cloud/local disagreement stays advisory until a \
                 promotion decision selects the winning `Proposal` or `PromotionRequest` and \
                 records rejected `Critique` refs. Stalled work should emit a `Recovery` message with \
                 `recovery_hint_ref`, preserve the ContextBundle handoff, and let validator or \
                 operator lanes append the next advisory verdict. Promotion never makes a \
                 model-authored edit authoritative by itself; the deterministic Rust host records \
                 the EventLedger decision and Locus ownership first.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "Promotion denies durably when the current EventLedger aggregate/version is \
                 stale or missing (`AggregateVersionMismatch`), the schema id does not match \
                 the current ModelLane registry row (`SchemaMismatch`), the CRDT base snapshot \
                 is stale (`StaleBase`), the CRDT state vector is stale \
                 (`StaleStateVector`), input refs are missing, cross-run, non-advisory, or lack \
                 selected CRDT state (`InputRefMismatch`), the request reports a direct authority \
                 mutation attempt (`DirectAuthorityMutation`), no validator/operator authority is \
                 present (`MissingPromotionAuthority`), or an otherwise approvable decision lacks \
                 a promoted artifact binding (`MissingPromotedArtifactBinding`). \
                 Same `idempotency_key` plus changed canonical content returns an explicit \
                 idempotency conflict instead of appending another decision.",
            ),
            section(
                "recovery",
                "Recovery and diagnostics",
                "Recover by replaying PostgreSQL rows through \
                 `ModelLaneStore::replay_promotion_decisions(run_id)` ordered by \
                 `event_ledger_seq`, then compare each row to its `kernel_event_ledger` receipt. \
                 Inspect `canonical_hash_basis`, `canonical_decision_hash`, \
                 `current_event_ledger_version`, `current_schema_id`, `denial_reason`, \
                 `state_history`, `final_state`, `promotion_gate_ref`, \
                 `promotion_receipt_ref`, `promotion_decision_id`, \
                 `promoted_artifact_ref`, `promoted_artifact_sha256`, \
                 `promoted_artifact_version`, and message routing fields. \
                 HBR-INT-009 posture: Flight Recorder/EventLedger is WIRED through \
                  `dexterity_model_lane` rows; internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer and must observe promotion rows without becoming \
                 authority.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-004 proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests model_lane_promotion_appends_eventledger_and_replays_decision -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests model_lane_promotion_reordered_inputs_keep_same_decision_hash -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_promotion_user_manual_entry_is_current -- --exact`. \
                 These exercise real PostgreSQL, EventLedger append/replay, schema registry \
                 rows, DB-derived CRDT base/state-vector guards, exact promotion decision and \
                 artifact binding, phantom input-ref denial, direct authority mutation rejection, \
                 duplicate idempotency conflict, deterministic sorted refs, typed message routing, \
                 and manual parity. \
                 There is no SQLite, mock, app/src, app/src-tauri, TypeScript, or structs-only \
                 proof path for Dexterity promotion.",
            ),
        ],
        anchors: vec![
            spec_anchor("4.3.9.2.5"),
            page_link("model-lane-schema"),
            page_link("model-lane-launch-adapters"),
            page_link("model-lane-context-bundle-handoff"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "model_lane_promotion_pg_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_model_lane_context_bundle_handoff() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-context-bundle-handoff".into(),
        title: "Dexterity ContextBundle Handoffs".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into()],
        sections: vec![
            section(
                "purpose",
                "What ContextBundle handoffs do",
                "Dexterity uses artifact-backed `ModelLaneContextBundleHandoff` rows to move \
                 model output between local, cloud, CLI, human, subagent, and validator lanes. \
                 A downstream lane receives only replayable artifact refs and typed context \
                 entries; hidden provider memory, chat history, and prompt-only state are never \
                 authority. The stable schemas are \
                 `hsk.model_lane_context_bundle_artifact@1` and \
                 `hsk.model_lane_context_bundle_handoff@1`, the EventLedger aggregate types are \
                 `model_lane_context_bundle_artifact` and \
                 `model_lane_context_bundle_handoff`, and the Rust APIs are \
                 `ModelLaneStore::record_context_bundle_artifact_binding`, \
                 `ModelLaneStore::record_context_bundle_handoff`, \
                 `ModelLaneStore::replay_context_bundle_handoffs`, \
                 `ModelLaneStore::consume_context_bundle_for_downstream`, \
                 `SwarmCoordinator::context_bundle_for_downstream_lane`, \
                 `SwarmCoordinator::invoke_downstream_context_bundle`, and \
                 `model_lane_context_bundle_id_for_handoff`.",
            ),
            section_with_json(
                "schema",
                "Handoff fields",
                "Each artifact binding stores `artifact_binding_id`, run/trace span data, \
                 `artifact_ref`, `artifact_sha256`, required `content_hash`, `artifact_kind`, \
                 `artifact_manifest_ref`, `artifact_payload_ref`, canonical `payload_json`, \
                 EventLedger stream refs, Locus WP/MT/task-board ownership, idempotency key, \
                 `artifact_binding_hash`, and `record_json` in \
                 `model_lane_context_bundle_artifacts`. Each handoff stores `handoff_id`, \
                 deterministic `context_bundle_id`, required `downstream_lane_id`, \
                 `source_lane_id`, `source_message_id`, `artifact_ref`, `artifact_sha256`, \
                 required `content_hash`, `source_kind`, `authority_state`, `selection_state`, \
                 `reason_code`, optional `decision_ref` and `reviewer_ref`, `replay_hint`, \
                 optional `crdt_payload`, `loom_refs`, `memory_pack_refs`, EventLedger stream \
                 refs, required `work_packet_id`, `micro_task_id`, `task_board_id`, \
                 idempotency key, replay order key, and diagnostic payload. Selection state is one of \
                 `selected`, `rejected`, `unresolved`, or `superseded`. The row-level \
                 `context_bundle_hash` covers the replayable handoff payload while the \
                 `context_bundle_id` groups multiple handoffs for one downstream replay.",
                json!({
                    "kernel": "Dexterity",
                    "artifact_schema_id": "hsk.model_lane_context_bundle_artifact@1",
                    "schema_id": "hsk.model_lane_context_bundle_handoff@1",
                    "artifact_table": "model_lane_context_bundle_artifacts",
                    "aggregate_type": "model_lane_context_bundle_handoff",
                    "selection_states": [
                        "selected",
                        "rejected",
                        "unresolved",
                        "superseded"
                    ],
                    "handoff_types": [
                        "ModelLaneContextBundleArtifactBindingRecord",
                        "NewModelLaneContextBundleArtifactBinding",
                        "ModelLaneContextBundleHandoffRecord",
                        "NewModelLaneContextBundleHandoff",
                        "ModelLaneDownstreamContextBundle",
                        "ModelLaneCrdtHandoffMetadata",
                        "ModelLaneLoomHandoffRef",
                        "ModelLaneMemoryPackHandoffRef"
                    ],
                    "artifact_api": "ModelLaneStore::record_context_bundle_artifact_binding",
                    "replay_api": "ModelLaneStore::replay_context_bundle_handoffs(run_id, context_bundle_id)",
                    "downstream_api": "ModelLaneStore::consume_context_bundle_for_downstream(run_id, context_bundle_id, downstream_lane_id)",
                    "coordinator_api": "SwarmCoordinator::context_bundle_for_downstream_lane(run_id, context_bundle_id, downstream_lane_id)",
                    "adapter_invocation_api": "SwarmCoordinator::invoke_downstream_context_bundle(run_id, context_bundle_id, downstream_lane_id, adapter, actor)",
                    "kernel_conversion": "ModelLaneDownstreamContextBundle::to_kernel_context_bundle"
                }),
            ),
            section(
                "workflows",
                "Model-to-model handoff workflow",
                "Record source `ModelLaneMessage` rows first, then record one \
                 `NewModelLaneContextBundleArtifactBinding` for the payload with \
                 `record_context_bundle_artifact_binding`. The binding requires canonical \
                 `payload_json` whose sha256 equals `artifact_sha256` and `content_hash`, \
                 writes `model_lane_context_bundle_artifacts`, and appends \
                 `ARTIFACT_STORED` to EventLedger. Build a `NewModelLaneContextBundleHandoff` \
                 for every output the downstream lane may see, derive the shared \
                 `context_bundle_id` with `model_lane_context_bundle_id_for_handoff`, then call \
                 `record_context_bundle_handoff`. Dexterity resolves `source_message_id` and \
                 the artifact binding from PostgreSQL inside the transaction, checks same-run \
                 and source-lane parity, requires `artifact_ref`, `artifact_sha256`, and \
                 `content_hash` to match both the source message and artifact binding, appends \
                 `CONTEXT_BUNDLE_RECORDED` to EventLedger, stamps the final EventLedger id/seq \
                 into the row payload, and stores the replay row. A downstream lane uses \
                 `ModelLaneStore::consume_context_bundle_for_downstream` or \
                 `SwarmCoordinator::context_bundle_for_downstream_lane` to resolve only \
                 handoffs addressed to its lane in `event_ledger_seq` order and can convert the \
                 returned `ModelLaneDownstreamContextBundle` with `to_kernel_context_bundle`. \
                 `to_kernel_context_bundle` preserves the downstream context hash and derives the \
                 kernel `CTX-<hash>` id required by ContextBundle V1. \
                 `SwarmCoordinator::invoke_downstream_context_bundle` then passes that kernel \
                 `ContextBundle` to the adapter boundary through `ModelAdapterRequest`.",
            ),
            section(
                "workflows",
                "CRDT, Loom, and FEMS rules",
                "CRDT handoffs use `ModelLaneCrdtHandoffMetadata` with \
                 `schema_id = hsk.model_lane_crdt_payload@1`, `document_id`, `workspace_id`, \
                 `actor_id`, `actor_kind`, `lane_id`, `crdt_site_id`, positive `update_seq`, \
                 Yjs-compatible format `yjs_update_v1` only, \
                 `update_bytes_ref`, `update_sha256`, `state_vector`, \
                 `base_snapshot_ref`, `materialized_projection_hash`, object \
                 `replay_metadata`, `promotion_gate_ref`, optional `promotion_receipt_ref`, \
                 `validation_runner_ref`, and `authority_effect = advisory_only`. Canonical \
                 CRDT authority is the append-only `kernel_crdt_snapshots` and \
                 `kernel_crdt_updates` rows joined to their identity-, payload-, and hash-checked \
                 `kernel_event_ledger` events. Validation locks and replays the cited snapshot \
                 followed by the contiguous, dependency-resolved update chain, derives actor/site \
                 attribution, the state vector, and the Yjs v1 materialization, and verifies \
                 `materialized_projection_hash`. CRDT-bearing source-message admission also resolves \
                 exactly one active `knowledge_crdt_agent_lane_leases` row. Its lane, actor, actor \
                 kind, session, and `correlation_id` must match the resolved update trace; its scope \
                 must be either `workspace:<workspace_id>` or \
                 `document:<crdt_document_id>`; and the database admission timestamp must satisfy \
                 `claimed_at_utc <= lease_admitted_at_utc < expires_at_utc` while \
                 `released_at_utc IS NULL`. Zero matches and multiple covering workspace/document \
                 matches both fail closed. The server-derived `ModelLaneCrdtAuthorityBinding` stores \
                 `lease_id`, `lease_correlation_id`, `lease_scope_kind`, `lease_scope_id`, \
                 `lease_claimed_at_utc`, `lease_expires_at_utc`, and \
                 `lease_admitted_at_utc` in both the message projection and its immutable \
                 `MODEL_RESPONSE_RECORDED` EventLedger payload. The existing `replay_metadata` object is \
                 authoritative: its `replay_order_key`, `dependency_update_ids`, and \
                 `schema_version` must exactly match the persisted PostgreSQL update replay \
                 metadata. `promotion_gate_ref` must equal \
                 `promotion-gate://model-lane-message/<source_message_id>`, \
                 `validation_runner_ref` must equal `eventledger://<update_event_id>`, and \
                 `promotion_receipt_ref` remains null while `authority_effect = advisory_only`. If the source \
                 message carries CRDT refs, the handoff must carry CRDT metadata and \
                 `update_bytes_ref` must match the source `crdt_update_ref`. Loom handoffs use \
                 `ModelLaneLoomHandoffRef` with workspace/block ids, optional source/target block \
                 ids, optional materialized artifact ref, content hash/version, \
                 `event_ledger_evidence_ref` beginning with `eventledger://`, and \
                 `flight_recorder_evidence_ref` beginning with `flight-recorder://`; \
                 loom_refs exceeds bounded limit at 64 refs. FEMS context uses explicit \
                 `ModelLaneMemoryPackHandoffRef` rows with `memory_pack_ref`, \
                 `memory_pack_hash`, `scope_tag`, `review_status`, `cloud_safe`, \
                 `classification`, optional `projection_ref`, and `evidence_ref`; \
                 `review_status must be reviewed`, operator_reviewed, or validator_reviewed, \
                 `classification` must be cloud_safe_context, local_only_context, or \
                 operator_reviewed_context, `evidence_ref` must begin with `eventledger://` or \
                 `flight-recorder://`, and `memory_pack_refs exceeds bounded FEMS limit` at 16 \
                 refs. Cloud lanes reject missing, non-cloud-safe, or local_only_context memory \
                 packs. Hidden provider/session memory checks trim and normalize URI case, and \
                 apply to both `memory_pack_ref` and `projection_ref`.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "ContextBundle handoff writes fail closed when `context_bundle_id` does not match \
                 the deterministic shared context, `source_message_id` is missing or not \
                 replayable, source run/lane/kind/authority differs, `downstream_lane_id`, \
                 `work_packet_id`, `micro_task_id`, or `task_board_id` is missing, \
                 `artifact_ref`, `artifact_sha256`, or `content_hash` differs from the source \
                 message or `ArtifactStore/EventLedger authority`, canonical `payload_json` \
                 does not hash to the artifact content hash, a cloud downstream receives no \
                 `MemoryPack` refs, a `cloud_safe = false` ref, or local_only_context memory, \
                 hidden provider/session memory is supplied through `memory_pack_ref` or \
                 `projection_ref`, `memory_pack_refs exceeds bounded FEMS limit`, \
                 `loom_refs exceeds bounded limit`, `review_status` is not reviewed, \
                 operator_reviewed, or validator_reviewed, CRDT source messages lack \
                 `crdt_payload`, `update_bytes_ref` does not match source `crdt_update_ref`, \
                 `replay_metadata` does not declare Yjs v1 or its `replay_order_key`, \
                 `dependency_update_ids`, or `schema_version` differs from the persisted update, \
                 the snapshot/update rows or their \
                 EventLedger identity/payload/hash evidence disagree, replay is non-contiguous or \
                 has an unresolved dependency, actor/site/vector/materialization derivation fails, \
                 `materialized_projection_hash` differs, run/lane/session/trace ownership crosses, \
                 no exact active lease exists at admission, the lease is released or expired, its \
                 correlation or scope differs, or both workspace and document leases ambiguously \
                 cover the same message, \
                 `promotion_gate_ref` or `validation_runner_ref` differs from its exact derived \
                 value, `authority_effect` is not `advisory_only`, or \
                 `promotion_receipt_ref` is non-null while advisory, Loom evidence refs are missing or use non-EventLedger / \
                 non-Flight Recorder prefixes, or idempotency is reused with a different \
                 `context_bundle_hash` or `artifact_binding_hash`.",
            ),
            section(
                "recovery",
                "Recovery and diagnostics",
                "Recover by querying `model_lane_context_bundle_artifacts` for the \
                 `artifact_ref`/hash binding, then query `model_lane_context_bundle_handoffs` \
                 through `ModelLaneStore::replay_context_bundle_handoffs(run_id, \
                 context_bundle_id)` or the downstream-only \
                 `ModelLaneStore::consume_context_bundle_for_downstream`. Creation, idempotent \
                 retry, downstream consume, replay, promotion, and recovery all fail closed through \
                 the same stored-authority validators. A CRDT-bearing message projection must equal \
                 its exact `MODEL_RESPONSE_RECORDED` EventLedger record and full \
                 `crdt_authority_binding`; a ContextBundle projection must recompute its canonical \
                 `context_bundle_hash` and equal its exact `CONTEXT_BUNDLE_RECORDED` EventLedger \
                 record. Old rows with a missing binding are rejected rather than trusted. Inspect \
                 `artifact_manifest_ref`, `artifact_payload_ref`, `payload_json`, \
                 `artifact_binding_hash`, `selection_state`, `source_message_id`, \
                 `downstream_lane_id`, `artifact_ref`, `artifact_sha256`, `content_hash`, \
                 `context_bundle_hash`, `event_ledger_event_id`, `event_ledger_seq`, \
                 `work_packet_id`, `micro_task_id`, `task_board_id`, \
                 `crdt_payload.state_vector`, `crdt_payload.base_snapshot_ref`, \
                 `crdt_payload.materialized_projection_hash`, `crdt_payload.replay_metadata`, \
                 `crdt_payload.promotion_gate_ref`, `crdt_payload.validation_runner_ref`, and the \
                 source message's full `crdt_authority_binding` across run, lane, lane/model/CRDT \
                 sessions, lane/CRDT traces, workspace/document/CRDT document, actor/kind/site, \
                 lease id/correlation/scope/claimed/expiry/admission evidence, \
                 update id/sequence/bytes ref, snapshot ref, vector, projection hash, proposal ref, \
                 and update EventLedger event. Lease claim, renew, release, expiry sweep, takeover, \
                 and ModelLane admission share one PostgreSQL transaction advisory-lock domain. \
                 Locks are ordered deterministically by `workspace:<workspace_id>` then \
                 `crdt_document:<crdt_document_id>`, so release, sweep, and a second covering claim \
                 cannot appear as phantoms between admission and `MODEL_RESPONSE_RECORDED`. Re-run \
                 the locked snapshot-to-update replay and \
                 compare the binding, `replay_metadata` order/dependencies/schema version, derived \
                 vector, materialized hash, and EventLedger evidence; \
                 historical replay resolves the exact persisted `lease_id` and proves that the \
                 lease covered the lane/update at `lease_admitted_at_utc`. A later release or natural \
                 expiry is valid and must not invalidate the immutable admission receipt; a release \
                 at or before admission, shortened expiry, changed identity/scope/correlation, missing \
                 lease row, changed binding, mutable projection/EventLedger disagreement, or \
                 recomputed ContextBundle hash disagreement fails closed. \
                 Do not rewrite the append-only CRDT update or snapshot rows. Also inspect \
                 `loom_refs` and `memory_pack_refs`. \
                 HBR-INT-009 posture: EventLedger is WIRED through `ARTIFACT_STORED` and \
                 `CONTEXT_BUNDLE_RECORDED` rows. Flight Recorder/EventLedger recovery for MT-005 \
                 uses those EventLedger rows plus `flight_recorder_evidence_ref` fields; direct \
                 Flight Recorder event emission is DEFERRED-with-reason until the MT-008 \
                  diagnostics surface. internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer and must observe handoff rows without becoming authority.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-005 proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests model_lane_context_bundle_persists_selection_state_and_replays -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests model_lane_context_bundle_missing_artifact_ref_fails_closed -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_context_bundle_user_manual_entry_is_current -- --exact`. \
                 These exercise real PostgreSQL, EventLedger append/replay, schema registry rows, \
                 selected/rejected/unresolved/superseded replay, artifact binding authority, \
                 downstream-only consumption, coordinator adapter invocation, kernel \
                 ContextBundle CTX-hash conversion, fail-closed \
                 artifact mismatch, missing replay source, cloud-safe FEMS MemoryPack \
                 enforcement, local_only_context cloud rejection, hidden projection_ref \
                 rejection, normalized hidden-memory URI rejection, bounded Loom/FEMS refs, CRDT \
                 state-vector and Yjs update ref validation, exact active lease admission, \
                 released/expired/correlation/scope/ambiguity denial probes, historical replay \
                 after lease release, Loom evidence refs, Flight Recorder \
                 refs, and manual parity. There is no SQLite, mock, \
                 app/src, app/src-tauri, TypeScript, \
                 prompt-only, or hidden-memory proof path for Dexterity ContextBundle handoffs.",
            ),
        ],
        anchors: vec![
            spec_anchor("4.3.9.2.5"),
            page_link("model-lane-schema"),
            page_link("model-lane-promotion"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "model_lane_context_bundle_pg_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_model_lane_cloud_projection_consent() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-cloud-projection-consent".into(),
        title: "Dexterity Cloud Projection and Consent".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into(), "5.8".into(), "6.13".into()],
        sections: vec![
            section(
                "purpose",
                "What this cloud boundary does",
                "Dexterity cloud lanes use durable PostgreSQL/EventLedger authority before any \
                 BYOK provider call. A cloud launch must resolve \
                 `ModelLaneCloudProjectionPlanRecord` and \
                 `ModelLaneCloudConsentReceiptRecord` rows through \
                 `ModelLaneStore::record_cloud_projection_plan`, \
                 `ModelLaneStore::record_cloud_consent_receipt`, and \
                 `ModelLaneStore::preflight_cloud_spawn_request`. String refs alone are not \
                 authority. The cloud provider is never allowed to become the source of \
                 truth; cloud output stays `ModelLaneAuthority::Advisory` until an approved \
                 PromotionGate decision creates authority.",
            ),
            section_with_json(
                "schema",
                "Durable records",
                "The stable machine schemas are `hsk.model_lane_cloud_projection_plan@2` and \
                 `hsk.model_lane_cloud_consent_receipt@2`. PostgreSQL authority tables are \
                 `model_lane_cloud_projection_plans` and \
                 `model_lane_cloud_consent_receipts`. `single_lane` authority binds `run_id`, \
                 `lane_id`, `model_session_id`, `provider_kind`, and `requested_model_id` \
                 exactly. `single_run` authority drops those lane identity bindings and \
                 authorizes only launches whose `run_id` matches; revocation cancels every \
                 durable lane in that run that references the receipt. Both scopes also bind `scope_hash`, \
                 source artifact refs, payload artifact refs, hashes, retention/export \
                 posture, fan-out targets, EventLedger event id/seq, `user_manual_behavior_ref`, \
                 and Locus fields. Replay uses \
                 `ModelLaneStore::replay_cloud_consent_authority(run_id)` and \
                 `ModelLaneStore::replay_run(run_id)`.",
                json!({
                    "projection_plan": {
                        "table": "model_lane_cloud_projection_plans",
                        "schema_id": "hsk.model_lane_cloud_projection_plan@2",
                        "record": "ModelLaneCloudProjectionPlanRecord",
                        "input": "NewModelLaneCloudProjectionPlan",
                        "event_type": "ARTIFACT_STORED",
                        "aggregate_type": "model_lane_cloud_projection_plan"
                    },
                    "consent_receipt": {
                        "table": "model_lane_cloud_consent_receipts",
                        "schema_id": "hsk.model_lane_cloud_consent_receipt@2",
                        "record": "ModelLaneCloudConsentReceiptRecord",
                        "input": "NewModelLaneCloudConsentReceipt",
                        "event_type": "ARTIFACT_STORED",
                        "aggregate_type": "model_lane_cloud_consent_receipt"
                    },
                    "denial": {
                        "schema_id": "hsk.model_lane_cloud_consent_denial@1",
                        "table": "kernel_event_ledger",
                        "reason_code": "CX-MM-007",
                        "aggregate_type": "model_lane_cloud_consent_denial",
                        "provider_call_attempted": false,
                        "partial_authority_state_created": false
                    }
                }),
            ),
            section(
                "workflows",
                "Launch workflow",
                "Use durable ArtifactStore/ContextBundle refs for redacted cloud payload \
                 authority. When the run already exists, create or replay the redacted cloud \
                 payload through ArtifactStore/ContextBundle first, then record a ProjectionPlan, \
                 record a matching ConsentReceipt, attach their refs to the cloud lane, and call \
                 `SwarmCoordinator::spawn_session`. operator-chat cloud launches precompute \
                 deterministic ArtifactStore refs (`cloud-input.json` and \
                 `cloud-projection-payload.json`) in the ProjectionPlan before cloud consent \
                 preflight; after `spawn_session` records the ModelLaneRun, \
                 `OperatorChatLaunchService` records both refs with \
                 `ModelLaneStore::record_context_bundle_artifact_binding` before output capture. \
                 If that post-run binding fails, the spawned session is cancelled instead of \
                 continuing with partial cloud authority. The \
                 coordinator invokes `ModelLaneStore::preflight_cloud_spawn_request` before \
                 `factory.create`, so missing, expired, mismatched, or revoked consent returns \
                 `CX-MM-007` with EventLedger evidence and no provider call. Consent binding \
                 checks for `single_lane` cover `projection_plan_hash`, `run_id`, `lane_id`, \
                 `model_session_id`, `provider_kind`, `requested_model_id`, `scope_hash`, retention \
                 policy, export posture, and fan-out targets. Checks for `single_run` cover \
                 `projection_plan_hash`, `run_id`, `scope_hash`, retention policy, export posture, \
                 and fan-out targets, and reject lane-bound identity fields. Operator Chat exposes \
                 the governed launch route `/operator-chat/cloud/single-run/grant-launch` and the \
                 matching revocation route `/operator-chat/cloud/single-run/revoke`.",
            ),
            section(
                "failure_modes",
                "Failure and recovery",
                "Failures are durable and typed: missing ProjectionPlan, missing ConsentReceipt, \
                 expired validity window, mismatched projection hash, provider mismatch, model \
                 session mismatch, scope mismatch, retention/export/fan-out mismatch, revoked \
                 ConsentReceipt, hidden provider/session memory refs, and attempted promoted \
                 cloud output without PromotionGate approval all fail closed. Denials append \
                 `model_lane_cloud_consent_denial` EventLedger rows with `CX-MM-007`, \
                 `consent_status`, `provider_call_attempted = false`, and \
                 `user_manual_behavior_ref`. Use \
                 `ModelLaneStore::revoke_cloud_consent_receipt` to revoke a receipt; it cancels \
                 covered non-terminal lanes as `ModelLaneStatus::Cancelled`, sets \
                 `failstate_code = CX-MM-007`, writes a `model_lane_terminal` EventLedger row, \
                 and keeps the lane replayable.",
            ),
            section(
                "recovery",
                "Flight Recorder and Palmistry posture",
                "EventLedger is WIRED through `kernel_event_ledger` rows for projection, \
                 consent, denial, advisory cloud output, and revocation terminal state. Direct \
                 Flight Recorder event emission is DEFERRED-with-reason until the FR-EVT-CLOUD \
                  emitter is wired to these EventLedger rows. internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer; it joins by `run_id`, `lane_id`, \
                 `model_session_id`, and EventLedger refs without becoming launch authority.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-006 proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests cloud_projection_and_consent_receipts_persist_and_replay -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests cloud_lane_rejects_missing_expired_mismatched_and_revoked_consent -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests cloud_consent_revocation_cancels_pending_lanes_with_eventledger_evidence -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests cloud_model_lane_policy_user_manual_entry_is_current -- --exact`. \
                 Use `-j 1` locally if Windows linker fan-out leaves stale cargo/rustc/link \
                 workers during test development. Passing tests must use real PostgreSQL plus \
                 EventLedger and must not rely on SQLite, prompt-only state, synthetic refs, \
                 or frontend/Tauri launch authority.",
            ),
        ],
        anchors: vec![
            page_link("model-lane-launch-adapters"),
            page_link("model-lane-context-bundle-handoff"),
            page_link("model-lane-promotion"),
            route_anchor("POST", "/operator-chat/cloud/single-run/grant-launch"),
            route_anchor("POST", "/operator-chat/cloud/single-run/revoke"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "cloud_model_lane_policy_pg_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_model_lane_recovery() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-recovery".into(),
        title: "Dexterity Recovery and Replay".into(),
        page_kind: "state_recovery",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into(), "5.8".into(), "6.13".into()],
        sections: vec![
            section(
                "purpose",
                "What recovery reconstructs",
                "Dexterity recovery reconstructs ModelLaneRun, ModelLane, ModelLaneMessage, \
                 ArtifactStore payload refs, lane leases, diagnostic posture, and MT runtime \
                 status from PostgreSQL plus kernel_event_ledger. It must not depend on chat \
                 history, terminal scrollback, UI rows, provider traces, or prompt-only state. \
                 Models keep proposing; Handshake performs deterministic checkpoint, replay, \
                 validation, and typed failure recording.",
            ),
            section_with_json(
                "schema",
                "Checkpoint and event records",
                "Recovery checkpoints use `hsk.model_lane_recovery_checkpoint@1` in \
                 `model_lane_recovery_checkpoints`. Each checkpoint carries `run_id`, \
                 `lane_id`, `session_id`, `model_session_id`, lane `status`, \
                 `last_event_ledger_seq`, `last_message_id`, open payload refs, `lease_id`, \
                 `idempotency_scope`, and `recovery_state`. Recovery events use \
                 `hsk.model_lane_recovery_event@1` in `model_lane_recovery_events` and map \
                 canonical event families such as RUN_CREATED, LANE_STARTED, \
                 MESSAGE_RECORDED, PAYLOAD_REF_MISSING, RECOVERY_REQUESTED, \
                 REPLAY_RECONSTRUCTED, RECOVERY_FAILED, and ORPHAN_DETECTED to \
                 EventLedger rows with trace/span/session/model-session metadata.",
                json!({
                    "checkpoint_schema": "hsk.model_lane_recovery_checkpoint@1",
                    "event_schema": "hsk.model_lane_recovery_event@1",
                    "lease_schema": "hsk.model_lane_lease@1",
                    "diagnostic_schema": "hsk.model_lane_diagnostic_tier@1",
                    "mt_status_schema": "hsk.model_lane_mt_runtime_status@1",
                    "tables": [
                        "model_lane_recovery_checkpoints",
                        "model_lane_recovery_events",
                        "model_lane_leases",
                        "model_lane_diagnostic_tier_statuses",
                        "model_lane_mt_runtime_statuses",
                        "model_lane_context_bundle_artifacts",
                        "kernel_event_ledger"
                    ],
                    "failure_codes": ["CX-MM-006", "CX-MM-009", "CX-MM-012", "CX-MM-013", "CX-MM-014"]
                }),
            ),
            section(
                "workflows",
                "Recovery workflow",
                "Call `ModelLaneStore::recover_run_after_restart(run_id)`. Recovery loads the \
                 latest checkpoint, validates the checkpoint EventLedger high-watermark, replays \
                 recovery events up to that high-watermark in `replay_order_seq`, rejects \
                 divergent duplicate idempotency, resolves payload refs through \
                 `model_lane_context_bundle_artifacts` plus EventLedger, verifies CRDT \
                 `base_snapshot_ref` and `state_vector` against recorded ModelLaneMessage rows, \
                 reconstructs run/lane/message state from checkpoint-bounded EventLedger \
                 payloads, includes checkpoint-bounded failed cloud consent denial receipts, and \
                 restores checkpoint/forward-bound MT runtime status refs. Lane leases are not \
                 replay adjunct state: recovery separately reads the latest committed EventLedger \
                 authority for every lease in the run, so a lease acquired after the checkpoint is \
                 surfaced as active or reclaimed as expired without widening the deterministic \
                 replay watermark or injecting a post-checkpoint lane into `ModelLaneReplay`.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "Missing/corrupt payload refs return `CX-MM-006` with recovery hints. \
                 Corrupt checkpoints, missing checkpoint high-watermarks, EventLedger sequence \
                 gaps, missing source EventLedger rows, divergent duplicate idempotency keys, \
                 stale CRDT bases, and expired active lease orphans fail closed or record typed \
                 recovery status. Orphan recovery records durable CX-MM-009 `orphan_detected` \
                 recovery events for expired leases found through current lease authority before \
                 takeover or denial. A post-checkpoint lease is therefore never hidden merely \
                 because replay remains checkpoint-bounded.",
            ),
            section(
                "navigation",
                "Diagnostics and operators",
                "HBR-INT-009 is represented by `ModelLaneDiagnosticTierStatusRecord` with \
                 `behavior_id`, tier, state, reason, `follow_up_ref`, and `evidence_ref`. \
                  Flight Recorder/EventLedger evidence alone must fail. `internal_diagnostics` is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer. Operator-facing recovery \
                 should inspect this page, \
                 `model_lane_recovery_pg_tests`, and the native diagnostic surface from MT-008.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-007 proof commands: \
                `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_replays_from_postgres_eventledger_checkpoint -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_includes_current_leases_but_bounds_replay_adjunct_state -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_rejects_corrupt_checkpoint_and_event_seq_gap -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_restores_mt_runtime_status_refs_after_restart -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests diagnostic_tier_record_rejects_flight_recorder_only_evidence -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_rejects_missing_payload_stale_crdt_and_duplicate_idempotency -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_uses_eventledger_checkpoint_authority_over_mutable_row -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_rejects_post_checkpoint_payload_and_crdt_repairs -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_recovery_user_manual_entry_is_current -- --exact`.",
            ),
        ],
        anchors: vec![
            page_link("model-lane-schema"),
            page_link("model-lane-context-bundle-handoff"),
            page_link("model-lane-cloud-projection-consent"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "model_lane_recovery_pg_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_model_lane_diagnostics() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-diagnostics".into(),
        title: "Dexterity Lane Diagnostics".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into(), "5.8".into(), "6.13".into()],
        sections: vec![
            section(
                "purpose",
                "What the diagnostics pane proves",
                "Dexterity Lane Diagnostics is the native Rust operator/model surface for \
                 inspecting live and recovered ModelLaneRun state. It reads \
                 `ModelLaneStore::diagnostics_projection(run_id)` through PostgreSQL plus \
                 kernel_event_ledger, never from chat history, terminal scrollback, \
                 provider state, Tauri/WebView authority, React state, or prompt-only \
                 diagnostics. Models propose lane work; Handshake records, projects, \
                 filters, and drills into deterministic state.",
            ),
            section_with_json(
                "schema",
                "Projection contract",
                "The native pane consumes `native_swarm_lane_diagnostics`. The projection \
                 includes run identity, lane status and message counts, payload errors, \
                 orphan and reclaimable lease state, message payload refs, promotion state, \
                 trace/span/link IDs, EventLedger event IDs, FlightRecorder correlation \
                 IDs, HBR-INT-009 diagnostic tiers, Locus/Loom/FEMS refs, ContextBundle \
                 refs, memory pack refs and hashes, ArtifactStore refs, and CRDT \
                 base/state-vector refs.",
                json!({
                    "surface_contract_id": "native_swarm_lane_diagnostics",
                    "backend_methods": [
                        "ModelLaneStore::diagnostics_projection",
                        "ModelLaneStore::latest_diagnostics_projection"
                    ],
                    "http_routes": [
                        "GET /swarm/model-lanes/diagnostics/latest",
                        "GET /swarm/model-lanes/diagnostics/{run_id}"
                    ],
                    "native_author_ids": [
                        "swarm-lane-diagnostics.surface",
                        "swarm-lane-diagnostics.filter.run",
                        "swarm-lane-diagnostics.filter.lane",
                        "swarm-lane-diagnostics.filter.message",
                        "swarm-lane-diagnostics.action.refresh",
                        "menu.models.swarm-lane-diagnostics",
                        "settings.swarm-lane-diagnostics-default-open"
                    ],
                    "required_tiers": ["flight_recorder", "internal_diagnostics", "palmistry"],
                    "authority_tables": [
                        "model_lane_runs",
                        "model_lanes",
                        "model_lane_messages",
                        "model_lane_leases",
                        "model_lane_diagnostic_tier_statuses",
                        "model_lane_mt_runtime_statuses",
                        "kernel_event_ledger"
                    ]
                }),
            ),
            section(
                "workflows",
                "Operator and model workflow",
                "Open the pane from `MODELS > Open Lane Diagnostics` or from the command \
                 palette action `swarmdiagnostics.open`. The settings checkbox \
                 `settings.swarm-lane-diagnostics-default-open` persists whether Lane \
                 Diagnostics is included with Swarm defaults. In the pane, use the run, \
                 lane, and message filters to narrow a run; use payload and promotion \
                 drilldowns to inspect message payload authority, EventLedger linkage, \
                 FlightRecorder correlation, CRDT base/state-vector refs, Locus/Loom/FEMS \
                 refs, and PromotionGate state. In this diagnostics path, \
                 `flight_recorder_correlation_id` is an EventLedger-backed alias: the \
                 EventLedger event ID is the durable FlightRecorder correlation until a \
                 distinct FlightRecorder row is emitted for the same lane/message.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "The native surface rejects projections with a wrong surface contract, \
                 empty run ID, lane/message count mismatch, missing stable lane author IDs, \
                 missing payload refs, missing EventLedger/FlightRecorder evidence, or \
                 missing HBR-INT-009 tiers. Backend projection failures surface as a pane \
                 error with author ID `swarm-lane-diagnostics.error`. Flight Recorder-only \
                 diagnostics are not enough. Tier-2 internal_diagnostics is WIRED for typed \
                 panic-latch, UI heartbeat, frame-time, resource, backend-route, GUI-action and \
                 mechanical-job events. The native panic hook durably writes a bounded, content-free \
                 crash record containing build/session/process identity, payload class only, a hashed \
                 source location, at most 128 hashed backtrace lines, and at most 32 recent typed \
                 mechanical events; raw panic payloads, source paths, stack text, prompts, keys, and \
                 project content are not fields in that record. Tier-3 Palmistry is WIRED as a quiet separate process \
                 with authenticated readiness, freeze/exit observation, durable survivor records, \
                 idempotent recovery into Flight Recorder, and a projection in the Problems pane.",
            ),
            section(
                "recovery",
                "Palmistry crash and freeze recovery",
                "Open the Problems pane from `RUN` then `Open Problems` \
                 (`menu.run.problems`) or from `Settings` then `Model Runtime` then `Open Problems` \
                 (`settings.model-runtime.open-problems`) to inspect the live internal_diagnostics heartbeat, frame \
                 and resource counters, recent typed events, and recovered Palmistry records. \
                 Palmistry writes only mechanical survivor metadata locally. Production startup \
                 requires a release SHA-256 pin compiled into the launcher; the adjacent sidecar \
                 and `HANDSHAKE_PALMISTRY_SHA256` override are accepted only by development builds. \
                 Unsigned ZIPs are development-only and are not production installers. The backend \
                 durably stores the watcher Ed25519 public verifier while the \
                 private key reaches only that watcher over a one-shot inherited pipe. Handshake \
                 processes a bounded survivor recovery page at startup and once per second, verifies \
                 their signatures, sends a sanitized summary through \
                 POST /internal-diagnostics/palmistry/recover, records a deterministic Diagnostic \
                 Flight Recorder event keyed by the survivor record UUID, then writes a durable \
                 local acknowledgement. A pending marker means the backend was unavailable or the \
                 durable import did not complete; restart Handshake after restoring the backend. \
                 Each recovered row exposes stable Problems-pane author IDs for hung-window probe, \
                 minidump status, and Flight Recorder import status. Minidumps remain local and are \
                 never part of the recovery payload.",
            ),
            section(
                "navigation",
                "Diagnostics and related pillars",
                "Dexterity Lane Diagnostics bridges the model-lane kernel to Argus, \
                 FlightRecorder/EventLedger, Locus, Loom, and FEMS. Argus observes the \
                 native AccessKit author IDs instead of a WebView DOM. FlightRecorder and \
                 EventLedger provide durable business-event evidence. An Argus click is applied only \
                 after the exact target handler acknowledges that action ID and a newer render revision \
                 is published; target removal or unrelated semantic drift is never success evidence. A \
                 non-secret SetValue requires its exact requested value in the newer target snapshot. \
                 The action result is independent from evidence durability: \
                 POST `/internal-diagnostics/argus/action-receipt` appends the sanitized EventLedger \
                 receipt and best-effort mirrors its reference into Flight Recorder, while append \
                 failure remains visible in `durability_error` and does not rewrite an observed UI \
                 result. Action values, errors, prompts, and content are excluded from the route's \
                 closed request shape. MT-008 uses the \
                 EventLedger event ID as the FlightRecorder correlation alias for the lane \
                 diagnostics projection. Locus/Loom/FEMS refs \
                 keep model messages connected to workspace locality, artifact libraries, \
                 and typed memory capsules. The surface is part of the Rust native frontend \
                 and Rust backend; it does not rely on React, TypeScript, Tauri command \
                 authority, npm tests, or WebView inspection.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-008 proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test swarm_lane_diagnostics_pg_tests swarm_lane_diagnostics_backend_projection_matches_eventledger -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test swarm_lane_diagnostics_pg_tests swarm_lane_diagnostics_rejects_flight_recorder_only_hbr_posture -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_swarm_lane_diagnostics_argus swarm_lane_diagnostics_argus_lists_filters_and_drills_down -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_swarm_lane_diagnostics_argus swarm_lane_diagnostics_argus_rejects_missing_author_id_and_count_mismatch -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_top_menu_bar run_menu_opens_swarm_lane_diagnostics -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_command_palette typing_diagnostics_filters_to_swarm_lane_diagnostics_and_runs -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_settings_dialog swarm_lane_diagnostics_setting_persists -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_diagnostics_user_manual_entry_is_current -- --exact`.",
            ),
        ],
        anchors: vec![
            page_link("model-lane-recovery"),
            page_link("model-lane-context-bundle-handoff"),
            page_link("usermanual-surface"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "swarm_lane_diagnostics_pg_tests".into(),
                http_method: "",
            },
        ],
    }
}

fn page_model_lane_navigation() -> NewUserManualPage {
    surface_page(
        "model-lane-navigation",
        "Dexterity ModelLane Backend Navigation",
        SurfaceGroup::ModelLaneNavigation,
        "Dexterity ModelLane navigation is the no-context lookup surface for model-lane \
         runtime state. It resolves runs, lanes, messages, artifact/context bundle rows, \
         traces/spans, diagnostic tiers, and recovery rows from PostgreSQL plus \
         kernel_event_ledger. The navigation projection is read-only: models propose edits, \
         Handshake validates and performs deterministic writes elsewhere, and this surface \
         shows the linked authority, Flight Recorder aliases, Locus/Loom/FEMS refs, \
         ContextBundle refs, MemoryPack refs, and UserManual recovery routes.",
        vec![
            section_with_json(
                "schema",
                "Projection contract",
                "Every route returns `hsk.model_lane_navigation@1` as \
                 `ModelLaneNavigationProjection`. The output includes `route_id`, \
                 `lookup_kind`, `lookup_ref`, input/output schema refs, manual refs, run, \
                 lane, message, artifact, context handoff, recovery checkpoint/event, lease, \
                 diagnostic tier, MT runtime status rows, EventLedger refs, Flight Recorder \
                 refs, error codes, and recovery routes. Lookup keys include `run_id`, \
                 `lane_id`, `message_id`, `model_session_id`, `session_id`, `wp_id`, \
                 `mt_id`, `task_board_id`, `artifact_ref`, `context_bundle_id`, Locus refs, Loom refs, FEMS \
                 MemoryPack refs, EventLedger event IDs/sequences, `trace_id`, `span_id`, \
                 and error codes carried by the recovered rows. Selectors that are not \
                 natural route path parameters use `GET /swarm/model-lanes/navigation/lookup` \
                 with exactly one query selector.",
                json!({
                    "schema_id": "hsk.model_lane_navigation@1",
                    "surface_contract_id": "native_swarm_lane_diagnostics",
                    "backend_methods": [
                        "ModelLaneStore::navigation_by_run",
                        "ModelLaneStore::navigation_by_lane",
                        "ModelLaneStore::navigation_by_message",
                        "ModelLaneStore::navigation_by_artifact_or_context",
                        "ModelLaneStore::navigation_by_trace",
                        "ModelLaneStore::navigation_by_diagnostics",
                        "ModelLaneStore::navigation_by_recovery",
                        "ModelLaneStore::navigation_by_lookup"
                    ],
                    "routes": [
                        "GET /swarm/model-lanes/navigation/runs/{run_id}",
                        "GET /swarm/model-lanes/navigation/lanes/{lane_id}",
                        "GET /swarm/model-lanes/navigation/messages/{message_id}",
                        "GET /swarm/model-lanes/navigation/artifacts",
                        "GET /swarm/model-lanes/navigation/traces/{trace_id}",
                        "GET /swarm/model-lanes/navigation/diagnostics/{run_id}",
                        "GET /swarm/model-lanes/navigation/recovery/{run_id}",
                        "GET /swarm/model-lanes/navigation/lookup"
                    ],
                    "authority": [
                        "PostgreSQL",
                        "kernel_event_ledger",
                        "model_lane_runs",
                        "model_lanes",
                        "model_lane_messages",
                        "model_lane_context_bundle_artifacts",
                        "model_lane_context_bundle_handoffs",
                        "model_lane_recovery_checkpoints",
                        "model_lane_recovery_events",
                        "model_lane_leases",
                        "model_lane_diagnostic_tier_statuses",
                        "model_lane_mt_runtime_statuses"
                    ]
                }),
            ),
            section(
                "workflows",
                "Lookup workflow",
                "Start with the narrowest stable id. Use `/runs/:run_id` for a full run, \
                 `/lanes/:lane_id` for a lane and its messages/recovery rows, \
                 `/messages/:message_id` for a payload-centered drilldown, `/artifacts` with \
                 `artifact_ref`, `artifact_binding_id`, `artifact_manifest_ref`, \
                 `artifact_payload_ref`, `artifact_sha256`, `content_hash`, or \
                 `context_bundle_id` for ContextBundle handoff recovery, \
                 `/traces/:trace_id?span_id=` for trace \
                 drilldown, `/diagnostics/:run_id?behavior_id=&tier=&mt_id=` for HBR/MT \
                 posture, `/recovery/:run_id` for checkpoint/event/lease recovery, and \
                 `/lookup` for `model_session_id`, `session_id`, `wp_id`, `mt_id`, \
                 `task_board_id`, `Locus`, `Loom`, `FEMS`, `MemoryPack`, EventLedger, \
                 trace/span, or error-code selectors. Use \
                 returned EventLedger refs and Flight Recorder refs before trusting UI rows or \
                 provider traces.",
            ),
            section(
                "failure_modes",
                "Failure modes",
                "Navigation fails closed when the id token is empty, the row is absent, the \
                 artifact/context query omits both an artifact selector and \
                 `context_bundle_id`, multiple distinct artifact selector values are supplied, \
                 a shared artifact hash/MemoryPack selector spans multiple runs without \
                 `run_id`, PostgreSQL is unavailable, or diagnostics detects mutable \
                  projection drift against EventLedger authority. Treat empty `event_ledger_refs` \
                  as a producing-lane defect. internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer. None of \
                 these gaps permits inference from chat history.",
            ),
            section(
                "recovery",
                "Recovery and related pillars",
                "For operator recovery, open `model-lane-recovery` and \
                 `model-lane-diagnostics`, then use this page's routes to find the exact row \
                 and EventLedger receipt. Locus links identify the work locality, Loom refs \
                 identify workspace artifacts, FEMS/MemoryPack refs identify bounded memory \
                 capsules, ContextBundle refs identify model-to-model handoff payloads, and \
                 Palmistry refs are observation evidence only; none of these replace \
                 PostgreSQL/EventLedger authority.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-010 proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_navigation_api_tests model_lane_navigation_routes_return_run_lane_message_artifact_trace_and_recovery -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_navigation_api_tests model_lane_navigation_user_manual_registry_rows_match_runtime_routes -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_navigation_user_manual_entries_are_current -- --exact`.",
            ),
        ],
        vec![
            page_link("model-lane-schema"),
            page_link("model-lane-diagnostics"),
            page_link("model-lane-recovery"),
            page_link("model-lane-validation-harness"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "model_lane_navigation_api_tests".into(),
                http_method: "",
            },
        ],
        vec![
            "4.3.9.2.5".into(),
            "5.8".into(),
            "6.13".into(),
            "10.15.8".into(),
        ],
    )
}

fn page_model_lane_validation_harness() -> NewUserManualPage {
    NewUserManualPage {
        slug: "model-lane-validation-harness".into(),
        title: "Dexterity Mixed-Lane Validation Harness".into(),
        page_kind: "surface_guide",
        audience: "model_and_operator",
        spec_anchors: vec!["4.3.9.2.5".into(), "5.8".into(), "6.13".into()],
        sections: vec![
            section(
                "purpose",
                "What the validation harness proves",
                "The mixed-lane validation harness proves that Dexterity can create a \
                 mixed local/cloud/subagent ModelLaneRun using deterministic provider fakes, \
                 persist it through PostgreSQL and kernel_event_ledger, replay and recover it \
                 after restart, inspect it through native_swarm_lane_diagnostics, and fail \
                 closed for direct endpoints, missing consent, stale CRDT base state, missing \
                 payload authority, and FlightRecorder-only diagnostic posture. The harness is \
                 explicit that a subagent lane is created through \
                 `SwarmCoordinator::launch_operator_subagent_model_lane`: it is a \
                 SubagentManager-owned no-OS lane and never invokes ModelSessionFactory. The \
                 coordinator returns an unforgeable `OperatorSubagentManagerLane` receipt; \
                 manager output enters only through \
                 `record_operator_subagent_manager_output`, which rechecks run/lane/owner and \
                 no-OS launch authority, then atomically commits the typed ModelLaneMessage and \
                 payload binding. A cancelled terminal lane rejects both rows. The \
                 harness is also the proof surface for mid-stream cancellation: coordinator-owned generation \
                 persists each newline-complete activity before polling the next chunk, so a \
                 captured prefix remains replayable; a cancelled terminal lane receipt is durable; \
                 and captures that reach the terminal gate cannot create later message, tool, \
                 artifact-binding, or Flight Recorder activity authority. A Flight Recorder event \
                 for a capture that was already durably accepted may be emitted after the terminal \
                 receipt as delayed diagnostic correlation, never as a standalone capture claim. \
                 CRDT update receipts are atomically committed with append-only PostgreSQL \
                 `kernel_crdt_updates` and EventLedger evidence; append-only snapshot rows retain \
                 explicit EventLedger linkage and promotion evidence. Canonical resolution locks \
                 the cited snapshot and contiguous dependency-resolved update chain, validates \
                 EventLedger identity/payload/hash parity, derives actor/site/vector state and the \
                 Yjs v1 materialization, and verifies `materialized_projection_hash`. Existing \
                 ContextBundle `replay_metadata.replay_order_key`, `dependency_update_ids`, and \
                 `schema_version` must exactly equal the canonical persisted update metadata; \
                 forged values fail closed before any handoff row is written. Each \
                 CRDT-bearing message persists a server-derived `crdt_authority_binding` across \
                 run/lane, lane/model/CRDT sessions, lane/CRDT traces, workspace/document/CRDT \
                 document, actor/kind/site, update id/sequence/bytes ref, snapshot ref, vector, \
                 materialized hash, proposal ref, and update EventLedger event; cross-run, \
                 cross-lane, cross-session, or cross-trace attribution is rejected. ContextBundle \
                 metadata accepts only `yjs_update_v1`; its `promotion_gate_ref` is exactly \
                 `promotion-gate://model-lane-message/<source_message_id>`, its \
                 `validation_runner_ref` is exactly `eventledger://<update_event_id>`, and its \
                 `promotion_receipt_ref` is null while `authority_effect = advisory_only`. \
                 Ordinary generated routing text is an advisory Proposal, not a CRDT \
                 mutation: `proposal_ref`, `crdt_update_ref`, `crdt_base_snapshot_ref`, \
                 `crdt_state_vector`, `crdt_proposal_ref`, and `crdt_stale_base_ref` remain null \
                 for all six routing policies. Non-null CRDT posture is accepted only as a \
                 complete set backed by canonical PostgreSQL Yjs v1 bytes, a verified update hash, \
                 and the post-update state vector; partial, missing, hash-mismatched, stale, or \
                 replay-reordered CRDT authority fails closed. \
                 The harness is \
                 Rust-only product validation; React, TypeScript, Tauri/WebView, npm tests, \
                 terminal scrollback, provider chat history, and chat memory are not authority.",
            ),
            section_with_json(
                "schema",
                "Behavior coverage matrix",
                "The Rust function `model_lane_behavior_coverage_matrix()` is the \
                 machine-readable coverage matrix for this WP. It is keyed by behavior_id and \
                 carries schema/event family, runtime surface id, UserManual page/tool id, \
                 EventLedger/FlightRecorder evidence path, internal_diagnostics posture, \
                 Palmistry posture, computed consistency proof, deferred reason, and follow-up \
                 ref. The consistency proof resolves the exact runtime id through a typed \
                 compile anchor or the canonical HTTP surface registry, then checks schema, \
                 page, tool, event-evidence, and diagnostic-posture authorities. A renamed or \
                 deleted claimed Rust symbol fails compilation; an unknown nonempty string \
                 fails consistency. Operator Chat and Model Access route rows are generated \
                 from the shipped route registry rather than a duplicated expected list. Private \
                 helper names retained in procedural detail are descriptive disclosure only; the \
                 consistency contract anchors their public owning type, method, event, or route \
                 boundary and does not label the private helper itself verified.",
                json!({
                    "schema_id": "hsk.user_manual_behavior_coverage@1",
                    "matrix_function": "user_manual::model_lane_behavior_coverage_matrix",
                    "verification_function": "user_manual::verify_model_lane_behavior_coverage",
                    "computed_consistency": "BehaviorCoverageRow::self_consistency_result -> Result<BehaviorConsistencyProof, Vec<BehaviorCoverageError>>",
                    "runtime_anchor_registry": "compiled internal Rust function/type anchors plus wp009_surface_registry route anchors",
                    "required_tiers": ["flight_recorder", "internal_diagnostics", "palmistry"],
                    "palmistry_policy": "WIRED through the authenticated watcher and survivor recovery importer; follow_up_ref remains a stable diagnostic correlation URI",
                    "authority_inputs": [
                        "ModelLaneStore::schema_registry_rows",
                        "UserManualStore::list_pages",
                        "UserManualStore::list_tool_entries",
                        "kernel_event_ledger"
                    ]
                }),
            ),
            section(
                "inputs_outputs",
                "Inputs and outputs",
                "Inputs are deterministic local/cloud/subagent lane fixtures, ProjectionPlan \
                 and ConsentReceipt rows for cloud lanes, bounded artifact payload refs, CRDT \
                 base snapshot/state-vector refs, ProcessOwnershipLedger-equivalent lane refs, \
                 cancellation refs, recovery checkpoints, diagnostic tier rows, and MT runtime \
                 status rows. Outputs are ModelLaneRun/ModelLane/ModelLaneMessage rows, \
                 EventLedger event IDs/sequences, diagnostics projections, recovery replay \
                 status, Rust UserManual behavior coverage matrix entries, and native AccessKit author IDs \
                 visible to Argus. Message payload bindings and their ModelLaneMessage are \
                 persisted together, so a terminal-write rejection cannot leave a detached \
                 ArtifactStore payload authority row.",
            ),
            section(
                "failure_modes",
                "Mandatory fail-closed cases",
                "Negative proof must reject direct endpoint/app-src/Tauri/terminal launch \
                 authority, cloud lane launch without durable ProjectionPlan and approved \
                 ConsentReceipt, unbounded retry/backpressure posture, hidden provider \
                 payloads, missing model_lane_context_bundle_artifacts payload authority, \
                 stale CRDT base_snapshot_ref/state_vector, non-v1 encoding, missing or mismatched \
                 EventLedger identity/payload/hash, replay gaps or unresolved dependencies, wrong \
                 replay-metadata order/dependencies/schema version, \
                 actor/site/vector/materialization, mismatched `materialized_projection_hash`, \
                 cross-run/lane/session/trace binding, non-exact promotion/validation refs, or a \
                 non-null advisory promotion receipt, corrupt recovery checkpoint or \
                 replay order gaps, Argus projection count mismatch or missing author IDs, \
                 and FlightRecorder-only diagnostics that omit internal_diagnostics or \
                 Palmistry posture. Once a lane is Completed, Failed, or Cancelled, every \
                 later source/target ModelLaneMessage (including ToolRequest) must fail closed; \
                 the terminal receipt and the prefix captured before cancellation remain the \
                 only replayable authority. Production routing permits at most three durable \
                 attempts for one stage. Exhaustion records `bounded_recovery_exhausted`, never \
                 creates attempt four, and an AfterFailure fallback receives the canonical initial \
                 input plus typed failed-predecessor state and causal message-span linkage. Never \
                 repair an advisory routing output by inventing a `crdt-*://` URI; either keep all \
                 CRDT fields null or commit a real Yjs v1 update and derived state vector first.",
            ),
            section(
                "run_commands",
                "Proof commands",
                "Exact MT-009 proof commands: \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_model_lane_negative_guards_fail_closed -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_concurrent_model_and_operator_lanes_converge_on_shared_crdt_key -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_midstream_cancellation_preserves_prefix_and_rejects_late_messages -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_real_postgres_yjs_updates_compaction_receipts_and_lane_state_converge -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_yjs_atomic_cross_connection_race_keeps_eventledger_and_crdt_receipts_in_lockstep -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests ac9_bounded_retry_exhaustion_fails_after_three_durable_attempts -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests operator_chat_launch_coordinator_cancellation_preserves_prefix_and_rejects_late_activity -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests coordinator_cancellation_fence_rejects_generation_during_terminal_pg_write -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests coordinator_cancellation_fence_retries_after_terminal_pg_failure -- --exact`; \
                 The backend mixed-run command and native Argus command must run in the same shell \
                 with canonical `HANDSHAKE_ARTIFACTS_DIR` and one fresh \
                 `HANDSHAKE_MT009_DIAGNOSTICS_PROOF_NONCE`; the backend produces the typed \
                 projection/provenance artifact before native consumes it. Then run \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_swarm_lane_diagnostics_argus mixed_model_lane_run_is_inspectable_through_argus -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_validation_harness_user_manual_entry_is_current -- --exact`; \
                 `cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_behavior_coverage_tests mixed_model_lane_behaviors_have_manual_coverage -- --exact`.",
            ),
            section(
                "recovery",
                "Recovery steps",
                "When a mixed run fails validation, first replay `ModelLaneStore::replay_run` \
                 and compare backend lane/message counts to native diagnostics rows. Then \
                 inspect `ModelLaneStore::recover_run_after_restart` for checkpoint high-water \
                 mark, recovery events, active/reclaimable leases, cloud consent denials, and \
                  MT runtime status. For CRDT failures, lock and replay the cited append-only \
                  snapshot plus contiguous dependency-resolved updates, then compare the durable \
                  authority binding, derived vector, materialized projection hash, and EventLedger \
                  identity/payload/hash evidence before promotion. internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer.",
            ),
        ],
        anchors: vec![
            page_link("model-lane-launch-adapters"),
            page_link("model-lane-cloud-projection-consent"),
            page_link("model-lane-recovery"),
            page_link("model-lane-diagnostics"),
            NewManualAnchor {
                anchor_kind: "test",
                anchor_value: "mixed_model_lane_integration_pg_tests".into(),
                http_method: "",
            },
            NewManualAnchor {
                anchor_kind: "primitive",
                anchor_value: "hsk.user_manual_behavior_coverage@1".into(),
                http_method: "",
            },
        ],
    }
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
        SurfaceGroup::ModelAccess => vec![
            "400 empty_api_key (BYOK key body is empty)".into(),
            "404 provider_not_offered (provider is excluded or unknown)".into(),
            "503 keychain_unavailable (fail-closed when no OS keychain vault is wired)".into(),
            "500 vault_error (vault refused the write/delete without exposing key material)".into(),
        ],
        SurfaceGroup::ModelRuntimeRegistry => vec![
            "400 MODEL_RUNTIME_SELECTION_INVALID (missing, oversized, or control-bearing selection input)".into(),
            "409 MODEL_RUNTIME_SELECTION_REJECTED (stale current model, non-READY target, timeout, or audit failure)".into(),
            "500 MODEL_RUNTIME_REGISTRY_INTEGRITY_ERROR (durable/catalog identity drift)".into(),
            "503 MODEL_RUNTIME_REGISTRY_UNAVAILABLE (PostgreSQL authority unavailable)".into(),
        ],
        SurfaceGroup::OperatorChat => vec![
            "400 bad_request (invalid operator chat launch selection)".into(),
            "503 launch_not_wired / transcript_not_wired (live coordinator or ModelLaneStore absent)"
                .into(),
            "500 launch_failed_closed / model_lane_error / recorder_error (fail-closed authority path)"
                .into(),
            "500 selection_audit_failed (model catalog selection receipt failed)".into(),
        ],
        SurfaceGroup::ModelLaneCloudConsent => vec![
            "400 bad_request (invalid run-scoped authority or mismatched identity fields)".into(),
            "409 consent/revocation conflict (stale or incompatible canonical authority)".into(),
            "500 launch or revocation finalization failed closed with durable retry state".into(),
        ],
        SurfaceGroup::ModelLaneNavigation => vec![
            "400 bad_request / invalid input (empty lookup token or missing artifact/context query)"
                .into(),
            "404 not_found (unknown run/lane/message/trace/artifact/context id)".into(),
            "diagnostics projection row drift against kernel_event_ledger".into(),
            "500 internal_error (PostgreSQL unavailable; fail-closed)".into(),
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
        SurfaceGroup::ModelAccess => vec![
            "GET /model-access/providers to confirm non-secret provider status before retrying a launch.".into(),
            "DELETE /model-access/byok/:provider/key, then PUT a fresh key if the operator rotates credentials.".into(),
            "If keychain_unavailable persists, keep the provider unavailable rather than writing secrets to a fallback store.".into(),
        ],
        SurfaceGroup::ModelRuntimeRegistry => vec![
            "Refresh GET /model-runtime/registry and select only a current READY live_model_id.".into(),
            "On selection rejection, keep the prior active model and inspect the returned integrity/audit detail before retrying.".into(),
        ],
        SurfaceGroup::OperatorChat => vec![
            "If launch_not_wired appears, wire a live OperatorChatLaunchService before retrying POST /operator-chat/launch.".into(),
            "If transcript_not_wired appears, use ModelLane navigation/EventLedger refs until the ModelLaneStore-backed transcript route is wired.".into(),
            "If selection audit fails, inspect the ModelCatalog recorder path before trusting the picker state.".into(),
        ],
        SurfaceGroup::ModelLaneCloudConsent => vec![
            "Reload the canonical ProjectionPlan and ConsentReceipt before retrying a failed grant-launch.".into(),
            "Retry revocation with the same consent_receipt_ref until every covered lane has durable cleanup and terminal evidence.".into(),
        ],
        SurfaceGroup::ModelLaneNavigation => vec![
            "Use the narrowest known id first, then follow event_ledger_refs to kernel_event_ledger authority.".into(),
            "If artifact/context lookup fails, recover through model_lane_context_bundle_artifacts and model_lane_context_bundle_handoffs before trusting a payload ref.".into(),
            "If diagnostic rows drift, repair the producing ModelLane writer and rerun the MT-009 negative guard before trusting navigation.".into(),
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

    let model_access_route_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "model_access_route_tests",
            "name": "MT-015 model-access route behavior proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_access_route_tests",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model access route tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "model_access_route_tests".into(),
        page_id: None,
        name: "MT-015 model-access route behavior proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_access_route_tests".into(),
        ),
        http_route: Some("/model-access/providers".into()),
        http_method: String::new(),
        description:
            "Exact Rust route proof targets for MT-015 model-access HTTP behavior: non-secret provider enumeration, typed auth-status wire mapping without account fields or Gemini, backend-owned CLI login returning only an opaque pid handle, BYOK store without echoing the key, idempotent delete/rotate, Gemini exclusion, empty-key rejection, and keychain-unavailable fail-closed 503. Production parser, attached-runner, pinned foreground launcher, and picker-launchability proofs are separate unit/integration targets documented on the Cloud Model Access page."
                .into(),
        expected_input:
            "test-utils feature enabled; injected InMemorySecretsVault provider for 200/400/404/delete paths; injected KeychainUnavailableProvider for 503 path; loopback Axum model-access router."
                .into(),
        expected_output:
            "GET /model-access/providers returns non-secret BYOK configured/unavailable rows, typed CLI auth_status rows, and excluded=[gemini]; POST /model-access/cli-bridge/{provider}/login returns only provider plus pid launch handle; PUT stores only in the injected vault and never echoes the key; DELETE removes the vault key idempotently; invalid providers and empty keys return stable errors."
                .into(),
        schema_fields: vec![
            "GET /model-access/providers".into(),
            "PUT /model-access/byok/{provider}/key".into(),
            "DELETE /model-access/byok/{provider}/key".into(),
            "POST /model-access/cli-bridge/{provider}/login".into(),
            "put_store_returns_200_and_never_echoes_the_key".into(),
            "delete_byok_key_is_idempotent_and_updates_status".into(),
            "get_providers_reflects_configured_and_excludes_gemini".into(),
            "cli_bridge_typed_status_wire_mapping_excludes_account_fields_and_gemini".into(),
            "cli_login_route_returns_only_backend_owned_launch_handle".into(),
            "put_empty_key_is_400".into(),
            "put_gemini_is_404_excluded".into(),
            "keychain_unavailable_is_503".into(),
        ],
        common_errors: vec![
            "key_echoed_in_response".into(),
            "cli_auth_probe_leaked_credentials_or_inherited_api_key".into(),
            "gemini_offered".into(),
            "delete_not_idempotent".into(),
            "plaintext_fallback_on_keychain_unavailable".into(),
        ],
        recovery_steps: vec![
            "If the key appears in any HTTP response, inspect StoreKeyBody handling and response JSON before touching the vault.".into(),
            "If CLI auth status is wrong, inspect the canonical launch-target wiring, exact provider grammar, and attached auxiliary runner; do not add PATH rediscovery or free-text expiry inference.".into(),
            "If Gemini appears, inspect ByokProvider::all and the excluded provider list.".into(),
            "If delete fails or leaves configured status, inspect CloudModelAccess::remove_byok_key and the enumeration registry status.".into(),
        ],
        origin: "wp1_mt015_cloud_model_access".into(),
        content_hash: model_access_route_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let official_cli_attached_lifecycle_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "official_cli_attached_lifecycle_tests",
            "name": "Official CLI attached-sandbox lifecycle proof",
            "status": "wired",
            "runtime_anchors": [
                "LiveCliSpawner::spawn",
                "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio",
                "GuardedCliChild::terminate_and_collect"
            ],
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --lib model_runtime::cloud::official_cli_bridge::tests::explicit_failed_terminate_leaves_start_open_without_stop -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_official_cli_bridge_tests failed_termination_with_never_eof_pipe_returns_within_cleanup_deadline -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_official_cli_bridge_tests continuous_output_cannot_starve_live_timeout_polling -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_official_cli_bridge_tests continuous_output_cannot_starve_live_cancellation_polling -- --exact"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("official CLI attached lifecycle tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "official_cli_attached_lifecycle_tests".into(),
        page_id: None,
        name: "Official CLI attached-sandbox lifecycle proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --lib model_runtime::cloud::official_cli_bridge::tests::explicit_failed_terminate_leaves_start_open_without_stop -- --exact".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof for official CLI spawn through the HandshakeNative attached-sandbox and guarded process cleanup path."
                .into(),
        expected_input:
            "LiveCliSpawner configuration pinned to HandshakeNativeSandboxAdapter; test-utils feature enabled."
                .into(),
        expected_output:
            "LiveCliSpawner::spawn delegates OS-process creation to HandshakeNativeSandboxAdapter::spawn_attached_with_stdio. All terminal paths—success, failure, timeout, cancellation, and unwind—converge on GuardedCliChild::terminate_and_collect; ProcessOwnershipLedger STOP is recorded only after termination and reap succeed. An unreaped termination leaves the durable START open with no fabricated STOP so reconciliation can recover it."
                .into(),
        schema_fields: vec![
            "LiveCliSpawner::spawn".into(),
            "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio".into(),
            "GuardedCliChild::terminate_and_collect".into(),
            "ProcessOwnershipLedger START".into(),
            "ProcessOwnershipLedger STOP".into(),
            "official_cli_bridge".into(),
        ],
        common_errors: vec![
            "official CLI process spawned outside the HandshakeNative attached-sandbox".into(),
            "timeout, cancellation, or unwind bypasses terminate-and-reap cleanup".into(),
            "termination or reap fails while the process lifecycle START is open".into(),
            "STOP is fabricated before successful termination and reap".into(),
        ],
        recovery_steps: vec![
            "Inspect the open official_cli_bridge ProcessOwnershipLedger START and preserve it as cleanup-pending evidence.".into(),
            "Retry process-tree termination and reap through GuardedCliChild::terminate_and_collect; do not synthesize STOP while the child remains unreaped.".into(),
            "Record STOP only after termination and reap are proven, then reconcile the durable lifecycle row.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: official_cli_attached_lifecycle_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let cloud_byok_leak_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "cloud_byok_access_config_leak_tests",
            "name": "MT-015 BYOK OS-keychain leak guard proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features \"test-utils,os-keychain\" --test cloud_byok_access_config_leak_tests byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain -- --exact",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("cloud BYOK leak tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "cloud_byok_access_config_leak_tests".into(),
        page_id: None,
        name: "MT-015 BYOK OS-keychain leak guard proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features \"test-utils,os-keychain\" --test cloud_byok_access_config_leak_tests byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain -- --exact".into(),
        ),
        http_route: Some("/model-access/byok/:provider/key".into()),
        http_method: "PUT".into(),
        description:
            "Security-critical MT-015 canary proof that BYOK keys are stored only in OsKeychainSecretsVault, round-trip only for provider Authorization use, create no consent approval, and never leak into logs, Flight Recorder-adjacent tracing, cloud invocation audit rows, Debug output, enumeration JSON, or HTTP bodies."
                .into(),
        expected_input:
            "Windows target with os-keychain feature enabled; unique test keychain namespace; wiremock OpenAI-compatible endpoint; canary BYOK key."
                .into(),
        expected_output:
            "The canary appears only in the required provider Authorization header and the OS keychain round-trip; every non-keychain surface is canary-free and the keychain entry is deleted before assertions."
                .into(),
        schema_fields: vec![
            "CloudModelAccess::production".into(),
            "OsKeychainSecretsVault".into(),
            "VaultApiKeyProvider".into(),
            "cloud_invocations".into(),
            "byok_canary_key_never_leaks_and_round_trips_only_through_os_keychain".into(),
        ],
        common_errors: vec![
            "in_memory_vault_used_in_production".into(),
            "canary_leaked_to_debug_or_logs".into(),
            "consent_preapproved_on_key_save".into(),
        ],
        recovery_steps: vec![
            "If the production service is not OsKeychainSecretsVault, inspect CloudModelAccess::production wiring.".into(),
            "If the canary leaks, inspect the failing sink first and remove key formatting or body echoing before rerunning.".into(),
            "If consent is preapproved by key save, separate CloudModelAccess from ConsentGate state.".into(),
        ],
        origin: "wp1_mt015_cloud_model_access".into(),
        content_hash: cloud_byok_leak_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let cloud_models_argus_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "test_cloud_models_settings_argus",
            "name": "MT-015 Cloud Models Settings Argus proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_cloud_models_settings_argus",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("cloud models Argus tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "test_cloud_models_settings_argus".into(),
        page_id: None,
        name: "MT-015 Cloud Models Settings Argus proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_cloud_models_settings_argus".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Native Argus/AccessKit proof for Settings > Cloud Models: stable provider author IDs, visible logged-in/logged-out/expired states for Claude Code and Codex, no Gemini controls, static BYOK key-entry fallback when the backend is unreachable, UI key-buffer clearing on save/close, an addressable foreground-terminal confirmation, and fixed provider-owned official CLI login commands without terminal launch during headless tests. The same controls are reachable in BOTH settings hosts: the main-window modal (Argus window `main`, root node `settings.dialog`) and the detached Settings window (Argus window `popout-settings`, root node `popout-window-settings`, title `Handshake – Settings`) entered via `settings.popout` and left via `settings.redock`."
                .into(),
        expected_input:
            "egui_kittest harness with AccessKit enabled; seeded CloudAccessSnapshot for positive rows and empty snapshot/no client for backend-unreachable fallback."
                .into(),
        expected_output:
            "Addressable settings.cloud.* author IDs for Anthropic/OpenAI BYOK and Claude Code/Codex CLI rows; each CLI status target renders logged in, logged out, and session expired; no gemini author IDs; typed BYOK drafts are wiped; the first login click only opens confirmation; confirmed fixed provider CLI command is recorded while terminal launch remains suppressed in the test shell."
                .into(),
        schema_fields: vec![
            "settings.cloud.byok.anthropic.key".into(),
            "settings.cloud.byok.openai.key".into(),
            "settings.cloud.cli.claude_code.login".into(),
            "settings.cloud.cli.codex.login".into(),
            // Detached Settings window (MT-015): the same surface hosted in its own OS window. These are
            // the targeting handles a driver needs; the exact proofs live in test_settings_dialog (see
            // the cloud-model-access page's navigation + behavior-matrix sections).
            "settings.popout".into(),
            "settings.redock".into(),
            "popout-settings".into(),
            "popout-window-settings".into(),
            "cloud_models_controls_are_addressable_and_gemini_is_never_offered".into(),
            "cli_bridge_auth_status_renders_all_three_states_for_claude_and_codex".into(),
            "typing_and_saving_a_byok_key_clears_the_ui_buffer".into(),
            "cloud_models_key_entry_renders_when_backend_unreachable".into(),
            "typed_byok_key_is_wiped_from_egui_memory_after_close".into(),
            "cli_bridge_login_records_the_official_command_without_stealing_focus".into(),
        ],
        common_errors: vec![
            "missing_accesskit_author_id".into(),
            "cli_auth_status_missing_or_mislabeled".into(),
            "gemini_control_rendered".into(),
            "key_buffer_lingers_after_save_or_close".into(),
            "login_command_not_provider_owned".into(),
        ],
        recovery_steps: vec![
            "If a provider control is missing, inspect render_cloud_models_body and cloud_byok_*_author_id helpers.".into(),
            "If CLI auth state is wrong, inspect CliBridgeAuthStatus parsing, the model-access auth probe, and CloudCliAuthStatus::from_wire.".into(),
            "If a typed key remains in UI memory, inspect CloudModelsSettingsState::clear_key_drafts and reset_cloud_key_edit_memory.".into(),
            "If login launches the wrong command, inspect CloudCliRow login_program/login_args plumbing.".into(),
        ],
        origin: "wp1_mt015_cloud_model_access".into(),
        content_hash: cloud_models_argus_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let model_lane_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "model_lane_schema_pg_tests",
            "name": "Dexterity ModelLane PostgreSQL proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests model_lane_schema_persists_and_replays_eventledger_rows -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests dexterity_launch_records_real_swarm_spawn_session_runtime_path -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests model_lane_schema_serializes_competing_terminal_updates -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests model_lane_schema_rejects_missing_locus_binding_and_idempotency_conflict -- --exact"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model lane tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "model_lane_schema_pg_tests".into(),
        page_id: None,
        name: "Dexterity ModelLane PostgreSQL proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof targets for Dexterity ModelLaneRun, ModelLane, ModelLaneMessage storage, and SwarmCoordinator runtime launch wiring."
                .into(),
        expected_input:
            "Real PostgreSQL test URL or Handshake-managed PostgreSQL; test-utils feature enabled."
                .into(),
        expected_output:
            "EventLedger-backed ModelLane rows, schema registry rows, runtime spawn_session launch rows, idempotency behavior, and replay ordered by event_ledger_seq."
                .into(),
        schema_fields: vec![
            "ModelLaneRun".into(),
            "ModelLane".into(),
            "ModelLaneMessage".into(),
            "DexterityLaunchContract".into(),
            "SpawnRequest::with_dexterity_launch".into(),
            "locus_binding_ref".into(),
            "event_ledger_seq".into(),
            "payload_sha256".into(),
            "replay_order_key".into(),
            "recovery_state".into(),
            "promotion_receipt_ref".into(),
            "memory_pack_ref".into(),
            "memory_pack_hash".into(),
            "determinism_mode".into(),
            "budget_summary_ref".into(),
            "selected_model_id".into(),
            "candidate_model_ids".into(),
            "procedural_review_status".into(),
            "truncation_warning_ref".into(),
            "rejection_reason_refs".into(),
            "ArtifactStore".into(),
            "CRDT".into(),
            "Flight Recorder".into(),
            "internal_diagnostics".into(),
            "Palmistry".into(),
        ],
        common_errors: vec![
            "missing PostgreSQL/EventLedger migration".into(),
            "missing locus_binding_ref".into(),
            "mismatched Locus WP/MT/task-board/session owner".into(),
            "unsupported provider_kind or missing capability snapshot".into(),
            "malformed trace/span linkage".into(),
            "proposal without CRDT base snapshot".into(),
            "payload_sha256 is not lowercase sha256 hex".into(),
            "idempotency conflict".into(),
        ],
        recovery_steps: vec![
            "Run migrations against the active PostgreSQL authority.".into(),
            "Replay by event_ledger_seq and compare ModelLane records to EventLedger rows.".into(),
            "Reuse the same idempotency_key only when payload_sha256 is unchanged.".into(),
            "For HBR-INT-009, inspect Flight Recorder/EventLedger rows; internal_diagnostics is WIRED through the native producer and Problems projection, and Palmistry is WIRED through the authenticated watcher and survivor recovery importer.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: model_lane_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let model_lane_launch_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "model_lane_launch_tests",
            "name": "Dexterity launch adapter runtime proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_all_lane_kinds_through_rust_registry -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_rejects_direct_endpoint_frontend_tauri_and_terminal_bypass -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_cancellation_reclaim_contracts_all_lane_kinds -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_records_factory_failure_through_swarm_coordinator -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests production_builder_wires_model_lane_store_for_failed_dexterity_launch -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_rejects_ready_transition_before_persistence_commit -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_cancel_session_records_terminal_model_lane_state -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests model_lane_launch_reaper_records_terminal_state_before_teardown -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_schema_pg_tests dexterity_launch_records_real_swarm_spawn_session_runtime_path -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_launch_user_manual_entry_is_current -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_schema_user_manual_entry_is_current -- --exact"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model lane launch tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "model_lane_launch_tests".into(),
        page_id: None,
        name: "Dexterity launch adapter runtime proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_launch_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof targets for Dexterity launch adapter normalization and runtime-owned launch paths across local, cloud, CLI, human, subagent, and validator lanes."
                .into(),
        expected_input:
            "Real PostgreSQL test URL or Handshake-managed PostgreSQL; test-utils feature enabled."
                .into(),
        expected_output:
            "Registry-normalized launches, SwarmCoordinator preflight, coordinator-owned no-OS lanes with live-authority caller receipts, ModelRuntime load/unload proof, no Ready/runtime exposure before ModelLane persistence, EventLedger stream-backed rows, production builder store wiring, missing-contract bypass rejection, factory failure records, terminal-failure refs for runtime failed state, durable cancellation terminal state, lease-reaper terminal persistence before teardown, retryable terminal intent before runtime teardown, per-lane terminal serialization, bypass rejection, cancellation/reclaim contracts, schema runtime proof, and manual parity."
                .into(),
        schema_fields: vec![
            "DexterityLaunchAdapterRegistry".into(),
            "DexterityNormalizedLaunch".into(),
            "SwarmCoordinator::spawn_session".into(),
            "ModelRuntime".into(),
            "CloudLane/BYOK".into(),
            "CliBridge".into(),
            "Operator".into(),
            "SubagentManager".into(),
            "ValidatorRunner".into(),
            "cancellation_ref".into(),
            "reclaim_policy_ref".into(),
            "terminal_status_mapping_ref".into(),
            "process_ownership_ref".into(),
            "no_os_process_reason_ref".into(),
            "startup_failure_ref".into(),
            "event_ledger_stream_id".into(),
            "DexterityNoOsLaunchCaller".into(),
            "model_lane_terminal".into(),
            "provider_feature_profile_ref".into(),
            "requested_execution_policy_ref".into(),
            "effective_execution_policy_ref".into(),
            "Flight Recorder".into(),
            "EventLedger".into(),
            "Palmistry".into(),
        ],
        common_errors: vec![
            "direct endpoint launch bypass".into(),
            "app/src or app/src-tauri launch authority".into(),
            "terminal-only CLI launch state".into(),
            "missing ModelLaneStore".into(),
            "ModelLaneStore-backed coordinator without SpawnRequest::with_dexterity_launch".into(),
            "missing cancellation or reclaim metadata".into(),
            "unsupported tool capability".into(),
            "BYOK cloud missing provider/projection/consent refs".into(),
            "human/subagent/validator lane without no-OS-process equivalent".into(),
            "Ready transition before ModelLane persistence commit".into(),
            "terminal state write failure before runtime teardown".into(),
            "stale no-OS caller receipt after authority session removal".into(),
        ],
        recovery_steps: vec![
            "Route through DexterityLaunchAdapterRegistry, attach SpawnRequest::with_dexterity_launch, and call SwarmCoordinator before runtime creation.".into(),
            "Use ModelLaneStore on the same PostgreSQL/EventLedger authority path as the runtime.".into(),
            "For cloud lanes, provide explicit BYOK provider, projection_plan_ref, and consent_receipt_ref.".into(),
            "For no-OS lanes, authorize from a live Ready/Generating authority session and record no_os_process_reason_ref instead of faking a process.".into(),
            "If terminal lane persistence fails, retry the terminal action while the live handle still exists; terminal writes serialize by lane_id.".into(),
            "For HBR-INT-009, inspect Flight Recorder/EventLedger rows; internal_diagnostics is WIRED through the native producer and Problems projection, and Palmistry is WIRED through the authenticated watcher and survivor recovery importer.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: model_lane_launch_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let model_lane_promotion_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "model_lane_promotion_pg_tests",
            "name": "Dexterity promotion decision runtime proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests model_lane_promotion_appends_eventledger_and_replays_decision -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests model_lane_promotion_reordered_inputs_keep_same_decision_hash -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_promotion_user_manual_entry_is_current -- --exact"
            ],
            "hardening": [
                "final_state",
                "typed_message_routing",
                "db_derived_crdt_current_state",
                "exact_promotion_decision_and_artifact_binding",
                "phantom_input_ref_denial"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model lane promotion tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "model_lane_promotion_pg_tests".into(),
        page_id: None,
        name: "Dexterity promotion decision runtime proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_promotion_pg_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof targets for Dexterity routing policies and advisory-to-authority promotion decisions."
                .into(),
        expected_input:
            "Real PostgreSQL test URL or Handshake-managed PostgreSQL; test-utils feature enabled; advisory ModelLaneMessage rows with CRDT refs."
                .into(),
        expected_output:
            "EventLedger-backed ModelLanePromotionDecision rows, replay ordered by event_ledger_seq, typed approved/denied state_history plus final_state, DB-derived CRDT base/state-vector denials, schema and aggregate-version denials, phantom input-ref denial, exact promotion_decision_id and promoted artifact binding, direct authority mutation rejection, duplicate idempotency conflict, typed message routing, and canonical decision hash stable across reordered input refs."
                .into(),
        schema_fields: vec![
            "ModelLaneRoutingPolicy".into(),
            "ModelLaneRoutingMetadata".into(),
            "ModelLanePromotionDecision".into(),
            "NewModelLanePromotionDecision".into(),
            "hsk.model_lane_promotion_decision@1".into(),
            "model_lane_promotion_decision".into(),
            "local_first".into(),
            "cloud_review".into(),
            "cloud_plan_local_execute".into(),
            "parallel_debate".into(),
            "validator_lane".into(),
            "operator_lane".into(),
            "input_refs".into(),
            "selected_input_refs".into(),
            "rejected_input_refs".into(),
            "target_role".into(),
            "target_session".into(),
            "correlation_id".into(),
            "requires_ack".into(),
            "ack_for".into(),
            "canonical_hash_basis".into(),
            "canonical_decision_hash".into(),
            "final_state".into(),
            "expected_event_ledger_version".into(),
            "current_event_ledger_version".into(),
            "base_snapshot_ref".into(),
            "current_base_snapshot_ref".into(),
            "state_vector".into(),
            "current_state_vector".into(),
            "schema_id".into(),
            "promotion_decision_id".into(),
            "promotion_gate_ref".into(),
            "promotion_receipt_ref".into(),
            "promoted_artifact_ref".into(),
            "promoted_artifact_sha256".into(),
            "promoted_artifact_version".into(),
            "event_ledger_seq".into(),
            "Flight Recorder".into(),
            "EventLedger".into(),
            "internal_diagnostics".into(),
            "Palmistry".into(),
        ],
        common_errors: vec![
            "AggregateVersionMismatch".into(),
            "SchemaMismatch".into(),
            "StaleBase".into(),
            "StaleStateVector".into(),
            "InputRefMismatch".into(),
            "DirectAuthorityMutation".into(),
            "MissingPromotionAuthority".into(),
            "MissingPromotedArtifactBinding".into(),
            "Promoted ModelLaneMessage without approved PromotionGate resolution".into(),
            "idempotency conflict".into(),
        ],
        recovery_steps: vec![
            "Replay promotion decisions by run_id with ModelLaneStore::replay_promotion_decisions and compare event_ledger_seq to kernel_event_ledger.".into(),
            "For stale CRDT denials, inspect the selected advisory ModelLaneMessage rows because current_base_snapshot_ref/current_state_vector are DB-derived, then request promotion again with a new idempotency_key.".into(),
            "For schema denials, compare schema_id to model_lane_schema_registry before retry.".into(),
            "For aggregate version denials, read the current kernel_event_ledger aggregate version and rebuild the decision input.".into(),
            "For input-ref denials, verify every model-lane-message:// ref exists in the same run, is advisory or promotion_candidate, and carries selected CRDT state.".into(),
            "Never write ModelLaneAuthority::Promoted directly; first record an approved ModelLanePromotionDecision with matching promotion_decision_id, promotion_gate_ref, promotion_receipt_ref, promoted_artifact_ref, promoted_artifact_sha256, and promoted_artifact_version.".into(),
            "For HBR-INT-009, inspect Flight Recorder/EventLedger rows; internal_diagnostics is WIRED through the native producer and Problems projection, and Palmistry is WIRED through the authenticated watcher and survivor recovery importer.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: model_lane_promotion_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let model_lane_context_bundle_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "model_lane_context_bundle_pg_tests",
            "name": "Dexterity ContextBundle handoff runtime proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests model_lane_context_bundle_persists_selection_state_and_replays -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests model_lane_context_bundle_missing_artifact_ref_fails_closed -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_context_bundle_user_manual_entry_is_current -- --exact"
            ],
            "hardening": [
                "artifact_binding_authority_table",
                "dedicated_postgresql_handoff_table",
                "eventledger_artifact_stored",
                "eventledger_context_bundle_recorded",
                "source_message_artifact_ref_hash_match",
                "artifact_binding_hash_match",
                "downstream_lane_consumption",
                "coordinator_adapter_invocation",
                "kernel_context_bundle_v1_identity",
                "selected_rejected_unresolved_superseded_replay",
                "crdt_update_bytes_ref_state_vector_base_snapshot",
                "yjs_compatible_replay_metadata",
                "loom_flight_recorder_evidence",
                "fems_memory_pack_cloud_safe",
                "fems_local_only_cloud_rejection",
                "hidden_projection_ref_rejection",
                "normalized_hidden_memory_uri_rejection",
                "bounded_loom_and_memory_refs"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model lane ContextBundle tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "model_lane_context_bundle_pg_tests".into(),
        page_id: None,
        name: "Dexterity ContextBundle handoff runtime proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_context_bundle_pg_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof targets for Dexterity model-to-model ContextBundle handoff persistence with artifact binding, downstream replay, CRDT/Loom/FEMS binding, and fail-closed artifact refs."
                .into(),
        expected_input:
            "Real PostgreSQL test URL or Handshake-managed PostgreSQL; test-utils feature enabled; source ModelLaneMessage rows with replayable payload refs; NewModelLaneContextBundleArtifactBinding rows whose payload_json sha256 equals artifact_sha256/content_hash; downstream_lane_id; work_packet_id; micro_task_id; task_board_id; explicit reviewed MemoryPack refs for cloud lanes."
                .into(),
        expected_output:
            "EventLedger-backed ModelLaneContextBundleArtifactBindingRecord and ModelLaneContextBundleHandoff rows, ARTIFACT_STORED payload stamping, model_lane_context_bundle_artifacts authority rows with artifact_manifest_ref/artifact_payload_ref/payload_json/artifact_binding_hash, replay ordered by event_ledger_seq for one context_bundle_id, downstream-only ModelLaneDownstreamContextBundle consumption through ModelLaneStore::consume_context_bundle_for_downstream and SwarmCoordinator::context_bundle_for_downstream_lane, SwarmCoordinator::invoke_downstream_context_bundle adapter invocation, to_kernel_context_bundle conversion with ContextBundle V1 CTX-<hash> identity, selected/rejected/unresolved/superseded selection states, schema registry rows hsk.model_lane_context_bundle_artifact@1 and hsk.model_lane_context_bundle_handoff@1, fail-closed missing source and artifact_ref/artifact_sha256/content_hash mismatch against ArtifactStore/EventLedger authority, cloud-safe FEMS MemoryPack enforcement, local_only_context cloud rejection, review_status reviewed, operator_reviewed, or validator_reviewed, hidden provider/session memory rejection including projection_ref and normalized hidden-memory URI checks, memory_pack_refs exceeds bounded FEMS limit, canonical append-only PostgreSQL/EventLedger CRDT state_vector/base_snapshot_ref/update_bytes_ref validation with Yjs-compatible format yjs_update_v1 only, exact replay_metadata replay_order_key/dependency_update_ids/schema_version parity with the persisted update, forged replay metadata producing no handoff row, full crdt_authority_binding parity, exact promotion/validation refs, null advisory promotion receipt, materialized_projection_hash verification, Loom event_ledger_evidence_ref and flight_recorder_evidence_ref replay, loom_refs exceeds bounded limit, duplicate idempotency returning the original context_bundle_hash, and manual parity."
                .into(),
        schema_fields: vec![
            "ModelLaneContextBundleArtifactBindingRecord".into(),
            "NewModelLaneContextBundleArtifactBinding".into(),
            "ModelLaneContextBundleHandoffRecord".into(),
            "NewModelLaneContextBundleHandoff".into(),
            "ModelLaneDownstreamContextBundle".into(),
            "ModelLaneCrdtHandoffMetadata".into(),
            "ModelLaneLoomHandoffRef".into(),
            "ModelLaneMemoryPackHandoffRef".into(),
            "hsk.model_lane_context_bundle_artifact@1".into(),
            "hsk.model_lane_context_bundle_handoff@1".into(),
            "model_lane_context_bundle_artifacts".into(),
            "model_lane_context_bundle_handoff".into(),
            "context_bundle_id".into(),
            "context_bundle_hash".into(),
            "artifact_binding_hash".into(),
            "artifact_manifest_ref".into(),
            "artifact_payload_ref".into(),
            "payload_json".into(),
            "downstream_lane_id".into(),
            "SwarmCoordinator::invoke_downstream_context_bundle".into(),
            "ModelAdapterRequest".into(),
            "source_message_id".into(),
            "artifact_ref".into(),
            "artifact_sha256".into(),
            "content_hash".into(),
            "work_packet_id".into(),
            "micro_task_id".into(),
            "task_board_id".into(),
            "to_kernel_context_bundle".into(),
            "CTX-<hash>".into(),
            "selected".into(),
            "rejected".into(),
            "unresolved".into(),
            "superseded".into(),
            "schema_id".into(),
            "document_id".into(),
            "workspace_id".into(),
            "actor_id".into(),
            "actor_kind".into(),
            "lane_id".into(),
            "crdt_site_id".into(),
            "update_seq".into(),
            "update_bytes_ref".into(),
            "update_sha256".into(),
            "state_vector".into(),
            "base_snapshot_ref".into(),
            "materialized_projection_hash".into(),
            "replay_metadata".into(),
            "replay_order_key".into(),
            "dependency_update_ids".into(),
            "schema_version".into(),
            "yjs_update_v1".into(),
            "promotion_gate_ref".into(),
            "validation_runner_ref".into(),
            "authority_effect".into(),
            "loom_refs".into(),
            "event_ledger_evidence_ref".into(),
            "flight_recorder_evidence_ref".into(),
            "memory_pack_ref".into(),
            "memory_pack_hash".into(),
            "scope_tag".into(),
            "review_status".into(),
            "cloud_safe".into(),
            "classification".into(),
            "projection_ref".into(),
            "Flight Recorder".into(),
            "EventLedger".into(),
            "internal_diagnostics".into(),
            "Palmistry".into(),
        ],
        common_errors: vec![
            "artifact binding must exist before context handoff".into(),
            "artifact_payload_ref must match artifact_ref".into(),
            "payload_json sha256 must match content_hash".into(),
            "source_message_id is not replayable".into(),
            "handoff.artifact_ref must match source.payload_ref".into(),
            "handoff.artifact_sha256 must match source.payload_sha256".into(),
            "handoff.content_hash must match source.payload_sha256".into(),
            "handoff artifact hash must match ArtifactStore/EventLedger authority".into(),
            "downstream_lane_id is required".into(),
            "work_packet_id is required".into(),
            "micro_task_id is required".into(),
            "task_board_id is required".into(),
            "cloud downstream handoff requires every MemoryPack ref to be cloud_safe".into(),
            "cloud downstream handoff cannot use local_only_context MemoryPack refs".into(),
            "MemoryPack handoff cannot use hidden provider/session memory as authority".into(),
            "MemoryPack handoff projection_ref cannot use hidden provider/session memory as authority".into(),
            "MemoryPack review_status must be reviewed, operator_reviewed, or validator_reviewed".into(),
            "memory_pack_refs exceeds bounded FEMS limit".into(),
            "CRDT ModelLaneMessage handoff requires crdt_payload metadata".into(),
            "crdt_payload.update_bytes_ref must match source.crdt_update_ref".into(),
            "crdt_payload.replay_metadata must declare Yjs-compatible format yjs_update_v1".into(),
            "crdt_payload.replay_metadata must exactly match persisted replay_order_key, dependency_update_ids, and schema_version".into(),
            "crdt_payload.authority_effect must be advisory_only before promotion".into(),
            "loom_refs exceeds bounded limit".into(),
            "idempotency conflict".into(),
        ],
        recovery_steps: vec![
            "Recover artifact authority first with ModelLaneStore::record_context_bundle_artifact_binding and verify model_lane_context_bundle_artifacts.payload_json hashes to artifact_sha256/content_hash.".into(),
            "Replay handoffs with ModelLaneStore::replay_context_bundle_handoffs(run_id, context_bundle_id) and compare event_ledger_seq to kernel_event_ledger.".into(),
            "For downstream recovery, call ModelLaneStore::consume_context_bundle_for_downstream or SwarmCoordinator::context_bundle_for_downstream_lane, convert ModelLaneDownstreamContextBundle with to_kernel_context_bundle, and verify the kernel ContextBundle id follows CTX-<hash>.".into(),
            "For runtime model invocation, call SwarmCoordinator::invoke_downstream_context_bundle so the adapter receives ModelAdapterRequest.context_bundle from PostgreSQL/EventLedger replay.".into(),
            "For missing source failures, record the source ModelLaneMessage first and retry with a new idempotency_key.".into(),
            "For artifact failures, copy artifact_ref/artifact_sha256/content_hash from the source ModelLaneMessage row instead of trusting caller memory.".into(),
            "For cloud handoff failures, use explicit reviewed MemoryPack refs with memory_pack_hash, scope_tag, classification, projection_ref, evidence_ref, cloud_safe = true, and classification other than local_only_context.".into(),
            "For CRDT failures, copy update_bytes_ref, state_vector, and base_snapshot_ref from the source ModelLaneMessage CRDT fields and keep authority_effect = advisory_only until PromotionGate approval.".into(),
            "For Loom failures, include workspace/block refs plus EventLedger and Flight Recorder evidence refs before retry.".into(),
            "For HBR-INT-009, inspect EventLedger rows and Flight Recorder evidence refs; direct Flight Recorder event emission is DEFERRED-with-reason to MT-008, internal_diagnostics is WIRED through the native producer and Problems projection, and Palmistry is WIRED through the authenticated watcher and survivor recovery importer.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: model_lane_context_bundle_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let cloud_model_lane_policy_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "cloud_model_lane_policy_pg_tests",
            "name": "Dexterity cloud ProjectionPlan and ConsentReceipt runtime proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests cloud_projection_and_consent_receipts_persist_and_replay -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests cloud_lane_rejects_missing_expired_mismatched_and_revoked_consent -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests cloud_consent_revocation_cancels_pending_lanes_with_eventledger_evidence -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests cloud_model_lane_policy_user_manual_entry_is_current -- --exact"
            ],
            "hardening": [
                "durable_projection_plan_table",
                "durable_consent_receipt_table",
                "eventledger_denial_rows",
                "pre_factory_provider_suppression",
                "cx_mm_007_consent_status",
                "revocation_cancels_covered_lanes",
                "advisory_until_promotion"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("cloud model-lane policy tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "cloud_model_lane_policy_pg_tests".into(),
        page_id: None,
        name: "Dexterity cloud ProjectionPlan and ConsentReceipt runtime proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test cloud_model_lane_policy_pg_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof targets for Dexterity cloud ProjectionPlan/ConsentReceipt persistence, CX-MM-007 denial evidence, pre-factory provider suppression, advisory cloud outputs, and revocation cancellation."
                .into(),
        expected_input:
            "Real PostgreSQL test URL or Handshake-managed PostgreSQL; test-utils feature enabled; NewModelLaneCloudProjectionPlan rows; NewModelLaneCloudConsentReceipt rows; BYOK cloud SpawnRequest with DexterityLaunchContract; cloud ModelLane rows with projection_plan_ref and consent_receipt_ref; revoked receipt id for cancellation proof."
                .into(),
        expected_output:
            "EventLedger-backed ModelLaneCloudProjectionPlanRecord and ModelLaneCloudConsentReceiptRecord rows in model_lane_cloud_projection_plans and model_lane_cloud_consent_receipts, schema registry rows hsk.model_lane_cloud_projection_plan@2, hsk.model_lane_cloud_consent_receipt@2, and hsk.model_lane_cloud_consent_denial@1, replay through ModelLaneStore::replay_cloud_consent_authority, single-lane cloud launch allowed only when durable ProjectionPlan and ConsentReceipt match projection_plan_hash/run_id/lane_id/model_session_id/provider_kind/requested_model_id/scope_hash/retention/export/fan_out_targets, single-run launch allowed only when durable run-scoped authority matches run_id plus the shared non-lane authority fields, missing/expired/mismatched/revoked consent rejected with CX-MM-007 and model_lane_cloud_consent_denial EventLedger payload provider_call_attempted = false, SwarmCoordinator::spawn_session preflight blocks before factory.create and spawn_cloud_consent_batch preflights every run-scoped request before dispatch, cloud ModelLaneMessage diagnostic_payload carries projection/redaction metadata, ModelLaneAuthority::Promoted rejects without approved PromotionGate, and ModelLaneStore::revoke_cloud_consent_receipt cancels every durable covered lane with failstate_code CX-MM-007 and per-lane model_lane_terminal EventLedger evidence."
                .into(),
        schema_fields: vec![
            "NewModelLaneCloudProjectionPlan".into(),
            "ModelLaneCloudProjectionPlanRecord".into(),
            "NewModelLaneCloudConsentReceipt".into(),
            "ModelLaneCloudConsentReceiptRecord".into(),
            "ModelLaneCloudConsentAuthorityReplay".into(),
            "ModelLaneStore::record_cloud_projection_plan".into(),
            "ModelLaneStore::record_cloud_consent_receipt".into(),
            "ModelLaneStore::replay_cloud_consent_authority".into(),
            "ModelLaneStore::preflight_cloud_spawn_request".into(),
            "ModelLaneStore::revoke_cloud_consent_receipt".into(),
            "hsk.model_lane_cloud_projection_plan@2".into(),
            "hsk.model_lane_cloud_consent_receipt@2".into(),
            "hsk.model_lane_cloud_consent_denial@1".into(),
            "model_lane_cloud_projection_plans".into(),
            "model_lane_cloud_consent_receipts".into(),
            "model_lane_cloud_consent_denial".into(),
            "model_lane_terminal".into(),
            "CX-MM-007".into(),
            "consent_status".into(),
            "provider_call_attempted".into(),
            "projection_plan_hash".into(),
            "scope_hash".into(),
            "retention_policy".into(),
            "export_posture".into(),
            "fan_out_targets".into(),
            "redaction_policy_ref".into(),
            "user_manual_behavior_ref".into(),
            "Flight Recorder".into(),
            "EventLedger".into(),
            "internal_diagnostics".into(),
            "Palmistry".into(),
        ],
        common_errors: vec![
            "ProjectionPlan is not durable".into(),
            "ConsentReceipt is not durable".into(),
            "ConsentReceipt validity window is not current".into(),
            "ConsentReceipt is revoked".into(),
            "ConsentReceipt policy fields must match ProjectionPlan scope, retention, export, and fan-out".into(),
            "cloud lane launch denied before provider call".into(),
            "Promoted ModelLaneMessage requires approved PromotionGate resolution".into(),
            "source_artifact_refs cannot use hidden provider/session memory".into(),
            "idempotency conflict".into(),
        ],
        recovery_steps: vec![
            "Record or replay the ProjectionPlan with ModelLaneStore::record_cloud_projection_plan, then compare projection_plan_hash and event_ledger_seq to kernel_event_ledger.".into(),
            "Record or replay the ConsentReceipt with ModelLaneStore::record_cloud_consent_receipt. For single_lane verify projection_plan_hash/run_id/lane_id/model_session_id/provider_kind/requested_model_id/scope_hash/retention/export/fan_out_targets; for single_run verify projection_plan_hash/run_id/scope_hash/retention/export/fan_out_targets and confirm no lane-bound identity is present.".into(),
            "For CX-MM-007 denials, inspect model_lane_cloud_consent_denial payloads and confirm provider_call_attempted = false before retrying with a new valid receipt.".into(),
            "For revocations, call ModelLaneStore::revoke_cloud_consent_receipt and replay the affected run to confirm Cancelled lanes with failstate_code CX-MM-007.".into(),
            "For cloud outputs, keep ModelLaneAuthority::Advisory until a PromotionGate decision exists; never write ModelLaneAuthority::Promoted directly.".into(),
            "For HBR-INT-009, inspect EventLedger rows; direct Flight Recorder event emission is DEFERRED-with-reason to FR-EVT-CLOUD wiring, internal_diagnostics is WIRED through the native producer and Problems projection, and Palmistry is WIRED through the authenticated watcher and survivor recovery importer.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: cloud_model_lane_policy_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let model_lane_recovery_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "model_lane_recovery_pg_tests",
            "name": "Dexterity recovery and replay runtime proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_replays_from_postgres_eventledger_checkpoint -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_includes_current_leases_but_bounds_replay_adjunct_state -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_rejects_corrupt_checkpoint_and_event_seq_gap -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_restores_mt_runtime_status_refs_after_restart -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests diagnostic_tier_record_rejects_flight_recorder_only_evidence -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_rejects_missing_payload_stale_crdt_and_duplicate_idempotency -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_uses_eventledger_checkpoint_authority_over_mutable_row -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests model_lane_recovery_rejects_post_checkpoint_payload_and_crdt_repairs -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_recovery_user_manual_entry_is_current -- --exact"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model-lane recovery tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "model_lane_recovery_pg_tests".into(),
        page_id: None,
        name: "Dexterity recovery and replay runtime proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_recovery_pg_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof targets for Dexterity checkpoint/EventLedger recovery, lane leases, payload authority, CRDT stale-base rejection, MT runtime status restoration, and HBR-INT-009 diagnostic posture."
                .into(),
        expected_input:
            "Real PostgreSQL test URL or Handshake-managed PostgreSQL; test-utils feature enabled; ModelLaneRun/ModelLane/ModelLaneMessage rows; ArtifactStore context-bundle artifact bindings; recovery checkpoint/event/lease/diagnostic/MT-status rows."
                .into(),
        expected_output:
            "EventLedger-backed ModelLaneRecoveryCheckpointRecord, ModelLaneRecoveryEventRecord, ModelLaneLeaseRecord, ModelLaneDiagnosticTierStatusRecord, and ModelLaneMtRuntimeStatusRecord rows; checkpoint-bounded replay through ModelLaneStore::recover_run_after_restart; payload refs resolved through model_lane_context_bundle_artifacts plus kernel_event_ledger; CRDT base/state-vector validation; checkpoint-bounded failed cloud consent denial receipts; active versus expired lease classification from latest committed current lease authority without widening replay; durable CX-MM-009 orphan_detected events for expired leases including post-checkpoint leases; divergent idempotency rejected; CX-MM-006 and CX-MM-009 failure paths; Flight Recorder-only HBR-INT-009 evidence rejected; manual parity."
                .into(),
        schema_fields: vec![
            "NewModelLaneRecoveryCheckpoint".into(),
            "ModelLaneRecoveryCheckpointRecord".into(),
            "NewModelLaneRecoveryEvent".into(),
            "ModelLaneRecoveryEventRecord".into(),
            "NewModelLaneLease".into(),
            "ModelLaneLeaseRecord".into(),
            "NewModelLaneDiagnosticTierStatus".into(),
            "ModelLaneDiagnosticTierStatusRecord".into(),
            "NewModelLaneMtRuntimeStatus".into(),
            "ModelLaneMtRuntimeStatusRecord".into(),
            "ModelLaneStore::recover_run_after_restart".into(),
            "hsk.model_lane_recovery_checkpoint@1".into(),
            "hsk.model_lane_recovery_event@1".into(),
            "hsk.model_lane_lease@1".into(),
            "hsk.model_lane_diagnostic_tier@1".into(),
            "hsk.model_lane_mt_runtime_status@1".into(),
            "model_lane_recovery_checkpoints".into(),
            "model_lane_recovery_events".into(),
            "model_lane_leases".into(),
            "model_lane_diagnostic_tier_statuses".into(),
            "model_lane_mt_runtime_statuses".into(),
            "kernel_event_ledger".into(),
            "EventLedger".into(),
            "Flight Recorder".into(),
            "model_lane_context_bundle_artifacts".into(),
            "CX-MM-006".into(),
            "CX-MM-009".into(),
            "internal_diagnostics".into(),
            "Palmistry".into(),
        ],
        common_errors: vec![
            "missing_payload_authority".into(),
            "event_ledger_sequence_gap".into(),
            "stale_crdt_base".into(),
            "orphaned_subagent".into(),
            "idempotency conflict".into(),
            "FlightRecorder-only".into(),
        ],
        recovery_steps: vec![
            "Call ModelLaneStore::recover_run_after_restart(run_id) and inspect the checkpoint high-watermark plus recovery_events replay_order_seq.".into(),
            "For CX-MM-006, create or repair a model_lane_context_bundle_artifacts row whose artifact_ref/artifact_payload_ref matches the message payload ref and whose kernel_event_ledger row matches.".into(),
            "For stale CRDT, replay the advisory ModelLaneMessage CRDT base_snapshot_ref and state_vector before retrying recovery.".into(),
            "For orphaned subagents, compare the latest EventLedger-backed current lease authority and lease_expires_at_utc before takeover; do not widen checkpoint replay to discover post-checkpoint leases; use CX-MM-009 for unrecoverable orphan paths.".into(),
            "For HBR-INT-009, require EventLedger/Flight Recorder; internal_diagnostics is WIRED through the native producer and Problems projection, and Palmistry is WIRED through the authenticated watcher and survivor recovery importer.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: model_lane_recovery_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let swarm_lane_diagnostics_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "swarm_lane_diagnostics_runtime_proof",
            "name": "Dexterity lane diagnostics runtime proof",
            "status": "wired",
            "http_routes": [
                "GET /swarm/model-lanes/diagnostics/latest",
                "GET /swarm/model-lanes/diagnostics/{run_id}"
            ],
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test swarm_lane_diagnostics_pg_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test swarm_lane_diagnostics_pg_tests swarm_lane_diagnostics_backend_projection_matches_eventledger -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test swarm_lane_diagnostics_pg_tests swarm_lane_diagnostics_rejects_flight_recorder_only_hbr_posture -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_swarm_lane_diagnostics_argus swarm_lane_diagnostics_argus_lists_filters_and_drills_down -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_swarm_lane_diagnostics_argus swarm_lane_diagnostics_argus_rejects_missing_author_id_and_count_mismatch -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_top_menu_bar run_menu_opens_swarm_lane_diagnostics -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_command_palette typing_diagnostics_filters_to_swarm_lane_diagnostics_and_runs -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_settings_dialog swarm_lane_diagnostics_setting_persists -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_diagnostics_user_manual_entry_is_current -- --exact"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("swarm lane diagnostics tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "swarm_lane_diagnostics_runtime_proof".into(),
        page_id: None,
        name: "Dexterity lane diagnostics runtime proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test swarm_lane_diagnostics_pg_tests".into(),
        ),
        http_route: Some("/swarm/model-lanes/diagnostics/latest".into()),
        http_method: "GET".into(),
        description:
            "Exact Rust proof targets for the Dexterity Lane Diagnostics native pane, backend diagnostics projection, settings/menu/palette paths, and HBR-INT-009 posture."
                .into(),
        expected_input:
            "Real PostgreSQL test URL or Handshake-managed PostgreSQL; test-utils feature enabled; ModelLaneRun/ModelLane/ModelLaneMessage rows; EventLedger rows; diagnostic tier and MT runtime status rows; native Rust app AccessKit/Argus harness."
                .into(),
        expected_output:
            "A native_swarm_lane_diagnostics projection from ModelLaneStore::diagnostics_projection; GET /swarm/model-lanes/diagnostics/latest and GET /swarm/model-lanes/diagnostics/{run_id}; stable AccessKit author IDs for menu.models.swarm-lane-diagnostics, swarm-lane-diagnostics.surface, run/lane/message filters, payload and promotion drilldowns, and settings.swarm-lane-diagnostics-default-open; lanes and messages linked to EventLedger event IDs, EventLedger-backed FlightRecorder correlation IDs and aliases, trace/span/link IDs, CRDT refs, Locus/Loom/FEMS refs, context bundle refs, memory pack refs, artifact refs, HBR-INT-009 tiers, and MT runtime status refs; projection validation rejects missing author IDs, schema_id mismatch, count mismatch, missing payload/EventLedger/FlightRecorder evidence, missing internal_diagnostics/Palmistry tiers, missing HBR tier state, and deferred tiers without follow_up_ref."
                .into(),
        schema_fields: vec![
            "ModelLaneDiagnosticsProjection".into(),
            "SwarmLaneDiagnosticsProjection".into(),
            "SwarmLaneDiagnosticsPaneFactory".into(),
            "SwarmLaneDiagnosticsClient".into(),
            "ModelLaneStore::diagnostics_projection".into(),
            "ModelLaneStore::latest_diagnostics_projection".into(),
            "native_swarm_lane_diagnostics".into(),
            "swarm-lane-diagnostics.surface".into(),
            "swarm-lane-diagnostics.filter.run".into(),
            "swarm-lane-diagnostics.filter.lane".into(),
            "swarm-lane-diagnostics.filter.message".into(),
            "swarmdiagnostics.open".into(),
            "menu.models.swarm-lane-diagnostics".into(),
            "settings.swarm-lane-diagnostics-default-open".into(),
            "GET /swarm/model-lanes/diagnostics/latest".into(),
            "GET /swarm/model-lanes/diagnostics/{run_id}".into(),
            "kernel_event_ledger".into(),
            "EventLedger".into(),
            "model_lane_diagnostic_tier_statuses".into(),
            "model_lane_mt_runtime_statuses".into(),
            "FlightRecorder".into(),
            "Flight Recorder".into(),
            "internal_diagnostics".into(),
            "Palmistry".into(),
            "Locus".into(),
            "Loom".into(),
            "FEMS".into(),
        ],
        common_errors: vec![
            "projection_contract_mismatch".into(),
            "lane_message_count_mismatch".into(),
            "missing_stable_author_id".into(),
            "missing_payload_ref".into(),
            "missing_eventledger_evidence".into(),
            "missing_flightrecorder_correlation".into(),
            "missing_hbr_int_009_tier".into(),
            "schema_id_mismatch".into(),
            "missing_hbr_tier_state".into(),
            "missing_deferred_follow_up_ref".into(),
        ],
        recovery_steps: vec![
            "If the pane shows swarm-lane-diagnostics.error, fetch GET /swarm/model-lanes/diagnostics/{run_id} and inspect the backend projection error first.".into(),
            "If lane/message counts disagree, replay ModelLaneStore::replay_run(run_id) and compare model_lanes.message_count with model_lane_messages rows.".into(),
            "If author IDs are missing, repair the native Rust pane row/drilldown construction before relying on Argus inspection.".into(),
            "If EventLedger or FlightRecorder refs are missing, repair the model lane event linkage instead of treating UI rows as authority.".into(),
            "If HBR posture is suspected to be FlightRecorder-only, run swarm_lane_diagnostics_rejects_flight_recorder_only_hbr_posture before trusting the diagnostics pane.".into(),
            "internal_diagnostics is WIRED through the native producer and Problems projection. Palmistry is WIRED through the authenticated watcher and survivor recovery importer; do not silently skip HBR-INT-009.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: swarm_lane_diagnostics_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let model_lane_navigation_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "model_lane_navigation_api_tests",
            "name": "Dexterity ModelLane navigation runtime proof",
            "status": "wired",
            "http_routes": [
                "GET /swarm/model-lanes/navigation/runs/{run_id}",
                "GET /swarm/model-lanes/navigation/lanes/{lane_id}",
                "GET /swarm/model-lanes/navigation/messages/{message_id}",
                "GET /swarm/model-lanes/navigation/artifacts",
                "GET /swarm/model-lanes/navigation/traces/{trace_id}",
                "GET /swarm/model-lanes/navigation/diagnostics/{run_id}",
                "GET /swarm/model-lanes/navigation/recovery/{run_id}",
                "GET /swarm/model-lanes/navigation/lookup"
            ],
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_navigation_api_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_navigation_api_tests model_lane_navigation_routes_return_run_lane_message_artifact_trace_and_recovery -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_navigation_api_tests model_lane_navigation_user_manual_registry_rows_match_runtime_routes -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_navigation_user_manual_entries_are_current -- --exact"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model-lane navigation tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "model_lane_navigation_api_tests".into(),
        page_id: None,
        name: "Dexterity ModelLane navigation runtime proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test model_lane_navigation_api_tests".into(),
        ),
        http_route: Some("/swarm/model-lanes/navigation/runs/{run_id}".into()),
        http_method: "GET".into(),
        description:
            "Exact Rust proof targets for no-context Dexterity ModelLane navigation routes and UserManual registry parity. \
             Routes: GET /swarm/model-lanes/navigation/runs/{run_id}, \
             GET /swarm/model-lanes/navigation/lanes/{lane_id}, \
             GET /swarm/model-lanes/navigation/messages/{message_id}, \
             GET /swarm/model-lanes/navigation/artifacts, \
             GET /swarm/model-lanes/navigation/traces/{trace_id}, \
             GET /swarm/model-lanes/navigation/diagnostics/{run_id}, \
             GET /swarm/model-lanes/navigation/recovery/{run_id}, \
             GET /swarm/model-lanes/navigation/lookup. \
             Selectors that are not natural route path parameters resolve through the \
             `model_lane.navigation.lookup` lookup_kind (ModelLaneNavigationLookup) with exactly one \
             query selector, served by ModelLaneStore::navigation_by_lookup. Every returned row carries \
             trace_id, span_id, event_ledger_event_id, event_ledger_seq, and error_code, plus EventLedger \
             authority refs and Flight Recorder refs, the HBR-INT-009 internal_diagnostics and Palmistry \
             posture, and Locus, Loom, FEMS, ContextBundle, and MemoryPack refs."
                .into(),
        expected_input:
            "Real PostgreSQL/EventLedger test schema; ModelLaneRun, lane, message, artifact binding, recovery, lease, diagnostic tier, and MT status rows with trace/span, Locus, Loom, FEMS, ContextBundle, MemoryPack, Flight Recorder, and Palmistry refs."
                .into(),
        expected_output:
            "ModelLaneNavigationProjection rows from every navigation route with hsk.model_lane_navigation@1 schema, route_id, lookup_kind, run/lane/message/artifact/context/recovery/diagnostic/MT rows, EventLedger refs, Flight Recorder refs, error codes, recovery routes, UserManual page links, runtime router rows, WP-009 registry rows, and tool/manual parity."
                .into(),
        schema_fields: vec![
            "ModelLaneNavigationProjection".into(),
            "ModelLaneStore::navigation_by_run".into(),
            "ModelLaneStore::navigation_by_lane".into(),
            "ModelLaneStore::navigation_by_message".into(),
            "ModelLaneStore::navigation_by_artifact_or_context".into(),
            "ModelLaneStore::navigation_by_trace".into(),
            "ModelLaneStore::navigation_by_diagnostics".into(),
            "ModelLaneStore::navigation_by_recovery".into(),
            "ModelLaneStore::navigation_by_lookup".into(),
            "ModelLaneNavigationLookup".into(),
            "hsk.model_lane_navigation@1".into(),
            "model_lane.navigation.run".into(),
            "model_lane.navigation.lane".into(),
            "model_lane.navigation.message".into(),
            "model_lane.navigation.artifact_context".into(),
            "model_lane.navigation.trace_span".into(),
            "model_lane.navigation.diagnostic_tier".into(),
            "model_lane.navigation.recovery".into(),
            "model_lane.navigation.lookup".into(),
            "kernel_event_ledger".into(),
            "Flight Recorder".into(),
            "EventLedger".into(),
            "internal_diagnostics".into(),
            "Palmistry".into(),
            "Locus".into(),
            "Loom".into(),
            "FEMS".into(),
            "ContextBundle".into(),
            "MemoryPack".into(),
            "model_session_id".into(),
            "session_id".into(),
            "wp_id".into(),
            "mt_id".into(),
            "task_board_id".into(),
            "event_ledger_event_id".into(),
            "event_ledger_seq".into(),
            "trace_id".into(),
            "span_id".into(),
            "error_code".into(),
        ],
        common_errors: vec![
            "missing artifact_ref or context_bundle_id".into(),
            "unknown run_id/lane_id/message_id/trace_id".into(),
            "missing EventLedger refs".into(),
            "missing Flight Recorder refs".into(),
            "missing UserManual registry row".into(),
            "router 404/405 for documented route".into(),
            "diagnostics projection row drift".into(),
        ],
        recovery_steps: vec![
            "Start with /swarm/model-lanes/navigation/runs/{run_id}, then narrow to lane/message/artifact/trace/diagnostics/recovery routes.".into(),
            "Use event_ledger_refs to inspect kernel_event_ledger before trusting UI rows, provider traces, or chat history.".into(),
            "For artifact recovery, query artifact_ref, artifact_binding_id, artifact_manifest_ref, artifact_payload_ref, artifact_sha256, content_hash, or context_bundle_id through /swarm/model-lanes/navigation/artifacts.".into(),
            "For HBR-INT-009 gaps, compare diagnostic tier rows with model-lane-diagnostics and keep Palmistry as observation-only evidence unless the separate watcher is wired.".into(),
            "If the registry/manual route tests fail, add the route, registry row, page anchor, and tool entry in the same implementation unit.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: model_lane_navigation_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let mixed_model_lane_validation_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "mixed_model_lane_integration_pg_tests",
            "name": "Dexterity mixed-lane validation harness",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests",
            "exact_commands": [
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_model_lane_negative_guards_fail_closed -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_concurrent_model_and_operator_lanes_converge_on_shared_crdt_key -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_midstream_cancellation_preserves_prefix_and_rejects_late_messages -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_real_postgres_yjs_updates_compaction_receipts_and_lane_state_converge -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_yjs_atomic_cross_connection_race_keeps_eventledger_and_crdt_receipts_in_lockstep -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests ac9_bounded_retry_exhaustion_fails_after_three_durable_attempts -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests operator_chat_launch_coordinator_cancellation_preserves_prefix_and_rejects_late_activity -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests coordinator_cancellation_fence_rejects_generation_during_terminal_pg_write -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests coordinator_cancellation_fence_retries_after_terminal_pg_failure -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_swarm_lane_diagnostics_argus mixed_model_lane_run_is_inspectable_through_argus -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_validation_harness_user_manual_entry_is_current -- --exact",
                "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_behavior_coverage_tests mixed_model_lane_behaviors_have_manual_coverage -- --exact"
            ],
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("mixed model lane validation tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "mixed_model_lane_integration_pg_tests".into(),
        page_id: None,
        name: "Dexterity mixed-lane validation harness".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests".into(),
        ),
        http_route: None,
        http_method: "".into(),
        description:
            "Exact Rust proof targets for mixed local/cloud/subagent ModelLaneRun persistence, replay, recovery, diagnostics projection, negative guards, real Yjs atomicity, coordinator-owned cancellation capture, and UserManual behavior coverage."
                .into(),
        expected_input:
            "Real PostgreSQL/EventLedger test schema; deterministic local/cloud/subagent lane fixtures; ProjectionPlan/ConsentReceipt rows; bounded payload artifacts; CRDT base/state-vector refs; recovery checkpoints; diagnostic tier rows; native AccessKit Argus harness."
                .into(),
        expected_output:
            "A replayable mixed ModelLaneRun with backend lane/message counts matching native diagnostics rows; EventLedger IDs/sequences on all authority rows; atomic PostgreSQL/EventLedger Yjs receipts under cross-connection races; coordinator cancellation preserving a captured prefix while rejecting late message/tool/artifact/Flight Recorder activity; recovery from checkpoint without FlightRecorder/provider history; explicit cloud consent denial and stale CRDT/missing payload/direct endpoint failures; hsk.user_manual_behavior_coverage@1 Rust coverage matrix/contract entries covering every model-lane behavior with FlightRecorder/internal_diagnostics/Palmistry posture."
                .into(),
        schema_fields: vec![
            "hsk.user_manual_behavior_coverage@1".into(),
            "ModelLaneStore::replay_run".into(),
            "ModelLaneStore::recover_run_after_restart".into(),
            "ModelLaneStore::diagnostics_projection".into(),
            "model_lane_behavior_coverage_matrix".into(),
            "verify_model_lane_behavior_coverage".into(),
            "kernel_event_ledger".into(),
            "EventLedger".into(),
            "Flight Recorder".into(),
            "native_swarm_lane_diagnostics".into(),
            "ProcessOwnershipLedger".into(),
            "ContextBundle".into(),
            "CRDT".into(),
            "Locus".into(),
            "Loom".into(),
            "FEMS".into(),
        ],
        common_errors: vec![
            "direct_endpoint_bypass".into(),
            "missing_cloud_consent".into(),
            "missing_payload_authority".into(),
            "stale_crdt_base".into(),
            "replay_order_gap".into(),
            "argus_count_mismatch".into(),
            "missing_manual_coverage".into(),
            "FlightRecorder-only".into(),
        ],
        recovery_steps: vec![
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_model_lane_negative_guards_fail_closed -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mixed_concurrent_model_and_operator_lanes_converge_on_shared_crdt_key -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_midstream_cancellation_preserves_prefix_and_rejects_late_messages -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_real_postgres_yjs_updates_compaction_receipts_and_lane_state_converge -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests mt009_yjs_atomic_cross_connection_race_keeps_eventledger_and_crdt_receipts_in_lockstep -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test mixed_model_lane_integration_pg_tests ac9_bounded_retry_exhaustion_fails_after_three_durable_attempts -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests operator_chat_launch_coordinator_cancellation_preserves_prefix_and_rejects_late_activity -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests coordinator_cancellation_fence_rejects_generation_during_terminal_pg_write -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests coordinator_cancellation_fence_retries_after_terminal_pg_failure -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/frontend/handshake_native/Cargo.toml --test test_swarm_lane_diagnostics_argus mixed_model_lane_run_is_inspectable_through_argus -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_api_tests model_lane_validation_harness_user_manual_entry_is_current -- --exact".into(),
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test user_manual_behavior_coverage_tests mixed_model_lane_behaviors_have_manual_coverage -- --exact".into(),
            "Replay ModelLaneStore::replay_run(run_id) before trusting UI row counts.".into(),
            "Use ModelLaneStore::recover_run_after_restart(run_id) to reconstruct checkpoint, recovery events, leases, cloud denial, and MT runtime status from PostgreSQL/EventLedger.".into(),
            "Repair missing payloads by recording model_lane_context_bundle_artifacts rows that bind payload_ref to bounded artifact refs and EventLedger evidence.".into(),
            "Reject stale CRDT bases until state_vector and base_snapshot_ref match the current replay posture.".into(),
            "Repair UserManual gaps by adding canonical UserManual page/tool entries and hsk.user_manual_behavior_coverage@1 Rust contract entries; do not use markdown-only docs as proof.".into(),
        ],
        origin: "wp1_model_lane".into(),
        content_hash: mixed_model_lane_validation_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    // WP-1 MT-013 (AC#5): tool entries backing the embedded-model lifecycle
    // ledger + fail-closed/embedding Flight Recorder behavior coverage rows.
    let embedded_model_ledger_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "embedded_model_ledger_tests",
            "name": "Embedded model ProcessOwnershipLedger START/STOP + orphan-reconcile proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test embedded_model_ledger_tests",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("embedded model ledger tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "embedded_model_ledger_tests".into(),
        page_id: None,
        name: "Embedded model ProcessOwnershipLedger START/STOP + orphan-reconcile proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test embedded_model_ledger_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
             "Exact Rust proof targets for the embedded-model ProcessOwnershipLedger obligation: all-or-none pre-artifact START+STOP reservation, store-acknowledged pid-less START on load (normally keyed on the minted UUIDv7, with a distinct quarantine UUID for invalid/duplicate returned identities), a pre-reserved STOP emitted only after worker quiescence and offered to a bounded background-writer drain, and OS-loopback-lease-aware hard-crash reconciliation when graceful durability cannot be proven."
                .into(),
        expected_input:
            "test-utils feature enabled; controlled in-process ModelRuntime barriers for the deterministic lifecycle legs; real Handshake-managed PostgreSQL (127.0.0.1:5544) or POSTGRES_TEST_URL isolated schema for the orphan-reconcile leg."
                .into(),
        expected_output:
             "A store-acknowledged pid-less ProcessOwnershipLedger START row (os_pid=NULL; valid path process_uuid == model UUIDv7; identity-contract failures use a distinct quarantine UUID and metadata), a matching pre-reserved STOP emitted via LlmClient::shutdown_gracefully only after the exact runtime workers exit and durably flushed on a successful drain-and-join, plus open-START reconciliation when shutdown or drain cannot prove success."
                .into(),
        schema_fields: vec![
            "EmbeddedModelProcess::record_reserved_load_with_durable_ack".into(),
            "EmbeddedModelProcess::shutdown".into(),
            "LlmClient::shutdown_gracefully".into(),
            "LlmClient::leave_open_for_reconciliation".into(),
            "drain_and_join_ledger_writer".into(),
            "reclaim_pidless_embedded_orphans".into(),
            "acquire_embedded_runtime_instance_lease".into(),
            "kernel_process_lifecycle".into(),
        ],
        common_errors: vec![
            "missing_start_row".into(),
            "missing_stop_row".into(),
            "synthetic_pid_forbidden".into(),
            "orphan_not_reconciled".into(),
            "reserved_stop_retained_batch_loss".into(),
            "runtime_quiescence_timeout".into(),
            "orphan_reclaim_lock_timeout_deferred".into(),
            "orphan_reclaim_instance_cap_deferred".into(),
            "orphan_reclaim_unsafe_metadata_deferred".into(),
            "token_stream_terminal_missing_under_backpressure".into(),
        ],
        recovery_steps: vec![
            "If the STOP row is missing after Ctrl-C/SIGTERM, confirm Axum completed its connection drain, the exact runtime worker barriers reached idle, and the process-ledger drain completed before managed PostgreSQL stop and final OS-lease release. Connection-drain or quiescence timeout intentionally leaves START open for next-boot reconciliation rather than reporting a graceful STOP.".into(),
            "If an orphan persists, inspect runtime_instance_schema_id, runtime_instance_id, runtime_host_scope_id, runtime_lease_protocol, runtime_lease_address, and runtime_lease_port. Reconciliation intentionally skips missing, malformed, foreign-host, conflicting, or address-in-use evidence; never force-close it from age alone.".into(),
            "If boot reports deferred orphan reconciliation, inspect the typed lock-timeout and instance-cap fields. Leave the START open, remove the contending database transaction if appropriate, and allow a later bounded boot sweep to retry; never rewrite terminal columns by hand.".into(),
        ],
        origin: "wp1_mt013_embedded_model".into(),
        content_hash: embedded_model_ledger_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let candle_real_load_ledger_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "candle_e2e_smoke::mt013_real_candle_default_load_emits_process_ledger_start_stop",
            "name": "MT-013 real Candle default-load ProcessOwnershipLedger START/STOP proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features \"test-utils,candle-runtime-engine\" --test candle_e2e_smoke mt013_real_candle_default_load_emits_process_ledger_start_stop -- --ignored --exact --nocapture",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("real Candle ledger tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id:
            "candle_e2e_smoke::mt013_real_candle_default_load_emits_process_ledger_start_stop"
                .into(),
        page_id: None,
        name: "MT-013 real Candle default-load ProcessOwnershipLedger START/STOP proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features \"test-utils,candle-runtime-engine\" --test candle_e2e_smoke mt013_real_candle_default_load_emits_process_ledger_start_stop -- --ignored --exact --nocapture".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Non-skipping MT-013 managed-resource proof for the default embedded LlmClient path: it loads real Candle weights through build_default_local_client, drains the ProcessOwnershipLedger, calls LlmClient::shutdown_gracefully to quiesce real workers before STOP, drains again, and prints [MT-013_REAL_CANDLE_LEDGER_DUMP] containing the matching pid-less START/STOP rows."
                .into(),
        expected_input:
            "Features test-utils,candle-runtime-engine enabled; HANDSHAKE_TEST_CANDLE_MODEL_DIR set to a directory containing real model.safetensors and tokenizer.json; external CARGO_TARGET_DIR under ..\\Handshake_Artifacts\\handshake-cargo-target."
                .into(),
        expected_output:
            "Exactly one real Candle START row keyed on the LlmClient profile UUIDv7, os_pid=NULL with os_pid_absent_reason=in_process_library_load_no_os_process, model_artifact_sha256 matching the real artifact, then exactly one STOP row with stop_reason=llm-client-shutdown; missing model env fails loudly, not skipped."
                .into(),
        schema_fields: vec![
            "build_default_local_client".into(),
            "CandleRuntime::load".into(),
            "EmbeddedModelProcess::record_reserved_load_with_durable_ack".into(),
            "LlmClient::shutdown_gracefully".into(),
            "MT-013_REAL_CANDLE_LEDGER_DUMP".into(),
        ],
        common_errors: vec![
            "HANDSHAKE_TEST_CANDLE_MODEL_DIR_unset".into(),
            "missing_model_safetensors".into(),
            "missing_tokenizer_json".into(),
            "missing_real_start_row".into(),
            "missing_real_stop_row".into(),
        ],
        recovery_steps: vec![
            "Set HANDSHAKE_TEST_CANDLE_MODEL_DIR to a real Candle model directory and rerun the exact ignored test command.".into(),
            "If START is missing after load, inspect EmbeddedModelProcess::record_reserved_load_with_durable_ack and the supplied LedgerBatcher; the default path must fail closed rather than continue unledgered.".into(),
            "If STOP is missing after shutdown, inspect LocalModelRuntimeLlmClient::shutdown_gracefully, runtime quiescence, and the manual/spawned ledger drain before moving MT-013 to READY_FOR_VALIDATION.".into(),
        ],
        origin: "wp1_mt013_embedded_model".into(),
        content_hash: candle_real_load_ledger_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    // WP-1 MT-012: operator chat/launch capture proof tool entry backing the
    // operator-chat behavior coverage rows.
    let operator_chat_capture_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "operator_chat_capture_tests",
            "name": "Operator chat/launch: spawn_session launch + CLI-capture -> ModelLaneMessage proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("operator chat capture tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "operator_chat_capture_tests".into(),
        page_id: None,
        name: "Operator chat/launch: spawn_session launch + CLI-capture -> ModelLaneMessage proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test operator_chat_capture_tests".into(),
        ),
        http_route: Some("/operator-chat/launch".into()),
        http_method: "POST".into(),
        description:
            "Exact Rust proof targets for the MT-012 operator chat/launch surface: a non-mocked lane launch through SwarmCoordinator::spawn_session persists ModelLaneRun/ModelLane; a realistic multi-line stream-json turn yields exactly ONE ModelLaneMessage per completed activity block (ToolCall->ToolRequest, rendered tool_result->ToolResult, prompt/answer/thought->Status discriminated by diagnostic_payload.activity_kind) with a matching FR-EVT-AGENT-* event; the operator prompt is a HUMAN_OPERATOR message; the selection decision emits FR-EVT-MODEL-SELECTION-RECORDED; a launch without a ModelLaneStore fails closed; the operator working_dir is the real CLI subprocess cwd."
                .into(),
        expected_input:
            "test-utils feature enabled; real Handshake-managed PostgreSQL (127.0.0.1:5544) or an isolated POSTGRES_TEST_URL schema; the real parse_agent_activity_line parser + a capturing FlightRecorder; the LiveCliSpawner for the real-cwd leg."
                .into(),
        expected_output:
            "One ModelLaneMessage per completed activity block persisted + replayable under the run; typed kinds and activity_kind labels; a capturing recorder holding FR-EVT-AGENT-* and FR-EVT-MODEL-SELECTION-RECORDED events; a fail-closed LedgerFailed error when the coordinator has no ModelLaneStore; a launched subprocess whose cwd equals the operator selection."
                .into(),
        schema_fields: vec![
            "OperatorChatLaunchService::launch".into(),
            "ModelLaneCaptureRecorder::capture_cli_stream".into(),
            "ModelLaneCaptureRecorder::record_operator_prompt".into(),
            "ModelLaneStore::record_message".into(),
            "cli_bridge_config_with_working_dir".into(),
            "flight_recorder".into(),
        ],
        common_errors: vec![
            "launch_not_fail_closed_without_store".into(),
            "duplicate_message_per_delta_line".into(),
            "thought_coerced_to_unlabelled_status".into(),
            "working_dir_not_applied_to_subprocess".into(),
        ],
        recovery_steps: vec![
            "If more than one message is recorded for a streaming turn, confirm capture iterates parse_agent_activity_line output (one per completed block) and not raw delta lines.".into(),
            "If a launch without a ModelLaneStore does not error, confirm the coordinator was built with new_with_model_lane_store only for the positive path and that spawn_session's fail-closed guard is reached.".into(),
        ],
        origin: "wp1_mt012_operator_chat".into(),
        content_hash: operator_chat_capture_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let llm_local_routing_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "llm_client_local_routing_tests",
            "name": "LlmClient fail-closed + embedding Flight Recorder proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test llm_client_local_routing_tests",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("llm local routing tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "llm_client_local_routing_tests".into(),
        page_id: None,
        name: "LlmClient fail-closed + embedding Flight Recorder proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test llm_client_local_routing_tests".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Exact Rust proof targets for master-spec §4.2.3.2(3) Flight Recorder emission on EVERY LlmClient call path: fail-closed DisabledLlmClient::completion and ::embedding, local completion error branches, and the embedding lane (success + error) — all emitted at CALL TIME, never at construction."
                .into(),
        expected_input:
            "test-utils feature enabled; a capturing FlightRecorder; configurable in-process ModelRuntime and a recorder-wired DisabledLlmClient fallback."
                .into(),
        expected_output:
            "One Flight Recorder event per call: zeroed-usage llm_inference events (error_kind llm_disabled / llm_error / embedding_disabled) on error/disabled paths and data_embedding_computed on embedding success; no construction-time emission."
                .into(),
        schema_fields: vec![
            "DisabledLlmClient::completion".into(),
            "DisabledLlmClient::embedding".into(),
            "LocalModelRuntimeLlmClient::embedding".into(),
            "emit_llm_call_error_event".into(),
            "FlightRecorderEventType::LlmInference".into(),
            "FlightRecorderEventType::DataEmbeddingComputed".into(),
        ],
        common_errors: vec![
            "missing_fr_event_on_error_path".into(),
            "construction_time_emit_false_green".into(),
            "embedding_unsupported_silent".into(),
        ],
        recovery_steps: vec![
            "If an error path emits no FR event, route it through emit_llm_call_error_event at call time.".into(),
            "If DisabledLlmClient::embedding is silent, confirm the ::embedding override emits before returning EmbeddingUnsupported.".into(),
        ],
        origin: "wp1_mt013_embedded_model".into(),
        content_hash: llm_local_routing_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    let dedicated_embedding_tool_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "dedicated_embedding_model_tests",
            "name": "Dedicated embedding model routing proof",
            "status": "wired",
            "cli_flag": "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test loom_search_v2_tests mt016_loom_search_routes_reindex_and_search_to_registry_embedding_model -- --exact",
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("dedicated embedding model tool serializes"),
    );
    tools.push(UserManualToolEntry {
        tool_id: "dedicated_embedding_model_tests".into(),
        page_id: None,
        name: "Dedicated embedding model routing proof".into(),
        status: "wired".into(),
        ipc_channel: None,
        tauri_command: None,
        cli_flag: Some(
            "cargo test --target-dir ..\\Handshake_Artifacts\\handshake-cargo-target --manifest-path src/backend/handshake_core/Cargo.toml --features test-utils --test loom_search_v2_tests mt016_loom_search_routes_reindex_and_search_to_registry_embedding_model -- --exact".into(),
        ),
        http_route: None,
        http_method: String::new(),
        description:
            "Representative Rust proof target plus machine-readable references for the full MT-016 proof suite: ModelCapabilities consistency, ready catalog selection by embedding dimension, shared boot registry with distinct chat and embedding registrations, LoomSearchV2 reindex/search using the stable embedding-space id, no chat embedding fallback, and UserManual behavior coverage."
                .into(),
        expected_input:
            "test-utils feature enabled; in-process fake ModelRuntime for boot proof; real Handshake-managed PostgreSQL or POSTGRES_TEST_URL isolated schema for LoomSearchV2 storage proof; model catalog fixtures with chat-only and embedding-capable registrations."
                .into(),
        expected_output:
            "The chat/completion model remains profile().model_id; ModelCatalog selects the READY supports_embedding=true embedding_dimension=768 registration; LocalRouter rejects chat UUIDv7 embedding calls before runtime dispatch; LoomSearchV2 routes calls with the dedicated embedding model UUID but stores and queries with the stable embedding-space id; PostgreSQL vector scoring ignores rows from another embedding space; no-model fallback does not call the chat model."
                .into(),
        schema_fields: vec![
            "ModelCapabilities::supports_embedding".into(),
            "ModelCapabilities::embedding_dimension".into(),
            "ModelCatalogEntry::embedding_space_id".into(),
            "ModelCatalog::embedding_model_for_dim".into(),
            "LocalRouter::require_embedding_model".into(),
            "LoomSearchV2Request::query_embedding_model".into(),
            "loom_block_search_index.embedding_model".into(),
            "SemanticUnavailableReason::NoModel".into(),
            "FR-EVT-LOOM-SEMANTIC-DEGRADED".into(),
            "proof:model_registry_tests::mt016_model_capabilities_declare_embedding_dimension_and_validate_consistency".into(),
            "proof:model_catalog_tests::mt016_catalog_selects_ready_embedding_capable_model_distinct_from_chat".into(),
            "proof:llm_default_boot_resolution_tests::mt016_default_boot_registers_distinct_embedding_model_when_configured".into(),
            "proof:loom_search_v2_tests::mt016_loom_search_routes_reindex_and_search_to_registry_embedding_model".into(),
            "proof:loom_search_v2_tests::mt016_loom_search_no_embedding_model_degrades_without_chat_embedding_call".into(),
            "proof:user_manual_behavior_coverage_tests::dedicated_embedding_model_behaviors_have_manual_coverage".into(),
        ],
        common_errors: vec![
            "chat_model_used_for_embedding".into(),
            "missing_embedding_dimension".into(),
            "wrong_embedding_dimension".into(),
            "cross_model_vector_space_scored".into(),
            "semantic_degrade_silent".into(),
        ],
        recovery_steps: vec![
            "Configure HANDSHAKE_LOCAL_EMBEDDING_MODEL_PATH and HANDSHAKE_LOCAL_EMBEDDING_MODEL_SHA256 for the dedicated embedding artifact.".into(),
            "Set HANDSHAKE_LOCAL_EMBEDDING_MODEL_DIMENSION to 768 or leave it unset for the 768 default.".into(),
            "Reindex Loom blocks after changing the embedding model so loom_block_search_index.embedding_model matches the active model.".into(),
            "If semantic search is unavailable, inspect the catalog for a READY supports_embedding=true embedding_dimension=768 row before debugging PostgreSQL.".into(),
        ],
        origin: "wp1_mt016_dedicated_embedding_model".into(),
        content_hash: dedicated_embedding_tool_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

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
        SurfaceGroup::ModelLaneNavigation,
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

    let tool_ids = vec![
        "model_lane_schema_pg_tests".to_string(),
        "model_lane_launch_tests".to_string(),
        "official_cli_attached_lifecycle_tests".to_string(),
        "model_lane_promotion_pg_tests".to_string(),
        "model_lane_context_bundle_pg_tests".to_string(),
        "cloud_model_lane_policy_pg_tests".to_string(),
        "model_lane_recovery_pg_tests".to_string(),
        "swarm_lane_diagnostics_runtime_proof".to_string(),
        "mixed_model_lane_integration_pg_tests".to_string(),
    ];
    let title = "WP-1 Dexterity model-lane launch, storage, promotion, handoff, cloud consent, recovery, and diagnostics"
        .to_string();
    let description = "Dexterity ModelLaneRun/ModelLane/ModelLaneMessage storage, launch adapter normalization, advisory-to-authority promotion decisions, ContextBundle model-to-model handoffs, durable cloud ProjectionPlan/ConsentReceipt policy, checkpoint/EventLedger recovery, native lane diagnostics, runtime proof, and operator manual coverage.".to_string();
    let content_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "wp1.dexterity_model_lane",
            "title": title,
            "description": description,
            "tool_ids": tool_ids,
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("model lane feature serializes"),
    );
    features.push(UserManualFeatureEntry {
        feature_id: "wp1.dexterity_model_lane".into(),
        title,
        description,
        tool_ids,
        origin: "wp1_model_lane".into(),
        content_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

    // WP-1 MT-014: the shared enumerable/labeled model catalog, the LoomSearchV2
    // embedding-dimension degrade-not-error contract, and the Work Profiles
    // provider_ref resolver — the surfaces a no-context model needs to list the
    // configured local model, understand why a semantic search degraded, and
    // resolve a legacy provider_ref.
    let tool_ids = vec![
        "mt014_catalog_enumerates_and_labels_configured_model".to_string(),
        "mt014_catalog_empty_registry_is_empty_list".to_string(),
        "mt014_catalog_unknown_model_id_sentinel_label".to_string(),
        "mt014_catalog_records_selection_decision_event".to_string(),
        "mt014_persistent_registry_survives_restart_and_reads_back_selection".to_string(),
        "mt014_concurrent_incompatible_adapter_selection_has_one_winner".to_string(),
        "mt014_display_name_change_preserves_selection_and_revision".to_string(),
        "mt014_primary_and_embedding_registration_is_atomic_on_conflict".to_string(),
        "mt014_registry_authority_shape_rejects_semantic_drift".to_string(),
        "mt014_registry_rejects_eventledger_chain_and_immutable_row_tampering".to_string(),
        "mt014_non_advisory_row_lock_times_out_without_registry_or_audit_mutation".to_string(),
        "mt014_registry_api_joins_real_pg_rows_to_current_ready_catalog_by_sha256".to_string(),
        "mt014_registry_api_rejects_ready_catalog_capability_drift".to_string(),
        "mt014_registry_api_rejects_unloaded_catalog_row_without_durable_authority".to_string(),
        "mt014_registry_api_rejects_duplicate_ready_and_unloaded_catalog_sha".to_string(),
        "mt014_registry_api_rejects_unloaded_catalog_adapter_drift".to_string(),
        "mt014_registry_api_rejects_ready_uuid_without_committed_observation".to_string(),
        "mt014_selection_post_prevalidates_then_returns_audited_projection".to_string(),
        "mt014_selection_post_audit_failure_preserves_prior_selection".to_string(),
        "mt014_selection_post_rejects_stale_target_before_swap".to_string(),
        "mt014_selection_post_rejects_embedding_role_before_swap".to_string(),
        "mt014_selection_post_integrity_failure_occurs_before_swap".to_string(),
        "model_runtime_selection_failure_recovery_rows_match_compiled_api_contract".to_string(),
        "mt014_stable_switch_author_id_posts_then_reobserves_backend_projection".to_string(),
        "mt014_embedding_role_row_has_no_default_switch_action".to_string(),
        "mt014_argus_renders_real_pg_live_and_dormant_registry_rows".to_string(),
        "mt014_argus_operator_menu_fetches_real_pg_projection_through_production_transport"
            .to_string(),
        "mt014_model_runtime_real_pg_frame_png".to_string(),
        "run_menu_opens_real_model_runtime_pane".to_string(),
        "mt014_dim_mismatch_degrades_not_errors_on_reindex_and_search".to_string(),
        "mt014_provider_ref_migrates_ollama_to_local_runtime".to_string(),
    ];
    let title = "WP-1 MT-014 durable model-runtime registry, shared catalog, Loom embedding-dim degrade, and provider_ref resolver".to_string();
    let description = concat!(
        "PURPOSE: persist the selected embedded runtime adapter in PostgreSQL, then expose the ",
        "same successfully loaded registration through one shared, enumerable, labeled live ",
        "catalog. STARTUP: the normal handshake_core binary starts/adopts managed PostgreSQL, ",
        "runs the migration chain (including `0348_model_runtime_registry.sql` and `0356_model_runtime_role_authority.sql`), and passes that ",
        "pool into `ModelRegistryStore` before resolving a configured local model. No separate ",
        "registry daemon or operator-started database is required. INPUTS: ",
        "`HANDSHAKE_LOCAL_MODEL_PATH`, `HANDSHAKE_LOCAL_MODEL_SHA256`, ",
        "`HANDSHAKE_LOCAL_MODEL_BINDING`, and optional `HANDSHAKE_LOCAL_MODEL_NAME`; the ",
        "equivalent `HANDSHAKE_LOCAL_EMBEDDING_MODEL_*` variables add the MT-016 dedicated ",
        "embedding registration. The artifact SHA-256 is the durable, relocation-safe identity; ",
        "the path and display/base-model label are current-boot observation metadata. WORKFLOW: ",
        "DECLARED PROOF TARGETS are not executed by UserManual coverage validation; the tool_ids ",
        "below are exact targets to run separately, and an executed verdict requires the Cargo ",
        "test result or its durable proof receipt. ",
        "production boot verifies the registry schema and recovers the configured immutable ",
        "artifact-to-adapter/capability/runtime-role selection before reading model weights. It loads through ",
        "the existing in-process Candle/llama.cpp ModelRuntime, atomically persists and reads ",
        "back the complete primary-plus-embedding boot set, appending ",
        "`KernelEventType::ModelRuntimeSelectionRecorded` as the typed EventLedger evidence for ",
        "each persistent adapter selection, and only then exposes the client and ",
        "live `ModelCatalog`. OUTPUTS: each PostgreSQL row carries `schema_id`, stable artifact ",
        "locator/hash, selected runtime binding, explicit runtime role/default eligibility and capabilities, selection revision, mutable ",
        "last-observed runtime UUID/label/actor/timestamp, and the EventLedger audit reference. ",
        "Every read validates the complete causation-linked EventLedger selection chain, canonical ",
        "payload hashes, revisions, endpoints, and immutable-selection continuity in the same ",
        "PostgreSQL snapshot. A normal restart or display-name/path change preserves selection ",
        "revision; a conflicting adapter/capability choice fails closed, including any conflicting runtime-role choice. PostgreSQL separately owns `application/default` and `embeddings/default` by stable artifact SHA-256; boot restores both before routing exposure. The panel switches only ",
        "`application/default` between current READY completion-role models; an embedding-role row may own `embeddings/default` but is not eligible for that application switch and is excluded from Operator Chat default-model inventory. It never rewrites durable artifact-to-adapter binding. A ",
        "non-active READY row posts `POST /model-runtime/selection`. The local runtime serializes ",
        "the SwapRequest after the API prevalidates the durable/catalog projection, current selection, and target UUIDv7/READY/default-selectable state, resolves ",
        "the actual runtime, appends the active-selection EventLedger record and compare-and-set in one PostgreSQL transaction, then publishes the committed model to the current router projection and cancels old-default in-flight requests. Success returns `selection_receipt_ref`. Audit failure, PostgreSQL revision conflict, stale ",
        "invalid target_model_id, actor, or reason input, stale current selection, non-READY or embedding-role target, integrity failure, or timeout keeps the prior active model. NAVIGATION: in the Rust-native app choose ",
        "`RUN` then `Open Model Runtime`; the action switches through the existing `STUDIO` module ",
        "workflow and activates its `Model Runtime` surface. The equivalent direct path is `STUDIO` ",
        "then `Model Runtime`; ",
        "the pane fetches `GET /model-runtime/registry` off the frame thread and exposes stable ",
        "AccessKit author ids under `model-runtime.registry.*` for parallel model inspection. ",
        "Refresh re-reads the durable `hsk.model_runtime_registry_projection@3` projection. Rows show active purposes/revision, model id, canonical artifact path plus SHA-256, selected adapter/runtime state, KV bytes/cap/hit rate/quantization, ordered LoRA ids/strengths, typed steering availability, ProcessOwnershipLedger link, tokens/s, VRAM, last call, action availability/reasons, selection ",
        "revision, artifact SHA-256 locator, audit reference, and `LIVE / READY` versus ",
        "`DORMANT`; a dormant row never exposes its last-observed boot UUID as currently loaded. ",
        "The backend rejects any READY or unloaded catalog row that has no durable row, duplicates ",
        "an artifact SHA-256, or disagrees on adapter, persisted runtime role/default eligibility, or catalog-visible embedding capabilities. A ",
        "READY UUID and label must also equal the row's atomically committed last observation. ",
        "Only READY rows expose `LIVE` plus a current UUID; a matching unloaded row remains ",
        "`DORMANT` with no UUID. `AppState::model_catalog()` returns the live ",
        "projection when the configured client exposes one; `ModelCatalog::list()` enumerates ",
        "`{model_id (per-boot UUIDv7), display_name/base_model_tag, artifact_sha256 (stable), ",
        "runtime_binding, runtime_role, default_selectable, embedding capability/dimension, ready}`. `label_for(model_id)` returns ",
        "the 'unknown model' sentinel for an unknown id (never panic/blank); an empty registry ",
        "lists nothing. `ModelCatalog::record_selection_decision` separately emits ",
        "`FR-EVT-MODEL-SELECTION-RECORDED` for interactive picker decisions. FAILURE/RECOVERY: ",
        "missing migration, malformed schema/constraints/nullability, PostgreSQL failure (`503 MODEL_RUNTIME_REGISTRY_UNAVAILABLE`), ",
        "selection conflict, invalid EventLedger selection chain, or partial primary/embedding conflict blocks ",
        "model exposure and returns a typed disabled-client reason. Restore the current migration ",
        "chain/database authority and restore configuration to the persisted SHA/binding. Inspect ",
        "the durable selection revision and audit reference to diagnose a conflict; never edit or ",
        "delete database rows, event refs, or revisions to bypass it. Moving ",
        "the project or model file needs only a valid current path with the same artifact hash. ",
        "LOOM CONTRACT: when the ",
        "configured model returns an embedding whose dimensionality != 768, LoomSearchV2 ",
        "reindex and search DEGRADE to keyword/trigram (they do NOT hard-error or 400): they ",
        "emit `FR-EVT-LOOM-SEMANTIC-DEGRADED` and set the response's typed ",
        "`semantic_unavailable_reason = DimMismatch{expected, actual}` so the drop is never ",
        "silent; recovery is to configure the MT-016 dedicated embedding model with the required ",
        "dimension. Work Profiles `provider_ref` resolves against the canonical ",
        "provider id set (`local_runtime`, `openai_compat`); the retired `ollama` id migrates ",
        "deterministically to `local_runtime`, surfaced via an `FR-EVT-PROFILE-` event (never a ",
        "silent rewrite); an unrecognized provider_ref resolves to a typed Unknown. ",
        "HBR-INT-009 posture: PostgreSQL/EventLedger authority and Tier-1 Flight Recorder events ",
        "are WIRED for durable adapter selection inspection, interactive selection, semantic ",
        "degrade, and provider_ref migration; Tier-2 internal_diagnostics is WIRED through the native producer and Problems projection; ",
        "Tier-3 Palmistry is WIRED through the authenticated watcher and survivor recovery importer.",
        " The coverage validator checks this typed-event and WIRED declaration against ",
        "compiled canonical anchors and the deployed UserManual row; it does not claim to query ",
        "live EventLedger or diagnostic-tier rows."
    )
    .to_string();
    let content_hash = sha256_hex(
        &serde_json::to_string(&json!({
            "id": "wp1.mt014_model_catalog_and_loom_degrade",
            "title": title,
            "description": description,
            "tool_ids": tool_ids,
            "manual_version": USER_MANUAL_VERSION,
        }))
        .expect("mt014 feature serializes"),
    );
    features.push(UserManualFeatureEntry {
        feature_id: "wp1.mt014_model_catalog_and_loom_degrade".into(),
        title,
        description,
        tool_ids,
        origin: "wp1_mt014".into(),
        content_hash,
        manual_version: USER_MANUAL_VERSION.into(),
    });

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
    let version_metadata_changed = existing_version.as_ref().is_some_and(|row| {
        row.seed_content_hash != seed_hash
            || row.page_count != corpus.pages.len() as i32
            || row.tool_count != corpus.tools.len() as i32
            || row.feature_count != corpus.features.len() as i32
    });
    let version_receipt =
        if anything_changed || existing_version.is_none() || version_metadata_changed {
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
                            "version_metadata_changed": version_metadata_changed,
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
    fn every_registry_surface_group_page_slug_exists_in_the_corpus() {
        let corpus = seed_corpus();
        let slugs = corpus
            .pages
            .iter()
            .map(|page| page.slug.as_str())
            .collect::<BTreeSet<_>>();

        for surface in wp009_surface_registry() {
            assert!(
                slugs.contains(surface.group.page_slug()),
                "registry surface {} maps to missing UserManual page slug {}",
                surface.surface_id,
                surface.group.page_slug()
            );
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
    fn cloud_projection_consent_page_documents_operator_chat_artifact_binding_order() {
        let corpus = seed_corpus();
        let page = corpus
            .pages
            .iter()
            .find(|page| page.slug == "model-lane-cloud-projection-consent")
            .expect("cloud ProjectionPlan/ConsentReceipt page is seeded");
        let body = page
            .sections
            .iter()
            .map(|section| section.body_md.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        for required in [
            "operator-chat cloud launches precompute deterministic ArtifactStore refs",
            "cloud-input.json",
            "cloud-projection-payload.json",
            "after `spawn_session` records the ModelLaneRun",
            "before output capture",
            "ModelLaneStore::record_context_bundle_artifact_binding",
        ] {
            assert!(
                body.contains(required),
                "cloud ProjectionPlan/ConsentReceipt page must document {required}"
            );
        }
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
