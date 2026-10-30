---
file_id: LANE-SMB-ID-FRAME-AUTOSIZE-1
file_kind: lane_brief
updated_at: 2026-10-30T16:43:48Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-FRAME-AUTOSIZE-1
lane_file: builders/lanes/hsk-studio-id-frame-autosize-1.json
crates: [hsk-studio-layout]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="InDesign text-frame auto-size and first-baseline kinds in layout reflow (engine)">

## Target

Gap source: `research/parity-20261023-adobe-print.json`:
- key `apps.indesign.top_gaps_remaining` rank 5, row `ID-04`: "Frame auto-size and first-baseline kinds typed-refused", spec `STU-LAY-119 (:5775)`;
- the matching `apps.indesign.changed_rows` entry "ID-04 ... PARTIAL unchanged (auto-size/first-baseline refusals ...)".

Why this pick:
- The other InDesign gaps are done, oracle-bound or clause-less. ID-11 kinsoku and ID-23 liquid are done (SMB-ID-CJK-KINSOKU-1, SMB-ID-LIQUID-LAYOUT-1). ID-27 needs the Operator-attended PDF/X oracle. ID-31, open PDF, has no clause.
- Text frames are InDesign's base object, and both behaviours are core frame options.

Still open at 91cb8889 (checked by planner, read-only):
- `layout src/folio_input.rs:124-143` `supported()` refuses `LayoutError::UnsupportedComposition` whenever `auto_sizing_type != Off` or `first_baseline_offset != Fixed`.
- Folio already stores both enumerations (`folio src/layout.rs:101` `FirstBaseline`, `:117` `AutoSizing`, frame fields `:146-151`). No schema change is needed.
- Illustrator area text already implements first-baseline offsets from run metrics (`type src/area_text.rs:548` `first_offset`, `ComposeRun` with `cap_height_pt` / `x_height_pt`). It is the model to follow; do not change type.
- Ledger: SMB-ID-PREPRESS-2 (queued, no commits) lists "frame auto-size". This lane takes it; the lead removes it from PREPRESS-2.

Slices, with a local commit each:

1. **First baseline, all six STU-LAY-120 kinds.**
   - `ascent`, `cap_height`, `leading`, `em_box_height`, `x_height`, and `fixed`, which stays as today. The offset is `max(kind metric, minimum_first_baseline_offset)` (STU-LAY-118 `minimum_first_baseline_offset`).
   - `ShapedParagraph` (`layout src/paragraph.rs:40`) carries `ascent_pt` and `line_height_pt` only. Add `cap_height_pt: Option<f64>` and `x_height_pt: Option<f64>`.
     - Its 7 literals are all inside layout (2 in src, 5 in layout tests), so fix them here.
     - Fill them where layout shapes through type, from the face's OS/2 `capital_height()` / `x_height()` (`type src/provider/ttf/mod.rs:1682`, `:1695`, already public) scaled by size / units-per-em.
     - A face without the OS/2 value refuses that kind typed (`first_baseline_metric_unavailable:<kind>`). It never falls back silently.
   - `em_box_height` is the font size (Studio reading, UNVERIFIED).
2. **Auto-size, the five STU-LAY-119 kinds, on the threaded reflow** (`threaded_reflow.rs` `reflow` / `reflow_flow` plus a new `src/autosize.rs`).
   - `height_only`: the last frame of the thread (or an unthreaded frame) grows or shrinks so the story's last line fits, insets included. A mid-thread frame with auto-size is refused typed (`autosize_threaded_frame`), a Studio reading, UNVERIFIED.
   - `width_only`: lines do not wrap; width is the widest line plus the insets; height is fixed.
   - `height_and_width`: the width rule, plus height fits.
   - `height_and_width_proportionally`: the smallest uniform scale of the authored frame at which the story fits, found by deterministic bisection to a stated tolerance.
   - Multi-column frames: `height_only` balances nothing and grows to fit the last column. Other kinds with more than one column are refused typed (`autosize_multi_column`).
   - The reference point (STU-LAY-118 `auto_sizing_reference_point`, "automatically adjusted to a compatible value when the type changes") is not stored by Folio. Studio reading: the top edge is fixed for height, the left edge is fixed for width (right edge for right-to-left, which stays refused anyway), and the top-left is fixed for proportional.
   - Minimum height and width gates and `use_no_line_breaks` are also not stored. Readings: no minimum; line breaks allowed.
   - Add a spec-delta / follow-up for an additive Folio record for these six STU-LAY-118 rows (package suite). Do not add it here.
3. **Output.**
   - If `LayoutProposal` carries a prospective document, write the auto-sized frame bounds into it, validated by Folio. Then any host that commits the reflow (the layout editor, IDML/PDF export) gets the new bounds with no host change.
   - Otherwise add an accessor `LayoutProposal::autosized_frames()`, additive, with no new pub field.
   - Report every resize, frame id plus before and after, in the proposal's receipt.
   - InDesign parity is UNVERIFIED (D5). The layout D5 oracle covers composer rows only (`apps.indesign.oracle`). Add no oracle claim.

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (grepped in `master-spec-v02.225/spec-modules/14-studio-creative-suite.md`)

- `[STU-LAY-118]` (:5730): the text-frame preference contract rows `first_baseline_offset`, `minimum_first_baseline_offset`, `auto_sizing_type`, `auto_sizing_reference_point`, the min height/width gates and `use_no_line_breaks_for_auto_sizing`.
- `[STU-LAY-119]` (:5775): the auto-sizing enumeration (off, height_only, width_only, height_and_width, height_and_width_proportionally).
- `[STU-LAY-120]` (:5787): the first-baseline enumeration (ascent, cap_height, leading, em_box_height, x_height, fixed).
- `[STU-LAY-121]` (:5803): vertical justification. Interaction: auto-size height makes vertical justification moot; state that reading.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-frame-autosize-1`, base 91cb8889)

- `hsk-studio-layout`:
  - `src/folio_input.rs`, `src/paragraph.rs`, `src/threaded_reflow.rs`;
  - new `src/autosize.rs`;
  - the `DESCRIPTOR`;
  - the 5 test literals of `ShapedParagraph` (`tests/cjk_kinsoku.rs`, `tests/cjk_mojikumi.rs`, `tests/threaded_reflow.rs`, `tests/typeset.rs`);
  - new `tests/frame_autosize.rs`.
- Do not touch:
  - `hsk-studio-type`: read only. Its OS/2 accessors are public. If you need a new type API, report the signature to the lead; type is upstream of composite and render.
  - `hsk-studio-folio`: no schema change in this lane.
  - `hsk-studio-interop-layout` (IDML mapping of the auto-size attributes is a follow-up).
  - every `harness-view` and `harness` file (SEQ-EXPORT-UI-1, FIG-GRID-2). This lane is not a GUI lane.
  - the Master Spec and root inputs.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **First baseline.**
  - With a fixture face of known OS/2 cap height and x height, the first baseline sits at the hand-computed offset for each of the six kinds.
  - `minimum_first_baseline_offset` larger than the metric wins.
  - A face without the OS/2 value refuses `first_baseline_metric_unavailable`.
- **Auto-size.**
  - `height_only`: a 3-line story at a known leading gives the hand-computed height. Adding a line grows it; deleting lines shrinks it.
  - `width_only`: the width is the widest line plus the insets.
  - `proportional`: the aspect ratio is kept within tolerance and the story fits.
  - A mid-thread auto-size frame and multi-column non-height kinds are refused typed.
  - `off` is byte-identical to before (pin the existing reflow tests unchanged).
- **Hostile input (V4-01/02).**
  - A story that cannot fit at any width (an unbreakable glyph run past the width cap) is refused by the declared bound, never by looping.
  - NaN metrics.
  - Bisection is bounded by the iteration cap and the cancel token.

## Closure and run timing

- `layout` is upstream of `interop-layout`, `harness-view` and `harness`.
- Its dependencies (`type`, `folio`) are unchanged.
- This lane should push FIRST in the wave: SEQ-EXPORT-UI-1 and FIG-GRID-2 both have layout in their closure through harness-view, so they rebase onto it before their bundled runs.
- The bundled run covers (rules 4a and 4b):
  - clippy over layout and its direct dependents;
  - the WHOLE suites of layout and `interop-layout` (which pins reflow output in IDML and PDF export tests);
  - doctests if doc examples change (rule 4d).
- Step 0: `lane-check.sh --step0-only` first. Fixture font paths are relative (rule 4c).
- Do NOT start the bundled run until the lead names the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
