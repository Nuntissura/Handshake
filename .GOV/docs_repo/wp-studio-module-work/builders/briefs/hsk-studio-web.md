---
file_id: BUILDER-BRIEF-HSK-STUDIO-WEB
file_kind: builder_brief
updated_at: 2026-10-11T07:40:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-web
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-WEB` / key `web` / crate `hsk-studio-web`; foundation MT-35558 (test target `source_dom_patch`; cases `dom_edit_changes_exact_source_span`, `code_edit_refresh_same_revision`, `breakpoint_keyboard_a11y_preview`, `external_asset_denial_cancel_stale`; deadline 45 s); write scope `handshake-studio-assets/crates/hsk-studio-web/**`. Files per MT: `src/source_dom_patch.rs`, `src/responsive.rs`, `tests/source_dom_patch.rs`.
- Responsibility (verbatim): "Visual/code/DOM/template/responsive workflow over one source revision and shared editor/preview mechanics. No separate app/server/job or persistence authority."
- Foundation capability: "Real source-to-DOM and DOM-to-source edit/preview with conflict and granted asset resolution."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-observe`. Slice uses accord + observe only.
- Forbidden: handshake_core/native/SurrealDB; network, server runtime, publishing/upload (STU-WEB-002/006/127: out of scope here), executing scripts, fetching assets, a web-only shell (shared Controls/preview ports), a second persistence authority.
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `web-consumer` (`--descriptor`), `bounded_case` (45 s). Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`, web domain from l.30093)
- STU-WEB-003 (l.30097): a web document is a `StudioDocument` with `mode = web`; authority = ordered SOURCE TEXT plus a parsed element tree that is only a projection; EVERY structural edit MUST be expressible as a text edit; an unmodelled document still round-trips byte-for-byte. STU-WEB-004: on every save preserve original line endings, indentation char/width, attribute quoting style, tag-name case and any construct not parsed; reformat only on an explicit format command (STU-WEB-028). STU-WEB-020 (l.30209): formatting-rule record (11 keys); protected contents `pre/script/style/textarea` stay verbatim. STU-WEB-002/006: no server/DB, local-first, no network except publish. STU-WEB-043: control descriptors with stable author IDs and typed setters via shared command/Argus ports; STU-WEB-061/069/071/078 CSS panel and media-feature vocabulary (23 media features) for responsive.
- MT-35558 acceptance (via prep): "Source patch and revision-bound DOM/preview, unmappable/loss/conflict diagnostics; no new server/job/storage authority"; oracle "Independent parsed DOM/source re-read and actual preview/accessibility tree; publish proof needs separately authorized exact target".
- STU-ARC-011 (revision footprints/conflicts), STU-ARC-008/012/013 (bounded parse, cancel between chunks), STU-ARC-015 (preview receipts bind source revision), STU-ARC-016.
- Later scope: STU-WEB-009..145 (458 registry rows; tag libraries, snippets, templates, CSS designer, code intelligence, linting, site/publish), `_mt_plan_14_automation_whiteboard_xcut.json` (web MTs), MT-34218 family (181 primary owners per visual-team inventory). Web-team prep items needing pinned payloads before parity: STU-WEB-098/100/101/103/104/106/109/110/116/123/124/132/143 (see web-team `open_source_decisions`) - NOT slice scope.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, ~800-1100 LOC)
PREP DECISION (visual-team `preparation.json#/representative_proposals[9]`, MT-35558): provider "NOT_INSPECTED; parser/formatter/preview provider pin selection is blocking, no fake provider choice". Therefore this slice picks NO external provider: it implements an in-crate, span-preserving scanner for a DOCUMENTED SUBSET of HTML (what the text-is-authority rule needs) and refuses everything else with typed errors. The provider decision (full HTML5 tree building, CSS parsing, linting) goes to the lead (candidates in deps).
Prep refs: web-team `preparation.json` (`source_plans` STU-WEB-020/043/061/078, `open_source_decisions`, `fixed_freeze`); `.GOV/reference/studio_app_feature_research/19-studio-local-first-rust-posture.md` (Figma Sites/web = compatibility/optional; core local), `21-figma-feature-map.md`; Craft has no web crate to reuse (designcraft/photocraft/lightcraft/vectorcraft/pdfcraft - none).
Reuse (verified): `accord::{CancellationToken, DomainId}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. Folio is allowed but a web document's `mode` field does not exist in `folio::StudioDocument` today (grep: no `mode`) - SPEC/CODE GAP to flag; the slice takes `(bytes, revision:u64)` directly.
1. `source_dom_patch.rs` scanner: `pub fn parse(src:&str, revision:u64, &Limits{max_bytes,max_nodes,max_depth}, cancel:&CancellationToken) -> Result<WebDoc, WebError>`; `WebDoc{revision, source_sha256_hex, nodes:Vec<Node>}` with `Node{key:NodeKey /*stable path e.g. "0/2/1:div"*/, kind:Element|Text|Comment|Doctype|Raw, name:Option<String> /*original case kept*/, span:Range<usize> /*whole node*/, open_span, close_span:Option<Range<usize>>, attrs:Vec<Attr{name, value:Option<String>, quote:Quote::{Double,Single,None,Absent}, name_span, value_span}>, children:Vec<usize>, parent:Option<usize>}`. Subset v1: elements, attributes (3 quoting styles), text, comments, doctype, void elements (`br hr img input meta link ...`), raw-text elements `script`/`style`/`textarea`/`pre` kept as one `Raw` child (protected, STU-WEB-020). Optional-end-tag cases, misnested tags, unclosed raw text, `<![CDATA[`, foreign content (svg/math) beyond pass-through -> `WebError::UnsupportedConstruct{span}` or `Malformed{span}` (never guessed, never silently re-nested).
2. DOM -> source edits as TEXT patches: `pub struct SourcePatch{base_revision:u64, base_sha256_hex:String, edits:Vec<TextEdit{span:Range<usize>, replacement:String}>}`; `pub fn set_attribute(doc:&WebDoc, node:&NodeKey, name:&str, value:&str) -> Result<SourcePatch, WebError>` (replace the value span keeping the ORIGINAL quote style; if absent insert ` name="value"` after the tag name using double quotes; escape per quote), `set_text(doc, node, text)` (escape `& < >`), `remove_attribute`, `remove_node`; `pub fn apply(src:&str, &SourcePatch) -> Result<(String, u64 /*new revision = base+1 checked*/), WebError>` rejecting overlapping edits, `Conflict{span}` when the base hash/revision differs from the current source, `StaleRevision`. Oracle: every byte outside the patched span is identical (line endings, indentation, quoting, case, comments, unknown constructs) - assert with a byte-diff helper (`dom_edit_changes_exact_source_span`).
3. Code -> DOM refresh (`code_edit_refresh_same_revision`): `pub fn refresh(old:&WebDoc, new_src:&str, new_revision:u64, ..) -> Result<(WebDoc, KeyMap), WebError>` re-parses and maps `NodeKey`s old->new by path+tag+order (nodes whose key survives keep it; added/removed reported); an editor text change at revision R produces a DOM at the SAME R+1 with no divergence between `parse(apply(..))` and `refresh` (equality of both trees asserted).
4. Granted asset resolution + hostile input: `pub trait AssetPort: Send+Sync { fn resolve(&self, reference:&str) -> AssetDecision::{Granted{id:String}, Denied, Unavailable}; }`; `pub fn collect_assets(doc:&WebDoc, port:&dyn AssetPort) -> Vec<AssetRef{node, attr, reference, decision}>` over `img[src] link[href] script[src] source[src] video[src] iframe[src] a[href] use[href]` and CSS `url(...)` in inline styles: relative refs go to the port, `http(s)://`, `//`, `file:` and `javascript:` are `Denied` without fetching (`external_asset_denial_cancel_stale`); no I/O in this crate.
5. `responsive.rs` + preview model: `pub struct Breakpoint{min_width_px:Option<f64>, max_width_px:Option<f64>}`; `pub fn parse_media_query(q:&str) -> Result<Vec<Breakpoint>, WebError>` for the subset `(min-width: Npx)`, `(max-width: Npx)`, `and`, commas; `pub fn preview_tree(doc:&WebDoc, viewport_width_px:f64, ..) -> Result<PreviewTree, WebError>`: structured (non-pixel) preview = element tree with `role` (from tag: `nav main header footer button a[href] img[alt] h1..h6 ul li form input label`), `accessible_name` (aria-label > alt > text content, whitespace-normalised), `focusable` + keyboard `tab_order` (natural order; `tabindex`>0 first, negative skipped), and active inline `<style>` rules whose media query matches the width (simple selectors only: tag, `.class`, `#id`, comma lists; others -> `unmatched_unsupported` list). `PreviewTree` carries `source_revision` + `viewport_width_px` (STU-ARC-015). Same source drives editor and preview (`breakpoint_keyboard_a11y_preview`).
6. Observability: `DESCRIPTOR` + `report(...)`: `Conflict|StaleRevision|Malformed|UnsupportedConstruct` -> `Failure(Validation)`/`Unsupported`, `Denied assets` inside an Ok result -> `Success` + list, `Canceled` -> `Canceled`. Example `web-consumer --set-attr NODEKEY name value FILE.html`.
Oracle note: not on the operator's Adobe-crate list (the web reference app is Dreamweaver 2021, installed at `C:/Program Files/Adobe/`, but spec forbids vendor-named surfaces); oracle = independent re-parse/byte diff in tests plus a later browser-engine accessibility tree via the built-in browser (black-box).

</topic>

<topic id="deps" status="active" summary="third-party deps (verified 2026-10-11)">

## Deps
- Slice: NONE (std + accord + observe). Hand-formatted JSON in the example.
- Provider candidates for the later full parser (decision for the lead, NOT made here): `html5ever 0.40.1` (servo; MIT OR Apache-2.0; 2026-09-14 - 0.40.0 only 3 days earlier, very fresh; browser-grade tree building but NO source spans by default, so it cannot alone satisfy text-is-authority); `lol_html 3.0.1` (Cloudflare; BSD-3-Clause; 2026-07-29; streaming rewriter that preserves untouched bytes - a good fit for patching, weaker as a DOM tree); CSS: `cssparser` / `lightningcss` (not evaluated, UNVERIFIED); `taffy 0.14.0` (MIT, 2026-08-24) only for a future layout preview. All pure Rust except nothing native; each needs STU-ARC-018 provenance + feature review first.
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-web --tests` (then `clippy`, `test --test source_dom_patch`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/source_dom_patch.rs">

## Tests (<=5, `bounded_case`; MT case names)
1. `dom_edit_changes_exact_source_span`: CRLF document with tabs, single/double/unquoted attributes, mixed-case tags, a comment and a `<script>` block; `set_attribute`/`set_text` change exactly the intended span; every other byte identical; new revision = old+1; quoting style and tag case preserved.
2. `code_edit_refresh_same_revision`: a text edit applied through `apply` then `parse` equals `refresh`; surviving `NodeKey`s keep identity, added/removed nodes reported; `Raw` blocks untouched.
3. `breakpoint_keyboard_a11y_preview`: `@media (max-width: 600px) { .nav { display:none } }`: at 500 px the rule is active, at 800 px not; roles/accessible names (`img alt`, `aria-label`, text) and tab order (tabindex 1 before natural, -1 skipped) exact; preview carries `source_revision` and width.
4. `external_asset_denial_cancel_stale`: `http://`, `//cdn`, `file:`, `javascript:` refs `Denied` with the port never called for them; relative ref goes to the port (`Granted`/`Unavailable`); pre-cancelled -> `Canceled`; patch with old base hash -> `Conflict`; stale revision -> `StaleRevision`.
5. `malformed_and_hostile_input`: misnested tags, unclosed `<script>`, `<![CDATA[`, > `max_nodes`, > `max_depth`, > `max_bytes` -> typed errors with spans, no panic, bounded allocation; unsupported constructs never produce a patch.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No full HTML5 tree construction (optional end tags, foster parenting), no CSS cascade/specificity engine or layout, no JS execution, no templates/snippets/tag libraries/linting/code completion/formatter (STU-WEB-028 explicit format command is later), no site/publish/FTP/server models, no Folio `mode=web` mapping until Folio defines it, no panel/Controls wiring (descriptors only), no network or file I/O.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
