---
file_id: LANE-SMB-PR-NEST-2
file_kind: lane_brief
updated_at: 2026-10-30T15:45:34Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-NEST-2
lane_file: builders/lanes/hsk-studio-pr-nest-2.json
crates: [hsk-studio-pulse (nest.rs), hsk-studio-harness-core (one additive Run step), hsk-studio-harness-view (timeline/nest, timeline_export, monitor, mixer/audio_session sites, mod.rs open_monitor/monitor_ui)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close SMB-PR-NEST-1's three gaps: nested export, envelopes that follow a nest, monitor without a timeline sync">

## Target

Gap source: `builders/lanes/hsk-studio-pr-nest-1.json`, keys `notes.gaps` and `notes.spec_deltas` (READY at bdc15f15). Each gap below was re-checked at origin 2a60acf3.

1. **Export of a sequence with a nested clip.**
   - Today the export is refused: `timeline_export_job` (`harness-view src/view/timeline_export.rs:281`) returns `nest_unresolved` because the job holds no document.
   - Make export resolve nested clips:
     - video through `ProgramMonitor::set_document`, the same path the program monitor uses (`frame_layers_nested`);
     - audio through `NestedAudioSources` (`pulse src/nest.rs:756`) wrapping the job's `AudioSources`, with the export audio rendered from the same nested mix as the mixer.
   - **Struct-literal impact.** `TimelineExport` (`timeline_export.rs:232`) has all-pub fields and 5 struct literals in tests:
     - `harness-view tests/viewer_timeline_nest.rs:412`;
     - `harness tests/timeline_captions.rs:117`;
     - `harness tests/timeline_export.rs:194`, `:280`, `:302`.
     - Do NOT add a field.
   - **Smallest non-breaking design.**
     - Add `timeline_export_job_with(t, nested: NestedDocument, cancel)`, where `NestedDocument` holds the document's sequences plus the audio routing set (step 2).
     - `timeline_export_job(t, cancel)` becomes `timeline_export_job_with(t, NestedDocument::none(), cancel)`. With no document it still refuses `nest_unresolved`, so the existing test at `viewer_timeline_nest.rs:432` stays valid unchanged.
     - A new test drives the `_with` form.
   - Today no window or CLI code calls `timeline_export_job`; only tests do (git grep at 2a60acf3). If the render-queue UI (`timeline_export/render_queue.rs`) can list sequences cheaply, wire it with the run's document. Otherwise list "window/CLI sequence export" as remaining. Do not invent a new export surface.
2. **Nesting clips that carry routing envelopes.**
   - Today the nest is refused: `nest_envelope_guard` (`harness-view src/view/timeline/nest.rs:187`) returns `nest_clip_envelopes_unsupported`.
   - Clip gain and pan points are offsets from the clip's own `timeline_start` (`folio src/audio_routing.rs` `StudioClipGainPoint` / `StudioClipPanPoint`), so a moved clip keeps its points unchanged.
   - Nest moves each moved clip's `clip_gains` / `clip_pans` entries from the parent sequence's `StudioAudioRouting` to a routing for the new sequence (created if absent; parent bus and track settings untouched).
   - The sequence edits and the routing entry commit as ONE history step:
     - add one additive `Run` method, `submit_sequence_edits_and_entry(op, edits, token, bytes)`, modelled on `submit_sequence_and_entry` (`harness-core src/harness.rs:1337`) and `submit_sequence_edits` (`:1353`);
     - a refused entry write undoes the sequence step (same rule as `sequence_and_entry_step`);
     - add a `RUN_COMMANDS` row (`action.rs`; F-R42-3 corpus test).
   - The nested mix must APPLY the moved envelopes. `NestedAudioSources` today renders the "plain timeline mix" (NEST-1 reading). Add a constructor `NestedAudioSources::with_routing(.., routing: &StudioAudioRoutingSet, ..)`:
     - it uses `TimelineMixStream::with_clip_envelopes` (`pulse src/audio.rs:489`) per nested sequence;
     - `new(..)` keeps its signature and delegates with an empty set, so its callers in `mixer.rs:189` and `audio_session.rs:72` compile unchanged until you switch them to `with_routing`.
   - Make Subsequence copies clips with fresh ids, so it copies their envelopes under the new ids into the new sequence's routing, in the same single step.
   - Undo restores the sequences and the routing entry byte-identical.
3. **Program monitor without a prior timeline sync.**
   - Today `open_monitor` (`harness-view src/view/mod.rs:387`) builds the monitor without `set_document`. Only the timeline panel calls `nest_sync_monitor` (`timeline.rs:1227`, `timeline/nest.rs:142`), so a monitor opened or refreshed without the timeline panel refuses nested clips with `nest_unresolved`.
   - Set the document in `open_monitor`.
   - Move the revision-checked sync into the monitor host path (`monitor_ui`, `mod.rs:419`, and the headless refresh it calls), so every refresh resyncs when the run revision changed, whether or not the timeline panel is open.
   - Keep `ProgramMonitor`'s pub API additive.

Studio readings stay as NEST-1 declared them, Premiere UNVERIFIED (D5). New reading: an envelope moves with its clip unchanged, in clip-relative ticks. Add a spec-delta request: no clause defines how clip envelopes follow a nest (NEST-1 `spec_deltas` item 3). STU-VID-030a stays deferred (it needs a folio record and would collide with SMB-FIG-GRID-1 on folio).

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (module 14, master-spec-v02.225)

- `[STU-VID-030]` (:23543): nest, subsequence, opaque nested clip, recursive.
- `[STU-VID-040]` (:23671): one reversible history entry per operation. The nest and the envelope move are one entry.
- `[STU-FX-137]` (:18454) and `[STU-FX-135]` (:18444): routing and audio stacks are part of the document.
- The export path's clauses are those of SMB-REEL-ENCODE and SMB-PR-NEST-1 (cite them from their lane files). Add no new export behaviour beyond resolving nested clips.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-nest-2`, base 2a60acf3, rebased onto the base the lead names)

- `hsk-studio-pulse`: `src/nest.rs` (additive `with_routing` constructor) and `tests/nest.rs`.
- `hsk-studio-harness-core`: additive hunks in `src/harness.rs`, plus one `RUN_COMMANDS` row in `src/action.rs`.
- `hsk-studio-harness-view`:
  - `src/view/timeline/nest.rs`, `src/view/timeline.rs` (sync call site only);
  - `src/view/timeline_export.rs`, `src/view/timeline_export/render_queue.rs` (only if step 1's optional wiring is cheap);
  - `src/view/monitor.rs`;
  - `src/view/mixer.rs` and `src/view/audio_session.rs` (the `NestedAudioSources` call sites only);
  - `src/view/mod.rs`: `open_monitor` and `monitor_ui` hunks only;
  - new `tests/viewer_timeline_nest_2.rs` (`#![cfg(feature = "viewer")]`).
- `hsk-studio-harness`: a new test only if the export test belongs with `tests/timeline_export.rs`. Never edit the 5 existing `TimelineExport` literals.
- Do not touch:
  - `src/view/vector.rs`, `src/view/vector/**`, `src/view/hosted.rs`, `hsk-studio-interop-fig-native`, `hsk-studio-interop-vector-svg` (SMB-AI-ARROWHEADS-2);
  - `hsk-studio-folio`, `hsk-studio-components` (SMB-FIG-GRID-1);
  - `hsk-studio-session` (read only);
  - `RUN-UI.md` (list its lines in the notes), the Master Spec, and root inputs.
- This is a GUI lane (1 of 2, with ARROWHEADS-2). The two lanes' harness-view files are disjoint.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Export.**
  - A sequence with a nested video+audio clip at speed 1 exports.
  - The decoded frame at a hand-picked tick equals the program monitor's frame for that tick.
  - The audio track equals the parent mix sample for sample within 1e-6.
  - The `timeline_export_job` form without a document still refuses `nest_unresolved` (unchanged test).
  - An inner media source that is missing fails the export typed (never a silent black frame or silence).
- **Envelopes.**
  - The parent mix before the nest equals the mix after it, sample for sample within 1e-6, for clips with a gain envelope and a pan envelope.
  - The routing entry moved the entries and kept their offsets.
  - Undo restores the sequences and the routing byte-identical; redo re-applies them.
  - A refused entry write leaves both unchanged.
  - Make Subsequence copies the envelopes under the new ids.
- **Monitor.** Open the monitor on a sequence with a nested clip with the timeline panel never opened, and the frame renders. After an edit that changes the nested sequence, the next refresh shows the change without the timeline panel.

## Closure and run timing

- Pulse is upstream of `harness-view`, `harness` and `interop-media`.
- `harness-core` is upstream of `harness-view` and `harness`.
- `harness-view` is upstream of `harness`.
- Overlap with SMB-AI-ARROWHEADS-2 is at the crate level only (harness-view, harness): disjoint files. Whichever pushes second rebases and re-runs the harness-view and harness suites.
- SMB-FIG-GRID-1 changes folio, which is upstream of this lane. It pushes after this lane.
- The bundled run covers (rules 4a and 4e):
  - clippy over pulse, harness-core and their direct dependents;
  - the WHOLE suites of pulse, harness-core, harness-view (round units `["viewer"]` and `["desktop","audio-batch"]`) and harness.
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals (rule 4c).
- Do NOT start the bundled run until the lead names the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
