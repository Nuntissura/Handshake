---
file_id: LANE-SMB-R47-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-31T13:07:07Z
wp: WP-KERNEL-STUDIO
lane: SMB-R47-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r47-followups-1.json
crates: [hsk-studio-reel (audio decode sample budget, additive), hsk-studio-harness (cli export-sequence only), hsk-studio-harness-core (manual.rs export-sequence entry, business-event entry path, tests), hsk-studio-render-gpu (tests only), hsk-studio-interop-fig-base (resolve_variables), hsk-studio-accord (business_event id only), hsk-studio-session (additive correlation submit path, additive deadline outcome), hsk-studio-harness-view (job correlations, deadline, two clippy allows)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree, HOST INTEGRATION)
---

<topic id="target" status="active" summary="Close R47 findings F-R47-1..4 and R48 findings F-R48-1/2/3/6/8">

## Target

Read each entry's `finding`, `repair_direction` and `file_line` in `validation/round-R47.json` → `findings_not_failures` first.

1. **F-R47-1 (LOW), one audio decode exceeds the decoded budget.** `cli_audio_within` (harness `src/cli.rs`, call at :835) passes a frame limit to `reel::decode_audio_file` (reel `src/audio/mod.rs:223`) and charges frames x channels x 4 only after the decode; reel's `decode_all` (`src/audio/engine.rs:247`) bounds kept frames, not samples, and admits up to `MAX_AUDIO_CHANNELS = 8` (`mod.rs:67`), so one 8-channel FLAC can hold 16.6 GB before `audio_too_large`.
   - Add a sample budget to reel's whole-file decode, additively: existing `decode_audio_file` callers (reel tests, harness tests, harness-view `audio_batch.rs:637`) keep their behaviour unchanged. Use an extra limit or a new entry point; do not change the meaning of the existing frame argument.
   - `decode_all` checks kept samples (frames x channels) per block against that budget and refuses with a typed error before the block is kept.
   - The harness passes `remainder / 4` as samples. Red test: a constant-subframe 8-channel FLAC (a few KiB) whose decode would exceed the budget is refused with no allocation above the budget. Assert the refusal, not timing.
2. **F-R47-2 (LOW), manual entry stale.** harness-core `src/manual.rs:27` (export-sequence `CommandSpec`): add `audio_too_many`, `audio_too_large` and the bounds (16 `--audio` files, 1 GiB per audio file, the 4 147 200 000-byte decoded budget). Add a test that every refusal code `cli.rs` emits for export-sequence appears in that entry. If the harness owns the code list, the test lives in the harness and reads the manual through harness-core's public API.
3. **F-R47-3 (LOW), DX12 side of the threshold fix.** In render-gpu `tests/gpu_plan.rs`, run `threshold_tie_decides_as_pigment_at_every_cut_step` and `threshold_8bit_ties_at_level_128_match_pigment` also on `TransientAdapter::dx12_for_tests` in the test-faults units, as the FXC rows do (Windows only; a `NOT_PROVEN:` line when no DX12 adapter opens). Tests only; no kernel change unless the DX12 row fails. A DX12 failure is evidence: record bits per backend as in F-R46-1 and stop for the lead before changing `kernel.wgsl`.
4. **F-R47-4 (LOW), fig-base alias walk.** interop-fig-base `src/nodes.rs` `resolve_variables` (:2083; linear `values.iter().find` per alias step): index each variable's values by mode once (HashMap), or bound value lists at decode as GRID now does. Poll the cancel token (the function takes `cx`). Red test: a hub variable with many values and many aliasing variables completes within a work bound or is refused, and a cancelled token stops it.

R48 findings (appended 2026-10-31 at dispatch; read each entry in `validation/round-R48.json` → `findings_not_failures` first):

5. **F-R48-1 (LOW), entry-only business-event ids collide across runs.** `set_document_entry` makes no revision, so RAW_DEVELOPED / TEXT_EDITED / VECTOR_PATH_EDITED and the undo/redo of entry-only steps are minted at the previous commit's revision; the per-revision sequence lives in the Run, so a second Run on the same saved package at the same revision mints an earlier event's id and the collision refusal drops the new event (accord `src/business_event.rs` `business_event_id`; harness-core `src/harness.rs` `business_event_entry`, `emit_entry_business_event`, the history entry-swap branch).
   - Measure first: check whether the ids your fix relies on (for example the RunContext correlation under the fixed test-host context) are themselves unique across two runs; a deterministic counter that restarts at 0 does not fix this.
   - Prefer an identity that survives runs without changing what the document revision means (other lanes' tests and goldens observe revisions). Make entry writes advance a revision only if no other route works, and then list every changed golden and test.
   - Red test: two runs on one saved package, each with an entry-only edit at the same revision, produce disjoint event ids and no counted collision.
6. **F-R48-2 (LOW), entry events untested and emitted on no-change.** Add tests for the TEXT_EDITED (entry token text) and vector entry mappings. Emit an entry event only when the entry bytes changed: compare with the `EntryRestore` bytes the step already holds, as the chronicle path does (`changed && !replayed`). Red test: rewriting identical entry bytes emits nothing (harness-core `src/harness.rs` `entry_business_event`, `emit_entry_business_event`; `tests/business_events_run.rs`).
7. **F-R48-3 (LOW), job frames carry ordinals.** harness-view `src/view/job_admission.rs` (`AdmittedExportQueue::submit` uses `pending.len()`, `record_superseded_preview` its frame revision) and `src/view/audio_batch.rs` (run loop uses `item.handle.id`): mint each submitted job's correlation from the viewer's RunContext correlation source, one per job, and carry it on every frame of that job.
   - Deadline: HBR-HOST-004 requires overload, cancellation, deadline, stale-revision and release behaviour for every job. Add a typed deadline outcome additively (lifecycle types are additive; the `JobPort` trait does not change), or, if a job class has no deadline by nature, record why in the lane notes with the job class named.
   - Document-less jobs (the hosted audio batch with nothing open) stay Unaddressed until MT-36228 defines a host scope: no change, keep the disclosure.
   - Red tests: two Export Media jobs carry different correlations, neither an ordinal; a job past its deadline ends in the deadline outcome (or the notes record why none).
8. **F-R48-6 (LOW), session commit frames carry the operation counter.** `Session::settle_with` passes `self.operations` to `emit_report` (session `src/session.rs:2139-2146`). Let the `Session::submit*` paths take the caller's `CommandContext` correlation through an additive session API and emit it; existing callers keep compiling. Extend harness-core `tests/run_attribution.rs` so that, for commits (rename, property, tile, path, structure, sequence), `observe.correlation` equals the op record's correlation.
9. **F-R48-8 (INFO), clippy false positive.** Add `#[allow(clippy::explicit_auto_deref)]` with a one-line reason at harness-view `src/view/audio_batch.rs:395` and `src/view/job_admission.rs:265` (the suggested code would not compile: `&&'static str` deref coercion yields a borrow-scoped `&str`).

Out of scope:
- F-R47-8: the scoped default unit's package set. Root input `tools/run-round.sh`, a steward change.
- F-R48-4: a per-sha compile check in lane-check (kernel tool, steward); this lane follows the RED COMMITS rule as the dispatch states it.
- F-R48-5, F-R48-7: no product change (F-R48-7 is a lead records fix).
- F-R46-8: the harness dev-dependency, which belongs to the GUI lane.
- harness-view: only the job correlation, deadline and allow edits of items 7 and 9; no other harness-view change and no new egui code (D1).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "central_broker_client", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "business_event_ids", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required", "jobs": "bounded_admission_via_job_port", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r47-followups-1`, base set at dispatch)

- reel: `src/audio/mod.rs`, `src/audio/engine.rs` (the sample budget only) and a new or existing reel audio test file.
- harness: `src/cli.rs` (export-sequence audio path) and `tests/cli_export_sequence.rs`.
- harness-core: `src/manual.rs` (export-sequence entry only), plus the manual test if it lives there.
- render-gpu: `tests/gpu_plan.rs` (threshold rows only).
- interop-fig-base: `src/nodes.rs` (`resolve_variables` and the variable value decode) and its tests.
- accord: `src/business_event.rs` (the event id only, item 5).
- harness-core: `src/harness.rs` (business-event entry path, items 5-6) and `tests/business_events_run.rs`, `tests/run_attribution.rs`.
- session: `src/session.rs` (additive correlation-carrying submit path, item 8) and `src/jobs.rs` (additive deadline outcome, item 7).
- harness-view: `src/view/job_admission.rs`, `src/view/audio_batch.rs` (items 7 and 9) and `tests/viewer_job_admission.rs`.
- No crate `Cargo.toml` change (feature-set changes rebuild dependents under new identities, F-R46-8); root inputs and the Master Spec stay untouched.

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Red first per finding; pushed red commits must compile.
- One bundled run on the lane disk the lead names: the whole suites of reel, harness (under the round's units), harness-core, render-gpu (round units plus the test-faults units; DX12 rows explicit), interop-fig-base, accord, session and harness-view (the round's viewer units), plus clippy over the changed crates' direct dependents (rules 4a, 4b for the session API and 4e). Name the log per RUN LOGS; lane-check prints the `HEAD=` header.
- Vendor behaviour stays UNVERIFIED (D5).

</topic>
