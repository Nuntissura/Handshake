---
file_id: LANE-SMB-R54-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-11-02T14:03:34Z
wp: WP-KERNEL-STUDIO
lane: SMB-R54-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r54-followups-1.json
crates: [hsk-studio-type (forced line break U+2028; terminator fast path), hsk-studio-layout (marker attribution before overset)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R54 F-R54-3 (U+2028), F-R54-6 and F-R54-4">

## Target

Read `validation/round-R54.json` F-R54-3, F-R54-4 and F-R54-6 (finding, repair_direction, file_line) and the lane notes of SMB-SENTINELS-NATIVE-1 (NATIVE-2) first. Spec: Master Spec v02.229 STU-TYP-207 (:9106-9160): the forced line break is a member of the break catalog.

1. **F-R54-3, forced line break.** Type refuses U+2028 with MissingGlyph. Per STU-TYP-207 the forced line break ends the LINE, not the paragraph: the engine treats U+2028 as a mandatory line break (UAX #14 class BK), never shapes it as a glyph, and keeps the stored code point. Bidi treats it per the spec / UAX #9 (U+2028 is class WS; it does not end the paragraph). Red test: a paragraph with U+2028 breaks the line there in the default and harfrust units; the NOT_PROVEN forced-line-break host test in interop-layout (`tests/sentinels_native_type.rs`) then proves it (turn its NOT_PROVEN into a real assertion).
2. **F-R54-6, marker at a line end before overset.** A marker at the end of a line directly before overset text is attributed to the line before. Fix the attribution so a marker resolves for the line (and page) it sits on; red test from the validator's case (read its probe in `Handshake_Artifacts/WP-KERNEL-STUDIO/validator-r54-probe/`).
3. **F-R54-4, terminator cost.** The public `hsk_studio_type::ends_paragraph` runs a full lookup per character. Measure it (a bounded micro-benchmark or a counted-operations test, no wall-clock assertion) on a long story; if the cost is material, add a fast path (e.g. ASCII and the common BMP range answered without the full lookup) that returns identical results for every code point (exhaustive agreement test over all code points). Record the measurement either way.
4. **MTs.** Claim only what is met: MT-31008 (breaks: forced line break row), MT-31005 / MT-31007 rows touched.

Out of scope: OOXML / ODF mappings; GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: shaping and composition are not mutating commands", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r54-followups-1`, branch `lane/smb-r54-followups-1`, base 96ace3ca)

- type: the engine's line-break / segmentation and validation modules, `src/validate.rs`, `src/lib.rs`, tests.
- layout: the sentinel-resolution and reflow modules (marker attribution only), tests.
- interop-layout: `tests/sentinels_native_type.rs` (turn the NOT_PROVEN case into an assertion).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of the changed crates and direct dependents.
- One bundled run on C: (`lane-check.sh --owner smb-r54-followups-1`): step 0; clippy over type, layout, interop-layout and direct dependents; nextest WHOLE suites of type (default and harfrust units), layout and interop-layout; doctests if added.
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form (re-fetch and rebase on a non-fast-forward).
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
