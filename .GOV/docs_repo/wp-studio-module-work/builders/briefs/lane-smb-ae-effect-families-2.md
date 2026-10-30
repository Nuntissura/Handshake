---
file_id: LANE-SMB-AE-EFFECT-FAMILIES-2
file_kind: lane_brief
updated_at: 2026-10-30T13:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AE-EFFECT-FAMILIES-2
lane_file: builders/lanes/hsk-studio-ae-effect-families-2.json
crates: [hsk-studio-composite (effect stack), hsk-studio-folio (effect records, only if a field is needed)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e incl. 4b, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="After Effects AE-17: blur / glow / colour families and entry blend + mask in the per-layer effect stack">

## Target

Gap: After Effects rank 2, AE-17 "per-layer effect stack (blur/glow/colour still absent from the stack)" (`research/parity-20261023-video-audio.json`). SMB-AE-EFFECT-STACK-1 (PASS) built the stack with distort/generate families; read its lane file `builders/lanes/hsk-studio-ae-effect-stack-1.json` notes (`spec_basis`, `gaps_left`) FIRST. Spec (grep by id only): `[STU-FX-001..004]`, `[STU-FX-007]`, `[STU-FX-008]` (entry composes with its blend mode + opacity exactly as a layer blends), `[STU-FX-131]` (parameter records), `[STU-CMP-015]` (adjustment layers). After Effects' own effect maths is UNVERIFIED (D5): declared Studio readings, cited per effect.

REUSE, do not re-implement: pigment already has blur, effect, filter and adjustment kernels (`src/blur_math.rs`, `src/effect_math.rs`, `src/filter_math.rs`, `src/adjust_math.rs`, `src/adjust_colour.rs`; composite already depends on pigment). The stack entries map AE parameters onto those kernels; a missing kernel capability is reported to the lead with its signature, not copied into composite.

Slices (commit locally per slice; one bundled run per push):
1. Blur family: Gaussian Blur, Fast Box Blur, Directional Blur, Radial Blur (spin/zoom if pigment has it, else refused typed), Camera Lens Blur only if a pigment kernel exists (else refused typed and listed). Parameters per the recorded AE rows where typed; repeat-edge-pixels and blur dimensions honoured. Tests: golden per effect on a small fixture, `blur_radius_zero_is_identity`, animated parameter keyframes evaluate per frame.
2. Glow + Stylize basics: Glow (threshold, radius, intensity, composite original on top/behind, colours A/B or original) built from pigment threshold + blur + blend; Find Edges if pigment has it. Tests: golden, threshold 100 % = no glow.
3. Colour family: Levels, Curves, Hue/Saturation, Tint, Tritone, Exposure, Brightness & Contrast, Color Balance, Invert, through pigment adjustment kernels in the stack's working space (colour params stay profile-honest per STU-FX-118 or are refused typed). Tests: golden per effect, identity parameters are identity.
4. Entry semantics: per-entry blend modes beyond normal (STU-FX-008, through the existing layer blend code), entry mask reference (STU-FX-004, reusing the composition mask path incl. feather/opacity/expansion from SMB-AE-MASK-FEATHER-1), and effects on adjustment layers (STU-CMP-015) if the composition path allows it in one slice; else keep refused typed and list.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ae-effect-families-2`)

- composite: the effect-stack modules (`src/temporal/composition/effects.rs` and the family modules SMB-AE-EFFECT-STACK-1 created; find them), `src/lib.rs` (DESCRIPTOR hunk), NEW `tests/effect_families_2*.rs`.
- folio: effect record additions only if a new typed parameter field is unavoidable (rule 4b package suite then).
- Do NOT touch: composite `src/track_apply.rs` (SMB-AE-MASK-FEATHER-2 until it is on origin), pigment (report kernel gaps), harness-view, root Cargo.toml, Cargo.lock, tools/, .config/. No new dependencies or edges.

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Build on C: (`--disk ssd`, owner smb-ae-effect-families-2). A validation round (R42) holds the C: lock with priority now: code first; your bundled run will wait for the lock, which is expected.
- One bundled lane-check per push over the WHOLE composite suite with its round features (motion), plus folio + hsk-studio-package if folio changes, plus direct dependents of any existing public item whose signature changes. Rebase onto origin right before the run. Run log path contains the lane id and the sha.
- NOT_PROVEN lines printed from one test: After Effects parity per family (no D5 oracle), GPU path.
- Estimated gain: AE-17 PARTIAL -> BUILT-pending-validation for blur/glow/colour; After Effects engine +1 to +2 pp.

</topic>
