---
file_id: LANE-SMB-AE-MASK-FEATHER-1
file_kind: lane_brief
updated_at: 2026-10-30T11:00:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AE-MASK-FEATHER-1
lane_file: builders/lanes/hsk-studio-ae-mask-feather-1.json
crates: [hsk-studio-folio, hsk-studio-composite, hsk-studio-render-cpu (new test file only), hsk-studio-package (externals hunk only if its test demands)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules
---

<topic id="target" status="active" summary="After Effects composition masks: feather, opacity, expansion, static and animated">

## Target

Engine only. Base: origin c719b1dc. AE-13 ("Masks: Bezier mask paths, feather, expansion, modes, animated") is PARTIAL. Composition masks carry path, mode and inverted; feather, opacity and expansion are absent (`hsk-studio-folio/src/composition.rs:643`).

Read first:
- `hsk-studio-composite/src/mask_paths.rs`.
- `src/temporal/composition.rs`: `LayerMasks` :319, `mask_property` :326, the build at :985-1000, the `MaskAtTick` loops at :1576 and :2314.
- `src/temporal/composition/effects.rs` :216.
- `src/track_apply.rs` :487.
- folio `validate_mask` :884.
- pigment `coverage_math::{feather_kernel, feather_support, feather_sigma}`.
- nib `edit::offset_path`.

Push after each green slice, after the lead's go (see paths).

1. **Folio record** (`src/composition.rs`). `StudioCompositionMask` gains three optional keyframable streams. Each has the path stream's form (`static_value` + `keyframes` of `StudioCompositionKeyframe`), for example a small `StudioCompositionMaskStream` record.
   - `feather`: 2 components (x, y px), each >= 0. This is a Studio reading of the uniform "Mask Feather" (STU-MOT-095 table). The two-axis form is After Effects behaviour, UNVERIFIED (D5).
   - `opacity`: 1 component, STORED normalised 0..=1 and displayed as percent (STU-CMP-026).
   - `expansion`: 1 component, px, signed, finite, within folio's coordinate bound.
   - Absent means feather 0, opacity 1, expansion 0. Use serde default and skip-when-absent, so every existing record serialises byte for byte as before. `deny_unknown_fields` stays.
   - `validate_mask` checks component counts, finiteness, ranges and keyframe rules, the same way it does for the path.
   - Update the `track_apply.rs` literal with the fields absent.
   - Tests in new `folio/tests/composition_mask_properties.rs`:
     - red-first: a mask carrying `feather` is refused at c719b1dc as an unknown field, and accepted after;
     - existing records are byte-identical after decode and encode;
     - refused: opacity 1.5 (stored form, STU-CMP-026), feather -1, non-finite expansion, a wrong component count, bad keyframes;
     - the 64-mask and 1024-vertex budgets are unchanged.

2. **Shaped coverage in composite** (`src/mask_paths.rs`, additive).
   - Keep `MaskAtTick` and `mask_coverage` unchanged. `MaskAtTick` is built by `tests/support/masked_scene.rs`, which render-cpu and render-gpu `composition_mask_paths.rs` include by `#[path]`.
   - Add a per-mask shape (for example `MaskShape {feather_px: [f64; 2], opacity, expansion_px}` whose `Default` is the identity) and `mask_coverage_shaped(&[(MaskAtTick, MaskShape)], matrix, rect, max_pixels)`.
   - Per mask, in this order (Studio reading; After Effects' order and arithmetic are UNVERIFIED, D5):
     1. expansion: offset the path with nib `offset_path`, round join (STU-CMP-027: same vector primitive; STU-VEC-113). Declare whether the offset is applied in layer space before the matrix or in composition pixels after it;
     2. coverage over the rect grown by `feather_support`;
     3. separable feather with `feather_kernel` per axis, cropped back to the rect. Use the same maths as the Photoshop vector-mask feather and declare how feather width maps to radius;
     4. invert;
     5. multiply by opacity;
     6. combine in order with the existing `combine` (STU-CMP-025a).
   - The identity shape must be bit-identical to `mask_coverage`.
   - A feather beyond pigment's support limits is a typed refusal naming the mask. An expansion that collapses the region gives no coverage, not an error.
   - Tests in new `composite/tests/mask_shape.rs`:
     - identity equals `mask_coverage` bit-exact;
     - expansion +k on an axis-aligned rect equals the rect grown by k with round corners, exact inside and outside, corners within 1/255; -k shrinks it;
     - a collapse gives empty coverage;
     - feather is a symmetric ramp centred on the edge (about 0.5 at the edge), integral preserved within a declared tolerance;
     - x-only feather softens only vertical edges;
     - opacity 0.5 halves an `add` mask, and mask order still matters with opacity.

3. **Temporal wiring.** These paths are behind the `motion` feature (`temporal.rs:77-84`); the round unit composite `["motion"]` exists.
   - Carry the three streams through `LayerMasks`, replacing the tuple with a struct.
   - Sample them per tick with the same `property()` conversion as the path, and call the shaped coverage at both loops and in `effects.rs`.
   - Tests in new `composite/tests/composition_mask_shape.rs`, gated `#![cfg(feature = "motion")]`:
     - feather keyframed 0 -> 8 px gives different mattes at two ticks, each equal to the static evaluation at that tick;
     - animated opacity and animated expansion;
     - a layer with an effect stack uses the shaped mask.
   - CPU render proof: new `render-cpu/tests/composition_mask_shape.rs` with new support file `composite/tests/support/mask_shape_scene.rs`. Never edit `masked_scene.rs`. The frame must equal the coverage oracle for every chunking, like `composition_mask_paths.rs`.
   - GPU: print a `NOT_PROVEN:` line. Both backends already render the same coverage plane, and the lane avoids the GPU lock.

4. **Small items.** Composite DESCRIPTOR string hunk, only if it lists mask capabilities. The `mask_paths.rs` module rustdoc lists every Studio reading.

</topic>

<topic id="spec" status="active" summary="Spec basis (module 14 v02.225)">

## spec_basis

- STU-CMP-025 (:29388): a mask entry carries path, feather (optionally variable width), opacity and expansion, all keyframable.
- STU-CMP-025a (:29396): seven modes, combined in order, with an independent inverted flag.
- STU-CMP-026 (:29403): opacity stored 0..1 and displayed 0..100.
- STU-CMP-027 (:29413): mask geometry is the vector path primitive.
- STU-CMP-010 step 2 (:29278): masks combine before effects.
- STU-MOT-095 table (:27893-27899): Mask Opacity, Mask Expansion (`ADBE Mask Offset`), Mask Feather (uniform), and variable-width feather points (`ADBE Mask Interp`).
- Declared gap: variable-width feather (`ADBE Mask Interp`). There is no D5 oracle case for After Effects masks.

</topic>

<topic id="paths" status="active" summary="Owned paths, do-not-touch, coordination">

## Owned paths (lane worktree `wt-studio-lanes/smb-ae-mask-feather-1`)

- folio: `src/composition.rs` (mask record and `validate_mask`), NEW `tests/composition_mask_properties.rs`.
- composite:
  - `src/mask_paths.rs` (additive);
  - `src/temporal/composition.rs` (mask hunks only);
  - `src/temporal/composition/effects.rs` (mask hunk only);
  - `src/track_apply.rs` (literal);
  - `src/lib.rs` (DESCRIPTOR string hunk only);
  - NEW `tests/mask_shape.rs`, `tests/composition_mask_shape.rs`, `tests/support/mask_shape_scene.rs`.
- render-cpu: NEW `tests/composition_mask_shape.rs` only.
- package: one `tests/externals.rs` classification hunk, only if its test demands one for the new fields.
- Do NOT touch:
  - `composite/tests/support/masked_scene.rs` or the existing mask tests (keep them compiling unchanged);
  - `src/composition/effect_stack.rs` (AE-17 is the next lane there);
  - nib (read-only; SMB-AI-SIMPLIFY-1 adds files there);
  - render-gpu;
  - harness-view;
  - other folio files;
  - root `Cargo.toml`, `Cargo.lock`, `tools/`, `.config/`.
- Coordination: folio is a dependency of every running lane (SMB-R41-FOLLOWUPS-1, SMB-AU-MULTITRACK-1), so push only after the lead's go. The queued SMB-AE-PARTICLES-2 (composite + folio + render-gpu) must be sequenced after this lane.

</topic>

<topic id="proof" status="active" summary="Proof and tier">

## Proof

- **Bundled run, per push.** One bundled lane-check on C: over the WHOLE suites:
  - slices 1-2: `--disk ssd --owner smb-ae-mask-feather-1 --tests <slice tests> -- cargo nextest run -p hsk-studio-folio -p hsk-studio-composite -p hsk-studio-package --features hsk-studio-composite/motion`;
  - from slice 3, add `-p hsk-studio-render-cpu`.
- **Rule 4b.** A Folio record field is added, so the package suite runs. The new pub fields on `StudioCompositionMask` change folio's public signature. At c719b1dc only composite names the type (`git grep StudioCompositionMask -- crates/`). Re-grep at your base and add the suite of any new hit's crate.
- **Composite API.** Additive only, so composite's dependents' suites are not required.
- **Default build.** The round runs composite both default and `["motion"]`. Keep `mask_paths.rs` free of motion-only items, and state in the notes how you verified the default build is warning-free.
- **Rules 4c-4d.** Run `--step0-only` first. Doc examples mean `cargo test --doc -p <crate>`.
- **Test style.** Red-first. Print `NOT_PROVEN:` lines for After Effects mask parity, variable-width feather, and the GPU render of shaped masks.
- **Model tier.** opus/high: multi-crate, spec clauses.

</topic>

<topic id="gain" status="active" summary="Estimated parity gain">

## Estimated parity gain

- AE-13, After Effects (24 rows): PARTIAL -> BUILT_NOT_VALIDATED (+1.04 pp AE engine); VALIDATED after a round PASS (+3.13 pp).
- Pooled 9-app engine: +0.08 to +0.24 pp.
- Risk: if the audit treats variable-width feather (`ADBE Mask Interp`) as a required part of the row, AE-13 stays PARTIAL (0 pp).
- GUI: none.

</topic>
