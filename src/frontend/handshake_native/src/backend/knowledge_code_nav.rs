//! WP-KERNEL-012 MT-039 (E6 — backend reuse wiring): the CONSOLIDATED typed native Rust client for the
//! EXISTING handshake_core `/knowledge/code/*` HTTP surface (WP-KERNEL-009 MT-106 `CodeNavigationApi`,
//! backend `api::knowledge_code_nav`). FRONTEND WIRING ONLY — it binds the routes the backend already
//! serves; it does NOT change the backend (a backend gap is a typed blocker, never a backend edit).
//!
//! ## Relationship to the MT-008 [`crate::code_editor::code_nav`] client (REUSE-NOT-FORK)
//!
//! A prior MT (MT-008) already bound FOUR of these routes through a `serde_json::Value`-based,
//! graceful-degradation client living in [`crate::code_editor::code_nav`] (lookup / get_symbol /
//! references / file_lens), which calls [`crate::backend_client::code_nav_get`] and turns any backend
//! error into EMPTY results so the live completion/hover path keeps working when the backend is down.
//! That is the right shape for the inline editor fast-path, but it (a) speaks loose `Value` rather than
//! typed structs, and (b) does NOT bind `tests` or `spans`.
//!
//! This MT-039 module is the SAME `/knowledge/code/*` API (verified: both bind the identical route
//! prefix against `api::knowledge_code_nav`) but a DISTINCT, fully-typed client mirroring MT-037
//! ([`crate::backend::knowledge_documents`]) / MT-038: it binds ALL SIX routes (adding `tests` +
//! `spans`), returns typed response structs with PRESERVED audit-receipt ids, and surfaces typed
//! errors instead of swallowing them. It does NOT fork a second divergent symbol-lookup with different
//! match semantics — it reuses the SAME shared transport plumbing
//! ([`crate::backend_client::shared_http_client`] + [`crate::backend_client::BACKEND_BASE_URL`] + the
//! canonical `x-hsk-*` header constants) so there is ONE connection pool and ONE set of header names.
//! The MT-008 `Value` client remains the inline editor fast-path; THIS client is the typed binding the
//! panel/audit layer consumes.
//!
//! ## Identity headers (REUSE MT-037 `HskDocumentHeaders` — NO 4th copy)
//!
//! All six routes REQUIRE the same backend-navigation identity contract as `/knowledge/documents/*`
//! (verified against `api::knowledge_code_nav::nav_context`): a missing required header is a hard HTTP
//! 400 (`"<header> header is required"`). Rather than define a fourth header struct, this module REUSES
//! the canonical [`HskDocumentHeaders`] from MT-037 and adds a nav-specific builder
//! ([`code_nav_headers`]) that defaults `actor_kind` to `system` — the verified-valid kind the backend's
//! own quiet-nav lane uses for an automated UI navigation (`KernelActor::System`), distinct from the
//! `operator` kind a document WRITE asserts. The five fields map 1:1 to the backend constants:
//!   * `actor_id`           -> `x-hsk-actor-id`            (required)
//!   * `kernel_task_run_id` -> `x-hsk-kernel-task-run-id`  (required)
//!   * `session_run_id`     -> `x-hsk-session-run-id`      (required)
//!   * `actor_kind`         -> `x-hsk-actor-kind`          (optional; `system` for nav)
//!   * `correlation_id`     -> `x-hsk-correlation-id`      (optional)
//!
//! ## Do NOT fabricate identity (stateless adapter)
//!
//! This module holds NO navigation state. Every function takes typed request arguments + an
//! [`HskDocumentHeaders`] the CALLER builds from the current editor session context (see
//! [`EditorSessionContext`]); a call site never fabricates `actor_id` / `session_id`. State (the open
//! symbol, the loaded lens) lives in the editor/panel layer that calls this.
//!
//! ## Verification provenance (SPEC-REALISM GATE)
//!
//! Every route + response shape below was VERIFIED READ-ONLY against the REAL backend source
//! `src/backend/handshake_core/src/api/knowledge_code_nav.rs` (the `routes()` table + the per-handler
//! `json!` bodies) and `src/backend/handshake_core/src/knowledge_code_index/{monaco_bridge,staleness}.rs`
//! — NOT taken from the MT contract prose. The verified facts that shape this client:
//!   * 6 routes (`knowledge_code_nav.rs:87-101`): symbols lookup, get, references, tests, spans, files
//!     lens.
//!   * `lookup_symbols` 200 body (`:488-493`): `{ workspace_id, matches: [SymbolNavProjection],
//!     nav_receipt_event_id, quiet_background_work_receipt_id }`. A query with NONE of name/prefix/path
//!     is a backend 400 (`:423-427`) — so this client pre-flight-GUARDS it ([`CodeNavError::EmptyLookup`])
//!     BEFORE the round-trip (RISK-2/MC-2). `limit` is server-clamped to 1..=500 (`:428`); this client
//!     clamps the same range CLIENT-SIDE (RISK-6/MC-5) so a caller never silently over-fetches.
//!   * `SymbolNavProjection` (`symbol_to_json`, `:387-397`): `{ symbol_entity_id, symbol_key,
//!     display_name, symbol_kind, owning_wp?, primary_source_id?, lifecycle_state, definition?,
//!     staleness }`. `definition` is the first `ast` span or JSON `null` (`:367-385`).
//!   * The `staleness` object (`served_staleness`, `:325-352`) is `{ state: String, fresh: bool, ... }`
//!     with VARIABLE extra fields per state (`indexed_content_hash`/`indexed_parser_version` for fresh /
//!     marked_stale; `detail` for unindexed/failed/unknown). [`StalenessState`] is therefore a TOLERANT
//!     `state: String` (NEVER a closed enum) with every optional field `#[serde(default)]` plus a
//!     flattened `extra` catch-all, so an UNKNOWN future state (`{state:"custom_future_state",fresh:false}`)
//!     deserializes WITHOUT error/panic (RISK-3/MC-3/MC-6).
//!   * `symbol_references` 200 body (`:577-584`): `{ symbol_entity_id, staleness, callers:
//!     [SymbolCallerCallee], callees: [SymbolCallerCallee], nav_receipt_event_id,
//!     quiet_background_work_receipt_id }`; each caller/callee (`:540-547`) is `{ symbol_entity_id,
//!     symbol_key, display_name, confidence, evidence_spans: [EvidenceSpanRef], staleness }`; each
//!     evidence span (`edge_span_refs`, `:782-786`) is `{ span_id, line_start, line_end }`.
//!   * `symbol_tests` 200 body (`:634-640`): `{ symbol_entity_id, staleness, tests: [SymbolTestEntry],
//!     ... }`; each test (`:611-618`) is `{ test_entity_id, test_symbol_key, display_name, confidence,
//!     evidence_spans, staleness }`.
//!   * `symbol_spans` 200 body (`:689-695`): `{ symbol_entity_id, staleness, spans: [SymbolSpan], ... }`;
//!     each span (`:663-674`) is `{ span_id, source_id, span_kind, line_start, line_end, range_start?,
//!     range_end?, section_path?, content_sha256?, parser_version? }`.
//!   * `file_lens` 200 body (`:737-749` + `monaco_bridge::MonacoCodeLensPayload`/`CodeLensEntry`): the
//!     SERIALIZED `MonacoCodeLensPayload` `{ workspace_id, relative_path, staleness: StalenessVerdict,
//!     truncated, entries: [CodeLensEntry] }` with `nav_receipt_event_id` +
//!     `quiet_background_work_receipt_id` INSERTED at the top level. NOTE: the REAL `CodeLensEntry`
//!     (`monaco_bridge.rs:37-55`) is `{ symbol_entity_id, symbol_key, display_name, symbol_kind,
//!     definition: LineRange, references: [LineRange], doc?, caller_count }` — it has NO
//!     `callee_count`/`test_count`/per-entry `line_start`/`staleness` fields the MT-039 contract PROSE
//!     listed (that prose `FileLensEntry` was a fabrication; this client binds the REAL shape, per the
//!     Spec-Realism gate). `file_lens` `staleness` here is the backend `StalenessVerdict`
//!     (`staleness.rs:18-37`, `#[serde(tag="state", rename_all="snake_case")]`), whose serialized form
//!     ALSO has a `state` string + variant fields — [`StalenessState`] deserializes it tolerantly too
//!     (`fresh` derives from `state == "fresh"` when the bool field is absent).
//!   * `file_lens` path encoding (`:707` `decode_path` -> `:820-824` `replace("%2F","/")`): the `:path`
//!     segment MUST percent-encode an embedded `/` as `%2F`; a bare `/` is mis-parsed by axum's path
//!     extractor into multiple segments -> 404 (RISK-1/MC-1). This client encodes it
//!     ([`encode_path_segment`]) and REJECTS absolute paths / any `..`/`.` segment client-side
//!     (`is_safe_relative_path`, mirrors the backend `:829-835` traversal guard) BEFORE building the URL.
//!   * Symbol `entity_id` values are OPAQUE strings (`KE-`/`KEN-<uuid>`); this client NEVER parses or
//!     assumes their format (RISK-5).
//!   * `nav_receipt_event_id` + `quiet_background_work_receipt_id` are the EventLedger retrieval-trace
//!     receipt (spec 2.3.13.11) + the `ParallelSwarmStateRecoveryStore` quiet-work receipt; they are
//!     PRESERVED in EVERY response struct (`#[allow(dead_code)]`-friendly `pub` fields) so the audit
//!     trail is never dropped (RISK-4/MC-4), even when the immediate consumer ignores them.
//!
//! ## AccessKit requirement for the FUTURE `CodeSymbolPanel` widget (NO GUI in this MT)
//!
//! This MT creates BACKEND CLIENT CODE ONLY — there is no widget, no screenshot, and no AccessKit node
//! here. When the code-symbol panel widget is built (porting `app/src/components/CodeSymbolPanel.tsx`),
//! EVERY interactive element MUST be registered through the existing
//! [`crate::accessibility`] registry with a STABLE author_id, an appropriate role, and the navigation
//! action, so a swarm agent can drive it by id (HBR-SWARM):
//!   * the symbol-name button  -> `Role::Button`,   `Action::Click` (jump to definition),
//!     author_id e.g. `code-symbol-panel.symbol.<entity_id>`;
//!   * each reference list row  -> `Role::ListItem`, `Action::Click` (jump to reference),
//!     author_id e.g. `code-symbol-panel.reference.<index>`;
//!   * each test row            -> `Role::ListItem`, `Action::Click`,
//!     author_id e.g. `code-symbol-panel.test.<index>`;
//!   * each lens entry row      -> `Role::ListItem`, `Action::Click`,
//!     author_id e.g. `code-symbol-panel.lens.<entity_id>`.
//!
//! The panel MUST also render the [`StalenessState`] for every served symbol (never show a non-fresh
//! symbol as authoritative — the freshness rule below).

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::backend_client::{shared_http_client, BACKEND_BASE_URL, CODE_NAV_ACTOR_ID};

// Reuse MT-037's canonical identity-header struct (NO 4th copy of the 5-field x-hsk-* contract).
pub use crate::backend::knowledge_documents::HskDocumentHeaders;

/// Per-request timeout. A nav call must not hang the caller's worker; on timeout the client returns a
/// [`CodeNavError::Transport`] the editor layer surfaces as a transient error (and the inline fast-path
/// MT-008 client degrades to empty results).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The backend list cap, mirrored from `api::knowledge_code_nav::LIST_CAP` (verified `:83`). The backend
/// server-clamps `limit` to `1..=500`; this client clamps the SAME range CLIENT-SIDE (RISK-6/MC-5) so a
/// caller never asks for more than the backend will ever return and then mistakes a clamped page for the
/// full set.
pub const LIST_CAP: i64 = 500;

/// The verified-valid `x-hsk-actor-kind` for an automated UI navigation: the backend maps `system` to
/// `KernelActor::System` (the same kind its own quiet-nav lane uses). Reused from
/// [`crate::backend_client::CODE_NAV_ACTOR_KIND`] semantics; named here for the nav-header builder.
pub const CODE_NAV_ACTOR_KIND: &str = "system";

/// Build the nav identity for a code-navigation request from the current editor session context. The
/// three required `*_id` fields come from the CALLER's [`EditorSessionContext`] (never fabricated); the
/// `actor_kind` defaults to `system` (the verified nav kind). This is the `build_headers(ctx)` role the
/// MT contract names — the editor session supplies the identity, this only shapes the header struct.
pub fn code_nav_headers(ctx: &EditorSessionContext) -> HskDocumentHeaders {
    HskDocumentHeaders {
        actor_id: ctx.actor_id.clone(),
        kernel_task_run_id: ctx.kernel_task_run_id.clone(),
        session_run_id: ctx.session_run_id.clone(),
        actor_kind: Some(CODE_NAV_ACTOR_KIND.to_string()),
        correlation_id: ctx.correlation_id.clone(),
    }
}

/// The identity a code-navigation call site MUST supply from the current editor session. The MT contract
/// is explicit: "Do not fabricate actor_ids or session_ids: call sites must supply them from the current
/// editor session context." This is that context — the editor session layer constructs it once and the
/// nav client reads it; the client itself NEVER invents an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSessionContext {
    /// `x-hsk-actor-id` — who navigates (the native-editor actor, or a steering agent's id).
    pub actor_id: String,
    /// `x-hsk-kernel-task-run-id` — the kernel task run this nav belongs to.
    pub kernel_task_run_id: String,
    /// `x-hsk-session-run-id` — the session run within that task.
    pub session_run_id: String,
    /// `x-hsk-correlation-id` — optional correlation-chain id.
    pub correlation_id: Option<String>,
}

impl EditorSessionContext {
    /// A convenience identity for the native editor surface itself (NOT a fabricated agent). `actor_id`
    /// is the stable [`crate::backend_client::CODE_NAV_ACTOR_ID`] ("handshake-native-editor") so nav
    /// receipts are attributable to the surface (HBR-SWARM); the run ids are derived from the supplied
    /// session id so each session's navs are individually traceable. A steering AGENT builds its own
    /// context with its own actor id instead of using this.
    pub fn for_native_editor(session_run_id: impl Into<String>) -> Self {
        let session_run_id = session_run_id.into();
        Self {
            actor_id: CODE_NAV_ACTOR_ID.to_string(),
            kernel_task_run_id: format!("native-editor-code-nav-{session_run_id}"),
            session_run_id,
            correlation_id: None,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed result of one `/knowledge/code/*` call. Status codes are mapped to DISTINCT variants so the
/// panel layer can react correctly (a 400 missing-header / a 404 missing-symbol is not mistaken for a
/// transport failure), AND two CLIENT-SIDE guard variants reject a malformed request BEFORE the wire:
/// [`Self::EmptyLookup`] (no name/prefix/path — the backend would 400; RISK-2/MC-2) and
/// [`Self::UnsafePath`] (an absolute / `..`-bearing file-lens path — RISK-1 path-traversal guard).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeNavError {
    /// HTTP 400 — a malformed request the SERVER rejected (e.g. a missing required identity header, or a
    /// lookup the client guard did not catch). Carries the backend `detail` when present.
    BadRequest(String),
    /// HTTP 404 — the symbol / file is not indexed.
    NotFound(String),
    /// HTTP 5xx — the backend failed internally. Carries the status + any body detail.
    Server(String),
    /// A non-success status that is none of the above. Carries the status + body.
    UnexpectedStatus { status: u16, body: String },
    /// A transport failure (connect / timeout / TLS) — the request never reached a status.
    Transport(String),
    /// The response body could not be parsed into the expected typed shape.
    Parse(String),
    /// CLIENT-SIDE guard: a `lookup_symbols` call supplied NONE of name/prefix/path. The backend rejects
    /// this with a 400 (`api::knowledge_code_nav::lookup_symbols:423-427`); this client catches it BEFORE
    /// the round-trip so the caller gets a compile-shaped typed error, not a backend-400 surprise
    /// (RISK-2/MC-2).
    EmptyLookup,
    /// CLIENT-SIDE guard: a `file_lens` path is absolute, contains a backslash, or has a `..`/`.` segment.
    /// Mirrors the backend traversal guard (`is_safe_relative_path:829-835`) so a path-traversal attempt
    /// is rejected client-side and never built into a request URL (RISK-1 security control).
    UnsafePath(String),
}

impl std::fmt::Display for CodeNavError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(d) => write!(f, "bad request: {d}"),
            Self::NotFound(d) => write!(f, "not found: {d}"),
            Self::Server(d) => write!(f, "server error: {d}"),
            Self::UnexpectedStatus { status, body } => {
                write!(f, "unexpected status {status}: {body}")
            }
            Self::Transport(d) => write!(f, "transport error: {d}"),
            Self::Parse(d) => write!(f, "parse error: {d}"),
            Self::EmptyLookup => write!(
                f,
                "lookup_symbols requires at least one of name/prefix/path (none supplied)"
            ),
            Self::UnsafePath(p) => write!(
                f,
                "file_lens path must be a repo-relative POSIX path with no '..'/'.' segments: {p}"
            ),
        }
    }
}

impl std::error::Error for CodeNavError {}

/// The typed result alias for `/knowledge/code/*` calls.
pub type CodeNavResult<T> = Result<T, CodeNavError>;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Staleness (TOLERANT — never a closed enum).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The served-staleness flag attached to EVERY symbol / file the nav API returns (spec 2.3.13.11 "mark
/// stale, never serve stale silently"). It is INTENTIONALLY a tolerant shape: `state` is a free `String`
/// (NOT a closed enum) and every other field is optional, with a flattened `extra` catch-all, so the
/// client deserializes an UNKNOWN future backend state (e.g. `{state:"custom_future_state",fresh:false}`)
/// WITHOUT error or panic (RISK-3/MC-3/MC-6).
///
/// It deserializes BOTH backend shapes verified against the source:
///   * the `symbol_*` nav shape (`served_staleness`, free `json!` object with an explicit `fresh` bool):
///     `{state:"fresh",fresh:true,indexed_content_hash,indexed_parser_version}` /
///     `{state:"unindexed",fresh:false,detail}` / `{state:"marked_stale",fresh:false,...}` etc.;
///   * the `file_lens` shape (the backend `StalenessVerdict` enum
///     `#[serde(tag="state", rename_all="snake_case")]`): `{state:"fresh"}` /
///     `{state:"source_changed",indexed_hash,current_hash}` / `{state:"parser_changed",...}` /
///     `{state:"marked_stale"}` — these carry NO explicit `fresh` bool, so [`Self::is_fresh`] derives
///     freshness from `state == "fresh"`.
///
/// The freshness rule (the load-bearing invariant): anything not PROVABLY fresh is stale. A consumer
/// MUST NOT render a symbol whose [`Self::is_fresh`] is false as authoritative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StalenessState {
    /// The state label (e.g. `fresh` / `marked_stale` / `unindexed` / `source_changed` /
    /// `parser_changed` / `failed` / `partial` / `unknown` / any future string). Free `String`, never a
    /// closed enum, so a new backend state never breaks deserialization.
    #[serde(default)]
    pub state: String,
    /// The explicit freshness bool when the backend supplies one (the `symbol_*` nav shape). When ABSENT
    /// (the `file_lens` `StalenessVerdict` shape) this is `None`, and [`Self::is_fresh`] falls back to
    /// `state == "fresh"`.
    #[serde(default)]
    pub fresh: Option<bool>,
    /// The indexed content hash, when the state carries one (fresh / marked_stale).
    #[serde(default)]
    pub indexed_content_hash: Option<String>,
    /// The indexed parser version, when the state carries one (fresh / marked_stale).
    #[serde(default)]
    pub indexed_parser_version: Option<String>,
    /// A human-readable detail, when the state carries one (unindexed / failed / unknown).
    #[serde(default)]
    pub detail: Option<String>,
    /// Any ADDITIONAL fields a future backend state adds (e.g. the `StalenessVerdict::SourceChanged`
    /// `indexed_hash`/`current_hash`, or `ParserChanged` versions). Captured so nothing is dropped and an
    /// unknown shape never fails to deserialize (RISK-3).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

impl StalenessState {
    /// True ONLY when the symbol is PROVABLY fresh. Uses the explicit `fresh` bool when present (the
    /// `symbol_*` nav shape); otherwise derives freshness from `state == "fresh"` (the `file_lens`
    /// `StalenessVerdict` shape, which has no bool). Anything else — including any UNKNOWN future state —
    /// is treated as STALE (fail-closed). A consumer uses this to refuse to render a non-fresh symbol as
    /// authoritative.
    pub fn is_fresh(&self) -> bool {
        match self.fresh {
            Some(fresh) => fresh,
            None => self.state == "fresh",
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Response structs (REAL backend shapes — verified against api/knowledge_code_nav.rs json! blocks).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A definition span (the first `ast` span of a symbol). `api::knowledge_code_nav::symbol_to_json:372`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionSpan {
    pub span_id: String,
    pub source_id: String,
    pub line_start: i64,
    pub line_end: i64,
    #[serde(default)]
    pub range_start: Option<i64>,
    #[serde(default)]
    pub range_end: Option<i64>,
    #[serde(default)]
    pub section_path: Option<String>,
}

/// A symbol projection. `api::knowledge_code_nav::symbol_to_json:387-397`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolNavProjection {
    pub symbol_entity_id: String,
    pub symbol_key: String,
    pub display_name: String,
    pub symbol_kind: String,
    #[serde(default)]
    pub owning_wp: Option<String>,
    #[serde(default)]
    pub primary_source_id: Option<String>,
    pub lifecycle_state: String,
    /// The definition span, or `None` when the backend emitted JSON `null` (no `ast` span found).
    #[serde(default)]
    pub definition: Option<DefinitionSpan>,
    pub staleness: StalenessState,
}

/// `GET /knowledge/code/symbols` response. `lookup_symbols:488-493`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolLookupResponse {
    pub workspace_id: String,
    pub matches: Vec<SymbolNavProjection>,
    /// EventLedger retrieval-trace receipt id (spec 2.3.13.11). PRESERVED for audit (RISK-4/MC-4).
    pub nav_receipt_event_id: String,
    /// `ParallelSwarmStateRecoveryStore` quiet-background-work receipt id. PRESERVED for audit.
    pub quiet_background_work_receipt_id: String,
}

/// `GET /knowledge/code/symbols/:entity_id` response. `get_symbol:510-512`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolGetResponse {
    pub symbol: SymbolNavProjection,
    pub nav_receipt_event_id: String,
    pub quiet_background_work_receipt_id: String,
}

/// An evidence span ref on a reference/test edge. `edge_span_refs:782-786`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSpanRef {
    pub span_id: String,
    pub line_start: i64,
    pub line_end: i64,
}

/// One caller or callee of a symbol. `symbol_references:540-547` / `:553-560`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolCallerCallee {
    pub symbol_entity_id: String,
    pub symbol_key: String,
    pub display_name: String,
    /// Edge confidence (0.0..=1.0). f64 to match the backend `edge.confidence`.
    pub confidence: f64,
    pub evidence_spans: Vec<EvidenceSpanRef>,
    pub staleness: StalenessState,
}

/// `GET /knowledge/code/symbols/:entity_id/references` response. `symbol_references:577-584`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolReferencesResponse {
    pub symbol_entity_id: String,
    pub staleness: StalenessState,
    /// Incoming `references` edges: who calls this symbol.
    pub callers: Vec<SymbolCallerCallee>,
    /// Outgoing `references` edges: what this symbol calls.
    pub callees: Vec<SymbolCallerCallee>,
    pub nav_receipt_event_id: String,
    pub quiet_background_work_receipt_id: String,
}

/// One test that validates a symbol. `symbol_tests:611-618`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolTestEntry {
    pub test_entity_id: String,
    pub test_symbol_key: String,
    pub display_name: String,
    pub confidence: f64,
    pub evidence_spans: Vec<EvidenceSpanRef>,
    pub staleness: StalenessState,
}

/// `GET /knowledge/code/symbols/:entity_id/tests` response. `symbol_tests:634-640`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolTestsResponse {
    pub symbol_entity_id: String,
    pub staleness: StalenessState,
    pub tests: Vec<SymbolTestEntry>,
    pub nav_receipt_event_id: String,
    pub quiet_background_work_receipt_id: String,
}

/// One citation span of a symbol. `symbol_spans:663-674`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSpan {
    pub span_id: String,
    pub source_id: String,
    pub span_kind: String,
    pub line_start: i64,
    pub line_end: i64,
    #[serde(default)]
    pub range_start: Option<i64>,
    #[serde(default)]
    pub range_end: Option<i64>,
    #[serde(default)]
    pub section_path: Option<String>,
    #[serde(default)]
    pub content_sha256: Option<String>,
    #[serde(default)]
    pub parser_version: Option<String>,
}

/// `GET /knowledge/code/symbols/:entity_id/spans` response. `symbol_spans:689-695`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolSpansResponse {
    pub symbol_entity_id: String,
    pub staleness: StalenessState,
    pub spans: Vec<SymbolSpan>,
    pub nav_receipt_event_id: String,
    pub quiet_background_work_receipt_id: String,
}

/// A 1-based inclusive line range. Mirrors the backend `monaco_bridge::LineRange:29-33`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    pub start_line: i32,
    pub end_line: i32,
}

/// One code-lens entry for a symbol. Mirrors the REAL backend `monaco_bridge::CodeLensEntry:37-55`
/// EXACTLY (NOT the MT-039 contract-prose `FileLensEntry`, which listed `callee_count`/`test_count`/
/// per-entry line fields the real payload does NOT have — the Spec-Realism gate binds the real shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeLensEntry {
    pub symbol_entity_id: String,
    pub symbol_key: String,
    pub display_name: String,
    pub symbol_kind: String,
    /// The definition line range (the symbol's `ast` span).
    pub definition: LineRange,
    /// In-file reference ranges.
    pub references: Vec<LineRange>,
    /// Doc-comment text, if any.
    #[serde(default)]
    pub doc: Option<String>,
    /// Count of callers (incoming `references` edges) across the workspace.
    pub caller_count: u32,
}

/// `GET /knowledge/code/files/:path/lens` response. The serialized `monaco_bridge::MonacoCodeLensPayload`
/// (`:59-69`) with the two receipt ids inserted at the top level (`file_lens:743-749`). `staleness` is
/// the backend `StalenessVerdict` (`staleness.rs:18-37`), deserialized tolerantly by [`StalenessState`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileLensResponse {
    pub workspace_id: String,
    pub relative_path: String,
    pub staleness: StalenessState,
    /// True when the bounded symbol lookup hit its cap — the lens is PARTIAL, not the full file. A
    /// consumer surfaces a partial-lens warning rather than treating it as complete.
    pub truncated: bool,
    pub entries: Vec<CodeLensEntry>,
    pub nav_receipt_event_id: String,
    pub quiet_background_work_receipt_id: String,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Request params.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed `lookup_symbols` request. `workspace_id` is required; AT LEAST ONE of `name`/`prefix`/`path`
/// must be present (enforced by [`Self::validate`] BEFORE the request is built — RISK-2/MC-2). `limit` is
/// clamped to `1..=`[`LIST_CAP`] client-side (RISK-6/MC-5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LookupRequest {
    pub workspace_id: String,
    pub name: Option<String>,
    pub prefix: Option<String>,
    pub path: Option<String>,
    pub limit: Option<i64>,
}

impl LookupRequest {
    /// A lookup by exact simple name.
    pub fn by_name(workspace_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            name: Some(name.into()),
            ..Self::default()
        }
    }

    /// A lookup by name prefix (completion).
    pub fn by_prefix(workspace_id: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            prefix: Some(prefix.into()),
            ..Self::default()
        }
    }

    /// A lookup by file path (all symbols in a file).
    pub fn by_path(workspace_id: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            path: Some(path.into()),
            ..Self::default()
        }
    }

    /// True when at least one of name/prefix/path is non-empty (after trimming). A lookup with none is
    /// the backend 400 this client refuses to send (RISK-2/MC-2).
    fn has_filter(&self) -> bool {
        let non_empty = |o: &Option<String>| o.as_deref().map(str::trim).is_some_and(|s| !s.is_empty());
        non_empty(&self.name) || non_empty(&self.prefix) || non_empty(&self.path)
    }

    /// Validate the pre-flight guard: reject an empty lookup BEFORE the wire ([`CodeNavError::EmptyLookup`]).
    fn validate(&self) -> CodeNavResult<()> {
        if self.has_filter() {
            Ok(())
        } else {
            Err(CodeNavError::EmptyLookup)
        }
    }

    /// The query pairs for the GET, with `limit` clamped to `1..=`[`LIST_CAP`] (RISK-6/MC-5). Empty
    /// optional filters are omitted (the backend trims+filters them anyway, but omitting keeps the URL
    /// clean and matches the React `lookupCodeSymbols` query shape).
    fn query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = vec![("workspace_id".to_string(), self.workspace_id.clone())];
        let push_if = |pairs: &mut Vec<(String, String)>, key: &str, val: &Option<String>| {
            if let Some(v) = val.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                pairs.push((key.to_string(), v.to_string()));
            }
        };
        push_if(&mut pairs, "name", &self.name);
        push_if(&mut pairs, "prefix", &self.prefix);
        push_if(&mut pairs, "path", &self.path);
        if let Some(limit) = self.limit {
            // Clamp client-side to the backend cap so a caller never silently over-fetches and mistakes a
            // clamped page for the full set (RISK-6/MC-5).
            let clamped = limit.clamp(1, LIST_CAP);
            pairs.push(("limit".to_string(), clamped.to_string()));
        }
        pairs
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Path encoding (file_lens %2F — RISK-1/MC-1).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// True when `path` is a repo-relative POSIX path with no traversal: non-empty, no leading `/`, no
/// backslash, and no `.`/`..` segment. Mirrors the backend `is_safe_relative_path:829-835` so an unsafe
/// path is rejected CLIENT-SIDE and never built into a request URL (RISK-1 path-traversal control).
fn is_safe_relative_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') {
        return false;
    }
    path.split('/')
        .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

/// Percent-encode an embedded `/` in the `file_lens` `:path` segment as `%2F`. The backend decodes
/// `%2F`->`/` (`decode_path:820-824`); a bare `/` would be mis-parsed by axum's path extractor into
/// MULTIPLE path segments and 404 (RISK-1/MC-1). Only `/` needs encoding here (the path is already a
/// validated repo-relative POSIX path with no other reserved characters in practice), but we also encode
/// a literal `%` so an already-encoded input is not double-decoded by the backend.
fn encode_path_segment(path: &str) -> String {
    path.replace('%', "%25").replace('/', "%2F")
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The client.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The stateless typed client for the `/knowledge/code/*` surface. Holds ONLY a shared
/// [`reqwest::Client`] (cheaply cloneable; an `Arc` internally) and the base URL — NO navigation state.
/// The base URL is resolved from [`crate::backend_client::BACKEND_BASE_URL`] (config/environment via the
/// WP-011 backend client), NEVER hardcoded at a call site (GLOBAL-PORTABILITY-004). Mirrors the MT-037
/// [`crate::backend::knowledge_documents::KnowledgeDocumentsClient`] construction exactly.
#[derive(Clone)]
pub struct KnowledgeCodeNavClient {
    client: reqwest::Client,
    base_url: String,
}

impl Default for KnowledgeCodeNavClient {
    fn default() -> Self {
        Self::production()
    }
}

impl KnowledgeCodeNavClient {
    /// Construct against the production backend base URL (the same config-resolved `BACKEND_BASE_URL`
    /// every other native client uses), sharing the ONE process-wide
    /// [`crate::backend_client::shared_http_client`] connection pool rather than minting a second reqwest
    /// stack (the REUSE-NOT-DUPLICATE pool concern).
    pub fn production() -> Self {
        Self::with_client(shared_http_client(), BACKEND_BASE_URL)
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock server
    /// with an isolated pool). The base URL is the authority for the host — never hardcoded in a method.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Reuse an already-constructed [`reqwest::Client`] so the app shares ONE connection pool.
    pub fn with_client(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// `GET /knowledge/code/symbols?workspace_id=&name=&prefix=&path=&limit=` — symbol lookup
    /// (`api::knowledge_code_nav::lookup_symbols`). At least one of name/prefix/path is REQUIRED; this
    /// client GUARDS it client-side ([`CodeNavError::EmptyLookup`]) BEFORE sending so a no-filter lookup
    /// is a typed error, not a backend-400 surprise (RISK-2/MC-2). `limit` is clamped to `1..=`[`LIST_CAP`].
    pub async fn lookup_symbols(
        &self,
        headers: &HskDocumentHeaders,
        request: &LookupRequest,
    ) -> CodeNavResult<SymbolLookupResponse> {
        request.validate()?;
        let builder = self
            .client
            .get(self.url("/knowledge/code/symbols"))
            .query(&request.query_pairs());
        self.send_json(headers.apply_nav(builder)).await
    }

    /// `GET /knowledge/code/symbols/:entity_id` — one symbol with its definition + staleness
    /// (`api::knowledge_code_nav::get_symbol`). `entity_id` is an OPAQUE string — never parsed (RISK-5).
    pub async fn get_symbol(
        &self,
        headers: &HskDocumentHeaders,
        entity_id: &str,
    ) -> CodeNavResult<SymbolGetResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/code/symbols/{entity_id}")));
        self.send_json(headers.apply_nav(builder)).await
    }

    /// `GET /knowledge/code/symbols/:entity_id/references` — callers (incoming `references` edges) +
    /// callees (outgoing `references` edges) (`api::knowledge_code_nav::symbol_references`).
    pub async fn symbol_references(
        &self,
        headers: &HskDocumentHeaders,
        entity_id: &str,
    ) -> CodeNavResult<SymbolReferencesResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/code/symbols/{entity_id}/references")));
        self.send_json(headers.apply_nav(builder)).await
    }

    /// `GET /knowledge/code/symbols/:entity_id/tests` — tests with a `validates` edge targeting this
    /// symbol (`api::knowledge_code_nav::symbol_tests`).
    pub async fn symbol_tests(
        &self,
        headers: &HskDocumentHeaders,
        entity_id: &str,
    ) -> CodeNavResult<SymbolTestsResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/code/symbols/{entity_id}/tests")));
        self.send_json(headers.apply_nav(builder)).await
    }

    /// `GET /knowledge/code/symbols/:entity_id/spans` — all citation spans of the symbol
    /// (`api::knowledge_code_nav::symbol_spans`).
    pub async fn symbol_spans(
        &self,
        headers: &HskDocumentHeaders,
        entity_id: &str,
    ) -> CodeNavResult<SymbolSpansResponse> {
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/code/symbols/{entity_id}/spans")));
        self.send_json(headers.apply_nav(builder)).await
    }

    /// `GET /knowledge/code/files/:path/lens?workspace_id=&content_hash=&parser_version=` — the Monaco
    /// code-lens payload for a file (`api::knowledge_code_nav::file_lens`). The `:path` segment is a
    /// repo-relative POSIX path whose embedded `/` MUST be `%2F`-encoded (the backend decodes `%2F`->`/`;
    /// a bare `/` 404s — RISK-1/MC-1). This client REJECTS an absolute / `..`-bearing path client-side
    /// ([`CodeNavError::UnsafePath`]) and then `%2F`-encodes the segment.
    pub async fn file_lens(
        &self,
        headers: &HskDocumentHeaders,
        relative_path: &str,
        workspace_id: &str,
        content_hash: &str,
        parser_version: &str,
    ) -> CodeNavResult<FileLensResponse> {
        if !is_safe_relative_path(relative_path) {
            return Err(CodeNavError::UnsafePath(relative_path.to_string()));
        }
        let encoded = encode_path_segment(relative_path);
        let builder = self
            .client
            .get(self.url(&format!("/knowledge/code/files/{encoded}/lens")))
            .query(&[
                ("workspace_id", workspace_id),
                ("content_hash", content_hash),
                ("parser_version", parser_version),
            ]);
        self.send_json(headers.apply_nav(builder)).await
    }

    /// Send a built request (timeout attached), map the HTTP status to a typed [`CodeNavError`], and
    /// deserialize a success body into `T`. 400/404 map to their distinct variants carrying the backend
    /// `detail`; 5xx -> [`CodeNavError::Server`]; any other non-success -> [`CodeNavError::UnexpectedStatus`].
    /// The body is parsed exactly once. Mirrors the MT-037 `send_json` shape.
    async fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> CodeNavResult<T> {
        let resp = builder
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| CodeNavError::Transport(e.to_string()))?;
        let status = resp.status();
        if status.is_success() {
            return resp
                .json::<T>()
                .await
                .map_err(|e| CodeNavError::Parse(e.to_string()));
        }
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(map_error_status(code, &body))
    }
}

/// Map a non-success status + body into the typed [`CodeNavError`]. Pure (no IO) so the
/// status-to-variant contract is unit-provable without a live socket.
fn map_error_status(status: u16, body: &str) -> CodeNavError {
    let detail = serde_json::from_str::<Value>(body)
        .ok()
        .as_ref()
        .and_then(|v| v.get("detail").or_else(|| v.get("reason")))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| body.to_string());
    match status {
        400 => CodeNavError::BadRequest(detail),
        404 => CodeNavError::NotFound(detail),
        500..=599 => CodeNavError::Server(detail),
        other => CodeNavError::UnexpectedStatus {
            status: other,
            body: body.to_string(),
        },
    }
}

/// Nav-flavoured header application. MT-037's `HskDocumentHeaders::apply` is private to its module, so
/// this trait extension attaches the identity headers for a code-nav request without forking the struct
/// or re-deriving the header names (the canonical `x-hsk-*` constants live in
/// [`crate::backend_client`]). The three required headers are always attached; `actor_kind` /
/// `correlation_id` only when present.
trait ApplyNavHeaders {
    fn apply_nav(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder;
}

impl ApplyNavHeaders for HskDocumentHeaders {
    fn apply_nav(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        use crate::backend_client::{
            HSK_HEADER_ACTOR_ID, HSK_HEADER_ACTOR_KIND, HSK_HEADER_KERNEL_TASK_RUN_ID,
            HSK_HEADER_SESSION_RUN_ID,
        };
        let mut builder = builder
            .header(HSK_HEADER_ACTOR_ID, &self.actor_id)
            .header(HSK_HEADER_KERNEL_TASK_RUN_ID, &self.kernel_task_run_id)
            .header(HSK_HEADER_SESSION_RUN_ID, &self.session_run_id);
        if let Some(kind) = &self.actor_kind {
            builder = builder.header(HSK_HEADER_ACTOR_KIND, kind);
        }
        if let Some(correlation) = &self.correlation_id {
            // The optional correlation-id header constant lives in the MT-037 module (the only place that
            // declared it); reuse it so the name is not re-derived here.
            builder = builder.header(
                crate::backend::knowledge_documents::HSK_HEADER_CORRELATION_ID,
                correlation,
            );
        }
        builder
    }
}

#[cfg(test)]
mod tests {
    //! Pure unit proofs for the serialization + guard contracts that DO NOT need a socket. The
    //! wire-level proofs (mock-server round-trips for lookup / references / file_lens %2F) live in
    //! `tests/test_knowledge_code_nav.rs` against REAL backend payload shapes verified read-only against
    //! `api/knowledge_code_nav.rs`.

    use super::*;
    use serde_json::json;

    #[test]
    fn empty_lookup_is_rejected_before_the_wire() {
        // No name/prefix/path -> EmptyLookup, never a sent request (RISK-2/MC-2).
        let req = LookupRequest {
            workspace_id: "ws-1".into(),
            ..Default::default()
        };
        assert_eq!(req.validate(), Err(CodeNavError::EmptyLookup));
        // Whitespace-only is also empty.
        let blank = LookupRequest {
            workspace_id: "ws-1".into(),
            name: Some("   ".into()),
            ..Default::default()
        };
        assert_eq!(blank.validate(), Err(CodeNavError::EmptyLookup));
        // Any one non-empty filter passes.
        assert!(LookupRequest::by_name("ws-1", "Foo").validate().is_ok());
        assert!(LookupRequest::by_prefix("ws-1", "Fo").validate().is_ok());
        assert!(LookupRequest::by_path("ws-1", "src/lib.rs").validate().is_ok());
    }

    #[test]
    fn limit_is_clamped_client_side_to_list_cap() {
        // Over-cap clamps DOWN to LIST_CAP (RISK-6/MC-5).
        let over = LookupRequest {
            limit: Some(99_999),
            ..LookupRequest::by_name("ws-1", "Foo")
        };
        let pairs = over.query_pairs();
        let limit = pairs.iter().find(|(k, _)| k == "limit").map(|(_, v)| v.as_str());
        assert_eq!(limit, Some("500"), "over-cap limit clamps to LIST_CAP=500");
        // Below-1 clamps UP to 1.
        let under = LookupRequest {
            limit: Some(0),
            ..LookupRequest::by_name("ws-1", "Foo")
        };
        let pairs = under.query_pairs();
        let limit = pairs.iter().find(|(k, _)| k == "limit").map(|(_, v)| v.as_str());
        assert_eq!(limit, Some("1"), "below-1 limit clamps up to 1");
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        assert!(!is_safe_relative_path("/abs/path.rs"), "absolute path rejected");
        assert!(!is_safe_relative_path("../escape.rs"), "..-traversal rejected");
        assert!(!is_safe_relative_path("a/../b.rs"), "embedded .. rejected");
        assert!(!is_safe_relative_path("a/./b.rs"), "embedded . rejected");
        assert!(!is_safe_relative_path("win\\path.rs"), "backslash rejected");
        assert!(!is_safe_relative_path(""), "empty rejected");
        assert!(is_safe_relative_path("src/lib.rs"), "repo-relative POSIX path accepted");
        assert!(is_safe_relative_path("a/b/c/d.ts"), "deep repo-relative path accepted");
    }

    #[test]
    fn path_segment_encodes_slash_as_pct_2f() {
        // The load-bearing RISK-1/MC-1 encoding: '/' -> %2F (backend decodes %2F -> '/').
        assert_eq!(encode_path_segment("src/lib.rs"), "src%2Flib.rs");
        assert_eq!(encode_path_segment("a/b/c.ts"), "a%2Fb%2Fc.ts");
        // No slash -> unchanged.
        assert_eq!(encode_path_segment("lib.rs"), "lib.rs");
        // A literal '%' is encoded first so a pre-encoded input is not double-decoded.
        assert_eq!(encode_path_segment("a%2Fb"), "a%252Fb");
    }

    #[test]
    fn staleness_deserializes_fresh_marked_stale_and_unknown_future_state() {
        // The 'fresh' symbol shape (explicit fresh:true + indexed hashes).
        let fresh: StalenessState = serde_json::from_value(json!({
            "state": "fresh", "fresh": true,
            "indexed_content_hash": "sha:abc", "indexed_parser_version": "rust@1"
        }))
        .expect("fresh staleness deserializes");
        assert!(fresh.is_fresh());
        assert_eq!(fresh.indexed_content_hash.as_deref(), Some("sha:abc"));

        // The 'unindexed' shape (fresh:false + detail).
        let unindexed: StalenessState = serde_json::from_value(json!({
            "state": "unindexed", "fresh": false, "detail": "no index state"
        }))
        .expect("unindexed staleness deserializes");
        assert!(!unindexed.is_fresh());
        assert_eq!(unindexed.detail.as_deref(), Some("no index state"));

        // The CRITICAL one (RISK-3/MC-3/MC-6): an UNKNOWN future state must NOT error or panic.
        let future: StalenessState = serde_json::from_value(json!({
            "state": "custom_future_state", "fresh": false
        }))
        .expect("an unknown future staleness state must deserialize WITHOUT error");
        assert!(!future.is_fresh(), "an unknown state is treated as STALE (fail-closed)");
        assert_eq!(future.state, "custom_future_state");

        // The file_lens StalenessVerdict shape: NO explicit `fresh` bool -> is_fresh derives from state.
        let verdict_fresh: StalenessState =
            serde_json::from_value(json!({"state": "fresh"})).expect("verdict fresh deserializes");
        assert!(verdict_fresh.is_fresh(), "a tagged 'fresh' verdict with no bool is fresh");
        let verdict_changed: StalenessState = serde_json::from_value(json!({
            "state": "source_changed", "indexed_hash": "a", "current_hash": "b"
        }))
        .expect("verdict source_changed deserializes");
        assert!(!verdict_changed.is_fresh(), "a non-'fresh' verdict is stale");
        // The extra variant fields are captured, not dropped.
        assert_eq!(verdict_changed.extra.get("indexed_hash").and_then(Value::as_str), Some("a"));
    }

    #[test]
    fn status_mapping_is_distinct_per_code() {
        assert_eq!(
            map_error_status(400, r#"{"error":"bad_request","detail":"x-hsk-actor-id header is required"}"#),
            CodeNavError::BadRequest("x-hsk-actor-id header is required".into())
        );
        assert_eq!(
            map_error_status(404, r#"{"error":"not_found","detail":"symbol 'KE-x' not found"}"#),
            CodeNavError::NotFound("symbol 'KE-x' not found".into())
        );
        assert!(matches!(map_error_status(500, "{}"), CodeNavError::Server(_)));
        assert!(matches!(
            map_error_status(418, "teapot"),
            CodeNavError::UnexpectedStatus { status: 418, .. }
        ));
    }

    #[test]
    fn nav_headers_default_to_system_kind_from_session_context() {
        let ctx = EditorSessionContext::for_native_editor("session-9");
        let headers = code_nav_headers(&ctx);
        assert_eq!(headers.actor_kind.as_deref(), Some("system"), "nav uses the system actor-kind");
        assert_eq!(headers.actor_id, CODE_NAV_ACTOR_ID, "actor id is the native-editor surface");
        assert_eq!(headers.session_run_id, "session-9", "session id comes from the supplied context, not fabricated");
    }
}
