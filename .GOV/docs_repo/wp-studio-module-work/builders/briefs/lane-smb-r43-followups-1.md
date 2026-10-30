---
file_id: LANE-SMB-R43-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-30T17:45:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-R43-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r43-followups-1.json
crates: [hsk-studio-pigment, hsk-studio-folio (additive), hsk-studio-composite]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R43 findings F-R43-3 and F-R43-4">

## Target

Close these two LOW findings of round STUDIO-R43. They are in `validation/round-R43.json` → `findings_not_failures`; read each entry's `finding`, `repair_direction` and `file_line`.

1. **F-R43-3, pigment.** `radial_blur_window_cost` (`crates/hsk-studio-pigment/src/blur_math.rs:784`) walks every output pixel before any budget check, so a hostile `out` rectangle spends unbounded time before it is refused.
   - Before the walk, refuse when `area × BLUR_SAMPLE_BYTES > max_bytes` or `area × 4 > max_work`. Use checked arithmetic and the existing typed error.
   - Add a hostile huge-rect case to `tests/ae_kernels.rs` that is refused in bounded time. Assert that it returns quickly: no timing flakiness, just a refusal before any allocation.
2. **F-R43-4, lead decision: profile-honest colour parameters per STU-FX-118.** STU-FX-118 says every colour parameter "carries a required StudioColorProfile reference". Folio's composition-effect records (`crates/hsk-studio-folio/src/composition/effects.rs:41`) have no such field. So composite (`src/temporal/composition/effects.rs`, the Tint, Tritone and Glow A & B Colors arms) reads the colours as straight RGB in the plan's colour tag.
   - **Folio:** add the StudioColorProfile reference to colour-valued effect parameters, additively.
     - Use the same profile-reference shape folio already uses elsewhere. Grep `StudioColorProfile` and use what exists.
     - Records without the field keep their bytes. Pin that with a test.
     - Validation: a colour parameter without a profile reference is refused typed when the effect is applied, not at load.
   - **Composite:** convert each colour parameter from its referenced profile into the plan's working space through prism's existing conversion. Read the prism API; do not write new colour maths.
     - A parameter with no profile reference is refused typed: `composition_effect_option`, with the detail naming the parameter.
     - Update AE-EFFECT-FAMILIES-2's tests that use Tint, Tritone or Glow colours so they carry a profile reference. List each changed test in the notes for the validator.
   - Add tests:
     - sRGB-referenced Tint matches today's result exactly;
     - a Display P3 (or another profile prism has) reference gives the converted value, checked against prism's own conversion;
     - a missing reference is refused.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r43-followups-1`, base ff4ca080)

- pigment: `src/blur_math.rs` (the radial cost path only) and `tests/ae_kernels.rs`.
- folio: the composition effects record module and its tests. The DESCRIPTOR sentence may change.
- composite: `src/temporal/composition/effects.rs` and its effect tests.
- Do not touch:
  - harness-view, interop-fig-base, interop-fig-native and interop-fig (SMB-FIG-GRID-2 is running);
  - root inputs;
  - the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof and order">

## Proof and order

- Write the tests red first. Pushed red commits must compile (RED COMMITS).
- Do one bundled run per rules 4a and 4b: the whole suites of pigment, folio, composite and package, plus clippy over their dependents. Run on D: (`--disk d`) unless the lead names C:. Name the log per RUN LOGS; lane-check now prints a `HEAD=` header line.
- Push order: folio is upstream of everything, so push AFTER SMB-FIG-GRID-2. Do not start the bundled run until the lead messages you with the disk and the base.
- After Effects behaviour stays UNVERIFIED (D5).

</topic>
