---
file_id: BUILDER-BRIEF-HSK-STUDIO-INTEROP-VECTOR
file_kind: builder_brief
updated_at: 2026-10-11T06:00:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-vector
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-INTEROP_VECTOR` / key `interop_vector` / crate `hsk-studio-interop-vector`; foundation MT-35549 (test target `vector_edit_roundtrip`; cases `svg_edit_transform_style_reopen_twice`, `ai_native_editability_or_explicit_blocker`, `external_entity_link_denial_budget_cancel`, `unit_loss_stale_profile`); write scope `handshake-studio-assets/crates/hsk-studio-interop-vector/**`. Files per MT: `src/svg_adapter.rs`, `src/ai_adapter.rs`, `src/pdf_port.rs`, `tests/vector_edit_roundtrip.rs`.
- Responsibility (verbatim): "SVG/AI vector import/export with fallible loss-aware proposals; PDF decoding/editing exposed through Press provider ports, not unconditional all-PDF engine."
- Foundation capability: "Import actual SVG/AI/PDF selected adapter and emit explicit fallback/loss; independent editable envelope proof later."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-nib`, `hsk-studio-type`, `hsk-studio-prism`, `hsk-studio-observe`. NO `press` edge: PDF is a TRAIT port defined here, implemented later by a host/harness adapter. Slice uses accord, folio, nib, observe.
- Forbidden: handshake_core/native/SurrealDB; network access / external entity or `href` fetching (hostile by default); all-PDF engine dependency; a private vector/style store (ENG-DAG-FORBID-003; styles belong to the PAINT style owner, STU-CON-033).
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `interop-vector-consumer` (`--descriptor`), 30 s `bounded_case`. Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-CON-032..034 (l.22528+): canonical Path input = `hsk.studio.vector_path@1`, `path_id` SVPT, `address {layer_id SLYR, object_key, property:"geometry"}`, `closed`, `anchors`, `winding_rule` (NONZERO|EVENODD|NONE), `fill_style_id` (SSTY|null); coordinates AND handle components are length records in POINTS - "callers perform explicit existing Accord conversion before supplying ... retaining any original import provenance separately"; handles are RELATIVE vectors from the anchor (segment i->i+1 controls = `position_i+outgoing_i`, `position_next+incoming_next`; both zero = straight); `handle_mirroring` NONE|ANGLE|ANGLE_AND_LENGTH; never auto-close open paths.
- STU-IO-006 rows 15-20 (l.~21330): AI/AIT (NRT, preserved/transformed + preservation blob), SVG/SVGZ (SOX), EPS/PS, PDF; STU-IO-004 five-artifact gate, STU-IO-005 preservation blob + receipt, STU-IO-186 (l.211): native editable AI is NOT satisfied by PDF preview/parse; missing decoder = blocking scope gap, never a silent exclusion; STU-ARC-019: imported AI-generated/opaque objects keep explicit preservation/loss semantics; STU-ARC-018 selective reuse with pinned hash + licence provenance; STU-ARC-008/012 bounded admission before decode (entry/node/coordinate limits).
- STU-DOC-003 (unit law), STU-RAS-/STU-VEC-* path model 14.5.1 (l.3058+), 14.5 drawing/shape catalogue (l.3268+). Later scope: gradients/patterns/appearance/graphic styles (14.5.5-7), brushes, live effects, text-on-path, artboards (14.5.9), AI native decode, EPS/PostScript, `_mt_plan_05_vector.json` (58 MTs), `_mt_plan_1213_raw_interop.json` MT-456..461.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, ~800-1100 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[11]` (MT-35549; "Fallible semantic vector import/export preserving editable paths/styles/transforms and source-qualified identity; PDF via Press port, native AI version gaps explicit"; provider "Vectorcraft pathops inspected; actual SVG/AI decoder source NOT_INSPECTED. Press exposes PDF decoding/editing, not an all-PDF dependency"; oracle "Original Illustrator-produced source and independent native reopen/render/edit comparison; PDF preview alone cannot prove native AI editability"; gap "decoder/provider/version pins and text/gradient/appearance/linked-image field mappings"); Craft reference (pinned cache `.../vectorcraft-825d4fdb.../crates/svg/src/{import.rs 1248, export.rs 2080, css.rs}`: imports through `usvg` with transforms baked; documents export approximations; MIT OR Apache-2.0) - read for the list of lossy cases only; `.GOV/reference/studio_app_feature_research/20-illustrator-feature-map.md`, `27-illustrator-figma-parity-matrix.md`, `19-studio-local-first-rust-posture.md` (compat rule: explicit unsupported diagnostics over silent lossy conversion).
Reuse (verified): `nib::{Anchor{position,incoming,outgoing: Point, handle_mirroring: Mirroring}, Point::pt(x,y) (Length in Unit::Points), Mirroring::None, Winding::{NonZero,EvenOdd,None}}`; `folio::{StudioLayer, Payload::Vector{path_ids}, Payload::Unsupported{..}, validate_document, Budget, Unresolved}` (vector layer = `kind:"vector"`, `Payload::Vector{path_ids:["SVPT-..."]}`); `accord::{Length::convert(Unit::Points, Some(pixels_per_inch)), Unit, DomainId (prefix SVPT, SSTY valid), CancellationToken}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. Path geometry is carried as `OwnedPath` here (nib's `Path<'a>` is a borrowed transport); style data is plain `StyleProposal` (fill rgba, fill_rule, stroke rgba/width, opacity) - NOT a style primitive.
1. `svg_adapter.rs` import: `pub fn import_svg(bytes:&[u8], policy:&ImportPolicy, mint:&mut dyn IdMint, cancel:&CancellationToken) -> Result<VectorProposal, VectorError>`; `ImportPolicy{limits:Limits{max_bytes,max_nodes,max_depth,max_segments}, pixels_per_inch:f64 /*CSS px = 96 -> pt factor 0.75*/, allow_external:false}`. Parse XML with `roxmltree` (DTD disallowed, node limit set), handle `svg` (viewBox/width/height), `g` (transform), `path` (`d`), `rect/circle/ellipse/line/polyline/polygon`, presentation attrs `fill, stroke, stroke-width, fill-rule, opacity` and inline `style=""`. Path data via `svgtypes` simplifying parser (arcs -> cubics = `LossKind::Approximated`), cubic/quadratic -> relative-handle `nib::Anchor`s (quad elevated to cubic exactly), `Z` -> `closed=true`; transforms BAKED into points (`LossKind::TransformBaked`); every px->pt conversion recorded once in `Provenance{source_unit:"px", ppi, factor}`. `VectorProposal{snapshot:folio::Snapshot, paths:Vec<OwnedPath{path_id, closed, anchors, winding}>, styles:Vec<StyleProposal>, loss:Vec<LossEntry{path:String, kind:LossKind, code:&'static str}>, provenance}` where the Folio doc is built with `Payload::Vector` layers (validated via `folio::validate_document`). Unmapped elements (filter, mask, clipPath, gradient/pattern fills, text, image, use/symbol, style sheets, animation) each yield a `LossEntry` (never silently dropped); unknown elements may be preserved as `Payload::Unsupported` bytes.
2. Hostile input (`external_entity_link_denial_budget_cancel`): DOCTYPE/entities rejected; `<image href>` / `<use href>` / `xlink:href` to anything not `#local` or `data:` -> `LossKind::ExternalDenied` (no fetch, ever); node/depth/segment/byte budgets -> `BudgetExceeded`; non-finite or > 1e9 coordinates -> `InvalidGeometry`; cancel checked per element.
3. `svg_adapter.rs` export: `pub fn export_svg(paths:&[OwnedPath], styles:&[StyleProposal], opts:&ExportOptions) -> Result<String, VectorError>` writing SVG 1.1 (`viewBox` in px user units from pt via the recorded factor, `d` with absolute `M/C/L/Z` from `position_i+outgoing_i` / `position_next+incoming_next`, `fill-rule`, fill/stroke attrs), deterministic number formatting. Roundtrip oracle: import -> export -> import twice gives identical anchors (|d| < 1e-9 pt) and identical loss kinds (`svg_edit_transform_style_reopen_twice`); an edit (move one anchor via a plain `Vec` change) survives the reopen.
4. `ai_adapter.rs`: `pub fn identify_ai(bytes:&[u8]) -> AiKind::{NotAi, PdfCompatible{has_private_data:bool}, LegacyPostScript, Unknown}` (header sniff: `%PDF-` + Illustrator private-data markers; marker names UNVERIFIED - confirm on an Illustrator-saved fixture; Illustrator is installed on the operator machine) and `pub fn import_ai(..) -> Result<VectorProposal, VectorError::NativeEditabilityBlocked{version:Option<String>, reason:&'static str}>` ALWAYS returns the explicit blocker until a decoder exists (STU-IO-186); the original bytes are returned as a quarantined preservation blob in the error/outcome so nothing is lost.
5. `pdf_port.rs`: `pub trait PdfVectorProvider: Send+Sync { fn import_page(&self, pdf:&[u8], page:u32, cancel:&CancellationToken) -> Result<VectorProposal, VectorError>; }` plus `NoPdfProvider` (always `Unavailable`) so the "selected adapter" decision is explicit; `press` implements it later via a host adapter.
6. Observability: `DESCRIPTOR` + `report(...)`: `ExternalDenied|Approximated` (inside an Ok proposal) -> `Success` with loss list; `NativeEditabilityBlocked|PdfUnavailable` -> `Failure(Unavailable)`; budget/invalid -> `Failure(Validation)`/`Unsupported`; `Canceled` -> `Canceled`.
Oracle note: not on the operator's Adobe list, but the same rule holds: later compare Illustrator-produced SVG/AI fixtures black-box (anchors, handle vectors, transforms, fill rules, units) against our import; PDF preview never proves native AI editability.

</topic>

<topic id="deps" status="active" summary="third-party deps (verified 2026-10-11)">

## Deps
- `roxmltree 0.21.1` (RazrFalcon; MIT OR Apache-2.0; 2025-10-12; 83.8M downloads; pure Rust; NOTE 0.21.0 is yanked - pin `=0.21.1`; `ParsingOptions`/`nodes_limit` API names UNVERIFIED, read docs.rs).
- `svgtypes 0.16.1` (linebender; Apache-2.0 OR MIT; 2026-01-09; pure Rust; verify it does not pull more than it needs: Craft vectorcraft-svg uses `svgtypes = "0.16"`). Needed for path data, colours, transforms, lengths.
- `kurbo =0.13.1` (already pinned by nib) only if arc/quad conversion is done locally instead of via svgtypes.
- Rejected for the core: `usvg` 0.48 (Craft's choice; bakes everything, loses editable structure - fine as a RENDER cross-check later, not as importer), `resvg`, `krilla`/`hayro` (PDF, belongs to press), `lopdf`. SVGZ gzip via `flate2` rust backend is a later capability.
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-interop-vector --tests` (then `clippy`, `test --test vector_edit_roundtrip`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/vector_edit_roundtrip.rs">

## Tests (<=5, `bounded_case`; MT case names)
1. `svg_edit_transform_style_reopen_twice`: SVG with `<g transform="translate(10,20)">`, a cubic path, a rect and `fill-rule=evenodd`; import -> hand-computed anchors in pt (px*0.75, handles relative) -> export -> import twice, identical; one anchor edited and reopened.
2. `ai_native_editability_or_explicit_blocker`: PDF-compatible AI-like bytes -> `NativeEditabilityBlocked` with the original bytes preserved byte-exact in the blob; plain SVG bytes -> `NotAi`.
3. `external_entity_link_denial_budget_cancel`: `<!DOCTYPE>` with entity, `<image href="http://...">`, `<use xlink:href="file.svg#a">` denied/reported with zero network/file access; over-budget nodes/segments -> `BudgetExceeded`; pre-cancelled -> `Canceled`.
4. `unit_loss_stale_profile`: arcs reported `Approximated`, gradient fill reported unsupported (not dropped silently), non-finite coordinate -> `InvalidGeometry`; `NoPdfProvider` -> `Unavailable`.
5. `folio_vector_layer_validates`: proposal snapshot passes `folio::validate_document`, `path_ids` match `paths`, ids have prefix SVPT/SLYR.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No gradients/patterns/filters/masks/clips/text/images/symbols/CSS, no appearance stacks or live effects, no stroke geometry/booleans (nib), no AI decode, no EPS/PS, no PDF decode (press port only), no SVGZ, no font handling (type), no ICC (prism), no style primitive creation, no host wiring.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
