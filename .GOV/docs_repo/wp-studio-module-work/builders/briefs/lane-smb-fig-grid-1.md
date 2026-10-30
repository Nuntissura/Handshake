---
file_id: LANE-SMB-FIG-GRID-1
file_kind: lane_brief
updated_at: 2026-10-30T15:45:34Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIG-GRID-1
lane_file: builders/lanes/hsk-studio-fig-grid-1.json
crates: [hsk-studio-folio (additive auto-layout fields), hsk-studio-components (grid solver + commands), hsk-studio-interop-fig-native (one literal line), hsk-studio-package (suite only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Figma GRID auto layout: records and solver, engine only">

## Target

Gap source: `research/parity-20261023-figma-affinity-shell.json`, key `per_app[app=figma].top_gaps` rank 4: "GRID auto layout and SLOT component properties refused (FG-07 / FG-04 weak AGREE); layout grids MISSING (FG-22)", spec `[STU-DS-146]`, `[STU-DS-110]`. This lane covers GRID only; SLOT and layout grids are not in scope.

Still open at 2a60acf3 (checked by planner, read-only):
- Folio refuses GRID: `src/autolayout.rs:248`, `layout_mode == Grid` returns `UnsupportedDescriptor`. The module note says the grid container and child fields (STU-DS-146/147/150) are "recognised and refused".
- The components solver refuses GRID: `src/auto_layout.rs:285` returns `UnsupportedLayout("grid")`.
- `LayoutMode::Grid` already exists, so no enum change is needed.

Slices, with a local commit each:

1. **Folio records**, additive (package suite, rule 4b).
   - Add `StudioAutoLayout::grid: Option<StudioGridLayout>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Its fields:
     - `grid_row_count`, `grid_column_count` (1..=1000 `studio_declared`, default 1);
     - `grid_row_gap`, `grid_column_gap` (the STU-DS-145 grid column: 0..=100000 `studio_declared`);
     - `grid_row_sizes`, `grid_column_sizes` (`Vec<StudioGridTrackSize>`; STU-DS-147 `{type FIXED|FLEX|HUG, value?}`, where `value` is required for FIXED and FLEX and absent for HUG; FLEX hard_min 0, default 1);
     - `grid_auto_tracks` (`NONE|ROWS`);
     - `grid_items_positioning` (`MANUAL|ROW_AUTO_FLOW`).
   - Add `StudioChildLayout::grid_child: Option<StudioGridChild>` (STU-DS-150) with `grid_row_anchor_index`, `grid_column_anchor_index`, `grid_row_span`, `grid_column_span`. `grid_child_index` is read-only and derived, never stored.
   - Validation:
     - `grid` must be `Some` exactly when `layout_mode == GRID`;
     - track lists must match the counts;
     - spans must stay within the declared counts (STU-DS-150: "a validation error");
     - `layout_wrap` and the flow-only fields keep their current rules;
     - each failure gets a typed `Code` at `<node>/auto_layout/grid/<field>`.
   - Documents without grid fields stay byte-identical (test this).
   - **Struct-literal impact.** Adding a field to `StudioAutoLayout` breaks its 8 literals: 5 in `components tests/auto_layout.rs`, 2 in `folio src/autolayout.rs`, 1 in `interop-fig-native src/native.rs`. Adding a field to `StudioChildLayout` breaks its 2 literals: 1 in `components tests/auto_layout.rs`, 1 in folio.
     - Fix each with `grid: None` / `grid_child: None`.
     - The `native.rs` one-liner is the only hunk outside folio and components. SMB-AI-ARROWHEADS-2 owns that file this wave: stage only your line and rebase after ARROWHEADS-2 pushes.
     - Rejected alternatives:
       - a member on `StudioNodeDesign`, which has 12 literals in 6 crates;
       - a member on `StudioDesignRecords`, which has about 30 literals;
       - a separate grid record set, which would give the solver a second input and put two homes on one STU-DS-146 contract.
   - Update the folio `DESCRIPTOR` design-records sentence (`src/lib.rs`).
2. **Components solver** (new `src/auto_layout/grid.rs` or `src/grid_layout.rs`, called from `solve_auto_layout` in place of the refusal).
   - Track sizing: FIXED, then HUG (the content size of non-spanning items, then spanning items distributed), then FLEX (the leftover space by factor), with gaps and padding.
   - `ROW_AUTO_FLOW` places items in document order, row-major, skipping occupied cells. `MANUAL` uses the anchors and spans.
   - `grid_auto_tracks = ROWS` adds HUG rows when auto-flow runs out of cells. Under `NONE`, overflow is a typed refusal.
   - Child alignment within a cell follows `layout_align` and STU-DS-148 sizing (`FILL` stretches to the cell area).
   - Container hug (AUTO) sums the tracks.
   - Absolute children keep their constraints (STU-DS-048), as in the flow path.
   - The track algorithm is a Studio reading modelled on the W3C CSS Grid track sizing algorithm (css-grid-1 §11, simplified). Figma's algorithm is UNVERIFIED (D5). Write the readings into the module docs and the `DESCRIPTOR`, and keep every number hand-checkable in f64 (the existing module's stance; `taffy` not adopted).
3. **Commands** (pure functions in components, Folio-validated proposals; components mints no ids):
   - `grid_append_child_at(design, container, node, row, column)`;
   - `grid_reorder_rows(design, container, options)` and `grid_reorder_columns(design, container, options)`. Both return the resulting ordered track entries, as STU-DS-146 requires.
   - Each returns the new design records or a typed refusal (out of range, cell occupied under MANUAL, not a GRID container).

Not in scope:
- The Figma importer mapping of GRID frames (`interop-fig-native` still refuses them: a follow-up once ARROWHEADS-2 has released `native.rs`).
- A GUI.
- SLOT properties (STU-DS-110).
- Layout grids (STU-DS-153..156).
- `BASELINE`.

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (grepped in `master-spec-v02.225/spec-modules/14-studio-creative-suite.md`)

- `[STU-DS-144]` (:19873): the container field set. GRID is a `layout_mode` member.
- `[STU-DS-145]` (:19897): the spacing contract, including the `grid_row_gap` / `grid_column_gap` column (hard_min 0, Studio-declared hard_max 100000, labelled `studio_declared`).
- `[STU-DS-146]` (:19912): exactly eight grid fields and three commands, with the reorder commands returning the ordered tracks.
- `[STU-DS-147]` (:19927): the track-size record.
- `[STU-DS-148]` (:19929): child align and sizing fields.
- `[STU-DS-149]` (:19939): clamps; min > max is a validation error.
- `[STU-DS-150]` (:19941): grid child anchors and spans, read-only `grid_child_index`; a span past the count is a validation error.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-fig-grid-1`, base 2a60acf3, rebased onto the base the lead names)

- `hsk-studio-folio`:
  - `src/autolayout.rs`;
  - the `DESCRIPTOR` sentence in `src/lib.rs`;
  - new `tests/auto_layout_grid_records.rs` (or new rows in the existing design-records contract test).
- `hsk-studio-components`:
  - `src/auto_layout.rs`, a new grid module, and the command functions;
  - `tests/auto_layout.rs` (the 6 literal lines);
  - new `tests/auto_layout_grid.rs`.
- `hsk-studio-interop-fig-native`: ONE line, `grid: None`, at the `StudioAutoLayout` literal in `src/native.rs`, after ARROWHEADS-2 has pushed.
- `hsk-studio-package`: `tests/externals.rs` only, and only if its classifier needs the new field names (rule 4b).
- Do not touch:
  - any `hsk-studio-harness-view` or `hsk-studio-harness` file (this is not a GUI lane);
  - `hsk-studio-pulse`, `hsk-studio-harness-core` (SMB-PR-NEST-2);
  - `hsk-studio-interop-vector-svg` and the rest of `interop-fig-native` (SMB-AI-ARROWHEADS-2);
  - `hsk-studio-nib`, `hsk-studio-composite`;
  - the Master Spec and root inputs.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Folio.**
  - A GRID record round-trips through wire JSON.
  - A non-grid document is byte-identical to before.
  - Typed refusals:
    - `grid` absent under GRID, or present under HORIZONTAL;
    - a track count mismatch;
    - FLEX without a value, or HUG with a value;
    - a span past the count;
    - a gap of -1 or 100001.
- **Solver.**
  - A 2x3 grid with FIXED 100 / FLEX 1 / FLEX 2 columns in a 500-wide container, gap 10 and padding 20: column positions computed by hand.
  - HUG rows taking the tallest child.
  - A spanning child.
  - `ROW_AUTO_FLOW` skips a MANUAL-anchored cell.
  - `grid_auto_tracks ROWS` adds a row; `NONE` overflow is refused typed.
  - A nested grid inside a horizontal flow.
- **Commands.**
  - `append_child_at` sets the anchors.
  - Reorder rows returns the ordered tracks and moves anchored children with their rows.
  - Out-of-range and occupied-cell refusals change nothing.
- **Hostile input (V4-01/02).** Each case is refused before allocation:
  - 1000x1000 counts with huge track lists;
  - NaN sizes;
  - spans of `u32::MAX`.

## Closure and run timing

- Folio is upstream of every crate in the workspace.
- `components` is upstream of `harness-view`, `harness`, `interop-fig-native` and `interop-fig`.
- So this lane pushes LAST in the wave, after NEST-2 and ARROWHEADS-2. Pushing earlier forces both GUI lanes to rebase and re-run.
- The bundled run covers (rules 4a, 4b and 4e):
  - clippy over folio, components, `interop-fig-native` and their direct dependents (with folio this is the workspace);
  - the WHOLE suites of folio, components, `interop-fig-native`, `interop-fig` and `hsk-studio-package` (rule 4b: Folio record fields were added);
  - doctests if doc examples change (rule 4d).
- Step 0: `lane-check.sh --step0-only` first. No drive-letter literals (rule 4c).
- Code and test now with focused runs on D:. Do NOT start the bundled run until the lead names the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
