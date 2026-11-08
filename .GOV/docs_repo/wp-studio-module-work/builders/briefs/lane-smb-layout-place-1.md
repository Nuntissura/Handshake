---
file_id: LANE-SMB-LAYOUT-PLACE-1
file_kind: lane_brief
updated_at: 2026-11-08T08:50:53Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYOUT-PLACE-1
lane_file: builders/lanes/hsk-studio-layout-place-1.json
crates: hsk-studio-harness-view (layout graphic frames: place, fit, link states and the layout PDF's asset resolver), hsk-studio-harness-core (asset rows, one Run step for layout records plus the raster recor
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A picture placed into a graphic frame on a layout page is stored, fitted, link-checked, one undo step, saved and drawn in the layout PDF, through asset.place and one fit command">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/width-plan-20261108-print-web.json. Every open question below takes the planner's recommended default as a lead ruling. Its code facts were checked at 0a733db8; origin is now eb17fad6, whose new commits touch only hsk-studio-native-host and kumihimo-ui. Re-read every file:line at your base before you edit it.

- Is a frame required when placing on a page, or does asset.place without a frame make one at the content's 100 % size? -> Required on a pages-only document (frame_required): deterministic for a model and no hidden page-origin rule; a later slice can add the content-size frame.
- Content size at 100 %: from the source file's ppi, or always 72 ppi? -> The decoder's ppi when the format states one, else 72, recorded in the result so a model sees which; raise an SD item.
- Export with a modified linked placement: refuse (place.rs convention) or render the current bytes (STU-ASSET-007 row modified)? -> Refuse placed_link_modified and name asset.update_all_modified, because the cached tiles are the old bytes and rendering the new ones at export needs a decode the export path does not do; raise an SD item.
- Name of the fit command: asset.fit, or a layout.* id? -> asset.fit in commands/asset.rs: no spec id is verified for fitting, asset rows already route to place.rs, and commands/layout.rs is edited by the in-flight indent lane.
- Zero shared files with SMB-TYPE-INDENT-SPACING-1, or disjoint hunks allowed in layout_export.rs? -> Disjoint hunks allowed (the in-flight list names layout_export.rs:771); the layout_export.rs commit is the lane's last and rebases on whichever lands first.
- Does a layout instance in raster_records keep the identity transform in source pixels, with the frame owning placement? -> Yes: the frame record is the one placement authority; the instance keeps source and resample only, so there is no second transform to disagree.

- Process: red first per item, then focused runs on D:. Stop at REVIEW READY for the read-only code-check and red-team review, then run ONE bundled run, then stop at BUNDLED GREEN. Lanes land one at a time on the lead's RELEASED (HANDOFF topic workflow, items 10, 11 and 16).
- Any lane that changes a Viewer command's refusal order also runs every harness process test that drives that command. This is the lesson from the 2026-11-08 trunk break at 35c50d3b.

## Target

Evidence: print scenario gap G5 (rank 4; steps 48, 49, 50, 51, 54, 83), deck step 42 of scenario-board-slides-web, ledger `direction.next_lane_candidates` 2026-11-07T16:35:52Z "Placed graphics, vector paths and raster tiles in the layout PDF (listed as losses)". The lane name is the code's own: `harness-view/src/view/place.rs:9-10` says a document with pages only refuses `canvas_unavailable` and "layout graphic frames are lane SMB-LAYOUT-PLACE-1". All file:line below are READ at 0a733db8 unless marked; paths are under `handshake-studio-assets/crates/hsk-studio-`.

What exists and has no caller (reuse it, build nothing beside it):
- Folio record: `StudioGraphicFrame` `folio/src/longdoc.rs:380-405` (layer_id of a raster layer, page_id, bounds in pt page coordinates, `shape` Rectangle | Ellipse | Path `:361-368`, content size at 100 %, `fit`, `frame_fitting`, anchor, crop), held in `StudioLongDocRecords.graphic_frames` `:1470`, validated `:1688-1768`, budget `MAX_GRAPHIC_FRAMES` 65536 `:101`; fit tokens `StudioFit` `:311`, anchor `StudioFitAnchor` `:326`.
- Layout geometry: `fit_content` `layout/src/fitting.rs:81` (all STU-LAY-131 members; `content_aware` is a typed `content_aware_unavailable`, docs `:18-19`), `place_graphic` `:198` returning `PlacedGraphic` `:182`.
- PDF lowering: `draw_layer` `interop-layout/src/pdf_export.rs:665-680` (raster arm `:675-678`) sends a raster layer that has a graphic frame to `graphic_layer` `pdf_export/longdoc.rs:320-372` (tiles validated, content mapped by the fitting transform, clipped to rectangle or ellipse, 16-bit ICCBased with the tile's own profile, SMask from alpha); `PdfAssets` `pdf_export.rs:127-140`, `PlacedRaster` `:118-124`, `PdfLongDoc.graphics` `longdoc.rs:56`.
- Persistence: `LayoutRecords` carries `longdoc` (`session/src/layout_records.rs:46-56`); `Run::submit_layout` `harness-core/src/harness.rs:2053`; `Viewer::layout_commit` `harness-view/src/view/shell.rs:792`.
- Place pipeline (artboards): `decode_source` `place.rs:545`, source sharing `:700-712` (STU-RAS-013), `stage_tile` use `:627` (tiles) and `:723` (source), entry `raster_records` `:92` schema `:96`, commit `restructure_and_entry` `:757` (`harness.rs:1488`), `list_placements` `:922`, `update_all_modified` `:936`, `embed_link` `:1019`, codes `:53-65`.
- Command route: `asset.place` row `harness-core/src/action/commands/asset.rs:11-13`; dispatch `harness-view/src/view/commands.rs:149` to `view/commands/place.rs:22-37`.

What fails today: `place_inner` takes `self.canvas()` and refuses `canvas_unavailable` for a pages-only document (`place.rs:681-683`, also `placement_tiles` `:639`). The layout PDF resolves nothing: `NoAssets` `layout_export.rs:193-205`, `PdfLongDoc { page_labels, ..default }` `:691`, `design: None` `:694`, `assets: &NoAssets` `:698`, declared at `:52-57`.

Clauses (module 14, v02.229): STU-LAY-028 :6093, STU-LAY-029 :6132, STU-LAY-131 :6137, STU-LAY-030 :6162, STU-LAY-132 :6168, STU-LAY-031 :6202, STU-LAY-133 :6206, STU-LAY-054 :6959, STU-LAY-158 :7117, STU-RAS-012 :664, STU-RAS-013 :669, STU-ASSET-005 :31320, STU-ASSET-006 :31336, STU-ASSET-007 :31338, STU-ASSET-009 :31352, STU-ASSET-010 :31354, STU-AUT-147 :21905, STU-CON-007 :22896, STU-PDF-002 :243.

### Items, red first

1. **Red, compiling.** New `harness/tests/layout_place.rs`. Through `Viewer::run_command` and once through the process (the method of `harness/tests/layout_pdf_host.rs:292`): `document.new` profile `layout` 595 x 842 pt (`harness-core/src/document_new.rs:298-300`); `asset.place` of an embedded 64 x 48 PNG written by the test into a frame on `page-1`; `layout.export_pdf`; assert `tiles_drawn` > 0 (summary field `layout_export.rs:727`) and no `raster_unresolved` loss; `layout.render_pdf_page` of page 0 and assert pixels inside the frame are not white and pixels just outside it are white (clip). Commands are strings, so the red commit compiles. Prove red by running: today `asset.place` refuses `canvas_unavailable`.

2. **Place into a frame.** New egui-free `harness-view/src/view/layout_place.rs`; `place.rs` gains only a branch at `:681` (and `:639`) that hands a pages-only document to it.
   - `asset.place` gains optional `page`, `frame_x`, `frame_y`, `frame_width`, `frame_height` (pt, page coordinates: the record's `bounds`), `shape` (`rectangle` | `ellipse`; a `Path` frame needs a vector path id and refuses `frame_shape_unsupported` in this slice), `fit` (the `StudioFit` tokens legal for a frame: not `honour_existing_style`, fitting.rs docs), `frame_fitting`, `anchor`. On a pages-only document `page` and the four frame numbers are required (`frame_required`; see open question).
   - Pixels: the layer's tiles hold the SOURCE at its own pixel size (no resample, no canvas clip), so the PDF carries the full resolution; `content_width`/`content_height` = pixels x 72 / source ppi (open question for the ppi source). Bound `MAX_PLACE_PIXELS` (`place.rs:100`), refused before decode where the header gives the size.
   - Records: one raster layer whose parent chain reaches the page's spread (otherwise PDF reports `layer_not_on_page_spread`, `pdf_export.rs:668-671`); its `StudioRasterLayerRecord.placed_asset` and `Instance` in the `raster_records` entry as on artboards; one `StudioGraphicFrame` in `longdoc.graphic_frames`. Follow how `LayoutEdit::DrawFrame` adds a layer to the layout document (NOT_INSPECTED: its layer-graph handling; read `layout_ui.rs` `transform` DrawFrame arm before writing).
   - ONE undo step: new `Run::submit_layout_and_entry(op, command, document, records, token, bytes)` beside `submit_layout` (`harness.rs:2053`), committing the `LayoutStep` and the entry into one `HistoryStep` (both fields `layout` and `entry` exist, `harness.rs:358-380`), modelled on `restructure_and_entry_step` (`:1492-1532`): a refused entry write undoes the layout half, no split step. NOT_INSPECTED: whether the undo and redo dispatch applies both fields of one step; read it first and extend it there if not. Test: undo removes layer, frame and entry bytes together; redo restores them byte-identically.

3. **Fit.** New row `asset.fit` (name: open question) in `commands/asset.rs`, arm in `view/commands/place.rs:22`: `{layer, fit, anchor?, crop_top?, crop_left?, crop_bottom?, crop_right?, frame_fitting?}` re-fits a graphic frame through `fit_content`; `frame_to_content` resizes the frame record; `apply_frame_fitting_options` uses the stored rule and refuses without one; one layout step; a layer without a frame refuses `layer_not_graphic_frame`.

4. **Links on layout frames.** `list_placements` (`place.rs:922`) lists layout placements with `page_id`, frame bounds, fit and the derived `effective_ppi` and `actual_ppi` (STU-LAY-132 last paragraph, spec :6168 block). `update_all_modified` (`:936`) re-decodes a layout placement at source resolution, not through `placement_tiles`' canvas clip, and keeps its frame. `transform.move`, `transform.free_transform`, `transform.reset` (`place.rs:773-785`) on a graphic-frame layer refuse `graphic_frame_transform_unsupported`: moving the pixels while the frame record stays would draw wrong; frame move is a later slice.

5. **The layout PDF draws it.** `layout_export.rs`, hunks `:52-57` (docs), `:193-205`, `:665-698` only; never `:740-776` (SMB-TYPE-INDENT-SPACING-1 edits `:768-775`).
   - `NoAssets` replaced by a resolver in new `view/layout_assets.rs`: `raster(layer_id)` builds `PlacedRaster` from the layer's `TileRef`s and their bodies (the `ResolvedTile` shape as `view/brush_mix.rs:75-76` `tile_view` builds it; NOT_INSPECTED: where a reopened package's tile bodies are read from, read how `render` gets them before writing); `profile(id)` gives the working profile's bytes; `path` stays `None` (rectangle and ellipse only).
   - `PdfLongDoc.graphics` filled from `place_graphic` for every record in `rec.longdoc.graphic_frames`.
   - STU-ASSET-009 gate before any shaping: a linked layout placement whose state is `missing`, `unauthorized` or `unsupported` refuses `placed_link_missing`, `placed_link_unauthorized`, `placed_link_unsupported` (codes of `place.rs:53-65`) and writes nothing; `modified` refuses `placed_link_modified` (open question). The summary gains `placements`: per placement `layer_id`, `source_id`, `placement_kind`, `uri`, content digest and link state at resolution time.
   - Module docs `:52-57`: placed graphics drawn; vector paths, tables, outlines, CMYK image numbers stay declared gaps.

6. **Docs and MTs.** `place.rs` module manual (`:1-75`) gains the layout section and the new codes; `commands/asset.rs` row docs. Read MT-30721, MT-30722, MT-30723, MT-30730..MT-30736, MT-157, MT-162, MT-163, MT-170 (print G5 list, NOT_INSPECTED in this plan) and claim a row only if it is read and proven here. Do not edit MT files.

Hostile input (V4-01/02), each a typed refusal that changes nothing (package bytes identical): frame numbers non-finite, zero, negative size, a side over 14400 pt (`PAGE_SIDE_PT`, `layout_export.rs:91`); unknown page; source over `MAX_SOURCE_BYTES`; pixels over `MAX_PLACE_PIXELS`; the 65537th frame (`MAX_GRAPHIC_FRAMES`); a crop that empties the content box; `fit` on a layer without a frame; `content_aware`; a link outside the document folder (`link_outside_document_folder`); a canceled token during decode and during export.

Determinism: two exports of the same document are byte-identical (press export is deterministic, `press-write/src/export.rs:15-16`); save, reopen, export gives the same sha256.

Vendor parity stays UNVERIFIED (D5): 100 % at source ppi, frame-required placement and the modified-link refusal are Studio readings.

Out of scope: frame move and resize; anchored graphics and text wrap; clipping paths, alpha and detect-edges (STU-LAY-133); placing PDF, AI, INDD or a layout document (STU-LAY-130 crop members); multi-file load and place cursor; IDML export of graphic frames; vector paths, tables, outlines and tags in the PDF; CMYK image numbers (step 50: place decodes into the working space); preflight link rules; any panel, menu item, egui line, `Cargo.toml` change, Master Spec edit or MT edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: typed codes through CommandRefusal (view/commands/place.rs:15-17) and LayoutExportRefusal (layout_export.rs:104-114); the export keeps its one admitted job and terminal Observe frame (layout_export.rs:543-561); no new sink", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: every commit goes through Run (submit_layout and the new submit_layout_and_entry) under the run's command context, as layout_commit (shell.rs:792) and restructure_and_entry (harness.rs:1488) do; no default_actor() or literal actor in new code", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-toc-persist-1`, branch `lane/smb-layout-place-1`, base eb17fad6)

- harness-view: new `src/view/layout_place.rs`; new `src/view/layout_assets.rs`; `src/view/place.rs` (branch at :639 and :681, list_placements :922, update_all_modified :936, transform refusals :773-785, module docs :1-75); `src/view/layout_export.rs` (only :52-57, :193-205, :665-698); `src/view/commands/place.rs` (asset.place parameters, asset.fit arm); `src/view/mod.rs` (two mod lines beside :94 and :116).
- harness-core: `src/action/commands/asset.rs` (parameters, one row); `src/harness.rs` (submit_layout_and_entry beside :2053; undo and redo arm only if a step with both layout and entry is not applied today).
- harness: new `tests/layout_place.rs`.
- package: `tests/externals.rs` only if rule 4b requires it (no new entry kind is expected: layout_records and raster_records exist).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-id-toc-persist-1 --disk d`): step 0; clippy over hsk-studio-harness-core, hsk-studio-harness-view (viewer) and hsk-studio-harness; nextest WHOLE suite of hsk-studio-harness-core; hsk-studio-harness-view with [viewer]; hsk-studio-harness tests layout_place, layout_pdf_host, layout_persist, place_asset, harness_drive_typed, document_new (rule 4e: confirm the units in .config/round-features.toml); session, layout, interop-layout and folio suites only if the lane edits them, which this scope does not; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-id-toc-persist-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
