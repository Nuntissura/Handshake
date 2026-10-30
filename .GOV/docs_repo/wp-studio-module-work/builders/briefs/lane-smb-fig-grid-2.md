---
file_id: LANE-SMB-FIG-GRID-2
file_kind: lane_brief
updated_at: 2026-10-30T16:43:48Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIG-GRID-2
lane_file: builders/lanes/hsk-studio-fig-grid-2.json
crates: [hsk-studio-interop-fig-base (decode), hsk-studio-interop-fig-native (map), hsk-studio-harness-view (design/auto_layout + one Properties hook), hsk-studio-harness (viewer test)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Figma GRID end to end: .fig GRID frames import; window auto-layout section with grid mode, tracks and spans">

## Target

Gap source:
- `builders/lanes/hsk-studio-fig-grid-1.json` (READY at 91cb8889): the records and solver exist, but nothing imports or edits them.
- `research/parity-20261023-figma-affinity-shell.json` `per_app[app=figma].top_gaps` ranks 2 and 4: no design-editing surface for auto layout, and GRID refused.

Read FIG-GRID-1's `notes.declared_readings` first. Columns are the primary axis. `grid_child` present means anchored; absent means auto-flowed. Reorder options are `GridTrackMove {from_index, to_index}`.

State at 91cb8889 (checked by planner, read-only):
- **Decode.** `interop-fig-base src/nodes.rs:177` `AutoLayout` decodes only `stackMode`, `stackSpacing` and the four paddings (`:1192-1199`). No grid field is read.
- **Mapping.** `interop-fig-native src/native.rs:1477` maps `"GRID"` to `LayoutMode::Grid` and then writes `grid: None` (`:1520`). Folio refuses that with `MissingField` (GRID-1 reading), so a `.fig` with a GRID frame fails its design records.
- **Window: correction to the dispatch premise.** There is no auto-layout panel at all, flow or grid. No `harness-view` source mentions `auto_layout`, `LayoutMode` or `solve_auto_layout` (git grep). The window never runs the components solver. Imported auto-layout frames are static geometry, and their design records ride in the vector entry's `design` (`vector.rs:316`) and the import receipt.

Slices, with a local commit each:

1. **Decode** (`interop-fig-base`).
   - Read the grid fields from the `.fig` kiwi message, using the field names the file's OWN embedded kiwi schema declares. `.fig` files carry their schema, so read it from the Operator fixtures; never guess.
   - The fields are: the container row and column counts, the gaps, the row and column track sizes (type and value), auto tracks and item positioning; and per child, the row and column anchor and span.
   - Add them as `AutoLayout::grid: Option<FigGrid>` and a child-side `Option<FigGridChild>` on the node payload. `AutoLayout` has one literal, in `nodes.rs:1192`.
   - If no fixture schema declares grid fields, stop slice 1 and report the exact names you searched for. Do not invent kiwi names; Figma's Plugin API names are not the kiwi names, UNVERIFIED.
2. **Map** (`interop-fig-native src/native.rs`).
   - GRID writes `grid: Some(StudioGridLayout)` and the children's `grid_child`, within FIG-GRID-1's bounds.
   - Out-of-range or missing values get a typed loss (`auto_layout_grid_*`) and fall back to NONE layout. Never write `grid: None` under GRID again.
   - Run the components solver on the imported tree in a test, and compare child rects to the `.fig`'s stored child geometry within a stated tolerance. That compares against Figma's own output: real parity evidence. Record it as FIXTURE_PROVEN or UNVERIFIED per frame.
3. **Window, headless first** (new `src/view/design/auto_layout.rs`).
   - Add `Viewer` commands on the selected frame or group layer with design records:
     - `set_auto_layout_mode(node, NONE|HORIZONTAL|VERTICAL|GRID)`;
     - `set_grid_tracks(node, rows, columns)` (counts plus `StudioGridTrackSize` lists);
     - `set_grid_gaps(node, row_gap, column_gap)`;
     - `set_grid_child(node, Option<StudioGridChild>)`;
     - `grid_append_child_at(container, node, row, column)` and `grid_reorder_rows` / `grid_reorder_columns`, through the components commands (`components src/auto_layout/grid.rs:553`, `:717`, `:722`).
   - Each command:
     1. edits the design records;
     2. re-solves the container with `solve_auto_layout` (`components src/auto_layout.rs:732`);
     3. applies the solved rects to the children;
     4. commits all of it as ONE undo step, the document change plus the `vector` entry's `design` member, through the existing entry-carrying step (`Run::set_vector_paths_and_entry`, as the vector edits do).
   - Applying rects, as a Studio reading:
     - translate every child;
     - resize only children whose geometry is a frame, artboard or rectangle;
     - a FILL/HUG size change on any other child kind is a typed refusal (`auto_layout_resize_unsupported:<kind>`). Never scale paths silently.
   - Folio and solver refusals pass through typed.
   - Projected in `state_json()["design"]["auto_layout"]`.
4. **Window section.**
   - `properties_ui` (`mod.rs:1081`) calls one new `auto_layout_ui(ui)` from `design/auto_layout.rs`. That is the only `mod.rs` hunk.
   - The section shows, for a frame with design records:
     - mode;
     - for flow: spacing and padding (read and edit, using the same commands);
     - for GRID: row and column counts, a track-size row per track (FIXED/FLEX/HUG plus value), gaps, auto tracks, item positioning, and Reorder buttons;
     - for a child of a GRID frame: anchor and span fields.
   - Widget ids are `design.auto_layout.*`. Values outside the bounds show folio's typed error inline. Nothing is clamped in the view.

Figma behaviour stays UNVERIFIED (D5) except where slice 2 compares against fixture geometry. Not in scope: SLOT properties (STU-DS-110), layout grids (STU-DS-153..156), BASELINE, and resizing vector children.

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (module 14, master-spec-v02.225)

- `[STU-DS-144]` (:19873), `[STU-DS-145]` (:19897), `[STU-DS-146]` (:19912), `[STU-DS-147]` (:19927), `[STU-DS-148]` (:19929), `[STU-DS-149]` (:19939), `[STU-DS-150]` (:19941): the container, track, child and command contracts. Bounds and readings are FIG-GRID-1's.
- `[STU-DS-001]` (design import) and `[STU-IO-006]` row 35 (`.fig` edit-preserve): import losses stay typed.
- `[STU-DS-160]`: properties bound contextually to the selection.
- `[STU-CMP-110]`: every control has a headless Viewer path.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-fig-grid-2`, base 91cb8889)

- `hsk-studio-interop-fig-base`: `src/nodes.rs` (decode) plus a new test.
- `hsk-studio-interop-fig-native`: `src/native.rs` (the `auto_layout` mapping and the child `grid_child` mapping) plus new `tests/fig_grid.rs`.
- `hsk-studio-harness-view`:
  - new `src/view/design/auto_layout.rs`;
  - `src/view/design.rs` (the `pub mod` line);
  - `src/view/mod.rs`: ONE hunk in `properties_ui`.
- `hsk-studio-harness`: new `tests/viewer_design_auto_layout.rs` (`#![cfg(feature = "viewer")]`).
- Do not touch:
  - the timeline files, `timeline_export*` and `cli.rs` (SMB-PR-SEQ-EXPORT-UI-1);
  - `hsk-studio-layout` and `hsk-studio-interop-layout` (SMB-ID-FRAME-AUTOSIZE-1);
  - `hsk-studio-folio` and `hsk-studio-components` (read only: use FIG-GRID-1's API; if a change is needed, report the signature to the lead);
  - `RUN-UI.md` (list its lines in the notes), the Master Spec, and root inputs.
- This is GUI lane 2 of 2 (with SEQ-EXPORT-UI-1).

</topic>

<topic id="proof" status="active" summary="Proof, closure and run timing">

## Proof

Red first: write each slice's tests and commit them red before the code.

Focused tests, 5 or fewer per slice:

- **Decode and map.**
  - An Operator `.fig` fixture or a synthetic kiwi message built from the fixture schema's field names: a 2x3 GRID frame imports with the counts, sizes, gaps and child anchors/spans, and `validate_design_records` passes.
  - A malformed track gives a typed loss and falls back to NONE.
  - Solved child rects against the stored `.fig` geometry (tolerance stated).
  - A non-grid `.fig` imports byte-identically to before.
- **Commands.**
  - Switch a 3-child frame to GRID 2x2: the children move to the hand-computed cells, in one undo step that undo restores byte-identical.
  - Set a FLEX column: positions are recomputed.
  - A span past the count: folio refusal, document unchanged.
  - Reorder rows: anchored children follow.
  - A FILL child that is a vector path: `auto_layout_resize_unsupported`.
- **Section.** The `design.auto_layout.*` ids drive the same commands headlessly. A frame without design records shows no section.

## Closure and run timing

- `interop-fig-base` is upstream of `interop-fig-native` and `interop-fig`; `interop-fig-native` of `interop-fig`; `interop-fig` of `harness-view` and `harness`; `harness-view` of `harness`.
- SMB-ID-FRAME-AUTOSIZE-1's `layout` is upstream of harness-view. If it pushes first, rebase and re-run.
- SEQ-EXPORT-UI-1 shares harness-view and harness with disjoint files. Whichever pushes second rebases and re-runs.
- The bundled run covers (rules 4a and 4e):
  - clippy over `interop-fig-base`, `interop-fig-native` and their direct dependents;
  - the WHOLE suites of `interop-fig-base`, `interop-fig-native`, `interop-fig`, `harness-view` (round units `["viewer"]` and `["desktop","audio-batch"]`) and `harness`;
  - the fig corpus `#[ignore]` units named in `.config/round-features.toml`, if the fixtures are used.
- Step 0: `lane-check.sh --step0-only` first. Fixture paths are relative to the corpus root, never drive-letter literals (rule 4c).
- Do NOT start the bundled run until the lead names the disk and the base. R43 holds C:.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS.

</topic>
