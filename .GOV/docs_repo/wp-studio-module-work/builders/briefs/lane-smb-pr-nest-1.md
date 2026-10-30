---
file_id: LANE-SMB-PR-NEST-1
file_kind: lane_brief
updated_at: 2026-10-30T14:53:22Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-NEST-1
lane_file: builders/lanes/hsk-studio-pr-nest-1.json
crates: [hsk-studio-pulse, hsk-studio-harness-core (one additive Run step), hsk-studio-harness-view (timeline + monitor files)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Premiere Nest / Make Subsequence / open nested, with nested playback">

## Target

Gap source: `research/parity-20261023-video-audio.json`, key `per_app[app=premiere_pro].top_gaps` rank 3, row `PR-08 Nested sequences` ("PARTIAL; no nest command").

Still open at 5328a6a2 (checked by planner, read-only):
- Folio already stores a nested clip: `ClipSource::Sequence { sequence_id }` (`folio src/timeline.rs:293`), with the acyclic check in document validation.
- Pulse has no nest planner. Its only `ClipSource::Sequence` readers are the multicam planners.
- `pulse src/multicam.rs:904` says "pulse playback does not resolve nested sequences":
  - `frame_layers` (`src/playback.rs`) passes a Sequence source to the host as if it were media;
  - the audio mix asks `AudioSources::audio(&clip.source_ref)` and fails `SourceExtentUnknown` when that is `None` (`src/audio.rs:310`, `src/bus_mix.rs:133`).
  - So a nested clip today is black in the program monitor and can fail the whole mix.
- `Run::submit_sequence` (`harness-core src/harness.rs:1296`) only replaces an existing sequence.
  - The multi-sequence step exists one layer down: `hsk_studio_session::MulticamStep` with `SequenceEdit::{Replace, Insert, Remove}` and `Session::submit_multicam` (`session src/multicam_step.rs`, `src/session.rs:811`). It applies as one revision with one inverse.
  - No `Run` method exposes it.

Slices, in this order, with a local commit each:

1. **Pulse planners** (new `src/nest.rs`).
   - `plan_nest` takes the selected clips (one or more tracks, a link group taken whole, as `timeline_edit` does). It returns:
     - one `SequenceEdit::Insert` of a NEW sequence that holds those clips, re-based to start at 0, on mirrored tracks (same kinds, same per-kind order, names kept);
     - one `SequenceEdit::Replace` of the parent, where the selection becomes ONE nested clip per kind present. The video one goes on the lowest selected video track and the audio one on the lowest selected audio track, both linked by a new `link_group`, spanning the selection's extent.
   - `plan_make_subsequence` builds the same new sequence from the selection, or from the in-to-out range (clips cut at the range edges), and leaves the parent unchanged: Insert only.
   - Pulse mints no ids. The caller passes the new sequence id, the clip ids and the link group, following the `LinkedRazor` precedent.
   - The nested sequence takes the parent's `frame_rate`.
   - A selection that would create a cycle, an empty selection, or a locked track is a typed refusal. The prospective document is validated by Folio, as `timeline_edit` does; never re-implement validation.
2. **Pulse nested resolution**, in the same module, additive. Existing signatures stay unchanged.
   - `frame_layers_nested(document, sequence, tick, ...)` expands a Sequence-source layer into the nested sequence's layers at the mapped source tick (speed and `source_in` applied), recursively.
   - A `SourceExtents` wrapper answers a Sequence source with the nested sequence's duration, so trimming the nested clip trims the window onto it (STU-VID-030 "opaque to timeline operations").
   - An `AudioSources` wrapper renders a nested sequence's mix through the existing timeline mix path as that clip's source, bounded by the stream limits and the cancel token.
   - STU-VID-030 says "no fixed depth limit": bound the recursion by the caller budget and a cycle guard, not by a depth constant.
   - A nested source that cannot be resolved gets a typed per-clip report. It never fails the parent mix silently and never drops audio silently (V4-03).
3. **Run step and window** (harness-core, harness-view).
   - Add one additive `Run` method that submits a `Vec<SequenceEdit>` as ONE history step through `Session::submit_multicam`. The multicam records are passed unchanged (`records_expect == records_write == live`).
   - Add the matching `HistoryStep` field and the undo and redo arms.
   - Add a `RUN_COMMANDS` row in `harness-core src/action.rs`. The corpus test `corpus_lists_every_run_operation_with_recordability` fails without it (F-R42-3).
   - The step's session provider name is `multicam.step`. That is a Studio reading; note it, and do not rename it in session.
   - Timeline toolbar buttons through `TimelineCommand` and `button(...)` in `timeline_ui.rs`, with widget ids in `timeline.rs` `widget`:
     - `Nest` and `Make Subsequence`;
     - `Open Nested` (also on a double-click of a nested clip), the STU-VID-030 reveal command, which switches the timeline's `sequence_id` and keeps a back stack;
     - `Back to Parent`.
   - The program monitor (`monitor.rs`) and the mixer/audio path use the step 2 wrappers, so nested clips show and play.

Premiere behaviour is UNVERIFIED (D5) for which track receives the nested clip, track mirroring, and the subsequence range cut. Declare each as a Studio reading in the module docs and the `DESCRIPTOR`.

Deferred, not in this lane: the STU-VID-030a booleans (preserve the nested frame rate, preserve the nested resolution).
- Folio has no field for them, and `StudioSequence` has no frame-size field at all.
- They are a "MAY" clause, and they would put folio, and with it the whole workspace, in this lane's closure while SMB-AI-ARROWHEADS-1 holds folio.
- Write the needed record shape as a spec-delta / follow-up note: additive folio side record, package suite (rule 4b).

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (grepped in `master-spec-v02.225/spec-modules/14-studio-creative-suite.md`)

- `[STU-VID-030]` (:23543):
  - `Nest` replaces a selection with one clip whose source is a newly created sequence;
  - `Make subsequence` creates a sequence from a selection or the in-to-out range without replacing;
  - a nested clip is opaque to timeline operations;
  - an explicit reveal command opens it;
  - nesting is recursive with no fixed depth limit;
  - a cycle is a validation error.
- `[STU-VID-030a]` (:23550): the preserve frame rate / resolution booleans. Deferred (see target); multicam's follow-nest boolean already exists (`folio src/multicam.rs:91`).
- `[STU-VID-040]` (:23671): every operation is one reversible `StudioHistoryEntry`.
- `[STU-VID-020]` (:23405) and `[STU-VID-021]` (:23397): clip and track records.
- `[STU-VID-026]` (:23476): the trim vocabulary that a nested clip obeys as an ordinary clip.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-nest-1`, base 5328a6a2, rebased onto the base the lead names)

- `hsk-studio-pulse`:
  - new `src/nest.rs`;
  - additive `pub mod` and `pub use` lines and the `DESCRIPTOR` in `src/lib.rs`;
  - new `tests/nest.rs`.
- `hsk-studio-harness-core`:
  - additive hunks in `src/harness.rs` (the new `Run` method, the `HistoryStep` field, the undo and redo arms);
  - one `RUN_COMMANDS` row in `src/action.rs`;
  - a new test or test rows in the existing corpus test.
- `hsk-studio-harness-view`:
  - `src/view/timeline.rs`, `src/view/timeline_ui.rs`, new `src/view/timeline/nest.rs`;
  - `src/view/monitor.rs`;
  - `src/view/mixer.rs` and `src/view/audio_session.rs`, only at the sites that build `AudioSources` / `SourceExtents`;
  - `src/view/shell.rs`, only the extents construction site near line 1355, if the mixer's extents are built there;
  - new `tests/viewer_timeline_nest.rs` (`#![cfg(feature = "viewer")]`, like `tests/viewer_timeline_trim.rs`).
- Do not touch:
  - `src/view/mod.rs`, `src/view/edit.rs`, `src/view/fill_tools.rs`, `src/view/brush_mix.rs`, and `hsk-studio-pigment` (SMB-PS-GRADIENT-BUCKET-1 is running). If you need a `mod.rs` hunk, report it to the lead;
  - `src/view/develop.rs` and `src/view/develop/**` (SMB-LR-EXPORT-GUI-1);
  - `hsk-studio-folio`, `hsk-studio-nib`, `hsk-studio-composite` (SMB-AI-ARROWHEADS-1);
  - `hsk-studio-session`: read only, use `submit_multicam` as it is. If a generic multi-sequence step is required, report the exact signature to the lead;
  - `RUN-UI.md` (list its lines in the notes), the Master Spec, and root inputs.
- This is GUI lane 2 of 2 (GRADIENT-BUCKET-1 holds the other).

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Nest.**
  - Two video clips plus their linked audio are nested into one video and one audio nested clip, linked, with the right extent. The new sequence holds the re-based clips.
  - The document validates.
  - Undo restores the document byte-identical; redo re-applies it.
- **Make subsequence from an in-to-out range.**
  - Clips are cut at the edges.
  - The parent is unchanged.
- **Refusals.**
  - Nesting a selection that contains a clip nesting the parent (cycle).
  - An empty selection.
  - A locked track.
  - Hostile ids: duplicates, an existing sequence id, path-like ids.
  - Every refusal leaves the document unchanged.
- **Resolution.**
  - The program frame at tick t of a nested clip at speed 2 equals the inner sequence's frame at the hand-computed source tick.
  - A nested clip's audio mix equals the inner mix sample for sample.
  - A 3-level nest resolves.
  - An unresolvable inner media source gives a typed per-clip report while the parent mix still renders.
- **Window.**
  - Nest from the toolbar is one undo step.
  - Open Nested, then Back to Parent, switches the timeline sequence.
  - Trimming the nested clip changes only the window onto the nested sequence.

## Closure and run timing

- Pulse is upstream of `harness-view`, `harness` and `interop-media`.
- `harness-core` is upstream of `harness-view` and `harness`.
- Pulse's dev-dependencies (`composite`, `render-cpu`, `pigment`, `prism`) put GRADIENT-BUCKET-1's pigment push inside this lane's test closure. Rebase after it lands.
- The bundled run covers (rules 4a and 4e):
  - clippy over pulse, harness-core and their direct dependents;
  - the WHOLE suites of pulse, harness-core, harness-view (the round units `["viewer"]` and `["desktop","audio-batch"]`) and harness.
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals (rule 4c).
- Do NOT start the bundled run until the lead messages you naming the disk (`--disk ssd` or `--disk d`) and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
