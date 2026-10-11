---
file_id: BUILDER-BRIEF-LANE-COMPOSITE-ADJUSTMENTS
file_kind: builder_brief
updated_at: 2026-10-11T05:45:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-composite
lane: SMB-COMPOSITE-ADJ-1
depends_on: [pigment ff92dea (PlanExtensions, adjust_math, coverage_math), folio ff92dea (StudioRasterRecords)]
decision: diagnostics/adjustments-advanced-blending.md
---

<topic id="scope" status="active" summary="Lower folio raster records into the pigment extension envelope">

## Scope
Lower `folio::StudioRasterRecords` into `pigment::render_plan::PlanExtensions`, next to the existing v1 `RenderPlan`. Write scope is `crates/hsk-studio-composite/**` only.

The lowering covers:
1. **Adjustment layers.**
   - A `StudioLayer { kind: "adjustment", payload: PrimitiveReference { schema_id: "hsk.studio.adjustment@1", object_id: <own id> } }` becomes a `Layer` in its group.
   - Its `input` is a new `PlanNode::Solid { rect: <group extent>, rgba: [0.0; 4] }` placeholder.
   - Add `LayerExtension { group, layer, adjustment: Some(op) }`, where `op` is converted from `folio::StudioAdjustment`:
     - Levels: `normalized_0_1` maps 1:1. Channel records become `LevelsRecord`; an absent channel is `LevelsRecord::IDENTITY`.
     - Curves: points are divided by 255. Corner flags are dropped, and each drop is recorded as `Approximate` in the lowering report (pigment has no corner support).
     - Hue/Saturation: master values only. Any band, or colorize with non-zero bands, is refused `Unavailable("hue_saturation_bands")`.
     - Brightness/Contrast: carries `use_legacy`. Pigment refuses modern mode at admission. Do not emulate it.
     - Exposure: `linearize_gamma = 1.0` when the plan blend space is `linear_light`, and `2.2` for `encoded`. This is the psd-tools stand-in; record it in the report.
     - Invert, threshold (`level / 255`), posterize.
     - Threshold with non-default `false_colour`/`true_colour` is refused `Unavailable("threshold_colours")`.
2. **Mask parameters.** `StudioMaskParameters { density, feather, inverted }` maps to `MaskInput.density = density / 100`, `MaskInput.inverted = inverted`, and `LayerExtension.mask_feather_px = feather`.
3. **Blend-If.** `StudioAdvancedBlending.blend_ranges` maps to `LayerExtension.blend_if`. Values are divided by 255, in the order `[black_start, black_end, white_start, white_end]`: `source_black ++ source_white`, `destination_black ++ destination_white`. Channels: gray/red/green/blue.
4. **Knockout.** `StudioKnockout::Enabled` (and `Inherited` that resolves to enabled) becomes `Knockout::Shallow`. Pigment refuses it until implemented, so the plan is refused, never rendered without knockout. `Disabled`/`Unknown` map to `Knockout::None`.
5. **Effect stacks** (enabled, with at least one enabled instance): refuse the layer with `Unavailable("layer_effects_not_rendered")` until effects wave 1 lands. Never drop them silently.
6. **Evaluation.** In `evaluate_static`, derive classes from `ExtOperator::intrinsic()` and `key()`. Never use hand-written literals: the R5 finding was about composite literal drift.

</topic>

<topic id="files" status="active" summary="Files">

## Files
- `src/lower_render_plan.rs`. Add `lower_to_render_plan_extended(...) -> Result<(RenderPlan, PlanExtensions, SourceTable), CompositeError>`. Keep the existing function unchanged; it is the empty-envelope path.
- `src/evaluate_static.rs`: class derivation as in scope item 6.
- `tests/render_plan_lowering.rs`: add cases.
- `DESCRIPTOR`: remove `effect` (still unsupported), `knockout`, `blend_if` and `mask density feather invert` from the unsupported list only where the envelope now carries them.

</topic>

<topic id="constraints" status="active" summary="Constraints">

## Constraints
- `LayerExtension`, `PlanExtensions` and `AdjustmentOp` are `#[non_exhaustive]` (pigment 2587930). Build them with `LayerExtension::new(group, layer).with_adjustment(..).with_blend_if(..).with_mask_feather(..).with_knockout(..)` and `PlanExtensions::from_layers(..)`. Struct literals do not compile outside pigment.
- Consume the lowest owner (CX-CAP-001).
  - Pigment owns all maths and classes: `adjust_math`, `coverage_math`, `render_plan::{PlanExtensions, ExtOperator, admit_extended}`.
  - Composite adds no formula and no class literal.
- Folio types stay out of the plan (`SourceKey` plus content hashes only).
- An unknown or unsupported feature is a typed refusal, never a silent drop.
- Workflow v4 (DISPATCH_TEMPLATES topic `v4-rules`):
  - V4-03: a Photoshop-parity claim without oracle data is `NOT_PROVEN:`.
  - V4-06: name the failure class and its regex for any remediation.
  - V4-07: DESCRIPTOR claims cite tests.
  - V4-09: commit messages via `-F` file.

</topic>

<topic id="acceptance" status="active" summary="Expected behaviour and acceptance">

## Acceptance (hand-derived, at most 5 focused tests)
1. A document with raster(1), adjustment(2, invert) and raster(3) in an isolated group lowers to a plan with a transparent `Solid` placeholder for layer 2, plus one `LayerExtension { adjustment: Some(Invert) }`. `PlanExtensions::validate` returns `[AdjustmentLayer, Adjustment(Invert)]`.
2. Mask parameters `(50, 4.0, true)` lower to `density 0.5`, `inverted true` and `mask_feather_px 4.0`.
3. A Blend-If gray source `[50,100]/[200,240]` lowers to `[50/255, 100/255, 200/255, 240/255]`.
4. Knockout enabled is refused by `admit_extended` as `Unsupported { operator: "knockout_shallow" }`. An effect stack is refused with `layer_effects_not_rendered`.
5. The evaluation ledger keys equal the pigment `ExtOperator::key()` values. Test this with a grep-free equality against the pigment constants.

</topic>

<topic id="verification" status="active" summary="Cargo form">

## Verification
`cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo <check|clippy|test> -p hsk-studio-composite --tests` (clippy `--no-deps` while another lane is mid-edit in a dependency). Commit with a pathspec (`-- crates/hsk-studio-composite`), push, and update the lane file. Set READY_FOR_VALIDATION only after a pushed, green commit.

</topic>
