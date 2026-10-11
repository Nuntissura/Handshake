---
file_id: WP-STUDIO-MODULE-WORK-DIAG-ADJUSTMENTS-ADVANCED-BLENDING
file_kind: architecture_decision
updated_at: 2026-10-11T11:25:21Z
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
| `levels` | Approximate | GIMP and psd-tools: `(x-ib)/(iw-ib)` clamped, `^(1/gamma)`, output lerp; channel record, then composite. Photoshop order UNVERIFIED |
| `curves` | Approximate | natural cubic spline, flat outside the end points, clamped; channel curve, then composite (psd-tools, Krita). Photoshop interpolation unpublished |
| `hue_saturation` (master + colorize) | Approximate | psd-tools empirical path. Hue bands are not in the IR (composite refuses them) |
| `brightness_contrast` legacy | Approximate | relayed Photoshop-team statement plus a third-party measurement; pivot 0.5 |
| `brightness_contrast` modern | Unsupported | no documented formula; typed refusal (`plan_adjustment_unsupported` → `Unsupported{brightness_contrast}`) |
| `exposure` | Approximate | psd-tools: linearise with `linearize_gamma` (1 for linear plans, 2.2 for encoded plans), `*2^ev + offset`, clamp, `^(1/gamma)`, re-encode. Photoshop linearisation UNVERIFIED |
| `threshold` | Approximate | `lum(0.3/0.59/0.11) >= level` (psd-tools); Photoshop metric UNVERIFIED |
| `posterize` | Approximate | `floor(x n 255/256)/(n-1)` (psd-tools fitted toward Photoshop; GIMP rounds instead) |
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

### Spec and local evidence
- Spec v02.215 14.4: STU-RAS-007, 035, 040, 041, 113, 115, 124, 151-159; STU-VEC-153.
- Local evidence: the product files and lines cited above. Photoshop 2026 `Presets/Curves/*.acv` was decoded read-only; nothing was copied into assets.

### Research reports (2026-10-11)
Two online researcher agents produced these reports. helpx.adobe.com returned 403, so the Adobe help text comes from search snippets and is a paraphrase.
- **Adobe PSD File Formats Specification** (https://www.adobe.com/devnet-apps/photoshop/fileformatashtml/):
  - Blending ranges: 4 bytes per range, composite gray source/destination, then per channel.
  - Mask parameters: flag bit 4; user/vector density u8 and feather f64.
  - `levl`: 29 x 5 u16, gamma x100.
  - `curv`, `hue2`, `brit`/`CgEd`, `expA` (f32 x3), `nvrt`, `thrs`/`post` (u16 + pad), `mixr`.
  - `knko`/`infx`/`clbl`/`tsly`: 1 byte + 3 pad.
  - `lfx2`: version 0 + descriptor 16.
- **psd-tools** (https://github.com/psd-tools/psd-tools, head b58704c, MIT) `src/psd_tools/composite/adjustments.py`:
  - Levels: channel LUTs then composite (`_apply_luts`, `apply_levels`).
  - Curves: scipy CubicSpline natural, flat outside, clipped.
  - Hue/Saturation master: RGB lightness, then HSL hue, then `s/(1-S+0.01)` or `s(1+S)`; colorize (`_huesaturation`, `_apply_saturation`, `_apply_lightness`, `_huesaturation_colorize`).
  - Exposure: x^2.2 linearisation.
  - Posterize: floor with a 255/256 LUT correction.
  - Threshold: `lum >= level`.
  - Legacy brightness/contrast: not implemented there.
  - `composite/composite.py`: mask density `share = density/255; shape = share*mask + (1-share)`; knockout NONE/SHALLOW/DEEP = 0/1/2 (issue 707: Photoshop deep = 2). Blend-If and feather are not applied.
- **GIMP** (gitlab.gnome.org/GNOME/gimp):
  - `app/operations/gimpoperationlevels.c`: per-channel then master.
  - `gimpoperationposterize.c`: round-based.
  - `gimpoperationthreshold.c`: max(R,G,B) by default.
  - `gimpoperationbrightnesscontrast.c`.
  - `core/gimpcurve.c`: Bezier-segment curves, not a natural spline.
- **Krita** `libs/image/kis_cubic_curve*`: natural cubic spline.
- **ag-psd** (https://github.com/Agamnentzar/ag-psd, head 04d927d):
  - `psdReader.ts:523-541`: blending ranges as 4 x u8.
  - `descriptor.ts:1155-1184`: `lfx2`/`lmfx` keys and the `*Multi` lists.
  - Reads `knko` as bool.
- **Legacy brightness/contrast**:
  - A Photoshop-team statement relayed on W3C public-fx (https://lists.w3.org/Archives/Public/public-fx/2012JanMar/0060.html): brightness adds n, contrast is linear, +100 gives 0/255, -100 gives 127, -100 with brightness +100 gives 227.
  - photocraft PR 2615: a third-party measurement, pivot 128/255.
- **Mask feather sigma**: UNVERIFIED. The sources conflict: photocraft PR 2529 uses sigma = feather; a Montagent issue uses sigma = feather/2.

</topic>

<topic id="implementation" status="active" summary="Commits and verification (2026-10-11)">

## Implementation (codex/handshake-studio-assets, pushed)
- **bd480f9**: `pigment::adjust_math` and `pigment::coverage_math`, with hand-computed oracle tests (`tests/adjust_coverage.rs`).
- **d576ec5**:
  - The `render_plan::PlanExtensions` envelope (`src/render_plan/extensions.rs`), `ExtOperator`, `admit_extended`, the `RenderBackend::extension_exactness`/`execute_extended` defaults, and `digest_with`.
  - `StackNode::Adjustment` and `BlendError::Adjustment`.
  - Formula revisions to the psd-tools/GIMP-cited sources.
  - R5 remediation: `admit` refuses a pass-through layer with fill < 1 as `group_pass_through_fill_opacity` for every backend.
- **ff92dea**: `folio::StudioRasterRecords` (`src/raster.rs`, `tests/raster_records_contract.rs`) and V4-07 doc citations.
- **2587930**: `LayerExtension`, `PlanExtensions` and `AdjustmentOp` are now `#[non_exhaustive]`, with builder methods.
- **c2bbb9b and 2f705f1** (V4-06 sweep of the R5 failure class: shared admission admitted features that every backend then refused at execute):
  - The v1 `admit` refuses `MaskInput.density < 1` as `mask_density`. The envelope admits it as `ExtOperator::MaskDensity`.
  - Shared admission refuses a `Solid` colour outside [0,1] as `sample_range_outside_unit`.
  - Both use the operator keys the backends already report, so the render-cpu and render-gpu tests stay green.
- Brightness/contrast: modern mode is a typed refusal (`AdjustError::Unsupported("brightness_contrast.modern")`, `PlanError::AdjustmentUnsupported`). No documented formula exists; psd-tools carries a reverse-engineered spline and polynomial that a later lane may evaluate against the D5 oracle.

</topic>

<topic id="blend-if-clip-roles" status="decided" version="1" wp="WP-KERNEL-STUDIO" owner="SMB-PIGMENT-BLENDIF-CLIP" summary="Blend-If on a clip base and on a clipped layer: no public definition, pigment refuses both for every backend" updated_at="2026-10-11T07:57:25Z">

## Blend-If on a clip base and on a clipped layer (decision: option b, refuse)

Problem: `evaluate_stack` has no clip node and no Blend-If input, and the clip-to-base rule (`MaskFormat::BaseAlpha`) lists only the base's mode, opacity and fill. The backends disagreed: render-cpu (7274827) applied the base layer's Blend-If ranges to the clip-run composite; render-gpu (a6ac630) refused `Unsupported{blend_if_clip_base}` and kept Blend-If on a clipped layer (parity tested only against a test-local reference).

### Evidence (research 2026-10-11; nothing guessed)

| Source | What it says | Read how |
|---|---|---|
| Spec v02.215 STU-RAS-007, 015, 035, 041, 159 (`14-studio-creative-suite.md`) | Lists Blend-If ranges (eight values per channel), the interior-effects / clipped-effects / transparency-shapes toggles, the clipping-mask form and clip-to-parent adjustment layers. No sentence combines Blend-If with a clipping run. | read locally |
| Adobe PSD File Format spec (adobe.com/devnet-apps/photoshop/fileformatashtml) | Blending-range fields only. `clbl` = "Blend clipped elements: boolean", `infx` = "Blend interior elements: boolean", `knko`, `tsly`; layer record "Clipping: 0 = base, 1 = non-base". No semantics for how ranges interact with clipping. | fetched |
| Adobe Help (helpx.adobe.com: layer-styles, revealing-layers-clipping-masks, layer-opacity-blending) | HTTP 403 to the fetch tool, so the primary text was NOT read (UNVERIFIED). Search-result summaries say only that Blend Clipped Layers as Group decides whether the base layer's blend mode applies to the whole clipping group or only to the base (default on). No Blend-If clause surfaced. | search snippets only |
| psd-tools `composite/composite.py` (main) | No `blend_if` or blending-range use anywhere. The clip run is composited into the base's colour before the base's mask, blend mode, opacity and fill; clipped layers' own coverage is discarded; `# TODO: Consider Tag.BLEND_CLIPPING_ELEMENTS`. | fetched source |
| Krita `libs/psd/psd_layer_record.cpp` (master) | Blending ranges are read into `blendingRanges` and never applied; writer: "blending ranges are not implemented yet"; the clipping field is written back "unused". | fetched source |
| GIMP `plug-ins/file-psd/psd-load.c` (master) | Blending-ranges block: "Blending ranges size ... (not imported)" with a FIXME. Clipping becomes a GIMP group (`clipping_group_type`) composited with the backdrop mode. | fetched source |
| Adobe Community bug report "Blend if stops working when there is a clipped layer on the group" (Feb 2024, community.adobe.com/bug-reports-711/...-657703) and a second thread (14974117, Nov 2024) | Users report Blend If (Underlying Layer) ceasing to work once a layer is clipped to the group; the community manager forwarded it to the product team. No rule, no fix stated. A search summary attributes to an expert the guess that the clipping group's Blend-If basis is the "cumulated appearance"; that sentence was not found in the page text read (UNVERIFIED). | fetched pages |
| ag-psd (`psdReader.ts:523-541`), Krita docs layer styles page | Ranges read as 4 x u8 only; the Krita manual page says nothing about Blend If. | earlier research / fetched |

Conclusion: no public source defines Blend-If on a clip base, Blend-If on a clipped layer, or their interaction with Blend Clipped Layers as Group. Open implementations either ignore Blend-If entirely (psd-tools, Krita, GIMP) or have no clip semantics for it; Photoshop's own behaviour is publicly reported inconsistent. Any rule written now would be a guess.

### Rule (pigment, single owner; `render_plan/extensions.rs`, tests `tests/blend_if_clip.rs`; commit 0d28c03 on codex/handshake-studio-assets)

- The role of a layer in its group comes from `Group::clip_role(index)` -> `ClipRole::{Unclipped, Base, Clipped}` (Clipped = carries `MaskInput::clip_to_base()`; Base = unclipped with a clipped layer directly above it).
- `PlanExtensions::validate` reports a non-identity Blend-If on a Base as `ExtOperator::BlendIfClipBase` and on a Clipped layer (including a clipped adjustment layer) as `ExtOperator::BlendIfClippedLayer`, INSTEAD of `ExtOperator::BlendIf`. Keys: `blend_if_clip_base`, `blend_if_clipped_layer`. Intrinsic class `Unsupported`.
- `admit_extended` therefore refuses them with `BackendError::Unsupported { operator: <key>, exactness: Unsupported }` before any sink call, for every backend, even one whose `extension_exactness` claims `Exact`. All-identity ranges are not Blend-If and are admitted anywhere.
- Unchanged: Blend-If on an unclipped layer or unclipped adjustment layer (`blend_if`, Approximate; `evaluate_stack` Leaf/Adjustment with the weight folded into the node opacity); pass-through layers (`PlanError::BlendIfPassThrough`); an adjustment layer as a clip base (`PlanError::AdjustmentClipBase`). `evaluate_stack` gets no clip node (a doc note records why).
- Exactness: not Approximate (option a was not taken): Unsupported, because there is no formula to be approximate about.
- Not modelled either: Blend Clipped Layers as Group switched OFF (`clbl = 0`). Pigment's `BaseAlpha` rule is the ON reading (run unit composited with the base's mode); folio stores the toggle (`blend_clipped_effects`). A consumer must not silently ignore `clbl = 0`; that is a separate gap.

### What each backend and consumer must now do

- render-cpu: the shared `admit_extended` already refuses both roles before `execute_extended_detailed` runs; remove the dead clip-base Blend-If application (`plan_sink.rs` `base.fx.blend_if` and the run-composite weight) (no existing render-cpu or composite test uses Blend-If on a clip role: both suites pass, 51 tests, against this change).
- render-gpu: keep the `blend_if_clip_base` check as defence in depth (same key); the clipped-layer case is now refused by shared admission, so update `ext_blend_if_parity` (`tests/gpu_plan.rs` group/clip case puts Blend-If on a clipped layer, root layer 2) and `fx_admission_and_refusals_run_without_adapter` (expects the clipped-layer Blend-If to reach the adapter).
- composite: `lower_extensions.rs` `operators()` must report `BlendIfClipBase` / `BlendIfClippedLayer` for layers whose appearance is a clip role (or refuse in lowering like `REFUSE_EFFECT_ON_CLIPPED`) so its ledger matches the backends' refusal.
- interop-psd: a PSD whose clip base or clipped layer carries non-identity ranges needs a typed loss entry (the owner picks the code); the ranges stay preserved opaque.

### D5 oracle cases added (Operator-approved foreground session; never driven by agents)

18. Clip base with Blend-If This Layer (gray 50/100 split, white 200/240) over a gradient backdrop, one Multiply clipped layer, Blend Clipped Layers as Group ON and OFF: does the ramp weigh the base only, the run composite, or each clipped layer?
19. Clip base with Blend-If Underlying Layer (gray split): does it read the parent backdrop below the base, and are clipped layers' pixels part of what it reads?
20. Clipped layer (Normal) with This Layer ranges: weighed against its own colour; applied before or after the multiplication by the base alpha?
21. Clipped layer with Underlying Layer ranges: does "underlying" mean the base / run unit or the parent backdrop? Move a bright layer below the base to tell them apart.
22. Clipped Invert adjustment layer with This Layer ranges: is the weighed colour f(run unit), as for an unclipped adjustment layer (case 9)?
23. Reproduce the reported bug (Underlying Layer ranges stop working when a layer is clipped to a group) in the validation Photoshop build, to separate a bug from a semantic.

When 18-23 are answered, the definition lands additively: a clip-run node in `evaluate_stack`, the two `ExtOperator`s change class, and the tests in `tests/blend_if_clip.rs` become the migration list.

</topic>

<topic id="effects-ops" status="decided" version="1" wp="WP-KERNEL-STUDIO" owner="SMB-PIGMENT-FX-OPS" summary="Effects + Blend-If on one layer: pigment refuses for every backend; nested effects: backends declare support, admission refuses uniformly" updated_at="2026-10-11T11:06:15Z">

## Effects with Blend-If on one layer, and nested effects (R7 review A; commits d3ce607 + fe5fde0 on codex/handshake-studio-assets)

Problem (R5 "admitted then refused" class): `PlanExtensions::validate` admitted a layer with effects AND a non-identity Blend-If as `Effect(kind)` + `BlendIf`, and a layer with effects whose content holds another layer with effects as plain `Effect(kind)`. render-cpu (`plan_sink.rs` `effects_with_blend_if`, `effects_nested`) and render-gpu (`plan_sink.rs` `effects_with_blend_if`) then refused after admission; render-gpu rendered nested effects that render-cpu refused. composite's ledger promised both as renderable.

### Evidence (research 2026-10-11; nothing guessed)

| Source | What it says | Read how |
|---|---|---|
| Spec v02.215 STU-RAS-041, STU-RAS-159, STU-FX-025 (`14-studio-creative-suite.md`) | Blend-If is a canonical layer field with eight values per channel; layer effects are `StudioLiveFilter` kinds on raster, vector, text and group layers; "Shadow/glow effects read the layer's alpha and compose behind or inside it". No sentence combines Blend-If with effects, and none says whether the alpha a group's effect reads includes the effects of the group's children. | read locally (grep of all Blend-If mentions in the spec) |
| Adobe Help (helpx.adobe.com layer-style effects and options overview) | HTTP 403 to the fetch tool: primary text NOT read (UNVERIFIED). | fetch failed |
| Web search results (Blend If with drop shadow; PSD blending ranges with layer styles) | Tutorials describe Blend If as luminance-based see-through of the layer or the underlying layer, and describe Blend Interior Effects as Group for interior effects (inner glow, satin, overlays). None states whether the ranges weigh a drop shadow or glow matte, the pre- or post-range layer alpha, or what "underlying" reads for an effect. The search summaries themselves label the interaction as unconfirmed. | search snippets only |
| Earlier evidence in topic `blend-if-clip-roles` | Adobe PSD spec: blending-range field names only; psd-tools, Krita and GIMP ignore Blend-If, so none defines it with effects either. | carried over |

Conclusion: no public source defines Blend-If composed with layer effects, and none defines the matte source of a group effect that contains effects. Any rule written now would be a guess. Lead decision (mirrors 0d28c03): refuse the combination; make the nesting a declared capability.

### Rule (pigment, single owner; `render_plan/extensions.rs`, tests `tests/effects_ops.rs`)

- `ExtOperator::EffectsWithBlendIf`, key `effects_with_blend_if`, intrinsic `Unsupported`. `validate` reports it INSTEAD of `ExtOperator::BlendIf` when a layer has a non-empty effect stack and a non-identity Blend-If (effects are refused on clip roles earlier, so only unclipped layers reach it). Order is first-use: `[Effect(kind).., EffectsWithBlendIf]`. `admit_extended` therefore refuses it with `BackendError::Unsupported { operator: "effects_with_blend_if", exactness: Unsupported }` before any sink call for every backend, even one whose `extension_exactness` claims Exact. Identity ranges are not Blend-If; effects and Blend-If on different layers are unaffected (`effect` + `blend_if`).
- `ExtOperator::EffectsNested`, key `effects_nested`, intrinsic `Approximate` (`EFFECT_EXACTNESS`, classed like the effects). `validate` reports it once per plan for any layer with effects whose content node (an EARLIER node; later or equal indexes are never followed) is a group holding a layer with effects, directly or through nested group layers (same definition as render-cpu's `has_effects[input]`). A sibling layer with effects, or an effect layer whose content holds none, is not nested. A backend declares support through `extension_exactness(EffectsNested)`: Unsupported -> `admit_extended` refuses as `effects_nested` before any sink call; Approximate (or Fitted) -> admitted and recorded at Approximate (a backend cannot claim better than pigment's class).
- Pigment defines no nested-effects evaluator: `render_layer_reference` renders ONE layer's effect stack over a content plane; nesting is a composition of those calls (render-gpu's parity test composes them). That is why the class is Approximate and the capability is declared per backend rather than refused by pigment.
- Additive: two `#[non_exhaustive]` variants; existing keys, classes and admission order unchanged. A backend whose `extension_exactness` ends in a wildcard arm reports both new operators Unsupported until it names them (see below).

### What each backend and consumer must now do

- render-cpu: no behaviour change at admission (its wildcard already declares `EffectsNested` Unsupported; `EffectsWithBlendIf` is intrinsic). Name `ExtOperator::EffectsNested => Exactness::Unsupported` explicitly in `extension_exactness` (`plan_sink.rs` ~1698-1720) so the declaration is visible. Keep the local `effects_with_blend_if` (`plan_sink.rs` ~582) and `effects_nested` (~583, `plan_sink/effects.rs` ~128 run-time guard) checks only as defence in depth under `ExtOperator::EffectsWithBlendIf.key()` / `ExtOperator::EffectsNested.key()`; the tests `cpu_effects.rs` ~534 and ~551 keep their keys but are now reached through shared admission. Descriptor text: say admission, not a local refusal.
- render-gpu: MUST add `ExtOperator::EffectsNested => true` to the supported arm of `extension_exactness` (`plan_sink.rs` ~1115-1131); its wildcard `_ => false` otherwise reports nesting Unsupported and `fx_stack_mask_group_and_nesting_parity` (`tests/gpu_plan.rs` ~4502-4577) would be refused at admission. Keep the local `effects_with_blend_if` check (~606, test `gpu_plan.rs` ~4794) as defence in depth under `ExtOperator::EffectsWithBlendIf.key()`; admission now refuses it first.
- composite: `lower_extensions.rs` `blend_if_operator(role)` (~601) must return `EffectsWithBlendIf` instead of `BlendIf` for an unclipped layer that has effects (or lowering refuses such a layer), and the evaluation ledger must record `EffectsNested` for a layer with effects whose content group holds a layer with effects, so its ledger equals pigment's `validate` (assert against `PlanExtensions::validate` on the lowered plan). Descriptor `unsupported` list: add "effects together with Blend-If on one layer (effects_with_blend_if)".
- interop-psd: a PSD layer with both a layer-effects block and non-identity blend ranges needs a typed loss entry like the clip-role one (`proposal.rs` ~649-695); the owner picks the code (suggest `effects_with_blend_if`); the ranges and the effects stay preserved.

### D5 oracle cases added (Operator-approved foreground session; never driven by agents)

24. Drop shadow + Blend-If This Layer (gray 50/100 split) on a layer over a gradient backdrop: does the shadow follow the layer alpha before the ranges, after them, or ignore them? Repeat with Fill 0 and with Transparency Shapes Layer off.
25. Same layer with Underlying Layer ranges: do the ranges weigh the shadow (its own blend mode) or only the layer pixels?
26. Inner shadow / colour overlay (interior effects) + Blend-If, Blend Interior Effects as Group on and off.
27. Group with a drop shadow whose child layer has its own drop shadow: is the group matte the child's pixel alpha only or the child composite including the child's shadow?

When 24-27 are answered, the definition lands additively: `EffectsWithBlendIf` changes class and gets an evaluator in `effect_math`; `EffectsNested` keeps its class and pigment gains a nested composition the backends match; `tests/effects_ops.rs` becomes the migration list.

</topic>

<topic id="admission-parity" status="decided" version="1" wp="WP-KERNEL-STUDIO" owner="SMB-PIGMENT-ADMIT-PARITY" summary="Sweep of every refusal render-cpu and render-gpu can return after admission; three plan-level gaps closed in pigment (mask on a pass-through layer, premultiplied alpha, leaf-root Normal class), the rest classified" updated_at="2026-10-11T11:25:21Z">

## Admission parity sweep (pigment commit b2f5f81 on codex/handshake-studio-assets)

Problem (R5 pass-through fill, R7 effects and Blend-If): pigment's shared admission admitted a plan feature that both backends then refused under a local key. A host that runs `admit` as a pre-flight saw "admitted"; `execute` then said "unsupported". The lead named `mask_on_pass_through` and `alpha_premultiplied` / `sample_format` (unverified).

Method (read-only on the backends, at pigment fe5fde0 plus b2f5f81): every `unsupported(` call in `render-cpu/src` and `render-gpu/src` (regex `unsupported\(`, excluding the `refusal_keys.rs` test modules, `fn unsupported` and descriptors: 77 sites, 52 render-cpu and 25 render-gpu), the two `map_err(unsupported)` sites (render-gpu `plan_sink.rs:587`, `:596`), the direct `BackendError::Unsupported` in render-gpu `leaf_root_class` (`plan_sink.rs:1047`), and every other typed port error the backends return after admission (SourceMismatch, UnresolvedSource, BudgetExceeded, Overflow, Unavailable, DeviceLost, Canceled, DeadlineExceeded, Sink). Each was read in context and compared with what `admit` / `admit_extended` / `validate` already decide.

Classes: (a) pigment defines the semantics, so the backend must implement them and declares support through a declarable operator that admission asks; (b) no defined semantics, so pigment refuses intrinsically at admission; (c) depends on source data, an opaque payload, a resource or a device, so admission cannot know it: kept and documented; (covered) admission already decides the rule and the backend check is defence in depth; (u) unreachable after admission (internal invariant, or a value pigment's validation already bounds).

### Counts (80 `unsupported` sites)

| Class | Sites | Rules |
|---|---|---|
| (a) fixed here | 5 | `alpha_premultiplied` (tile input and target, 4 sites), leaf-root `Blend(Normal)` class (1 site) |
| (b) fixed here | 2 | `mask_on_pass_through` |
| (covered) by earlier lanes | 22 | pass-through opacity / fill, non-isolated blended group, blend mode, mask density (v1), mask format, knockout, effects with Blend-If, effects nested, Blend-If clip roles, adjustment kinds, effect features, Solid sample range, clip base pass-through |
| (c) cannot be known at admission | 13 | listed below, plus the port-error families |
| (u) unreachable defence | 38 | listed below |

### The open rules (file:line before commit b2f5f81)

| Key | Where | Condition | Class | Decision |
|---|---|---|---|---|
| `mask_on_pass_through` | render-cpu `plan_sink.rs:603-605`; render-gpu `plan_sink.rs:526-528` | a coverage mask (any invert, density, feather) on a layer with mode PassThrough | (b) | Pigment refuses intrinsically for every backend: const `PASS_THROUGH_MASK_OPERATOR`, `RenderPlan::has_pass_through_mask`, checked by `admit` and `admit_extended` |
| `alpha_premultiplied` | render-cpu `:394-396` (tile), `:447-449` (target); render-gpu `:315-317`, `:381-383` | `TileInput.alpha` or `OutputTarget.alpha` is Premultiplied on a node REACHABLE from the root (correction 2026-10-11T12:44:14Z, R8-review-a: both backends skip unreachable nodes before verify_tile and the mask checks, render-cpu plan_sink.rs:473 / render-gpu plan_sink.rs:413 @b2f5f81; b2f5f81's pigment predicates `uses_premultiplied_alpha` and `has_pass_through_mask` read every node, so plans whose only such node is unreachable were newly refused; lead decision: restrict both predicates to reachable nodes in SMB-PARITY-FOLLOWUP for exact parity) | (a) | New declarable `ExtOperator::AlphaPremultiplied` (key `alpha_premultiplied`, intrinsic `Approximate`, `ALPHA_BOUNDARY_EXACTNESS`); `RenderPlan::uses_premultiplied_alpha`; undeclared = Unsupported, which both backends already are |
| leaf-root `normal` | render-gpu `plan_sink.rs:1032-1054` (`leaf_root_class`, after `admit`) | root is a leaf (tile, solid, vector): composited as one Normal layer, so the backend class of `Blend(Normal)` must pass the request floor | (a) | `RenderPlan::has_leaf_root`; admission asks `exactness(Blend(Normal))`; check only, the ledger is unchanged so receipts stay byte-identical |
| `sample_format` | render-cpu `:450-452`; render-gpu `:384-386` | `OutputTarget.format != RgbF32Le` | (u) | `SampleFormat` has one variant and is `non_exhaustive`: no plan can name another format. An in-crate exhaustive match (unit test `every_enum_a_backend_refuses_on_has_an_admission_decision`) breaks the build when a variant is added, so its decision must be written in `admit_plan_rules` |
| (order) | `admit_plan_rules` | runs last in `admit` and `admit_extended` | | every plan refused before keeps its key (test `earlier_refusals_keep_their_keys_before_the_new_rules`: v1 density wins over the pass-through mask, fill wins, sample range wins over alpha); order among the new rules is mask, alpha, leaf-root class |

### Evidence for (b) `mask_on_pass_through` (research 2026-10-11; nothing guessed)

| Source | What it says | Read how |
|---|---|---|
| pigment `blend_math::StackNode::Group` / `fold_nodes` | pass-through group opacity < 1 is a premultiplied lerp (Approximate, declared per backend as `PassThroughOpacity`); the group has no mask input; fill on a pass-through layer is intrinsically Unsupported (`PASS_THROUGH_FILL_OPERATOR`), Blend-If and effects on a pass-through layer are refused (`BlendIfPassThrough`, `EffectPassThrough`) | read locally |
| W3C Compositing and Blending 1 (w3.org/TR/compositing-1) | in SVG "mask always creates an isolated group"; opacity below 1 and masking force isolation, so a masked non-isolated group has no definition there | fetched |
| Spec v02.215 STU-CMP-022 / pass-through wording | pass-through composites straight onto the backdrop; no sentence defines a mask on it | read locally |
| Adobe Help (helpx.adobe.com) | HTTP 403 for the fetch tool: primary text NOT read (UNVERIFIED) | fetch failed |
| Web search (Photoshop mask on a Pass Through group) | tutorials say a Pass Through group lets adjustment layers inside it affect layers below; the Foundry Mari docs (an analogy, not Photoshop) say a mask on such a group limits the effect to the masked area. No Photoshop primary text | search snippets only |

Conclusion: the common Photoshop workflow (a mask on a Pass Through group of adjustment layers) probably means `lerp(before, after, mask)` per pixel, but no public source defines it, and pigment's own definition stops at group opacity. The same standard as `blend-if-clip-roles` and `effects-ops` applies: no definition, refuse for every backend and make the definition a D5 case. A product consequence the lead should weigh: PSD files with a masked Pass Through group (not rare) are typed-lossy until D5 case 28 is answered.

### Evidence for (a) `alpha_premultiplied`

Spec v02.215 STU-CMP-022 (`14-studio-creative-suite.md` l.29377): alpha is STRAIGHT or PREMULTIPLIED at every boundary; "Internal compositing operates on a single declared convention and converts at the boundary". Pigment's maths is straight (`Rgba::premultiplied`, `from_premultiplied` exist, `ResolvedTile::planes` yields Straight). So the semantics are spec-defined (convert at the boundary) and only the backends lack the conversion. The class is Approximate because the unpremultiply of a zero-alpha pixel discards its colour, a Studio rule rather than a published formula.

### Remaining sweep rows

Covered (admission decides; the backend check is defence in depth, 22 sites): render-cpu `plan_sink.rs:153` (`Operator::Blend(mode)` mapping), `:551` (pass-through opacity), `:560` (fill), `:564` (non-isolated blended), `:567` (blend mode), `:581` (`effects_with_blend_if`), `:584` and `effects.rs:128` (`effects_nested`), `:601` (mask density, v1), `:614` (mask format), `extended.rs:130` (effect feature); render-gpu `plan_sink.rs:443` (Solid range), `:468`, `:471`, `:475`, `:478`, `:490` (knockout), `:524`, `:534`, `:595` (adjustment kind, same predicate as `extension_exactness`), `:606`, `:618`.

(c) cannot be known at admission (13 sites and the port-error families):
1. Source resolution and verification: `UnresolvedSource`, `SourceMismatch` for declared shape, stride, hash, non-finite, alpha range and the colour tag (render-cpu `plan_sink.rs:484-485`, `:397-421`, `:608-631`; render-gpu `:424`, `:318-340`, `:538-545`). Depends on the resolved planes.
2. Tile sample range: `sample_range_outside_unit` from tile data (render-cpu `:436`; render-gpu `:352`). Data. The Solid half is covered.
3. Vector payload: `vector_geometry`, `vector_aa_samples`, fill colour range, path budget (render-cpu `coverage.rs:25,38`, `plan_sink.rs:340`). The payload type `V` is opaque to pigment.
4. Mask plane against its declaration and `mask_coverage_gap` (render-cpu `:632-640`; render-gpu `:546-550`): `SourceMismatch` in both, so they agree. The gap depends on the vector content bounds (payload), so pigment cannot decide it uniformly.
5. `effect_work_limit` (render-cpu `extended.rs:150,380`, `MAX_EFFECT_WINDOW_WORK`) and `effect_cost` (render-gpu `plan_sink.rs:1971`, `FX_PASS_MAX_OPS`): admitted effect parameters (size up to 1000 px, spread up to 1) times the backend's chunk size. render-cpu publishes pigment's `layer_window_cost` for a pre-flight.
6. `mask_feather_cost` (render-gpu `ext.rs:324`, `plan_sink.rs:365`): host blur cost `(rows + output pixels) x taps` above `2^31` for admitted feather radii up to 10000 px.
7. Working-set budgets: `BudgetExceeded` (render-cpu `plan_sink.rs:1356`, `extended.rs:146,182`; render-gpu `:240`, `:361`, `:1184`, `:1942`, `:1974`).
8. `extended.rs:127` (`over_budget` of the reference plane) and the three sample-range mappings `plan_sink.rs:150`, `extended.rs:103,110` (data domain of intermediate values).
9. Hostile geometry: typed `Overflow` (render-cpu `px.rs`, `plan_sink.rs:1456,1473`; render-gpu `overflow()` sites).
10. Execution control: `Canceled`, `DeadlineExceeded`, `Sink`, `worker_panic`, `worker_lost`, `plan_digest`, `effect_allocation`.
11. Device (render-gpu only): `Unavailable(no_adapter | no_device | adapter_timeout)`, `DeviceLost`, `Gpu(device_validation | readback_map_failed | device_fault | device_timeout)`, `Pipeline`.

(u) unreachable after admission (38 sites): `plan_shape` (render-cpu `plan_sink.rs:579,728,734,831,888,992`, `effects.rs:113,165,273`, `extended.rs:501,524`; render-gpu `:1402,1527,1579,1670,1781`), `chunk_scratch` (`:1200`), `effect_scratch` (`effects.rs:168`), `feather_scratch` (`extended.rs:570`), `effect_window_domain`, `effect_source`, `effect_parameter`, `plane_shape` (`extended.rs:126,133,157,159,160,161`), `effect_limit` (`extended.rs:124,155`: `PlanError::EffectLimit` at validate), `mask_feather` / `blend_if` / adjustment parameter mappings (`extended.rs:104,105,111,256`), `blend_error` other codes (`plan_sink.rs:154`), `sample_format` (`:451`, gpu `:385`), render-gpu `mask_feather_radius` (`:360`, validate bounds the radius to 10000 px), `effect_size` and `effect_parameter` (`fx_error`, `:239`): the widest admitted effect blur (`MAX_EFFECT_SIZE_PX` 1000) is 3001 taps and render-gpu `MAX_TAPS` is 3004, pinned by the pigment test `widest_admitted_effect_blur_kernel_fits_the_gpu_tap_bound`; `blend_if_channel` / `blend_if_entries` (`ext.rs:57-70`, `plan_sink.rs:587`: validate refuses a duplicate channel and there are four channels); `curves_lut` / `curves_points` and the unencodable adjustment key (`ext.rs:115-153`, `plan_sink.rs:596`: `AdjustmentOp::prepare` builds the same `PreparedCurve`). If pigment adds a `BlendChannel`, an adjustment kind or a larger effect limit, the matching backend arm becomes reachable: the change must come with its admission decision.

### Pigment changes (additive; v1 `Operator`, `PlanError`, `BackendError` and `MaskInput` untouched)

- `render_plan.rs`: `PASS_THROUGH_MASK_OPERATOR`, `RenderPlan::{has_pass_through_mask, uses_premultiplied_alpha, has_leaf_root}`, `admit_plan_rules` (private to the crate, called last by `admit` and `admit_extended`), descriptor key `admission_parity`, doc on `AlphaConvention` and `SampleFormat`.
- `render_plan/extensions.rs`: `ExtOperator::AlphaPremultiplied` (one new variant of a `non_exhaustive` enum), `ALPHA_BOUNDARY_EXACTNESS`. `PlanExtensions::validate` is unchanged and does not report it (composite compares its ledger with `validate`; the convention is declared by the plan alone).
- Tests: `tests/admit_parity.rs` (6) and one in-crate tripwire; pigment 59 pass; clippy clean for pigment.

### What each consumer must now do

- composite (`hsk-studio-composite`): a masked pass-through group lowers to a plan pigment now refuses. Mirror the pass-through fill rule (`evaluate_static.rs` `appearance_block` ~241-243 and `record_appearance` ~212-224, `lower_plan.rs` ~97-110 `Appearance::exactness`): return `Some(PASS_THROUGH_MASK_OPERATOR)` from `appearance_block` when `a.mode == PassThrough && a.mask` (step `Unavailable`, key from pigment), record `PASS_THROUGH_MASK_OPERATOR` as Unsupported in the ledger, lower the exactness to Unsupported, and add a test next to `render_plan_lowering.rs` ~1257 / ~2078 that the lowered plan's refusal under an all-Exact backend equals that key. Composite lowers Straight plans only, so no premultiplied change (do not map a premultiplied target without declaring it).
- render-cpu: no behaviour change (the wildcard already declares `AlphaPremultiplied` Unsupported). Name `ExtOperator::AlphaPremultiplied => Exactness::Unsupported` in `extension_exactness` (`plan_sink.rs` ~1699-1716) so the declaration is visible. Build the defence-in-depth refusals at `:395`, `:448`, `:604` from pigment (`ExtOperator::AlphaPremultiplied.key()`, `PASS_THROUGH_MASK_OPERATOR`) instead of literals (PLR-006) and extend `refusal_keys.rs` so `admission_and_prepare` covers them; change the descriptor wording from "unsupported" to "refused by pigment admission" for mask on a pass-through layer and premultiplied alpha. The leaf-root rule needs nothing (Normal is Exact).
- render-gpu: same literal-to-key change at `:316`, `:382`, `:527`; add `ExtOperator::AlphaPremultiplied` to the unsupported side explicitly in `extension_exactness` (`:1113-1136`). `leaf_root_class` (`:1032-1054`) stays only to record `normal: fitted` in the receipt (test `leaf_root_reports_the_blend_kernel_class`); the refusal now comes from admission first with the same error. Still open from `effects-ops`: declare `EffectsNested` (test `fx_stack_mask_group_and_nesting_parity` fails until then).
- interop-psd: a PSD group with the Pass Through blend key and a layer mask needs a typed loss entry like the clip-role one (`proposal.rs` ~649-695); the owner picks the code (suggest `mask_on_pass_through`); the mask and the group stay preserved.

### D5 oracle cases added (Operator-approved foreground session; never driven by agents)

28. Pass Through group holding a Levels or Curves adjustment layer and a pixel layer in Multiply, with a layer mask on the group (50 % ramp) over a gradient backdrop: per pixel, is the result `lerp(before, after, mask)` (the group contribution weighed by the mask, like group opacity)? Repeat with group opacity 50 % and with the mask inverted or at density 50 %.
29. Same group with Fill below 100 %: confirm whether Photoshop honours it (pigment refuses it, `PASS_THROUGH_FILL_OPERATOR`).

When 28 is answered the definition lands additively: `StackNode::Group` gets the mask in its lerp factor (`opacity x mask`), the intrinsic refusal becomes a declarable envelope-style operator (as `MaskDensity`), and `tests/admit_parity.rs` `pass_through_mask_*` become the migration list.

</topic>
