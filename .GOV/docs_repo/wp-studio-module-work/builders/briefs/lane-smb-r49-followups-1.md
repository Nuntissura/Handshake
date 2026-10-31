---
file_id: LANE-SMB-R49-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-31T16:35:22Z
wp: WP-KERNEL-STUDIO
lane: SMB-R49-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r49-followups-1.json
crates: [hsk-studio-harness-view (viewer audio import sample budget), hsk-studio-accord (DESCRIPTOR text only), hsk-studio-session (DESCRIPTOR text only), hsk-studio-observe (ring.rs doctest codes only)]
model: sonnet
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS incl. lane-check --per-sha once the steward lands it, loop cap, lane worktree, HOST INTEGRATION)
---

<topic id="target" status="active" summary="Close R49 F-R49-2 and F-R49-3, and R46 F-R46-12">

## Target

Read each entry's `finding`, `repair_direction` and `file_line` in `validation/round-R49.json` → `findings_not_failures` (and `round-R46.json` for F-R46-12) first.

1. **F-R49-2 (LOW), viewer audio import bounds frames only.** harness-view `src/view/audio_import.rs` `import_audio_external` (:288, decode call :297) calls `reel::decode_audio_file_with(src, &Limits::default(), limits.max_frames, ..)`; `ImportLimits::max_frames` is 518 400 000 (:58-72), and reel admits up to `MAX_AUDIO_CHANNELS = 8`, so one 8-channel file can hold about 16.6 GB before any refusal.
   - Decode through reel's sample-budget entry point added by SMB-R47-FOLLOWUPS-1 (`reel::audio::decode_audio_file_within` or whatever name it landed under; read reel `src/audio/mod.rs` at your base). Scope change (lead, 2026-10-31T17:25Z, after the builder reported reel's budgeted entry point takes no external decoder): add the budgeted external-decoder variant to reel additively (same per-block budget and typed refusal; existing callers unchanged) and call it for the ADTS / media-native case; no budget wrapper in harness-view.
   - Budget: `max_frames x 2` samples, the stereo bound the export path uses (1 036 800 000 samples = the 4 147 200 000-byte decoded budget). Put it in `ImportLimits` as a named field with that default.
   - Refuse with the existing typed code `audio_too_large` (the export-sequence path's code). If the viewer's refusal mapping has no such code, add it additively.
   - Red test in harness-view: a constant-subframe 8-channel FLAC of a few KiB whose decode would exceed the budget is refused before the budget is exceeded. Assert the refusal, not timing. Reuse the FLAC builder the R47 lane added for reel's or harness's test if it is reachable; otherwise write a minimal one in the test.
   - The import function is not UI code: no egui line is added or changed (D1). The test runs in the round's viewer units (4e).
2. **F-R49-3 (LOW), DESCRIPTOR text.** Add the APIs to the machine-readable manuals.
   - accord `src/lib.rs` DESCRIPTOR: `entry_event_sequence`, `ENTRY_EVENT_SEQUENCE_BIT`, and the entry-only id rule.
   - session `src/lib.rs` DESCRIPTOR: `submit_as`, `submit_timeline_as`, `submit_multicam_as`, `JobLifecycle::admit_by`, `AdmittedJob::deadline` / `deadline_passed`, the `DeadlineExceeded` refusal and outcome, the `refused_deadline` / `deadline_exceeded` counters, and the command-correlation variant of the report frame.
   - Read the code at your base; describe what is there, nothing more. If a test pins DESCRIPTOR text or a manual check reads it, run it.
3. **F-R46-12 (INFO), ring.rs doctests.** Pin the error code on the two no-text `compile_fail` doctests in observe `src/ring.rs` (module docs), e.g. `compile_fail,E0277`, using the code rustc reports for each. Prove it with `cargo test --doc -p hsk-studio-observe` (4d).

Out of scope:
- F-R49-1 (per-sha check) belongs to the steward. F-R49-4, -5 and -8 are spec-owner entries (SD-19..21). F-R49-6 goes to SMB-PDF-STOCK-PRESETS-1. F-R49-7 and -9 are lead records. F-R49-10 needs no asset change.
- F-R46-8 (the harness dev-dependency) belongs to the GUI lane.
- No other harness-view change, no reel change beyond the additive budgeted external-decoder entry point, no new egui code (D1). harness-view `src/view/audio_batch.rs:642` (read-back of the batch's own export, frames-only) stays as is: lead decision, an observation in the lane notes.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: no mutating command added", "jobs": "unchanged: the import stays a bounded call; HBR-HOST-004 overload class gains the sample budget", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r49-followups-1`, branch `lane/smb-r49-followups-1`, base f8a1c5f3)

- harness-view: `src/view/audio_import.rs` (`ImportLimits` and `import_audio_external` only) and a new or existing viewer audio-import test file.
- accord: `src/lib.rs` (DESCRIPTOR const only).
- session: `src/lib.rs` (DESCRIPTOR const only).
- observe: `src/ring.rs` (module doc examples only).
- reel: `src/audio/mod.rs`, `src/audio/engine.rs` (the additive budgeted external-decoder entry point only) and a reel audio test file (scope change 2026-10-31T17:25Z).
- No crate `Cargo.toml` change (F-R46-8), no root input, no Master Spec edit.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first for item 1; pushed red commits must compile, and every pushed commit must compile the round units of its changed crates and their direct dependents (F-R49-1).
- One bundled run on the disk the lead names: step 0; clippy over the changed crates and their direct dependents (lane-check does this); the WHOLE suites of harness-view (the round's viewer units), accord, session, observe and reel (4a); `cargo test --doc` for observe, accord and session (4d). Name the log per RUN LOGS.
- Push order: accord and observe sit in every lane's closure, so this lane pushes LAST of the wave, after the lead says so.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
