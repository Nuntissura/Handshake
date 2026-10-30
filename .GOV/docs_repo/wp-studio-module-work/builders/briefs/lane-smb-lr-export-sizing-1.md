---
file_id: LANE-SMB-LR-EXPORT-SIZING-1
file_kind: lane_brief
updated_at: 2026-10-30T13:47:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-LR-EXPORT-SIZING-1
lane_file: builders/lanes/hsk-studio-lr-export-sizing-1.json
crates: [hsk-studio-develop]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Lightroom export: image sizing, output sharpening, graphic watermark (engine)">

## Target

Gap source: `research/parity-20261023-adobe-print.json`.
- Key `apps.lightroom_classic.top_gaps_remaining` rank 5, row `LR-33`: "Export lacks resize, output sharpening, watermark, presets and a window button for TIFF/DNG".
- The matching `apps.lightroom_classic.changed_rows` entry for `LR-33`: "resize, output sharpening, watermark still absent; PARTIAL".

Still open at d208bc0e (checked by planner, read-only):
- `crates/hsk-studio-develop/src/export.rs` module note: "Not here (recorded gaps, not silently dropped): output resize and sharpen-for-output (develop has no resample-for-output or output-sharpening stage...)".
- `TiffExportSettings` (`export.rs:78`) has no sizing, sharpening or watermark field.
- The only downscale is the DNG preview's area average (`dng_write.rs:630`, `reduce_preview`).
- No lane in `builders/lanes/` covers LR-33.

This is the export stage every Lightroom export passes through: "Resize to fit: long edge 2048 px, don't enlarge, sharpen for Screen, Standard, add watermark". Build it as ONE develop output stage. It runs on the developed linear-light image (`pipeline.rs:1172`, `DevelopedImage`) before `render_output` applies the OETF, so it serves the PNG, JPEG, TIFF and DNG-preview paths alike. Push after each green slice:

1. **Image sizing** (new `src/export_stage.rs`, or a folder).
   - `OutputSizing` carries:
     - `resize_type`: all six members of STU-IO-136, `WidthHeight | Dimensions | LongEdge | ShortEdge | Megapixels | Percentage`;
     - `max_width`, `max_height`, `percentage` and `resolution`, with the STU-IO-137 bounds and defaults;
     - the resolution unit (`pixels | in | cm`, plus `ppinch | ppcm` for resolution);
     - `constrain_size` and `do_not_enlarge` (STU-IO-138). Where the preset spread splits, the default is labelled `studio_declared`.
   - Fit rules for each type, with rounding stated:
     - `Dimensions` fits both orientations;
     - `Megapixels` keeps the aspect ratio;
     - `do_not_enlarge` never upsamples.
   - Resample in linear light with a stated, anti-aliasing reduction filter (a Studio reading, UNVERIFIED against Lightroom, D5).
     - Reuse `hsk-studio-pigment` `transform_math` for the kernel where it fits; develop already depends on pigment.
     - A filter whose support does not widen with the reduction factor is not acceptable for downscaling: it aliases.
   - The output resolution is recorded, and the TIFF path writes it as dpi.
2. **Output sharpening.**
   - Media `Screen | Glossy | Matte` and amount `Low | Standard | High` (STU-IO-136); level 1..3, default 2 (STU-IO-137).
   - Applied after sizing, at the output size. Its kernel scales with media and output resolution.
   - Lightroom's algorithm is not public. Implement a stated unsharp-mask reading with a table of radius and amount per (media, level), marked UNVERIFIED (D5).
   - Clamping is counted in the existing `ClipReport` (never silent).
3. **Graphic watermark.**
   - `use_watermark` default false (STU-IO-138). The source is a caller-supplied straight-RGBA image.
   - Fields: opacity percent; size as proportional percent of the output width, `fit` or `fill`; anchor in 9 positions; inset x and y in percent; rotation `0 | 90 | 180 | 270`.
   - Composited in linear light after sharpening.
   - The 12 `watermarking` keys of STU-IO-135 are not enumerated in the spec. Every field is a Studio reading, UNVERIFIED. Add a spec-delta request in the lane notes for the field list.
   - Text watermarks need `hsk-studio-type`, a new module edge, and a crate SMB-AI-AREA-TYPE-4 is changing. They are a declared gap with a typed refusal. Do not add the dependency.
   - Do not confuse this with STU-FX-022's retired "Watermark" filter (a different surface).
4. **Entry points, additive only.**
   - `develop_to_output_staged(...)` returns the staged `OutputImage` and an `ExportStageReceipt`: source size, output size, the fit rule used, filter, sharpening table row, watermark rect, and clip counts.
   - A TIFF entry that takes the stage.
   - Existing signatures (`develop_to_output`, `develop_to_tiff`, `TiffExportSettings::new`) stay unchanged, so harness-view, harness, harness-core and interop-lrcat compile untouched.
   - In the notes, list the exact calls a later GUI lane needs in harness-view `src/view/develop.rs` (`DevelopExport`, `export_develop`).

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (grepped in `master-spec-v02.225/spec-modules/14-studio-creative-suite.md`)

- `[STU-IO-134]` (:21057): a stored export setting versus a surface-only field. Carry that flag per key; never promote a surface field silently.
- `[STU-IO-135]` (:21059): group set, including `image_sizing` (13), `watermarking` (12) and `output_sharpening` (4).
- `[STU-IO-136]` (:21061): resize types, resolution units, output sharpening media and amount domains.
- `[STU-IO-137]` (:21082): max width and height (default 1000), resize percentage (default 100), resolution (default 240), output sharpening level 1..3 (default 2).
- `[STU-IO-138]` (:21094): `constrain_size`, `do_not_enlarge`, `use_watermark`, and the `studio_declared` default rule.
- No Folio record is needed: the stage is develop-local settings, like `TiffExportSettings`. If you conclude that a stored preset record is required (STU-IO-134 "stored export setting"), stop and report the exact signature to the lead. Do not add a Folio field in this lane.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-lr-export-sizing-1`, base d208bc0e)

- `hsk-studio-develop`:
  - new `src/export_stage.rs` (or a folder);
  - additive `pub mod` and `pub use` lines in `src/lib.rs`;
  - additive hunks in `src/export.rs` and `src/output.rs` for the staged entries;
  - new `tests/develop_export_stage.rs`;
  - the crate `DESCRIPTOR` string.
- `hsk-studio-pigment`: read only. If a `transform_math` item you need is private, report it to the lead. SMB-PS-GRADIENT-BUCKET-1 owns pigment in this wave.
- Do not touch:
  - any `hsk-studio-harness-view` file (this is NOT a GUI lane);
  - `hsk-studio-harness-core` (SMB-R42-FOLLOWUPS-1);
  - `hsk-studio-type` (SMB-AI-AREA-TYPE-4);
  - `hsk-studio-pulse` (SMB-AU-MULTITRACK-3);
  - `hsk-studio-folio`, `hsk-studio-interop-tiff`, the Master Spec, and root inputs.

</topic>

<topic id="proof" status="active" summary="Proof and run timing">

## Proof and run timing

Red first: write each slice's tests and commit them red before the code. Record each red run per RUN LOGS.

Focused tests, 5 or fewer per slice:

- **Sizing.**
  - Every resize type on a portrait and a landscape source, against hand-computed output sizes.
  - `do_not_enlarge` on a small source.
  - `Megapixels` rounding.
  - A downscale of a 1-pixel checkerboard: the output mean equals the input mean within a stated tolerance, and there is no aliasing beat. Compute the expectation, do not snapshot it.
- **Sharpening.**
  - A step edge gains the stated overshoot per (media, level).
  - A flat field is unchanged.
  - Clip counts are reported.
- **Watermark.**
  - Anchor and inset placement of a 2x2 mark on a 10x6 output, by hand.
  - Opacity 50 blends in linear light.
  - Rotation 90.
  - A text watermark returns its typed refusal.
- **Receipt and regression.**
  - The staged TIFF carries the output size and dpi.
  - An unstaged `develop_to_tiff` is byte-identical to before. Pin the existing `tests/develop_export_tiff.rs` digests unchanged.
- **Hostile input (V4-01/02).** Each case gets a typed refusal, never a panic and never a silent green (V4-03):
  - zero or negative size;
  - percentage 0;
  - resolution NaN;
  - megapixels so large the output overflows;
  - a watermark 0x0 or larger than the cap;
  - opacity out of range.

Bundled run (rules 4a-4e):
- Code and test now, with focused check, clippy and nextest of develop.
- The bundled run covers:
  - clippy over develop and its direct dependents (`harness-core`, `harness-view`, `harness`, `interop-lrcat`);
  - the WHOLE develop suite, including its `#[ignore]` corpus units only where `.config/round-features.toml` names them;
  - develop doctests if you add doc examples (rule 4d);
  - the dependents' suites only if a public API they pin changed (rule 4b). The API must stay additive, so they should not need to run.
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals (rule 4c).
- Develop depends on pigment, and SMB-PS-GRADIENT-BUCKET-1 changes pigment in this wave.
  - Do NOT start the bundled run until the lead messages you naming the disk (C: `--disk ssd` or D: `--disk d`) and the base.
  - Rebase onto that base and run once.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit in the lane file (workflow-v3 keys). Never claim PASS.
- Lightroom equivalence stays UNVERIFIED (no oracle, D5).

</topic>
