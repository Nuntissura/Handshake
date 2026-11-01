---
file_id: LANE-SMB-R51-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-11-01T08:22:53Z
wp: WP-KERNEL-STUDIO
lane: SMB-R51-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r51-followups-1.json
crates: [hsk-studio-composite (Wave World / Foam unitless-row mappings per SD-27), hsk-studio-interop-psd (export a channel added in edit)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree, V4-06 failure class)
---

<topic id="target" status="active" summary="Close R51 F-R51-1 (MEDIUM) and the MT-038 / MT-30095 interchange gap">

## Target

Read `validation/round-R51.json` first: `findings_not_failures` F-R51-1 (finding, repair_direction, file_line) and `mt_judgments` for MT-038 and MT-30095.

1. **F-R51-1 (MEDIUM), unitless recovered rows driving engine scalars with an invented scale.** composite `src/simulations/wave_world.rs` `WAVE_PRODUCER_MAPPINGS` maps Wave World producer frequency (recovered row 54) and amplitude (row 52) to the engine with conversion none (k = 1), and the engine scalars took the recovered bounds. Spec v02.229 (STU-FX-131a note :13825-13830, STU-CMP-075b :30092-30102, Wave World table :15662-15715): a `--` unit cell is the `unitless` token, the per-row engine scale is a SPEC GAP never guessed, and until it is declared the engine's own scalar stays a `studio_declared` row under STU-FX-146 beside the stored recovered row.
   - Failure class (V4-06): a recovered STU-FX-131a row whose unit is `--` (unitless) mapped onto an engine scalar that carries a unit or drives the simulation, with any conversion (k = 1 included). Search regex: the mapping tables in `src/simulations/wave_world.rs` and `src/simulations/foam.rs` (`*_MAPPINGS`, `conversion`, `Conversion::None`), cross-checked against the recovered rows' unit cells. List every site in the lane notes as fixed or not applicable (a unit-carrying recovered row with a declared physical unit is not this class).
   - Fix: for each site, the engine scalar returns to its authored `studio_declared` row (bounds and default as at base 63ecf5a6, before SMB-R50-FOLLOWUPS-1 replaced them; `git show 63ecf5a6:<path>`), and the recovered row stays stored and validated beside it, documented as not yet driving the simulation (SPEC GAP SD-27). Mappings of rows with a declared unit stay as they are.
   - Tests: change `tests/simulations_records.rs` (the assert_mapping cases for amplitude and frequency, about :350-365) to assert the SD-27 reading (recovered row stored, engine scalar studio_declared with its authored bounds); list each changed assertion old -> new and the clause. Determinism and budgets unchanged.
2. **MT-038 / MT-30095, a channel added in edit is not exported.** interop-psd export refuses a NEW channel (saved from a selection) with the typed loss `channel_added_in_edit_not_exported`; only edits to imported channels write back. Make export write a new alpha (and spot, if the record kind says so) channel: its plane in the merged image data after the existing channels, its name in the 1006 Pascal list and the 1045 Unicode list (with the `channel_name_transcoded` loss rule as now), its display info in 1077 as the folio channel record says, at 8, 16 and 32 bits per the document depth. Re-import yields the same channel record and pixels (32-bit identical, 16-bit identical, 8-bit quantised exactly as the folio record's save rule states). Keep the typed loss only where PSD truly cannot carry the edit, and say which case that is.
   - Tests beside `tests/psd_alpha_channels.rs`: red first: a document with one imported alpha channel plus one channel saved from a selection exports and re-imports with both channels, names and pixels; the 1006/1045/1077 resources list both; channel count over the format limit still refuses typed.
3. **MTs.** Read MT-038 and MT-30095 (and the Wave World MTs MT-32013 / MT-34181 rows for frequency / amplitude) and claim only what is met.

Out of scope: declaring any engine scale (the spec owner's SD-27 gap); other simulations; other PSD resources; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: export and the simulation keep their existing context; no new mutating command", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r51-followups-1`, branch `lane/smb-r51-followups-1`, base a7be22a1)

- composite: `src/simulations/wave_world.rs`, `src/simulations/foam.rs` (mapping tables only), `src/simulations.rs` (module docs), `tests/simulations_records.rs`, `tests/simulations_water.rs`.
- interop-psd: `src/export.rs`, `src/resources.rs`, `src/proposal.rs` (loss-code docs), `tests/psd_alpha_channels.rs` and a new test file if needed.
- SMB-FOLIO-ANGLE-UNBOUNDED-1 also edits `tests/simulations_records.rs` (ANGLE bound cells) and pushes before you: rebase over it and keep both changes.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of composite, interop-psd and their direct dependents (RED COMMITS).
- One bundled run on C: (`lane-check.sh --owner smb-r51-followups-1`): step 0; clippy over the changed crates and their direct dependents; nextest WHOLE suites of composite (default and `motion` units) and interop-psd, plus harness's import-psd units (4e); doctests if added.
- Push after SMB-FOLIO-ANGLE-UNBOUNDED-1 is on origin (`git log origin/codex/handshake-studio-assets`), or when the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
