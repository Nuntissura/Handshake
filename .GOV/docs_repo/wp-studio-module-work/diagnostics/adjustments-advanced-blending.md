---
file_id: WP-STUDIO-MODULE-WORK-DIAG-ADJUSTMENTS-ADVANCED-BLENDING
file_kind: architecture_decision
updated_at: 2026-10-11T05:14:30Z
wp: WP-KERNEL-STUDIO
lane: SMB-PIGMENT-ENG-2 / SMB-FOLIO-ENG-5
owner: pigment-folio-architect
crates: [hsk-studio-pigment, hsk-studio-folio, hsk-studio-composite, hsk-studio-render-cpu, hsk-studio-render-gpu, hsk-studio-interop-psd]
status: decided
base_commit: e09a491 (codex/handshake-studio-assets)
prior_decision: diagnostics/render-plan-ir.md
---

# Adjustment layers, mask refinement, Blend-If, knockout and layer effects

Prose surface (context, not authority). Spec authority: `.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`.

<topic id="problem" status="active" summary="Why PSDs display wrong today">

## Problem

PSD display fidelity is the weakest point of the Craft rewrites (`research/craft-case-study.md` topic psd-display-issue: Craft re-composites layers with fitted maths; Advanced Blending still fails). Studio has the same gaps today (verified at e09a491):
- composite `DESCRIPTOR` lists effect, knockout, blend_if and mask density/feather/invert as unsupported.
- interop-psd `loss.rs` classifies adjustment layers as `adjustment_layer_not_applied`, and `lfx2`/`lrFX` as `layer_effects_not_rendered`.
- folio `appearance.rs` and `paint.rs` state that mask density/feather, Blend-If, knockout and effect stacks are not stored.
- render-cpu and render-gpu refuse `MaskInput.density != 1` (`render-cpu/src/plan_sink.rs:459`, `render-gpu/src/plan_sink.rs:315`).

</topic>

<topic id="additivity" status="active" summary="Hard constraint: the v1 IR and folio structs are struct-literal built and exhaustively matched elsewhere">

## Additivity constraint (evidence)

Adding fields or variants to these types would break crates this lane may not edit:
- `RenderPlan {..}`: composite `lower_render_plan.rs:452`, render-cpu `examples/render-cpu-consumer.rs:60`, `tests/cpu_plan.rs:56`.
- `Layer {..}` / `MaskInput {..}`: composite `lower_render_plan.rs:124,153`, render-cpu `tests/cpu_plan.rs:64`, example `:53`.
- `Operator` is matched exhaustively, with no wildcard, in render-cpu `plan_sink.rs:1311-1332`. `PlanNode` is matched in render-cpu `plan_sink.rs:375-411`. `BackendError` is matched exhaustively in render-cpu `report.rs:10-21`.
- folio `StudioLayer {..}`, `LayerAppearance {..}`, `StudioMask {..}` and `StudioDocument {..}` are built by struct literals in about 20 sites (components, layout, interop-vector, interop-psd, interop-layout, package, session, pulse, chronicle). `StudioNodeDesign {..}` is built in components tests and in interop-vector.

Safe extension points: `PlanError` (only ever matched as `InvalidPlan(_)` or by a single variant; ENG-1 already added `ClipBasePassThrough`), `MaskFormat` (`#[non_exhaustive]`), `StackNode` (no use outside pigment), new trait methods with default bodies on `RenderBackend`, and new types.

Decision: the new IR features travel in an additive envelope, `render_plan::PlanExtensions`, next to an unchanged v1 `RenderPlan`. The new authoring data lives in a sidecar record set, `folio::StudioRasterRecords`, next to an unchanged `StudioDocument`. This follows the existing `StudioDesignRecords` precedent (`folio/src/records.rs:1-5`). A future coordinated IR v2 (`#[non_exhaustive]` plus constructors, migrated by composite, render-cpu and render-gpu in one round) may fold the envelope in. Until then, the envelope is the only place these features exist.

</topic>

<topic id="owners" status="active" summary="Single owners (CX-CAP-001)">

## Single owners

| Capability | Owner | Consumers |
|---|---|---|
| Adjustment authoring records (`StudioAdjustment` first set, STU-RAS-151) | `hsk_studio_folio` module `raster` (`StudioRasterRecords`) | interop-psd (writes), composite (reads) |
| Mask refinement authoring (`StudioMask.density/feather`, invert; STU-RAS-115/124) | same sidecar, per-layer `StudioMaskParameters` | same |
| Advanced blending authoring (Blend-If 8 values per channel STU-RAS-159; knockout STU-VEC-153 four-member enum; interior/clipped/transparency-shapes toggles STU-RAS-041) | same sidecar, per-layer `StudioAdvancedBlending` | same |
| `StudioEffectStack` authoring (STU-RAS-040/156-158) | same sidecar (`SEFX` id) | same; rendering is deferred |
| Per-pixel adjustment maths + adjustment-layer compositing rule | `hsk_studio_pigment::adjust_math` (new) | render-cpu, render-gpu (WGSL tested against it), composite (classes) |
| Mask density/invert coverage, mask feather kernel, Blend-If weight | `hsk_studio_pigment::coverage_math` (new) | render-cpu, render-gpu |
| IR for all of the above + validation + admission | `hsk_studio_pigment::render_plan` (`PlanExtensions`, `ExtOperator`, `admit_extended`) | composite (builds), backends (implement) |
| Knockout semantics | `render_plan` (validated, refused until implemented) | - |
| Effect rendering (later) | pigment, with effect nodes in the envelope (design: `builders/briefs/lane-effects-wave-1.md`) | backends |
| Lowering of records to IR | composite | - |
| PSD bytes to records | interop-psd | - |

Pigment is the owner for the same reasons as `render-plan-ir.md` topic decision: it is the lowest module that composite, render-cpu, render-gpu, interop-psd and develop can all reach. Folio is the owner of authored state, because only folio may hold document records (folio depends on accord only).

</topic>

<topic id="curves-owner" status="active" summary="develop Curve and Photoshop curves adjustment are distinct capabilities">

## develop `Curve` versus the raster curves adjustment: two capabilities, two owners

Evidence:
1. `develop/src/curves.rs:1-40` defines a monotone Hermite curve. It requires endpoints exactly (0,0) and (255,255) and non-decreasing y, and caps at 32 points. It implements the STU-CON-044 `resolved_curves` of the develop pipeline.
2. The raster curves adjustment (STU-RAS-151 `curves`) is a Photoshop curve. Its points may decrease and its endpoints may move. The installed Photoshop 2026 preset `Presets/Curves/Negative (RGB).acv` decodes, as big-endian u16 (output, input) pairs per the PSD spec ACV layout, to version 4, 5 curves, composite `(255,0),(0,255)` = input 0 -> 255 and input 255 -> 0. That is a decreasing curve, which develop's `Curve::new` rejects. `Strong Contrast (RGB).acv` has 5 points. The fifth reserved curve matches STU-RAS-151 "composite + R + G + B + a fifth reserved".
3. The interpolation differs: monotone Hermite (no overshoot) in develop, and a smooth spline in Photoshop (see topic exactness).

Decision: these are distinct capabilities (different clauses, domains and maths), so CX-CAP-001 is not violated. `pigment::adjust_math` owns the raster curve. Develop may later consume `adjust_math` for a Photoshop-style curve (develop -> pigment is an allowed edge), but must never copy it.

</topic>

<topic id="ir-shapes" status="active" summary="Envelope types in pigment render_plan">

## IR additions (`hsk_studio_pigment::render_plan`)

```rust
pub struct PlanExtensions { pub layers: Vec<LayerExtension> }          // empty = plain v1 plan
pub struct LayerExtension {
    pub group: PlanNodeId,          // a PlanNode::Group
    pub layer: u32,                 // index into that group's bottom-first layers
    pub adjustment: Option<AdjustmentOp>, // Some = adjustment layer (see rule below)
    pub blend_if: Vec<BlendRangeEntry>,   // empty = identity
    pub mask_feather_px: f32,       // 0 = none; needs a coverage mask on the layer
    pub knockout: Knockout,         // None | Shallow | Deep; anything but None is refused
}
pub struct BlendRangeEntry { pub channel: BlendChannel /* Gray|Red|Green|Blue */,
    pub source: [f32; 4], pub destination: [f32; 4] } // [black_start, black_end, white_start, white_end], level/255
pub enum ExtOperator { AdjustmentLayer, Adjustment(AdjustmentKind), BlendIf, MaskFeather, MaskDensity, Knockout(Knockout) }
trait RenderBackend { /* default methods, additive */
    fn extension_exactness(&self, op: ExtOperator) -> Exactness { Exactness::Unsupported }
    fn execute_extended(&self, plan, ext, request, sources, cancel, sink) -> Result<RenderReceipt, BackendError>
        // default: ext empty -> execute(plan); else admit_extended (refuses for a backend without support)
}
pub fn admit_extended(plan, ext, backend, request, cancel) -> Result<ExactnessLedger, BackendError>;
impl PlanExtensions { fn validate(&self, plan) -> Result<Vec<ExtOperator>, PlanError>; fn digest_with(plan) }
```

Rules:
- **Adjustment layer.** `Layer.input` must be the canonical placeholder `PlanNode::Solid { rect: <extent>, rgba: [0;4] }`. The rect bounds where the adjustment applies. The layer's content is `f(backdrop)`, where the backdrop is the group's running result below this layer: from a transparent start in an isolated group, or from the parent backdrop in a pass-through group. Inside a clip-to-base run (`MaskFormat::BaseAlpha`), the backdrop is the run unit G, so a clipped adjustment affects only its base, as in Photoshop. Per pixel: `k = opacity * fill_opacity * mask * blend_if`, `C = lerp(Cb, B_mode(Cb, f(Cb)), k)`, alpha unchanged (`adjust_math::composite_adjustment`). Keeping alpha unchanged is a Studio rule, Approximate, D5 oracle. An adjustment layer cannot be a clip base (`PlanError::AdjustmentClipBase`).
- **Blend-If.** The weight is the product over entries of `w_src(channel of source) * w_dst(channel of backdrop)`, with `w = ramp_up(v; bs, be) * ramp_down(v; ws, we)` (`coverage_math::blend_if_weight`). It multiplies the source alpha. Gray is `blend_math::lum` (0.3/0.59/0.11; UNVERIFIED for Photoshop). Blend-If is refused on a pass-through layer.
- **Mask density/invert.** These stay in v1 `MaskInput`. Coverage is `m' = inverted ? 1-m : m`, then `1 - density * (1 - m')` (`coverage_math::mask_coverage`). When density != 1, `validate` reports `ExtOperator::MaskDensity` only through the extended path. The v1 path keeps today's backend refusal.
- **Mask feather.** A Gaussian blur of the coverage plane before density, with sigma = `coverage_math::feather_sigma(px)`. Spatial: backends need a support margin of `ceil(3 sigma)` px.
- **Knockout.** Not `None` -> `PlanError::KnockoutUnsupported`. At the port it maps to `BackendError::Unsupported { operator: "knockout_shallow"|"knockout_deep" }`.

</topic>

<topic id="exactness" status="active" summary="Exactness per operator (same scheme as blend_math)">

## Exactness per operator

| Operator | Class | Basis |
|---|---|---|
| `adjustment_layer` (alpha-preserving lerp rule) | Approximate | Studio rule; Photoshop behaviour on semi-transparent backdrops UNVERIFIED -> D5 |
| `invert` | Exact | `1 - x` per channel; Adobe help example 255->0, 5->250 (8-bit) |
| `levels` | Approximate | GIMP levels formula (`(x-ib)/(iw-ib)` clamp, `^(1/gamma)`, output lerp); Photoshop order of composite/channel records UNVERIFIED |
| `curves` | Approximate | spline through points; Photoshop interpolation not published |
| `hue_saturation` (master only) | Approximate | GIMP HSL model; per-band colour ranges Unsupported in this slice |
| `brightness_contrast` legacy and modern | Approximate | GIMP formulas; Photoshop modern curve not published |
| `exposure` | Approximate | `(x * 2^ev + offset)^(1/gamma)` applied to the numbers given; Photoshop linearisation for 8/16-bit UNVERIFIED |
| `threshold` | Approximate | `lum >= level`; Photoshop brightness metric UNVERIFIED |
| `posterize` | Approximate | GIMP-style rounding; Photoshop mapping UNVERIFIED |
| `blend_if` | Approximate | linear split-slider ramps; gray metric UNVERIFIED |
| `mask_density` | Approximate | linear density reading of Adobe help text |
| `mask_feather` | Approximate | Gaussian; radius-to-sigma relation UNVERIFIED |
| `knockout_*` | Unsupported | not implemented |

Formula sources are cited in each function's doc comment in `pigment/src/adjust_math.rs` and `coverage_math.rs`, and in topic sources.

</topic>

<topic id="folio-records" status="active" summary="Authoring records in folio::raster">

## Folio records (`hsk_studio_folio::StudioRasterRecords`, schema `hsk.studio.raster_records@1`)

- One set annotates one document (`document_id`). `layers: Vec<StudioRasterLayerRecord>` holds at most one entry per `SLYR` layer.
- `StudioRasterLayerRecord { layer_id, adjustment: Option<StudioAdjustment>, mask: Option<StudioMaskParameters>, advanced_blending: Option<StudioAdvancedBlending>, effects: Option<StudioEffectStack> }`.
- An adjustment layer is `StudioLayer { kind: "adjustment", payload: PrimitiveReference { schema_id: "hsk.studio.adjustment@1", object_id: <its own SLYR id> } }`. Accord defines no identity prefix for `hsk.studio.adjustment@1`, so the adjustment is keyed by its hosting layer (STU-RAS-007: one adjustment per adjustment layer). The validator checks both directions.
- `StudioAdjustment` is a tagged enum over the first set, using STU-RAS-151 names and canonical scales (STU-RAS-152: levels `normalized_0_1`, curve points `levels_0_255`): `levels`, `curves`, `hue_saturation`, `brightness_contrast`, `exposure`, `invert`, `threshold`, `posterize`.
- `StudioMaskParameters { density: percent 0..100, feather: px 0..10000, inverted }` (STU-RAS-115/124). It requires the layer's `appearance.mask`.
- `StudioAdvancedBlending { blend_ranges: Vec<StudioBlendRange{channel, source_black, source_white, destination_black, destination_white}>, knockout: StudioKnockout{unknown=-1,disabled=0,enabled=1,inherited=2}, blend_interior_effects, blend_clipped_effects, transparency_shapes_layer, channel_blend: {red, green, blue} }`. Each pair is `[start, end]` in levels_0_255; the defaults are the identity.
- `StudioEffectStack { effect_stack_id: SEFX, enabled, scale (percent), effects: ordered Vec<StudioEffect> }` (multi-instance; at least 10 instances per kind). `StudioEffect { instance_id, enabled, visible_in_editor, common block (STU-RAS-157), kind block }` covers all ten kinds; `StudioContour { name, points[{input, output, corner}] }` (STU-RAS-158). Gradient and pattern contents of `gradient_overlay`/`pattern_overlay`/stroke are references (raster `StudioGradient`/`StudioPattern` are declared gaps STU-RAS-160).
- `validate_raster_records(doc, records, token)` returns typed `Diagnostic` codes and enforces the spec ranges.

</topic>

<topic id="deferred" status="active" summary="Not in this lane">

## Deferred
- Effects rendering (drop/inner shadow, outer/inner glow, bevel/emboss, satin, colour/gradient/pattern overlay, stroke). Effects wave 1 is drop shadow, stroke and colour overlay (brief `lane-effects-wave-1.md`). Global Light document field: folio stores the per-effect `use_global_light` and angle; the document-level pair waits on a document-level record decision (StudioDocument cannot gain fields).
- Hue/Saturation colour bands. Adjustment kinds beyond the first eight.
- Knockout shallow/deep rendering. Interior/clipped-effects toggles only matter once effects render.
- Vector-mask density/feather (PSD keeps them separate; the spec has one mask parameter set).
- HDR/f64 adjustment variants. Exposure linearisation through prism.

</topic>

<topic id="oracle-d5" status="active" summary="Cases added to the D5 Photoshop oracle list">

## D5 oracle additions (Operator-approved foreground session; never driven by agents)
1. Adjustment layer over a semi-transparent backdrop in a Normal group: is alpha preserved? Repeat with a non-Normal adjustment mode.
2. Levels: composite and red records both set. Which applies first? Also a gamma ramp at 8/16-bit.
3. Curves: `Strong Contrast (RGB)` and a 3-point curve with overshoot, on a gray ramp. Which interpolation, and is the output clamped?
4. Hue/Saturation master: hue +60, saturation +/-50, lightness +/-50, colorize, on the 17^3 lattice.
5. Brightness/Contrast in modern and legacy mode: brightness +/-50, contrast +/-50, on a gray ramp.
6. Exposure +1 EV, offset 0.1, gamma 1.5 at 8-bit and 32-bit. Is it computed in linear light?
7. Threshold at level 128 on a colour lattice. Which brightness metric, and is the boundary inclusive?
8. Posterize at 2, 4 and 255 levels on a gray ramp.
9. Blend-If split sliders: This Layer black (50,100) and white (200,240), and Underlying gray/red, on ramps. Gray metric, ramp shape.
10. Mask density 50% and feather 2/10/50 px on a hard-edged mask. Density formula, and the feather radius-to-sigma relation.
11. Knockout shallow and deep in nested groups (later implementation).

</topic>

<topic id="spec-gaps" status="active" summary="For governed enrichment; not resolved here">

## Spec gaps
- Photoshop knockout is None/Shallow/Deep (raster advanced blending), but the only enumerated knockout in the spec is the STU-VEC-153 four-member `{unknown, disabled, enabled, inherited}`. Folio stores the spec enum. The raster depth (shallow/deep) has no spec field: interop-psd keeps the raw PSD value in preservation until enrichment.
- STU-RAS-159 does not say whether `start <= end` must hold across the black and white pairs. Only the order within each pair is enforced.
- No document-level Global Light field exists on `StudioDocument` (STU-RAS-158 requires one).
- No identity prefix exists for `hsk.studio.adjustment@1` (accord `SchemaId::identity_prefix`). The adjustment is keyed by its layer.

</topic>

<topic id="sources" status="active" summary="Sources">

## Sources
- Spec v02.215 14.4: STU-RAS-007, 035, 040, 041, 113, 115, 124, 151-159; STU-VEC-153.
- Local evidence: product files and lines cited above; Photoshop 2026 `Presets/Curves/*.acv` decoded read-only (no copy into assets).
- Research reports by two online researcher agents (2026-10-11), with the cited URLs folded into the code doc comments: Adobe PSD file format spec, GIMP/GEGL operations, psd-tools, ag-psd, Krita.

</topic>
