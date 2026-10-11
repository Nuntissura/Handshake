---
file_id: BUILDER-BRIEF-LANE-RENDER-GPU-ADJUSTMENTS
file_kind: builder_brief
updated_at: 2026-10-11T05:45:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-render-gpu
lane: SMB-RENDER-GPU-ADJ-1
depends_on: [pigment ff92dea, lane-render-cpu-adjustments (reference values)]
decision: diagnostics/adjustments-advanced-blending.md
---

<topic id="scope" status="active" summary="WGSL envelope operators tested against pigment">

## Scope
Implement the envelope operators on the GPU backend in this order:

1. Mask density/invert. This is a WGSL port of `coverage_math::mask_coverage` and replaces the `mask_density` refusal at `src/plan_sink.rs:315` in the extended path only.
2. Blend-If weight (`coverage_math::blend_if_weight`).
3. Adjustment layer for `invert`, `levels`, `posterize`, `threshold` and `exposure` (`adjust_math`).
   - Curves: upload a 4096-entry LUT built on the host from `PreparedCurve::eval`. The LUT is Fitted, not Exact; record that.
   - Hue/Saturation: second step, same pattern.
4. Mask feather: a host-side blur through `coverage_math::feather_kernel` before upload. A compute blur is a later optimisation.

`extension_exactness` returns `Fitted` for the operators implemented, because f32 GPU arithmetic is tolerance-equal and not bit-exact. Everything else returns `Unsupported`. Never fall back to the CPU silently (STU-FX-036).

</topic>

<topic id="files" status="active" summary="Files and constraints">

## Files
- `src/plan_sink.rs`: `extension_exactness` and `execute_extended`.
- `src/kernel.rs` and `src/kernel.wgsl`: formula bodies only.
- `tests/gpu_plan.rs`.

## Constraints
- WGSL is tested against pigment's scalar functions (`adjust_math`, `coverage_math`), never against render-cpu.
- Use the same tolerance policy as the existing blend parity: 1/255 + 4 ulp.
- Serialize device creation in tests.
- Workflow v4:
  - V4-03: a test without an adapter prints `NOT_PROVEN: no adapter` and never passes silently.
  - V4-01: deny `clippy::arithmetic_side_effects` in the touched host modules.
  - V4-07, V4-09.

</topic>

<topic id="acceptance" status="active" summary="Acceptance">

## Acceptance (at most 5 tests)
1. A parity grid of 17 x 17 backdrop/coverage values for `mask_coverage` (density 0, 0.5, 1; inverted on/off) is within tolerance of pigment.
2. Blend-If parity on a gray ramp with split ranges.
3. Adjustment parity on the 17^3 lattice for invert, levels (gamma 2), posterize 4, threshold 0.5 and exposure +1 EV.
4. Knockout and modern brightness/contrast are refused before any upload.
5. The receipt ledger lists `adjustment_layer: approximate` (pigment's intrinsic class is kept) with an Approximate floor.

</topic>

<topic id="verification" status="active" summary="Cargo form">

## Verification
Use the studio cargo form with `-p hsk-studio-render-gpu --tests`. Commit with a pathspec, push, and update the lane file. Name the backend proven (dx12 or vulkan) in the lane file.

</topic>
