---
file_id: LANE-SMB-PR-SEQ-EXPORT-UI-1
file_kind: lane_brief
updated_at: 2026-10-30T16:43:48Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-SEQ-EXPORT-UI-1
lane_file: builders/lanes/hsk-studio-pr-seq-export-ui-1.json
crates: [hsk-studio-harness-view (timeline files + new export form), hsk-studio-harness (cli.rs + tests)]
model: opus
effort: medium
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Export a sequence (nested clips included) from the window and the CLI">

## Target

Gap source: `builders/lanes/hsk-studio-pr-nest-2.json` `notes.gaps[0]`: "window/CLI sequence export: no window or CLI code builds a timeline export job (render_queue.rs handles compositions only); `timeline_export_job_with` is test-driven only".

Correction to the dispatch premise, checked by planner at 91cb8889:
- There is NO render-queue surface in the window for compositions either.
- `timeline_export::render_queue` (`session_render_queue_jobs`, `render_queue_build`) and `timeline_export_job_with` are called only from tests (`git grep` over `crates/*/src`: no caller outside `src/view/timeline_export*`).
- The CLI has `export` for still images (`cli.rs:669` `execute_export`) and no sequence or composition export.
- The parity rollup lists "timeline/composition/render-queue export functions" and the CLI as unreachable.

So this lane builds the FIRST window export surface, for sequences. Compositions stay API-only (listed as remaining).

Slices, with a local commit each:

1. **Headless first.** `Viewer::export_sequence(sequence_id, ExportSequenceRequest) -> Result<Value, MonitorFail>` in new `src/view/timeline/export_media.rs`.
   - The request holds: in/out (default the work area, else the whole sequence), `preset_id` (through the existing `preset_recipe`, `timeline_export.rs:443`), width and height, bit depth, chroma, the output path (relative to the save root, never a drive-letter literal), overwrite, and the artifact name.
   - It builds `TimelineExport` from the viewer's monitor `FrameSource` / `SourceExtents` and the mixer's `AudioSources`.
   - It calls `timeline_export_job_with(t, NestedDocument::new(run sequences, routing set), cancel)`, so nested clips resolve.
   - It submits the job to `export_queue(worker)` (`:424`), the existing reel queue, and returns the receipt JSON. The receipt and every refusal (`nest_unresolved`, `export_range`, `frame_unavailable`, …) go into the operation log.
   - A viewer without a frame decoder refuses typed. It never writes black frames (V4-03).
   - Never write a new `TimelineExport` field. It has 5 test literals, which NEST-2 kept intact.
2. **Window.**
   - The timeline toolbar gets an `Export Media…` button (`TimelineCommand::ExportMediaOpen`, widget `timeline.export_media`). Premiere's command id is UNVERIFIED; name the Studio id in the notes.
   - It opens an inline form row, like the Speed/Duration form (`timeline_ui.rs` `speed_form_row`):
     - a preset picker over the shipped presets `preset_recipe` accepts;
     - in/out from the monitor's in/out or the work area;
     - the output path;
     - overwrite;
     - Export / Cancel.
   - Progress and the result appear in the status line and in `state_json()["timeline"]["export"]`.
   - Export runs on the queue's worker thread. It never blocks the UI thread or takes focus (STU-QUIET-005).
3. **CLI.**
   - `harness export-sequence <package> --sequence <id> --preset <id> [--in T --out T] --output <path> [--overwrite]`.
   - It opens the package headlessly and drives the same `Viewer::export_sequence`, with the same refusals and the same receipt JSON with `--json`.
   - The parse goes beside the existing `export` arm (`cli.rs` around lines 236-300).

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (module 14, master-spec-v02.225)

- `[STU-VID-030]` (:23543): nested clips are clips. The export resolves them, which is the NEST-2 path.
- Export recipe and preset clauses: cite those in SMB-REEL-ENCODE and SMB-D24A-CODEC-DEFAULTS lane files (STU-VID-060..064, STU-IO-125, STU-IO-140); add no new codec behaviour.
- `[STU-CMP-110]` / `[STU-QUIET-005]`: the export runs headless and in the background. The window form only calls the Viewer method.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-seq-export-ui-1`, base 91cb8889)

- `hsk-studio-harness-view`:
  - new `src/view/timeline/export_media.rs`;
  - `src/view/timeline.rs` (`TimelineCommand` variants, widget ids, the module line);
  - `src/view/timeline_ui.rs` (button and form row);
  - new `tests/viewer_timeline_export_media.rs` (`#![cfg(feature = "viewer")]`, plus `media` if the round unit enables it: rule 4e).
- `hsk-studio-harness`:
  - `src/cli.rs` (the `export-sequence` arm and its parse);
  - new `tests/cli_export_sequence.rs`.
- Read only: `src/view/timeline_export.rs` and `timeline_export/**`. Use the NEST-2 API unchanged. If a change is needed there, report the signature to the lead.
- Do not touch:
  - `src/view/mod.rs` and `src/view/design/**` (SMB-FIG-GRID-2);
  - `hsk-studio-layout` and `hsk-studio-interop-layout` (SMB-ID-FRAME-AUTOSIZE-1);
  - `shell.rs` and `hosted.rs` (no new panel: the form lives in the timeline);
  - `hsk-studio-reel` and `hsk-studio-pulse` (read only);
  - `RUN-UI.md` (list its lines in the notes), the Master Spec, and root inputs.
- This is GUI lane 1 of 2 (with FIG-GRID-2).

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Headless.**
  - A sequence with a nested video+audio clip exports through `Viewer::export_sequence`.
  - The receipt names the frame count and the audio sample count, computed by hand from in/out.
  - The decoded frame at one tick equals the program monitor's frame.
  - A missing frame decoder refuses typed.
  - A nested sequence the document lacks refuses `nest_unresolved` naming the clip.
- **Window.**
  - `timeline.export_media`, then setting the form fields by id, then Export, writes the file. `state_json` shows the result.
  - Cancel writes nothing.
- **CLI.**
  - `export-sequence` on a saved package gives the same receipt JSON as the headless call.
  - An unknown `--preset`, a bad `--in`/`--out`, and a missing `--sequence` each give a typed error, exit non-zero.

## Closure and run timing

- `harness-view` is upstream of `harness` only.
- SMB-ID-FRAME-AUTOSIZE-1 changes `layout`, which is upstream of harness-view. If it pushes first, rebase and re-run.
- SMB-FIG-GRID-2 shares the crates harness-view and harness with disjoint files. Whichever pushes second rebases and re-runs.
- The bundled run covers (rules 4a and 4e):
  - clippy over harness-view and harness;
  - the WHOLE suites of harness-view (round units `["viewer"]` and `["desktop","audio-batch"]`) and harness (its `media`, `viewer` and `os-encode` units, where the new tests are gated).
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals (rule 4c).
- Do NOT start the bundled run until the lead names the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
