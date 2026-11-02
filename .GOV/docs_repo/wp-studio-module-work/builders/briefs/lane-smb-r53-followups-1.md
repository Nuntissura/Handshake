---
file_id: LANE-SMB-R53-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-11-02T02:34:03Z
wp: WP-KERNEL-STUDIO
lane: SMB-R53-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r53-followups-1.json
crates: [hsk-studio-harness-core (host adapter emitting folio swatch / book reports through observe; prism named-colour -> folio colour-book join), hsk-studio-interop-tiff and hsk-studio-harness-core tests (TIFF typed outcomes)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R53 F-R53-1 and F-R53-2, and the MT-31251 prism -> folio join">

## Target

Read `validation/round-R53.json` F-R53-1, F-R53-2 and F-R53-6 (finding, repair_direction, file_line) and the lane files of SMB-COL-SWATCH-GROUPS-1 and SMB-TIFF-CHANNELS-1 first. harness-core already depends on folio, observe, prism and (feature `import-tiff`) interop-tiff, so no new edge is needed.

1. **F-R53-1, folio reports through observe.** folio may depend only on accord, so its new entries (`apply_swatch_group_command`, `undo_swatch_group_command`, `add_colour_book_swatches`, `StudioColourBookLibrary::load` / `mark_absent`) return code-and-counter reports with an opaque correlation (`StudioSwatchEditReport`, `StudioBookReport`). Add a host adapter in harness-core that emits each report through observe in the form other producers use: codes, counters and the opaque correlation only, never names, ids, colours or document content; emission never changes a result. Tests with an observe test sink: one terminal event per report, right code, no text payload (a needle test over swatch, group and book names).
2. **MT-31251, prism -> folio colour-book join.** SMB-COL-SWATCH-GROUPS-1 added an ICC named-colour parser in prism (`wide.rs`) and a typed profile reference in folio, with no crate joining them. In harness-core, build a folio colour book from a parsed named-colour profile (the folio book's adapter input), honouring the per-book licensing posture (STU-COL-186: a book whose posture forbids bundling is only referenced) and the budgets; test with a synthetic named-colour profile (invented names only; never branded data).
3. **F-R53-2, TIFF typed outcomes.** Add the five negative tests: export losses `tiff_channel_solidity_not_exported` and `tiff_channel_visibility_not_exported`, and the tile errors `channel_tile_size`, `channel_tile_format`, `channel_tile_grid`, each asserting the typed outcome and no partial output. Red proof: as F-R53-7 accepted, a temporary uncommitted mutation that removes the check must make the test fail (log it).
4. **F-R53-6 (INFO), optional if cheap:** validate restored swatches in undo, as the repair direction says.
5. **MTs.** Claim only what is met: MT-31251 (join), MT-31248 / MT-31249 rows the join proves, MT-30095 TIFF rows the new tests touch.

Out of scope: a document TIFF export command (Registry id, RUN_COMMANDS row; needs the controls registry: later lane); EXR (SD-41); GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe (harness-core emits folio's code-only reports)", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: the adapter forwards the caller's context; loading a book stays a mutating edit with an ActorContext", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r53-followups-1`, branch `lane/smb-r53-followups-1`, base 92ae2232)

- harness-core: a new folio-report adapter module, a new colour-book join module, `lib.rs` exports, tests.
- interop-tiff: tests only. folio: `src/vector_tools.rs` / the swatch-library module only for item 4.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it (or by the logged temporary mutation for item 3); a pushed red commit must compile the round units of the changed crates and direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-r53-followups-1`; C: is reserved for STUDIO-R53): step 0; clippy over harness-core (its round feature units, 4e) and its direct dependents; nextest WHOLE suites of harness-core in the feature units the round runs, interop-tiff, and folio if item 4 changed it (plus package, 4b).
- Push only after STUDIO-R53 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
