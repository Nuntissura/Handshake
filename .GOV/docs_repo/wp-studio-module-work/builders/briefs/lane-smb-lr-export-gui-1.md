---
file_id: LANE-SMB-LR-EXPORT-GUI-1
file_kind: lane_brief
updated_at: 2026-10-30T14:53:22Z
wp: WP-KERNEL-STUDIO
lane: SMB-LR-EXPORT-GUI-1
lane_file: builders/lanes/hsk-studio-lr-export-gui-1.json
crates: [hsk-studio-harness-view (develop export files), hsk-studio-harness (CLI flags + viewer test)]
model: opus
effort: medium
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
queued: until SMB-PS-GRADIENT-BUCKET-1 has pushed (GUI slot); the lead dispatches
---

<topic id="target" status="active" summary="Lightroom export dialog in the window: sizing, output sharpening, watermark, TIFF/DNG buttons">

## Target

Gap source: `research/parity-20261023-adobe-print.json`, key `apps.lightroom_classic.top_gaps_remaining` rank 5, row `LR-33` ("Export lacks resize, output sharpening, watermark, presets and a window button for TIFF/DNG").

The engine half is SMB-LR-EXPORT-SIZING-1, READY at 61bb4d58. Its lane file `builders/lanes/hsk-studio-lr-export-sizing-1.json`, key `notes.gui_calls_for_a_later_lane`, lists the calls this lane makes:
- `develop_to_output_staged(&d.prepared.plane, &recipe, revision, depth, SRGB_V4_ICC, &stage, d.threads, self.cancel)`, which returns `(OutputImage, ExportStageReceipt, DevelopReceipt)` (`develop src/output.rs:181`);
- `develop_to_tiff_staged(&plane, &recipe, revision, &settings /* dpi None */, &stage, SRGB_V4_ICC, threads, cancel)` (`develop src/export.rs:372`);
- `validate_export_stage(&stage)` for live form checks (`export_stage.rs:1011`);
- `fit_output_size(&stage.sizing, w, h)` for the size preview (`export_stage.rs:645`);
- `EXPORT_STAGE_KEYS` for the panel rows (`export_stage.rs:222`);
- the DNG preview (`reduce_preview`) is not staged.

Still open at 5328a6a2 (checked by planner, read-only):
- The window's develop export is two buttons, PNG 16-bit and JPEG, in `harness-view src/view/develop.rs` near line 1333.
- `DevelopExport` (`develop.rs:662`) is `Copy` and carries no stage.
- TIFF and DNG exist only through the CLI (`harness src/cli.rs:482-517`).

Slices, with a local commit each:

1. **Headless first.**
   - The viewer holds an export stage: `Viewer::set_develop_export_stage(ExportStage)` and a getter, plus a `state_json` projection.
   - `export_develop` applies it through the staged entries for PNG, JPEG and TIFF. DNG ignores it with a note, because its preview is not staged.
   - Keep `DevelopExport` `Copy` and unchanged. The stage is viewer state beside it, never a field of it.
   - With the default (empty) stage, every existing export stays byte-identical. Pin the digests in the existing `harness tests/viewer_develop*.rs`.
   - The receipt fields (source and output size, fit rule, filter, sharpening row, watermark rect, clip counts) go into the export result JSON and the operation log.
2. **Export dialog** (new `src/view/develop/export_dialog.rs`, declared from `develop.rs` like `develop/spot.rs`, with its own `pub mod widget` ids).
   - Groups ordered as STU-IO-135: Image Sizing, Output Sharpening, Watermarking. Rows come from `EXPORT_STAGE_KEYS`, with their `studio_declared` labels shown.
   - Controls:
     - the resize type and its value fields;
     - constrain size and don't enlarge;
     - resolution and its unit;
     - sharpening on/off, media and amount;
     - watermark on/off; a graphic chosen through the existing `file_dialog`; opacity, size mode and value, anchor, insets, rotation.
   - Show a live "W x H px" preview from `fit_output_size`. Invalid values show `validate_export_stage`'s typed error inline and disable Export. Nothing is clamped in the view.
   - Add Export TIFF and Export DNG buttons next to PNG and JPEG, which closes LR-33's "window button" part. TIFF uses the existing `DevelopExport::Tiff` defaults.
   - All controls are AccessKit-reachable, with stable ids `develop.export.*`.
3. **CLI parity, optional if cheap.** `harness src/cli.rs` export flags for the stage: `--long-edge`, `--sharpen screen|glossy|matte:low|standard|high`, `--watermark FILE` and similar, mapped one-to-one to the same `ExportStage`. Skip this slice if it takes more than an hour, and list it as remaining.

Not in scope:
- text watermarks (an engine refusal; show it typed);
- export presets: STU-IO-134 "stored export setting" needs a Folio record; list it as remaining;
- the file-size limit.

Lightroom's dialog layout is UNVERIFIED (D5).

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (grepped in `master-spec-v02.225/spec-modules/14-studio-creative-suite.md`)

- `[STU-IO-134]` (:21057): show the stored versus surface-only class per key; never promote a surface field.
- `[STU-IO-135]` (:21059): the group set and order the dialog follows.
- `[STU-IO-136]` (:21061): resize type, resolution unit, sharpening media and amount domains.
- `[STU-IO-137]` (:21082): numeric bounds and defaults.
- `[STU-IO-138]` (:21094): `constrain_size`, `do_not_enlarge`, `use_watermark`, and the `studio_declared` label.
- Readings and defaults are those of SMB-LR-EXPORT-SIZING-1 (`notes.readings_declared`, `notes.studio_declared_defaults`). Do not restate or change them in the view.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-lr-export-gui-1`, base = the origin tip the lead names after GRADIENT-BUCKET-1 has pushed)

- `hsk-studio-harness-view`:
  - `src/view/develop.rs` (stage state, staged calls in `export_develop_inner`, the two new buttons, the `mod export_dialog;` line);
  - new `src/view/develop/export_dialog.rs`.
- `hsk-studio-harness`:
  - `src/cli.rs` (slice 3 only);
  - new `tests/viewer_develop_export_stage.rs` (`#![cfg(feature = "viewer")]`);
  - digest pins only in `tests/viewer_develop*.rs`, if the slice 1 regression needs them.
- Do not touch:
  - `src/view/mod.rs` (its `widget` module included; use the dialog's own `widget` module), `src/view/edit.rs`, `src/view/fill_tools.rs`, `src/view/brush_mix.rs` (GRADIENT-BUCKET-1);
  - `src/view/timeline*`, `src/view/timeline/**`, `src/view/monitor.rs`, `src/view/mixer.rs`, `src/view/audio_session.rs`, `src/view/shell.rs` (SMB-PR-NEST-1);
  - `hsk-studio-develop` (read only; if an engine change is needed, report the signature to the lead);
  - `hsk-studio-harness-core`, `hsk-studio-folio`, `hsk-studio-nib`, `hsk-studio-composite`;
  - `RUN-UI.md` (list its lines in the notes), the Master Spec, and root inputs.
- This is a GUI lane. It takes GRADIENT-BUCKET-1's slot once that lane has pushed.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Headless.**
  - Long edge 64 on a raw fixture gives the hand-computed output size in the exported PNG's header and in the result JSON.
  - Sharpen Screen/Standard changes the pixels against the unstaged export, and the receipt row matches the engine table.
  - A 2x2 watermark lands at the anchor rect the receipt reports.
  - An empty stage is byte-identical to the pinned digests.
- **Dialog**, driven through the real viewer like `tests/viewer_develop_2.rs`; aim after the first egui frame.
  - Setting the fields through their ids updates the "W x H" preview.
  - An invalid percentage shows the typed error and Export is disabled.
  - Export TIFF and Export DNG write files whose headers parse.
  - A text watermark choice is refused typed.
- **CLI** (slice 3): one argv maps to the same stage as the dialog. Compare the receipts.

## Closure and run timing

- `harness-view` is upstream of `harness` only, and `harness` is upstream of nothing.
- The lane's own crates depend on develop (already pushed) and on everything harness-view depends on.
- NEST-1 (pulse, harness-core, harness-view) and ARROWHEADS-1 (folio, nib, composite) are both upstream of this lane's crates. Whichever pushes first forces a rebase.
- The bundled run covers (rules 4a and 4e):
  - clippy over harness-view and harness;
  - the WHOLE suites of harness-view (round units `["viewer"]` and `["desktop","audio-batch"]`) and of harness (its viewer units).
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals: use relative paths and tempdirs (rule 4c).
- Do not start until the lead dispatches this lane after GRADIENT-BUCKET-1 pushes. Do not start the bundled run until the lead names the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
