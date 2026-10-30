---
file_id: LANE-SMB-AI-ARROWHEADS-2
file_kind: lane_brief
updated_at: 2026-10-30T15:45:34Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-ARROWHEADS-2
lane_file: builders/lanes/hsk-studio-ai-arrowheads-2.json
crates: [hsk-studio-interop-fig-native (import + export), hsk-studio-interop-vector-svg (import + export), hsk-studio-harness-view (vector style + Stroke panel), hsk-studio-harness (viewer test)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Arrowheads usable in the window: Figma and SVG import/export map stroke_markers; Stroke panel pickers">

## Target

Gap source:
- `builders/lanes/hsk-studio-ai-arrowheads-1.json` (READY at 2a60acf3): its records render, but nothing authors or imports them.
- `research/parity-20261023-adobe-print.json` `apps.illustrator.top_gaps_remaining` rank 5 (`AI-31 / AI-13`, arrowheads).

At 2a60acf3 only folio, nib, composite and their tests mention `stroke_markers` (git grep).

Importers checked by planner (read-only):

| Importer | Exists | Today | This lane |
|---|---|---|---|
| Figma `.fig`: `hsk-studio-interop-fig-native` `src/native.rs:938-949` | yes | Figma marker caps other than NONE, ROUND and SQUARE become a butt cap with the typed loss `stroke_cap_marker_no_folio_home` | map |
| SVG: `hsk-studio-interop-vector-svg` `src/svg_adapter.rs:1274` | yes | `marker-start` / `marker-mid` / `marker-end` other than `none` become the loss `unsupported_marker` | map |
| AI: `hsk-studio-interop-vector-core` `src/ai_import.rs` | yes | reads only the PDF-compatible stream; Illustrator private data, where live arrowheads live, is a typed loss (`ai_private_data_not_imported`); in the PDF stream an arrowhead is already plain geometry | out of scope: nothing to map |
| PDF | no PDF vector importer | | out of scope |
| IDML: `hsk-studio-interop-layout` | yes | refuses graphic page items (`idml_decode.rs:342` `graphic_page_item_not_interpreted`); its `LeftLineEnd`/`RightLineEnd` values match the 12 `arrow.*` tokens | follow-up |

Slices, with a local commit each:

1. **Figma import and export** (`interop-fig-native`).
   - Map the Figma `stroke_cap` values `ARROW_LINES`, `ARROW_EQUILATERAL`, `TRIANGLE_FILLED`, `DIAMOND_FILLED` and `CIRCLE_FILLED`, plus the six `ERD_*` values, to a `StudioStrokeMarkers` entry on that stroke row. The tokens are verbatim; this is the STU-VEC-146 collaborative and connector captures.
     - One node-level cap applies to both ends of an open path (Studio reading, UNVERIFIED).
     - Scale 100 (`studio_declared`).
     - The cap itself becomes butt.
   - Network strokes keep a typed loss: composite refuses `vector_marker_network`.
   - Unknown marker caps, such as washi tape, keep the existing loss.
   - The Figma exporter (`src/export.rs`) writes the cap back from the markers when both ends carry the same collaborative token. Anything else is a typed loss; never drop it silently.
2. **SVG import and export** (`interop-vector-svg`).
   - On import, map `marker-start` / `marker-end` to a token ONLY when the referenced `<marker>` has `orient="auto"` or `auto-start-reverse` and exactly one shape child that the lane recognises:
     - a closed 3-vertex polygon/path maps to `arrow.triangle`;
     - an open 3-point chevron maps to `arrow.simple`;
     - a circle maps to `arrow.circle_solid` or `arrow.circle` by its fill;
     - a rect maps to `arrow.square_solid` or `arrow.square`.
     - Scale comes from `markerWidth` x `markerUnits` (`strokeWidth`) relative to the token's nib size. The recognition table and the scale mapping are Studio readings, UNVERIFIED.
   - Everything else keeps `unsupported_marker`. `marker-mid` has no Studio home and keeps its loss.
   - On export (`src/svg_export.rs`, `svg_export/records.rs`), write each stroke marker as a `<marker>` def holding nib's `marker_outlines` shape, so SVG export and import round-trip for the recognised tokens. ERD tokens are a typed loss.
3. **Viewer commands** (harness-view, new `src/view/vector/markers.rs`).
   - `Viewer::set_stroke_markers(layer_id, row, Option<MarkerSpec>)` and `Viewer::swap_stroke_markers(layer_id, row)`, using folio's `swap_markers`.
     - Each is ONE undo step, the same entry-only history step as the style edits (`style.rs` module note).
     - Validated by folio `validate_vector_records` before the commit.
     - Projected in `state_json`.
   - Markers follow their stroke row through every restyle (`set_stroke`, graphic style apply/merge/break, `clear_appearance`) by row kind and per-kind ordinal, exactly as swatch and brush bindings do (`style.rs` `Bind`).
   - **Struct-literal impact.** `StrokePanel` has 14 struct literals, 10 of them in `harness tests/viewer_vector_styles.rs`. Do NOT add a field to `StrokePanel`; markers are a separate `MarkerSpec` and separate methods.
4. **Stroke panel** (`src/view/hosted.rs`, Stroke panel).
   - Start and End pickers: none plus the 18 drawn tokens, grouped by suite. ERD tokens imported from a file show as read-only "stored, not drawn".
   - Start scale and end scale (1..1000 %), alignment (inside or outside the path), and a Swap button.
   - New `PanelAction` variants call the step 3 methods. Widget ids are `panel.stroke.marker.*`.
   - Vector PDF export (`src/view/vector/pdf_export.rs`) reports markers as a typed loss (`markers_not_exported`), unless emitting nib's outlines as filled paths is a few lines. Then do it and test it.

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (module 14, master-spec-v02.225)

- `[STU-VEC-146]` (:3986): the 26-token catalogue, start and end scales (1..1000 %), `arrow_alignment`, independent start and end fields, and a swap command.
- `[STU-VEC-020]` (:3936): no captured attribute is dropped. That is why every unmapped marker stays a typed loss.
- `[STU-VEC-062]`: network strokes. Their markers stay refused (ARROWHEADS-1 accepted refusal).
- Readings and the shape table are ARROWHEADS-1's (`notes.declared_readings`, nib `markers` table). Do not change them. Illustrator, Figma and SVG-viewer parity is UNVERIFIED (D5).

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-arrowheads-2`, base 2a60acf3, rebased onto the base the lead names)

- `hsk-studio-interop-fig-native`: `src/native.rs` (the cap mapping and the vector-records push) and `src/export.rs`, plus new `tests/fig_markers.rs`.
- `hsk-studio-interop-vector-svg`: `src/svg_adapter.rs`, `src/svg_adapter/defs.rs`, `src/svg_export.rs`, `src/svg_export/records.rs`, plus new `tests/svg_markers.rs`.
- `hsk-studio-harness-view`:
  - new `src/view/vector/markers.rs`;
  - `src/view/vector/style.rs` (row-following hunks);
  - `src/view/vector.rs` (the `mod markers;` / `pub use` lines and `state_json` projection only);
  - `src/view/hosted.rs` (Stroke panel rows, `PanelAction` variants);
  - `src/view/vector/pdf_export.rs`.
- `hsk-studio-harness`: new `tests/viewer_vector_markers.rs` (`#![cfg(feature = "viewer")]`).
- Do not touch:
  - the timeline files, `timeline_export*`, `monitor.rs`, `mixer.rs`, `audio_session.rs`, `mod.rs`, `hsk-studio-pulse`, `hsk-studio-harness-core` (SMB-PR-NEST-2);
  - `hsk-studio-folio`, `hsk-studio-components` (SMB-FIG-GRID-1). FIG-GRID-1 adds one `grid: None` line at the `StudioAutoLayout` literal in `interop-fig-native src/native.rs`; this lane pushes first and that lane rebases;
  - `hsk-studio-nib` and `hsk-studio-composite` (read only; if you need a nib API, report the signature to the lead);
  - `hsk-studio-interop-layout` (the IDML follow-up);
  - `RUN-UI.md` (list its lines in the notes), the Master Spec, and root inputs.
- This is a GUI lane (2 of 2, with NEST-2). The two lanes' harness-view files are disjoint.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Figma.**
  - A synthetic `.fig` node with `stroke_cap TRIANGLE_FILLED` imports with a marker entry on both ends and no `stroke_cap_marker_no_folio_home` loss.
  - An ERD cap is stored and its render refusal is typed.
  - A network stroke keeps its loss.
  - Export writes the cap back.
  - An unknown cap keeps the old loss.
- **SVG.**
  - A triangle marker on `marker-end` maps to `arrow.triangle` with the hand-computed scale.
  - An unrecognised marker (a two-shape group) keeps `unsupported_marker`.
  - `marker-mid` keeps its loss.
  - Export, then import, gives the same `stroke_markers` for the recognised tokens.
  - Hostile input (V4-01/02): a self-referencing marker, a huge `markerWidth`, a NaN `viewBox`.
- **Viewer.**
  - Setting start `arrow.triangle` at 200 %, end `arrow.circle_solid`, alignment inside the path, is one undo step each.
  - The canvas render shows the marker (the coverage differs from the unmarked render near the end only).
  - Swap.
  - A restyle keeps the markers on their row.
  - The Stroke panel ids drive the same edits headlessly.
  - Vector PDF export reports `markers_not_exported`, or the outlines.

## Closure and run timing

- `interop-fig-native` is upstream of `interop-fig`, which is upstream of `harness-view` and `harness`.
- `interop-vector-svg` is upstream of `interop-vector`, which is upstream of `harness-view` and `harness`.
- `harness-view` is upstream of `harness`.
- NEST-2 overlaps only at the crate level (harness-view, harness). Whichever pushes second rebases and re-runs those suites.
- This lane pushes BEFORE SMB-FIG-GRID-1, which shares `native.rs` (one line) and is upstream of everything through folio.
- The bundled run covers (rules 4a and 4e):
  - clippy over the four crates and their direct dependents;
  - the WHOLE suites of `interop-fig-native`, `interop-fig`, `interop-vector-svg`, `interop-vector`, `harness-view` (round units `["viewer"]` and `["desktop","audio-batch"]`) and `harness`;
  - doctests of any crate whose doc examples change (rule 4d).
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals (rule 4c).
- Do NOT start the bundled run until the lead names the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
