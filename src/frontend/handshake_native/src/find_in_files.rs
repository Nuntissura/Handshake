//! WP-KERNEL-012 MT-029 — native Find-in-Files + Replace-in-Files surface (E4 Search).
//!
//! The native Rust/egui port of the React `WorkspaceSearchPanel`
//! (`app/src/components/WorkspaceSearchPanel.tsx`, MT-250). This is a **workspace-wide** search across
//! all rich documents and Loom blocks with a two-phase replace-preview-then-apply flow. It is DISTINCT
//! from the single-document `FindReplacePanel` (MT-004, code-editor-local).
//!
//! ## Backend reality (Spec-Realism Gate — VERIFIED READ-ONLY, NOT contract-assumed)
//!
//! The MT-029 contract body names `GET /loom/search` and `PATCH /knowledge/rich-documents/{id}`, but
//! BOTH were verified against the real backend + the React reference and ALIGNED:
//!
//! - **Search** binds `GET /workspaces/{ws}/loom/graph-search` (handler `search_loom_graph` →
//!   `Vec<LoomGraphSearchResult>` carrying `{source_kind, result_kind, ref_id, title, excerpt, metadata,
//!   block?}`). This is the endpoint the React `searchLoomGraph()` actually calls
//!   (`app/src/lib/api.ts:1320-1341`) and the ONLY one returning the `source_kind`/`ref_id` shape this
//!   panel needs. The plain `/loom/search` (handler `search_loom_blocks`) returns `Vec<{block, score}>`
//!   with no `source_kind`/`ref_id`, so it CANNOT satisfy the documentId-from-hit logic — it is the
//!   wrong endpoint despite the contract naming it. Verified params (api/loom.rs `LoomSearchQueryParams`
//!   + api.test.ts:771): `q, source_kinds (comma-joined), tag_ids, mention_ids, case_sensitive,
//!   whole_word, regex (NOTE: `regex`, not `isRegex`), path, limit, offset`.
//! - **Rich-document load/save** binds `GET /knowledge/documents/{id}` + `PUT /knowledge/documents/
//!   {id}/save` `{expected_version, content_json}` (the MT-017/020 VERIFIED routes the React
//!   `loadRichDocument`/`saveRichDocument` use — `app/src/lib/api.ts:3199-3263`), NOT the contract's
//!   `/knowledge/rich-documents/{id}` PATCH. Save 200 → `{document:{doc_version,..}, save_receipt_event_id}`;
//!   save 409 → optimistic-concurrency conflict (NEVER a silent overwrite — RISK-2 data-loss control).
//! - **Bookmarks** bind `GET/PUT /workspaces/{ws}/search-bookmarks` with the body shape
//!   `{schema_id:"hsk.workspace_search_bookmark_state@1", bookmarks:[..]}` carried INSIDE the verified
//!   `bookmark_state` blob field (`api/workspaces.rs:806-869`). The backend stores `bookmark_state`
//!   opaquely; the schema_id lives in the blob (RISK-6: a wrong schema_id silently breaks the React
//!   reader, so the const is asserted in tests).
//!
//! ## Two-phase replace + data-loss safety (HBR-STOP — the replace MUTATES documents)
//!
//! Replace is preview-then-apply with three guards mirroring the React reference:
//! 1. STALE-RESULT guard: `result_set_key = hash(search params)`. Preview Replace refuses if the live
//!    `result_set_key` no longer matches the params the results were fetched under (RISK-2/MC-2).
//! 2. STALE-PLAN guard: `preview_plan_key = hash(result_set_key + replacement)`. Apply refuses if the
//!    preview is stale vs the current search+replacement (prevents applying a stale plan to a
//!    since-edited doc = silent data loss).
//! 3. OPTIMISTIC CONCURRENCY: each save carries the `expected_version` captured at preview; a 409 is
//!    surfaced and the OTHER receipts are preserved — never a silent overwrite (PARTIAL-FAILURE,
//!    RISK-1/MC-1).
//!
//! The content_json walk mutates ONLY `node.text` + `node.attrs.code` and round-trips every other node
//! VERBATIM (RISK-4: hsLink/embed/table nodes are preserved). Zero-length regex matches advance by 1
//! (RISK-3, no infinite loop); a non-regex query is `regex::escape`'d (RISK-8); only `KRD-`-prefixed
//! document ids are loaded (RISK-5).
//!
//! ## Async / HBR-QUIET + AccessKit (HBR-SWARM / HBR-VIS)
//!
//! Every HTTP call runs off the egui UI thread on the app's tokio runtime, delivering into
//! `Arc<Mutex<Option<..>>>` cells the panel drains each frame; the loading spinner animates ONLY while a
//! request is genuinely in flight (never a perpetual spinner). Every interactive widget carries a stable
//! kebab-case `author_id` under the `find-in-files.` namespace via
//! [`crate::accessibility::emit_interactive_node`].

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use regex::Regex;

use crate::accessibility;
use crate::backend_client::{
    BookmarkStateCell, FindInFilesOperation, FindInFilesStamp, FindReplaceCell, GraphSearchCell,
    LoomGraphSearchHit, RichDocClient, WorkspaceSearchClient,
};
use crate::pane_registry::{PaneFactory, PaneId, PaneRenderContext, PaneType};
use crate::theme::HsPalette;

// ── Stable AccessKit author_ids (the MT-029 naming contract) ─────────────────────────────────────────

/// The query `TextEdit`.
pub const QUERY_AUTHOR_ID: &str = "find-in-files.query";
/// The replacement `TextEdit`.
pub const REPLACE_AUTHOR_ID: &str = "find-in-files.replace";
/// The case-sensitive toggle (`Aa`).
pub const TOGGLE_CASE_AUTHOR_ID: &str = "find-in-files.toggle-case";
/// The whole-word toggle (`W`).
pub const TOGGLE_WORD_AUTHOR_ID: &str = "find-in-files.toggle-word";
/// The regex toggle (`.*`).
pub const TOGGLE_REGEX_AUTHOR_ID: &str = "find-in-files.toggle-regex";
/// The kind-filter `ComboBox`.
pub const KIND_FILTER_AUTHOR_ID: &str = "find-in-files.kind-filter";
/// The tag-filter `TextEdit`.
pub const TAG_FILTER_AUTHOR_ID: &str = "find-in-files.tag-filter";
/// The path-filter `TextEdit`.
pub const PATH_FILTER_AUTHOR_ID: &str = "find-in-files.path-filter";
/// The `Search` button.
pub const SEARCH_AUTHOR_ID: &str = "find-in-files.search";
/// Durable action-completion observer for the asynchronous Search button.
pub const SEARCH_COMPLETION_AUTHOR_ID: &str = "find-in-files.search-completion";
const SEARCH_COMPLETION_EFFECT: &str = "find-in-files.search";
/// The `Preview Replace` button.
pub const PREVIEW_REPLACE_AUTHOR_ID: &str = "find-in-files.preview-replace";
/// The `Apply` button.
pub const APPLY_AUTHOR_ID: &str = "find-in-files.apply";
/// The `Cancel`/clear button.
pub const CANCEL_AUTHOR_ID: &str = "find-in-files.cancel";
/// The `Bookmark Search` button.
pub const SAVE_BOOKMARK_AUTHOR_ID: &str = "find-in-files.save-bookmark";
/// Live query/preview/apply/error status exposed as a polite AccessKit status node.
pub const STATUS_AUTHOR_ID: &str = "find-in-files.status";
/// Live bookmark load/save/remove status, including the producer receipt id when present.
pub const BOOKMARK_STATUS_AUTHOR_ID: &str = "find-in-files.bookmark-status";
/// Retry the failed mount-time bookmark load against the same active workspace.
pub const BOOKMARK_RETRY_AUTHOR_ID: &str = "find-in-files.bookmark-retry";
/// Prefix for one persisted bookmark's Restore action; the bookmark id is UTF-8 byte-hex encoded.
pub const BOOKMARK_RESTORE_AUTHOR_ID_PREFIX: &str = "find-in-files.bookmark-restore.";
/// Prefix for one persisted bookmark's Remove action; the bookmark id is UTF-8 byte-hex encoded.
pub const BOOKMARK_REMOVE_AUTHOR_ID_PREFIX: &str = "find-in-files.bookmark-remove.";
/// Prefix for a per-result row (both backend identity components are lowercase UTF-8 byte hex).
pub const RESULT_AUTHOR_ID_PREFIX: &str = "find-in-files.result.";
/// Prefix for a per-preview item (the document id is lowercase UTF-8 byte hex).
pub const PREVIEW_AUTHOR_ID_PREFIX: &str = "find-in-files.preview.";
/// Prefix for one preview plan's exact before-content label.
pub const PREVIEW_BEFORE_AUTHOR_ID_PREFIX: &str = "find-in-files.preview-before.";
/// Prefix for one preview plan's exact after-content label.
pub const PREVIEW_AFTER_AUTHOR_ID_PREFIX: &str = "find-in-files.preview-after.";

// ── Action-completion observers (canonical Argus terminal causality) ─────────────────────────────────
//
// `crate::mcp::action` terminalises a canonical `argus.click`/`argus.set_value` as `Applied` ONLY when
// the product publishes an action-specific completion token that makes an exact
// `generation -> generation + 1` transition bound to the clicked target and to the semantic value that
// target declared BEFORE dispatch. A visible value echo, an unchanged tree, or a disappearing target is
// deliberately NOT proof and terminates as `Indeterminate`. Every steerable Find-in-Files control
// therefore owns one of two product-side acknowledgements:
//
//  * SAME-TARGET (`handshake.click-completion/v1`, synchronous controls that stay mounted) — the control
//    carries its own counter in its AccessKit `value`; the panel advances that counter only where it
//    actually consumed the activation.
//  * DURABLE OBSERVER (`handshake.click-completion/v1`, asynchronous controls) — a `Role::Status` node
//    outlives the request and publishes Ready -> Pending -> Applied/Failed, carrying the exact terminal
//    proof detail (per-document before/after hashes + EventLedger save receipt ids for Apply).
//
// Because the observer must survive target disappearance, the state lives on
// [`FindInFilesPanelState`], never in `egui::Memory` (a fresh `egui::Context` snapshot reads Memory
// back empty).

/// Toggle state projections. The toggle buttons themselves carry their same-target completion token in
/// `value`, so the boolean is published on a dedicated sibling `Role::Status` node.
pub const TOGGLE_CASE_STATE_AUTHOR_ID: &str = "find-in-files.toggle-case-state";
/// See [`TOGGLE_CASE_STATE_AUTHOR_ID`].
pub const TOGGLE_WORD_STATE_AUTHOR_ID: &str = "find-in-files.toggle-word-state";
/// See [`TOGGLE_CASE_STATE_AUTHOR_ID`].
pub const TOGGLE_REGEX_STATE_AUTHOR_ID: &str = "find-in-files.toggle-regex-state";
/// Durable action-completion observer for the asynchronous Preview Replace button.
pub const PREVIEW_COMPLETION_AUTHOR_ID: &str = "find-in-files.preview-completion";
/// Durable action-completion observer for the asynchronous, DESTRUCTIVE Apply button.
pub const APPLY_COMPLETION_AUTHOR_ID: &str = "find-in-files.apply-completion";
/// Durable action-completion observer for the Cancel control.
pub const CANCEL_COMPLETION_AUTHOR_ID: &str = "find-in-files.cancel-completion";
/// Durable action-completion observer shared by bookmark save and bookmark remove (one persisted PUT).
pub const BOOKMARK_COMPLETION_AUTHOR_ID: &str = "find-in-files.bookmark-completion";
/// Durable action-completion observer for the bookmark-load Retry control.
pub const BOOKMARK_LOAD_COMPLETION_AUTHOR_ID: &str = "find-in-files.bookmark-load-completion";

/// Shell-owned durable observer for RESULT NAVIGATION. Activating a result row REPLACES this whole
/// surface with the routed editor, so the row is a TRANSIENT observer target bound to an observer the
/// shell projects outside the pane — the acknowledgement therefore survives the row's disappearance.
pub const RESULT_OPEN_COMPLETION_AUTHOR_ID: &str = "find-in-files.result-open-completion";
/// See [`RESULT_OPEN_COMPLETION_AUTHOR_ID`].
pub const RESULT_OPEN_COMPLETION_EFFECT: &str = "find-in-files.result-open";
/// See [`RESULT_OPEN_COMPLETION_AUTHOR_ID`].
pub const RESULT_OPEN_COMPLETION_CONTEXT: &str = "find-in-files.result-open:shell";

/// The shell's current result-navigation observer generation. `ready` is false while a routed
/// navigation is still in flight, which WITHHOLDS the declaration instead of publishing a stale one.
#[derive(Debug, Clone, Copy, Default)]
pub struct FindResultOpenCompletionBinding {
    pub generation: u64,
    pub ready: bool,
}

/// The exact pre-dispatch semantic identity of one result-row navigation.
pub fn open_result_semantic(source_kind: &str, ref_id: &str) -> String {
    serde_json::json!({
        "effect": "open-find-result",
        "source_kind": source_kind,
        "ref_id": ref_id,
    })
    .to_string()
}

const PREVIEW_COMPLETION_EFFECT: &str = "find-in-files.preview-replace";
const APPLY_COMPLETION_EFFECT: &str = "find-in-files.apply";
const CANCEL_COMPLETION_EFFECT: &str = "find-in-files.cancel";
const BOOKMARK_COMPLETION_EFFECT: &str = "find-in-files.bookmark-persist";
const BOOKMARK_LOAD_COMPLETION_EFFECT: &str = "find-in-files.bookmark-load";
const TOGGLE_COMPLETION_EFFECT: &str = "find-in-files.toggle";
const RESULT_COMPLETION_EFFECT: &str = "find-in-files.result-open";
const PREVIEW_ROW_COMPLETION_EFFECT: &str = "find-in-files.preview-row";
const BOOKMARK_RESTORE_COMPLETION_EFFECT: &str = "find-in-files.bookmark-restore";
/// `terminal_detail` is bounded by the MCP token contract; an Apply over a large plan set publishes the
/// exact rows that fit plus a complete digest, never a silently truncated audit.
const MAX_TERMINAL_DETAIL_AUDIT_ROWS: usize = 8;

/// Keep the contract base id on the factory's stable primary-pane lease; deterministically scope
/// every additional mounted instance so Argus never sees duplicate targets.
///
/// This is the STATIC-id scope helper: `author_id` must already be a bounded, product-authored
/// constant (`find-in-files.query`, ...). Content-derived routes must NOT be scoped through here —
/// they compose their scope in one step through [`compose_author_id`], which is the only path that
/// can shorten the CONTENT when the pane suffix no longer fits. Pane-scoping an already-oversized
/// base cannot be repaired at this layer, so it trips the authoring-time assertion instead of
/// silently emitting an unaddressable route.
pub fn pane_scoped_author_id(author_id: &str, secondary_pane_id: Option<&str>) -> String {
    let Some(pane_id) = secondary_pane_id else {
        return author_id.to_owned();
    };
    let verbatim = format!("{author_id}.pane-{}", encode_author_id_component(pane_id));
    if verbatim.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES {
        return verbatim;
    }
    let digested = format!("{author_id}.pane-{}", digest_author_id_component(pane_id));
    debug_assert!(
        digested.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES,
        "pane-scoped author_id is {} bytes, over the {MAX_COMPLETION_TARGET_AUTHOR_BYTES}-byte canonical completion target budget; base author_id `{author_id}` is not bounded — compose content-derived routes through compose_author_id instead",
        digested.len()
    );
    digested
}

/// 24-char context window each side of a match preview (the React `MATCH_PREVIEW_CONTEXT_CHARS`).
pub const MATCH_PREVIEW_CONTEXT_CHARS: usize = 24;

/// Max bookmarks persisted (the React `MAX_WORKSPACE_SEARCH_BOOKMARKS`).
pub const MAX_WORKSPACE_SEARCH_BOOKMARKS: usize = 20;

/// The backend-validated bookmark blob schema id (RISK-6: a wrong value silently fails the React
/// reader). Asserted in the bookmark-blob test.
pub const WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID: &str = "hsk.workspace_search_bookmark_state@1";

/// Bound on [`SearchBookmark::stable_id`] (MT-113). Chosen so the derived
/// `bookmark_remove_semantic_value` JSON stays FAR inside the canonical 2048-byte `semantic_value`
/// budget, while leaving every realistic saved search (a query of roughly 450 characters or fewer) in
/// the byte-identical verbatim regime.
pub const MAX_BOOKMARK_STABLE_ID_BYTES: usize = 1024;

// ── Bounded author_id composition (MT-113) ────────────────────────────────────────────────────────────
//
// A content-derived `author_id` is the ONLY name Argus has for a control, and the canonical
// `handshake.click-completion/v1` contract bounds every author-id-shaped token field at
// [`MAX_COMPLETION_TARGET_AUTHOR_BYTES`]. Before MT-113, composition was UNBOUNDED: a saved-search id
// hex-encodes the query and `bookmark_*_author_id` hex-encodes THAT result (a 4x expansion of the
// original user bytes), so a 23-character query overran the 256-byte `pending_target` budget,
// `serialize_*` returned `None`, the observer node carried NO value, and every action on that control
// terminalised `indeterminate` FOREVER with no diagnostic anywhere. Composition is therefore bounded
// BY CONSTRUCTION here rather than policed downstream.
//
// The encoding is a deterministic two-regime function of the exact component tuple:
//
//   * VERBATIM  — lowercase bytewise UTF-8 hex, used whenever the FULL composed route (INCLUDING its
//                 optional `.pane-<hex>` scope) already fits the budget. Every id that fits today is
//                 therefore byte-identical after MT-113 and stays exactly reversible.
//   * DIGESTED  — `zsha256-<64 lowercase hex>` per component, used only when the verbatim form would
//                 overrun. `z` is outside the hex alphabet, so a digested component can NEVER collide
//                 with a verbatim one, and SHA-256 keeps distinct content on distinct routes.
//
// INJECTIVITY: the two regimes are disjoint per component and `.` never occurs inside a component, so
// the composed route parses back to its component tuple unambiguously; distinct tuples therefore map
// to distinct routes (digested components under SHA-256 collision resistance).
//
// RESOLVABILITY: exact string-only reversal is mathematically impossible in the digested regime — no
// bounded string can injectively encode unbounded input — so a digested route is resolved by
// RECOMPUTATION against the live candidate set. [`hit_identity_from_result_author_id_in`] is TOTAL
// where the string-only [`hit_identity_from_result_author_id`] is partial, and it is exactly how the
// production panel already resolves an activated route (it recomposes the id per row).

/// The canonical byte budget for any author-id-shaped completion-token field. Composition is bounded
/// to the STRICTER of the two token budgets (`pending_target`/`observer_author_id` at 256 B, `context`
/// at 512 B) so a synchronous control and its asynchronous sibling can never diverge in provability
/// for the SAME id — see `mcp::action::MAX_CLICK_COMPLETION_AUTHOR_BYTES` for the asymmetry rationale.
pub const MAX_COMPLETION_TARGET_AUTHOR_BYTES: usize =
    crate::mcp::action::MAX_CLICK_COMPLETION_AUTHOR_BYTES;

/// The canonical byte budget for the pre-dispatch `semantic_value` an observer declaration carries.
pub const MAX_COMPLETION_SEMANTIC_BYTES: usize =
    crate::mcp::action::MAX_CLICK_COMPLETION_SEMANTIC_BYTES;

/// Sentinel introducing a DIGESTED author_id component. `z` is outside the lowercase-hex alphabet, so
/// a digested component is unambiguously distinguishable from a verbatim one at every position.
pub const AUTHOR_ID_DIGEST_SENTINEL: &str = "zsha256-";

/// Byte length of one digested component (`zsha256-` + 64 hex characters).
const DIGESTED_AUTHOR_ID_COMPONENT_BYTES: usize = AUTHOR_ID_DIGEST_SENTINEL.len() + 64;

fn encode_author_id_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

/// The bounded, injective stand-in for a component whose verbatim hex would overrun the route budget.
/// It digests the exact UTF-8 BYTES, so a multibyte codepoint or grapheme cluster is never split.
fn digest_author_id_component(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = format!(
        "{AUTHOR_ID_DIGEST_SENTINEL}{:x}",
        Sha256::digest(value.as_bytes())
    );
    debug_assert_eq!(digest.len(), DIGESTED_AUTHOR_ID_COMPONENT_BYTES);
    digest
}

fn join_author_id(prefix: &str, components: &[String], pane: Option<&str>) -> String {
    let mut composed = String::from(prefix);
    for (index, component) in components.iter().enumerate() {
        if index > 0 {
            composed.push('.');
        }
        composed.push_str(component);
    }
    if let Some(pane) = pane {
        composed.push_str(".pane-");
        composed.push_str(pane);
    }
    composed
}

/// Compose a content-derived route that is GUARANTEED to fit [`MAX_COMPLETION_TARGET_AUTHOR_BYTES`],
/// INCLUDING its optional pane scope, for ARBITRARY user content.
///
/// The reduction order is fixed so the result is a pure function of `(prefix, components, pane)`:
/// verbatim -> digest the content components -> digest the pane component. Nothing here can widen the
/// budget, so user content can never silently disable a control's completion token again.
fn compose_author_id(prefix: &str, components: &[&str], secondary_pane_id: Option<&str>) -> String {
    let verbatim: Vec<String> = components
        .iter()
        .map(|component| encode_author_id_component(component))
        .collect();
    let pane_verbatim = secondary_pane_id.map(encode_author_id_component);
    let composed = join_author_id(prefix, &verbatim, pane_verbatim.as_deref());
    if composed.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES {
        // The overwhelmingly common path, and the one that keeps every already-working route
        // byte-identical: the verbatim encoding is emitted UNCHANGED whenever it fits.
        return composed;
    }
    let digested: Vec<String> = components
        .iter()
        .map(|component| digest_author_id_component(component))
        .collect();
    let composed = join_author_id(prefix, &digested, pane_verbatim.as_deref());
    if composed.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES {
        return composed;
    }
    let pane_digested = secondary_pane_id.map(digest_author_id_component);
    let composed = join_author_id(prefix, &digested, pane_digested.as_deref());
    debug_assert!(
        composed.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES,
        "composed author_id is {} bytes, over the {MAX_COMPLETION_TARGET_AUTHOR_BYTES}-byte canonical completion target budget even fully digested; prefix `{prefix}` plus {} components does not fit the route contract",
        composed.len(),
        components.len()
    );
    composed
}

fn decode_author_id_component(value: &str) -> Option<String> {
    if !value.len().is_multiple_of(2) || !value.is_ascii() {
        return None;
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

/// The result-row author_id for a hit. Both identity components are encoded byte-for-byte so distinct
/// UTF-8 backend identities can never collapse onto the same AccessKit route, and the composed route is
/// bounded so an arbitrarily long backend `ref_id` (a file path, for example) cannot silently disable
/// this row's completion token.
pub fn result_author_id(source_kind: &str, ref_id: &str) -> String {
    pane_scoped_result_author_id(source_kind, ref_id, None)
}

/// [`result_author_id`] composed together with its pane scope in ONE bounded step, so the SCOPED route
/// also fits the canonical completion target budget.
pub fn pane_scoped_result_author_id(
    source_kind: &str,
    ref_id: &str,
    secondary_pane_id: Option<&str>,
) -> String {
    compose_author_id(
        RESULT_AUTHOR_ID_PREFIX,
        &[source_kind, ref_id],
        secondary_pane_id,
    )
}

/// Reverse an AccessKit result route into the exact backend `(source_kind, ref_id)` identity.
///
/// EXACT for the verbatim regime, which is every route whose composed form fits the budget — the case
/// MT-029's `accesskit_result_ids_are_utf8_injective_and_exactly_reversible` pins. A route carrying a
/// DIGESTED component (over-budget content) or a pane scope is not reversible from the string alone;
/// use [`hit_identity_from_result_author_id_in`], which resolves by recomputation and is total.
pub fn hit_identity_from_result_author_id(author_id: &str) -> Option<(String, String)> {
    let encoded = author_id.strip_prefix(RESULT_AUTHOR_ID_PREFIX)?;
    let (source_kind, ref_id) = encoded.split_once('.')?;
    if ref_id.contains('.') {
        return None;
    }
    Some((
        decode_author_id_component(source_kind)?,
        decode_author_id_component(ref_id)?,
    ))
}

/// Resolve ANY result route — verbatim or digested, scoped or unscoped — back to the exact backend
/// identity that produced it, by recomposing each live candidate through the SAME bounded composer.
///
/// This is the resolution the production panel already performs implicitly (it recomposes the route for
/// every rendered row), lifted into a reusable, total function so a bounded route is never a dead end.
pub fn hit_identity_from_result_author_id_in<'a, I>(
    author_id: &str,
    candidates: I,
    secondary_pane_id: Option<&str>,
) -> Option<(String, String)>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    if secondary_pane_id.is_none() {
        if let Some(identity) = hit_identity_from_result_author_id(author_id) {
            return Some(identity);
        }
    }
    candidates.into_iter().find_map(|(source_kind, ref_id)| {
        (pane_scoped_result_author_id(source_kind, ref_id, secondary_pane_id) == author_id)
            .then(|| (source_kind.to_owned(), ref_id.to_owned()))
    })
}

/// The preview-item author_id for a planned document:
/// `find-in-files.preview.{lowercase_hex(document_id.as_bytes())}`.
pub fn preview_author_id(document_id: &str) -> String {
    pane_scoped_preview_author_id(document_id, None)
}

/// See [`preview_author_id`]; composes the pane scope in the same bounded step.
pub fn pane_scoped_preview_author_id(document_id: &str, secondary_pane_id: Option<&str>) -> String {
    compose_author_id(PREVIEW_AUTHOR_ID_PREFIX, &[document_id], secondary_pane_id)
}

/// Exact before-content label for one planned document.
pub fn preview_before_author_id(document_id: &str) -> String {
    pane_scoped_preview_before_author_id(document_id, None)
}

/// See [`preview_before_author_id`]; composes the pane scope in the same bounded step.
pub fn pane_scoped_preview_before_author_id(
    document_id: &str,
    secondary_pane_id: Option<&str>,
) -> String {
    compose_author_id(
        PREVIEW_BEFORE_AUTHOR_ID_PREFIX,
        &[document_id],
        secondary_pane_id,
    )
}

/// Exact after-content label for one planned document.
pub fn preview_after_author_id(document_id: &str) -> String {
    pane_scoped_preview_after_author_id(document_id, None)
}

/// See [`preview_after_author_id`]; composes the pane scope in the same bounded step.
pub fn pane_scoped_preview_after_author_id(
    document_id: &str,
    secondary_pane_id: Option<&str>,
) -> String {
    compose_author_id(
        PREVIEW_AFTER_AUTHOR_ID_PREFIX,
        &[document_id],
        secondary_pane_id,
    )
}

/// Stable per-bookmark Restore route using the lowercase bytewise UTF-8 hex contract, bounded so a long
/// saved query cannot push the route past the canonical completion target budget.
pub fn bookmark_restore_author_id(bookmark_id: &str) -> String {
    pane_scoped_bookmark_restore_author_id(bookmark_id, None)
}

/// See [`bookmark_restore_author_id`]; composes the pane scope in the same bounded step.
pub fn pane_scoped_bookmark_restore_author_id(
    bookmark_id: &str,
    secondary_pane_id: Option<&str>,
) -> String {
    compose_author_id(
        BOOKMARK_RESTORE_AUTHOR_ID_PREFIX,
        &[bookmark_id],
        secondary_pane_id,
    )
}

/// Stable per-bookmark Remove route using the lowercase bytewise UTF-8 hex contract.
///
/// This is the exact route MT-029 measured as UNPROVABLE for a 23-character search query: it is derived
/// from [`SearchBookmark::stable_id`], which already hex-encodes the query, so the second hex pass was a
/// 4x expansion of the original user bytes. It is now bounded by construction.
pub fn bookmark_remove_author_id(bookmark_id: &str) -> String {
    pane_scoped_bookmark_remove_author_id(bookmark_id, None)
}

/// See [`bookmark_remove_author_id`]; composes the pane scope in the same bounded step.
pub fn pane_scoped_bookmark_remove_author_id(
    bookmark_id: &str,
    secondary_pane_id: Option<&str>,
) -> String {
    compose_author_id(
        BOOKMARK_REMOVE_AUTHOR_ID_PREFIX,
        &[bookmark_id],
        secondary_pane_id,
    )
}

// ── Kind filter ──────────────────────────────────────────────────────────────────────────────────────

/// One selectable source-kind filter. `All` omits `source_kinds` entirely; every other variant passes
/// exactly one `source_kind` to the backend. The labels + wire values mirror the React `SEARCHABLE_KINDS`
/// table (`WorkspaceSearchPanel.tsx:17-28`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    All,
    Document,
    LoomBlock,
    File,
    TagHub,
    Symbol,
    WorkPacket,
    MicroTask,
    UserManualPage,
    WikiPage,
}

impl KindFilter {
    /// Every variant in display order (the order the ComboBox lists them).
    pub const ALL: [KindFilter; 10] = [
        KindFilter::All,
        KindFilter::Document,
        KindFilter::LoomBlock,
        KindFilter::File,
        KindFilter::TagHub,
        KindFilter::Symbol,
        KindFilter::WorkPacket,
        KindFilter::MicroTask,
        KindFilter::UserManualPage,
        KindFilter::WikiPage,
    ];

    /// The human/model-readable label (React parity).
    pub fn label(self) -> &'static str {
        match self {
            KindFilter::All => "All kinds",
            KindFilter::Document => "Documents",
            KindFilter::LoomBlock => "Loom blocks",
            KindFilter::File => "Files",
            KindFilter::TagHub => "Tags",
            KindFilter::Symbol => "Symbols",
            KindFilter::WorkPacket => "Work packets",
            KindFilter::MicroTask => "Microtasks",
            KindFilter::UserManualPage => "UserManual",
            KindFilter::WikiPage => "Wiki pages",
        }
    }

    /// The backend `source_kind` wire value for the single-kind filter, or `None` for `All` (which omits
    /// the `source_kinds` param entirely — AC-4). The values match the backend `LoomSearchSourceKind`
    /// snake_case enum.
    pub fn source_kind(self) -> Option<&'static str> {
        match self {
            KindFilter::All => None,
            KindFilter::Document => Some("document"),
            KindFilter::LoomBlock => Some("loom_block"),
            KindFilter::File => Some("file"),
            KindFilter::TagHub => Some("tag_hub"),
            KindFilter::Symbol => Some("symbol"),
            KindFilter::WorkPacket => Some("work_packet"),
            KindFilter::MicroTask => Some("micro_task"),
            KindFilter::UserManualPage => Some("user_manual_page"),
            KindFilter::WikiPage => Some("wiki_page"),
        }
    }

    /// The stable wire token used in a bookmark blob's `kind` field (round-trips through restore).
    pub fn wire(self) -> &'static str {
        self.source_kind().unwrap_or("all")
    }

    /// Parse a bookmark blob `kind` token back to a filter. Unknown persisted values fail closed so a
    /// producer/schema drift cannot silently broaden a filtered bookmark to `All`.
    pub fn from_wire(s: &str) -> Result<KindFilter, String> {
        KindFilter::ALL
            .into_iter()
            .find(|k| k.wire() == s)
            .ok_or_else(|| format!("unsupported bookmark kind '{s}'"))
    }
}

// ── Match options ─────────────────────────────────────────────────────────────────────────────────────

/// The three match toggles (case / whole-word / regex). Copied into every pure replace fn so the logic
/// is unit-testable standalone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MatchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
}

impl MatchOptions {
    /// Project into the transport-layer [`crate::backend_client::SearchMatchOptions`] the search client
    /// forwards as query params (kept as a separate type so backend_client does not depend on this
    /// module's `MatchOptions`).
    pub fn to_search(self) -> crate::backend_client::SearchMatchOptions {
        crate::backend_client::SearchMatchOptions {
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            is_regex: self.is_regex,
        }
    }
}

// ── Regex compilation (PT-4, RISK-8) ──────────────────────────────────────────────────────────────────

/// Compile the search query into a `regex::Regex`. For non-regex mode the query is `regex::escape`'d
/// first so `.`/`*`/etc. are LITERAL (RISK-8). Case-insensitivity is the `(?i)` flag when not
/// case-sensitive. An empty query or an invalid pattern is an `Err(String)` (PT-4) — never a panic. The
/// Rust `regex` crate is linear-time (no catastrophic backtracking), so no thread-timeout is needed
/// (the MT-018 lesson).
pub fn compile_search_regex(query: &str, opts: MatchOptions) -> Result<Regex, String> {
    if query.trim().is_empty() {
        return Err("Search query is required".to_owned());
    }
    let base = if opts.is_regex {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    let pattern = if opts.case_sensitive {
        base
    } else {
        format!("(?i){base}")
    };
    Regex::new(&pattern).map_err(|e| format!("Invalid regular expression: {e}"))
}

// ── Whole-word boundary (mirrors the React isWordBoundary) ────────────────────────────────────────────

/// A Unicode word char (letter, number, or underscore) — the React `WORD_CHAR` = `[\p{L}\p{N}_]`.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a match `[start, end)` (BYTE indices into `text`) sits on a whole-word boundary, mirroring
/// the React `isWordBoundary` (`WorkspaceSearchPanel.tsx:222-230`): if the match STARTS on a word char
/// and the char immediately before is also a word char, the boundary fails; symmetrically for the end.
fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let starts_on_word = text[start..].chars().next().is_some_and(is_word_char);
    let ends_on_word = text[..end].chars().next_back().is_some_and(is_word_char);
    if starts_on_word && before.is_some_and(is_word_char) {
        return false;
    }
    if ends_on_word && after.is_some_and(is_word_char) {
        return false;
    }
    true
}

// ── Replacement-group expansion (mirrors the React expandReplacement) ─────────────────────────────────

/// Expand `$1..$9`, `$&` (whole match), and `$$` (literal `$`) in a regex-mode replacement template,
/// mirroring the React `expandReplacement` (`WorkspaceSearchPanel.tsx:232-239`). `groups[i]` is the
/// i-th capture group's text (empty string for an unmatched optional group). For NON-regex mode the
/// caller passes the literal replacement and never calls this.
fn expand_replacement(template: &str, match_text: &str, groups: &[String]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('$') => {
                chars.next();
                out.push('$');
            }
            Some('&') => {
                chars.next();
                out.push_str(match_text);
            }
            Some(d @ '1'..='9') => {
                chars.next();
                let idx = (d as usize) - ('1' as usize);
                if let Some(g) = groups.get(idx) {
                    out.push_str(g);
                }
            }
            // A `$` not followed by a recognized token is emitted literally (React's regex only
            // matches `$($|&|[1-9])`, leaving any other `$x` untouched).
            _ => out.push('$'),
        }
    }
    out
}

// ── Per-match preview (24-char context, mirrors the React replacementMatchPreview) ────────────────────

/// One match's before/after preview snippet (24-char context each side).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPreview {
    pub before_preview: String,
    pub after_preview: String,
}

/// Build a 24-char-context before/after preview for a single match `[start, end)` (BYTE indices) being
/// replaced by `inserted`, mirroring the React `replacementMatchPreview`. Slicing is done on CHAR
/// boundaries so a multibyte char near the window edge never panics.
fn match_preview(text: &str, start: usize, end: usize, inserted: &str) -> MatchPreview {
    let preview_start =
        floor_char_boundary(text, start.saturating_sub(MATCH_PREVIEW_CONTEXT_CHARS));
    let preview_end = ceil_char_boundary(text, (end + MATCH_PREVIEW_CONTEXT_CHARS).min(text.len()));
    let before_preview = text[preview_start..preview_end].to_owned();
    let after_preview = format!(
        "{}{}{}",
        &text[preview_start..start],
        inserted,
        &text[end..preview_end]
    );
    MatchPreview {
        before_preview,
        after_preview,
    }
}

/// Round `idx` DOWN to the nearest char boundary in `s` (std's `floor_char_boundary` is unstable, so
/// this is the stable equivalent the preview slicing needs to stay panic-free on multibyte text).
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Round `idx` UP to the nearest char boundary in `s`.
fn ceil_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

// ── Segment replace (mirrors the React replaceSegment, RISK-3 zero-length guard) ──────────────────────

/// The result of replacing in ONE text segment: the new text, the match count, and per-match previews.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentReplaceResult {
    pub text: String,
    pub count: usize,
    pub match_previews: Vec<MatchPreview>,
}

/// Replace every (whole-word-respecting) match of `regex` in `text` with `replacement`, mirroring the
/// React `replaceSegment` (`WorkspaceSearchPanel.tsx:249-288`):
/// - A ZERO-LENGTH match advances the scan cursor by one CHAR and is never replaced (RISK-3 — no
///   infinite loop).
/// - When `whole_word`, a match that fails [`is_word_boundary`] is SKIPPED (left as-is).
/// - In regex mode the replacement is `$1..$9`/`$&`/`$$`-expanded per match; in literal mode the
///   replacement is inserted verbatim.
///
/// Returns the rebuilt text + the count + a 24-char preview per replaced match.
pub fn replace_segment(
    text: &str,
    regex: &Regex,
    replacement: &str,
    opts: MatchOptions,
) -> SegmentReplaceResult {
    let mut next = String::with_capacity(text.len());
    let mut last_index = 0usize; // byte index of the end of the last copied region
    let mut count = 0usize;
    let mut match_previews = Vec::new();
    let mut search_from = 0usize; // byte index the next find starts at

    loop {
        // Terminate once the scan cursor passes the end of the text (a find AT len can still match a
        // zero-length pattern, so the guard is `> len`, and the zero-length branch below advances past
        // len to break — RISK-3 no-infinite-loop).
        if search_from > text.len() {
            break;
        }
        let Some(m) = regex.find_at(text, search_from) else {
            break;
        };
        let start = m.start();
        let end = m.end();
        if start == end {
            // Zero-length match: advance by one char and never replace (RISK-3). When the match is at
            // the very end of the text, advancing past `len` makes the next loop iteration break — so a
            // zero-length-capable pattern like `a*` always terminates.
            search_from = if start >= text.len() {
                text.len() + 1
            } else {
                ceil_char_boundary(text, start + 1)
            };
            continue;
        }
        if opts.whole_word && !is_word_boundary(text, start, end) {
            // Not a whole-word boundary: leave this match as-is, continue scanning after it.
            search_from = end;
            continue;
        }
        let inserted = if opts.is_regex {
            let caps = regex.captures_at(text, start);
            let groups: Vec<String> = match caps {
                Some(caps) => (1..caps.len())
                    .map(|i| {
                        caps.get(i)
                            .map(|g| g.as_str().to_owned())
                            .unwrap_or_default()
                    })
                    .collect(),
                None => Vec::new(),
            };
            expand_replacement(replacement, &text[start..end], &groups)
        } else {
            replacement.to_owned()
        };
        next.push_str(&text[last_index..start]);
        next.push_str(&inserted);
        last_index = end;
        count += 1;
        match_previews.push(match_preview(text, start, end, &inserted));
        search_from = end;
    }

    if count == 0 {
        return SegmentReplaceResult {
            text: text.to_owned(),
            count: 0,
            match_previews: Vec::new(),
        };
    }
    next.push_str(&text[last_index..]);
    SegmentReplaceResult {
        text: next,
        count,
        match_previews,
    }
}

// ── content_json walk (mirrors the React replaceInContent, RISK-4) ────────────────────────────────────

/// The result of walking a whole document's content_json: the replaced tree, the total match count,
/// the first match's before/after snapshot (whole-text), and the flattened per-match previews.
#[derive(Debug, Clone, PartialEq)]
pub struct ContentReplaceResult {
    pub content: serde_json::Value,
    pub count: usize,
    pub before_preview: String,
    pub after_preview: String,
    pub match_previews: Vec<MatchPreview>,
}

/// Recursively replace in a ProseMirror-style content_json tree (`serde_json::Value`), mirroring the
/// React `replaceInContent` (`WorkspaceSearchPanel.tsx:290-342`). ONLY two string fields are mutated —
/// `node.text` and `node.attrs.code` (code-block content) — and EVERY other node/field is round-tripped
/// VERBATIM (RISK-4: hsLink/embed/table/transclusion nodes are preserved). Recurses into `node.content`.
/// `before_preview`/`after_preview` capture the FIRST mutated text segment's whole-text before/after (the
/// React `??=` first-set semantics).
pub fn replace_in_content(
    content: &serde_json::Value,
    regex: &Regex,
    replacement: &str,
    opts: MatchOptions,
) -> ContentReplaceResult {
    let mut count = 0usize;
    let mut before_preview: Option<String> = None;
    let mut after_preview: Option<String> = None;
    let mut match_previews: Vec<MatchPreview> = Vec::new();
    let new_content = visit_node(
        content,
        regex,
        replacement,
        opts,
        &mut count,
        &mut before_preview,
        &mut after_preview,
        &mut match_previews,
    );
    ContentReplaceResult {
        content: new_content,
        count,
        before_preview: before_preview.unwrap_or_default(),
        after_preview: after_preview.unwrap_or_default(),
        match_previews,
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_node(
    node: &serde_json::Value,
    regex: &Regex,
    replacement: &str,
    opts: MatchOptions,
    count: &mut usize,
    before_preview: &mut Option<String>,
    after_preview: &mut Option<String>,
    match_previews: &mut Vec<MatchPreview>,
) -> serde_json::Value {
    // Non-object nodes (string/number/array elements handled by their parent) round-trip verbatim.
    let serde_json::Value::Object(map) = node else {
        return node.clone();
    };
    let mut next = map.clone();

    // 1) text node: mutate `node.text`.
    if let Some(serde_json::Value::String(text)) = map.get("text") {
        let replaced = replace_segment(text, regex, replacement, opts);
        if replaced.count > 0 {
            if before_preview.is_none() {
                *before_preview = Some(text.clone());
                *after_preview = Some(replaced.text.clone());
            }
            *count += replaced.count;
            match_previews.extend(replaced.match_previews);
            next.insert("text".to_owned(), serde_json::Value::String(replaced.text));
        }
    }

    // 2) code-block: mutate `node.attrs.code` (RISK-4 — code-block content must be searched too).
    if let Some(serde_json::Value::Object(attrs)) = map.get("attrs") {
        if let Some(serde_json::Value::String(code)) = attrs.get("code") {
            let replaced = replace_segment(code, regex, replacement, opts);
            if replaced.count > 0 {
                if before_preview.is_none() {
                    *before_preview = Some(code.clone());
                    *after_preview = Some(replaced.text.clone());
                }
                *count += replaced.count;
                match_previews.extend(replaced.match_previews);
                let mut new_attrs = attrs.clone();
                new_attrs.insert("code".to_owned(), serde_json::Value::String(replaced.text));
                next.insert("attrs".to_owned(), serde_json::Value::Object(new_attrs));
            }
        }
    }

    // 3) recurse into `node.content` (every child round-trips, mutated children replaced).
    if let Some(serde_json::Value::Array(children)) = map.get("content") {
        let new_children: Vec<serde_json::Value> = children
            .iter()
            .map(|child| {
                visit_node(
                    child,
                    regex,
                    replacement,
                    opts,
                    count,
                    before_preview,
                    after_preview,
                    match_previews,
                )
            })
            .collect();
        next.insert("content".to_owned(), serde_json::Value::Array(new_children));
    }

    serde_json::Value::Object(next)
}

// ── documentId-from-hit (mirrors the React documentIdFromLoomSearchHit, RISK-5) ───────────────────────

/// Extract the editable rich-document id for a search hit, mirroring the React
/// `documentIdFromLoomSearchHit` (`loom_search_open_target.ts:15-22`): try `metadata.rich_document_id`,
/// then `metadata.document_id`, then `block.document_id`, then (when `source_kind == "document"`) the
/// `ref_id`; accept the candidate ONLY if it starts with `KRD-` (RISK-5 — a non-rich-document id would
/// 404 the load). Returns `None` when no `KRD-` id is found.
pub fn document_id_from_hit(hit: &LoomGraphSearchHit) -> Option<String> {
    let metadata_str = |key: &str| -> Option<String> {
        hit.metadata
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let block_document_id = || -> Option<String> {
        hit.block
            .as_ref()
            .and_then(|b| b.get("document_id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let candidate = metadata_str("rich_document_id")
        .or_else(|| metadata_str("document_id"))
        .or_else(block_document_id)
        .or_else(|| {
            if hit.source_kind == "document" {
                let r = hit.ref_id.trim();
                (!r.is_empty()).then(|| r.to_owned())
            } else {
                None
            }
        });
    candidate.filter(|c| c.starts_with("KRD-"))
}

fn shared_find_block_field(hit: &LoomGraphSearchHit, field: &str) -> Option<String> {
    hit.block
        .as_ref()
        .and_then(|block| block.get(field))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// Project raw MT-029 graph-search rows into the two typed editor result lanes carried by the shared
/// InteractionBus. Classification is producer-backed: code files are `source_kind=file` or Loom blocks
/// whose `content_type=file`; notes are rich-document rows or Loom blocks whose `content_type=note`.
/// Unrelated graph entities remain in Find-in-Files but are intentionally absent from this editor bridge.
pub fn shared_editor_find_entries(
    hits: &[LoomGraphSearchHit],
) -> (
    Vec<crate::interop::SharedCodeFindEntry>,
    Vec<crate::interop::SharedNoteFindEntry>,
) {
    let mut code: Vec<crate::interop::SharedCodeFindEntry> = Vec::new();
    let mut note: Vec<crate::interop::SharedNoteFindEntry> = Vec::new();
    let mut code_identities = std::collections::HashSet::new();
    let mut note_identities = std::collections::HashMap::<String, usize>::new();
    for hit in hits {
        let content_type = shared_find_block_field(hit, "content_type").unwrap_or_else(|| {
            if hit.source_kind == "file" {
                "file".to_owned()
            } else if hit.source_kind == "document" {
                "knowledge_rich_document".to_owned()
            } else {
                String::new()
            }
        });
        let block_id = shared_find_block_field(hit, "block_id");
        if hit.source_kind == "file" || content_type == "file" || content_type == "code_file" {
            let identity = block_id.clone().unwrap_or_else(|| hit.ref_id.clone());
            if !code_identities.insert(identity) {
                continue;
            }
            code.push(crate::interop::SharedCodeFindEntry {
                source_kind: hit.source_kind.clone(),
                result_kind: hit.result_kind.clone(),
                ref_id: hit.ref_id.clone(),
                block_id,
                content_type,
                title: hit.title.clone(),
                excerpt: hit.excerpt.clone(),
            });
            continue;
        }
        let authority_table = hit
            .metadata
            .get("authority_table")
            .and_then(serde_json::Value::as_str);
        if hit.source_kind == "document"
            || content_type == "note"
            || content_type == "knowledge_rich_document"
            || authority_table == Some("knowledge_rich_documents")
        {
            let document_id = document_id_from_hit(hit);
            let identity = document_id
                .clone()
                .or_else(|| block_id.clone())
                .unwrap_or_else(|| hit.ref_id.clone());
            let entry = crate::interop::SharedNoteFindEntry {
                source_kind: hit.source_kind.clone(),
                result_kind: hit.result_kind.clone(),
                ref_id: hit.ref_id.clone(),
                block_id,
                document_id,
                content_type,
                title: hit.title.clone(),
                excerpt: hit.excerpt.clone(),
            };
            if let Some(index) = note_identities.get(&identity).copied() {
                // Prefer the authority `document` row over its derived Loom-note projection while
                // retaining one canonical note result on the shared editor lane.
                if hit.source_kind == "document" && note[index].source_kind != "document" {
                    note[index] = entry;
                }
            } else {
                note_identities.insert(identity, note.len());
                note.push(entry);
            }
        }
    }
    (code, note)
}

/// Typed shell-open target for a clicked Find-in-Files result. Find-in-Files intentionally reuses the
/// production quick-switcher target model so every source kind resolves through the same exhaustive,
/// field-tested route mapping instead of silently collapsing non-document hits into Loom blocks.
pub type FindInFilesOpenTarget = crate::quick_switcher::QuickSwitcherTarget;

pub fn shell_open_target_from_hit(hit: &LoomGraphSearchHit) -> Option<FindInFilesOpenTarget> {
    let canonical_hit = crate::quick_switcher::LoomGraphSearchHit {
        result_kind: hit.result_kind.clone(),
        source_kind: hit.source_kind.clone(),
        ref_id: hit.ref_id.clone(),
        title: hit.title.clone(),
        excerpt: hit.excerpt.clone(),
        block: hit.block.clone().unwrap_or(serde_json::Value::Null),
        score: 0.0,
        metadata: hit.metadata.clone(),
    };
    let target = crate::quick_switcher::resolve_open_target(&canonical_hit);
    target.enabled().then_some(target)
}

pub fn dispatch_shell_open_target(
    navigator: &mut dyn crate::quick_switcher::ShellNavigator,
    target: &FindInFilesOpenTarget,
) -> crate::quick_switcher::NavDispatchOutcome {
    crate::quick_switcher::dispatch_target(navigator, target)
}

// ── Client-side option filter (mirrors the React hitMatchesClientOptions) ─────────────────────────────

/// Whether a hit passes the client-side option filter, mirroring the React `hitMatchesClientOptions`
/// (`WorkspaceSearchPanel.tsx:344-361`): when NONE of case/word/regex is set, every hit passes; otherwise
/// the compiled regex must match the `title\nexcerpt` haystack respecting the whole-word boundary. A
/// query that fails to compile passes everything (the backend already filtered).
pub fn hit_matches_client_options(
    hit: &LoomGraphSearchHit,
    query: &str,
    opts: MatchOptions,
) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    if !opts.case_sensitive && !opts.whole_word && !opts.is_regex {
        return true;
    }
    let Ok(regex) = compile_search_regex(query, opts) else {
        return true;
    };
    hit_matches_regex(hit, &regex, opts)
}

/// Whether a hit's `title\nexcerpt` haystack matches an ALREADY-COMPILED `regex`, respecting the
/// whole-word boundary. Split out from [`hit_matches_client_options`] so the render path can compile the
/// regex ONCE per (query, options) change and reuse it across all hits (perf hygiene) instead of
/// recompiling per hit per frame.
pub fn hit_matches_regex(hit: &LoomGraphSearchHit, regex: &Regex, opts: MatchOptions) -> bool {
    let haystack = format!("{}\n{}", hit.title, hit.excerpt);
    let mut search_from = 0usize;
    loop {
        if search_from > haystack.len() {
            return false;
        }
        let Some(m) = regex.find_at(&haystack, search_from) else {
            return false;
        };
        let (start, end) = (m.start(), m.end());
        if start == end {
            // Zero-length match: advance past it (and past `len` at the end) so the scan terminates.
            search_from = if start >= haystack.len() {
                haystack.len() + 1
            } else {
                ceil_char_boundary(&haystack, start + 1)
            };
            continue;
        }
        if !opts.whole_word || is_word_boundary(&haystack, start, end) {
            return true;
        }
        search_from = end;
    }
}

// ── State keys (RISK-2/MC-2 stale guards) ─────────────────────────────────────────────────────────────

/// A deterministic string key for the current SEARCH params (query + kind + filters + options). Two
/// searches with identical params yield the same key; any change yields a different key. Used as
/// `result_set_key` so Preview Replace can detect a since-changed query (RISK-2). Built from a normalized
/// tuple serialized to JSON (stable field order).
pub fn search_plan_key(
    query: &str,
    kind: KindFilter,
    tag_filter: &str,
    path_filter: &str,
    opts: MatchOptions,
) -> String {
    let normalized = serde_json::json!({
        "query": query.trim(),
        "kind": kind.wire(),
        "tag": tag_filter.trim(),
        "path": path_filter.trim(),
        "case": opts.case_sensitive,
        "word": opts.whole_word,
        "regex": opts.is_regex,
    });
    normalized.to_string()
}

/// A deterministic key for a REPLACE plan: the search key + the replacement text. Used as
/// `preview_plan_key` so Apply can detect a since-changed search-or-replacement (RISK-2/MC-2).
pub fn replace_plan_key(search_key: &str, replacement: &str) -> String {
    serde_json::json!({ "search": search_key, "replacement": replacement }).to_string()
}

// ── Replacement plan ──────────────────────────────────────────────────────────────────────────────────

/// One document's planned replacement (the preview unit). Carries the `expected_version` captured at
/// preview so Apply's save uses optimistic concurrency (a 409 = the doc changed since preview → NO
/// overwrite, RISK-2 data-loss control).
#[derive(Debug, Clone, PartialEq)]
pub struct ReplacementPlan {
    /// Workspace authority loaded during Preview and reverified immediately before Apply.
    pub workspace_id: String,
    pub document_id: String,
    pub title: String,
    pub expected_version: u64,
    pub content_json_after: serde_json::Value,
    /// SHA-256 of the exact persisted JSON loaded during preview.
    pub before_sha256: String,
    /// SHA-256 of the exact JSON submitted by Apply.
    pub after_sha256: String,
    pub crdt_document_id: Option<String>,
    pub match_count: usize,
    pub before_preview: String,
    pub after_preview: String,
    pub match_previews: Vec<MatchPreview>,
}

pub fn content_json_sha256(content: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let encoded = serde_json::to_vec(content).expect("serde_json::Value always serializes");
    format!("{:x}", Sha256::digest(encoded))
}

// ── Bookmark ──────────────────────────────────────────────────────────────────────────────────────────

/// One saved search bookmark (the React `WorkspaceSearchBookmark`). Round-trips through the
/// `bookmark_state` blob.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchBookmark {
    pub id: String,
    pub label: String,
    pub query: String,
    pub kind: KindFilter,
    pub tag_filter: String,
    pub path_filter: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
    pub saved_at: String,
}

impl SearchBookmark {
    /// A stable, route-safe id derived from the exact semantic search tuple. Each component is framed
    /// by its UTF-8 byte length and encoded with lowercase bytewise hex, so case, Unicode, empty fields,
    /// and option values cannot collapse onto another saved search. Re-saving the same semantic search
    /// still replaces (dedups) the prior bookmark.
    ///
    /// MT-113: the id is BOUNDED at [`MAX_BOOKMARK_STABLE_ID_BYTES`]. A saved search whose verbatim id
    /// would overrun switches every component to its `zsha256-` digest behind the SAME `.{len}-` framing.
    /// That keeps the id injective (the byte length is still framed and the digest is collision
    /// resistant) and keeps dedup exact (it stays a pure function of the semantic tuple), while
    /// preventing a long query from pushing the derived `bookmark_remove_semantic_value` past the
    /// canonical 2048-byte `semantic_value` budget — the same silent-token failure class this MT closes
    /// for `pending_target`. Every id that fits today is byte-identical.
    pub fn stable_id(&self) -> String {
        let components = [
            self.query.trim(),
            self.kind.wire(),
            self.tag_filter.trim(),
            self.path_filter.trim(),
            if self.case_sensitive { "true" } else { "false" },
            if self.whole_word { "true" } else { "false" },
            if self.is_regex { "true" } else { "false" },
        ];

        let compose = |digested: bool| {
            let mut stable = String::from("bookmark-v1");
            for component in components {
                use std::fmt::Write as _;
                let _ = write!(stable, ".{}-", component.len());
                stable.push_str(&if digested {
                    digest_author_id_component(component)
                } else {
                    encode_author_id_component(component)
                });
            }
            stable
        };

        let verbatim = compose(false);
        if verbatim.len() <= MAX_BOOKMARK_STABLE_ID_BYTES {
            return verbatim;
        }
        let digested = compose(true);
        debug_assert!(
            digested.len() <= MAX_BOOKMARK_STABLE_ID_BYTES,
            "digested bookmark stable_id is {} bytes, over the {MAX_BOOKMARK_STABLE_ID_BYTES}-byte bound",
            digested.len()
        );
        digested
    }

    /// The display label (the React `bookmarkLabelForSearch`): the query if any, else the kind/filters.
    pub fn display_label(&self) -> String {
        let q = self.query.trim();
        if !q.is_empty() {
            return q.to_owned();
        }
        let kind_label = if self.kind == KindFilter::All {
            ""
        } else {
            self.kind.label()
        };
        let parts: Vec<&str> = [kind_label, self.tag_filter.trim(), self.path_filter.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            "Filtered search".to_owned()
        } else {
            parts.join(" / ")
        }
    }

    /// Serialize to the per-bookmark JSON shape the `bookmark_state.bookmarks[]` blob carries (React
    /// `WorkspaceSearchBookmark` field names — camelCase, since the React reader keys on them).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "label": self.label,
            "query": self.query,
            "kind": self.kind.wire(),
            "tagFilter": self.tag_filter,
            "pathFilter": self.path_filter,
            "caseSensitive": self.case_sensitive,
            "wholeWord": self.whole_word,
            "isRegex": self.is_regex,
            "savedAt": self.saved_at,
        })
    }

    /// Parse one bookmark entry from the blob; every required field and enum token fails closed.
    pub fn from_json(v: &serde_json::Value) -> Result<SearchBookmark, String> {
        let object = v
            .as_object()
            .ok_or_else(|| "bookmark must be an object".to_owned())?;
        let s = |k: &str| {
            object
                .get(k)
                .and_then(|x| x.as_str())
                .map(str::to_owned)
                .ok_or_else(|| format!("bookmark.{k} missing or not a string"))
        };
        let b = |k: &str| {
            object
                .get(k)
                .and_then(|x| x.as_bool())
                .ok_or_else(|| format!("bookmark.{k} missing or not a bool"))
        };
        let id = s("id")?;
        let label = s("label")?;
        if id.trim().is_empty() || label.trim().is_empty() {
            return Err("bookmark id and label must not be blank".to_owned());
        }
        let saved_at = s("savedAt")?;
        if saved_at.trim().is_empty() {
            return Err("bookmark.savedAt must not be blank".to_owned());
        }
        chrono::DateTime::parse_from_rfc3339(&saved_at)
            .map_err(|_| "bookmark.savedAt must be an RFC3339 timestamp".to_owned())?;
        Ok(SearchBookmark {
            id,
            label,
            query: s("query")?,
            kind: KindFilter::from_wire(&s("kind")?)?,
            tag_filter: s("tagFilter")?,
            path_filter: s("pathFilter")?,
            case_sensitive: b("caseSensitive")?,
            whole_word: b("wholeWord")?,
            is_regex: b("isRegex")?,
            saved_at,
        })
    }
}

/// Build the `bookmark_state` blob from a bookmark list (the React `workspaceSearchBookmarkBlob`): the
/// REQUIRED `schema_id` (RISK-6) + the `bookmarks` array, capped at [`MAX_WORKSPACE_SEARCH_BOOKMARKS`].
pub fn bookmark_state_blob(bookmarks: &[SearchBookmark]) -> serde_json::Value {
    let capped: Vec<serde_json::Value> = bookmarks
        .iter()
        .take(MAX_WORKSPACE_SEARCH_BOOKMARKS)
        .map(SearchBookmark::to_json)
        .collect();
    serde_json::json!({
        "schema_id": WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID,
        "bookmarks": capped,
    })
}

/// Strictly parse persisted bookmark state. An absent state (`{}`/`null`) is an empty list, while a
/// present but malformed schema fails closed instead of silently deleting or hiding entries.
pub fn parse_bookmark_state(blob: &serde_json::Value) -> Result<Vec<SearchBookmark>, String> {
    if blob.is_null() || blob.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(Vec::new());
    }
    let object = blob
        .as_object()
        .ok_or_else(|| "bookmark_state must be an object".to_owned())?;
    if object.get("schema_id").and_then(serde_json::Value::as_str)
        != Some(WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID)
    {
        return Err("bookmark_state schema_id is missing or unsupported".to_owned());
    }
    let entries = object
        .get("bookmarks")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "bookmark_state.bookmarks must be an array".to_owned())?;
    if entries.len() > MAX_WORKSPACE_SEARCH_BOOKMARKS {
        return Err(format!(
            "bookmark_state contains {} entries; maximum is {MAX_WORKSPACE_SEARCH_BOOKMARKS}",
            entries.len()
        ));
    }
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            SearchBookmark::from_json(entry)
                .map_err(|error| format!("bookmark_state.bookmarks[{index}] is malformed: {error}"))
        })
        .collect()
}

// ── Panel state machine ───────────────────────────────────────────────────────────────────────────────

/// One durable, product-side action-completion observer for an asynchronous Find-in-Files action.
///
/// The generation advances ONLY where the panel actually consumed an activation, and the terminal
/// transition is published ONLY from the authoritative delivery that owns the effect. This is what makes
/// a canonical Argus receipt `Applied` instead of `Indeterminate`: the acknowledgement is causal, not a
/// value echo and not the target disappearing.
#[derive(Debug, Clone)]
struct ActionCompletion {
    generation: u64,
    state: crate::mcp::action::ClickCompletionState,
    target: Option<String>,
    semantic: Option<String>,
    error: Option<String>,
    detail: Option<String>,
}

impl ActionCompletion {
    fn new() -> Self {
        Self {
            generation: 0,
            state: crate::mcp::action::ClickCompletionState::Ready,
            target: None,
            semantic: None,
            error: None,
            detail: None,
        }
    }

    /// Record that the panel consumed an activation of `target`. `semantic` MUST be stable across the
    /// whole action (the MCP boundary re-reads the post-dispatch declaration and requires an identical
    /// semantic value), so it names WHAT is being acted on, never the state the action mutates.
    fn begin(&mut self, target: String, semantic: String) {
        self.generation = self.generation.wrapping_add(1);
        self.state = crate::mcp::action::ClickCompletionState::Pending;
        self.target = Some(target);
        self.semantic = Some(semantic);
        self.error = None;
        self.detail = None;
    }

    fn complete(&mut self, detail: String) {
        if self.state == crate::mcp::action::ClickCompletionState::Pending {
            self.state = crate::mcp::action::ClickCompletionState::Applied;
            self.detail = Some(detail);
        }
    }

    /// Publish a TYPED terminal failure. The message carries a product-owned `"<effect> failed: "`
    /// envelope so an external verifier can bind the exact failing effect deterministically even when
    /// the underlying transport text is platform-dependent.
    fn fail(&mut self, effect: &str, message: impl AsRef<str>) {
        if self.state == crate::mcp::action::ClickCompletionState::Pending {
            self.state = crate::mcp::action::ClickCompletionState::Failed;
            self.error = Some(bounded_token_field(
                &format!("{effect} failed: {}", message.as_ref()),
                512,
            ));
        }
    }

    fn is_pending(&self) -> bool {
        self.state == crate::mcp::action::ClickCompletionState::Pending
    }

    /// Release a standing terminal result WITHOUT rewinding the generation, so the next declaration and
    /// the observer stay on the same monotonic counter.
    fn reset_ready(&mut self) {
        if self.state != crate::mcp::action::ClickCompletionState::Pending {
            self.state = crate::mcp::action::ClickCompletionState::Ready;
            self.target = None;
            self.semantic = None;
            self.error = None;
            self.detail = None;
        }
    }

    /// Declaration for a control that stays mounted through its own action (Search/Preview/Apply/
    /// Cancel/Bookmark Search). A disabled-while-in-flight target is explicitly allowed by the MCP
    /// boundary for the persistent + Pending case.
    fn persistent_declaration(
        &self,
        effect: &str,
        context: &str,
        observer: &str,
        semantic: &str,
    ) -> Option<String> {
        crate::mcp::action::serialize_persistent_observer_click_target(
            effect,
            context,
            self.generation,
            observer,
            semantic,
        )
    }

    /// Declaration for a control that is WITHDRAWN by its own action (the destructive Apply). The MCP
    /// boundary requires such a transient target to be absent at acknowledgement, which is exactly the
    /// production shape once the applied plans are consumed.
    fn observer_declaration(
        &self,
        effect: &str,
        context: &str,
        observer: &str,
        semantic: &str,
    ) -> Option<String> {
        crate::mcp::action::serialize_observer_click_target(
            effect,
            context,
            self.generation,
            observer,
            semantic,
        )
    }

    /// Declaration for a control whose SUCCESS removes it and whose typed FAILURE leaves it mounted
    /// (bookmark Remove rows, the bookmark-load Retry button).
    fn flexible_declaration(
        &self,
        effect: &str,
        context: &str,
        observer: &str,
        semantic: &str,
    ) -> Option<String> {
        crate::mcp::action::serialize_flexible_observer_click_target(
            effect,
            context,
            self.generation,
            observer,
            semantic,
        )
    }

    /// Publish the observer's state, or — when the token genuinely cannot be composed — the TYPED
    /// MT-113 completion-unavailable marker in its place.
    ///
    /// Before MT-113 a failed composition simply produced `None`: the observer node carried NO value,
    /// the MCP boundary saw an absent token, and the action terminalised `indeterminate` with no
    /// diagnostic anywhere. Composition is now bounded so an over-budget author id cannot reach this
    /// point, but the marker keeps the failure NAMED rather than silent for every remaining cause.
    fn observer_value(&self, effect: &str, context: &str) -> Option<String> {
        self.observer_token_value(effect, context).or_else(|| {
            let (field, bytes, budget) = self.token_overrun()?;
            crate::mcp::action::click_completion_unavailable_value(
                effect,
                context,
                self.generation,
                field,
                bytes,
                budget,
            )
        })
    }

    /// The exact token field that cannot be carried, measured against its canonical budget.
    fn token_overrun(&self) -> Option<(&'static str, usize, usize)> {
        if let Some(target) = self.target.as_deref() {
            if target.len() > MAX_COMPLETION_TARGET_AUTHOR_BYTES {
                return Some((
                    "pending_target",
                    target.len(),
                    MAX_COMPLETION_TARGET_AUTHOR_BYTES,
                ));
            }
        }
        if let Some(semantic) = self.semantic.as_deref() {
            if semantic.len() > MAX_COMPLETION_SEMANTIC_BYTES {
                return Some((
                    "semantic_value",
                    semantic.len(),
                    MAX_COMPLETION_SEMANTIC_BYTES,
                ));
            }
        }
        None
    }

    fn observer_token_value(&self, effect: &str, context: &str) -> Option<String> {
        match self.state {
            crate::mcp::action::ClickCompletionState::Ready => {
                crate::mcp::action::serialize_observer_click_state(
                    effect,
                    context,
                    self.generation,
                    self.state,
                    None,
                    None,
                )
            }
            crate::mcp::action::ClickCompletionState::Pending => {
                crate::mcp::action::serialize_observer_click_state(
                    effect,
                    context,
                    self.generation,
                    self.state,
                    self.target.as_deref(),
                    self.semantic.as_deref(),
                )
            }
            crate::mcp::action::ClickCompletionState::Applied => {
                crate::mcp::action::serialize_observer_click_applied(
                    effect,
                    context,
                    self.generation,
                    self.target.as_deref()?,
                    self.semantic.as_deref()?,
                    self.detail.as_deref().unwrap_or("{}"),
                )
            }
            crate::mcp::action::ClickCompletionState::Failed => {
                crate::mcp::action::serialize_observer_click_failure(
                    effect,
                    context,
                    self.generation,
                    self.target.as_deref()?,
                    self.semantic.as_deref()?,
                    self.error.as_deref().unwrap_or("action failed"),
                    self.detail.as_deref(),
                )
            }
        }
    }
}

/// Emit a stable, Argus-addressable `Role::Status` projection node. Completion observers and boolean
/// state projections both use this: the node outlives the control it describes, which is exactly what
/// lets an acknowledgement survive its target disappearing.
fn emit_status_node(ui: &egui::Ui, author_id: &str, label: &str, value: Option<String>) {
    let node_id = ui.make_persistent_id(author_id);
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(node_id, move |node| {
        node.set_role(egui::accesskit::Role::Status);
        node.set_author_id(author_id.clone());
        node.set_label(label.clone());
        if let Some(value) = value.clone() {
            node.set_value(value);
        }
    });
}

/// Emit the canonical sibling SetValue acknowledgement for a text target. The displayed value cannot
/// carry completion metadata, so `<target>.set-value-completion` publishes the exact
/// target/generation/value tuple only after the production widget consumed the AccessKit request.
fn emit_set_value_completion_node(
    ui: &egui::Ui,
    target_author_id: &str,
    label: &str,
    generation: u64,
    applied: Option<&str>,
) {
    let completion_author_id = crate::mcp::action::set_value_completion_author_id(target_author_id);
    let value =
        crate::mcp::action::serialize_set_value_completion(target_author_id, generation, applied);
    emit_status_node(ui, &completion_author_id, label, value);
}

/// Clamp a completion-token field to the MCP byte budget on a char boundary and strip control
/// characters, so a long backend error can never silently invalidate the whole terminal token.
fn bounded_token_field(value: &str, max_bytes: usize) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    if sanitized.len() <= max_bytes {
        return sanitized;
    }
    let mut end = max_bytes;
    while end > 0 && !sanitized.is_char_boundary(end) {
        end -= 1;
    }
    sanitized[..end].to_owned()
}

/// The exact terminal proof for a destructive Apply: per-document before/after content hashes, the
/// typed outcome, and the EventLedger save receipt id. This is bound INTO the Argus completion token,
/// so an accepted Apply receipt cannot be satisfied by an echo or by the target disappearing.
fn apply_terminal_detail(kind: &str, audit_receipts: &[ReplaceAuditReceipt]) -> String {
    let mut saved = 0usize;
    let mut conflict = 0usize;
    let mut failed = 0usize;
    let mut committed_without_receipt = 0usize;
    for receipt in audit_receipts {
        match receipt.outcome {
            ReplaceAuditOutcome::Saved => saved += 1,
            ReplaceAuditOutcome::CommittedWithoutReceipt => committed_without_receipt += 1,
            ReplaceAuditOutcome::Conflict => conflict += 1,
            ReplaceAuditOutcome::Failed => failed += 1,
        }
    }
    let rows: Vec<serde_json::Value> = audit_receipts
        .iter()
        .take(MAX_TERMINAL_DETAIL_AUDIT_ROWS)
        .map(|receipt| {
            serde_json::json!({
                "document_id": receipt.document_id,
                "outcome": format!("{:?}", receipt.outcome),
                "before_sha256": receipt.before_sha256,
                "after_sha256": receipt.after_sha256,
                "save_receipt_event_id": receipt.save_receipt_event_id,
            })
        })
        .collect();
    // A complete, order-sensitive digest of EVERY audit row, so a truncated `documents` array is still
    // externally checkable against the full delivery.
    let digest_source = audit_receipts
        .iter()
        .map(format_replace_audit_receipt)
        .collect::<Vec<_>>()
        .join("|");
    let audit_digest = {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(digest_source.as_bytes()))
    };
    let detail = serde_json::json!({
        "kind": kind,
        "audit_row_count": audit_receipts.len(),
        "saved": saved,
        "committed_without_receipt": committed_without_receipt,
        "conflict": conflict,
        "failed": failed,
        "documents": rows,
        "documents_truncated": audit_receipts.len() > MAX_TERMINAL_DETAIL_AUDIT_ROWS,
        "audit_digest_sha256": audit_digest,
    })
    .to_string();
    bounded_token_field(&detail, 2000)
}

/// The memoized client-side visible-hit filter (perf hygiene): the compiled regex + the filtered hit
/// index list, valid only while `key` matches the live (query + options + results-generation) key. This
/// hoists the per-hit `compile_search_regex` AND the per-frame filter+clone out of the render hot path —
/// without it, `show()` recompiled one [`Regex`] PER HIT and rebuilt the visible Vec EVERY frame
/// (typing/hover/scroll all repaint) for a paginated result set of hundreds-to-thousands of hits.
struct VisibleCache {
    /// `query + options + results_generation` digest; a mismatch invalidates the cache.
    key: String,
    /// The 0-based indices into `results` that pass the client-side option filter, in result order.
    indices: Vec<usize>,
}

/// All Find-in-Files panel state (the React component's `useState` hooks as one struct), plus the
/// off-thread delivery cells. Mirrors the MT-029 AC-1 required field set.
pub struct FindInFilesPanelState {
    pub query: String,
    pub replacement: String,
    pub kind: KindFilter,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub is_regex: bool,
    pub tag_filter: String,
    pub path_filter: String,
    /// The collected, paginated hits from the last search.
    pub results: Vec<LoomGraphSearchHit>,
    /// `true` while a search / preview / apply request is genuinely in flight (drives the loading
    /// indicator ONLY while pending — never a perpetual spinner).
    pub loading: bool,
    /// The last error string, or `None`.
    pub error: Option<String>,
    /// The last replace/preview/apply status string, or `None`.
    pub replace_status: Option<String>,
    /// The current preview plans (empty until Preview Replace runs).
    pub preview_plans: Vec<ReplacementPlan>,
    /// Preview rows expanded by either pointer or canonical AccessKit activation.
    expanded_preview_document_ids: HashSet<String>,
    /// The replace-plan key the current `preview_plans` were computed under (stale-apply guard).
    pub preview_plan_key: Option<String>,
    /// The search-plan key the current `results` were fetched under (stale-preview guard).
    pub result_set_key: Option<String>,
    /// The saved-search bookmarks.
    pub bookmarks: Vec<SearchBookmark>,
    /// The last bookmark op status string, or `None`.
    pub bookmark_status: Option<String>,
    /// True only after the active workspace's mount/retry bookmark GET terminates unsuccessfully.
    /// Drives the stable Retry control; cleared before each new attempt and after a valid response.
    pub bookmark_load_failed: bool,
    /// Monotonic diagnostic count of real bookmark GET attempts issued by this mounted panel state.
    pub bookmark_load_attempt_count: u64,
    /// Causal acknowledgement for the mounted query field's latest AccessKit SetValue request.
    /// This advances only where the production widget consumes the exact target-specific request.
    query_set_value_generation: u64,
    query_set_value_applied: Option<String>,
    /// Causal acknowledgement for the mounted replacement field's latest AccessKit SetValue request.
    /// The DESTRUCTIVE replacement input is gated behind the in-flight Apply, so this advances only
    /// where the production widget genuinely consumed the exact target-specific request.
    replacement_set_value_generation: u64,
    replacement_set_value_applied: Option<String>,
    tag_filter_set_value_generation: u64,
    tag_filter_set_value_applied: Option<String>,
    path_filter_set_value_generation: u64,
    path_filter_set_value_applied: Option<String>,
    /// Per-target activation counters for synchronous same-target click acknowledgements (match-option
    /// toggles, result rows, preview rows, bookmark Restore). Keyed by the UNSCOPED author id, because
    /// the state itself is already per-pane.
    same_target_activations: BTreeMap<String, u64>,
    /// Shell-owned result-navigation completion binding, pushed per frame by the pane factory. The
    /// unbound compatibility `show()` path has `ready == false` and falls back to a same-target token.
    result_open_completion: FindResultOpenCompletionBinding,
    /// Durable observers for the asynchronous actions.
    preview_action: ActionCompletion,
    apply_action: ActionCompletion,
    cancel_action: ActionCompletion,
    bookmark_action: ActionCompletion,
    bookmark_load_action: ActionCompletion,
    search_action_generation: u64,
    search_action_state: crate::mcp::action::ClickCompletionState,
    search_action_target: Option<String>,
    search_action_semantic: Option<String>,
    search_action_error: Option<String>,
    search_action_detail: Option<String>,

    /// Bumps every time `results` is replaced (in [`poll`](Self::poll)); part of the visible-cache key so
    /// a new result set invalidates the memoized filter even when query+options are unchanged.
    results_generation: u64,
    /// Memoized client-side visible-hit filter (perf hygiene — see [`VisibleCache`]). Interior mutability
    /// lets the `&self` render/status path refresh it lazily without taking `&mut self`.
    visible_cache: std::cell::RefCell<Option<VisibleCache>>,

    // ── Off-thread delivery cells ──
    search_cell: GraphSearchCell,
    /// The replace pipeline (preview document loads + apply saves) runs on a background task and
    /// delivers a typed [`ReplaceDelivery`] into this cell.
    replace_cell: FindReplaceCell,
    bookmark_cell: BookmarkStateCell,
    bound_workspace_id: Option<String>,
    bound_workspace_generation: u64,
    workspace_epoch: u64,
    next_sequence: u64,
    active_search: Option<FindInFilesStamp>,
    active_preview: Option<FindInFilesStamp>,
    active_apply: Option<FindInFilesStamp>,
    active_apply_cancel: Option<Arc<AtomicBool>>,
    active_bookmark_load: Option<FindInFilesStamp>,
    active_bookmark_save: Option<FindInFilesStamp>,
    refresh_search_after_apply: bool,
    refresh_search_workspace_id: Option<String>,
    /// Workspace whose last Apply terminal outcome is currently shown to the operator.
    pub last_apply_terminal_workspace_id: Option<String>,
    /// Workspace whose last bookmark-save terminal outcome is currently shown to the operator.
    pub last_bookmark_save_terminal_workspace_id: Option<String>,
    /// Durable backend event-ledger receipt for the last bookmark-save terminal outcome.
    pub last_bookmark_save_receipt_id: Option<String>,
}

impl Default for FindInFilesPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl FindInFilesPanelState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            replacement: String::new(),
            kind: KindFilter::All,
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            tag_filter: String::new(),
            path_filter: String::new(),
            results: Vec::new(),
            loading: false,
            error: None,
            replace_status: None,
            preview_plans: Vec::new(),
            expanded_preview_document_ids: HashSet::new(),
            preview_plan_key: None,
            result_set_key: None,
            bookmarks: Vec::new(),
            bookmark_status: None,
            bookmark_load_failed: false,
            bookmark_load_attempt_count: 0,
            query_set_value_generation: 0,
            query_set_value_applied: None,
            replacement_set_value_generation: 0,
            replacement_set_value_applied: None,
            tag_filter_set_value_generation: 0,
            tag_filter_set_value_applied: None,
            path_filter_set_value_generation: 0,
            path_filter_set_value_applied: None,
            same_target_activations: BTreeMap::new(),
            result_open_completion: FindResultOpenCompletionBinding::default(),
            preview_action: ActionCompletion::new(),
            apply_action: ActionCompletion::new(),
            cancel_action: ActionCompletion::new(),
            bookmark_action: ActionCompletion::new(),
            bookmark_load_action: ActionCompletion::new(),
            search_action_generation: 0,
            search_action_state: crate::mcp::action::ClickCompletionState::Ready,
            search_action_target: None,
            search_action_semantic: None,
            search_action_error: None,
            search_action_detail: None,
            results_generation: 0,
            visible_cache: std::cell::RefCell::new(None),
            search_cell: Arc::new(Mutex::new(VecDeque::new())),
            replace_cell: Arc::new(Mutex::new(VecDeque::new())),
            bookmark_cell: Arc::new(Mutex::new(VecDeque::new())),
            bound_workspace_id: None,
            bound_workspace_generation: 0,
            workspace_epoch: 0,
            next_sequence: 0,
            active_search: None,
            active_preview: None,
            active_apply: None,
            active_apply_cancel: None,
            active_bookmark_load: None,
            active_bookmark_save: None,
            refresh_search_after_apply: false,
            refresh_search_workspace_id: None,
            last_apply_terminal_workspace_id: None,
            last_bookmark_save_terminal_workspace_id: None,
            last_bookmark_save_receipt_id: None,
        }
    }

    fn next_stamp(
        &mut self,
        workspace_id: &str,
        operation: FindInFilesOperation,
    ) -> FindInFilesStamp {
        self.next_sequence = self.next_sequence.wrapping_add(1);
        FindInFilesStamp {
            workspace_id: workspace_id.to_owned(),
            operation,
            epoch: self.workspace_epoch,
            sequence: self.next_sequence,
        }
    }

    fn refresh_loading(&mut self) {
        self.loading = self.active_search.is_some()
            || self.active_preview.is_some()
            || self.active_apply.is_some()
            || self.active_bookmark_load.is_some()
            || self.active_bookmark_save.is_some();
    }

    fn search_action_semantic_value(&self) -> String {
        serde_json::json!({
            "query": self.query,
            "kind": self.kind.source_kind(),
            "tag_filter": self.tag_filter,
            "path_filter": self.path_filter,
            "case_sensitive": self.case_sensitive,
            "whole_word": self.whole_word,
            "regex": self.is_regex,
        })
        .to_string()
    }

    fn begin_search_action(&mut self, target: String, semantic: String) {
        self.search_action_generation = self.search_action_generation.wrapping_add(1);
        self.search_action_state = crate::mcp::action::ClickCompletionState::Pending;
        self.search_action_target = Some(target);
        self.search_action_semantic = Some(semantic);
        self.search_action_error = None;
        self.search_action_detail = None;
    }

    fn complete_search_action(&mut self, detail: String) {
        if self.search_action_state == crate::mcp::action::ClickCompletionState::Pending {
            self.search_action_state = crate::mcp::action::ClickCompletionState::Applied;
            self.search_action_detail = Some(detail);
        }
    }

    /// Publish a TYPED terminal failure for the Search action.
    ///
    /// The message is sanitized and enveloped before it enters the completion token. A raw backend or
    /// `regex` crate error is frequently MULTI-LINE, and a control character makes the whole
    /// `handshake.click-completion/v1` token invalid — which silently degrades an honest typed failure
    /// into an `indeterminate` receipt (and poisons the baseline for the NEXT search too). The
    /// product-owned `"<effect> failed: "` envelope also lets an external verifier bind the exact
    /// failing effect without depending on platform-specific transport text.
    fn fail_search_action(&mut self, error: String) {
        if self.search_action_state == crate::mcp::action::ClickCompletionState::Pending {
            self.search_action_state = crate::mcp::action::ClickCompletionState::Failed;
            self.search_action_error = Some(bounded_token_field(
                &format!("{SEARCH_COMPLETION_EFFECT} failed: {error}"),
                512,
            ));
        }
    }

    fn search_target_completion_value(
        &self,
        _target: &str,
        observer: &str,
        context: &str,
    ) -> Option<String> {
        crate::mcp::action::serialize_persistent_observer_click_target(
            SEARCH_COMPLETION_EFFECT,
            context,
            self.search_action_generation,
            observer,
            &self.search_action_semantic_value(),
        )
    }

    fn search_observer_completion_value(&self, context: &str) -> Option<String> {
        match self.search_action_state {
            crate::mcp::action::ClickCompletionState::Ready => {
                crate::mcp::action::serialize_observer_click_state(
                    SEARCH_COMPLETION_EFFECT,
                    context,
                    self.search_action_generation,
                    self.search_action_state,
                    None,
                    None,
                )
            }
            crate::mcp::action::ClickCompletionState::Pending => {
                crate::mcp::action::serialize_observer_click_state(
                    SEARCH_COMPLETION_EFFECT,
                    context,
                    self.search_action_generation,
                    self.search_action_state,
                    self.search_action_target.as_deref(),
                    self.search_action_semantic.as_deref(),
                )
            }
            crate::mcp::action::ClickCompletionState::Applied => {
                crate::mcp::action::serialize_observer_click_applied(
                    SEARCH_COMPLETION_EFFECT,
                    context,
                    self.search_action_generation,
                    self.search_action_target.as_deref()?,
                    self.search_action_semantic.as_deref()?,
                    self.search_action_detail.as_deref().unwrap_or("{}"),
                )
            }
            crate::mcp::action::ClickCompletionState::Failed => {
                crate::mcp::action::serialize_observer_click_failure(
                    SEARCH_COMPLETION_EFFECT,
                    context,
                    self.search_action_generation,
                    self.search_action_target.as_deref()?,
                    self.search_action_semantic.as_deref()?,
                    self.search_action_error
                        .as_deref()
                        .unwrap_or("search failed"),
                    self.search_action_detail.as_deref(),
                )
            }
        }
    }

    // ── Same-target (synchronous) click acknowledgements ────────────────────────────────────────────

    /// The pre-dispatch same-target completion token for a synchronous control that stays mounted.
    /// `Ready@0` until the panel has consumed an activation, then `Applied@n`. The MCP boundary accepts
    /// the click only when the very next rendered token is `Applied@n+1` for the same effect/context.
    fn same_target_completion_value(
        &self,
        effect: &str,
        base_author_id: &str,
        context: &str,
    ) -> Option<String> {
        let count = self
            .same_target_activations
            .get(base_author_id)
            .copied()
            .unwrap_or(0);
        crate::mcp::action::serialize_same_target_click_completion(
            effect,
            context,
            count,
            if count == 0 {
                crate::mcp::action::ClickCompletionState::Ready
            } else {
                crate::mcp::action::ClickCompletionState::Applied
            },
        )
    }

    /// Advance one synchronous control's causal counter. Called ONLY from the deferred dispatch block,
    /// i.e. only where the panel actually consumed that control's activation this frame.
    fn record_same_target_activation(&mut self, base_author_id: &str) {
        let entry = self
            .same_target_activations
            .entry(base_author_id.to_owned())
            .or_insert(0);
        *entry = entry.wrapping_add(1);
    }

    /// Push the shell's current result-navigation observer binding before this pane renders, so a
    /// result row declares the exact current observer generation.
    pub fn set_result_open_completion_binding(&mut self, binding: FindResultOpenCompletionBinding) {
        self.result_open_completion = binding;
    }

    /// True once the DESTRUCTIVE Apply action has published a terminal success and its plans were
    /// consumed. The Apply control is withdrawn in that state (and while its own execution is in
    /// flight) and returns with the next preview, which is what makes its canonical acknowledgement a
    /// transient-target completion rather than an unprovable disabled-target one.
    fn apply_control_withdrawn(&self) -> bool {
        matches!(
            self.apply_action.state,
            crate::mcp::action::ClickCompletionState::Pending
        ) || (self.preview_plans.is_empty()
            && matches!(
                self.apply_action.state,
                crate::mcp::action::ClickCompletionState::Applied
            ))
    }

    /// Stable semantic identity for the Preview action (what is previewed, never the mutated result).
    fn preview_action_semantic_value(&self) -> String {
        serde_json::json!({
            "effect": PREVIEW_COMPLETION_EFFECT,
            "replace_key": self.current_replace_key(),
        })
        .to_string()
    }

    /// Stable semantic identity for the DESTRUCTIVE Apply action. `preview_plans` is cleared by the
    /// terminal delivery, so plan count is deliberately NOT part of the identity.
    fn apply_action_semantic_value(&self) -> String {
        serde_json::json!({
            "effect": APPLY_COMPLETION_EFFECT,
            "replace_key": self.current_replace_key(),
        })
        .to_string()
    }

    fn cancel_action_semantic_value(&self) -> String {
        serde_json::json!({
            "effect": CANCEL_COMPLETION_EFFECT,
            "replace_key": self.current_replace_key(),
        })
        .to_string()
    }

    fn bookmark_save_semantic_value(&self) -> String {
        serde_json::json!({
            "effect": BOOKMARK_COMPLETION_EFFECT,
            "op": "save",
            "search_key": self.current_search_key(),
        })
        .to_string()
    }

    fn bookmark_remove_semantic_value(bookmark_id: &str) -> String {
        serde_json::json!({
            "effect": BOOKMARK_COMPLETION_EFFECT,
            "op": "remove",
            "bookmark_id": bookmark_id,
        })
        .to_string()
    }

    fn bookmark_load_semantic_value(&self) -> String {
        serde_json::json!({
            "effect": BOOKMARK_LOAD_COMPLETION_EFFECT,
            "workspace_id": self.bound_workspace_id.clone(),
            "epoch": self.workspace_epoch,
        })
        .to_string()
    }

    /// Rebind the state machine to the active workspace. Read-only work is detached, but an old
    /// destructive Apply is cooperatively cancelled and retained until its stamped terminal delivery
    /// reports every commit/receipt.
    pub fn bind_workspace(
        &mut self,
        workspace_id: Option<&str>,
        workspace_generation: u64,
    ) -> bool {
        if self.bound_workspace_id.as_deref() == workspace_id
            && self.bound_workspace_generation == workspace_generation
        {
            return false;
        }
        if let Some(cancel) = &self.active_apply_cancel {
            cancel.store(true, Ordering::Release);
        }
        let retained_apply_workspace = self
            .active_apply
            .as_ref()
            .map(|stamp| stamp.workspace_id.clone());
        let retained_bookmark_save_workspace = self
            .active_bookmark_save
            .as_ref()
            .map(|stamp| stamp.workspace_id.clone());
        self.bound_workspace_id = workspace_id.map(str::to_owned);
        self.bound_workspace_generation = workspace_generation;
        self.workspace_epoch = self.workspace_epoch.wrapping_add(1);
        self.results.clear();
        self.preview_plans.clear();
        self.bookmarks.clear();
        self.bookmark_load_failed = false;
        self.result_set_key = None;
        self.preview_plan_key = None;
        self.error = None;
        self.replace_status = retained_apply_workspace.as_ref().map(|old_workspace| {
            format!(
                "Workspace changed; cancelling Apply for workspace {old_workspace} and waiting for its terminal receipt."
            )
        });
        self.bookmark_status = retained_bookmark_save_workspace.as_ref().map(|old_workspace| {
            format!(
                "Workspace changed; waiting for bookmark save in workspace {old_workspace} to report its terminal receipt."
            )
        });
        self.active_search = None;
        self.active_preview = None;
        // Never detach an in-flight mutation: its old stamped delivery remains authoritative even
        // after the visible workspace changes.
        self.active_bookmark_load = None;
        if let Ok(mut queue) = self.search_cell.lock() {
            queue.clear();
        }
        if let Ok(mut queue) = self.replace_cell.lock() {
            queue.retain(|delivery| delivery.stamp.operation == FindInFilesOperation::Apply);
        }
        if let Ok(mut queue) = self.bookmark_cell.lock() {
            queue.retain(|delivery| delivery.stamp.operation == FindInFilesOperation::BookmarkSave);
        }
        self.results_generation = self.results_generation.wrapping_add(1);
        *self.visible_cache.borrow_mut() = None;
        self.refresh_loading();
        true
    }

    /// The current match options as a [`MatchOptions`].
    pub fn options(&self) -> MatchOptions {
        MatchOptions {
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            is_regex: self.is_regex,
        }
    }

    /// The current search-plan key (for the stale-result guard).
    pub fn current_search_key(&self) -> String {
        search_plan_key(
            &self.query,
            self.kind,
            &self.tag_filter,
            &self.path_filter,
            self.options(),
        )
    }

    /// The current replace-plan key (for the stale-preview guard).
    pub fn current_replace_key(&self) -> String {
        replace_plan_key(&self.current_search_key(), &self.replacement)
    }

    /// The current visible-cache key: live query + options + results generation. A change to ANY of these
    /// (re-search, query edit, toggle flip) invalidates the memoized visible-hit filter.
    fn visible_cache_key(&self) -> String {
        let opts = self.options();
        format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.results_generation,
            self.query,
            opts.case_sensitive as u8,
            opts.whole_word as u8,
            opts.is_regex as u8,
        )
    }

    /// Recompute (and cache) the indices of `results` that pass the client-side option filter, but ONLY
    /// when the visible-cache key changed. The compiled [`Regex`] is built ONCE here per (query, options)
    /// change and reused across all hits — never per hit, never per frame (perf hygiene). Runs the filtered
    /// closure with the cached index slice borrowed from the [`RefCell`].
    fn with_visible_indices<R>(&self, f: impl FnOnce(&[usize]) -> R) -> R {
        let key = self.visible_cache_key();
        {
            let cache = self.visible_cache.borrow();
            if cache.as_ref().is_some_and(|c| c.key == key) {
                return f(&cache.as_ref().expect("checked is_some_and above").indices);
            }
        }
        // Cache miss: rebuild the index list. Compile the option regex ONCE (or None when the query is
        // empty / no match-option is active / it fails to compile — in those cases every hit passes,
        // mirroring `hit_matches_client_options`).
        let opts = self.options();
        let regex = if self.query.trim().is_empty()
            || (!opts.case_sensitive && !opts.whole_word && !opts.is_regex)
        {
            None
        } else {
            compile_search_regex(&self.query, opts).ok()
        };
        let indices: Vec<usize> = match &regex {
            None => (0..self.results.len()).collect(),
            Some(re) => self
                .results
                .iter()
                .enumerate()
                .filter(|(_, h)| hit_matches_regex(h, re, opts))
                .map(|(i, _)| i)
                .collect(),
        };
        *self.visible_cache.borrow_mut() = Some(VisibleCache { key, indices });
        let cache = self.visible_cache.borrow();
        f(&cache.as_ref().expect("just stored above").indices)
    }

    /// The hits passing the client-side option filter (the React `visibleResults`), via the memoized
    /// index cache (no per-frame regex recompile or full-set clone — perf hygiene).
    pub fn visible_results(&self) -> Vec<&LoomGraphSearchHit> {
        self.with_visible_indices(|idx| idx.iter().map(|&i| &self.results[i]).collect())
    }

    /// The number of hits passing the client-side option filter, via the memoized cache (cheap — no clone).
    pub fn visible_result_count(&self) -> usize {
        self.with_visible_indices(<[usize]>::len)
    }

    /// Monotonic read-only result revision for host bridges. It advances on query/filter invalidation,
    /// workspace rebind, and accepted backend delivery, allowing the shell to avoid cloning a large
    /// paginated result set every frame.
    pub fn results_generation(&self) -> u64 {
        self.results_generation
    }

    /// `true` when a non-stale preview with plans exists (gates the Apply button — AC-8).
    pub fn can_apply(&self) -> bool {
        !self.preview_plans.is_empty()
            && self.preview_plan_key.as_deref() == Some(&self.current_replace_key())
    }

    pub fn search_in_flight(&self) -> bool {
        self.active_search.is_some()
    }
    pub fn preview_in_flight(&self) -> bool {
        self.active_preview.is_some()
    }
    pub fn apply_in_flight(&self) -> bool {
        self.active_apply.is_some()
    }
    pub fn bookmark_in_flight(&self) -> bool {
        self.active_bookmark_load.is_some() || self.active_bookmark_save.is_some()
    }

    fn invalidate_search_inputs(&mut self) {
        self.active_search = None;
        self.active_preview = None;
        // Keep the last completed result set and its producer key long enough for the mounted Preview
        // action to report the contract's explicit stale-result warning. Clearing either here made the
        // button disabled, so the UI could never exercise the stale guard that `run_preview_replace`
        // correctly enforces. A real Search clears `result_set_key` before dispatch and replaces the rows
        // only on an accepted current-generation delivery.
        self.preview_plans.clear();
        self.expanded_preview_document_ids.clear();
        self.preview_plan_key = None;
        self.replace_status = None;
        self.error = None;
        self.results_generation = self.results_generation.wrapping_add(1);
        *self.visible_cache.borrow_mut() = None;
        if let Ok(mut queue) = self.search_cell.lock() {
            queue.clear();
        }
        if let Ok(mut queue) = self.replace_cell.lock() {
            queue.retain(|delivery| delivery.stamp.operation == FindInFilesOperation::Apply);
        }
        self.refresh_loading();
    }

    fn invalidate_replacement_input(&mut self) {
        let invalidated_preview = self.preview_plan_key.is_some() || !self.preview_plans.is_empty();
        self.active_preview = None;
        self.preview_plans.clear();
        self.expanded_preview_document_ids.clear();
        self.preview_plan_key = None;
        if invalidated_preview {
            self.replace_status =
                Some("Preview is stale; run Preview Replace again before applying.".to_owned());
        }
        self.refresh_loading();
    }

    /// Drain the off-thread delivery cells, folding any arrived result into state. Returns `true` if
    /// anything was delivered (so the caller can request a repaint).
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        let search_deliveries = self
            .search_cell
            .lock()
            .ok()
            .map(|mut queue| queue.drain(..).collect::<Vec<_>>())
            .unwrap_or_default();
        for delivery in search_deliveries {
            if self.active_search.as_ref() == Some(&delivery.stamp) {
                self.active_search = None;
                let search_completion = match &delivery.outcome {
                    Ok((hits, key)) => Ok(serde_json::json!({
                        "result_count": hits.len(),
                        "result_set_key": key,
                    })
                    .to_string()),
                    Err(message) => Err(message.clone()),
                };
                match delivery.outcome {
                    Ok((hits, key)) => {
                        self.results = hits;
                        self.result_set_key = Some(key);
                        self.error = None;
                    }
                    Err(msg) => {
                        self.results = Vec::new();
                        self.result_set_key = None;
                        self.error = Some(msg);
                    }
                }
                // A new (or cleared) result set invalidates the memoized visible-hit filter.
                self.results_generation = self.results_generation.wrapping_add(1);
                match search_completion {
                    Ok(detail) => self.complete_search_action(detail),
                    Err(error) => self.fail_search_action(error),
                }
                changed = true;
            }
        }
        let replace_deliveries = self
            .replace_cell
            .lock()
            .ok()
            .map(|mut queue| queue.drain(..).collect::<Vec<_>>())
            .unwrap_or_default();
        for delivery in replace_deliveries {
            let active = match delivery.stamp.operation {
                FindInFilesOperation::Preview => &mut self.active_preview,
                FindInFilesOperation::Apply => &mut self.active_apply,
                _ => continue,
            };
            if active.as_ref() == Some(&delivery.stamp) {
                *active = None;
                // Terminal ACTION causality is published BEFORE the delivery is folded into visible
                // state, from the authoritative delivery itself (exact per-document before/after
                // hashes + EventLedger save receipt ids for the destructive Apply).
                self.publish_replace_action_completion(
                    delivery.stamp.operation,
                    &delivery.outcome,
                );
                if delivery.stamp.operation == FindInFilesOperation::Apply {
                    self.active_apply_cancel = None;
                    let terminal_workspace_id = delivery.stamp.workspace_id.clone();
                    self.apply_replace_delivery(delivery.outcome);
                    self.last_apply_terminal_workspace_id = Some(terminal_workspace_id.clone());
                    if let Some(status) = self.replace_status.take() {
                        self.replace_status =
                            Some(format!("Workspace {terminal_workspace_id}: {status}"));
                    }
                    if self.refresh_search_after_apply {
                        self.refresh_search_workspace_id = Some(terminal_workspace_id);
                    } else {
                        self.refresh_search_workspace_id = None;
                    }
                    changed = true;
                    continue;
                }
                self.apply_replace_delivery(delivery.outcome);
                changed = true;
            }
        }
        let bookmark_deliveries = self
            .bookmark_cell
            .lock()
            .ok()
            .map(|mut queue| queue.drain(..).collect::<Vec<_>>())
            .unwrap_or_default();
        for delivery in bookmark_deliveries {
            match delivery.stamp.operation {
                FindInFilesOperation::BookmarkLoad => {
                    if self.active_bookmark_load.as_ref() != Some(&delivery.stamp) {
                        continue;
                    }
                    self.active_bookmark_load = None;
                    match delivery.outcome {
                        Ok((blob, status, _receipt)) => match parse_bookmark_state(&blob) {
                            Ok(bookmarks) => {
                                let restored = bookmarks.len();
                                self.bookmarks = bookmarks;
                                self.bookmark_load_failed = false;
                                if status.is_some() {
                                    self.bookmark_status = status;
                                }
                                self.bookmark_load_action.complete(
                                    serde_json::json!({
                                        "kind": "bookmark_load",
                                        "bookmark_count": restored,
                                        "attempt_count": self.bookmark_load_attempt_count,
                                    })
                                    .to_string(),
                                );
                            }
                            Err(error) => {
                                self.bookmark_load_failed = true;
                                let error = format!("Persisted bookmark state rejected: {error}");
                                self.bookmark_load_action
                                    .fail(BOOKMARK_LOAD_COMPLETION_EFFECT, &error);
                                self.bookmark_status = Some(match self.bookmark_status.take() {
                                    Some(terminal) => format!("{error}; {terminal}"),
                                    None => error,
                                });
                            }
                        },
                        Err(msg) => {
                            self.bookmark_load_failed = true;
                            self.bookmark_load_action
                                .fail(BOOKMARK_LOAD_COMPLETION_EFFECT, &msg);
                            self.bookmark_status = Some(match self.bookmark_status.take() {
                                Some(terminal) => format!("{msg}; {terminal}"),
                                None => msg,
                            });
                        }
                    }
                    changed = true;
                }
                FindInFilesOperation::BookmarkSave => {
                    if self.active_bookmark_save.as_ref() != Some(&delivery.stamp) {
                        continue;
                    }
                    self.active_bookmark_save = None;
                    let terminal_workspace_id = delivery.stamp.workspace_id.clone();
                    let terminal_belongs_to_visible_binding = self.bound_workspace_id.as_deref()
                        == Some(terminal_workspace_id.as_str())
                        && self.workspace_epoch == delivery.stamp.epoch;
                    let (terminal_status, receipt) = match delivery.outcome {
                        Ok((blob, status, receipt)) => match parse_bookmark_state(&blob) {
                            Ok(bookmarks) => {
                                let persisted_count = bookmarks.len();
                                let persisted_ids: Vec<String> =
                                    bookmarks.iter().map(|entry| entry.id.clone()).collect();
                                if terminal_belongs_to_visible_binding {
                                    self.bookmarks = bookmarks;
                                    self.bookmark_load_failed = false;
                                }
                                self.bookmark_action.complete(bounded_token_field(
                                    &serde_json::json!({
                                        "kind": "bookmark_persist",
                                        "workspace_id": &terminal_workspace_id,
                                        "bookmark_count": persisted_count,
                                        "bookmark_ids": &persisted_ids,
                                        "save_receipt_event_id": &receipt,
                                    })
                                    .to_string(),
                                    2000,
                                ));
                                (
                                    status.unwrap_or_else(|| "Bookmark save completed".to_owned()),
                                    receipt,
                                )
                            }
                            Err(error) => {
                                let error = format!("Persisted bookmark state rejected: {error}");
                                self.bookmark_action.fail(BOOKMARK_COMPLETION_EFFECT, &error);
                                (error, receipt)
                            }
                        },
                        Err(msg) => {
                            self.bookmark_action.fail(BOOKMARK_COMPLETION_EFFECT, &msg);
                            (msg, None)
                        }
                    };
                    self.last_bookmark_save_terminal_workspace_id =
                        Some(terminal_workspace_id.clone());
                    self.last_bookmark_save_receipt_id = receipt.clone();
                    self.bookmark_status = Some(match receipt {
                        Some(receipt) => format!(
                            "Workspace {terminal_workspace_id}: {terminal_status}; receipt: {receipt}"
                        ),
                        None => format!("Workspace {terminal_workspace_id}: {terminal_status}"),
                    });
                    changed = true;
                }
                _ => continue,
            }
        }
        self.refresh_loading();
        changed
    }

    /// Drain completions and immediately refresh the current search after any committed Apply outcome.
    /// Kept as one production state-machine step so mounted UI and managed runtime proofs exercise the
    /// same refresh behavior.
    pub fn poll_with_search_refresh(
        &mut self,
        search_client: &WorkspaceSearchClient,
        workspace_id: Option<&str>,
    ) -> bool {
        let changed = self.poll();
        if self.refresh_search_after_apply
            && self.refresh_search_workspace_id.as_deref() == workspace_id
            && self.run_search(search_client, workspace_id)
        {
            self.refresh_search_after_apply = false;
            self.refresh_search_workspace_id = None;
            return true;
        }
        changed
    }

    /// Publish the TERMINAL action-completion for a replace-pipeline delivery from the authoritative
    /// delivery itself, before any visible state is folded.
    ///
    /// For the destructive Apply this binds terminal success to the exact per-document before/after
    /// content hashes and EventLedger save receipt ids, and binds a conflict/failure to the preserved
    /// before/after hashes plus the typed outcome with NO invented receipt. A partial Apply is a real
    /// terminal outcome of the action (some documents committed), so it terminalises as `Applied`
    /// carrying the typed conflict rows — never as a silent success and never as an echo.
    fn publish_replace_action_completion(
        &mut self,
        operation: FindInFilesOperation,
        outcome: &ReplaceDelivery,
    ) {
        match (operation, outcome) {
            (FindInFilesOperation::Preview, ReplaceDelivery::Preview { plans, key }) => {
                let documents: Vec<serde_json::Value> = plans
                    .iter()
                    .take(MAX_TERMINAL_DETAIL_AUDIT_ROWS)
                    .map(|plan| {
                        serde_json::json!({
                            "document_id": plan.document_id,
                            "match_count": plan.match_count,
                            "before_sha256": plan.before_sha256,
                            "after_sha256": plan.after_sha256,
                            "expected_version": plan.expected_version,
                        })
                    })
                    .collect();
                // A fresh preview restores the withdrawn Apply control (its previous terminal result no
                // longer stands). The generation is deliberately NOT rewound: the next Apply
                // declaration must still advance monotonically from the observer's current value.
                if !self.apply_action.is_pending() {
                    self.apply_action.reset_ready();
                }
                self.preview_action.complete(bounded_token_field(
                    &serde_json::json!({
                        "kind": "preview",
                        "plan_count": plans.len(),
                        "preview_plan_key": key,
                        "documents": documents,
                        "documents_truncated": plans.len() > MAX_TERMINAL_DETAIL_AUDIT_ROWS,
                    })
                    .to_string(),
                    2000,
                ));
            }
            (FindInFilesOperation::Preview, ReplaceDelivery::PreviewError(message)) => {
                self.preview_action
                    .fail(PREVIEW_COMPLETION_EFFECT, message);
            }
            (
                FindInFilesOperation::Apply,
                ReplaceDelivery::Applied { audit_receipts, .. },
            ) => {
                self.apply_action
                    .complete(apply_terminal_detail("applied", audit_receipts));
            }
            (
                FindInFilesOperation::Apply,
                ReplaceDelivery::AppliedPartial { audit_receipts, .. },
            ) => {
                self.apply_action
                    .complete(apply_terminal_detail("applied_partial", audit_receipts));
            }
            (
                FindInFilesOperation::Apply,
                ReplaceDelivery::Cancelled {
                    audit_receipts,
                    skipped_plan_count,
                    ..
                },
            ) => {
                let detail = apply_terminal_detail("cancelled", audit_receipts);
                self.apply_action.complete(detail.clone());
                // The Cancel control owns the SAME terminal delivery: cancellation is proven by the
                // authoritative committed/skipped split, never by the click being consumed.
                self.cancel_action.complete(bounded_token_field(
                    &serde_json::json!({
                        "kind": "cancel_honoured",
                        "skipped_plan_count": skipped_plan_count,
                        "apply_terminal": detail,
                    })
                    .to_string(),
                    2000,
                ));
            }
            (FindInFilesOperation::Apply, _) => {}
            _ => {}
        }
        if operation == FindInFilesOperation::Apply && self.cancel_action.is_pending() {
            // Apply terminalised without honouring cancellation (it had already finished). Report that
            // exact typed outcome instead of leaving the observer pending forever.
            self.cancel_action.complete(
                serde_json::json!({
                    "kind": "cancel_after_terminal_apply",
                    "note": "Apply reached its terminal delivery before cancellation could skip a plan",
                })
                .to_string(),
            );
        }
    }

    /// Fold a delivered replace-pipeline result into state.
    fn apply_replace_delivery(&mut self, delivery: ReplaceDelivery) {
        match delivery {
            ReplaceDelivery::Preview { plans, key } => {
                let plan_count = plans.len();
                self.preview_plans = plans;
                self.preview_plan_key = Some(key);
                self.replace_status = Some(if plan_count == 0 {
                    "No replacements matched in editable rich documents.".to_owned()
                } else {
                    format!("Previewed {plan_count} document replacement plan(s).")
                });
                self.error = None;
            }
            ReplaceDelivery::PreviewError(msg) => {
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                self.error = Some(msg);
            }
            ReplaceDelivery::Applied {
                receipts,
                plan_count,
                audit_receipts,
            } => {
                self.replace_status =
                    Some(format!(
                    "Applied {plan_count} document replacement plan(s); receipts: {}; mutation audit: {}",
                    receipts.join(", "),
                    audit_receipts.iter().map(format_replace_audit_receipt).collect::<Vec<_>>().join(", ")
                ));
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                self.error = None;
                self.refresh_search_after_apply = true;
            }
            ReplaceDelivery::AppliedPartial {
                receipts,
                audit_receipts,
                error,
            } => {
                // RISK-1 / MC-1: a partial failure NEVER loses the receipts already collected.
                let committed_count = audit_receipts
                    .iter()
                    .filter(|receipt| {
                        matches!(
                            receipt.outcome,
                            ReplaceAuditOutcome::Saved
                                | ReplaceAuditOutcome::CommittedWithoutReceipt
                        )
                    })
                    .count();
                self.replace_status = Some(format!(
                    "Applied {} document replacement plan(s) before failure; receipts: {}; mutation audit: {}",
                    committed_count,
                    receipts.join(", "),
                    audit_receipts.iter().map(format_replace_audit_receipt).collect::<Vec<_>>().join(", ")
                ));
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                self.error = Some(error);
                self.refresh_search_after_apply = committed_count > 0;
            }
            ReplaceDelivery::Cancelled {
                receipts,
                audit_receipts,
                skipped_plan_count,
            } => {
                let committed_count = audit_receipts
                    .iter()
                    .filter(|receipt| {
                        matches!(
                            receipt.outcome,
                            ReplaceAuditOutcome::Saved
                                | ReplaceAuditOutcome::CommittedWithoutReceipt
                        )
                    })
                    .count();
                self.replace_status = Some(format!(
                    "Cancellation honored after {committed_count} committed replacement plan(s); skipped {skipped_plan_count}; receipts: {}; mutation audit: {}",
                    receipts.join(", "),
                    audit_receipts
                        .iter()
                        .map(format_replace_audit_receipt)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                self.preview_plans.clear();
                self.preview_plan_key = None;
                self.error = None;
                self.refresh_search_after_apply = committed_count > 0;
            }
        }
    }

    /// Managed-proof seam: project an exact typed worker delivery through the same terminal reducer
    /// used by production polling before mounting that state in the production pane factory.
    #[doc(hidden)]
    pub fn accept_replace_delivery_for_test(&mut self, delivery: ReplaceDelivery) {
        self.apply_replace_delivery(delivery);
    }

    /// Managed-proof seam: place the replacement input behind the same in-flight Apply gate used by
    /// production so disabled AccessKit mutation behavior can be regression-tested without a backend.
    #[doc(hidden)]
    pub fn set_apply_in_flight_for_test(&mut self, in_flight: bool) {
        if in_flight {
            let workspace_id = self
                .bound_workspace_id
                .clone()
                .unwrap_or_else(|| "test-workspace".to_owned());
            self.active_apply = Some(self.next_stamp(&workspace_id, FindInFilesOperation::Apply));
        } else {
            self.active_apply = None;
        }
        self.refresh_loading();
    }

    /// Request cooperative cancellation. Apply keeps its active stamp until the worker reports exactly
    /// which saves committed; preview can be detached immediately because it is read-only.
    pub fn request_cancel(&mut self) {
        if let Some(cancel) = &self.active_apply_cancel {
            cancel.store(true, Ordering::Release);
            self.replace_status = Some(
                "Cancellation requested; waiting for the in-flight save receipt before reporting committed mutations."
                    .to_owned(),
            );
            return;
        }
        if self.active_preview.take().is_some() {
            self.preview_plans.clear();
            self.preview_plan_key = None;
            self.replace_status = Some("Replacement preview cancelled.".to_owned());
            self.error = None;
            self.refresh_loading();
            return;
        }
        self.preview_plans.clear();
        self.preview_plan_key = None;
        self.replace_status = Some("Replacement preview cleared.".to_owned());
        self.error = None;
    }

    /// Fire a workspace-wide search against `workspace_id` with the current query + filters + options.
    /// Guards no-workspace (MC-7, NO HTTP), empty-query, and regex-mode compile errors (PT-4) — each
    /// shows an error and fires no request. On a real fire, sets `loading`, clears the prior error, and
    /// resets the preview (a fresh search invalidates any stale plan).
    pub fn run_search(
        &mut self,
        client: &WorkspaceSearchClient,
        workspace_id: Option<&str>,
    ) -> bool {
        let Some(ws) = workspace_id else {
            self.error = Some("No workspace selected".to_owned());
            return false;
        };
        if self.active_apply.is_some() {
            self.error = Some(
                "Search is blocked until the in-flight Apply reports its terminal receipts."
                    .to_owned(),
            );
            return false;
        }
        if self.active_preview.take().is_some() {
            self.invalidate_search_inputs();
            self.error = Some(
                "Search was blocked while Preview was in flight; the preview was detached. Run Search again."
                    .to_owned(),
            );
            return false;
        }
        let trimmed = self.query.trim().to_owned();
        if trimmed.is_empty() {
            self.error = Some("Search query is required".to_owned());
            return false;
        }
        // Regex-mode pre-validation so a bad pattern shows the error WITHOUT a backend round-trip (PT-4).
        if self.is_regex {
            if let Err(e) = compile_search_regex(&trimmed, self.options()) {
                self.error = Some(e);
                return false;
            }
        }
        if self.active_search.is_some() {
            self.error = Some("Search is already in flight".to_owned());
            return false;
        }
        let key = self.current_search_key();
        let stamp = self.next_stamp(ws, FindInFilesOperation::Search);
        self.active_search = Some(stamp.clone());
        self.refresh_loading();
        self.error = None;
        if !self.refresh_search_after_apply {
            self.replace_status = None;
        }
        self.preview_plans = Vec::new();
        self.preview_plan_key = None;
        self.result_set_key = None;
        client.search_paginated(
            ws,
            &trimmed,
            self.kind.source_kind(),
            &self.tag_filter,
            &self.path_filter,
            self.options().to_search(),
            key,
            stamp,
            Arc::clone(&self.search_cell),
        );
        true
    }

    /// Begin the Preview Replace pipeline: stale-result guard (RISK-2/MC-2 — a since-changed query
    /// shows the stale warning and computes NOTHING), regex-compile guard, then load each
    /// `KRD-`-prefixed hit document off-thread, walk its content_json, and accumulate the plans into the
    /// replace cell. No-workspace/no-document cases set a status and fire nothing.
    pub fn run_preview_replace(&mut self, client: &RichDocClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.replace_status = Some("No workspace selected".to_owned());
            return;
        };
        let opts = self.options();
        let regex = match compile_search_regex(self.query.trim(), opts) {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(e);
                self.preview_plans = Vec::new();
                self.preview_plan_key = None;
                return;
            }
        };
        // STALE-RESULT guard: the results must have been fetched under the CURRENT search params.
        if self.result_set_key.as_deref() != Some(&self.current_search_key()) {
            self.replace_status = Some(
                "Search results are stale; run Search again before previewing replacements."
                    .to_owned(),
            );
            self.preview_plans = Vec::new();
            self.preview_plan_key = None;
            return;
        }
        // Unique KRD- document ids from the live result set (RISK-5).
        let mut seen = std::collections::HashSet::new();
        let document_ids: Vec<String> = self
            .results
            .iter()
            .filter_map(document_id_from_hit)
            .filter(|id| seen.insert(id.clone()))
            .collect();
        if document_ids.is_empty() {
            self.replace_status =
                Some("No editable rich documents in the backend result set.".to_owned());
            self.preview_plans = Vec::new();
            self.preview_plan_key = None;
            return;
        }
        if self.active_preview.is_some() || self.active_apply.is_some() {
            self.replace_status = Some("A replacement operation is already in flight".to_owned());
            return;
        }
        let key = self.current_replace_key();
        let stamp = self.next_stamp(ws, FindInFilesOperation::Preview);
        self.active_preview = Some(stamp.clone());
        self.refresh_loading();
        self.error = None;
        self.replace_status = None;
        client.preview_replace(
            ws,
            document_ids,
            regex,
            self.replacement.clone(),
            opts,
            key,
            stamp,
            Arc::clone(&self.replace_cell),
        );
    }

    /// Apply the current preview plans: stale-plan guard (RISK-2/MC-2 — a since-changed search or
    /// replacement shows the stale warning and applies NOTHING), then save each plan off-thread with its
    /// captured `expected_version` (optimistic concurrency; a 409 stops with partial receipts preserved).
    pub fn run_apply(&mut self, client: &RichDocClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.replace_status = Some("No workspace selected".to_owned());
            return;
        };
        if self.preview_plans.is_empty() {
            return;
        }
        // STALE-PLAN guard: the preview must match the current search+replacement.
        if self.preview_plan_key.as_deref() != Some(&self.current_replace_key()) {
            self.replace_status =
                Some("Preview is stale; run Preview Replace again before applying.".to_owned());
            return;
        }
        if self.active_preview.is_some() || self.active_apply.is_some() {
            self.replace_status = Some("A replacement operation is already in flight".to_owned());
            return;
        }
        let stamp = self.next_stamp(ws, FindInFilesOperation::Apply);
        let cancel = Arc::new(AtomicBool::new(false));
        self.active_apply = Some(stamp.clone());
        self.active_apply_cancel = Some(Arc::clone(&cancel));
        self.refresh_loading();
        self.error = None;
        client.apply_plans(
            ws,
            self.preview_plans.clone(),
            stamp,
            Arc::clone(&self.replace_cell),
            cancel,
        );
    }

    /// Load the saved bookmarks for `workspace_id` (called when the panel mounts). No-op when no
    /// workspace; clears any stale list.
    pub fn load_bookmarks(&mut self, client: &WorkspaceSearchClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.bookmarks = Vec::new();
            self.bookmark_load_failed = false;
            return;
        };
        if self.active_bookmark_load.is_some() {
            return;
        }
        let stamp = self.next_stamp(ws, FindInFilesOperation::BookmarkLoad);
        self.active_bookmark_load = Some(stamp.clone());
        self.bookmark_load_failed = false;
        self.bookmark_load_attempt_count = self.bookmark_load_attempt_count.wrapping_add(1);
        self.refresh_loading();
        self.bookmark_status = None;
        client.load_bookmarks(ws, stamp, Arc::clone(&self.bookmark_cell));
    }

    /// Save the current search as a bookmark (dedup by stable id, cap at 20), persisting the whole list.
    /// Refuses an empty search (no query + All kind + no filters).
    pub fn save_bookmark(&mut self, client: &WorkspaceSearchClient, workspace_id: Option<&str>) {
        let Some(ws) = workspace_id else {
            self.bookmark_status = Some("No workspace selected".to_owned());
            return;
        };
        if self.query.trim().is_empty()
            && self.kind == KindFilter::All
            && self.tag_filter.trim().is_empty()
            && self.path_filter.trim().is_empty()
        {
            self.bookmark_status = Some("Add a query or filter before bookmarking.".to_owned());
            return;
        }
        let mut bookmark = SearchBookmark {
            id: String::new(),
            label: String::new(),
            query: self.query.clone(),
            kind: self.kind,
            tag_filter: self.tag_filter.clone(),
            path_filter: self.path_filter.clone(),
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            is_regex: self.is_regex,
            saved_at: now_iso8601(),
        };
        bookmark.id = bookmark.stable_id();
        bookmark.label = bookmark.display_label();
        // Dedup by id, newest first, cap at 20.
        let mut next: Vec<SearchBookmark> = vec![bookmark.clone()];
        next.extend(
            self.bookmarks
                .iter()
                .filter(|b| b.id != bookmark.id)
                .cloned(),
        );
        next.truncate(MAX_WORKSPACE_SEARCH_BOOKMARKS);
        self.persist_bookmarks(
            client,
            ws,
            next,
            format!("Saved search bookmark {}", bookmark.label),
        );
    }

    /// Restore a bookmark into the live query/filter/option fields (purely local — no HTTP). Clears the
    /// stale result/preview state.
    pub fn restore_bookmark(&mut self, bookmark: &SearchBookmark) {
        self.query = bookmark.query.clone();
        self.kind = bookmark.kind;
        self.tag_filter = bookmark.tag_filter.clone();
        self.path_filter = bookmark.path_filter.clone();
        self.case_sensitive = bookmark.case_sensitive;
        self.whole_word = bookmark.whole_word;
        self.is_regex = bookmark.is_regex;
        self.invalidate_search_inputs();
        self.replace_status = None;
        self.error = None;
        self.bookmark_status = Some(format!("Restored search bookmark {}", bookmark.label));
    }

    /// Remove a bookmark (persisting the shortened list).
    pub fn remove_bookmark(
        &mut self,
        client: &WorkspaceSearchClient,
        workspace_id: Option<&str>,
        bookmark_id: &str,
    ) {
        let Some(ws) = workspace_id else {
            return;
        };
        let next: Vec<SearchBookmark> = self
            .bookmarks
            .iter()
            .filter(|b| b.id != bookmark_id)
            .cloned()
            .collect();
        self.persist_bookmarks(client, ws, next, "Removed search bookmark".to_owned());
    }

    /// Persist a bookmark list to the backend off-thread, delivering the saved (re-parsed) list +
    /// `status` into the bookmark cell.
    fn persist_bookmarks(
        &mut self,
        client: &WorkspaceSearchClient,
        ws: &str,
        bookmarks: Vec<SearchBookmark>,
        status: String,
    ) {
        if self.active_bookmark_save.is_some() {
            self.bookmark_status = Some("A bookmark save is already in flight".to_owned());
            return;
        }
        let stamp = self.next_stamp(ws, FindInFilesOperation::BookmarkSave);
        self.active_bookmark_save = Some(stamp.clone());
        self.refresh_loading();
        self.bookmark_status = None;
        let blob = bookmark_state_blob(&bookmarks);
        client.save_bookmarks(ws, blob, status, stamp, Arc::clone(&self.bookmark_cell));
    }

    /// The honest loading/status text for the header line.
    pub fn header_status(&self) -> String {
        if self.loading {
            return "Working…".to_owned();
        }
        match (&self.replace_status, &self.error) {
            // A partial Apply deliberately retains both the committed-document receipts/audit and the
            // terminal failure. Rendering only `error` hid the already-persisted mutations from the
            // operator, which is unsafe recovery guidance.
            (Some(status), Some(error)) => return format!("{status}; failure: {error}"),
            (Some(status), None) => return status.clone(),
            (None, Some(error)) => return error.clone(),
            (None, None) => {}
        }
        let n = self.visible_result_count();
        if self.result_set_key.is_some() {
            let plural = if n == 1 { "" } else { "s" };
            return format!("{n} result{plural}");
        }
        "Enter a query".to_owned()
    }
}

/// A typed result delivered by the off-thread replace pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceDelivery {
    /// Preview computed `plans` under `key`.
    Preview {
        plans: Vec<ReplacementPlan>,
        key: String,
    },
    /// Preview failed (a document load failed).
    PreviewError(String),
    /// All `plan_count` plans applied; `receipts` are the per-document save receipt ids.
    Applied {
        receipts: Vec<String>,
        audit_receipts: Vec<ReplaceAuditReceipt>,
        plan_count: usize,
    },
    /// Apply failed partway: `receipts` of the docs already saved are preserved; `error` is the failure.
    AppliedPartial {
        receipts: Vec<String>,
        audit_receipts: Vec<ReplaceAuditReceipt>,
        error: String,
    },
    /// Apply cancellation was honored between document saves. Already committed saves and their
    /// receipts/audit rows are preserved; `skipped_plan_count` was never submitted.
    Cancelled {
        receipts: Vec<String>,
        audit_receipts: Vec<ReplaceAuditReceipt>,
        skipped_plan_count: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplaceAuditOutcome {
    Saved,
    CommittedWithoutReceipt,
    Conflict,
    Failed,
}

/// Per-document mutation receipt. Hashes name the exact JSON loaded during preview and submitted by
/// Apply; conflict/failure rows are retained alongside any prior successful save receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceAuditReceipt {
    pub document_id: String,
    pub before_sha256: String,
    pub after_sha256: String,
    pub outcome: ReplaceAuditOutcome,
    pub save_receipt_event_id: Option<String>,
    pub error: Option<String>,
}

fn format_replace_audit_receipt(receipt: &ReplaceAuditReceipt) -> String {
    let error = receipt
        .error
        .as_deref()
        .map(|error| format!(" ({error})"))
        .unwrap_or_default();
    format!(
        "{}:{:?}:{}→{}{}",
        receipt.document_id, receipt.outcome, receipt.before_sha256, receipt.after_sha256, error
    )
}

/// A monotonic ISO-8601-ish timestamp for the bookmark `savedAt` field. Uses `chrono` (already a
/// transitive dep) so the value is a real UTC instant.
fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ── Callbacks ─────────────────────────────────────────────────────────────────────────────────────────

/// Callbacks the host wires into the panel.
pub struct FindInFilesCallbacks<'a> {
    /// Open a hit's target (document/loom-block/etc.) in place — routed by the shell. The
    /// `(source_kind, ref_id, document_id?)` tuple lets the shell pick the open path.
    pub on_open_hit: &'a mut dyn FnMut(&LoomGraphSearchHit),
}

// ── Render ────────────────────────────────────────────────────────────────────────────────────────────

/// Render the panel: query/replace bars, match toggles, kind/tag/path filters, action buttons, the
/// results list, and the preview list. Drains the async cells first; dispatches actions through the two
/// clients + the callbacks. `workspace_id` is the active workspace (the no-workspace guards show an error
/// rather than 404ing).
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut FindInFilesPanelState,
    palette: &HsPalette,
    search_client: &WorkspaceSearchClient,
    doc_client: &RichDocClient,
    workspace_id: Option<&str>,
    callbacks: &mut FindInFilesCallbacks<'_>,
) {
    show_with_author_scope(
        ui,
        state,
        palette,
        search_client,
        doc_client,
        workspace_id,
        callbacks,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn show_with_author_scope(
    ui: &mut egui::Ui,
    state: &mut FindInFilesPanelState,
    palette: &HsPalette,
    search_client: &WorkspaceSearchClient,
    doc_client: &RichDocClient,
    workspace_id: Option<&str>,
    callbacks: &mut FindInFilesCallbacks<'_>,
    secondary_pane_id: Option<&str>,
) {
    state.poll_with_search_refresh(search_client, workspace_id);
    if state.loading {
        ui.ctx().request_repaint();
    }

    ui.heading("Find in Files");
    ui.label(egui::RichText::new("Workspace-wide search + replace").weak());
    ui.add_space(4.0);

    // Deferred action flags (dispatched after the immutable borrows end).
    let mut fire_search = false;
    let mut fire_preview = false;
    let mut fire_apply = false;
    let mut fire_cancel = false;
    let mut fire_save_bookmark = false;
    let mut fire_retry_bookmarks = false;
    let scoped = |author_id: &str| pane_scoped_author_id(author_id, secondary_pane_id);
    let query_author_id = scoped(QUERY_AUTHOR_ID);
    let replace_author_id = scoped(REPLACE_AUTHOR_ID);
    let toggle_case_author_id = scoped(TOGGLE_CASE_AUTHOR_ID);
    let toggle_word_author_id = scoped(TOGGLE_WORD_AUTHOR_ID);
    let toggle_regex_author_id = scoped(TOGGLE_REGEX_AUTHOR_ID);
    let toggle_case_state_author_id = scoped(TOGGLE_CASE_STATE_AUTHOR_ID);
    let toggle_word_state_author_id = scoped(TOGGLE_WORD_STATE_AUTHOR_ID);
    let toggle_regex_state_author_id = scoped(TOGGLE_REGEX_STATE_AUTHOR_ID);
    let kind_filter_author_id = scoped(KIND_FILTER_AUTHOR_ID);
    let tag_filter_author_id = scoped(TAG_FILTER_AUTHOR_ID);
    let path_filter_author_id = scoped(PATH_FILTER_AUTHOR_ID);
    let search_author_id = scoped(SEARCH_AUTHOR_ID);
    let search_completion_author_id = scoped(SEARCH_COMPLETION_AUTHOR_ID);
    let search_completion_context = format!("find-in-files.search:{search_author_id}");
    let preview_replace_author_id = scoped(PREVIEW_REPLACE_AUTHOR_ID);
    let preview_completion_author_id = scoped(PREVIEW_COMPLETION_AUTHOR_ID);
    let preview_completion_context =
        format!("{PREVIEW_COMPLETION_EFFECT}:{preview_completion_author_id}");
    let apply_author_id = scoped(APPLY_AUTHOR_ID);
    let apply_completion_author_id = scoped(APPLY_COMPLETION_AUTHOR_ID);
    let apply_completion_context = format!("{APPLY_COMPLETION_EFFECT}:{apply_completion_author_id}");
    let cancel_author_id = scoped(CANCEL_AUTHOR_ID);
    let cancel_completion_author_id = scoped(CANCEL_COMPLETION_AUTHOR_ID);
    let cancel_completion_context =
        format!("{CANCEL_COMPLETION_EFFECT}:{cancel_completion_author_id}");
    let save_bookmark_author_id = scoped(SAVE_BOOKMARK_AUTHOR_ID);
    let bookmark_completion_author_id = scoped(BOOKMARK_COMPLETION_AUTHOR_ID);
    let bookmark_completion_context =
        format!("{BOOKMARK_COMPLETION_EFFECT}:{bookmark_completion_author_id}");
    let bookmark_load_completion_author_id = scoped(BOOKMARK_LOAD_COMPLETION_AUTHOR_ID);
    let bookmark_load_completion_context =
        format!("{BOOKMARK_LOAD_COMPLETION_EFFECT}:{bookmark_load_completion_author_id}");
    let status_author_id = scoped(STATUS_AUTHOR_ID);
    let bookmark_status_author_id = scoped(BOOKMARK_STATUS_AUTHOR_ID);
    let bookmark_retry_author_id = scoped(BOOKMARK_RETRY_AUTHOR_ID);

    // ── Query + match toggles ──
    ui.horizontal(|ui| {
        let edit = egui::TextEdit::singleline(&mut state.query)
            .hint_text("Search workspace")
            .desired_width(220.0);
        let resp = ui.add(edit);
        accessibility::emit_interactive_node(ui.ctx(), resp.id, &query_author_id);
        ui.ctx().accesskit_node_builder(resp.id, |node| {
            node.add_action(egui::accesskit::Action::SetValue);
        });
        let query_set_via_accesskit = crate::mcp::accesskit_string_set_value(ui, resp.id);
        if let Some(value) = query_set_via_accesskit.as_ref() {
            state.query.clone_from(value);
            state.query_set_value_generation = state.query_set_value_generation.wrapping_add(1);
            state.query_set_value_applied = Some(value.clone());
            ui.ctx().request_repaint();
        }
        if resp.changed() || query_set_via_accesskit.is_some() {
            state.invalidate_search_inputs();
        }
        emit_set_value_completion_node(
            ui,
            &query_author_id,
            "Find query SetValue completion",
            state.query_set_value_generation,
            state.query_set_value_applied.as_deref(),
        );
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            fire_search = true;
        }

        // Match-option toggles are synchronous and stay mounted, so they carry their own causal
        // same-target completion token in `value`. The boolean itself moves to a sibling Status node
        // (`find-in-files.toggle-*-state`) because `value` is now the acknowledgement channel.
        let case_btn = ui.add(egui::Button::new("Aa").selected(state.case_sensitive));
        accessibility::emit_interactive_node(ui.ctx(), case_btn.id, &toggle_case_author_id);
        let case_toggled = state.case_sensitive;
        let case_completion = state.same_target_completion_value(
            TOGGLE_COMPLETION_EFFECT,
            TOGGLE_CASE_AUTHOR_ID,
            &toggle_case_author_id,
        );
        ui.ctx().accesskit_node_builder(case_btn.id, move |node| {
            node.set_toggled(if case_toggled {
                egui::accesskit::Toggled::True
            } else {
                egui::accesskit::Toggled::False
            });
            if let Some(value) = case_completion.clone() {
                node.set_value(value);
            }
        });
        emit_status_node(
            ui,
            &toggle_case_state_author_id,
            "Find case-sensitive toggle state",
            Some(if state.case_sensitive { "true" } else { "false" }.to_owned()),
        );
        if case_btn.clicked() {
            state.case_sensitive = !state.case_sensitive;
            state.invalidate_search_inputs();
            state.record_same_target_activation(TOGGLE_CASE_AUTHOR_ID);
        }
        let word_btn = ui.add(egui::Button::new("W").selected(state.whole_word));
        accessibility::emit_interactive_node(ui.ctx(), word_btn.id, &toggle_word_author_id);
        let word_toggled = state.whole_word;
        let word_completion = state.same_target_completion_value(
            TOGGLE_COMPLETION_EFFECT,
            TOGGLE_WORD_AUTHOR_ID,
            &toggle_word_author_id,
        );
        ui.ctx().accesskit_node_builder(word_btn.id, move |node| {
            node.set_toggled(if word_toggled {
                egui::accesskit::Toggled::True
            } else {
                egui::accesskit::Toggled::False
            });
            if let Some(value) = word_completion.clone() {
                node.set_value(value);
            }
        });
        emit_status_node(
            ui,
            &toggle_word_state_author_id,
            "Find whole-word toggle state",
            Some(if state.whole_word { "true" } else { "false" }.to_owned()),
        );
        if word_btn.clicked() {
            state.whole_word = !state.whole_word;
            state.invalidate_search_inputs();
            state.record_same_target_activation(TOGGLE_WORD_AUTHOR_ID);
        }
        let regex_btn = ui.add(egui::Button::new(".*").selected(state.is_regex));
        accessibility::emit_interactive_node(ui.ctx(), regex_btn.id, &toggle_regex_author_id);
        let regex_toggled = state.is_regex;
        let regex_completion = state.same_target_completion_value(
            TOGGLE_COMPLETION_EFFECT,
            TOGGLE_REGEX_AUTHOR_ID,
            &toggle_regex_author_id,
        );
        ui.ctx().accesskit_node_builder(regex_btn.id, move |node| {
            node.set_toggled(if regex_toggled {
                egui::accesskit::Toggled::True
            } else {
                egui::accesskit::Toggled::False
            });
            if let Some(value) = regex_completion.clone() {
                node.set_value(value);
            }
        });
        emit_status_node(
            ui,
            &toggle_regex_state_author_id,
            "Find regex toggle state",
            Some(if state.is_regex { "true" } else { "false" }.to_owned()),
        );
        if regex_btn.clicked() {
            state.is_regex = !state.is_regex;
            state.invalidate_search_inputs();
            state.record_same_target_activation(TOGGLE_REGEX_AUTHOR_ID);
        }

        let search_btn = ui.add_enabled(
            !state.search_in_flight() && !state.preview_in_flight() && !state.apply_in_flight(),
            egui::Button::new("Search"),
        );
        accessibility::emit_interactive_node(ui.ctx(), search_btn.id, &search_author_id);
        let search_target_completion = state.search_target_completion_value(
            &search_author_id,
            &search_completion_author_id,
            &search_completion_context,
        );
        ui.ctx().accesskit_node_builder(search_btn.id, |node| {
            if let Some(value) = search_target_completion {
                node.set_value(value);
            }
        });
        if search_btn.clicked() {
            fire_search = true;
        }
    });

    let search_observer_completion =
        state.search_observer_completion_value(&search_completion_context);
    let search_completion_id = ui.make_persistent_id(&search_completion_author_id);
    ui.ctx()
        .accesskit_node_builder(search_completion_id, |node| {
            node.set_role(egui::accesskit::Role::Status);
            node.set_author_id(search_completion_author_id.clone());
            node.set_label("Find search action completion");
            if let Some(value) = search_observer_completion {
                node.set_value(value);
            }
        });

    // ── Replacement + replace actions ──
    ui.horizontal(|ui| {
        let replacement_enabled = !state.apply_in_flight();
        let edit = egui::TextEdit::singleline(&mut state.replacement)
            .hint_text("Replace with")
            .desired_width(220.0);
        let resp = ui.add_enabled(replacement_enabled, edit);
        accessibility::emit_interactive_node(ui.ctx(), resp.id, &replace_author_id);
        let mut replacement_set_via_accesskit = false;
        if replacement_enabled {
            ui.ctx().accesskit_node_builder(resp.id, |node| {
                node.add_action(egui::accesskit::Action::SetValue);
            });
            if let Some(value) = crate::mcp::accesskit_string_set_value(ui, resp.id) {
                state.replacement.clone_from(&value);
                state.replacement_set_value_generation =
                    state.replacement_set_value_generation.wrapping_add(1);
                state.replacement_set_value_applied = Some(value);
                ui.ctx().request_repaint();
                replacement_set_via_accesskit = true;
            }
        }
        // Emitted unconditionally: the observer identity must stay stable across the in-flight-Apply
        // gate so a disabled replacement field cannot silently drop its acknowledgement channel.
        emit_set_value_completion_node(
            ui,
            &replace_author_id,
            "Find replacement SetValue completion",
            state.replacement_set_value_generation,
            state.replacement_set_value_applied.as_deref(),
        );
        if resp.changed() || replacement_set_via_accesskit {
            state.invalidate_replacement_input();
        }

        let preview_enabled = state.result_set_key.is_some()
            && !state.preview_in_flight()
            && !state.apply_in_flight();
        let preview_btn = ui.add_enabled(preview_enabled, egui::Button::new("Preview Replace"));
        accessibility::emit_interactive_node(ui.ctx(), preview_btn.id, &preview_replace_author_id);
        let preview_declaration = state.preview_action.persistent_declaration(
            PREVIEW_COMPLETION_EFFECT,
            &preview_completion_context,
            &preview_completion_author_id,
            &state.preview_action_semantic_value(),
        );
        ui.ctx().accesskit_node_builder(preview_btn.id, |node| {
            if let Some(value) = preview_declaration {
                node.set_value(value);
            }
        });
        if preview_btn.clicked() {
            fire_preview = true;
        }

        // The DESTRUCTIVE Apply control is withdrawn while its own execution is in flight and while a
        // terminal success stands with its plans consumed; it returns with the next preview. That makes
        // it an exact TRANSIENT observer target (present+enabled at dispatch, absent at terminal), which
        // is the only shape the MCP acknowledgement boundary can prove for a control that can never be
        // re-enabled by its own completion. Outside that window it renders normally (disabled until a
        // non-stale preview exists), so the AC-8 gate is unchanged.
        if !state.apply_control_withdrawn() {
            let apply_btn = ui.add_enabled(
                state.can_apply() && !state.preview_in_flight() && !state.apply_in_flight(),
                egui::Button::new("Apply"),
            );
            accessibility::emit_interactive_node(ui.ctx(), apply_btn.id, &apply_author_id);
            let apply_declaration = state.apply_action.observer_declaration(
                APPLY_COMPLETION_EFFECT,
                &apply_completion_context,
                &apply_completion_author_id,
                &state.apply_action_semantic_value(),
            );
            ui.ctx().accesskit_node_builder(apply_btn.id, |node| {
                if let Some(value) = apply_declaration {
                    node.set_value(value);
                }
            });
            if apply_btn.clicked() {
                fire_apply = true;
            }
        }

        let cancel_btn = ui.button("Cancel");
        accessibility::emit_interactive_node(ui.ctx(), cancel_btn.id, &cancel_author_id);
        let cancel_declaration = state.cancel_action.persistent_declaration(
            CANCEL_COMPLETION_EFFECT,
            &cancel_completion_context,
            &cancel_completion_author_id,
            &state.cancel_action_semantic_value(),
        );
        ui.ctx().accesskit_node_builder(cancel_btn.id, |node| {
            if let Some(value) = cancel_declaration {
                node.set_value(value);
            }
        });
        if cancel_btn.clicked() {
            fire_cancel = true;
        }
    });

    emit_status_node(
        ui,
        &preview_completion_author_id,
        "Find preview action completion",
        state
            .preview_action
            .observer_value(PREVIEW_COMPLETION_EFFECT, &preview_completion_context),
    );
    emit_status_node(
        ui,
        &apply_completion_author_id,
        "Find apply action completion",
        state
            .apply_action
            .observer_value(APPLY_COMPLETION_EFFECT, &apply_completion_context),
    );
    emit_status_node(
        ui,
        &cancel_completion_author_id,
        "Find cancel action completion",
        state
            .cancel_action
            .observer_value(CANCEL_COMPLETION_EFFECT, &cancel_completion_context),
    );

    // ── Kind / tag / path filters ──
    ui.horizontal(|ui| {
        let combo = egui::ComboBox::from_id_salt(&kind_filter_author_id)
            .selected_text(state.kind.label())
            .show_ui(ui, |ui| {
                for kind in KindFilter::ALL {
                    ui.selectable_value(&mut state.kind, kind, kind.label());
                }
            });
        accessibility::emit_interactive_node(ui.ctx(), combo.response.id, &kind_filter_author_id);
        let selected_kind = state.kind.label().to_owned();
        ui.ctx()
            .accesskit_node_builder(combo.response.id, move |node| {
                node.set_label(format!("Kind filter: {selected_kind}"));
                node.set_value(selected_kind.clone());
            });
        if combo.response.changed() {
            state.invalidate_search_inputs();
        }

        let tag = egui::TextEdit::singleline(&mut state.tag_filter)
            .hint_text("tag ids")
            .desired_width(120.0);
        let tag_resp = ui.add(tag);
        accessibility::emit_interactive_node(ui.ctx(), tag_resp.id, &tag_filter_author_id);
        ui.ctx().accesskit_node_builder(tag_resp.id, |node| {
            node.add_action(egui::accesskit::Action::SetValue);
        });
        let tag_set_via_accesskit =
            crate::mcp::accesskit_string_set_value(ui, tag_resp.id).map(|value| {
                state.tag_filter.clone_from(&value);
                state.tag_filter_set_value_generation =
                    state.tag_filter_set_value_generation.wrapping_add(1);
                state.tag_filter_set_value_applied = Some(value);
                ui.ctx().request_repaint();
            });
        emit_set_value_completion_node(
            ui,
            &tag_filter_author_id,
            "Find tag filter SetValue completion",
            state.tag_filter_set_value_generation,
            state.tag_filter_set_value_applied.as_deref(),
        );
        if tag_resp.changed() || tag_set_via_accesskit.is_some() {
            state.invalidate_search_inputs();
        }

        let path = egui::TextEdit::singleline(&mut state.path_filter)
            .hint_text("path")
            .desired_width(120.0);
        let path_resp = ui.add(path);
        accessibility::emit_interactive_node(ui.ctx(), path_resp.id, &path_filter_author_id);
        ui.ctx().accesskit_node_builder(path_resp.id, |node| {
            node.add_action(egui::accesskit::Action::SetValue);
        });
        let path_set_via_accesskit =
            crate::mcp::accesskit_string_set_value(ui, path_resp.id).map(|value| {
                state.path_filter.clone_from(&value);
                state.path_filter_set_value_generation =
                    state.path_filter_set_value_generation.wrapping_add(1);
                state.path_filter_set_value_applied = Some(value);
                ui.ctx().request_repaint();
            });
        emit_set_value_completion_node(
            ui,
            &path_filter_author_id,
            "Find path filter SetValue completion",
            state.path_filter_set_value_generation,
            state.path_filter_set_value_applied.as_deref(),
        );
        if path_resp.changed() || path_set_via_accesskit.is_some() {
            state.invalidate_search_inputs();
        }

        let bm_btn = ui.add_enabled(
            !state.bookmark_in_flight(),
            egui::Button::new("Bookmark Search"),
        );
        accessibility::emit_interactive_node(ui.ctx(), bm_btn.id, &save_bookmark_author_id);
        let bookmark_save_declaration = state.bookmark_action.persistent_declaration(
            BOOKMARK_COMPLETION_EFFECT,
            &bookmark_completion_context,
            &bookmark_completion_author_id,
            &state.bookmark_save_semantic_value(),
        );
        ui.ctx().accesskit_node_builder(bm_btn.id, |node| {
            if let Some(value) = bookmark_save_declaration {
                node.set_value(value);
            }
        });
        if bm_btn.clicked() {
            fire_save_bookmark = true;
        }
    });

    emit_status_node(
        ui,
        &bookmark_completion_author_id,
        "Find bookmark persist action completion",
        state
            .bookmark_action
            .observer_value(BOOKMARK_COMPLETION_EFFECT, &bookmark_completion_context),
    );
    emit_status_node(
        ui,
        &bookmark_load_completion_author_id,
        "Find bookmark load action completion",
        state.bookmark_load_action.observer_value(
            BOOKMARK_LOAD_COMPLETION_EFFECT,
            &bookmark_load_completion_context,
        ),
    );

    // ── Status line ──
    ui.add_space(2.0);
    let status_text = state.header_status();
    let status_response = ui.label(&status_text);
    ui.ctx().accesskit_node_builder(status_response.id, |node| {
        node.set_role(egui::accesskit::Role::Status);
        node.set_author_id(status_author_id.clone());
        node.set_label(status_text.clone());
        node.set_value(status_text.clone());
        node.set_live(egui::accesskit::Live::Polite);
    });
    if let Some(bm_status) = &state.bookmark_status {
        let bookmark_status = ui.label(egui::RichText::new(bm_status).weak());
        ui.ctx().accesskit_node_builder(bookmark_status.id, |node| {
            node.set_role(egui::accesskit::Role::Status);
            node.set_author_id(bookmark_status_author_id.clone());
            node.set_label(bm_status.clone());
            node.set_value(bm_status.clone());
            node.set_live(egui::accesskit::Live::Polite);
        });
    }
    if state.bookmark_load_failed {
        let retry = ui.add_enabled(
            !state.bookmark_in_flight(),
            egui::Button::new("Retry saved searches"),
        );
        accessibility::emit_interactive_node(ui.ctx(), retry.id, &bookmark_retry_author_id);
        // Retry semantics: a successful reload REMOVES this control, a typed failure leaves it
        // mounted. That is exactly the flexible-target acknowledgement contract.
        let retry_declaration = state.bookmark_load_action.flexible_declaration(
            BOOKMARK_LOAD_COMPLETION_EFFECT,
            &bookmark_load_completion_context,
            &bookmark_load_completion_author_id,
            &state.bookmark_load_semantic_value(),
        );
        ui.ctx().accesskit_node_builder(retry.id, |node| {
            if let Some(value) = retry_declaration {
                node.set_value(value);
            }
        });
        if retry.clicked() {
            fire_retry_bookmarks = true;
        }
    }

    // ── Saved searches (bookmarks) ──
    let mut restore_bookmark: Option<SearchBookmark> = None;
    let mut remove_bookmark_id: Option<String> = None;
    if !state.bookmarks.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("Saved searches").strong());
        for bm in &state.bookmarks {
            // Content-derived routes compose their pane scope INSIDE the bounded composer (MT-113):
            // `scoped(...)` can only shorten the pane component, never the content, so pane-scoping an
            // already-oversized base would still overrun the canonical completion target budget.
            let restore_base_author_id = bookmark_restore_author_id(&bm.id);
            let restore_scoped_author_id =
                pane_scoped_bookmark_restore_author_id(&bm.id, secondary_pane_id);
            let remove_scoped_author_id =
                pane_scoped_bookmark_remove_author_id(&bm.id, secondary_pane_id);
            // Restore is purely local and keeps its row mounted -> same-target acknowledgement.
            let restore_completion = state.same_target_completion_value(
                BOOKMARK_RESTORE_COMPLETION_EFFECT,
                &restore_base_author_id,
                &restore_scoped_author_id,
            );
            // Remove is a persisted PUT: success removes the row, a typed failure leaves it mounted.
            let remove_declaration = state.bookmark_action.flexible_declaration(
                BOOKMARK_COMPLETION_EFFECT,
                &bookmark_completion_context,
                &bookmark_completion_author_id,
                &FindInFilesPanelState::bookmark_remove_semantic_value(&bm.id),
            );
            // WP-KERNEL-012 MT-119. The label is USER CONTENT and therefore unbounded: a saved search
            // carries the operator's whole query. Laying it out FIRST in a left-to-right row let it
            // consume the full pane width, pushing Restore past the pane edge (only "Re..." survived)
            // and Remove entirely OFF-PANE. Argus still addressed and clicked Remove — its AccessKit
            // node existed and terminalized — so an automated proof passed while a human operator could
            // neither see nor reach the control. Automated terminality and human reachability diverged.
            //
            // Fix: reserve the CONTROLS first against the row's right edge, then let the label truncate
            // into whatever remains. `right_to_left` places the first-added widget rightmost, so Remove
            // is added before Restore to preserve the visual order [label ... Restore Remove]. The full
            // query stays recoverable through the hover text (MT-119 AC-119-2 forbids a lossy truncation
            // that would stop an operator telling two saved searches apart).
            //
            // The author_ids are UNCHANGED — they are derived above from the bookmark id and pane scope
            // (MT-113's bounded composer), never from layout position — so this is layout-only and the
            // MT-113 identity contract is untouched.
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let remove = ui.small_button("Remove");
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        remove.id,
                        &remove_scoped_author_id,
                    );
                    ui.ctx().accesskit_node_builder(remove.id, |node| {
                        if let Some(value) = remove_declaration {
                            node.set_value(value);
                        }
                    });
                    if remove.clicked() {
                        remove_bookmark_id = Some(bm.id.clone());
                    }
                    let restore = ui.small_button("Restore");
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        restore.id,
                        &restore_scoped_author_id,
                    );
                    ui.ctx().accesskit_node_builder(restore.id, |node| {
                        if let Some(value) = restore_completion {
                            node.set_value(value);
                        }
                    });
                    if restore.clicked() {
                        restore_bookmark = Some(bm.clone());
                    }
                    // Truncates into the remaining width instead of expanding the row. egui's
                    // `show_tooltip_when_elided` defaults to TRUE and fires only when the galley was
                    // actually elided, so the untruncated query stays recoverable on hover EXACTLY when
                    // it is hidden (MT-119 AC-119-2) and a short, fully visible label pops no redundant
                    // tooltip. An explicit `.on_hover_text` here would stack a SECOND tooltip on top of
                    // that built-in one — egui documents combining them as the way to show *different*
                    // text, which is not what this row wants.
                    ui.add(egui::Label::new(&bm.label).truncate());
                });
            });
        }
    }

    // ── Results list (VIRTUALIZED — perf hygiene) ──
    // `open_hit_index` is a position into `visible_indices` (the on-screen visible list), resolved back
    // to `state.results` after the borrows end. `visible_indices` is the memoized client-side filter
    // result (cheap `Vec<usize>`, no per-frame regex recompile and no per-frame clone of the hits).
    let mut open_hit_index: Option<usize> = None;
    let mut activated_result_author_id: Option<String> = None;
    let visible_indices: Vec<usize> = state.with_visible_indices(<[usize]>::to_vec);
    if !visible_indices.is_empty() {
        ui.separator();
        // Borrow `results` (not all of `state`) so the row closure only holds the shared read it needs.
        let results = &state.results;
        let same_target_activations = &state.same_target_activations;
        let result_open_completion = state.result_open_completion;
        // Uniform slot height so `show_rows` lays out ONLY the on-screen rows (title line + excerpt line +
        // Frame::group padding) instead of materializing every row in a large paginated result set.
        let row_height = ui.text_style_height(&egui::TextStyle::Body) * 2.0 + 18.0;
        egui::ScrollArea::vertical()
            .id_salt("find-in-files.results")
            .max_height(220.0)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, visible_indices.len(), |ui, range| {
                for vi in range {
                    let hit = &results[visible_indices[vi]];
                    let frame = egui::Frame::group(ui.style());
                    let inner = frame.show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&hit.title);
                                ui.label(
                                    egui::RichText::new(format!("[{}]", hit.source_kind))
                                        .color(ui.visuals().weak_text_color())
                                        .small(),
                                );
                            });
                            if !hit.excerpt.is_empty() {
                                ui.label(egui::RichText::new(&hit.excerpt).small());
                            }
                        });
                    });
                    let row = inner.response.interact(egui::Sense::click());
                    let row_base_author_id = result_author_id(&hit.source_kind, &hit.ref_id);
                    let row_scoped_author_id = pane_scoped_result_author_id(
                        &hit.source_kind,
                        &hit.ref_id,
                        secondary_pane_id,
                    );
                    accessibility::emit_interactive_node(ui.ctx(), row.id, &row_scoped_author_id);
                    // Under the shell, activating a row REPLACES this surface with the routed editor,
                    // so the row is a TRANSIENT observer target bound to the shell-owned navigation
                    // observer: the acknowledgement survives the row (and this whole panel)
                    // disappearing, and it terminalises only on the exact routed tab identity. The
                    // unbound compatibility `show()` path never routes away, so it keeps a plain
                    // same-target token instead of declaring an observer that does not exist.
                    let row_completion = if result_open_completion.ready {
                        crate::mcp::action::serialize_observer_click_target(
                            RESULT_OPEN_COMPLETION_EFFECT,
                            RESULT_OPEN_COMPLETION_CONTEXT,
                            result_open_completion.generation,
                            RESULT_OPEN_COMPLETION_AUTHOR_ID,
                            &open_result_semantic(&hit.source_kind, &hit.ref_id),
                        )
                    } else {
                        let row_activation_count = same_target_activations
                            .get(&row_base_author_id)
                            .copied()
                            .unwrap_or(0);
                        crate::mcp::action::serialize_same_target_click_completion(
                            RESULT_COMPLETION_EFFECT,
                            &row_scoped_author_id,
                            row_activation_count,
                            if row_activation_count == 0 {
                                crate::mcp::action::ClickCompletionState::Ready
                            } else {
                                crate::mcp::action::ClickCompletionState::Applied
                            },
                        )
                    };
                    ui.ctx().accesskit_node_builder(row.id, |node| {
                        if let Some(value) = row_completion {
                            node.set_value(value);
                        }
                    });
                    if row.clicked() {
                        open_hit_index = Some(vi);
                        activated_result_author_id = Some(row_base_author_id);
                    }
                }
            });
    }

    // ── Preview list ──
    if !state.preview_plans.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("Replacement preview").strong());
        let text_color = ui.visuals().text_color();
        let mut toggled_preview_document_id = None;
        let mut activated_preview_author_id: Option<String> = None;
        let preview_activations = &state.same_target_activations;
        egui::ScrollArea::vertical()
            .id_salt("find-in-files.preview")
            .max_height(220.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for plan in &state.preview_plans {
                    let preview_row_base_author_id = preview_author_id(&plan.document_id);
                    let preview_row_author_id =
                        pane_scoped_preview_author_id(&plan.document_id, secondary_pane_id);
                    let preview_open = state
                        .expanded_preview_document_ids
                        .contains(&plan.document_id);
                    let header = egui::CollapsingHeader::new(format!(
                        "{} ({})",
                        plan.title, plan.match_count
                    ))
                    .id_salt(&preview_row_author_id)
                    .open(Some(preview_open))
                    .show(ui, |ui| {
                        let before = ui.label(
                            egui::RichText::new(format!("before: {}", plan.before_preview))
                                .small()
                                .weak(),
                        );
                        ui.ctx().accesskit_node_builder(before.id, |node| {
                            node.set_author_id(pane_scoped_preview_before_author_id(
                                &plan.document_id,
                                secondary_pane_id,
                            ));
                            node.set_label("Replacement preview before");
                            node.set_value(plan.before_preview.clone());
                        });
                        let after = ui.label(
                            egui::RichText::new(format!("after: {}", plan.after_preview))
                                .small()
                                .weak(),
                        );
                        ui.ctx().accesskit_node_builder(after.id, |node| {
                            node.set_author_id(pane_scoped_preview_after_author_id(
                                &plan.document_id,
                                secondary_pane_id,
                            ));
                            node.set_label("Replacement preview after");
                            node.set_value(plan.after_preview.clone());
                        });
                        // Render the after-preview with the matched replacement highlighted via
                        // the theme `search_highlight_bg` token (NO Color32 literal — the theme
                        // guard). Each per-match after_preview is a small highlighted chip.
                        for mp in &plan.match_previews {
                            let mut job = egui::text::LayoutJob::default();
                            job.append(
                                &mp.after_preview,
                                0.0,
                                egui::TextFormat {
                                    color: text_color,
                                    background: palette.search_highlight_bg,
                                    ..Default::default()
                                },
                            );
                            ui.label(job);
                        }
                    });
                    accessibility::emit_interactive_node(
                        ui.ctx(),
                        header.header_response.id,
                        &preview_row_author_id,
                    );
                    let preview_activation_count = preview_activations
                        .get(&preview_row_base_author_id)
                        .copied()
                        .unwrap_or(0);
                    let preview_row_completion =
                        crate::mcp::action::serialize_same_target_click_completion(
                            PREVIEW_ROW_COMPLETION_EFFECT,
                            &preview_row_author_id,
                            preview_activation_count,
                            if preview_activation_count == 0 {
                                crate::mcp::action::ClickCompletionState::Ready
                            } else {
                                crate::mcp::action::ClickCompletionState::Applied
                            },
                        );
                    ui.ctx()
                        .accesskit_node_builder(header.header_response.id, |node| {
                            if let Some(value) = preview_row_completion {
                                node.set_value(value);
                            }
                        });
                    let accesskit_clicked = ui.input(|input| {
                        input
                            .accesskit_action_requests(
                                header.header_response.id,
                                egui::accesskit::Action::Click,
                            )
                            .next()
                            .is_some()
                    });
                    if header.header_response.clicked() || accesskit_clicked {
                        toggled_preview_document_id = Some(plan.document_id.clone());
                        activated_preview_author_id = Some(preview_row_base_author_id);
                    }
                }
            });
        if let Some(document_id) = toggled_preview_document_id {
            if !state.expanded_preview_document_ids.remove(&document_id) {
                state.expanded_preview_document_ids.insert(document_id);
            }
            ui.ctx().request_repaint();
        }
        if let Some(author_id) = activated_preview_author_id {
            state.record_same_target_activation(&author_id);
        }
    }

    // ── Dispatch deferred actions (after immutable borrows end) ──
    if let Some(vi) = open_hit_index {
        if let Some(hit) = visible_indices
            .get(vi)
            .and_then(|&ri| state.results.get(ri))
        {
            (callbacks.on_open_hit)(hit);
        }
    }
    if let Some(author_id) = activated_result_author_id {
        state.record_same_target_activation(&author_id);
    }
    if let Some(bm) = restore_bookmark {
        let restore_author_id = bookmark_restore_author_id(&bm.id);
        state.restore_bookmark(&bm);
        state.record_same_target_activation(&restore_author_id);
    }
    if let Some(id) = remove_bookmark_id {
        // The observer's `pending_target` MUST be the exact route the row published, so it is composed
        // through the SAME bounded composer rather than by pane-scoping an unbounded base (MT-113).
        state.bookmark_action.begin(
            pane_scoped_bookmark_remove_author_id(&id, secondary_pane_id),
            FindInFilesPanelState::bookmark_remove_semantic_value(&id),
        );
        state.remove_bookmark(search_client, workspace_id, &id);
        if !state.bookmark_in_flight() {
            // The persisted PUT never started (no workspace binding); report that exact typed
            // terminal state instead of leaving the observer pending.
            let message = state
                .bookmark_status
                .clone()
                .unwrap_or_else(|| "bookmark remove did not start".to_owned());
            state
                .bookmark_action
                .fail(BOOKMARK_COMPLETION_EFFECT, message);
        }
    }
    if fire_cancel {
        let semantic = state.cancel_action_semantic_value();
        let apply_was_in_flight = state.apply_in_flight();
        state.cancel_action.begin(cancel_author_id, semantic);
        state.request_cancel();
        if !apply_was_in_flight {
            // Local-only cancellation: the preview clear IS the whole terminal effect, so complete it
            // now instead of waiting for a destructive delivery that will never arrive.
            state.cancel_action.complete(
                serde_json::json!({
                    "kind": "cancel_local_preview_clear",
                    "status": state.replace_status.clone(),
                })
                .to_string(),
            );
        }
    }
    if fire_search {
        let semantic = state.search_action_semantic_value();
        state.begin_search_action(search_author_id, semantic);
        if !state.run_search(search_client, workspace_id) {
            state.fail_search_action(
                state
                    .error
                    .clone()
                    .unwrap_or_else(|| "search did not start".to_owned()),
            );
        }
    }
    if fire_preview {
        let semantic = state.preview_action_semantic_value();
        state
            .preview_action
            .begin(preview_replace_author_id, semantic);
        state.run_preview_replace(doc_client, workspace_id);
        if !state.preview_in_flight() {
            // A stale-result guard, missing workspace, regex-compile failure, or empty document set
            // is a REAL terminal outcome of the Preview action with no HTTP request. Publish it as
            // such: a blocked destructive preview must never look like a pending or applied one.
            let message = state
                .error
                .clone()
                .or_else(|| state.replace_status.clone())
                .unwrap_or_else(|| {
                    "preview replace did not start and reported no status".to_owned()
                });
            state
                .preview_action
                .fail(PREVIEW_COMPLETION_EFFECT, message);
        }
    }
    if fire_apply {
        let semantic = state.apply_action_semantic_value();
        state.apply_action.begin(apply_author_id, semantic);
        state.run_apply(doc_client, workspace_id);
        if !state.apply_in_flight() {
            let message = state
                .replace_status
                .clone()
                .or_else(|| state.error.clone())
                .unwrap_or_else(|| "apply did not start and reported no status".to_owned());
            state.apply_action.fail(APPLY_COMPLETION_EFFECT, message);
        }
    }
    if fire_save_bookmark {
        let semantic = state.bookmark_save_semantic_value();
        state.bookmark_action.begin(save_bookmark_author_id, semantic);
        state.save_bookmark(search_client, workspace_id);
        if !state.bookmark_in_flight() {
            let message = state
                .bookmark_status
                .clone()
                .unwrap_or_else(|| "bookmark save did not start".to_owned());
            state
                .bookmark_action
                .fail(BOOKMARK_COMPLETION_EFFECT, message);
        }
    }
    if fire_retry_bookmarks {
        let semantic = state.bookmark_load_semantic_value();
        state
            .bookmark_load_action
            .begin(bookmark_retry_author_id, semantic);
        state.load_bookmarks(search_client, workspace_id);
        if !state.bookmark_in_flight() {
            let message = state
                .bookmark_status
                .clone()
                .unwrap_or_else(|| "bookmark reload did not start".to_owned());
            state
                .bookmark_load_action
                .fail(BOOKMARK_LOAD_COMPLETION_EFFECT, message);
        }
    }
}

// ── Pane factory (the in-product render path — AC, the WP-011 registry dispatch) ──────────────────────

/// Per-frame inputs the shell pushes to the [`FindInFilesPaneFactory`] (workspace id + palette IN) and
/// the open-hit requests it drains OUT. Mirrors the MT-028 `LoomSearchV2PaneShared` shape so the live
/// app threads the active workspace + theme through the `&self` `PaneFactory::render` without `&mut self`
/// on the factory map.
pub struct FindInFilesPaneShared {
    pub workspace_id: Option<String>,
    /// Monotonic shell generation. Unlike pane-local observation, this advances even while the pane is
    /// hidden, so A→B→A cannot accept an async completion from the first A binding.
    pub workspace_generation: u64,
    pub palette: HsPalette,
    /// Shell-authoritative active pane for diagnostics and routing. Author-id ownership is deliberately
    /// independent of focus and comes from the factory's stable primary-pane lease.
    pub active_pane_id: Option<PaneId>,
    /// Typed result requests retain the exact origin pane and workspace across the shared queue.
    pub open_requests: Vec<FindInFilesOpenRequest>,
    /// Bookmark mount bindings are pane-local: two Find panes must each load their workspace bookmarks
    /// without one pane suppressing the other's mount effect.
    pub(crate) bookmarks_loaded_for: BTreeMap<PaneId, (Option<String>, u64)>,
    /// Shell-owned result-navigation completion binding, pushed per frame BEFORE the pane renders so a
    /// result row can declare the exact current observer generation.
    pub result_open_completion: FindResultOpenCompletionBinding,
}

/// A result-navigation request retaining the exact mounted origin and workspace binding.
#[derive(Debug, Clone, PartialEq)]
pub struct FindInFilesOpenRequest {
    pub origin_pane_id: PaneId,
    pub workspace_id: String,
    pub hit: LoomGraphSearchHit,
}

impl FindInFilesPaneShared {
    pub fn new(palette: HsPalette) -> Self {
        Self {
            workspace_id: None,
            workspace_generation: 0,
            palette,
            active_pane_id: None,
            open_requests: Vec::new(),
            bookmarks_loaded_for: BTreeMap::new(),
            result_open_completion: FindResultOpenCompletionBinding::default(),
        }
    }
}

/// The CONCRETE `PaneFactory` for [`PaneType::FindInFiles`] — the in-product render path that makes the
/// "Find in Files" pane render the REAL panel instead of the placeholder. Mirrors the MT-028
/// `LoomSearchV2PaneFactory` exactly: panel state behind a `Mutex` (Send + Sync), the per-frame
/// workspace id + palette + open-hit drain flowing through [`FindInFilesPaneShared`], and the HTTP
/// transport reusing the real verified clients.
pub struct FindInFilesPaneFactory {
    states: Arc<Mutex<BTreeMap<PaneId, FindInFilesPanelState>>>,
    initial_state: Mutex<Option<FindInFilesPanelState>>,
    search_client: WorkspaceSearchClient,
    doc_client: RichDocClient,
    shared: Arc<Mutex<FindInFilesPaneShared>>,
    primary: Mutex<PrimaryPaneLease>,
}

/// Single-pane managed-proof view over the pane-keyed factory state. Production hosts use
/// [`FindInFilesPaneFactory::states_handle`]; this compatibility view intentionally selects the first
/// mounted pane and therefore cannot collapse two production pane states into one.
#[derive(Clone)]
pub struct FindInFilesStateHandle {
    states: Arc<Mutex<BTreeMap<PaneId, FindInFilesPanelState>>>,
}

pub struct FindInFilesStateGuard<'a>(
    std::sync::MutexGuard<'a, BTreeMap<PaneId, FindInFilesPanelState>>,
);

impl std::ops::Deref for FindInFilesStateGuard<'_> {
    type Target = FindInFilesPanelState;

    fn deref(&self) -> &Self::Target {
        self.0
            .values()
            .next()
            .expect("Find-in-Files state handle requires one rendered pane")
    }
}

impl std::ops::DerefMut for FindInFilesStateGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .values_mut()
            .next()
            .expect("Find-in-Files state handle requires one rendered pane")
    }
}

impl FindInFilesStateHandle {
    pub fn lock(&self) -> Result<FindInFilesStateGuard<'_>, &'static str> {
        self.states
            .lock()
            .map(FindInFilesStateGuard)
            .map_err(|_| "Find-in-Files pane state mutex poisoned")
    }
}

#[derive(Debug, Default)]
struct PrimaryPaneLease {
    pane_id: Option<PaneId>,
    last_seen_pass: u64,
}

impl PrimaryPaneLease {
    fn is_primary(&mut self, pane_id: &PaneId, pass: u64) -> bool {
        match self.pane_id.as_ref() {
            None => {
                self.pane_id = Some(pane_id.clone());
                self.last_seen_pass = pass;
                true
            }
            Some(primary) if primary == pane_id => {
                self.last_seen_pass = pass;
                true
            }
            Some(_) if pass > self.last_seen_pass.saturating_add(1) => {
                self.pane_id = Some(pane_id.clone());
                self.last_seen_pass = pass;
                true
            }
            Some(_) => false,
        }
    }
}

impl FindInFilesPaneFactory {
    pub fn new(
        search_client: WorkspaceSearchClient,
        doc_client: RichDocClient,
        shared: Arc<Mutex<FindInFilesPaneShared>>,
    ) -> Self {
        Self::with_state(
            search_client,
            doc_client,
            shared,
            FindInFilesPanelState::new(),
        )
    }

    pub fn with_state(
        search_client: WorkspaceSearchClient,
        doc_client: RichDocClient,
        shared: Arc<Mutex<FindInFilesPaneShared>>,
        state: FindInFilesPanelState,
    ) -> Self {
        Self {
            states: Arc::new(Mutex::new(BTreeMap::new())),
            initial_state: Mutex::new(Some(state)),
            search_client,
            doc_client,
            shared,
            primary: Mutex::new(PrimaryPaneLease::default()),
        }
    }

    /// Exact pane-keyed mounted state handle for structured diagnostics and managed runtime proofs.
    pub fn states_handle(&self) -> Arc<Mutex<BTreeMap<PaneId, FindInFilesPanelState>>> {
        Arc::clone(&self.states)
    }

    /// Compatibility handle for existing single-pane managed proofs.
    pub fn state_handle(&self) -> FindInFilesStateHandle {
        FindInFilesStateHandle {
            states: Arc::clone(&self.states),
        }
    }
}

impl PaneFactory for FindInFilesPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::FindInFiles
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        let (workspace_id, workspace_generation, palette, result_open_completion) = {
            let guard = self.shared.lock().unwrap_or_else(|p| p.into_inner());
            (
                guard.workspace_id.clone(),
                guard.workspace_generation,
                guard.palette.clone(),
                guard.result_open_completion,
            )
        };
        let secondary_pane_id = {
            let mut primary = self
                .primary
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if primary.is_primary(&ctx.record.pane_id, ui.ctx().cumulative_pass_nr()) {
                None
            } else {
                Some(ctx.record.pane_id.as_ref())
            }
        };
        let mut states = self.states.lock().unwrap_or_else(|p| p.into_inner());
        if !states.contains_key(&ctx.record.pane_id) {
            let initial = self
                .initial_state
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .take()
                .unwrap_or_default();
            states.insert(ctx.record.pane_id.clone(), initial);
        }
        let state = states
            .get_mut(&ctx.record.pane_id)
            .expect("pane state inserted immediately above");
        state.set_result_open_completion_binding(result_open_completion);
        if state.bind_workspace(workspace_id.as_deref(), workspace_generation) {
            let mut guard = self.shared.lock().unwrap_or_else(|p| p.into_inner());
            guard
                .open_requests
                .retain(|request| request.origin_pane_id != ctx.record.pane_id);
        }

        // Load the workspace's bookmarks exactly once per workspace (the React mount-effect).
        let needs_bookmark_load = {
            let mut guard = self.shared.lock().unwrap_or_else(|p| p.into_inner());
            let binding = (workspace_id.clone(), workspace_generation);
            if guard.bookmarks_loaded_for.get(&ctx.record.pane_id) != Some(&binding) {
                guard
                    .bookmarks_loaded_for
                    .insert(ctx.record.pane_id.clone(), binding);
                workspace_id.is_some()
            } else {
                false
            }
        };
        if needs_bookmark_load {
            state.load_bookmarks(&self.search_client, workspace_id.as_deref());
        }

        let shared_for_open = Arc::clone(&self.shared);
        let origin_pane_id = ctx.record.pane_id.clone();
        let open_workspace_id = workspace_id.clone();
        let mut on_open = move |hit: &LoomGraphSearchHit| {
            if let (Some(workspace_id), Ok(mut guard)) =
                (open_workspace_id.as_ref(), shared_for_open.lock())
            {
                guard.open_requests.push(FindInFilesOpenRequest {
                    origin_pane_id: origin_pane_id.clone(),
                    workspace_id: workspace_id.clone(),
                    hit: hit.clone(),
                });
            }
        };
        let mut callbacks = FindInFilesCallbacks {
            on_open_hit: &mut on_open,
        };
        show_with_author_scope(
            ui,
            state,
            &palette,
            &self.search_client,
            &self.doc_client,
            workspace_id.as_deref(),
            &mut callbacks,
            secondary_pane_id,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc(text_nodes: &[&str], code: Option<&str>) -> serde_json::Value {
        let mut content: Vec<serde_json::Value> = text_nodes
            .iter()
            .map(|t| json!({ "type": "text", "text": t }))
            .collect();
        if let Some(code) = code {
            content.push(json!({
                "type": "codeBlock",
                "attrs": { "code": code, "language": "rust" }
            }));
        }
        json!({ "type": "doc", "content": content })
    }

    #[test]
    fn compile_regex_escapes_non_regex_query() {
        // RISK-8: `a.b` in non-regex mode must NOT match `acb` (the dot is escaped to literal).
        let re = compile_search_regex("a.b", MatchOptions::default()).unwrap();
        assert!(re.is_match("a.b"), "literal dot matches a.b");
        assert!(
            !re.is_match("acb"),
            "RISK-8: escaped dot does NOT match acb"
        );
    }

    #[test]
    fn find_in_files_regex_compile_error() {
        // PT-4: an invalid regex returns Err with a non-empty message (no panic).
        let err = compile_search_regex(
            "[invalid",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(
            !err.is_empty(),
            "PT-4: invalid regex yields a non-empty error"
        );
    }

    #[test]
    fn compile_regex_empty_query_is_err() {
        assert!(compile_search_regex("   ", MatchOptions::default()).is_err());
    }

    #[test]
    fn case_insensitive_by_default() {
        let re = compile_search_regex("Foo", MatchOptions::default()).unwrap();
        assert!(
            re.is_match("foo") && re.is_match("FOO"),
            "case-insensitive when not case_sensitive"
        );
        let re2 = compile_search_regex(
            "Foo",
            MatchOptions {
                case_sensitive: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            re2.is_match("Foo") && !re2.is_match("foo"),
            "case-sensitive when set"
        );
    }

    #[test]
    fn replace_segment_zero_length_match_terminates() {
        // RISK-3: a pattern that can match empty (`a*`) must terminate (no infinite loop) and replace
        // the non-empty runs only.
        let re = compile_search_regex(
            "a*",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "baaab",
            &re,
            "X",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        );
        // The non-empty `aaa` run is replaced; the zero-length matches at b-positions are skipped.
        assert!(res.text.contains('X'), "the aaa run was replaced");
        assert!(res.count >= 1, "at least one non-empty match replaced");
    }

    #[test]
    fn replace_segment_whole_word_skips_substring() {
        let re = compile_search_regex(
            "cat",
            MatchOptions {
                whole_word: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "cat category",
            &re,
            "dog",
            MatchOptions {
                whole_word: true,
                ..Default::default()
            },
        );
        assert_eq!(
            res.count, 1,
            "only the standalone 'cat' replaced, not 'cat' inside 'category'"
        );
        assert_eq!(res.text, "dog category");
    }

    #[test]
    fn replace_segment_regex_group_expansion() {
        let re = compile_search_regex(
            r"(\w+)@(\w+)",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "user@host",
            &re,
            "$2.$1",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        );
        assert_eq!(res.text, "host.user", "$1/$2 group expansion");
    }

    #[test]
    fn replace_segment_dollar_literals() {
        let re = compile_search_regex(
            "x",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        )
        .unwrap();
        let res = replace_segment(
            "x",
            &re,
            "$$$&",
            MatchOptions {
                is_regex: true,
                ..Default::default()
            },
        );
        assert_eq!(res.text, "$x", "$$ => literal $, $& => whole match");
    }

    #[test]
    fn replace_in_content_walks_text_and_code_preserves_other_nodes() {
        // RISK-4: text nodes AND attrs.code are replaced; a non-text node (an embed) round-trips verbatim.
        let mut content = doc(&["hello FIND_TARGET world"], Some("let FIND_TARGET = 1;"));
        // Inject an embed node that must be preserved untouched.
        content["content"].as_array_mut().unwrap().push(json!({
            "type": "hsEmbed",
            "attrs": { "asset_id": "AST-1", "kind": "image" }
        }));
        let re = compile_search_regex("FIND_TARGET", MatchOptions::default()).unwrap();
        let res = replace_in_content(&content, &re, "REPLACED", MatchOptions::default());
        assert_eq!(res.count, 2, "one match in text, one in code");
        let arr = res.content["content"].as_array().unwrap();
        assert_eq!(arr[0]["text"], "hello REPLACED world");
        assert_eq!(arr[1]["attrs"]["code"], "let REPLACED = 1;");
        // The embed node is preserved VERBATIM.
        assert_eq!(arr[2]["type"], "hsEmbed");
        assert_eq!(arr[2]["attrs"]["asset_id"], "AST-1");
        assert!(res.after_preview.contains("REPLACED"));
    }

    #[test]
    fn replace_in_content_no_match_returns_zero_and_unchanged() {
        let content = doc(&["nothing here"], None);
        let re = compile_search_regex("ABSENT", MatchOptions::default()).unwrap();
        let res = replace_in_content(&content, &re, "X", MatchOptions::default());
        assert_eq!(res.count, 0);
        assert_eq!(res.content, content, "no-match returns the tree unchanged");
    }

    #[test]
    fn document_id_from_hit_requires_krd_prefix() {
        // RISK-5: a non-KRD- document_id returns None.
        let hit_bad = LoomGraphSearchHit {
            source_kind: "loom_block".into(),
            result_kind: "loom_block".into(),
            ref_id: "blk-1".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({ "document_id": "DOC-1" }),
            block: None,
        };
        assert_eq!(
            document_id_from_hit(&hit_bad),
            None,
            "RISK-5: non-KRD id rejected"
        );

        let hit_good = LoomGraphSearchHit {
            source_kind: "loom_block".into(),
            result_kind: "loom_block".into(),
            ref_id: "blk-1".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({ "rich_document_id": "KRD-42" }),
            block: None,
        };
        assert_eq!(document_id_from_hit(&hit_good), Some("KRD-42".to_owned()));

        // source_kind == document falls back to ref_id (when KRD-).
        let hit_doc = LoomGraphSearchHit {
            source_kind: "document".into(),
            result_kind: "loom_block".into(),
            ref_id: "KRD-99".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({}),
            block: None,
        };
        assert_eq!(document_id_from_hit(&hit_doc), Some("KRD-99".to_owned()));
    }

    #[test]
    fn document_id_from_hit_block_document_id() {
        let hit = LoomGraphSearchHit {
            source_kind: "loom_block".into(),
            result_kind: "loom_block".into(),
            ref_id: "blk-1".into(),
            title: "T".into(),
            excerpt: String::new(),
            metadata: json!({}),
            block: Some(json!({ "document_id": "KRD-7" })),
        };
        assert_eq!(document_id_from_hit(&hit), Some("KRD-7".to_owned()));
    }

    #[test]
    fn stale_plan_keys_change_with_query_and_replacement() {
        // RISK-2/MC-2: a query change OR a replacement change yields a different key.
        let opts = MatchOptions::default();
        let k1 = search_plan_key("cats", KindFilter::All, "", "", opts);
        let k2 = search_plan_key("cats and dogs", KindFilter::All, "", "", opts);
        assert_ne!(k1, k2, "query change => different search key");
        let r1 = replace_plan_key(&k1, "X");
        let r2 = replace_plan_key(&k1, "Y");
        assert_ne!(r1, r2, "replacement change => different replace key");
    }

    #[test]
    fn can_apply_false_when_preview_stale() {
        let mut s = FindInFilesPanelState::new();
        s.query = "cats".into();
        s.results = vec![]; // unused for the key
        s.result_set_key = Some(s.current_search_key());
        s.preview_plans = vec![ReplacementPlan {
            workspace_id: "ws-1".into(),
            document_id: "KRD-1".into(),
            title: "T".into(),
            expected_version: 1,
            content_json_after: json!({}),
            before_sha256: "0".repeat(64),
            after_sha256: "1".repeat(64),
            crdt_document_id: None,
            match_count: 1,
            before_preview: String::new(),
            after_preview: String::new(),
            match_previews: vec![],
        }];
        s.preview_plan_key = Some(s.current_replace_key());
        assert!(s.can_apply(), "fresh preview => can apply");
        // Change the query AFTER the preview => the plan key no longer matches => cannot apply.
        s.query = "dogs".into();
        assert!(
            !s.can_apply(),
            "RISK-2/MC-2: a since-changed query makes the preview stale"
        );
    }

    #[test]
    fn no_workspace_search_sets_error_without_loading() {
        let mut s = FindInFilesPanelState::new();
        s.query = "x".into();
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let client = WorkspaceSearchClient::new("http://test.local", rt.handle().clone());
        s.run_search(&client, None);
        assert_eq!(s.error.as_deref(), Some("No workspace selected"));
        assert!(!s.loading, "MC-7: no HTTP fired");
    }

    #[test]
    fn preview_stale_result_guard() {
        let mut s = FindInFilesPanelState::new();
        s.query = "cats".into();
        // result_set_key reflects an OLD query; the current query differs => stale.
        s.result_set_key = Some(search_plan_key(
            "old",
            KindFilter::All,
            "",
            "",
            MatchOptions::default(),
        ));
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let client = RichDocClient::new("http://test.local", rt.handle().clone());
        s.run_preview_replace(&client, Some("ws-1"));
        assert!(
            s.replace_status
                .as_deref()
                .unwrap_or_default()
                .contains("stale"),
            "RISK-2/MC-2: stale results show the warning, compute no preview"
        );
        assert!(s.preview_plans.is_empty());
    }

    #[test]
    fn bookmark_blob_round_trips_with_schema_id() {
        let bm = SearchBookmark {
            id: "alpha".into(),
            label: "alpha".into(),
            query: "alpha".into(),
            kind: KindFilter::Document,
            tag_filter: "t1".into(),
            path_filter: "src".into(),
            case_sensitive: true,
            whole_word: false,
            is_regex: true,
            saved_at: "2026-06-23T00:00:00Z".into(),
        };
        let blob = bookmark_state_blob(std::slice::from_ref(&bm));
        // RISK-6: the schema_id MUST be exactly the backend-validated value.
        assert_eq!(blob["schema_id"], WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID);
        let parsed = parse_bookmark_state(&blob).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], bm, "bookmark round-trips through the blob");
    }

    #[test]
    fn bookmark_blob_caps_at_twenty() {
        let many: Vec<SearchBookmark> = (0..30)
            .map(|i| SearchBookmark {
                id: format!("b{i}"),
                label: format!("b{i}"),
                query: format!("q{i}"),
                kind: KindFilter::All,
                tag_filter: String::new(),
                path_filter: String::new(),
                case_sensitive: false,
                whole_word: false,
                is_regex: false,
                saved_at: "2026-06-23T00:00:00Z".into(),
            })
            .collect();
        let blob = bookmark_state_blob(&many);
        assert_eq!(
            blob["bookmarks"].as_array().unwrap().len(),
            MAX_WORKSPACE_SEARCH_BOOKMARKS
        );
        assert_eq!(
            parse_bookmark_state(&blob)
                .expect("exactly twenty bookmarks are accepted")
                .len(),
            MAX_WORKSPACE_SEARCH_BOOKMARKS
        );
        let twenty_one = json!({
            "schema_id": WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID,
            "bookmarks": many
                .iter()
                .take(MAX_WORKSPACE_SEARCH_BOOKMARKS + 1)
                .map(SearchBookmark::to_json)
                .collect::<Vec<_>>(),
        });
        assert!(parse_bookmark_state(&twenty_one)
            .expect_err("twenty-one bookmarks must fail closed")
            .contains("maximum is 20"));
    }

    #[test]
    fn malformed_bookmark_payload_fails_closed() {
        let malformed = json!({
            "schema_id": WORKSPACE_SEARCH_BOOKMARK_SCHEMA_ID,
            "bookmarks": [
                {"id":"ok"},
                {"id":"also-incomplete"}
            ]
        });
        assert!(parse_bookmark_state(&malformed).is_err());
        assert!(parse_bookmark_state(&json!({"results": []})).is_err());
        let mut unknown_kind = bookmark_state_blob(&[SearchBookmark {
            id: "unknown-kind".into(),
            label: "unknown-kind".into(),
            query: "q".into(),
            kind: KindFilter::All,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-07-15T00:00:00Z".into(),
        }]);
        unknown_kind["bookmarks"][0]["kind"] = json!("future_kind");
        assert!(parse_bookmark_state(&unknown_kind)
            .expect_err("unknown kind must fail closed")
            .contains("unsupported bookmark kind"));

        let mut blank_saved_at = bookmark_state_blob(&[SearchBookmark {
            id: "blank-saved-at".into(),
            label: "blank-saved-at".into(),
            query: "q".into(),
            kind: KindFilter::All,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-07-15T00:00:00Z".into(),
        }]);
        blank_saved_at["bookmarks"][0]["savedAt"] = json!("   ");
        assert!(parse_bookmark_state(&blank_saved_at)
            .expect_err("blank savedAt must fail closed")
            .contains("savedAt must not be blank"));

        let mut invalid_saved_at = blank_saved_at;
        invalid_saved_at["bookmarks"][0]["savedAt"] = json!("not-a-timestamp");
        assert!(parse_bookmark_state(&invalid_saved_at)
            .expect_err("non-RFC3339 savedAt must fail closed")
            .contains("savedAt must be an RFC3339 timestamp"));

        let mut offset_saved_at = invalid_saved_at;
        offset_saved_at["bookmarks"][0]["savedAt"] = json!("2026-07-15T02:00:00+02:00");
        assert_eq!(
            parse_bookmark_state(&offset_saved_at)
                .expect("valid RFC3339 offset timestamp must round-trip")[0]
                .saved_at,
            "2026-07-15T02:00:00+02:00"
        );
    }

    #[test]
    fn bookmark_save_terminal_survives_workspace_rebind_without_state_contamination() {
        use crate::backend_client::FindInFilesDelivery;

        let bookmark_a = SearchBookmark {
            id: "a".into(),
            label: "A".into(),
            query: "workspace-a".into(),
            kind: KindFilter::Document,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-07-15T00:00:00Z".into(),
        };
        let bookmark_b = SearchBookmark {
            id: "b".into(),
            label: "B".into(),
            query: "workspace-b".into(),
            ..bookmark_a.clone()
        };
        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        let stamp = state.next_stamp("A", FindInFilesOperation::BookmarkSave);
        state.active_bookmark_save = Some(stamp.clone());
        state.refresh_loading();

        state.bind_workspace(Some("B"), 2);
        state.bookmarks = vec![bookmark_b.clone()];
        let load_b = state.next_stamp("B", FindInFilesOperation::BookmarkLoad);
        state.active_bookmark_load = Some(load_b.clone());
        assert_eq!(state.active_bookmark_save.as_ref(), Some(&stamp));
        assert!(state.loading);
        state
            .bookmark_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp,
                outcome: Ok((
                    bookmark_state_blob(std::slice::from_ref(&bookmark_a)),
                    Some("Saved search bookmark A".into()),
                    Some("evt-bookmark-A".into()),
                )),
            });
        state
            .bookmark_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: load_b,
                outcome: Ok((
                    bookmark_state_blob(std::slice::from_ref(&bookmark_b)),
                    None,
                    None,
                )),
            });

        assert!(state.poll());
        assert!(!state.bookmark_in_flight());
        assert_eq!(state.bookmarks, vec![bookmark_b]);
        assert_eq!(
            state.last_bookmark_save_terminal_workspace_id.as_deref(),
            Some("A")
        );
        assert_eq!(
            state.last_bookmark_save_receipt_id.as_deref(),
            Some("evt-bookmark-A")
        );
        let status = state.bookmark_status.as_deref().unwrap_or_default();
        assert!(status.contains("Workspace A"));
        assert!(status.contains("Saved search bookmark A"));
        assert!(status.contains("evt-bookmark-A"));
    }

    #[test]
    fn bookmark_save_a_b_a_rebind_reports_old_terminal_but_never_rehydrates_old_a_state() {
        use crate::backend_client::FindInFilesDelivery;

        let bookmark = |id: &str| SearchBookmark {
            id: id.into(),
            label: id.into(),
            query: id.into(),
            kind: KindFilter::All,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-07-15T00:00:00Z".into(),
        };
        let old_a = bookmark("old-a");
        let current_a = bookmark("current-a");
        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        let stamp = state.next_stamp("A", FindInFilesOperation::BookmarkSave);
        state.active_bookmark_save = Some(stamp.clone());
        state.bind_workspace(Some("B"), 2);
        state.bind_workspace(Some("A"), 3);
        state.bookmarks = vec![current_a.clone()];
        state
            .bookmark_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp,
                outcome: Ok((
                    bookmark_state_blob(std::slice::from_ref(&old_a)),
                    Some("Saved old A".into()),
                    Some("evt-old-A".into()),
                )),
            });

        assert!(state.poll());
        assert_eq!(state.bookmarks, vec![current_a]);
        assert_eq!(
            state.last_bookmark_save_terminal_workspace_id.as_deref(),
            Some("A")
        );
        assert_eq!(
            state.last_bookmark_save_receipt_id.as_deref(),
            Some("evt-old-A")
        );
        assert!(state
            .bookmark_status
            .as_deref()
            .unwrap_or_default()
            .contains("Saved old A"));
    }

    #[test]
    fn a_b_a_workspace_cycle_rejects_old_search_completion() {
        use crate::backend_client::FindInFilesDelivery;

        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        let stale = state.next_stamp("A", FindInFilesOperation::Search);
        state.active_search = Some(stale.clone());
        state.bind_workspace(Some("B"), 2);
        state.bind_workspace(Some("A"), 3);
        let fresh = state.next_stamp("A", FindInFilesOperation::Search);
        state.active_search = Some(fresh.clone());
        let old_hit = LoomGraphSearchHit {
            source_kind: "document".into(),
            result_kind: "document".into(),
            ref_id: "KRD-old".into(),
            title: "old".into(),
            excerpt: String::new(),
            metadata: json!({}),
            block: None,
        };
        let fresh_hit = LoomGraphSearchHit {
            ref_id: "KRD-fresh".into(),
            title: "fresh".into(),
            ..old_hit.clone()
        };
        state
            .search_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: stale,
                outcome: Ok((vec![old_hit], "old-key".into())),
            });
        state
            .search_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: fresh,
                outcome: Ok((vec![fresh_hit], "fresh-key".into())),
            });
        assert!(state.poll());
        assert_eq!(state.results[0].ref_id, "KRD-fresh");
        assert_eq!(state.result_set_key.as_deref(), Some("fresh-key"));
    }

    #[test]
    fn operation_completion_only_clears_its_own_in_flight_state() {
        use crate::backend_client::FindInFilesDelivery;

        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("ws"), 1);
        let search = state.next_stamp("ws", FindInFilesOperation::Search);
        let bookmark = state.next_stamp("ws", FindInFilesOperation::BookmarkLoad);
        state.active_search = Some(search.clone());
        state.active_bookmark_load = Some(bookmark);
        state.refresh_loading();
        state
            .search_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: search,
                outcome: Ok((Vec::new(), "key".into())),
            });
        state.poll();
        assert!(!state.search_in_flight());
        assert!(state.bookmark_in_flight());
        assert!(state.loading, "bookmark load keeps aggregate loading true");
    }

    #[test]
    fn hidden_a_b_a_generation_rebind_rejects_first_a_completion() {
        use crate::backend_client::FindInFilesDelivery;

        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        let stale = state.next_stamp("A", FindInFilesOperation::Search);
        state.active_search = Some(stale.clone());

        // The pane did not render during B. The shell generation still advances and the next render of
        // A must rebind even though the visible workspace id equals the last rendered id.
        state.bind_workspace(Some("A"), 3);
        let fresh = state.next_stamp("A", FindInFilesOperation::Search);
        state.active_search = Some(fresh.clone());
        state
            .search_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: stale,
                outcome: Ok((Vec::new(), "stale".to_owned())),
            });
        state
            .search_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: fresh,
                outcome: Ok((Vec::new(), "fresh".to_owned())),
            });

        assert!(state.poll());
        assert_eq!(state.result_set_key.as_deref(), Some("fresh"));
    }

    #[test]
    fn apply_delivery_requests_search_refresh_for_committed_mutations() {
        let mut state = FindInFilesPanelState::new();
        state.apply_replace_delivery(ReplaceDelivery::Applied {
            receipts: vec!["evt-1".to_owned()],
            audit_receipts: vec![ReplaceAuditReceipt {
                document_id: "KRD-1".to_owned(),
                before_sha256: "a".repeat(64),
                after_sha256: "b".repeat(64),
                outcome: ReplaceAuditOutcome::Saved,
                save_receipt_event_id: Some("evt-1".to_owned()),
                error: None,
            }],
            plan_count: 1,
        });
        assert!(state.refresh_search_after_apply);
    }

    #[test]
    fn apply_status_exposes_committed_without_receipt_and_receipt_error() {
        let mut state = FindInFilesPanelState::new();
        state.apply_replace_delivery(ReplaceDelivery::Applied {
            receipts: Vec::new(),
            audit_receipts: vec![ReplaceAuditReceipt {
                document_id: "KRD-1".to_owned(),
                before_sha256: "a".repeat(64),
                after_sha256: "b".repeat(64),
                outcome: ReplaceAuditOutcome::CommittedWithoutReceipt,
                save_receipt_event_id: None,
                error: Some("event ledger unavailable".to_owned()),
            }],
            plan_count: 1,
        });

        let status = state.replace_status.expect("Apply status");
        assert!(status.contains("CommittedWithoutReceipt"));
        assert!(status.contains("event ledger unavailable"));
        assert!(state.refresh_search_after_apply);
    }

    #[test]
    fn partial_apply_header_preserves_receipt_and_failure_together() {
        let mut state = FindInFilesPanelState::new();
        state.apply_replace_delivery(ReplaceDelivery::AppliedPartial {
            receipts: vec!["KE-saved-first".to_owned()],
            audit_receipts: vec![
                ReplaceAuditReceipt {
                    document_id: "KRD-1".to_owned(),
                    before_sha256: "a".repeat(64),
                    after_sha256: "b".repeat(64),
                    outcome: ReplaceAuditOutcome::Saved,
                    save_receipt_event_id: Some("KE-saved-first".to_owned()),
                    error: None,
                },
                ReplaceAuditReceipt {
                    document_id: "KRD-2".to_owned(),
                    before_sha256: "c".repeat(64),
                    after_sha256: "d".repeat(64),
                    outcome: ReplaceAuditOutcome::Conflict,
                    save_receipt_event_id: None,
                    error: Some("version conflict".to_owned()),
                },
            ],
            error: "Document KRD-2 changed since preview (version conflict)".to_owned(),
        });

        let visible = state.header_status();
        assert!(visible.contains("KE-saved-first"));
        assert!(visible.contains("KRD-2"));
        assert!(visible.contains("conflict"));
    }

    #[test]
    fn cancel_apply_retains_active_stamp_until_worker_reports_receipts() {
        let mut state = FindInFilesPanelState::new();
        let stamp = state.next_stamp("ws", FindInFilesOperation::Apply);
        let cancel = Arc::new(AtomicBool::new(false));
        state.active_apply = Some(stamp);
        state.active_apply_cancel = Some(Arc::clone(&cancel));
        state.refresh_loading();

        state.request_cancel();

        assert!(cancel.load(Ordering::Acquire));
        assert!(state.apply_in_flight());
        assert!(state.loading);
        assert!(state.header_status().contains("Working"));
    }

    #[test]
    fn workspace_rebind_retains_apply_until_workspace_attributed_terminal_receipt() {
        use crate::backend_client::FindInFilesDelivery;

        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        let stamp = state.next_stamp("A", FindInFilesOperation::Apply);
        let cancel = Arc::new(AtomicBool::new(false));
        state.active_apply = Some(stamp.clone());
        state.active_apply_cancel = Some(Arc::clone(&cancel));
        state.refresh_loading();

        state.bind_workspace(Some("B"), 2);
        assert!(cancel.load(Ordering::Acquire));
        assert_eq!(state.active_apply.as_ref(), Some(&stamp));
        assert!(state.loading);

        state
            .replace_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp,
                outcome: ReplaceDelivery::Applied {
                    receipts: vec!["evt-A-1".to_owned()],
                    audit_receipts: vec![ReplaceAuditReceipt {
                        document_id: "KRD-A-1".to_owned(),
                        before_sha256: "a".repeat(64),
                        after_sha256: "b".repeat(64),
                        outcome: ReplaceAuditOutcome::Saved,
                        save_receipt_event_id: Some("evt-A-1".to_owned()),
                        error: None,
                    }],
                    plan_count: 1,
                },
            });
        assert!(state.poll());
        assert!(!state.apply_in_flight());
        assert_eq!(state.last_apply_terminal_workspace_id.as_deref(), Some("A"));
        let status = state.replace_status.as_deref().unwrap_or_default();
        assert!(status.contains("Workspace A"));
        assert!(status.contains("evt-A-1"));
        assert_eq!(state.refresh_search_workspace_id.as_deref(), Some("A"));
    }

    #[test]
    fn in_flight_save_commits_after_rebind_and_real_worker_receipt_remains_visible() {
        use std::io::{Read, Write};
        use std::sync::mpsc;
        use std::time::Duration;

        fn read_request(stream: &mut std::net::TcpStream) {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 4096];
            let mut expected_len = None;
            loop {
                let read = stream.read(&mut buffer).expect("read mock HTTP request");
                if read == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..read]);
                if expected_len.is_none() {
                    if let Some(header_end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..header_end]);
                        let content_len = headers
                            .lines()
                            .find_map(|line| {
                                line.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|value| value.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        expected_len = Some(header_end + 4 + content_len);
                    }
                }
                if expected_len.is_some_and(|len| bytes.len() >= len) {
                    break;
                }
            }
        }

        fn respond_json(stream: &mut std::net::TcpStream, value: serde_json::Value) {
            let body = value.to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .expect("write mock HTTP response");
            stream.flush().unwrap();
        }

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (save_started_tx, save_started_rx) = mpsc::channel();
        let (release_save_tx, release_save_rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut load_stream, _) = listener.accept().expect("accept revalidation GET");
            read_request(&mut load_stream);
            respond_json(
                &mut load_stream,
                serde_json::json!({
                    "document": {
                        "rich_document_id":"KRD-A-1", "workspace_id":"A", "doc_version":1,
                        "title":"A", "content_json":{"type":"doc","content":[]},
                        "crdt_document_id":null, "authority_label":"canonical",
                        "owner_actor_kind":null, "owner_actor_id":null,
                        "project_ref":null, "folder_ref":null,
                        "created_at":"2026-07-15T00:00:00Z", "updated_at":"2026-07-15T00:00:00Z"
                    },
                    "tree":{"schema_version":"1","schema_matches":true,"block_ids":[],"blocks":[]},
                    "code_nodes":[]
                }),
            );
            let (mut save_stream, _) = listener.accept().expect("accept in-flight PUT");
            read_request(&mut save_stream);
            save_started_tx.send(()).unwrap();
            release_save_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("release in-flight save response");
            respond_json(
                &mut save_stream,
                serde_json::json!({"document":{},"save_receipt_event_id":"evt-A-real"}),
            );
        });

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let client = RichDocClient::new(base, runtime.handle().clone());
        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        state.query = "needle".to_owned();
        state.replacement = "replacement".to_owned();
        state.preview_plans = vec![ReplacementPlan {
            workspace_id: "A".to_owned(),
            document_id: "KRD-A-1".to_owned(),
            title: "A".to_owned(),
            expected_version: 1,
            content_json_after: serde_json::json!({"type":"doc","content":[]}),
            before_sha256: "a".repeat(64),
            after_sha256: "b".repeat(64),
            crdt_document_id: None,
            match_count: 1,
            before_preview: "needle".to_owned(),
            after_preview: "replacement".to_owned(),
            match_previews: Vec::new(),
        }];
        state.preview_plan_key = Some(state.current_replace_key());
        state.run_apply(&client, Some("A"));
        save_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("save reached in-flight PUT");
        state.bind_workspace(Some("B"), 2);
        assert!(state.apply_in_flight());
        let retained_apply_stamp = state.active_apply.clone();
        state.preview_plans = vec![ReplacementPlan {
            workspace_id: "B".to_owned(),
            document_id: "KRD-B-1".to_owned(),
            title: "B".to_owned(),
            expected_version: 1,
            content_json_after: serde_json::json!({}),
            before_sha256: "c".repeat(64),
            after_sha256: "d".repeat(64),
            crdt_document_id: None,
            match_count: 1,
            before_preview: String::new(),
            after_preview: String::new(),
            match_previews: Vec::new(),
        }];
        state.preview_plan_key = Some(state.current_replace_key());
        state.run_apply(&client, Some("B"));
        assert_eq!(
            state.active_apply, retained_apply_stamp,
            "new destructive Apply stays blocked until old workspace worker terminates"
        );
        release_save_tx.send(()).unwrap();
        for _ in 0..100 {
            state.poll();
            if !state.apply_in_flight() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        server.join().unwrap();
        assert!(!state.apply_in_flight());
        assert_eq!(state.last_apply_terminal_workspace_id.as_deref(), Some("A"));
        let status = state.replace_status.as_deref().unwrap_or_default();
        assert!(status.contains("Workspace A"));
        assert!(status.contains("evt-A-real"));
    }

    #[test]
    fn search_never_overlaps_apply_or_preview_completion_reordering() {
        use crate::backend_client::FindInFilesDelivery;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = WorkspaceSearchClient::new("http://127.0.0.1:9", runtime.handle().clone());
        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        state.query = "same-input".to_owned();

        let apply_stamp = state.next_stamp("A", FindInFilesOperation::Apply);
        state.active_apply = Some(apply_stamp);
        assert!(!state.run_search(&client, Some("A")));
        assert!(state.active_search.is_none());
        assert!(state
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("blocked"));
        state.active_apply = None;

        let preview_stamp = state.next_stamp("A", FindInFilesOperation::Preview);
        state.active_preview = Some(preview_stamp.clone());
        assert!(!state.run_search(&client, Some("A")));
        assert!(
            state.active_preview.is_none(),
            "new Search detaches Preview first"
        );
        assert!(
            state.active_search.is_none(),
            "the same click does not overlap Preview"
        );
        state
            .replace_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: preview_stamp,
                outcome: ReplaceDelivery::Preview {
                    plans: Vec::new(),
                    key: "same-input".to_owned(),
                },
            });
        assert!(
            !state.poll(),
            "detached Preview delivery cannot reorder after Search intent"
        );
        assert!(state.preview_plan_key.is_none());
        assert!(
            state.run_search(&client, Some("A")),
            "a second Search starts after detachment"
        );
    }

    #[test]
    fn automatic_refresh_flag_survives_until_search_actually_starts() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = WorkspaceSearchClient::new("http://127.0.0.1:9", runtime.handle().clone());
        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        state.query = "refresh-me".to_owned();
        state.refresh_search_after_apply = true;
        state.refresh_search_workspace_id = Some("A".to_owned());
        let apply_stamp = state.next_stamp("A", FindInFilesOperation::Apply);
        state.active_apply = Some(apply_stamp);

        state.poll_with_search_refresh(&client, Some("A"));
        assert!(state.refresh_search_after_apply);
        assert!(state.active_search.is_none());

        state.active_apply = None;
        assert!(state.poll_with_search_refresh(&client, Some("A")));
        assert!(!state.refresh_search_after_apply);
        assert!(state.active_search.is_some());
    }

    #[test]
    fn result_author_id_is_injective_and_reversible() {
        let slash = result_author_id("loom_block", "blk/1:x");
        let dash = result_author_id("loom_block", "blk-1-x");
        assert_ne!(slash, dash);
        assert_eq!(
            hit_identity_from_result_author_id(&slash),
            Some(("loom_block".to_owned(), "blk/1:x".to_owned()))
        );
        let utf8 = result_author_id("文档", "résumé/東京");
        assert_eq!(
            hit_identity_from_result_author_id(&utf8),
            Some(("文档".to_owned(), "résumé/東京".to_owned()))
        );
        assert_eq!(
            result_author_id("document", "KRD-1:/foo?x=1"),
            "find-in-files.result.646f63756d656e74.4b52442d313a2f666f6f3f783d31"
        );
        assert_eq!(
            utf8,
            "find-in-files.result.e69687e6a1a3.72c3a973756dc3a92fe69db1e4baac"
        );
        assert_eq!(
            preview_author_id("KRD-文/1"),
            "find-in-files.preview.4b52442de696872f31"
        );
        assert_eq!(
            bookmark_restore_author_id("saved:文/1"),
            "find-in-files.bookmark-restore.73617665643ae696872f31"
        );
        assert_eq!(
            bookmark_remove_author_id("saved:文/1"),
            "find-in-files.bookmark-remove.73617665643ae696872f31"
        );
    }

    #[test]
    fn kind_filter_all_omits_source_kind() {
        assert_eq!(KindFilter::All.source_kind(), None);
        assert_eq!(KindFilter::Document.source_kind(), Some("document"));
    }

    #[test]
    fn restore_bookmark_repopulates_fields() {
        let mut s = FindInFilesPanelState::new();
        let bm = SearchBookmark {
            id: "x".into(),
            label: "x".into(),
            query: "needle".into(),
            kind: KindFilter::WikiPage,
            tag_filter: "tag-1".into(),
            path_filter: "src/app".into(),
            case_sensitive: true,
            whole_word: true,
            is_regex: true,
            saved_at: "2026-06-23T00:00:00Z".into(),
        };
        s.restore_bookmark(&bm);
        assert_eq!(s.query, "needle");
        assert_eq!(s.kind, KindFilter::WikiPage);
        assert_eq!(s.tag_filter, "tag-1");
        assert_eq!(s.path_filter, "src/app");
        assert!(s.case_sensitive && s.whole_word && s.is_regex);
    }

    #[test]
    fn restore_bookmark_centrally_invalidates_results_cache_and_pending_reads() {
        use crate::backend_client::FindInFilesDelivery;

        let mut state = FindInFilesPanelState::new();
        state.bind_workspace(Some("A"), 1);
        state.query = "old".to_owned();
        state.results = vec![LoomGraphSearchHit {
            source_kind: "document".into(),
            result_kind: "knowledge_entity".into(),
            ref_id: "KRD-old".into(),
            title: "old".into(),
            excerpt: "old".into(),
            metadata: json!({}),
            block: None,
        }];
        state.result_set_key = Some(state.current_search_key());
        assert_eq!(state.visible_result_count(), 1, "populate visible cache");
        let generation_before = state.results_generation;
        let search_stamp = state.next_stamp("A", FindInFilesOperation::Search);
        let preview_stamp = state.next_stamp("A", FindInFilesOperation::Preview);
        state.active_search = Some(search_stamp.clone());
        state.active_preview = Some(preview_stamp.clone());
        state
            .search_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: search_stamp,
                outcome: Ok((Vec::new(), "old-key".to_owned())),
            });
        state
            .replace_cell
            .lock()
            .unwrap()
            .push_back(FindInFilesDelivery {
                stamp: preview_stamp,
                outcome: ReplaceDelivery::Preview {
                    plans: Vec::new(),
                    key: "old-preview".to_owned(),
                },
            });

        let bookmark = SearchBookmark {
            id: "new".into(),
            label: "new".into(),
            query: "new".into(),
            kind: KindFilter::Document,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-07-15T00:00:00Z".into(),
        };
        state.restore_bookmark(&bookmark);
        // MT-029 AC-7 (stale-result guard). `invalidate_search_inputs` DELIBERATELY RETAINS the last
        // completed result set and its producer key: clearing either one disables Preview Replace, and a
        // disabled button can never surface the contract's "Search results are stale" warning. Asserting
        // that they are cleared would therefore lock in a violation of a validated acceptance criterion.
        // What `restore_bookmark` MUST invalidate is the PENDING READS and the DERIVED caches — asserted
        // below, followed by a live proof that this retention is what keeps AC-7 reachable.
        assert_eq!(
            state.results.len(),
            1,
            "MT-029 AC-7: the completed result set is retained so Preview stays enabled to report staleness"
        );
        let current_key = state.current_search_key();
        assert!(
            state.result_set_key.is_some(),
            "MT-029 AC-7: the producer key is retained so the stale comparison has something to compare"
        );
        assert_ne!(
            state.result_set_key.as_deref(),
            Some(current_key.as_str()),
            "the retained key must now read STALE against the restored bookmark's search params"
        );
        assert_eq!(
            state.visible_result_count(),
            1,
            "the visible cache is invalidated and recomputed, not emptied — no match-option is active, \
             so every retained hit passes the client-side filter"
        );
        assert!(state.results_generation > generation_before);
        assert!(state.active_search.is_none() && state.active_preview.is_none());
        assert!(state.search_cell.lock().unwrap().is_empty());
        assert!(state.replace_cell.lock().unwrap().is_empty());
        assert!(
            !state.poll(),
            "old completions cannot be accepted after restore"
        );

        // AC-7 REACHABILITY, proven through the REAL path rather than a hand-built stale state (which is
        // what `preview_stale_result_guard` already covers): the retention asserted above is precisely
        // what lets the guard fire after a bookmark restore. If a future change clears `results` or
        // `result_set_key` in `invalidate_search_inputs`, this assertion goes red with the reason.
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let client = RichDocClient::new("http://test.local", rt.handle().clone());
        state.run_preview_replace(&client, Some("A"));
        assert!(
            state
                .replace_status
                .as_deref()
                .unwrap_or_default()
                .contains("stale"),
            "MT-029 AC-7: after restoring a bookmark the stale-result warning must fire, got {:?}",
            state.replace_status
        );
        assert!(
            state.preview_plans.is_empty(),
            "MT-029 AC-7: a stale preview computes NOTHING"
        );
    }

    // ══════════════════════════════════════════════════════════════════════════════════════════════
    // MT-113 — bounded author_id composition (AC-113-1/2/4/5, PT-113-1, PT-113-2)
    // ══════════════════════════════════════════════════════════════════════════════════════════════

    /// The exact pre-MT-113 composition, reproduced verbatim so byte-identity is proven against the
    /// REAL legacy algorithm rather than against a remembered description of it.
    fn legacy_verbatim_route(prefix: &str, components: &[&str], pane: Option<&str>) -> String {
        let mut composed = String::from(prefix);
        for (index, component) in components.iter().enumerate() {
            if index > 0 {
                composed.push('.');
            }
            composed.push_str(&encode_author_id_component(component));
        }
        if let Some(pane) = pane {
            composed.push_str(".pane-");
            composed.push_str(&encode_author_id_component(pane));
        }
        composed
    }

    fn bookmark_with_query(query: &str) -> SearchBookmark {
        let mut bookmark = SearchBookmark {
            id: String::new(),
            label: String::new(),
            query: query.to_owned(),
            kind: KindFilter::All,
            tag_filter: String::new(),
            path_filter: String::new(),
            case_sensitive: false,
            whole_word: false,
            is_regex: false,
            saved_at: "2026-08-05T00:00:00Z".to_owned(),
        };
        bookmark.id = bookmark.stable_id();
        bookmark.label = bookmark.display_label();
        bookmark
    }

    /// Deterministic adversarial corpus for PT-113-1. Every entry is built by appending WHOLE `&str`
    /// atoms, so a generated input can never split a codepoint: multibyte scalars, 4-byte astral
    /// scalars and multi-codepoint grapheme clusters are always appended intact.
    fn adversarial_author_id_inputs() -> Vec<String> {
        let atoms: [&str; 16] = [
            "a",
            "authentication middleware refactor",
            "r\u{e9}sum\u{e9}",
            "\u{6587}\u{6863}",
            "\u{1F469}\u{200D}\u{1F4BB}", // ZWJ grapheme cluster of 4-byte scalars
            "\u{1F1F3}\u{1F1F1}",         // regional-indicator pair: one grapheme, two scalars
            "e\u{0301}\u{0323}",          // base + two combining marks: one grapheme cluster
            "\u{10348}",                  // 4-byte astral scalar
            "\u{202E}rtl-override",       // bidi control
            ".",                          // the route separator itself
            "..",
            "pane-",
            "zsha256-deadbeef", // an input that MIMICS the digest sentinel
            "0123456789abcdef", // an input shaped exactly like verbatim hex output
            " leading and trailing ",
            "/path/with/slashes:and:colons?and=query",
        ];
        let mut generated = Vec::new();
        let mut state: u64 = 0x5DEE_CE66_D113_0001;
        for target_len in [
            0usize, 1, 11, 21, 22, 23, 24, 40, 64, 100, 128, 256, 512, 1024, 4096,
        ] {
            for variant in 0..4u32 {
                let mut value = String::new();
                while value.len() < target_len {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407)
                        ^ u64::from(variant);
                    let atom = atoms[(state >> 33) as usize % atoms.len()];
                    value.push_str(atom);
                }
                generated.push(value);
            }
        }
        generated.extend(atoms.iter().map(|atom| (*atom).to_owned()));
        generated.push(String::new());
        generated.push("a".repeat(22));
        generated.push("a".repeat(23));
        generated.push("\u{1F469}\u{200D}\u{1F4BB}".repeat(200));
        generated
    }

    /// PT-113-1 / AC-113-1. Over generated long, multibyte and adversarial inputs (grapheme clusters
    /// and 4-byte codepoints included, never split): every composed route — scoped and unscoped —
    /// stays inside the canonical completion-target budget and its completion token ALWAYS
    /// serializes. The bound holds for the content, for the pane scope, and for both together.
    #[test]
    fn mt113_pt1_composed_author_ids_stay_in_budget_and_always_serialize() {
        let long_pane = "\u{1F469}\u{200D}\u{1F4BB}".repeat(64);
        let panes: [Option<&str>; 4] = [
            None,
            Some("pane-b"),
            Some("6f0f2b0e-1c2d-4f3a-9b8c-7d6e5f4a3b2c"),
            Some(long_pane.as_str()),
        ];
        let mut checked = 0usize;
        for value in adversarial_author_id_inputs() {
            let bookmark = bookmark_with_query(&value);
            assert!(
                bookmark.id.len() <= MAX_BOOKMARK_STABLE_ID_BYTES,
                "bookmark stable_id is {} bytes for a {}-byte query",
                bookmark.id.len(),
                value.len()
            );
            let semantic = FindInFilesPanelState::bookmark_remove_semantic_value(&bookmark.id);
            assert!(
                semantic.len() <= MAX_COMPLETION_SEMANTIC_BYTES,
                "bookmark remove semantic_value is {} bytes",
                semantic.len()
            );
            for pane in panes {
                let routes = [
                    (
                        "bookmark-remove",
                        pane_scoped_bookmark_remove_author_id(&bookmark.id, pane),
                    ),
                    (
                        "bookmark-restore",
                        pane_scoped_bookmark_restore_author_id(&bookmark.id, pane),
                    ),
                    ("result", pane_scoped_result_author_id("file", &value, pane)),
                    ("preview", pane_scoped_preview_author_id(&value, pane)),
                    (
                        "preview-before",
                        pane_scoped_preview_before_author_id(&value, pane),
                    ),
                    (
                        "preview-after",
                        pane_scoped_preview_after_author_id(&value, pane),
                    ),
                ];
                for (label, route) in routes {
                    assert!(
                        route.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES,
                        "{label} route is {} bytes for a {}-byte input (pane {pane:?}); the canonical budget is {MAX_COMPLETION_TARGET_AUTHOR_BYTES}",
                        route.len(),
                        value.len()
                    );
                    assert!(
                        !route.chars().any(char::is_control),
                        "{label} route must stay control-character free"
                    );
                    // The exact call that returned `None` before MT-113.
                    assert!(
                        crate::mcp::action::serialize_observer_click_state(
                            BOOKMARK_COMPLETION_EFFECT,
                            "find-in-files.bookmark:ws-1",
                            7,
                            crate::mcp::action::ClickCompletionState::Pending,
                            Some(&route),
                            Some(&semantic),
                        )
                        .is_some(),
                        "{label} route ({} bytes) did not serialize a Pending observer token",
                        route.len()
                    );
                    // The SAME id must also be legal in the wider `context` field, so a synchronous
                    // control and its asynchronous sibling can never diverge (AC-113-4).
                    assert!(
                        crate::mcp::action::serialize_same_target_click_completion(
                            BOOKMARK_RESTORE_COMPLETION_EFFECT,
                            &route,
                            1,
                            crate::mcp::action::ClickCompletionState::Applied,
                        )
                        .is_some(),
                        "{label} route is legal as pending_target but not as context"
                    );
                    checked += 1;
                }
            }
        }
        assert!(
            checked > 1_000,
            "the property corpus must be substantial: {checked}"
        );
    }

    /// AC-113-1 regression pin: the exact inputs MT-029 measured. 22 characters already fitted, 23
    /// characters overran at 260 bytes, and the 34-character operator example composed a 304-byte
    /// route whose token was silently dropped.
    #[test]
    fn mt113_measured_regression_inputs_now_compose_in_budget() {
        for query in [
            "a".repeat(22),
            "a".repeat(23),
            "authentication middleware refactor".to_owned(),
        ] {
            let bookmark = bookmark_with_query(&query);
            let remove = bookmark_remove_author_id(&bookmark.id);
            let restore = bookmark_restore_author_id(&bookmark.id);
            assert!(
                remove.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES
                    && restore.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES,
                "query {:?} composes remove={} restore={} bytes",
                query,
                remove.len(),
                restore.len()
            );
            let semantic = FindInFilesPanelState::bookmark_remove_semantic_value(&bookmark.id);
            assert!(
                crate::mcp::action::serialize_observer_click_state(
                    BOOKMARK_COMPLETION_EFFECT,
                    "find-in-files.bookmark:ws-1",
                    1,
                    crate::mcp::action::ClickCompletionState::Pending,
                    Some(&remove),
                    Some(&semantic),
                )
                .is_some(),
                "query {query:?} must publish a Pending observer token"
            );
        }
    }

    /// AC-113-2 and the hard backward-compatibility constraint: EVERY route that fits today is
    /// BYTE-IDENTICAL after MT-113. Proven against the legacy algorithm itself, over the same
    /// adversarial corpus, for every composition site and every pane scope.
    #[test]
    fn mt113_every_route_that_fits_is_byte_identical_to_the_legacy_composition() {
        let panes: [Option<&str>; 3] = [
            None,
            Some("pane-b"),
            Some("6f0f2b0e-1c2d-4f3a-9b8c-7d6e5f4a3b2c"),
        ];
        let mut identical = 0usize;
        let mut bounded = 0usize;
        for value in adversarial_author_id_inputs() {
            for pane in panes {
                let cases = [
                    (
                        BOOKMARK_REMOVE_AUTHOR_ID_PREFIX,
                        vec![value.as_str()],
                        pane_scoped_bookmark_remove_author_id(&value, pane),
                    ),
                    (
                        BOOKMARK_RESTORE_AUTHOR_ID_PREFIX,
                        vec![value.as_str()],
                        pane_scoped_bookmark_restore_author_id(&value, pane),
                    ),
                    (
                        PREVIEW_AUTHOR_ID_PREFIX,
                        vec![value.as_str()],
                        pane_scoped_preview_author_id(&value, pane),
                    ),
                    (
                        PREVIEW_BEFORE_AUTHOR_ID_PREFIX,
                        vec![value.as_str()],
                        pane_scoped_preview_before_author_id(&value, pane),
                    ),
                    (
                        PREVIEW_AFTER_AUTHOR_ID_PREFIX,
                        vec![value.as_str()],
                        pane_scoped_preview_after_author_id(&value, pane),
                    ),
                    (
                        RESULT_AUTHOR_ID_PREFIX,
                        vec!["file", value.as_str()],
                        pane_scoped_result_author_id("file", &value, pane),
                    ),
                ];
                for (prefix, components, actual) in cases {
                    let legacy = legacy_verbatim_route(prefix, &components, pane);
                    if legacy.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES {
                        assert_eq!(
                            actual, legacy,
                            "a route that already fits MUST be byte-identical after MT-113"
                        );
                        identical += 1;
                    } else {
                        assert_ne!(
                            actual, legacy,
                            "an over-budget route must have been bounded"
                        );
                        assert!(actual.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES);
                        bounded += 1;
                    }
                }
            }
        }
        assert!(
            identical > 100,
            "byte-identity must be exercised broadly: {identical}"
        );
        assert!(
            bounded > 100,
            "bounding must be exercised broadly: {bounded}"
        );
    }

    /// AC-113-2: `SearchBookmark::stable_id` is likewise byte-identical wherever it already fits, so
    /// persisted bookmark identity and its dedup semantics are unchanged for real saved searches.
    #[test]
    fn mt113_bookmark_stable_id_is_byte_identical_where_it_fits_and_still_dedups() {
        let legacy_stable_id = |bookmark: &SearchBookmark| {
            let components = [
                bookmark.query.trim(),
                bookmark.kind.wire(),
                bookmark.tag_filter.trim(),
                bookmark.path_filter.trim(),
                if bookmark.case_sensitive {
                    "true"
                } else {
                    "false"
                },
                if bookmark.whole_word { "true" } else { "false" },
                if bookmark.is_regex { "true" } else { "false" },
            ];
            let mut stable = String::from("bookmark-v1");
            for component in components {
                use std::fmt::Write as _;
                let _ = write!(stable, ".{}-", component.len());
                stable.push_str(&encode_author_id_component(component));
            }
            stable
        };
        let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        for value in adversarial_author_id_inputs() {
            let bookmark = bookmark_with_query(&value);
            let legacy = legacy_stable_id(&bookmark);
            if legacy.len() <= MAX_BOOKMARK_STABLE_ID_BYTES {
                assert_eq!(bookmark.id, legacy, "in-budget stable_id must not change");
            }
            assert!(bookmark.id.len() <= MAX_BOOKMARK_STABLE_ID_BYTES);
            // Dedup is still an exact function of the semantic tuple.
            assert_eq!(bookmark.id, bookmark_with_query(&value).id);
            if let Some(previous) = seen.insert(bookmark.id.clone(), value.clone()) {
                assert_eq!(
                    previous, value,
                    "two distinct saved searches collided on one id"
                );
            }
        }
    }

    /// AC-113-1 plus the injectivity requirement: distinct content NEVER collapses onto one route,
    /// across the verbatim regime, the digested regime, and the boundary between them — including
    /// inputs deliberately shaped like verbatim hex and like the digest sentinel.
    #[test]
    fn mt113_bounded_routes_stay_injective_across_both_regimes() {
        let panes: [Option<&str>; 3] = [None, Some("pane-b"), Some("pane-c")];
        let mut seen: std::collections::HashMap<String, (String, Option<&str>)> =
            std::collections::HashMap::new();
        for value in adversarial_author_id_inputs() {
            for pane in panes {
                let route = pane_scoped_bookmark_remove_author_id(&value, pane);
                if let Some((previous, previous_pane)) =
                    seen.insert(route.clone(), (value.clone(), pane))
                {
                    assert!(
                        previous == value && previous_pane == pane,
                        "route collision: {previous:?}/{previous_pane:?} and {value:?}/{pane:?} both compose {route}"
                    );
                }
            }
        }
    }

    /// The resolvability requirement: a BOUNDED route is still resolvable back to the exact target it
    /// addresses. String-only reversal stays exact in the verbatim regime (MT-029's contract); the
    /// digested regime resolves by recomputation against the live candidate set, which is exactly how
    /// the production panel matches an activated row.
    #[test]
    fn mt113_bounded_result_routes_remain_resolvable_to_their_exact_identity() {
        let long_path = format!("/{}/deep/nested/path.md", "segment".repeat(40));
        let candidates: Vec<(&str, &str)> = vec![
            ("loom_block", "blk/1:x"),
            ("document", "KRD-1:/foo?x=1"),
            ("\u{6587}\u{6863}", "r\u{e9}sum\u{e9}/\u{6771}\u{4eac}"),
            ("file", long_path.as_str()),
            ("file", "/short.md"),
        ];
        for pane in [None, Some("6f0f2b0e-1c2d-4f3a-9b8c-7d6e5f4a3b2c")] {
            for (source_kind, ref_id) in candidates.iter().copied() {
                let route = pane_scoped_result_author_id(source_kind, ref_id, pane);
                assert!(route.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES);
                assert_eq!(
                    hit_identity_from_result_author_id_in(&route, candidates.iter().copied(), pane),
                    Some((source_kind.to_owned(), ref_id.to_owned())),
                    "every bounded route must resolve back to its exact backend identity"
                );
            }
        }
        // The long path is the case that is NOT string-reversible; it is still exactly resolvable.
        let digested = result_author_id("file", &long_path);
        assert!(
            digested.contains(AUTHOR_ID_DIGEST_SENTINEL),
            "the over-budget path must have taken the digested regime"
        );
        assert_eq!(
            hit_identity_from_result_author_id(&digested),
            None,
            "a digested route is honestly NOT string-reversible"
        );
        assert_eq!(
            hit_identity_from_result_author_id_in(&digested, candidates.iter().copied(), None),
            Some(("file".to_owned(), long_path.clone()))
        );
    }

    /// AC-113-5 (headless half): the production panel, rendered with a bookmark saved from a REALISTIC
    /// long query, publishes a Remove row whose completion observer carries a real token naming that
    /// exact route. This is the precise state that produced a permanently `indeterminate` receipt
    /// before MT-113 — the observer node carried no value at all.
    #[test]
    fn mt113_long_query_bookmark_remove_publishes_an_observable_completion_token() {
        use egui_kittest::kittest::NodeT as _;
        use std::sync::{Arc, Mutex};

        let query = "authentication middleware refactor across the workspace";
        let bookmark = bookmark_with_query(query);
        let remove_route = bookmark_remove_author_id(&bookmark.id);
        assert!(
            legacy_verbatim_route(BOOKMARK_REMOVE_AUTHOR_ID_PREFIX, &[&bookmark.id], None).len()
                > MAX_COMPLETION_TARGET_AUTHOR_BYTES,
            "this proof is only meaningful for a query whose LEGACY route overran the budget"
        );
        let mut panel = FindInFilesPanelState::new();
        panel.bind_workspace(Some("ws-mt113"), 0);
        panel.bookmarks = vec![bookmark.clone()];
        let panel = Arc::new(Mutex::new(panel));
        let render_state = Arc::clone(&panel);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build current-thread runtime");
        let search_client = crate::backend_client::WorkspaceSearchClient::new(
            "http://127.0.0.1:1",
            runtime.handle().clone(),
        );
        let doc_client = crate::backend_client::RichDocClient::new(
            "http://127.0.0.1:1",
            runtime.handle().clone(),
        );
        let mut harness = egui_kittest::Harness::builder()
            .with_size(egui::vec2(900.0, 760.0))
            .build_ui(move |ui| {
                let palette = crate::theme::HsTheme::Dark.palette();
                let mut on_open = |_hit: &crate::backend_client::LoomGraphSearchHit| {};
                let mut callbacks = FindInFilesCallbacks {
                    on_open_hit: &mut on_open,
                };
                show(
                    ui,
                    &mut render_state.lock().unwrap(),
                    &palette,
                    &search_client,
                    &doc_client,
                    Some("ws-mt113"),
                    &mut callbacks,
                );
            });
        // The panel keeps requesting repaints while it settles, so step a bounded number of
        // frames instead of running to quiescence.
        harness.run_steps(8);

        fn node_value(harness: &egui_kittest::Harness<'_, ()>, author_id: &str) -> Option<String> {
            use egui_kittest::kittest::NodeT as _;
            harness
                .root()
                .children_recursive()
                .find(|node| node.accesskit_node().author_id() == Some(author_id))
                .and_then(|node| node.accesskit_node().value())
        }

        let observer_before = node_value(&harness, BOOKMARK_COMPLETION_AUTHOR_ID)
            .expect("the bookmark completion observer must publish a Ready token");
        assert!(
            observer_before.contains("handshake.click-completion/v1"),
            "observer must carry the canonical schema, got {observer_before}"
        );

        harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(remove_route.as_str()))
            .expect("the long-query bookmark must publish an addressable Remove row")
            .click_accesskit();
        // The panel keeps requesting repaints while it settles, so step a bounded number of
        // frames instead of running to quiescence.
        harness.run_steps(8);

        let observer_after = node_value(&harness, BOOKMARK_COMPLETION_AUTHOR_ID)
            .expect("MT-113: the observer MUST still carry a token after a long-query Remove");
        assert!(
            observer_after.contains(&remove_route),
            "the observer token must name the exact clicked route {remove_route}; got {observer_after}"
        );
        assert!(
            !observer_after.contains("click-completion-unavailable"),
            "a bounded route must never publish the completion-unavailable marker: {observer_after}"
        );
    }

    /// PT-113-2 pin, restated inside the lib so a future edit to composition trips here too: the
    /// MT-029 exact-reversibility contract and its literal route strings are unchanged.
    #[test]
    fn mt113_mt029_exact_reversibility_contract_is_unchanged() {
        for (kind, reference) in [
            ("loom_block", "blk/1:x"),
            ("loom_block", "blk-1-x"),
            ("\u{6587}\u{6863}", "r\u{e9}sum\u{e9}/\u{6771}\u{4eac}"),
            ("document", "KRD-1:/foo?x=1"),
        ] {
            let route = result_author_id(kind, reference);
            assert_eq!(
                hit_identity_from_result_author_id(&route),
                Some((kind.to_owned(), reference.to_owned()))
            );
        }
        assert_eq!(
            bookmark_restore_author_id("saved:\u{6587}/1"),
            "find-in-files.bookmark-restore.73617665643ae696872f31"
        );
        assert_eq!(
            bookmark_remove_author_id("saved:\u{6587}/1"),
            "find-in-files.bookmark-remove.73617665643ae696872f31"
        );
    }

    /// Baseline reproduction 1, now a permanent regression pin. FAILED at bce12043 with a 304-byte
    /// route whose Pending observer token did not serialize.
    #[test]
    fn mt113_baseline_long_query_bookmark_remove_completion_token_is_silently_dropped() {
        let bookmark = bookmark_with_query("authentication middleware refactor");
        let remove_route = bookmark_remove_author_id(&bookmark.id);
        let semantic = FindInFilesPanelState::bookmark_remove_semantic_value(&bookmark.id);
        assert!(
            crate::mcp::action::serialize_observer_click_state(
                BOOKMARK_COMPLETION_EFFECT,
                "find-in-files.bookmark:ws-1",
                1,
                crate::mcp::action::ClickCompletionState::Pending,
                Some(&remove_route),
                Some(&semantic),
            )
            .is_some(),
            "MT-113: a {}-character query composed a {}-byte Remove route; its Pending observer token did NOT serialize",
            bookmark.query.chars().count(),
            remove_route.len(),
        );
    }

    /// Baseline reproduction 2, now a permanent regression pin. FAILED at bce12043: a 23-character
    /// query composed a 260-byte route.
    #[test]
    fn mt113_baseline_twenty_three_character_query_is_the_first_failing_input() {
        for length in [22usize, 23, 24, 64, 512] {
            let bookmark = bookmark_with_query(&"a".repeat(length));
            let route = bookmark_remove_author_id(&bookmark.id);
            assert!(
                route.len() <= MAX_COMPLETION_TARGET_AUTHOR_BYTES,
                "a {length}-character query composes a {}-byte route",
                route.len()
            );
        }
    }
}
