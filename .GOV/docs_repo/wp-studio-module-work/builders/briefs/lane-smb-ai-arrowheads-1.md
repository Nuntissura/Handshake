---
file_id: LANE-SMB-AI-ARROWHEADS-1
file_kind: lane_brief
updated_at: 2026-10-30T14:53:22Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-ARROWHEADS-1
lane_file: builders/lanes/hsk-studio-ai-arrowheads-1.json
crates: [hsk-studio-folio (additive vector record member), hsk-studio-nib (marker geometry), hsk-studio-composite (stroke rendering)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Illustrator / InDesign / Figma stroke arrowheads, engine only">

## Target

Gap source: `research/parity-20261023-adobe-print.json`, key `apps.illustrator.top_gaps_remaining` rank 5, row `AI-31 / AI-13` ("No per-object overprint; no arrowheads"; spec STU-VEC-146; "arrowheads: none").

Still open at 5328a6a2 (checked by planner, read-only):
- The `folio src/paint.rs` module note lists arrowheads (STU-VEC-143..146) as "not stored".
- `StudioStrokeStyle` (`paint.rs:184`) has no marker field.
- Nib's `StrokeStyle` (`nib src/stroke.rs:138`) and `outline_stroke` draw no end markers.
- Composite strokes are nib's stroke outline (`composite src/vector.rs:1304` `stroke_style`, the `StrokePlane` path).
- No crate source mentions an arrowhead.

Slices, with a local commit each:

1. **Folio record**, additive. Follow the `pattern_paints` precedent: a row of a `PAINT` style bound in `StudioVectorRecords` (`vector.rs:256` `StudioPatternPaint`, `vector.rs:450`).
   - Add the member `stroke_markers: Vec<StudioStrokeMarkers>` to `StudioVectorRecords`, `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
   - The struct is `#[non_exhaustive]`, so no literal elsewhere breaks; do NOT add a field to `StudioStrokeStyle`, which has struct literals in five crates.
   - Each entry: `style_id`, `row` (must be a `Stroke` row), `start` and `end` (the marker token), `start_scale` and `end_scale` (percent), `alignment` (`arrow_align.inside_path | arrow_align.outside_path`). At most one entry per `(style_id, row)`.
   - Validate in `validate_vector_records`: dangling style or row, a non-stroke row, a duplicate, a scale outside the hard bounds, an unknown token. Each gets a typed `Code`.
   - Update the `DESCRIPTOR` in `src/lib.rs` (the vector records sentence).
   - A swap command (STU-VEC-146 "a swap command") is a pure function over the entry (`swap_markers`), tested.
2. **Nib marker geometry** (new `src/markers.rs`). Do not add a field to `StrokeStyle`; its literals are spread across crates.
   - `marker_outlines(path, &MarkerSpec, stroke_width, options)` returns filled `OwnedPath` outlines at each open subpath's start and end.
     - Each outline is oriented by the end tangent (a degenerate end tangent uses the first non-zero derivative, else a typed refusal).
     - The size is `stroke_width * scale / 100`.
     - `inside_path` puts the tip at the path end and returns the trimmed path the stroke must use.
     - `outside_path` starts the marker at the end and extends past it.
   - Closed subpaths have no ends and get no marker. That is reported, not silent.
3. **Composite rendering.**
   - A stroke row with a marker entry unions the marker outlines into that row's stroke coverage plane, painted with the row's paint and opacity.
   - Under `inside_path`, the stroke is drawn on the trimmed path.
   - Dashes do not dash the marker.
   - The supersampled coverage mode keeps its typed refusal for the new case. Do not render markers there.
   - Refusals stay typed per layer (`PreparedVectors::refusal`).

## Marker set

STU-VEC-146 names the token set as the union of three captures, 26 tokens:
- the layout suite: `arrow.none`, `arrow.simple`, `arrow.simple_wide`, `arrow.triangle`, `arrow.triangle_wide`, `arrow.barbed`, `arrow.curved`, `arrow.circle`, `arrow.circle_solid`, `arrow.square`, `arrow.square_solid`, `arrow.bar`;
- the collaborative suite: `NONE`, `ROUND`, `SQUARE`, `ARROW_LINES`, `ARROW_EQUILATERAL`, `TRIANGLE_FILLED`, `DIAMOND_FILLED`, `CIRCLE_FILLED`;
- the connector extension: `ERD_ONE`, `ERD_MANY`, `ERD_ONE_OR_MORE`, `ERD_ZERO_OR_MORE`, `ERD_ZERO_OR_ONE`, `ERD_EXACTLY_ONE`.

The spec gives NO shapes, proportions or default scale.
- Folio stores all 26 tokens verbatim. The two `none` tokens mean no marker.
- Nib draws the 11 layout-suite shapes and the 7 collaborative-suite shapes, which are filled or outlined triangles, bars, circles, squares and diamonds.
- Every proportion is a Studio reading, UNVERIFIED (D5), in a table in the module docs and the `DESCRIPTOR`.
- The 6 ERD terminators are stored, and composite refuses them typed (`marker_shape_unbuilt`). Never draw a substitute.
- The default scale is 100, labelled `studio_declared` (the spec says `unknown`).
- Add a spec-delta request in the lane notes for the shape geometry, the default scale, and Illustrator's "align" semantics.

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (grepped in `master-spec-v02.225/spec-modules/14-studio-creative-suite.md`)

- `[STU-VEC-146]` (:3986): the arrowhead catalogue, which SUPERSEDES the informal marker names of `[STU-VEC-020]`.
  - Tokens as above.
  - `arrow_start_scale` and `arrow_end_scale` in percent: hard 1..1000, soft, default and precision unknown.
  - `arrow_alignment {arrow_align.inside_path, arrow_align.outside_path}`.
  - Independent start and end fields with independent scales, and a swap command.
- `[STU-VEC-020]` (:3936): a stroke entry carries the STU-VEC-143..148 attributes, and no captured attribute is dropped.
- `[STU-VEC-106]`: observed values are not bounds. This applies to the live-effect `head_arrow`/`tail_arrow` indices (0, 15, 16). Do not map those indices in this lane.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-arrowheads-1`, base 5328a6a2, rebased onto the base the lead names)

- `hsk-studio-folio`:
  - `src/vector.rs` (the new member and its validation call);
  - new `src/vector_markers.rs`;
  - additive `mod` / `pub use` lines and the `DESCRIPTOR` sentence in `src/lib.rs`;
  - new `tests/vector_markers.rs`.
- `hsk-studio-nib`: new `src/markers.rs`, additive lines in `src/lib.rs`, new `tests/markers.rs`.
- `hsk-studio-composite`: `src/vector.rs` (the stroke row path) and/or `src/vector_nib.rs`, plus new `tests/vector_markers.rs`.
- `hsk-studio-package`: only `tests/externals.rs`, and only if its classifier needs the new field names (rule 4b).
- Do not touch:
  - every `hsk-studio-harness-view` file and every importer (`interop-fig*`, `interop-vector*`, `interop-layout`). Importer mapping is a follow-up lane;
  - `hsk-studio-pigment` (SMB-PS-GRADIENT-BUCKET-1);
  - `hsk-studio-pulse`, `hsk-studio-harness-core` and `hsk-studio-session` (SMB-PR-NEST-1);
  - `hsk-studio-develop` (SMB-LR-EXPORT-GUI-1 reads it);
  - `StudioStrokeStyle`, nib `StrokeStyle` and `outline_stroke` signatures; the Master Spec; root inputs.
- This is not a GUI lane.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Folio.**
  - A record set round-trips through wire JSON.
  - A record set without markers is byte-identical to before.
  - Each refusal (dangling style, fill row, duplicate, scale 0 or 1001, unknown token) has its typed code.
  - Swap.
- **Nib.**
  - A triangle marker on a horizontal and on a 30-degree line, with the tip and base vertices computed by hand, for `inside_path` (the trimmed path end is computed by hand) and `outside_path`.
  - Scale 50 and 300.
  - A closed path reports no ends.
  - Hostile input (V4-01/02): NaN points, a zero-length path, a scale at the hard bounds.
- **Composite.**
  - A 2 px stroke line with `arrow.triangle` at the end. The coverage plane has the marker area within a stated tolerance of the analytic area.
  - Pixels away from the marker are identical to the unmarked render.
  - ERD tokens are refused typed.
  - A dashed stroke leaves the marker solid.

## Closure and run timing

- Folio is upstream of every crate in the workspace (pigment, pulse, session, develop, the harness crates included).
- Nib is upstream of `composite`, `harness-view`, `harness`, `interop-fig`, `interop-fig-native`, `interop-layout`, `interop-vector`, `interop-vector-core`, `interop-vector-svg` and `render-cpu`.
- Composite is upstream of `harness-view`, `harness`, `interop-fig`, `pulse`, `render-cpu` and `render-gpu`.
- So this lane's push invalidates the closure of every other lane in the wave. It should push LAST, after GRADIENT-BUCKET-1, NEST-1 and LR-EXPORT-GUI-1, unless the lead orders otherwise.
- The bundled run covers (rules 4a and 4b):
  - clippy over folio, nib, composite and their direct dependents;
  - the WHOLE suites of folio, nib, composite (with the round unit `["motion"]`) and `hsk-studio-package` (rule 4b: a folio record field was added);
  - doctests of any crate whose doc examples you change (rule 4d).
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals (rule 4c).
- Code and test now with focused runs on D:. Do NOT start the bundled run until the lead messages you naming the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.
- Illustrator, InDesign and Figma marker shapes stay UNVERIFIED (D5).

</topic>
