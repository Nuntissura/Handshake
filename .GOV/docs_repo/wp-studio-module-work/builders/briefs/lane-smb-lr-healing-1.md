---
file_id: BUILDER-BRIEF-LANE-SMB-LR-HEALING-1
file_kind: builder_brief
updated_at: 2026-10-24T04:23:07Z
wp: WP-KERNEL-STUDIO
lane: SMB-LR-HEALING-1
lane_file: builders/lanes/hsk-studio-lr-healing-1.json
crates: [hsk-studio-develop, hsk-studio-harness-view (view/develop_entry.rs only), hsk-studio-interop-lrcat (slice 4)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip b9c311e5); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, row, expected gain, model">

## Target

- App: Lightroom Classic. Row LR-21 "Healing / spot removal / distraction removal" is MISSING, rank 1 of the Lightroom gaps (parity-20261023-adobe-print.json; audit `parity-lightroom_classic.json` LR-21: "refusal only: crates/hsk-studio-develop/src/recipe.rs:58 (first_unsupported: red_eye, retouch, distraction_removal enable flags refuse)").
- Expected gain: LR-21 MISSING -> BUILT_NOT_VALIDATED (+1.4 pp Lightroom engine, 35 rows; +2.9 pp once validated; 9-app pool +0.16 pp). Red-eye stays refused (see spec gaps), and the row name doesn't include it.
- Model / effort: opus / high (STU-RAW-124 record field list, reuse contract with pigment retouch under STU-RAW-015/STU-DOC-004, a public-struct change rippling into harness-view).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-lr-healing-1`)

- Develop: NEW `crates/hsk-studio-develop/src/retouch.rs` (records, validation, rasterised target/source masks, the develop-scoped stage over pigment), NEW `tests/develop_retouch.rs`; additive hunks in `src/lib.rs` (`mod retouch;` / `pub use`), `src/adjust.rs` (one field `retouch: crate::retouch::RetouchStack` on `Adjustments` with an empty `Default`), `src/recipe.rs` (`Enables::first_unsupported` stops refusing `retouch` and `distraction_removal`; `red_eye` keeps refusing), `src/pipeline.rs` (the stage call), `src/local.rs:379` (the `Adjustments {..}` literal there already ends `..Adjustments::default()`: no edit expected). Update `tests/develop_recipe.rs` only where it pins the old `retouch`/`distraction_removal` refusal (rule 4a, F-R35-3); the `red_eye` pin at :267-281 stays.
- Harness-view: `src/view/develop_entry.rs`. Its `Adjustments {..}` literal (:339-396) has no `..Default`, so it must name the new field: decode a `retouch` key (list, bounded) and encode it back, or set the default with a lane-notes entry. No other harness-view file.
- Interop-lrcat (slice 4): `src/xmp_sidecar.rs` decode of the retouch-area list into the new record (STU-RAW-103 interchange decode), `tests/xmp_sidecar.rs`.

## Do NOT touch

- `develop/src/dng.rs`, `dng_write.rs`, `ljpeg_enc.rs` (SMB-DEVELOP-DNG-WRITER, READY in R39); `hsk-studio-pigment/**` (call `retouch::heal`, `clone_stamp`, `spot_heal`, `content_aware::fill`; a missing pigment capability = stop and report); harness-view `view/develop.rs`, `develop_more.rs`, `shell.rs`, `mod.rs`, `file_dialog.rs`, `timeline*`, `vector/**`, `design*` (SMB-FIG-EDIT-1 and SMB-PR-RATE-STRETCH-1 own those tonight); Folio, package, session.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-RAW-015]`: "The pipeline MUST provide non-destructive crop and straighten ..., a heal/clone spot-removal tool with source-point control, and a red-eye and pet-eye correction with pupil size and darken controls. These are develop-scoped operations on `StudioRawDevelop` and reuse the raster retouch primitives of 14.4 where the capability is shared ([STU-DOC-004]); they are not a parallel retouch implementation."
- `[STU-RAW-124]`: "A retouch area is a record carrying `spot_type` (`heal` or `clone`), `heal_version`, `feather`, `opacity`, `seed`, a `method` (`gaussian` observed), a `source_state` (`source_set_explicitly` observed, versus an engine-chosen source), a target dab-mask list with the same brush geometry as [STU-RAW-121], and the source offset (`source_x`, `source_y`, `offset_y`, `centre_x`, `centre_y`, `radius`). Retouch areas and red-eye areas are two lists, both defaulting to empty (`derived` from the always-written set), gated by `enable_retouch` and `enable_red_eye`." Generative path: "That path is a `StudioModelAdapter` lane under [STU-RAS-045] and [STU-RAS-164], not a core develop tool. Studio's core distraction removal is the on-device inpaint of [STU-RAS-143]. Where an adapter produced a fill, the record MUST carry the adapter identity, the model identity and version, whether it ran locally, the patch variation set, and the digests that make the result reproducible and staleness-detectable."
- `[STU-RAW-121]` (brush geometry reused): "A local adjustment is a **correction** holding a list of **masks**" (brush `dabs` as normalised strokes plus `radius/flow/feather`; develop `src/local.rs:1-18` already rasterises them).
- `[STU-RAW-105]`: "PANEL ENABLE FLAGS ARE PART OF THE STATE." `[STU-RAW-106]` determinism. `[STU-RAW-103]`: "INTERCHANGE DECODE AND ENCODE." (slice 4).
- `[STU-RAS-143]`: "CONTENT-AWARE FILL CONTROL CONTRACT (extends [STU-RAS-030])." (core distraction removal through pigment's native fill).
- Group table (grep `| Healing / red-eye |`): "| Healing / red-eye | 62 | see [STU-RAW-124] |".

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- Lane files `builders/lanes/hsk-studio-pigment-retouch.json`, `hsk-studio-pigment-content-aware-2.json` (pigment heal/clone/spot-heal/content-aware: Approximate, UNVERIFIED), `hsk-studio-develop-controls.json` (local masks), `hsk-studio-develop-dng-writer.json` (shared develop files).
- Code: pigment `src/retouch.rs:459 clone_stamp`, `:597 heal` (Poisson membrane, Perez 2003), `:781 spot_heal`, `:854 content_aware_fill`, `RgbaPlane` (straight RGBA f32 in [0, 1]); develop `src/local.rs` (`Brush`, dab rasteriser), `src/recipe.rs:22-66 Enables` / `first_unsupported`, `:283`; `src/adjust.rs:116 Adjustments` / `:220 Default`; harness-view `view/develop_entry.rs:330-398`.

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Records: `RetouchStack { areas: Vec<RetouchArea>, distraction: Vec<DistractionArea>, adapter_fills: Vec<AdapterFillRecord> }`; `RetouchArea` with every STU-RAW-124 field (spot_type heal|clone, heal_version, feather, opacity, seed, method, source_state explicit|engine_chosen, dab masks in STU-RAW-121 geometry, source_x/source_y/offset_y/centre_x/centre_y/radius); `AdapterFillRecord` carrying the mandated identity, version, locality, variations and digests (classified, never rendered: typed `model_unavailable`); bounds and hostile-input refusals (V4-01/02). Tests: `retouch_records_carry_every_spec_field`, `hostile_retouch_lists_are_refused_before_allocation`, `adapter_fill_records_are_classified_not_rendered`.
2. Heal and clone stage: with `enable_retouch`, each area rasterises its target mask from its dabs at the current resolution, maps the source offset, and calls pigment `clone_stamp` (clone) or `heal` (heal); `source_state = engine_chosen` uses pigment `spot_heal`'s candidate search; feather and opacity applied as coverage. Disabled flag = areas retained and not applied. Declared reading (stated in module docs): the stage position in the develop order and the scaling of scene-linear values into pigment's [0, 1] plane domain (refuse rather than clip if a value cannot be represented). Tests: `clone_area_copies_the_source_offset_exactly`, `heal_area_matches_pigment_heal_on_the_same_plane`, `disabled_retouch_keeps_areas_and_changes_nothing`, `retouch_is_deterministic_across_threads`.
3. Core distraction removal: with `enable_distraction_removal`, each `DistractionArea` (dab mask) fills through pigment `content_aware::fill` (STU-RAS-143; Auto sampling, declared seed use); adapter records stay `model_unavailable`. Tests: `distraction_area_fills_through_the_native_inpaint`, `adapter_path_reports_model_unavailable`.
4. Interchange + harness entry: `develop_entry.rs` decodes/encodes `retouch` (round trip through save and reopen); lrcat XMP sidecar decodes retouch areas into `RetouchArea` (unknown fields kept as loss notes). Tests: `develop_entry_round_trips_retouch_areas` (harness-view test file NEW `tests/develop_retouch_entry.rs`), lrcat `xmp_retouch_areas_decode_into_records`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Engine tests on synthetic planes with closed-form expectations (integer offsets copy exactly; heal equals pigment's own result on the same input); no GUI test beyond the entry round trip.
- NOT_PROVEN (`NOT_PROVEN:` lines): equality with Lightroom spot removal (no Lightroom oracle); the window's spot tool (no develop GUI hunk in this lane).

## Bundled lane-check (C: SSD, rule 4a whole suites)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-lr-healing-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-lr-healing-1 --disk ssd -- cargo nextest run -p hsk-studio-develop -p hsk-studio-harness-view -p hsk-studio-harness -p hsk-studio-interop-lrcat --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: develop, harness-view (develop_entry.rs), harness (its viewer_develop suites read develop entries), interop-lrcat (slice 4; drop `-p hsk-studio-interop-lrcat` from runs before slice 4). Rule 4b: no Folio field: package suite not required. `Adjustments` gains a public field: direct dependents that build it by literal are covered above (only `develop_entry.rs` on origin, `git grep "Adjustments {"`). Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, gaps, conflicts, deadline">

## Edges and dependencies

- None: develop -> pigment and harness-view -> develop are declared and used; the lrcat slice stays inside interop-lrcat (lrcat already builds `DevelopRecipe` in `src/xmp_sidecar.rs`), so no harness-view -> interop-lrcat edge is needed. No crates.io dependency.

## UNVERIFIED (D5)

- Lightroom heal algorithm, `heal_version` semantics, feather profile, engine-chosen source search, stage position in the develop order.

## Spec gaps (Operator decides Master Spec edits)

- STU-RAW-015 requires red-eye with "pupil size and darken controls" but STU-RAW-124 lists no red-eye record fields: red-eye stays refused (`UnsupportedContribution("red_eye")`); listed for the HANDOFF spec-delta list. STU-RAW-124 does not state the stage position or the meaning of `heal_version` values.

## Risks and conflicts

- SMB-DEVELOP-DNG-WRITER (READY in R39) touched develop `lib.rs`: additive lines only; post-freeze pushes go to R40 (lead).
- V4-14: if R40 judges a READY lane with harness-view in scope (TRIM-1, SOFT-PROOF-2), the lead pins R40's candidate or lists these commits.
- Pigment planes are bounded to [0, 1]; scene-linear develop values above 1 must not be silently clipped (named refusal or an exact scale/unscale; test it).

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
