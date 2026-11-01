---
file_id: LANE-SMB-PRESS-OBSERVE-1
file_kind: lane_brief
updated_at: 2026-11-01T13:05:59Z
wp: WP-KERNEL-STUDIO
lane: SMB-PRESS-OBSERVE-1
lane_file: builders/lanes/hsk-studio-press-observe-1.json
crates: [hsk-studio-press-write (observe dependency; terminal outcomes of coverage evaluation and ink edits through observe)]
model: sonnet
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close R51 F-R51-7: press-write emits its terminal outcomes through observe (HOST INTEGRATION rule 5)">

## Target

Read `validation/round-R51.json` F-R51-7 and the lane files of SMB-PRESS-INK-COVERAGE-1 and SMB-PRESS-INK-MANAGER-1 (both set `internal_diagnostics` to none because press-write had no observe edge). `builders/WORKSPACE_RULES.md` topic path-deps: every crate may use accord + observe (folio -> accord only), so this edge needs no lead decision.

1. **Dependency.** Add `hsk-studio-observe = { path = "../hsk-studio-observe", version = "=0.1.0", default-features = false }` to `crates/hsk-studio-press-write/Cargo.toml` `[dependencies]`, exactly in the form the rules state. This is the one crate `Cargo.toml` change this lane is allowed; root `Cargo.toml` and `Cargo.lock` stay untouched and uncommitted (the next steward sync derives `Cargo.lock`; the next round runs the canary).
2. **Emission.** Every terminal outcome of the total-ink-coverage evaluation (`coverage.rs`: within limit, within warning margin, over limit, not checked, indeterminate, and refusals) and of the ink-manager edits (`inks.rs`: applied, refused with its typed code) is emitted through observe in the form other crates use (read observe's producer API and an existing producer, e.g. layout or composite): codes, counters and opaque ids only, never text, never document content. Emission never changes a result.
3. **Tests.** Red first: an observe test sink (the pattern existing crates use) sees exactly one terminal event per evaluation and per ink edit, with the right code and no text payload; results are unchanged with and without a sink.
4. **Lane files.** Set `internal_diagnostics` to `producer_through_observe` in this lane's host_integration block and record which outcomes emit which codes.

Out of scope: any other press-write behaviour; no egui, eframe, winit or wgpu line; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-press-observe-1`, branch `lane/smb-press-observe-1`, base c420f273)

- press-write: `Cargo.toml` (the one observe line), `src/coverage.rs`, `src/inks.rs`, `src/lib.rs`, tests.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first, proven red by running it; a pushed red commit must compile the round units of press-write and its direct dependents.
- One bundled run on C: (`lane-check.sh --owner smb-press-observe-1`): step 0; clippy over press-write and its direct dependents; nextest WHOLE suites of press-write and press.
- Push when green: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form. Tell the lead in your final message that the steward sync must follow (Cargo.lock).
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
