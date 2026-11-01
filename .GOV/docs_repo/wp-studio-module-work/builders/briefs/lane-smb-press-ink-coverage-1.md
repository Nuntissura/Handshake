---
file_id: LANE-SMB-PRESS-INK-COVERAGE-1
file_kind: lane_brief
updated_at: 2026-11-01T02:58:58Z
wp: WP-KERNEL-STUDIO
lane: SMB-PRESS-INK-COVERAGE-1
lane_file: builders/lanes/hsk-studio-press-ink-coverage-1.json
crates: [hsk-studio-press-write (total ink coverage contract on the separations path)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Total ink coverage contract STU-LAY-250..254 on press-write's separations">

## Target

Spec: Master Spec `14-studio-creative-suite.md` (v02.226 lines; v02.227 changes only wgpu/egui wording) STU-LAY-250..254 at :7236-7392. Read them whole, plus STU-LAY-100 (unknown-bound rule), STU-LAY-154 (preflight severity vocabulary), STU-LAY-160 (ink contract: spot-to-process, ink alias, `ink_type`), STU-LAY-161 (flattener), STU-LAY-166 (constraint and error vocabulary) and STU-LAY-057 (findings to the command surface).

Existing code: `hsk-studio-press-write/src/separations.rs` already has an ink-limit preview (`InkLimits`, `LimitKind`, `Severity`, `InkLimitFinding`, `InkLimitReport`, `PlateCoverage`, `InkReading`) and `src/inks.rs` the `InkManager`. Evolve these to the contract; do not add a second ink-limit model beside them.

1. **STU-LAY-250, parameter table.** `total_ink_limit`, `single_ink_limit`, `black_ink_limit`, `coverage_warning_margin`, `coverage_sample_resolution` as seven-field records (bounds separate, `unknown` stored as unknown and never mirrored). The three limits are three independent fields. The 400 `hard_max` is the four-ink case: for a separation of n inks the structural maximum is 100 x n and the engine scales, never clamps to 400. `coverage_warning_margin` is its own field, never inferred.
2. **STU-LAY-251, sources and precedence.** `ink_limit_source` ordered enumeration; the effective limit is the first DECLARED site in member order and the result records which site supplied it. `undeclared` is terminal: evaluation then reports `check_state.not_checked`, never a pass. `ink_limit_severity` uses the preflight severity vocabulary of STU-LAY-154 with exactly its four member tokens: if press-write's existing `Severity` differs, change it to those tokens rather than keeping a second scale, and list each changed assertion. `ink_limit_scope` and `ink_limit_check_state` (five members; `indeterminate` for a placed asset that cannot be separated: missing link, unresolvable profile, passthrough) as typed enumerations, never collapsed.
3. **STU-LAY-252, evaluation.** Evaluate on the composed and separated device values at a declared sample resolution, never on authored swatch values. The three metrics (`maximum`, `mean`, `area_above_limit`) are all computed and the sample resolution is part of the result. Opaque and opaque-ignore inks contribute per their declared `ink_type`, not as a plain sum (read how `inks.rs` types inks). `coverage_input_stage`: attribute `introduced_by` for the stages this pipeline actually performs (spot-to-process conversion and ink aliasing in the ink manager, overprint composition in the separations preview, authored rich black, opaque ink handling); a stage the pipeline does not perform yet (for example transparency flattening, placed-image separation) is a typed member that is never reported as the cause, and the lane notes say which stages are live. No invented behaviour.
4. **STU-LAY-253, finding.** A typed structured finding with the thirteen fields of the table (rule id, severity, check state, effective limit, limit source, measured maximum, mean, area above limit, per-ink contribution at the worst location, page reference with region, object references, introduced-by stage, sample resolution), serialisable as data. Reaching the model-steerable command surface (STU-LAY-057) is a host-side follow-up: say so in the lane notes; claim only the record.
5. **STU-LAY-254, no default.** Studio ships with `total_ink_limit` undeclared and reports `not_checked` until a site declares one; a test proves it. The published reference figures are not a default and are not stored as Studio names. Studio never re-separates a document to get under a limit (no repair inside an export).
6. **MTs.** Read each MT JSON under `.GOV/task_packets/WP-KERNEL-STUDIO/` and claim only what is met, one assertion per concrete acceptance row: MT-35425, MT-35426, MT-35427, MT-35428, MT-35429, MT-35430, MT-35431, MT-35432; MT-35695 is a source companion spanning ink, overprint, trap, separations and the flattener: claim only its ink-coverage rows.

Existing tests change only where a contract changes a pinned value or token; list each changed assertion with old -> new and the clause that drives it (the validator judges loosening). Budgets and refusals (`SepLimits`, `SepError`) stay as they are or get tighter. Hostile sizes (V4-01/02): ink counts, sample grids and region lists are budgeted with typed refusals; `area_above_limit` and means use checked arithmetic. Vendor parity UNVERIFIED (D5).

Out of scope:
- Trap presets (SMB-ID-TOC-INDEX-1 owns STU-LAY-152 in layout), the flattener (STU-LAY-161), the preflight rule engine in layout, any command-registry row (control_descriptors none).
- No egui, eframe, winit or wgpu line (D1, D7); no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.226", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: the evaluation is a read-only check; no mutating command added", "jobs": "none: evaluation stays a bounded call under SepLimits", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-press-ink-coverage-1`, branch `lane/smb-press-ink-coverage-1`, base 31454a07)

- press-write: `src/separations.rs`, `src/inks.rs` (ink-type reads and additive accessors only), a new coverage module if you split it out, `lib.rs` exports and DESCRIPTOR text, existing separations tests where a pinned token changes, and new test files.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item; a pushed red commit must compile the round units of its changed crates and direct dependents (RED COMMITS).
- One bundled run on D: (`lane-check.sh --owner smb-press-ink-coverage-1`, one lock hold; D: because C: is busy with this wave's pushes): step 0; clippy over press-write and its direct dependents; nextest WHOLE suite of press-write and of every direct dependent that pins the changed API (4a, 4b); doctests if added (4d). Name the log per RUN LOGS.
- Push after SMB-R50-FOLLOWUPS-1 is on origin (`git log origin/codex/handshake-studio-assets`) or when the lead releases you: rebase, `lane-check.sh --owner smb-press-ink-coverage-1 --per-sha origin/codex/handshake-studio-assets`, push only on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
