---
file_id: LANE-SMB-R50-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-31T22:54:36Z
wp: WP-KERNEL-STUDIO
lane: SMB-R50-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r50-followups-1.json
crates: [hsk-studio-composite (Foam / Wave World recovered STU-FX-131a records, docs), hsk-studio-folio (additive record enum variants only, if a row needs one), hsk-studio-interop-psd (typed loss for a transcoded 1006 channel name), hsk-studio-harness-core (import keeps channels without layer records)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R50 F-R50-1 (with MT-32012 / MT-32013), F-R50-5 and F-R50-6">

## Target

Read each entry's `finding`, `repair_direction` and `file_line` in `validation/round-R50.json` → `findings_not_failures` first.

1. **F-R50-1 (LOW) + MT-32012 (Foam) / MT-32013 (Wave World): recovered records.** The water-sims brief wrongly said no typed record was recovered; the spec (v02.225 `14-studio-creative-suite.md`) recovers Foam `APC Foam` at :15367 (47 rows) and Wave World `APC Wave World` at :15433 (48 rows), STU-FX-131a. STU-FX-146 authoring applies only where no record exists.
   - LEAD DECISION G2 (2026-10-31T22:54:36Z, Operator overnight waiver; spec owner may override, SD-22): MAP, recovered first. Store every recovered row of both tables as a typed record with its seven fields separate (hard_min, hard_max, soft_min, soft_max, default, unit, precision), `--` stored as unknown and never mirrored from the opposite bound, kind, 1-based enumeration options with the declared default index, and flags mapped per STU-FX-113 (`CANNOT_TIME_VARY` → `animatable = false`, `CANNOT_INTERP` → `interpolable = false`). These are the MT-32012 / MT-32013 acceptance criteria: read both MT JSONs (`.GOV/task_packets/WP-KERNEL-STUDIO/`), one assertion per row.
   - Record type: reuse folio `src/composition/effects.rs` `StudioEffectParamSpec` with `StudioBoundState`, `StudioParameterKind`, `StudioUnitToken` (composite already depends on folio). If a row needs a unit or kind folio lacks, add the variant to folio additively; no second record type.
   - Engine mapping: where a recovered parameter is the same quantity as an engine scalar, the engine takes its hard bounds (enforced) and default from the recovered row, and the authored row goes. Engine scalars with no recovered counterpart stay authored `studio_declared` rows, labelled as STU-FX-146 scope beside a recovered record. Recovered parameters the engine does not use yet are stored and validated, and documented as not yet driving the simulation: no invented behaviour. Put the mapping table (recovered row → engine scalar or none, unit conversion if any) in the lane notes; vendor parity UNVERIFIED (D5).
   - Existing tests change only where a mapped default or bound changes; list each changed assertion with old → new value and the row that drives it (the validator judges loosening). Determinism, budgets and refusals stay as they are.
   - Fix the false-basis docs: composite `src/simulations.rs:12-18`, `src/simulations/wave_world.rs:53`, `src/simulations/foam.rs:39`.
   - MT-31728 / MT-31731 (catalogue rows for Foam / Wave World): read them; claim only what is met.
2. **F-R50-5 (INFO), interop-psd 1006 channel name.** When the legacy 1006 Pascal name cannot carry a renamed channel's name exactly (a character outside Latin-1 replaced by `?`, or the cut at 255 bytes), the edit records the typed loss `channel_name_transcoded`, as layer names record `layer_name_truncated`. The 1045 Unicode path is unchanged. Red test beside `tests/psd_alpha_channels.rs`.
3. **F-R50-6 (INFO), harness-core import drops channels.** `src/import.rs:787` writes `proposal.raster_records` only when `layers` is non-empty, so a PSD whose records hold channels but no layer records loses its channels on import. Gate on layers OR channels, and store the channel tiles as package assets through the package channel-tile API the folio lane added (harness-core already inserts layer and mask tiles the same way at :775-785). Red test in harness `tests/import_psd.rs`: a PSD with alpha channels and no layer records imports with its channels and their tiles. Reuse interop-psd's test PSD builder if reachable; otherwise a minimal one in the test.

Out of scope:
- F-R50-3 and F-R50-4: spec-owner entries (SD-23, SD-24). F-R50-2, F-R50-7, F-R50-8: no asset change. MEDIUM F-R46-8: the paused GUI lane.
- No other composite simulation change; no egui, eframe, winit or wgpu line (D1); no crate `Cargo.toml` change (artifact-identity trap F-R46-8); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: no mutating command added; the PSD channel edit and the import keep their existing context", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r50-followups-1`, branch `lane/smb-r50-followups-1`, base 63ecf5a6)

- composite: `src/simulations.rs` (module docs and the shared record helpers only), `src/simulations/foam.rs`, `src/simulations/wave_world.rs`, `tests/simulations_water.rs` and a new records test file.
- folio: `src/composition/effects.rs` (additive `StudioUnitToken` / `StudioParameterKind` variants only, if needed).
- interop-psd: `src/export.rs`, `src/resources.rs` (`encode_alpha_names_pascal`), `src/proposal.rs` (the loss code) and `tests/psd_alpha_channels.rs`.
- harness-core: `src/import.rs` (the records gate and channel tiles only).
- harness: `tests/import_psd.rs`.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; pushed red commits must compile the round units of their changed crates and direct dependents (RED COMMITS).
- One bundled run on C: (lane-check semantics, one lock hold): step 0; clippy over the changed crates and their direct dependents (lane-check computes them; folio's dependents are wide, so expect a long clippy if folio changes); nextest WHOLE suites of composite (default and `motion` units), interop-psd, harness-core (its import feature units) and harness (the round's import-psd units, 4e), and folio if it changed (4a); doctests if added (4d). Name the log per RUN LOGS.
- Push order: nib, motion, type and prism (this wave's other four lanes) all sit in this lane's closure, so this lane pushes LAST, after those four are on origin or the lead releases it: rebase onto origin, then `lane-check.sh --owner smb-r50-followups-1 --per-sha origin/codex/handshake-studio-assets`; push only on `PER_SHA_RESULT OK`.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
