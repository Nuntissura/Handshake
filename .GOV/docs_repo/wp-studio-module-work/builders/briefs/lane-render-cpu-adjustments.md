---
file_id: BUILDER-BRIEF-LANE-RENDER-CPU-ADJUSTMENTS
file_kind: builder_brief
updated_at: 2026-10-11T05:45:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-render-cpu
lane: SMB-RENDER-CPU-ADJ-1
depends_on: [pigment ff92dea]
decision: diagnostics/adjustments-advanced-blending.md
---

<topic id="scope" status="active" summary="Implement the envelope operators in the CPU reference backend">

## Scope
Implement `RenderBackend::execute_extended` and `extension_exactness` for `Renderer`. render-cpu stays the reference oracle.

1. `extension_exactness`:
   - `AdjustmentLayer`, `Adjustment(k)`, `BlendIf`, `MaskDensity` and `MaskFeather` return `Exactness::Exact`. The backend implements pigment's formulas verbatim; the intrinsic class (Approximate) is what reaches the receipt.
   - `Knockout(_)` returns `Unsupported`.
2. `execute_extended`:
   - Call `admit_extended` first.
   - Then reuse the existing chunked compositor, extended as follows:
     - **Adjustment layer** at layer i of a group: for every pixel of the placeholder rect, compute `k = opacity * fill * mask * blend_if`, then call `adjust_math::composite_adjustment(mode, backdrop, &prepared, k)` on the group's running buffer. Prepare once per plan with `AdjustmentOp::prepare`. Inside a clip-to-base run, the backdrop is the run buffer G.
     - **Blend-If**: multiply the source alpha by `coverage_math::blend_if_weight(entries, src_rgb, backdrop_rgb)` before `composite_over`.
     - **Mask density/invert**: use `coverage_math::mask_coverage(c, inverted, density)`. This replaces the `mask_density` refusal at `src/plan_sink.rs:459`, which stays in the v1 `execute` path.
     - **Mask feather**: apply a separable blur with `coverage_math::feather_kernel(px)` to the mask coverage plane, read with a `feather_support(px)` margin and clamped at the plane border, before density.
   - Keep the scratch budget arithmetic checked (V4-01).
3. Everything else is unchanged. A plan with an empty envelope takes the v1 path, and its bytes are identical.

</topic>

<topic id="files" status="active" summary="Files and constraints">

## Files
- `src/plan_sink.rs`: the `impl RenderBackend` block at about line 1306, `prepare` at line 340, and the compositing loops at lines 749 and 835.
- `tests/cpu_plan.rs`.

## Constraints
- Do not copy or vary any formula; call pigment (CX-CAP-001).
- Make no new public blend or exactness types.
- Charge feather scratch to `max_working_bytes`, so a feather larger than the budget is refused with `BudgetExceeded`.
- Workflow v4:
  - V4-01: deny `clippy::arithmetic_side_effects` in the touched modules.
  - V4-02: add an `aggregate_over_budget`-style test for the feather margin.
  - V4-07: DESCRIPTOR claims cite tests.
  - V4-09: commit messages via `-F`.

</topic>

<topic id="acceptance" status="active" summary="Acceptance">

## Acceptance (hand-derived, at most 5 tests)
1. **Invert adjustment** over an opaque `(0.2, 0.4, 0.6)` solid, in an isolated group, at opacity 0.5: the pixel is `(0.5, 0.5, 0.5, 1.0)`. Derivation: `0.2 + (0.8 - 0.2) * 0.5` etc. Repeat over a semi-transparent backdrop of alpha 0.5: alpha stays 0.5.
2. **Mask density** 0.5 over coverage 0.25 gives an effective coverage of 0.625. A Normal red layer over black at that pixel gives red channel 0.625.
3. **Blend-If** gray source `[0.2, 0.4, 0.6, 1.0]` on a source of gray 0.3 gives source alpha × 0.5.
4. **Feather** 2 px on a hard 0/1 mask edge: the centre tap is 0.39905, and the sum of taps is 1. Check one pixel next to the edge against the hand-convolved value.
5. Knockout is refused as `Unsupported { "knockout_shallow" }` before any sink call (sink.calls == 0). The bytes of the v1 path are unchanged on an existing plan (regression).

</topic>

<topic id="verification" status="active" summary="Cargo form">

## Verification
Use the studio cargo form with `-p hsk-studio-render-cpu --tests`. Commit with a pathspec, push, and update the lane file. Photoshop parity remains `NOT_PROVEN` (D5 cases 1, 9, 10; V4-03).

</topic>
