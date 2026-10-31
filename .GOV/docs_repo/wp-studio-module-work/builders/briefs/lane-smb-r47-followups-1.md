---
file_id: LANE-SMB-R47-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-31T11:20:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-R47-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r47-followups-1.json
crates: [hsk-studio-reel (audio decode sample budget, additive), hsk-studio-harness (cli export-sequence only), hsk-studio-harness-core (manual.rs export-sequence entry + its test), hsk-studio-render-gpu (tests only), hsk-studio-interop-fig-base (resolve_variables)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree, HOST INTEGRATION)
---

<topic id="target" status="active" summary="Close R47 findings F-R47-1..4 (R48 findings appended at dispatch if any)">

## Target

Read each entry's `finding`, `repair_direction` and `file_line` in `validation/round-R47.json` → `findings_not_failures` first.

1. **F-R47-1 (LOW), one audio decode exceeds the decoded budget.** `cli_audio_within` (harness `src/cli.rs`, call at :835) passes a frame limit to `reel::decode_audio_file` (reel `src/audio/mod.rs:223`) and charges frames x channels x 4 only after the decode; reel's `decode_all` (`src/audio/engine.rs:247`) bounds kept frames, not samples, and admits up to `MAX_AUDIO_CHANNELS = 8` (`mod.rs:67`), so one 8-channel FLAC can hold 16.6 GB before `audio_too_large`.
   - Add a sample budget to reel's whole-file decode, additively: existing `decode_audio_file` callers (reel tests, harness tests, harness-view `audio_batch.rs:637`) keep their behaviour unchanged. Use an extra limit or a new entry point; do not change the meaning of the existing frame argument.
   - `decode_all` checks kept samples (frames x channels) per block against that budget and refuses with a typed error before the block is kept.
   - The harness passes `remainder / 4` as samples. Red test: a constant-subframe 8-channel FLAC (a few KiB) whose decode would exceed the budget is refused with no allocation above the budget. Assert the refusal, not timing.
2. **F-R47-2 (LOW), manual entry stale.** harness-core `src/manual.rs:27` (export-sequence `CommandSpec`): add `audio_too_many`, `audio_too_large` and the bounds (16 `--audio` files, 1 GiB per audio file, the 4 147 200 000-byte decoded budget). Add a test that every refusal code `cli.rs` emits for export-sequence appears in that entry. If the harness owns the code list, the test lives in the harness and reads the manual through harness-core's public API.
3. **F-R47-3 (LOW), DX12 side of the threshold fix.** In render-gpu `tests/gpu_plan.rs`, run `threshold_tie_decides_as_pigment_at_every_cut_step` and `threshold_8bit_ties_at_level_128_match_pigment` also on `TransientAdapter::dx12_for_tests` in the test-faults units, as the FXC rows do (Windows only; a `NOT_PROVEN:` line when no DX12 adapter opens). Tests only; no kernel change unless the DX12 row fails. A DX12 failure is evidence: record bits per backend as in F-R46-1 and stop for the lead before changing `kernel.wgsl`.
4. **F-R47-4 (LOW), fig-base alias walk.** interop-fig-base `src/nodes.rs` `resolve_variables` (:2083; linear `values.iter().find` per alias step): index each variable's values by mode once (HashMap), or bound value lists at decode as GRID now does. Poll the cancel token (the function takes `cx`). Red test: a hub variable with many values and many aliasing variables completes within a work bound or is refused, and a cancelled token stops it.

Out of scope:
- F-R47-8: the scoped default unit's package set. Root input `tools/run-round.sh`, a steward change.
- F-R46-8: the harness dev-dependency, which belongs to the GUI lane.
- No harness-view change.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "central_broker_client", "diagnostics": {"internal_diagnostics": "none", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r47-followups-1`, base set at dispatch)

- reel: `src/audio/mod.rs`, `src/audio/engine.rs` (the sample budget only) and a new or existing reel audio test file.
- harness: `src/cli.rs` (export-sequence audio path) and `tests/cli_export_sequence.rs`.
- harness-core: `src/manual.rs` (export-sequence entry only), plus the manual test if it lives there.
- render-gpu: `tests/gpu_plan.rs` (threshold rows only).
- interop-fig-base: `src/nodes.rs` (`resolve_variables` and the variable value decode) and its tests.
- No crate `Cargo.toml` change (feature-set changes rebuild dependents under new identities, F-R46-8); root inputs and the Master Spec stay untouched.

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Red first per finding; pushed red commits must compile.
- One bundled run on the lane disk the lead names: the whole suites of reel, harness (export-sequence and audio tests under the round's units), harness-core, render-gpu (round units plus the test-faults units; DX12 rows explicit) and interop-fig-base, plus clippy over the changed crates' direct dependents (rules 4a and 4e). Name the log per RUN LOGS; lane-check prints the `HEAD=` header.
- Vendor behaviour stays UNVERIFIED (D5).

</topic>
