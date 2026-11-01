---
file_id: LANE-SMB-R52-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-11-01T12:58:23Z
wp: WP-KERNEL-STUDIO
lane: SMB-R52-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r52-followups-1.json
crates: [hsk-studio-interop-psd (typed-loss tests, fractional opacity loss), hsk-studio-interop-tiff (Folio channel records as extra samples)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R52 F-R52-3 and the TIFF part of F-R52-2">

## Target

Read `validation/round-R52.json` F-R52-2 and F-R52-3 (finding, repair_direction, file_line) first.

1. **F-R52-3, interop-psd.** `src/export.rs` `added_channel_fits` keeps `channel_added_in_edit_not_exported` for five conditions; two are tested. Add red-first negative fixtures for the three untested branches (multichannel mode or a depth other than 8/16/32; no merged image data; extra plane 0 of a merged-transparency file), each asserting the typed loss and an unchanged file. `channel_edits_in_place` rounds a fractional channel opacity with `.round().clamp(0, 100)` and records nothing: record a typed transformed loss `channel_opacity_rounded` (in the proposal's loss vocabulary, as `layer_name_truncated` and `channel_name_transcoded` are) whenever the stored opacity is not integral, with a test.
2. **F-R52-2 (TIFF), interop-tiff.** Spec v02.229 STU-RAS-022 (:844), STU-RAS-043 (:1885) and STU-IO-006 row 3 (TIFF, I-EP-X-RT, alpha preserved): map Folio channel records (`StudioRasterRecords.channels`, the folio channel API the SMB-FOLIO-CHANNEL-RECORD lane added) to and from TIFF extra samples. interop-tiff already scans and encodes ExtraSamples (`src/scan.rs`, `src/encode.rs`, `src/model.rs`); add the record mapping on import (each extra sample beyond the declared associated / unassociated alpha becomes a channel record with its kind as the extra-sample value says) and on export (each channel record becomes an extra sample plane with the right ExtraSamples value), at 8/16/32 bits as the TIFF supports, with typed losses where TIFF cannot carry a record field (names, display colour, solidity: say which). Red test: a document with an alpha channel and a channel saved from a selection exports to TIFF and re-imports with both channel records and identical pixels. Check whether interop-tiff may depend on folio under WORKSPACE_RULES path-deps; if the edge is not allowed, stop that item and report (a new edge is a lead decision; no Cargo.toml change in this lane).
3. **EXR (F-R52-2, row 9).** Do not implement. Find which crate reads or writes OpenEXR (if any) and record in the lane notes what a channel-record mapping there would need; the lead decides next.
4. **MTs.** Claim only what is met: MT-30095 (round-trippable interchange rows for TIFF), MT-038 rows touched by item 1.

Hostile input (V4-01/02): extra-sample counts and plane sizes are budgeted. Vendor parity UNVERIFIED (D5).

Out of scope: EXR implementation, other formats, GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: import and export keep their existing context", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r52-followups-1`, branch `lane/smb-r52-followups-1`, base 99836457)

- interop-psd: `src/export.rs`, `src/proposal.rs` (loss code), `tests/psd_alpha_channels.rs` and new test files.
- interop-tiff: `src/scan.rs`, `src/encode.rs`, `src/model.rs`, a new channel-mapping module, `lib.rs` exports and DESCRIPTOR, tests.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of the changed crates and direct dependents (RED COMMITS).
- One bundled run on D: (`lane-check.sh --owner smb-r52-followups-1`): step 0; clippy over interop-psd, interop-tiff and their direct dependents; nextest WHOLE suites of interop-psd and interop-tiff, plus harness's import units that read TIFF / PSD (4e); doctests if added.
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form (re-fetch and rebase on a non-fast-forward).
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
