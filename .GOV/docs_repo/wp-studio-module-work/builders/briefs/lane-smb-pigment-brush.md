---
file_id: BUILDER-BRIEF-LANE-SMB-PIGMENT-BRUSH
file_kind: builder_brief
updated_at: 2026-10-14T19:06:53Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-pigment
lane: SMB-PIGMENT-BRUSH
owned_paths: [crates/hsk-studio-pigment/src/brush.rs, crates/hsk-studio-pigment/src/brush/**, crates/hsk-studio-pigment/tests/brush.rs, one `pub mod brush;` line in crates/hsk-studio-pigment/src/lib.rs]
depends_on: [SMB-PIGMENT-SELECTION slice 1 pushed (CoveragePlane, CombineMode); same crate, so run after it, not in parallel]
author_role: brief writer (parity follow-up 2026-10-14); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 9a2c840
parity_refs: [PS-15, PS-17, LR-20 (brush mask), STU-RAS-021 quick mask, top_gap photoshop#2]
---

<topic id="finding" status="active" summary="What exists today">

## Finding (verified in code)
- Only the paint APPLICATION primitive exists: `masked_patch::masked_replace(request, port, cancel)` (`src/masked_patch.rs:357`) blends a `before` tile toward a `replacement` tile by a coverage `Mask` (`Request{grid, rectangle, mask, tiles, ...}`, `:150-165`), binary64 alpha-aware, with revision/epoch/lease/finalization ports and an immutable preimage (DESCRIPTOR `math`, `src/lib.rs:90`). Pigment DESCRIPTOR lists "richer masks/brushes/blends" as pending.
- No tip, dab, spacing, dynamics, stroke or eraser code (parity PS-15 PARTIAL, PS-17 MISSING; grep `brush|dab|eraser` in crates/: none outside comments and develop's recipe flag `hsk-studio-develop/src/recipe.rs:29`).
- Reusable: `Rect`/`Grid` checked algebra (`src/tile.rs:9-75`), `Mask` coverage formats (`src/tile.rs:190`), the selection lane's `CoveragePlane` + `CombineMode`, `blend_math` single owner of per-pixel blending (`src/blend_math.rs:503 blend_rgb`, `:618 composite_over`) for a later non-Normal brush mode.

</topic>

<topic id="ownership" status="active" summary="Owner per CX-CAP-001">

## Owner
- **hsk-studio-pigment** ("masks, brush/raster operations"). STU-RAS-125 requires exactly ONE engine for painting, retouching, erasers, mask painting, selection brush, refinement brush and develop-time local brush: develop reaches it through its existing pigment edge; selection brush/quick mask through `CombineMode`; layer paint through `masked_replace`.
- The engine produces data (dab list) and coverage; it never publishes document state itself (publication stays in `masked_replace`'s ports).

</topic>

<topic id="spec-basis" status="active" summary="Clauses (spec v02.216 module 14)">

## spec_basis
- STU-RAS-125 (l.834): "Studio MUST implement exactly one native brush engine ... driving every painting tool, every retouching tool, every eraser, every mask-painting surface, every selection brush, every refinement brush, and every develop-time local brush."
- STU-RAS-126 (l.847): `StudioBrushTip` with `tip_kind` `parametric | sampled | erodible | airbrush_spray`; "the id — not the name — is the reference".
- STU-RAS-127 (l.863): `diameter` hard 1..5000 px; `hardness` 0..100 %; `angle` -180..180 default 0; `roundness` 0..100 default 100; `spacing` default 25 %, "MUST NOT be clamped to 100"; `spacing_enabled` default true.
- STU-RAS-130/131 (l.927, l.963): `StudioBrushDynamics` fields `base_value, variance, jitter, minimum, control, variance_mode, fade_step, time_limit_ms, reverse, response_curve`; driving signals closed set "`off = 0`, `fade = 1`, `pen_pressure = 2`, `pen_tilt = 3`, ... `velocity = 9`"; eighteen channels must exist in the schema; `minimum_diameter` floor field.
- STU-RAS-136 (l.1071): smoothing is "one numeric strength plus five independent booleans" (slice 2+).
- STU-RAS-028 (l.1178): eraser family "erase to transparency or background colour"; STU-RAS-170 (l.1218): on a non-destructive layer "the erase MUST be applied to that layer's `StudioMask` ... and the command receipt MUST say so".
- STU-RAW-121 (l.2525, table l.2555): develop brush mask fields `dabs` (ordered stroke list in normalized coordinates), `radius`, `flow`, `feather`; l.2558: "Brush dab lists are stroke data, not a rasterized mask: the mask is re-rendered from the dab list at the current resolution".

</topic>

<topic id="scope" status="active" summary="Slice 1 (~1-2 h)">

## Slice 1: parametric round tip, pressure, dabs, stroke coverage, paint + erase via masked_replace
New module `pigment::brush` (`#![deny(clippy::arithmetic_side_effects)]`):
1. `BrushTip { tip_kind: TipKind (only Parametric accepted; the other three members exist and refuse `tip_kind_unsupported`), diameter_px: f64 [1, 5000], hardness_pct [0, 100], angle_deg [-180, 180], roundness_pct (0, 100], spacing_pct > 0 (no upper clamp; work bounded by the dab cap), spacing_enabled: bool }`. Validation refuses out-of-range/nonfinite with typed codes; never clamps.
2. `BrushDynamics` (one record type, STU-RAS-130 field set; slice 1 evaluates only `control ∈ {Off, PenPressure}` with `minimum`; every other control value is stored and refuses at evaluation with `dynamics_control_unsupported`). Channels evaluated in slice 1: `size`, `opacity`, `flow` (the 18-channel schema is declared as an enum; unevaluated channels must be `Off`).
3. Input model: `StrokeSample { x: f64, y: f64, pressure: Option<f32> /* 0..=1 */, time_ms: Option<f64> }` in document pixels. Missing pressure = 1.0 (recorded).
4. `place_dabs(tip, dynamics, samples) -> Vec<Dab>`: first dab at the first sample; then one dab per `spacing_pct/100 * current_diameter` of arc length along the polyline, carrying the remainder across segments; pressure linearly interpolated between samples; `spacing_enabled == false` = one dab per sample. `Dab { x, y, diameter, opacity, flow, angle, roundness }` is `Serialize`/`Deserialize` (this is the STU-RAW-121 dab list).
5. `stroke_coverage(dabs, hardness, canvas: Rect) -> CoveragePlane`: dab footprint = 1 inside `r * hardness`, smoothstep falloff to 0 at `r` (elliptical by angle/roundness); accumulation `buf = buf + (1 - buf) * flow * dab` per dab (build-up), stroke coverage = `opacity * buf`. Footprint and accumulation are Studio readings (Photoshop hardness curve and flow/opacity interaction UNVERIFIED -> D5 cases); class Approximate.
6. Application helpers (no new publication path):
   - `paint_request(...)`: builds the `masked_replace` inputs: `mask` = stroke coverage (`coverage_u16le`), `replacement` = caller-resolved tile filled with the foreground colour (straight alpha 1). Normal mode only.
   - `erase_request(...)`: same with replacement alpha 0 (erase to transparency). A caller flag `target_not_pixel_editable` routes the coverage into the layer's mask instead (`CombineMode::Diminish` on the mask plane) and the returned receipt names `erase_routed_to_mask` (STU-RAS-170).
   - `paint_mask(plane, stroke, CombineMode)`: brush on a mask/quick mask/selection via the selection lane's `combine`.
7. DESCRIPTOR `brush` entry (parameters, readings, NOT_PROVEN parity, limits).

## Non-goals (slice 1)
- Sampled/erodible/airbrush tips, bristle physics, scatter, dual tip, texture, colour dynamics, wet mixing (STU-RAS-128..135); tilt/rotation/velocity/fade signals; response curves (stored, not evaluated); smoothing (STU-RAS-136); brush presets/library (STU-RAS-165/166); non-Normal brush blend modes; retouch tools (clone/heal, STU-RAS-026); background/magic eraser; history brush; real tablet input (OS/GUI host, D11); GPU path.

## Slice 2
- Tilt/velocity/fade controls + response curve evaluation, jitter with a seeded deterministic RNG, smoothing record and the `pulled_string` mode, brush blend modes through `blend_math`, sampled tip from an artifact-resolved bitmap.

</topic>

<topic id="native" status="active" summary="Relation to the native format">

## Native format
- Raster painting stores no brush data: the result is new tile revisions (`TileRef` with a new content digest) via `masked_replace`'s accepted patch, i.e. the existing package tile assets.
- Develop brush masks store the DAB LIST, not pixels (STU-RAW-121; failure class RAW-MASK-001 "a brush mask is stored rasterized rather than as its dab list"): the `Dab` serde shape is designed to be that record. The Folio/develop record that holds it belongs to the develop/folio owner (later lane); this lane only guarantees a stable, normalized-coordinate-convertible dab list.
- Brush tips and presets are Loom/asset-library items (STU-UNI-003), not package-private (not in this slice).

</topic>

<topic id="ui" status="active" summary="UI dependency">

## UI
- None now. Pointer/tablet capture, cursor outline and live stroke preview need the GUI host (D11). The `StrokeSample` model is the contract that host will feed; tests and models drive it headlessly.

</topic>

<topic id="deps" status="active" summary="Edges and crates.io">

## Dependencies
- No new Studio edge, no crates.io dependency.

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (at most 5 focused tests, `tests/brush.rs`)
1. **Spacing.** A straight 100 px stroke, diameter 20, spacing 25 % gives exactly 21 dabs at x = 0, 5, ..., 100; spacing 190 % is accepted (no clamp) and gives dabs every 38 px; a two-segment polyline carries the remainder (dab positions hand-computed).
2. **Pressure.** `size` channel with `control = pen_pressure`, `minimum = 20`: pressure 0 -> diameter 4, pressure 1 -> 20, linear between samples; `control = pen_tilt` refuses `dynamics_control_unsupported`.
3. **Coverage hand values.** Hardness 100 single dab: centre 65535, a pixel fully outside 0; flow 0.5 two coincident dabs -> buf 0.75; opacity 0.5 caps the stroke at 0.375 for that case (documented reading).
4. **Paint and erase through the existing path.** `paint_request` + `masked_replace` on a 64x64 test tile (reuse the fixture pattern of `tests/masked_patch.rs`: `Fixture` at :158 and the `ExecutionPort` impl at :373; test binaries do not share code, so copy only the minimal part into `tests/brush.rs`) changes only pixels under coverage > 0, and the accepted patch's preimage equals the original bytes; `erase_request` leaves colour bytes and lowers alpha by exactly the coverage; `target_not_pixel_editable` leaves the tile untouched and returns the mask route receipt.
5. **Hostile (V4-02).** `dims_u32_max` (canvas overflow refuses before allocation), `aggregate_over_budget` (a stroke whose dab count would exceed `BrushLimits.max_dabs`, default 1_000_000, refuses before placing; diameter 1 with spacing 1 % over 10^6 px is the vector), nonfinite sample coordinates/pressure refuse, diameter 5001 and hardness 101 refuse (no clamp).

## Proof commands
- `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-pigment --tests`
- same prefix: `cargo clippy -p hsk-studio-pigment --tests`
- same prefix: `cargo nextest run -p hsk-studio-pigment --test brush`
- same prefix: `cargo check -p hsk-studio-render-cpu -p hsk-studio-composite -p hsk-studio-develop --tests` (additive change in a shared engine).

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Fidelity vs Photoshop** (hardness curve, flow/opacity, spacing origin). Mitigation: Studio readings named in DESCRIPTOR, class Approximate; D5 oracle cases: one 100 px stroke at hardness 0/50/100, flow 10 %, opacity 50 %, spacing 25/190 %.
- **Whole-canvas coverage per stroke** costs memory on large documents. Mitigation: coverage plane limited to the stroke bounding box grown by the radius (not the canvas); budget check before allocation.
- **Using `masked_replace` for paint** means Normal mode only and full port ceremony per stroke. Accepted for slice 1 (single publication path, preimage/undo for free); a stroke-buffer composite with `blend_math` modes is slice 2.
- **Determinism.** f64 maths in a fixed order, no SIMD, no hash-map iteration; tests compare exact u16 values.

</topic>

<topic id="cohesion" status="active" summary="How it serves several apps">

## Cohesion
- Photoshop: brush, pencil-like hard tip (hardness 100, AA off later), eraser, quick-mask painting, selection brush all from one engine.
- Lightroom Classic: develop brush masks are the same dab list re-rendered at any resolution (STU-RAW-121).
- Illustrator blob brush and After Effects paint/roto brush (later) consume the same dabs; AE paint strokes keyframe the dab list over time via motion.
- Masks painted for any app feed the same `MaskInput` the render plan already composites.

</topic>
