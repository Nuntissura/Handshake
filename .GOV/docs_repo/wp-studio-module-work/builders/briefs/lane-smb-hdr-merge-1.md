---
file_id: LANE-SMB-HDR-MERGE-1
file_kind: lane_brief
updated_at: 2026-10-30T09:10:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-HDR-MERGE-1
lane_file: builders/lanes/hsk-studio-hdr-merge-1.json
crates: [hsk-studio-pigment, hsk-studio-develop (slices 3-4)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Merge bracketed exposures to HDR (align, deghost, tone-map) in pigment; Lightroom HDR merge entry in develop">

## Target

Gap: Photoshop Merge to HDR / Lightroom LR-23 (HDR merge MISSING, weight 1.0; `research/parity-20261023-adobe-print.json` lightroom_classic top_gaps_remaining rank 2). No HDR merge, tone-mapping or image auto-align code exists at c6b3733f (git grep: none).

Spec (grep by id, read only these): `[STU-RAS-033]` (14-studio-creative-suite.md:1478: merge-bracketed-exposures-to-HDR with ghost removal and tone-mapping; tone-mapping between 32/16/8-bit; 32-bit storage beyond display range), `[STU-RAS-149]` (:1484: depth capability metadata; a command at an unsupported depth fails typed naming the depth and the supported set; no silent promotion), `[STU-RAS-044]` (:1799: native on-device, offline), `[STU-RAW-129]` (:2617: HDR develop state, read only to keep names consistent; HDR edit mode itself is NOT this lane). Vendor behaviour (Photoshop's Merge to HDR Pro presets, Lightroom's auto-align/auto-settings/deghost amounts) is UNVERIFIED (D5): implement declared Studio readings and say so.

Slices (commit locally per slice; one bundled run per push):

1. pigment `hdr_merge`: inputs are N >= 2 linear-light RGB f32 frames of equal size with exposure values (EV or relative exposure times; refuse typed when missing, mismatched sizes, N < 2, non-finite). Weighted radiance merge in linear light (declared hat weight that drops near-black noise and near-clip samples; clipped samples never contribute when a non-clipped sample exists); output a 32-bit f32 frame whose values may exceed 1.0. Deterministic (fixed summation order). Tests: `merge_recovers_known_radiance_from_synthetic_brackets` (exact within 1e-4 for a scene rendered at -2/0/+2 EV), `clipped_highlights_come_from_the_darker_frame`, `merge_refuses_mismatched_or_unexposed_inputs`, `merge_is_deterministic`.
2. pigment alignment + deghost + tone-mapping: translation-only alignment by median-threshold bitmaps (Ward 2003, published algorithm; implement from the paper) with a bounded search; deghost by per-pixel consistency against a chosen reference exposure (declared reading: inconsistent pixels take the reference frame's radiance; amount 0..3 levels declared); tone-mapping 32 -> 16/8 with a declared global operator (exposure + Reinhard-style compression + gamma to the output space through prism, no ad hoc gamma) and the `[STU-RAS-149]` capability metadata for the new commands. Tests: `misaligned_bracket_is_shifted_back` (known integer shift recovered), `moving_object_is_not_doubled_with_deghost_on`, `tone_map_keeps_order_and_range`, `unsupported_depth_is_refused_typed`.
3. develop entry: merge bracketed raw sources through the existing develop pipeline to linear scene-referred frames (no tone curve, no output transform) and pigment's merge; refuse typed when the brackets differ in camera, size or CFA layout; EVs from the raw metadata the decoders already read. Tests on the existing raw fixtures (synthesise brackets by scaling one fixture if no real brackets exist; state that as NOT_PROVEN for real brackets).
4. (only if slices 1-3 are green with time left) float DNG output of the merged result: extend `src/dng_write.rs` additively with SampleFormat 3 (IEEE float, 32 bits, uncompressed) per the public Adobe DNG specification 1.4+; existing integer output byte-identical. Otherwise record the exact signature needed in the notes.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-hdr-merge-1`)

- pigment: NEW `src/hdr_merge.rs` (submodules under `src/hdr_merge/` allowed), `src/lib.rs` (mod + pub use + DESCRIPTOR hunks only), NEW `tests/hdr_merge.rs`.
- develop (slices 3-4): NEW `src/hdr_merge.rs`, `src/lib.rs` (mod hunk), `src/dng_write.rs` (slice 4 additive only), NEW `tests/hdr_merge.rs`.
- Do NOT touch: composite, folio (SMB-AE-MASK-FEATHER-1), harness-core (SMB-PS-ACTIONS-1), harness-view, layout, root Cargo.toml, Cargo.lock, tools/, .config/. No new dependencies or edges (develop -> pigment already exists). Never read GPL/LGPL/MPL source (OpenCV is Apache-2.0 and may be read; papers may be read; cite what you use in the notes).

</topic>

<topic id="proof" status="active" summary="Proof and push coordination">

## Proof

- One bundled tools/lane-check.sh run on C: (`--disk ssd`) per push over the WHOLE pigment suite, plus the whole develop suite when develop changes (rule 4a). New items only, so no dependents unless an existing public signature changes (say so in the notes).
- PUSH HOLD: pigment is upstream of composite (SMB-AE-MASK-FEATHER-1 is mid-run). Commit locally; after the green run set HOLD_FOR_RELEASE with the green sha and end your turn; the lead messages RELEASE.
- NOT_PROVEN lines printed from one test: Photoshop / Lightroom HDR parity (no D5 oracle), real camera brackets if synthesised.
- Estimated gain: Lightroom LR-23 MISSING -> PARTIAL (HDR half; panorama stays missing); Photoshop STU-RAS-033 HDR merge engine.

</topic>
