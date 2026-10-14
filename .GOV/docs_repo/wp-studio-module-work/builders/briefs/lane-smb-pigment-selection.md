---
file_id: BUILDER-BRIEF-LANE-SMB-PIGMENT-SELECTION
file_kind: builder_brief
updated_at: 2026-10-14T19:06:53Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-pigment
lane: SMB-PIGMENT-SELECTION
owned_paths: [crates/hsk-studio-pigment/src/selection.rs, crates/hsk-studio-pigment/src/selection/**, crates/hsk-studio-pigment/tests/selection.rs, one `pub mod selection;` line in crates/hsk-studio-pigment/src/lib.rs]
depends_on: []
sequenced_before: [SMB-PIGMENT-BRUSH (same crate; it consumes CoveragePlane and the combine function)]
author_role: brief writer (parity follow-up 2026-10-14); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 9a2c840 (files read at 1f9b30b; pigment, folio and render-cpu unchanged since)
parity_refs: [PS-13, PS-14, LR-20, AI (STU-VEC-071 shared selection), top_gap photoshop#1]
---

<topic id="finding" status="active" summary="What exists today">

## Finding (verified in code)
- No selection primitive anywhere: grep `StudioSelectionSet|lasso|magic_wand|selection` over `crates/` finds only comments (parity PS-13 evidence; folio `values.rs:4` comment only).
- Pigment already owns the coverage vocabulary that a selection must produce:
  - `Mask` (borrowed coverage plane, formats `coverage_u8` / `coverage_u16le`, sha256-checked): `src/tile.rs:190-230`.
  - Render-plan mask slot `MaskFormat::{CoverageU8, CoverageU16Le}` and `MaskInput{placement, density, inverted}`: `src/render_plan.rs:56`, `:114`.
  - Feather: `coverage_math::feather_sigma/feather_support/feather_kernel` (Gaussian, sigma = r/2, separable, hard max 10000 px): `src/coverage_math.rs:63-101`.
  - Density/invert: `coverage_math::mask_coverage`: `src/coverage_math.rs:54`.
  - Integer rect algebra with checked arithmetic: `Rect::{end, intersect, contains, area}`: `src/tile.rs:9-51`.
- Folio stores masks as coverage tiles: `StudioMask{tiles: Vec<TileRef>, enabled, linked}` (`hsk-studio-folio/src/appearance.rs:49`) and `StudioMaskParameters{density, feather, inverted}` (`hsk-studio-folio/src/raster.rs:181`). There is no `StudioSelectionSet` record.
- Render-cpu path coverage rule to stay consistent with: N x N pixel-centred samples, `hits / N^2` (`hsk-studio-render-cpu/src/coverage.rs:98-112`). Render-cpu depends on pigment, not the reverse; pigment has no nib edge.

</topic>

<topic id="ownership" status="active" summary="Owner per CX-CAP-001">

## Owner
- **hsk-studio-pigment** (STUDIO_MODULES: "CPU tiles, sample formats, masks, brush/raster operations"). Selection is pixel coverage, the same primitive as `StudioMask`; STU-RAS-118 says the combine set is shared by "every selection-producing tool, every mask boolean". One module, consumed by brush (quick mask, selection brush), develop (local masks: develop -> pigment edge exists) and composite/render (as `MaskInput`).
- Polygon coverage lives here because pigment is the lowest coverage owner. Render-cpu keeps its curve coverage for now; consolidating render-cpu's straight-segment case onto this module is a later lead decision (record as CX-CAP-001 watch item, not this lane).
- Vector selection (STU-VEC-071: same `StudioSelectionSet`) is object membership, not pixels: out of scope here; the selection record that unifies both is a Folio schema task (slice 3).

</topic>

<topic id="spec-basis" status="active" summary="Clauses (spec v02.216 module 14)">

## spec_basis
- STU-RAS-017 (l.695): "`StudioSelectionSet` (14.3) is the single canonical selection primitive. All selection tools produce, refine, or consume a `StudioSelectionSet`; there is no per-tool bespoke selection representation. Every selection-producing tool MUST expose the shared combine modes, a document-wide default anti-alias toggle, and per-tool feather."
- STU-RAS-118 (l.700): "closed four-member enumeration ... `replace = 1`, `extend = 2` (add), `diminish = 3` (subtract), `intersect = 4`" and the vector boolean set "MUST NOT be merged with the selection combine set".
- STU-RAS-018 (l.709) rows: "Geometric marquee | Rectangular, elliptical, single-row (1px), and single-column (1px) pixel selections ..."; "Polygonal lasso | Straight-segment click-to-build selection"; "Freehand lasso"; "Color-range select | Eyedropper-sampled fuzziness-masked selection by color similarity with a live preview".
- STU-RAS-119 (l.729): `feather` hard_min 0.0, soft_max 250.0, default 0.0, "hard max not declared by any source"; `anti_alias` default true.
- STU-RAS-120 (l.746): Grow/Shrink/Feather/Smooth/Border/Expand/Contract "MUST NOT clamp to a guessed ceiling" (slice 2).
- STU-RAS-022 (l.820): save/load selection and alpha channels "are one shared mechanism, not three" (native relation below).
- STU-RAW-014b (l.2519): "Masks MUST compose ... combine by add, subtract and intersect ... Intersect and subtract semantics are the canonical `StudioMask` semantics of 14.23; the develop pipeline does not fork them."
- STU-UNI-003 (l.22386): "ONE selection model (`StudioSelectionSet` regardless of raster/vector/layout context)".

</topic>

<topic id="scope" status="active" summary="Slice 1 (~1-2 h)">

## Slice 1: pixel selection engine (pure functions, no ports, no Folio writes)
New module `pigment::selection` (`#![deny(clippy::arithmetic_side_effects)]`, V4-01):
1. `CoveragePlane { bounds: Rect, data: Vec<u16> }` (u16 = coverage/65535, row-major, `bounds.area()` samples). Constructors check `area * 2 <= SelectionLimits.max_bytes` (default = pigment `MAX_BYTES`, 16 MiB) before allocating. `to_mask_bytes(format)` emits `coverage_u8` / `coverage_u16le` bytes exactly as `Mask::validate` (`tile.rs:198`) accepts, so a selection feeds `MaskInput` and `masked_replace` with no conversion.
2. `CombineMode { Replace = 1, Extend = 2, Diminish = 3, Intersect = 4 }` (`#[repr(u8)]`, STU-RAS-118 discriminants, `from_u8` refuses others). `combine(base: Option<&CoveragePlane>, new: &CoveragePlane, mode) -> CoveragePlane`: per pixel on the union (Extend) or base bounds; Studio formulas: extend `a + b - a*b`, diminish `a * (1 - b)`, intersect `a * b`, replace `b`; computed in u32 fixed point with round-half-up, documented as Studio readings (Photoshop formula UNVERIFIED -> D5 oracle list).
3. Shape producers, all taking `anti_alias: bool` (true = 4x4 pixel-centred samples, the render-cpu rule; false = centre sample only) and returning a plane clipped to a caller `canvas: Rect`:
   - `rect(r: RectF)`, `ellipse(r: RectF)` (f64 inputs, finite-checked), single row/column = `rect` with height/width 1.
   - `polygon(points: &[[f64; 2]], rule: FillRule)` for polygonal and freehand lasso (freehand = many points). `FillRule` is explicit (`NonZero | EvenOdd`); default `EvenOdd` is a Studio reading, Photoshop self-intersection rule UNVERIFIED (D5 oracle case). Scanline crossing count per sample (f64, no allocation per sample).
4. `feather(plane, radius_px)` reuses `coverage_math::feather_kernel` (separable; margin grows bounds by `feather_support`); radius > 10000 refuses with `CoverageError::FeatherOutOfRange` (existing code).
5. `colour_range(planes: &TilePlanes-like rgb_f32le view, sample: [f32; 3], fuzziness: f32)`: coverage = `clamp(1 - d / f, 0, 1)` with `d` = Euclidean distance in the plane's encoded RGB; formula is a Studio reading (Photoshop fuzziness UNVERIFIED, class Approximate, D5). Input view = the same `ResolvedTile` colour bytes pigment already validates (`tile.rs:77`); alpha 0 pixels get coverage 0.
6. `invert(plane, canvas)`.
7. `DESCRIPTOR` addition: a `selection` entry in pigment's DESCRIPTOR JSON (operations, limits, Studio readings, NOT_PROVEN parity).

## Non-goals (slice 1)
- Magnetic lasso, quick/selection brush (needs SMB-PIGMENT-BRUSH), flood/magic-wand select (slice 2: contiguous fill over the same plane, cheap), object/subject/sky select (on-device model; STU-RAS-019), tonal/luminosity select, Select-and-Mask surface (STU-RAS-121/122), modify ops of STU-RAS-120 (slice 2), marquee constraint modes of STU-RAS-119 (tool layer, needs UI/controls), vector-object selection.
- No Folio record, no package change, no ports/leases (pure compute; the patch path stays `masked_replace`).
- No render-cpu change.

## Slice 2 (same lane id only if slice 1 PASSes; else new lane id per V4-15)
- STU-RAS-120 modify ops (grow/shrink = morphological distance on coverage >= 0.5, smooth, border, expand/contract; no clamping), flood select (tolerance, contiguous, sample-all-layers via caller-composited plane).
## Slice 3 (Folio owner, separate lane)
- `StudioSelectionSet` record `hsk.studio.selection_set@1` (STU-CON-002 schema id) in folio: coverage tiles (`TileRef` with `coverage_u16le`, same as `StudioMask.tiles`) + optional vector-object membership; save-to-channel = the same tiles (STU-RAS-022 one mechanism).

</topic>

<topic id="native" status="active" summary="Relation to the native format">

## Native format (package / Folio graph)
- Slice 1 stores nothing: a live selection is transient session state.
- Persisted forms reuse what already exists: a selection applied as a layer mask becomes `StudioMask.tiles` (coverage `TileRef`s, `appearance.rs:49`) written through the existing tile/patch path; save-selection-to-channel becomes the slice-3 `StudioSelectionSet` record with the same coverage tile encoding. The package stores those tiles as ordinary tile assets; no new package entry kind.

</topic>

<topic id="ui" status="active" summary="UI dependency">

## UI
- None. Engine API only; marquee drag, marching ants and modifier keys are tool-layer work for the GUI host (open decision D11). The engine already takes the typed `CombineMode` argument STU-RAS-118 requires next to the modifier-key affordance.

</topic>

<topic id="deps" status="active" summary="Edges and crates.io">

## Dependencies
- No new Studio edge (pigment: accord, observe, prism, folio already allowed; selection needs none of prism/folio).
- No crates.io dependency (pure std).

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (at most 5 focused tests, `tests/selection.rs`)
1. **Hand values.** `rect` with half-pixel edges at 4x4 AA gives exactly 32768/65535-rounded coverage on edge pixels and 65535 inside; AA off gives 0/65535 only. `ellipse` total coverage sum is within 1 % of pi*a*b for a 40x20 ellipse.
2. **Combine table.** For coverages {0, 0.25, 0.5, 1} x {0, 0.5, 1} each `CombineMode` matches the documented formula within 1 LSB; discriminants 1..=4 round-trip, 0 and 5 refuse.
3. **Polygon.** A self-intersecting bow-tie gives different coverage under `EvenOdd` and `NonZero` exactly at the overlap; a closed square polygon equals `rect` on the same square.
4. **Interop with existing consumers.** `to_mask_bytes(coverage_u16le)` validates through `Mask::validate` and is accepted as `MaskInput` by `RenderPlan::validate`; feathering a hard rect equals separable application of `feather_kernel` (reference computed in the test).
5. **Hostile (V4-02).** `dims_u32_max`: a `canvas` of `u32::MAX x u32::MAX` refuses with `Overflow`/`BudgetExceeded` before allocation; `aggregate_over_budget`: polygon with 10^7 points refuses at the point-count cap (`SelectionLimits.max_points`, default 1_000_000) before work; NaN/inf coordinates and fuzziness refuse with `Nonfinite`; `feather(10001)` refuses.

## Proof commands (WORKSPACE_RULES cargo-form)
- `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-pigment --tests`
- same prefix: `cargo clippy -p hsk-studio-pigment --tests`
- same prefix: `cargo nextest run -p hsk-studio-pigment --test selection`
- Dependents keep compiling: same prefix `cargo check -p hsk-studio-render-cpu -p hsk-studio-composite --tests` (pigment is a shared engine; additive change only).

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Photoshop formulas unknown** (combine maths, fuzziness, lasso fill rule, AA sample count). Mitigation: every formula is a named Studio reading in DESCRIPTOR, class Approximate; add cases to the D5 oracle list (marquee feather 10 px, bow-tie lasso, colour range fuzziness 40).
- **Second coverage rasterizer next to render-cpu.** Mitigation: same sampling rule (pixel-centred N x N); record the consolidation candidate for the lead instead of silently duplicating; no curve support here (straight segments only).
- **Memory on huge canvases.** A full 30000 x 30000 selection is 1.8 GB at u16. Slice 1 refuses above the budget; tiled `CoveragePlane` storage (per pigment `Grid`) is the slice-2 answer when a real document needs it.
- **Shared crate.** pigment is consumed by render-cpu, render-gpu, composite, develop, interop-psd: additive module only; dependents' check in the proof list.

</topic>

<topic id="cohesion" status="active" summary="How it serves several apps">

## Cohesion
- Photoshop: marquee/lasso/colour range, quick mask (STU-RAS-021) and save-to-channel use one plane type.
- Lightroom Classic: develop local masks (STU-RAW-014a/014b) use the same combine set and coverage planes; develop can call it through its existing pigment edge (no fork, as STU-RAW-014b demands).
- Illustrator/InDesign/Figma: effect scoping to a selection (STU-FX-012a) and raster effects on vector art consume the same `MaskInput`.
- After Effects: compositing masks (STU-CMP-010 step 2) are coverage planes of the same kind once composite lowers them.

</topic>
