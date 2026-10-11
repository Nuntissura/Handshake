---
file_id: BUILDER-BRIEF-LANE-EFFECTS-WAVE-1
file_kind: builder_brief
updated_at: 2026-10-11T05:45:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-pigment
lane: SMB-PIGMENT-EFFECTS-1
depends_on: [pigment ff92dea, folio ff92dea (StudioEffectStack records)]
decision: diagnostics/adjustments-advanced-blending.md
---

<topic id="scope" status="active" summary="Pigment effect nodes: drop shadow, stroke, colour overlay first">

## Scope (pigment only; backends and composite follow in their own lanes)
1. **New single owner `pigment::effect_math`** (CX-CAP-001).
   - Per-pixel and neighbourhood reference definitions, each with an exactness class, all `Approximate`: Photoshop's effect maths is unpublished, and Craft fits it to a corpus (`research/craft-case-study.md`, `effects.rs:730`).
   - **Drop shadow.** The matte is the layer alpha.
     - Spread: matte' = `min(1, matte / (1 - spread))` with spread in `[0, 1)` (choke-before-blur reading). Spread 100% is a hard matte.
     - Offset `(dx, dy) = distance * (-cos(angle), sin(angle))` in pixel space, with y down and angle in degrees. Photoshop's sign convention is UNVERIFIED; it is D5 case 12.
     - Gaussian blur of size `size_px`, with `sigma = size / 2`, using the same rule as the mask feather, `coverage_math::feather_sigma`.
     - Colour: the effect colour. Mode and opacity: the effect's own.
     - Placement: behind the layer content, inside the layer's group.
     - `layer_knocks_out`: shadow alpha *= `1 - layer alpha`.
   - **Colour overlay.** Colour composited with the effect mode and opacity atop the layer content, clipped to the layer alpha. It is placed over the content, at full strength regardless of `fill_opacity` (STU-RAS-115).
   - **Stroke (solid).** The stroke matte comes from a Euclidean distance transform of the layer alpha at threshold 0.5, with anti-aliased edges:
     - `outside` = (d_out <= size) minus alpha
     - `inside` = alpha minus (d_in > size)
     - `centre` = band of width `size` about the edge
     - Placed above the content and overlay.
   - Every effect fades with layer opacity, not with fill.
   - `effect_support_px(op)` gives the margin a backend must read.
2. **Envelope extension.** Add `effects: Vec<EffectOp>` to `LayerExtension`, which is additive because `LayerExtension` is `#[non_exhaustive]`.
   - `EffectOp` is `#[non_exhaustive]` and carries `DropShadow {..}`, `ColourOverlay {..}` and `Stroke {..}`, plus `EffectOrder` placement (`behind_content` | `over_content`).
   - Add the `ExtOperator::Effect(EffectKind)` member (`ExtOperator` is `#[non_exhaustive]`), with intrinsic class Approximate.
   - `validate`: there is no effect on an adjustment placeholder or on a pass-through layer. Render order is list order within each placement (STU-RAS-156). Global light is resolved by the builder: the plan carries absolute angles.
3. **Single-pixel and small-plane oracle.** `effect_math::render_reference(plane, op) -> plane` over at most 64 x 64 f32 planes. Backends test against it.

</topic>

<topic id="constraints" status="active" summary="Constraints">

## Constraints
- Additive only. Dependents keep compiling: check `-p hsk-studio-composite -p hsk-studio-render-cpu -p hsk-studio-render-gpu -p hsk-studio-interop-psd`.
- Do not change v1 IR types (decision note topic additivity).
- Neighbourhood ops are bounded. Check radius times plane size against a budget, with checked arithmetic (V4-01/02: `aggregate_over_budget` test).
- Each kind's fixed Photoshop order (shadow behind; overlays then stroke over content) versus the spec's list order is a SPEC QUESTION. interop-psd builds lists in the Photoshop order, and pigment renders list order per placement. Record it in the lane notes.
- Workflow v4: V4-03 Photoshop parity `NOT_PROVEN` (D5 cases 12-14), V4-07, V4-09.

</topic>

<topic id="acceptance" status="active" summary="Acceptance">

## Acceptance (hand-derived, at most 5 tests)
1. **Drop shadow**, distance 0, spread 0, size 0 on a 1-pixel opaque layer: the shadow equals the matte, and it sits under the content, so it is hidden where alpha is 1. With `layer_knocks_out` false and the layer at fill 0, the shadow is visible at the effect opacity.
2. **Drop shadow** offset `(distance 2, angle 180)` moves the matte by `(+2, 0)` under the stated convention. Size 2 gives a blurred edge equal to the hand-convolved feather taps (`0.39905` centre).
3. **Colour overlay** red, Normal, 50% over a blue opaque pixel gives `(0.5, 0, 0.5)`. Over a transparent pixel it stays transparent (clipped).
4. **Stroke** outside 1 px around a 3 x 3 opaque square gives ring coverage 1 on the 8-neighbour ring at distance 1 and 0 at distance 2 or more. Inside gives the inner ring.
5. A plan with an effect on a backend without support is refused with `Unsupported { operator: "drop_shadow" }`, with no sink calls.

</topic>

<topic id="verification" status="active" summary="Cargo form">

## Verification
Use the studio cargo form with `-p hsk-studio-pigment -p hsk-studio-folio --tests`, then the dependents check. Commit with a pathspec, push, and update the lane file. The next lanes after this one: the composite lowering of `StudioEffectStack` to `EffectOp`, then render-cpu and render-gpu, then the rest of the kinds (inner shadow, glows, satin, gradient/pattern overlay, bevel).

</topic>
