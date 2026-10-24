---
file_id: BUILDER-BRIEF-LANE-SMB-LR-HEALING-2
file_kind: builder_brief
updated_at: 2026-10-24T06:10:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-LR-HEALING-2
lane_file: builders/lanes/hsk-studio-lr-healing-2.json
crates: [hsk-studio-harness-view (view/develop.rs + view/develop/**), hsk-studio-interop-lrcat (src/xmp_sidecar.rs), hsk-studio-develop (src/records.rs only)]
model: opus
effort: medium
author_role: lane planner brief (read-only planning, origin tip e3fc44b5); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
depends_on: [SMB-LR-HEALING-1 READY at c9a32dd6]
---

<topic id="target" status="active" summary="App, rows, expected gain, model">

## Target

- App: Lightroom Classic, row LR-21. HEALING-1 built the engine (develop `retouch.rs`, entry round trip, XMP decode). Its lane file gaps: "viewer develop.rs enable wiring and a GUI spot tool" (NOT_PROVEN: "the viewer does not set enable_retouch from the stack, so retouch areas are stored but not applied in the window"), "XMP encode of RetouchAreas ... new areas are not written", "Lightroom circular/gradient retouch masks ... drop the area with crs_retouch_field_missing", "records.rs copy-settings group for spot removal".
- Expected gain: the strict UI row LR-21 becomes doable in the window (+1 of 35 = +2.9 pp Lightroom GUI strict); a spot-removal workflow is not one of the 13 pooled Lightroom workflows (audit `gui-surfaces.json`), so pooled GUI stays; engine LR-21 loses its "write-back / window" caveats (no level change expected beyond HEALING-1's BUILT_NOT_VALIDATED).
- Model / effort: opus / medium (GUI over a finished engine; one codec write path).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-lr-healing-2`)

- Harness-view: `src/view/develop.rs` (additive: enable wiring beside `:658` `e.enable_mask_group_based_corrections = ...`, new `DevelopOp` variants beside `:347-375`, the spot section in the panel, `state_json` develop keys), NEW `src/view/develop/spot.rs` declared with `mod spot;` inside `develop.rs` (NOT in `view/mod.rs`), NEW `tests/viewer_develop_spot.rs`.
- Interop-lrcat: `src/xmp_sidecar.rs` (`write_sidecar` gains the managed `crs:RetouchAreas` structure; circle masks decode), `tests/xmp_sidecar.rs`.
- Develop: `src/records.rs` only (a `SettingsGroup` member for spot removal, copied by copy / sync settings), `tests/develop_records.rs`. Add the enum member only if no other crate matches `SettingsGroup` exhaustively (`git grep "SettingsGroup::"`); otherwise stop and report.
- RUN-UI.md is SMB-FIG-EDIT-1's tonight: put the operator/model manual lines in the lane file `notes.run_ui_md_lines`.

## Do NOT touch

- `src/view/mod.rs` (FIG-EDIT-1's shared hunks; kept out of this lane so the spot tool is NOT a canvas tool tonight: no `Tool` variant, no `canvas_ui` arm), SMB-FIG-EDIT-2 files (`view/design*`, `tests/viewer_fig_edit*.rs`), `src/view/shell.rs`, `src/view/file_dialog.rs`, `src/view/vector.rs`, `src/view/design*`, `crates/hsk-studio-harness/RUN-UI.md`.
- SMB-ID-LIQUID-LAYOUT-1 files: Folio, `hsk-studio-layout`, package.
- `develop/src/retouch.rs`, `develop/src/local.rs`, `develop/src/dng*.rs` (call them; an engine defect = stop and report), `view/develop_entry.rs` (HEALING-1's round trip already carries `retouch`), `view/develop_more.rs`, `view/edit.rs`, timeline files.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-RAW-015]`: "a heal/clone spot-removal tool with source-point control ... These are develop-scoped operations on `StudioRawDevelop` and reuse the raster retouch primitives of 14.4".
- `[STU-RAW-124]`: "A retouch area is a record carrying `spot_type` (`heal` or `clone`), ... the source offset (`source_x`, `source_y`, `offset_y`, `centre_x`, `centre_y`, `radius`). Retouch areas and red-eye areas are two lists ... gated by `enable_retouch` and `enable_red_eye`."
- `[STU-RAW-105]`: "PANEL ENABLE FLAGS ARE PART OF THE STATE." (the window must set `enable_retouch` / `enable_distraction_removal` from the edit, as it already does for the other groups at develop.rs:654-666).
- `[STU-RAW-018]`: "Every `StudioRawDevelop` control group, mask source, enhance operation, profile and preset action, and workflow option MUST be exposed as a typed, model-steerable command with a stable identifier and MUST be observable through the Studio visual-debug/inspection surface (Argus)".
- `[STU-RAW-103]`: "INTERCHANGE DECODE AND ENCODE." (XMP write-back of the retouch list).
- `[STU-RAW-128]` (snapshots / copy settings basis cited by `records.rs:1-4`).
- Command ids: the spec declares none for spot removal (`grep -n "cmd\.[a-z.]*\(spot\|heal\|retouch\)"` = 0 hits); use stable control ids in the existing `develop.*` scheme (`develop.spot.<i>.<field>`), declared.

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-lr-healing-1.json` (capabilities, `declared_readings`: stage position, power-of-two domain scaling, source offset = centre - source, circle mask when no dab mask, `retouch_source_outside_frame`; `gaps`; `spec_deltas`).
- Code: develop `src/retouch.rs` (`RetouchStack`, `RetouchArea`, `DistractionArea`, `SpotType`, `SourceState`, limits); harness-view `view/develop.rs:1-40` (flow: each accepted change re-develops and commits tiles as ONE chronicle edit), `:347 DevelopOp`, `:470` (`AddBrushDabs` handling), `:640-670` (enables), `:830` (state_json enables); `view/develop_entry.rs` (retouch key); lrcat `src/xmp_sidecar.rs:1-40` (edit model: only managed spans change; retouch list currently "never written back"), `:1200 write_sidecar`; develop `src/records.rs:18-24, :89 SettingsGroup`.

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Enable wiring + typed ops: `enable_retouch = base || !a.retouch.areas.is_empty()`, `enable_distraction_removal = base || !a.retouch.distraction.is_empty()` (adapter records never enable rendering); `DevelopOp::{AddSpot { spot_type, centre, radius, source: Option<point>, feather, opacity }, MoveSpotSource, SetSpot{..}, RemoveSpot, AddDistraction { centre, radius }, RemoveDistraction}` each one undo step through the existing re-develop + tiles commit; control ids `develop.spot.<i>.*`. Tests: `stored_retouch_areas_now_render_in_the_window`, `add_heal_spot_is_one_undo_step`, `clone_spot_source_move_changes_the_copy`, `distraction_spot_fills_and_undo_restores`.
2. Spot section in the Develop panel (panel-local, no canvas tool tonight): a spot list (type heal/clone/distraction, feather, opacity, radius), and a preview image of the developed frame inside the section where a click places a spot, a drag sets its radius and Alt+drag (or a "source" handle drag) sets the source point; spot overlays (target circle, source circle, link line) drawn there; all actions are the slice 1 typed ops. Tests (headless egui + AccessKit, settle the layout before pointer events, F-TL-2): `clicking_the_preview_adds_a_spot_at_the_pointer`, `dragging_the_source_handle_moves_the_source`, `spot_controls_are_accesskit_nodes`, `hostile_spot_values_are_develop_refusals`.
3. XMP write-back + circle masks (lrcat): `write_sidecar` writes `crs:RetouchAreas` as a managed `rdf:Seq` (same CamelCase field names as the decoder; equal list -> bytes unchanged; changed list replaces only that structure's span); decoder accepts a circle-only area (no dab masks: centre/radius) instead of dropping it; gradient retouch masks stay a named loss. Tests: `retouch_areas_round_trip_through_write_and_read`, `unchanged_retouch_list_leaves_the_packet_bytes`, `circle_retouch_area_decodes`.
4. Copy / sync settings group for spot removal (develop `records.rs`): the retouch stack copies with its group (image-specific source points copied as normalised coordinates; declared). Test: `spot_removal_group_copies_and_syncs`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Harness-view headless tests assert through `state_json()["develop"]` (edit, enables, spot list), the committed tiles changing exactly where the spot covers, and EDIT > Undo; lrcat and develop tests are byte/value exact.
- NOT_PROVEN (`NOT_PROVEN:` lines): a spot tool on the main canvas (needs `view/mod.rs`, owned by FIG-EDIT-1 tonight; follow-up); Lightroom spot-tool behaviour and XMP field spelling (no Lightroom oracle, D5); real desktop window.

## Bundled lane-check (C: SSD, rule 4a whole suites)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-lr-healing-2/handshake-studio-assets" && CARGO_BUILD_JOBS=6 bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-lr-healing-2 --disk ssd -- cargo nextest run -p hsk-studio-harness-view -p hsk-studio-harness -p hsk-studio-interop-lrcat -p hsk-studio-develop --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Infra (FIG-EDIT-1 finding): C: builds hit os error 1455 (paging file too small) at default parallelism; prefix your own lane-check and cargo runs on C: with `CARGO_BUILD_JOBS=6` (builders only, never rounds).
- Rule 4a: harness-view, harness (its viewer_develop suites), interop-lrcat, develop (records.rs; drop `-p hsk-studio-develop` from runs before slice 4). Rule 4b: no Folio field: package suite not required. Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, conflicts, deadline">

## Edges and dependencies

- None: harness-view -> develop and interop-lrcat -> develop are declared and used. No crates.io dependency.

## UNVERIFIED (D5)

- Lightroom spot tool gestures (click to place, drag radius, source auto-pick), XMP `crs:RetouchAreas` field spelling and circle-mask encoding.

## Risks and conflicts

- SMB-FIG-EDIT-2 (added by the lead) runs the harness-view/harness suites in parallel and owns `view/design*`, folio, session `source.rs`, package externals and RUN-UI.md tonight: disjoint from this lane; the later pusher rebases and re-runs once. RUN-UI.md lines still go to `notes.run_ui_md_lines`.
- V4-14: HEALING-1 (develop, harness-view develop_entry, lrcat) and SMB-DEVELOP-DNG-WRITER (view/develop.rs) are READY: the lead pins their round candidate at their READY shas or lists these commits.
- The preview inside the panel re-develops at panel resolution: bound its size and reuse the existing develop pipeline (no second develop path).

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
