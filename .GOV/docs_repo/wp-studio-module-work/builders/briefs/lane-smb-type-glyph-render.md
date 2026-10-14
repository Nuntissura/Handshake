---
file_id: BUILDER-BRIEF-LANE-SMB-TYPE-GLYPH-RENDER
file_kind: builder_brief
updated_at: 2026-10-14T19:06:53Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-type
lane: SMB-TYPE-GLYPH-RENDER
owned_paths: [crates/hsk-studio-type/src/outline.rs, crates/hsk-studio-type/src/raster.rs, crates/hsk-studio-type/tests/glyph_render.rs, the `mod`/`pub use` lines in crates/hsk-studio-type/src/lib.rs, the descriptor entry in crates/hsk-studio-type/src/descriptor.rs]
depends_on: []
author_role: brief writer (parity follow-up 2026-10-14); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 9a2c840
parity_refs: [PS-23, AI-28, AI-29, FG-15, ID (text in PDF needs glyph ids only), AE-11, top_gap photoshop#5]
---

<topic id="finding" status="active" summary="What exists today">

## Finding (verified in code)
- Type shapes but never draws: `TextEngine::shaped_runs` (`src/engine.rs:16`) returns `Prepared` -> `ParagraphResult` -> `Run` (`src/result.rs:27-90`) with `glyphs(): &[Glyph{glyph_id, cluster, x_advance_pt, y_advance_pt, x_offset_pt, y_offset_pt}]` (`src/result.rs:4-11`), `units_per_em()`, `font_size_pt()`, `resolved(): FontResource{identity, location, content_hash, face_index, ...}` (`src/contract.rs:68`). Descriptor: excluded routes include "outline" work (`src/descriptor.rs`, `selected_profile.excluded_routes`).
- The vendored, bounded ttf-parser fork already extracts outlines: `trait OutlineBuilder { move_to, line_to, quad_to, curve_to, close }` (`src/provider/ttf/mod.rs:569`) and `Face::outline_glyph(glyph_id, &mut dyn OutlineBuilder) -> Option<Rect>` covering gvar/glyf/CFF/CFF2 (`:2117-2154`). The module is `pub(crate)` (`src/provider/mod.rs:4`).
- Font admission to reuse (hash check, face index, CFF2 refusal, table budget): `src/fonts.rs:30-63` (`ttf::RawFace::parse`, `CFF2` -> `Error::UnsupportedProfile`).
- Fixture fonts with licences: `tests/fixtures/fonts/Inter-Regular.ttf` (glyf, OFL.txt) and `NotoSansSC-Regular.otf` (CFF, NotoSans-OFL.txt), provenance in `tests/fixtures/FIXTURES_PROVENANCE.json`.
- Consumers that would draw text have no path: composite DESCRIPTOR lists `vector_or_text_to_image` as unsupported (`hsk-studio-composite/src/lib.rs:156`); render-cpu has path coverage for nib paths only (`hsk-studio-render-cpu/src/coverage.rs`).

</topic>

<topic id="ownership" status="active" summary="Who owns outlines and drawing">

## Owner
- **hsk-studio-type owns BOTH outline extraction and glyph rasterisation.** STU-TYP-127 names them explicitly as `TextEngine` responsibilities and says the render engine "receives positioned glyph runs and never re-shapes". Pigment/nib/render-cpu must not grow a second glyph rasteriser (CX-CAP-001).
- Type has no nib or pigment edge, so its outputs are neutral data: outline commands in font units, and a coverage plane in the exact `coverage_u8` byte layout pigment's `Mask`/`MaskFormat::CoverageU8` consumes (`hsk-studio-pigment/src/tile.rs:190`, `src/render_plan.rs:56`). Consumers convert:
  - composite (edge composite -> type is ALLOWED, unused today): text layer -> `PlanNode::Solid` + `MaskInput` from the glyph coverage (slice 2, composite owner);
  - layout / motion / interop-vector / render-cpu (all have nib + type edges): outline -> nib `OwnedPath` for create-outlines (STU-TYP-024) and vector/PDF paths;
  - press PDF export embeds fonts and glyph ids and needs no rasterisation (SMB-LAYOUT-PDF-EXPORT).

</topic>

<topic id="spec-basis" status="active" summary="Clauses (spec v02.216 module 14)">

## spec_basis
- STU-TYP-127 (l.8048): "`TextEngine` owns, and is the only owner of: ... composition into line boxes; and glyph outline extraction and rasterisation. `RenderEngine` receives positioned glyph runs and never re-shapes."
- STU-TYP-131 (l.8074): "the same input closure composed on at least two hosts produces byte-identical positioned-glyph output ... and byte-identical rasterised output at a fixed sample size."
- STU-TYP-128 (l.8055): "No ambient state - no process locale, no system font cache ... may influence the result."
- STU-TYP-126 (l.8037): native Rust stack, no platform text/raster service "in any build profile, including tests".
- STU-TYP-024 (l.9588): "Convert-to-outline: a `StudioTextStory` MAY be converted to editable vector geometry — a `StudioVectorNetwork`/`StudioVectorPath` of glyph outlines" (consumer slice, uses this lane's outlines).
- STU-VEC-178 (l.5006): create-outlines typed errors: "outlines cannot be created from OVERSET text; outlines cannot be created from bad or unresolvable glyph data" (this lane supplies the `glyph_outline_unavailable` code).

</topic>

<topic id="scope" status="active" summary="Slice 1 (~1-2 h)">

## Slice 1 (type crate only; additive public API)
1. `outline.rs` (`#![deny(clippy::arithmetic_side_effects)]`):
   - `pub enum OutlineCommand { MoveTo([f32;2]), LineTo([f32;2]), QuadTo([f32;2],[f32;2]), CurveTo([f32;2],[f32;2],[f32;2]), Close }` in font units, y up.
   - `pub struct GlyphOutline { glyph_id: u32, units_per_em: u16, commands: Vec<OutlineCommand>, bounds: Option<[i16;4]> }`.
   - `pub fn glyph_outlines(font: &FontBytes /* bytes + FontResource */, glyph_ids: &[u32], limits: &OutlineLimits, cancel: &CancellationToken) -> Result<Vec<GlyphOutline>, Error>`: verifies `content_hash` like `fonts.rs:36`, parses with the vendored `ttf::Face` (same face-index rules), refuses CFF2 exactly as `fonts.rs:50`, collects through a bounded `OutlineBuilder` (segment cap per glyph and per call; nonfinite refuses). Variable fonts: default instance only in slice 1 (axis coords from the run are a slice-2 input; a run with non-default axes refuses `variable_outline_unsupported`). A glyph with no outline (space) returns empty commands; a glyph id >= numGlyphs refuses `glyph_outline_unavailable`. COLR/sbix/SVG glyphs: monochrome outline only, colour layers refused (recorded).
2. `raster.rs`:
   - `pub struct GlyphCoverage { x: i64, y: i64, width: u32, height: u32, data: Vec<u8> }` — row-major `coverage_u8`, identical layout to pigment `Mask` bytes (stride = width).
   - `pub fn rasterize_run(run: &Run<'_>, outlines: &[GlyphOutline], origin_px: [f64;2], px_per_pt: f64, limits: &RasterLimits, cancel) -> Result<GlyphCoverage, Error>`: pen position from the run's advances/offsets (pt -> px via `px_per_pt`, scale = `font_size_pt * px_per_pt / units_per_em`), y flipped to raster down, glyph coverages summed with saturation into one plane over the run bounds.
   - Own deterministic rasteriser: flatten quads/cubics with a fixed subdivision rule (tolerance 1/16 px, recursion cap), then exact signed-area accumulation per cell in integer fixed point (24.8), non-zero winding (TrueType/CFF fill rule), unhinted. No floating-point accumulation order dependence, no SIMD, no lookup of host fonts -> STU-TYP-131 byte identity is provable by hashing the plane.
3. `COMPOSITION_VERSION` is NOT bumped (shaping unchanged); add a separate `RASTER_VERSION` constant recorded in the descriptor so a cached raster can be invalidated when the rasteriser changes.
4. Descriptor: remove "outline" from `excluded_routes`, add `outline`/`raster` entries (API, limits, unhinted, nonzero, default-instance-only, colour glyphs refused, NOT_PROVEN visual parity vs Adobe).

## Non-goals (slice 1)
- Hinting, subpixel/LCD AA, gamma/stem darkening, colour glyphs (COLR/CPAL/sbix/SVG), variable-font instancing, synthetic bold/italic, glyph cache, GPU atlas, text-on-path, decorations (underline/strike), line layout (layout owns it), composite/render integration (slice 2, composite owner), create-outlines command (slice 3, interop-vector or layout owner).

## Slice 2 (composite owner, separate lane `SMB-COMPOSITE-TEXT`)
- Composite gains the existing allowed `type` edge and lowers a point-text layer (single line, StudioTextStory + resolved fonts supplied by the caller) to `PlanNode::Solid{rect, rgba}` with a `MaskInput` (`coverage_u8`) whose plane is this lane's `GlyphCoverage`; render-cpu needs no change. Area text needs layout's composed lines; composite has no layout edge -> lead decision when that slice is planned.

</topic>

<topic id="native" status="active" summary="Relation to the native format">

## Native format
- Nothing new is stored. Text stays `StudioTextStory` (`hsk-studio-folio/src/story.rs`), fonts are referenced by content hash (STU-TYP-128/129). Outlines and coverage planes are derived, disposable outputs keyed by (font content hash, glyph id, size, `RASTER_VERSION`); never package authority.
- Create-outlines (later) writes ordinary vector records (`StudioVectorPath`) from these outlines; that is the only persisted product of this engine.

</topic>

<topic id="ui" status="active" summary="UI dependency">

## UI
- None. On-canvas text editing (caret, selection highlight) belongs to the GUI host (D11); the coverage planes are exactly what a host preview would composite.

</topic>

<topic id="deps" status="active" summary="Edges and crates.io">

## Dependencies
- No new Studio edge; no crates.io dependency (own rasteriser over the already-vendored parser).
- Rejected: `ab_glyph_rasterizer` 0.1.10 (in the lock via egui; Apache-2.0) — f32 accumulation and `unsafe` in `raster.rs` (local registry grep), and type has kept its whole stack vendored and bounded for STU-TYP-131; `tiny-skia`/`zeno`/`swash` — new dependency closures for ~150 lines of code.

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (at most 5 focused tests, `tests/glyph_render.rs`; fixtures from `tests/fixtures/fonts` only, V4-03)
1. **Outlines.** Inter `glyph_outlines` for the shaped glyph of 'l' returns a closed contour whose bounds equal the glyf bbox; NotoSansSC (CFF) returns cubic commands; a space returns empty commands; glyph id = numGlyphs refuses `glyph_outline_unavailable`; a CFF2 face refuses `UnsupportedProfile` (reuse of the existing rule).
2. **Rasteriser hand values.** A synthetic outline square 10x10 px aligned to the grid gives 255 inside and 0 outside; offset by 0.5 px gives 128 (+/-1 by the fixed-point rule, exact value asserted) on edge pixels; a triangle's coverage sum equals its area within 0.5 %.
3. **Run raster.** Shape "Hi" with Inter at 24 pt, 96 ppi (`px_per_pt = 96/72`) through the existing test harness in `tests/support/mod.rs`; the plane's ink sum is > 0, its x extent equals the summed advances within 1 px, and the SHA-256 of the plane equals a constant asserted in the test (determinism anchor; the cross-host half of STU-TYP-131 is NOT_PROVEN until a second host runs it).
4. **Interop layout.** `GlyphCoverage.data` passes `hsk_studio_pigment::Mask::validate` as `coverage_u8` — only if type may dev-depend on pigment (no edge exists; do NOT add one). Instead assert the documented layout (len = width*height, stride = width); the composite slice proves the plug-in.
5. **Hostile (V4-02).** `dims_u32_max`: `px_per_pt = 1e12` or `font_size_pt` huge refuses `raster_over_limit` before allocation; `aggregate_over_budget`: 10^6 glyph ids or an outline with more than `OutlineLimits.max_segments` refuses before work; nonfinite origin/scale refuses; a truncated font byte slice refuses `CorruptFont` without panic.

## Proof commands
- `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-type --tests`
- same prefix: `cargo clippy -p hsk-studio-type --tests`; `cargo nextest run -p hsk-studio-type --test glyph_render`
- same prefix: `cargo check -p hsk-studio-layout -p hsk-studio-interop-psd --tests` (type is consumed by them; additive API only).

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Visual parity:** unhinted greyscale AA looks lighter than Adobe at small sizes. Mitigation: declared in the descriptor; Adobe comparison stays D5 (text cases) and is NOT claimed.
- **Composite-glyph recursion / huge contours in hostile fonts.** The vendored parser bounds recursion; the bounded builder caps segments; tests include a truncated font.
- **Fixture licence:** Inter and Noto are OFL with the licence files already beside them; no new font downloads.
- **Shared crate (61k LOC):** additive modules only; existing `foundation_contract` tests must keep compiling (proof list).

</topic>

<topic id="cohesion" status="active" summary="How it serves several apps">

## Cohesion
- One outline + raster engine draws text for Photoshop type layers, Figma text nodes, Illustrator point/area type and create-outlines, After Effects text layers (each frame re-rasterises the same runs) and InDesign preview; PDF export uses the same shaped glyph ids with embedded fonts, so screen and PDF agree on glyph selection.

</topic>
