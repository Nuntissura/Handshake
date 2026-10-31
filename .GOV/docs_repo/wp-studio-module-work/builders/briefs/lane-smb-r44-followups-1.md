---
file_id: LANE-SMB-R44-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-31T00:07:10Z
wp: WP-KERNEL-STUDIO
lane: SMB-R44-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r44-followups-1.json
crates: [hsk-studio-pigment, hsk-studio-prism (additive decode), hsk-studio-harness-view (egui-free call sites and tests only), hsk-studio-harness (CLI), hsk-studio-harness-core (manual entry), hsk-studio-pulse, hsk-studio-interop-vector-svg, hsk-studio-composite, hsk-studio-interop-fig-native, hsk-studio-layout (last, after SMB-ID-FRAME-AUTOSIZE-2)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close the six MEDIUM R44 findings and the cheap LOW ones; engine-only">

## Target

Close these findings of round STUDIO-R44. Each is in `validation/round-R44.json` → `findings_not_failures`. Read its `finding`, `repair_direction` and `file_line` before the slice. The line numbers were read at ff4ca080 and re-checked at 3cf0f149 (planner, read-only); they have not moved.

This is an engine lane (Operator decision D6). It writes no GUI surface code and no egui code. The harness-view edits below are call sites and tests in files that have no egui use, plus one function body in `view/fill_tools.rs`. The lead's D6 reading allows a call-site change in a view file that adds no toolkit code.

Slices, one local commit each, red tests first:

1. **F-R44-1: a gradient rewrites pixels it does not paint (MEDIUM). Also closes the decode copy of F-R44-8.** The fix is in three places. It is not pigment-only: the second sRGB curve is in harness-view.
   - **Pigment** (`src/fill_tools.rs`, `Region::new` :615-635 and `Region::paint` :640-677). Where paint alpha × fill opacity is 0, the coverage is 0, so `masked_replace` keeps the before bytes bit for bit. This covers the default foreground-to-transparent drag past its end. Keep the existing refusal for an all-zero coverage plane. Update the module doc: unpainted pixels inside the bounds are not rewritten.
   - **Prism** (`src/view_transform.rs`). Add `DisplayEncoding::decode`, the exact inverse of `encode`, for each arm Prism has a CTL source for: `moncurve_f` for `Srgb`, and the power inverse for `Bt1886` and `Gamma22`. Use PQ's inverse only if Prism already carries the ST 2084 inverse; otherwise refuse it typed. Never guess a curve. This is an additive public method.
   - **Harness-view** (`src/view/fill_tools.rs:176-178`). Replace the body of `fn decode` with Prism's `Srgb` decode and delete the IEC constants. That function is the only edit allowed in this file, which contains egui code (`fill_options_ui` :685 and the rest stay untouched).
2. **F-R44-2: the export-sequence CLI cannot export sound (MEDIUM), with F-R44-15 (LOW) in the same function.** Work in `hsk-studio-harness` `src/cli.rs:749-810` and the `export-sequence` entry in `hsk-studio-harness-core` `src/manual.rs`.
   - **Preferred:** a `--audio FILE` flag (repeatable). Decode each file with reel's audio decoder, the one `view/audio_import.rs` uses: `hsk_studio_reel::decode_audio_file`.
     - Key each file as the `sha256:<hex>` clip source, exactly as `bind_program_media` keys video.
     - Bind the files with the existing `Viewer::set_export_audio(sources, extents, rate)`.
     - All files must share one rate, or the export is refused typed. This path has no resampler.
     - The port adapter lives in harness (`cli.rs` or a new harness module), never in harness-view.
   - **Fallback, only if the preferred route needs a harness-view change:** check before any decode. A sequence with an enabled audio clip is refused `audio_sources_missing`, with a detail that names the unimplemented `--audio` flag. The manual then says the command exports video only.
   - The manual must match the code. Today it says the command "mixes the sound".
   - A CLI flag is not a new command, so rule (3) needs no Registry id here. The flag goes in the existing `manual.rs` usage and flag text.
   - F-R44-15, three fixes:
     - an `absolute()` error fails typed (stage `output`);
     - an `--out` past the sequence end is refused `export_range` before any media is read;
     - the total of `--media` and `--audio` bytes is capped by a named constant before reading, and over the cap it is refused `media_too_large`. The constant is a declared Studio reading, stated in the doc.
3. **F-R44-3: exported sound ignores the root mix (MEDIUM), with F-R44-12 and F-R44-13 (LOW).** All three are in harness-view files with no egui use.
   - **F-R44-3.** `src/view/timeline_export.rs:379-391` and `:417-418` build the root mix with the plain `TimelineMixStream::new`. Build it from the stream the mixer plays instead: `Viewer::bus_stream` (`src/view/audio_session.rs:52`, pulse `TimelineBusStream` with the stored routing, root clip envelopes, bus routes, sends, effects and automation; nested clips go through `NestedAudioSources::with_routing`). No pulse change is needed; the stream already exists.
     - If bus routing cannot run on the export path, refuse typed instead: an export whose root has envelopes, routing, sends or effects gets `export_root_mix_unsupported`. This fallback applies only if the `Clone + Send` bounds of the export sources cannot be met, and the reason goes in the notes.
     - Update the module docs at `src/view/timeline/export_media.rs:25-35` and the manual entry.
   - **F-R44-12.** In `src/view/timeline/audio.rs`, the ClipPan arm (:997-1012) and the clip-volume arm must check `samples_from_ticks_exact` on `clip.timeline_start + offset`, like the Track, Master and Bus arms. An off-grid point is refused `not_sample_aligned`. Add a hostile `AddPoint` test: refused, with no history step.
   - **F-R44-13.** In `tests/viewer_timeline_nest_2.rs:399-444`:
     - undo once before the refused write, then assert the redo history is unchanged;
     - add an export case with `enveloped_routing()` compared against the pre-nest mix. It proves F-R44-3 end to end.
     - This is SMB-PR-NEST-2's test, so list it in the notes as another lane's test made stricter.
4. **F-R44-7: nested frame expansion is not bounded by work (LOW).** In pulse `src/nest.rs:712-750` `expand`, charge every Sequence-layer expansion and every `unresolved` push against `max_layers`, or against a separate visit budget named in the module docs. Over the budget is the typed `limit_exceeded`. Add a fan-out test: 4 tracks nested 12 deep, refused `limit_exceeded`. Assert the refusal, not a time. Existing nest tests stay green unchanged.
5. **F-R44-4: SVG marker import drops the marker's own paint (MEDIUM), with F-R44-10 (LOW).**
   - **F-R44-4.** In `hsk-studio-interop-vector-svg` `src/svg_adapter.rs:2687` `recognise_marker` and `:2756-2767`:
     - map a marker only when the child's paint is absent, `context-stroke`, or equal in colour and opacity to the line's stroke, and the filled child has no stroke of its own;
     - everything else returns `None`, which keeps the typed `unsupported_marker` loss.
     - Tests: a red arrowhead on a black line is a loss; a black filled one on a black line maps; `context-stroke` maps; a filled child with a stroke is a loss.
   - **F-R44-10, SVG side.** In `svg_adapter.rs:1365`, `polygon` and closed `path` elements that carry markers get `unsupported_marker` at import. SVG viewers draw markers there; Studio does not, so the import must not drop them silently.
   - **F-R44-10, composite side.** In `src/vector.rs:1798-1812`, keep Nib's `closed_subpaths` and `consumed_subpaths` and surface them as a typed per-layer note on `PreparedVectors` (:353).
     - If only `refusal` (:484) exists, add the note additively. Check every struct literal of the type across the workspace first.
     - A marker entry on a closed path is no longer a silent plain stroke.
6. **F-R44-5: the .fig frame export paints marker overlaps twice (MEDIUM).** In `hsk-studio-interop-fig-native` `src/export.rs:609-635`, paint the stroke and its markers with one alpha application. Either:
   - fill the union of the Nib stroke outline of the trimmed path and the marker outlines as one path; or
   - use a transparency group with the alpha applied once.

   Add a 50 % opacity test that proves the overlap is painted once, from the PDF content operators. The reviewer reports the same pattern in harness-view `src/view/pdf_export.rs:160` (no egui use; NOT_INSPECTED by the validator). Read it. If it is confirmed, apply the same repair there with a test; if not, say so in the notes.
7. **F-R44-6: plain reflow lays out auto-sized frames without reporting the new size (MEDIUM), with F-R44-9 (LOW). This is the LAST slice. Rebase onto the origin that carries SMB-ID-FRAME-AUTOSIZE-2 before writing it.**
   - **F-R44-6.** In layout `src/threaded_reflow.rs:392-472`, the plain entry points (`reflow`, `reflow_typeset`, `reflow_flow`, `reflow_grid`, `reflow_cjk`) compose an auto-sized frame at its authored size. Only `reflow_autosize` (:428) resizes and returns the `FrameResize` list.
     - **Lane decision:** keep the authored size; do not refuse. The proposal then matches the document. Refusing would undo the IDML imports that compose today.
     - Say so in the module docs and the `DESCRIPTOR`.
   - **The pin at `tests/frame_autosize.rs:243` changes.** Today it asserts plain reflow equals the auto-sized layout. Make it assert plain reflow equals the authored-size composition and differs from `reflow_autosize` where the sizes differ.
     - This test belongs to SMB-ID-FRAME-AUTOSIZE-1/2; list it in the notes.
     - If AUTOSIZE-2's pushed tests pin plain-entry auto-size behaviour, list each one the same way.
   - **Callers:**
     - interop-layout `src/reflow.rs:203` (IDML) now gets the authored size. Do not edit interop-layout; run its suite.
     - harness-view `src/layout_ui.rs:454` (layout window, GUI) shows the authored size until the GUI follow-up commits `reflow_autosize` bounds, which AUTOSIZE-2 already declared. Do not edit it.
   - **F-R44-9 (2).** In `tests/threaded_reflow.rs:699-712`, give the Ascent case a minimum below its ascent, so the pin fails if the fixed-minimum path is taken.
   - **F-R44-9 (1).** Confirm after the rebase that AUTOSIZE-2's `FirstBaselineMetricUnavailable(CapHeight)` pin is in. If it is not, note that; do not change interop-layout.

### Not in this lane

- F-R44-8 beyond the decode: the block-core dedupe is a refactor of an egui file, so it waits for the GUI lanes.
- F-R44-16: FIG-GRID-2 (READY, R45) already maps GRID frames in fig-native at 3340855b. The per-solve cell budget in components goes to a lane after R45 judges FIG-GRID-2.
- F-R44-11, -14 and -17..24: INFO, no repair.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

## host_integration

```json
{
  "schema_id": "hsk.studio_mt_host_integration@1",
  "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030",
  "spec_version": "v02.225",
  "gui_host": "none",
  "gpu": "via_render_provider_port",
  "diagnostics": {
    "internal_diagnostics": "producer_through_observe",
    "flight_recorder": "none",
    "palmistry": "none"
  },
  "control_descriptors": "none",
  "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing",
  "jobs": "bounded_admission_via_job_port",
  "settings": "none",
  "spec_refs": ["STU-ARC-002", "STU-ARC-008", "STU-ARC-012", "STU-ARC-013", "STU-VAL-002"],
  "refactor_exposure": "low",
  "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief",
  "notes": {
    "gui_host": "no surface code: harness-view edits are call sites and tests in egui-free files plus the decode body in view/fill_tools.rs (D6 lead reading)",
    "gpu": "pigment, prism and composite stay render providers behind the pigment RenderBackend trait; no device, queue or budget owner added",
    "jobs": "the export path keeps reel's existing bounded queue and limits; this lane adds no queue, worker or job; JobPort adoption is RF-08",
    "control_descriptors": "the CLI --audio flag extends an existing command's manual entry; no new command or control id",
    "attribution": "no new mutating op; the ClipPan and clip-volume refusals tighten an existing op"
  },
  "recorded_at": "2026-10-31T00:07:10Z"
}
```

</topic>

<topic id="paths" status="active" summary="Owned paths and do-not-touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-r44-followups-1`, branch `lane/smb-r44-followups-1`, base 3cf0f149)

- pigment: `src/fill_tools.rs` (the `Region` coverage path and module doc), its fill tests.
- prism: `src/view_transform.rs` (additive `decode`), a new decode test file.
- harness-view (no egui code, no `*_ui` function, no widget, panel or shell edit):
  - `src/view/fill_tools.rs`: the `decode` body only;
  - `src/view/timeline_export.rs`;
  - `src/view/timeline/export_media.rs` (module docs);
  - `src/view/timeline/audio.rs` (the ClipPan and clip-volume arms);
  - `src/view/pdf_export.rs`, only if F-R44-5's reviewer note is confirmed;
  - tests: `tests/viewer_timeline_nest_2.rs` (F-R44-13) and new test files.
  - The five source files above were grepped egui-free at 3cf0f149, except `fill_tools.rs`.
- harness: `src/cli.rs` (`execute_export_sequence` and its argument parsing), CLI tests. If `--audio` needs a manifest change, stop and ask the lead: SMB-GPU-CLIENT-PORT-1 owns a line in `hsk-studio-harness/Cargo.toml`.
- harness-core: `src/manual.rs`, the `export-sequence` entry only.
- pulse: `src/nest.rs` (`expand`), nest tests.
- interop-vector-svg: `src/svg_adapter.rs` (`recognise_marker` and the markable check), marker import tests.
- composite: `src/vector.rs` (the marker adapter and `PreparedVectors` note), new marker-note tests.
- interop-fig-native: `src/export.rs` (stroke and marker painting), export tests.
- layout (after AUTOSIZE-2 pushes): `src/threaded_reflow.rs` entry points, `src/lib.rs` DESCRIPTOR sentence, `tests/frame_autosize.rs` (the :243 pin), `tests/threaded_reflow.rs` (the Ascent pin).
- Do not touch:
  - `hsk-studio-render-gpu` (SMB-GPU-CLIENT-PORT-1);
  - `hsk-studio-observe` (SMB-OBSERVE-RING-PORT-1);
  - `hsk-studio-type`, `hsk-studio-interop-layout`, and layout before AUTOSIZE-2 has pushed (SMB-ID-FRAME-AUTOSIZE-2);
  - harness-view `view/display.rs`, `view/render.rs`, `view/mod.rs`, `view/shell.rs`, `view/hosted.rs`, `layout_ui.rs`, `design/auto_layout.rs` (FIG-GRID-3), and every egui item;
  - the harness-core `Run` (FIG-GRID-3);
  - `hsk-studio-components` (F-R44-16 deferred);
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run order">

## Proof

- Red first. Pushed red commits must compile (RED COMMITS). Run at most 5 focused tests per slice.
- **F-R44-1:**
  - pigment: a foreground-to-transparent gradient leaves coverage 0 past its end;
  - prism: `decode(encode(x)) == x` within f32 rounding over a grid that crosses the sRGB linear segment (below about 0.00304) and its breakpoint;
  - harness-view: a layer holding linear 1.5 and 0.002 outside the painted part keeps its exact tile bytes after the gradient commits; painted pixels match Prism's round trip; undo identity holds.
- **F-R44-3:** a sequence with a clip pan envelope and a master volume lane exports samples equal to `bus_stream`'s render within 1e-6. Include a negative control against the plain `TimelineMixStream`.
- **F-R44-2:** a CLI test with an audio clip: either the sound is exported, or the typed refusal names `--audio`.
- **Rule 4e:** check that every harness-view and harness test you add or change compiles under a round unit:
  - harness-view units: `["viewer"]`, `["desktop", "audio-batch"]`;
  - harness units: include `["media"]`.
  - If export-media, fill or timeline tests need a unit the round lacks, say so in the notes for the steward.

## Closure and run timing

- **Upstream reach:**
  - prism is upstream of composite, develop, harness, harness-core, harness-view, interop-layout, interop-psd, pigment, render-cpu and render-gpu (dev: interop-fig, pulse);
  - pigment is upstream of render-gpu, render-cpu, composite and develop;
  - composite and pulse are upstream of harness-view and harness.
  - So this lane touches the closure of SMB-GPU-CLIENT-PORT-1 (render-gpu depends on prism and pigment).
- **The bundled run (rules 4a and 4b):**
  - step 0 first: `lane-check.sh --step0-only`;
  - clippy over every changed crate and its direct dependents;
  - the WHOLE suites of prism, pigment, pulse, composite (round unit `["motion"]`), layout, interop-vector-svg, interop-fig-native, harness-core, harness-view (both round units) and harness (the units that build the CLI and viewer tests);
  - the suites of interop-layout, interop-vector and interop-fig, which pin the changed reflow, SVG import and .fig export behaviour (rule 4b);
  - doctests of any crate whose doc examples change (rule 4d).
- **Disk:** run on D: (`--disk d`) unless the lead names C:. R45 holds C:.
- **Wait for the lead:** code and test now with focused runs. Do NOT start the bundled run until the lead messages you with the disk and the base.
- **Logs:** name the log per RUN LOGS: lane id and sha in the path, sha on the first line.
- **Push order:**
  - SMB-ID-FRAME-AUTOSIZE-2 pushes first. Rebase onto it before slice 7, and push this lane after it.
  - Across the three new lanes the order is SMB-GPU-CLIENT-PORT-1, then this lane, then SMB-OBSERVE-RING-PORT-1, unless the lead orders otherwise.
  - Rebase first, and re-run if the rebase touched your crates or their dependencies.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS. Vendor behaviour (SVG viewers, Figma, InDesign) stays UNVERIFIED (D5).

</topic>
