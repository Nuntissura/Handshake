---
file_id: BUILDER-BRIEF-LANE-SMB-LAYOUT-PDF-EXPORT
file_kind: builder_brief
updated_at: 2026-10-14T19:06:53Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-press (slice 1: PDF writer API) ; hsk-studio-interop-layout (slice 2: document -> writer lowering, needs edge decision L1)
lane: SMB-LAYOUT-PDF-EXPORT
owned_paths: [crates/hsk-studio-press/src/export.rs, crates/hsk-studio-press/src/export/**, crates/hsk-studio-press/tests/export.rs, the `mod export;` + `pub use export::...` lines and the DESCRIPTOR `api` entry in crates/hsk-studio-press/src/lib.rs]
depends_on: [nothing for slice 1; decision L1 (edge interop-layout -> press) for slice 2; press lanes SMB-PRESS-* own other press files: touch only the owned paths above (V4-14: do not push under their READY claims; coordinate commit timing with the lead)]
author_role: brief writer (parity follow-up 2026-10-14); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 9a2c840
parity_refs: [ID-27, FG-20 (PDF), AI interop "PDF / EPS save MISSING", indesign interop "PDF save MISSING", top_gap indesign#2]
---

<topic id="finding" status="active" summary="What exists today">

## Finding (verified in code)
- No crate writes a NEW PDF. Press opens, edits, redacts, flattens and full-saves EXISTING PDFs (`src/lib.rs:1-60`; `PdfDoc::open` `src/pdf_edit.rs:152`, `save_full` `:436`).
- Press already owns every serialisation primitive an exporter needs (all `pub(crate)` today):
  - object/dict/name/string/real writers: `src/writer.rs:17` `write_object`, `:48` `write_dict`, `:59` `format_real`, `:68` `write_name`;
  - complete file writer with xref and trailer, bounded by `Limits` (`#![deny(clippy::arithmetic_side_effects)]`, hostile `dims_u32_max` test): `src/writer.rs:130` `write_document`;
  - content-stream operator serialisation: `src/canon.rs:124` `write_op`, `:150` `write_ops`; Flate stream compression with `miniz_oxide::deflate::compress_to_vec_zlib` (`src/canon.rs:421-434`);
  - a raster page writer used by flatten (image XObject + content): `src/flatten.rs:589` `raster_content`.
- Independent reader for proof: press renders pages through `hayro-interpret` 0.7.0 into its own device (`src/paint.rs`, path fill `:356`, glyph outlines `:438`, images `:450`) — a third-party interpreter, not the writer's code.
- Press deps: hayro-interpret, kurbo, miniz_oxide, sha2 (`Cargo.toml`); Studio edges allowed: accord, folio, observe only.
- Layout composes frames/lines (`hsk-studio-layout/src/threaded_reflow.rs`, `paragraph.rs`) and type gives glyph ids/advances per run (`hsk-studio-type/src/result.rs:4-90`); neither can reach press (no edge), and press cannot reach them.

</topic>

<topic id="ownership" status="active" summary="Owners per CX-CAP-001">

## Owners
- **Press owns PDF serialisation** (STUDIO_MODULES: "prepress/accessibility/redaction/full-save"; interop-vector's responsibility text says PDF is "exposed through Press provider ports"). A second PDF writer elsewhere would duplicate writer.rs/canon.rs. So slice 1 adds a PUBLIC builder API to press that takes neutral, already-resolved page content (no Folio, layout or type types): press stays at its current edges.
- **Lowering the native document to that page content** needs layout (frames/lines), type (glyph runs, font bytes), nib (paths), pigment/prism (tiles, colour). Recommended owner: **hsk-studio-interop-layout** ("IDML/INDD/print interchange conversion"; already has folio, layout, type, prism, package edges) + new edge **interop-layout -> press** (decision L1; no cycle: press -> folio/accord/observe only). Rejected: press -> layout/type edges (turns the PDF owner into an all-engine facade, ENG-DAG/STU-ARC-018 "all-engine facades MUST NOT"); layout -> press (engine above an interop provider).

</topic>

<topic id="spec-basis" status="active" summary="Clauses (spec v02.216 module 14)">

## spec_basis
- STU-IO-006 row 20 (l.21336): "Portable document format | `.pdf` | Vector/doc | I·EP·X | Multi-page import (pages as layers/spreads); vector+text+raster; text preserved on export where not rasterized".
- STU-IO-006 row 26 (l.21342): "Prepress PDF | PDF/X (`.pdf`) | Layout (prepress) | X | Standards-constrained prepress PDF; color conversion, marks, bleed preserved to the chosen PDF/X standard".
- STU-LAY-061 (l.7091): "PDF (print) export MUST expose its full panel surface." STU-LAY-158 (l.7093): 88-property export preference; Output panel `pdf_destination_profile`, `include_icc_profiles`, `output_condition`, `output_condition_name`, `oc_registry` and "the four `effective_*` read-back fields that report what the export actually resolved".
- STU-PDF-002 (l.243): "Shared print/export plans MUST preserve selected output profile hash/intent ... Artifact-produced status, PDF/A object-subset check or approximate preview MUST NOT prove PDF/X, PDF/UA, plate or tagged-order conformity. Independent conformance/separation/structure checks and original-authored edited fixtures are required."
- STU-TYP-127 (l.8048): the render side "receives positioned glyph runs and never re-shapes" (the writer takes glyph ids + positions, never text to shape).

</topic>

<topic id="scope" status="active" summary="Slice 1 (~1-2 h)">

## Slice 1: press `export` builder (pure bytes out; module `#![deny(clippy::arithmetic_side_effects)]`)
Public API (names are proposals; no canonical names verified):
1. `PdfExport::new(ExportOptions{ version: "1.7", output_intent: Option<OutputIntent{ icc: &[u8], components: 1|3|4, output_condition_identifier: &str, output_condition: Option<&str>, registry: Option<&str>, subtype: GtsPdfx }>, info_title: Option<&str> }, &Limits)`.
2. `add_page(PageSpec{ media_box_pt: [f64;4], trim_box_pt: Option<[f64;4]>, bleed_box_pt: Option<[f64;4]> }) -> PageHandle` and per page, in paint order:
   - `fill_path(path: &[PathCommand /* MoveTo, LineTo, CubicTo, Close in pt */], rule: NonZero|EvenOdd, colour: DeviceGray|DeviceRgb|DeviceCmyk|IccBased(handle))`; `stroke_path(.., width_pt, cap, join)`.
   - `draw_image(ImageSpec{ width, height, bits: 8|16, colour: DeviceGray|DeviceRgb|DeviceCmyk|IccBased(handle), samples: &[u8], alpha: Option<&[u8]> /* -> /SMask */ }, placement: [f64;6] /* cm matrix */)`; streams FlateDecode-compressed with the existing miniz_oxide call.
   - `show_glyphs(font: FontHandle, size_pt, origin_pt, glyphs: &[(u16 glyph_id, f64 x_pt, f64 y_pt)])` written as `BT /Fn size Tf ... TJ`/`Tm` with 2-byte CIDs.
3. `embed_truetype(bytes: &[u8], face_index: u32, widths: &[(u16 gid, u16 width_1000em)], to_unicode: &[(u16 gid, String)]) -> FontHandle`: Type0 + CIDFontType2, `/Encoding /Identity-H`, `/CIDToGIDMap /Identity`, `/W` from the caller's widths, `/FontFile2` = the full font (no subsetting in slice 1), `/ToUnicode` CMap from the caller map (text stays extractable/searchable). CFF/OpenType-CFF refuses `font_cff_not_embedded` (slice 2: `/FontFile3 /OpenType`). A font whose OS/2 `fsType` forbids embedding refuses `font_embedding_restricted` (read 2 bytes from the OS/2 table; bounded).
4. `finish(cancel) -> Result<(Vec<u8>, ExportReceipt)>` through the existing `write_document`; `ExportReceipt` carries the effective read-back values STU-LAY-158 asks for (`effective_output_condition`, `effective_destination_profile_sha256`, pages, fonts embedded, images, bytes) and `conformance: "not_proven"`.
5. Every refusal is typed (`PressError` gets new variants or a new `ExportError` with stable snake_case codes); no partial file is returned on error.

## Non-goals (slice 1)
- Font subsetting, CFF embedding, Type3/colour fonts, transparency groups and blend modes (beyond the image SMask), spot colours/separations/overprint, marks and bleeds drawing, PDF/X or PDF/A claims (STU-PDF-002), tagged PDF/structure (row 27), links/bookmarks, security/encryption, presets (STU-IO-133), downsampling, JPEG passthrough, incremental update.
- No Folio/layout/type/nib/pigment knowledge in press.

## Slice 2 (interop-layout owner, after decision L1)
- `export_pdf(snapshot, layout composition, resolved fonts/tiles, options)`: each `StudioPageSpread` page -> `add_page` (media/trim/bleed from Folio layout records), text frames -> composed lines from layout + shaped runs from type -> `show_glyphs` (fonts embedded by content hash, once), vector records -> nib paths -> `fill_path`/`stroke_path`, placed raster -> tiles converted by prism to the output profile -> `draw_image`; losses (effects, blend modes, unsupported fonts) in a typed export receipt (STU-IO-004 receipts).
## Slice 3
- Subsetting (own glyf subsetter or the `subsetter` crate after licence/age review), CFF, transparency groups, PDF/X-4 output intent + an INDEPENDENT conformance check (e.g. veraPDF as an optional external oracle, never a prerequisite, CX-503S).

</topic>

<topic id="native" status="active" summary="Relation to the native format">

## Native format
- Export is a derived deliverable: the package/Folio graph stores nothing new. The export recipe (`StudioExportRecipe`, STU-UNI-003 "ONE export surface") and its effective read-back receipt are the persisted parts later (session/export owner), never the PDF bytes as authority (press DESCRIPTOR: "PDF bytes are provider transport").
- Fonts are referenced in the document by content hash and embedded at export time from the same bytes the shaper used, so the PDF shows exactly the shaped glyphs.

</topic>

<topic id="ui" status="active" summary="UI dependency">

## UI
- None. The export panel surface (STU-LAY-061/158) is GUI-host work (D11); slice 1's `ExportOptions` + receipt are the typed command surface a model or the headless harness drives.

</topic>

<topic id="deps" status="active" summary="Edges and crates.io">

## Dependencies
- Slice 1: none (reuses miniz_oxide, sha2 already in press).
- Slice 2: edge `hsk-studio-interop-layout -> hsk-studio-press` (decision L1; `steward-sync.sh --add-edge hsk-studio-interop-layout=hsk-studio-press`); interop-layout also needs `nib`/`pigment` for paths/tiles: nib and pigment are NOT in its allowed list today (allowed: folio, layout, type, prism, package) -> include them in L1 or let layout hand over nib paths (layout already has nib).

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (slice 1; at most 5 focused tests in `tests/export.rs` plus one unit test inside `src/export` for the hayro render, since the paint device is crate-private)
1. **Structure re-read by press's own reader:** a 2-page export (A4 and Letter media boxes, trim box on page 2) opens with `PdfDoc::open`; `inspect().page_count == 2` and each `PageView.media_box` (`src/pdf_edit.rs:84-91`) equals the input; the trim box is checked in the in-crate unit test through `PdfDoc` dict access (PageView has no trim box); the output intent dictionary carries the given identifier and the ICC stream bytes equal the input (sha256).
2. **Independent render (hayro):** page 1 = a filled red 100x100 pt square at (100,100), a 2x2 RGB image with an alpha SMask placed at (300,300) scaled to 50 pt, and the glyphs of "Hi" from `hsk-studio-type/tests/fixtures/fonts/Inter-Regular.ttf` (referenced with `include_bytes!` from the type crate's fixture path, OFL licence file beside it; no copy). Rendered through the hayro device: probe pixels inside the square are red, outside white; the image region shows the expected colours with alpha; the text bounding region has ink > 0 and nothing else on the page does.
3. **Text extractability:** the `/ToUnicode` CMap maps the two glyph ids back to "H" and "i" (parse the stream back with press's reader).
4. **Determinism:** two exports of the same input are byte-identical (no timestamps or random ids unless given in options).
5. **Hostile (V4-02):** `dims_u32_max` (image width x height x components overflow refuses before allocation), `aggregate_over_budget` (total stream bytes over `Limits::max_bytes` refuses before writing; page count over `max_object_number` refuses), nonfinite coordinates refuse, a truncated/garbage font refuses `font_unreadable` (no panic), CFF font refuses `font_cff_not_embedded`.

## Proof commands
- `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-press --tests`
- same prefix: `cargo clippy -p hsk-studio-press --tests`; `cargo nextest run -p hsk-studio-press --test export` and `cargo nextest run -p hsk-studio-press --lib export`

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Press is under active remediation (SMB-PRESS-* lanes, pre-review loops).** Mitigation: new module only, no edits to writer.rs/canon.rs bodies (only `pub(crate)` visibility is reused from inside the crate); commit on its own lane id; tell the lead before pushing so press READY claims are not disturbed (V4-14).
- **Hostile text in user strings** (titles, output condition names): escaped by the existing `write_string` (`src/writer.rs:79`), UTF-16BE via `text_codec::utf16be_bytes` (`src/text_codec.rs:70`); a test with `)`, `\`, and non-ASCII (UTF-16BE with BOM) is in test 1 (V4-08 class: hostile text formats).
- **Self-verification is not conformance** (STU-PDF-002). The receipt says `conformance: not_proven`; hayro rendering proves drawability only. PDF/X needs an independent checker later.
- **Full-font embedding size** (Inter 680 KB per font). Accepted for slice 1; subsetting is slice 3.
- **Licensing of embedded fonts:** fsType check refuses restricted fonts rather than embedding silently.

</topic>

<topic id="cohesion" status="active" summary="How it serves several apps">

## Cohesion
- One PDF writer serves InDesign PDF export (slice 2), Illustrator "save as PDF"/PDF-compatible AI, Figma frame-to-PDF, Photoshop "save as PDF" (one page, one image), and press's own flatten/redaction (which can later move onto the same builder instead of `raster_content`). Corporate-compat priority: text stays text with embedded fonts and ToUnicode, colour is tagged with an output intent from slice 1.

</topic>
