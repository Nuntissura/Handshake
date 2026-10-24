---
file_id: BUILDER-BRIEF-LANE-SMB-AI-AREA-TYPE-1
file_kind: builder_brief
updated_at: 2026-10-24T08:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-AREA-TYPE-1
lane_file: builders/lanes/hsk-studio-ai-area-type-1.json
crates: [hsk-studio-type, hsk-studio-composite, hsk-studio-folio (src/vector.rs member + src/vector_path_text.rs only), hsk-studio-harness-view (src/view/vector/path_text.rs only, slice 3)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip bfb5843c); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, rows, expected gain, model">

## Target

- App: Illustrator. Two PARTIAL rows (audit `parity-illustrator.json`): AI-28 "Type: point/area type, type on a path" (path text laid out by type, records in Folio, "a story through StudioPathText is not drawn by the viewer yet"; no area type) and AI-29 "Create outlines from text" (notes: "Glyph outline extraction validated; path-text -> outlines READY; Grep 'create_outlines|text_to_outlines|outline_text' finds no general command for point/area text").
- Expected gain (revised from the wave-4 estimate, which counted AI-28 only): AI-28 PARTIAL -> BUILT (+0.25 row) and AI-29 PARTIAL -> BUILT (+0.25 row) = +1.6 pp Illustrator engine (32 rows), +0.8 pp if only one row closes; +3.1 pp once validated. 9-app pool +0.16 pp.
- Model / effort: opus / high (layout inside arbitrary closed shapes, a composite lowering that draws live text records, a Folio record member under rule 4b, four crates).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-area-type-1`)

- Type: NEW `crates/hsk-studio-type/src/area_text.rs` (compose a story inside a closed polygon/Bezier container: line bands intersected with the container, inset, first baseline, overset report), NEW `src/create_outlines.rs` (outlines for any composed layout: point, area, path), `src/lib.rs` `mod`/`pub use` lines, NEW `tests/ai_area_text.rs`, `tests/ai_create_outlines.rs`.
- Folio: `src/vector_path_text.rs` (NEW type `StudioAreaText`: story `STXT` bound to a closed container path of a vector layer, inset, first-baseline mode, auto-size STU-TYP-116 member) and `src/vector.rs` (one `serde(default, skip_serializing_if = "Vec::is_empty")` member `area_texts` on `StudioVectorRecords` (:450), validated like `path_texts`). Records are built through serde in every test (no `StudioVectorRecords {..}` literal on origin), so no other crate changes. NO edit to folio `src/lib.rs` (SMB-ID-CJK-KINSOKU-1 owns it tonight; the module is already re-exported).
- Composite: NEW `src/vector_text.rs` (live lowering of `path_texts` and `area_texts`: glyph outlines from type placed by the layout, filled with the story's paint rows), `src/lib.rs` `mod` line, an additive hunk in `src/vector.rs` where vector records expand (`expand_vectors`), NEW `tests/vector_text_lowering.rs`.
- Harness-view (slice 3, only if the window does not draw path/area text once composite does): `src/view/vector/path_text.rs`.

## Do NOT touch

- Folio `src/lib.rs`, any new Folio file, `crates/hsk-studio-package/tests/externals.rs`: SMB-ID-CJK-KINSOKU-1 owns them tonight. Name the new fields so the externals classifier VOCAB (`path`, `source`, `asset`, `story`, `link`, ...) does not match (for example `container_layer_id`, `container_id`, `text_id`); if the package suite still demands a classification row, stop, record it, and add the row only after CJK-KINSOKU-1's externals push is on origin (rebase), never in parallel.
- `hsk-studio-layout` (CJK-KINSOKU-1), `hsk-studio-pigment`, harness-view `view/mod.rs`, `shell.rs`, `view/info*`, `view/develop*` (SMB-PS-INFO-HISTOGRAM-1), every other harness-view file.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-TYP-114]`: "`text_kind` values. `point`, `area`, `path`, `frame_grid`. ... Conversion between `point` and `area` MUST preserve every character and paragraph attribute, every style binding and every override, and MUST be undoable as one history entry."
- `[STU-TYP-002]`: "`area` — text reflowed inside a closed container (rectangular frame or arbitrary closed `StudioVectorPath`), with overflow/overset state (provenance: paragraph text, frame text, area type)." and "`path` — text set along an open or closed `StudioVectorPath` (text-on-path)".
- `[STU-TYP-116]` (enumeration, members are acceptance criteria): "`off` | Container geometry is fixed; excess content is overset." / "`height_only`" / "`width_only`" / "`height_and_width`" / "`height_and_width_proportional`".
- `[STU-TYP-120]` / `[STU-TYP-121]`: path-text geometry (`start_t`, `end_t`, `spacing`, `flipped`, per-run `baseline_offset`; bounds UNKNOWN in the table) and "`rainbow` is the default and rotates each glyph to the path tangent; the other four are shipped, non-optional members." (already modelled; this lane draws them).
- `[STU-VEC-038]`: "A `StudioVectorPath`/`StudioVectorNetwork` MUST be usable as a typographic baseline (text on a path) and as a text-frame boundary (area type). The vector domain owns only the geometry; the text run, shaping, path-text options and area-type behaviour are owned by the Typography engine (14.7) ... and MUST NOT reimplement text layout. Converting text to outlines produces standard vector geometry under this sub-section."
- `[STU-TYP-127]`: "`TextEngine` owns, and is the only owner of: Unicode ..." (shaping, outline extraction: `type/src/outline.rs:1-3`).
- Menu leaves (grep "Convert to Area Text"): "`Convert` | submenu → Create Outlines, Convert to Point Text, Convert to Area Text, ..."; tool registry (grep "| Area Type | `area_type`"): "| Area Type | `area_type` | `type` | WORKSPACE > Tools > Type and tables | illustrator, indesign |".

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- Lane files `builders/lanes/hsk-studio-ai-type-path-recolor.json` (path text, outlines of path text), `hsk-studio-type-glyph-render.json` (outline extraction), `hsk-studio-type.json`.
- Code: type `src/path_text.rs:550` (layout_path_text), `:701` (path text -> outlines), `src/outline.rs:184`; folio `src/vector_path_text.rs` (record pattern, bounds), `src/vector.rs:450`, `src/story.rs:36 StoryTextKind`; composite `src/vector.rs` (`expand_vectors`), `src/text.rs` (existing text lowering), `src/lower_render_plan.rs:1827 vector_layer_render_plan`; harness-view `view/vector/path_text.rs:1-30` (layout only, not drawn).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Area type engine (type): compose a story inside a closed container (rectangle or Bezier path flattened with a declared tolerance; non-zero fill rule), per-line horizontal bands intersected with the container (widest interval per band, declared reading), inset, first baseline, auto-size members of STU-TYP-116 on rectangular containers (non-rectangular containers: `off` only, others refused named), overset report; point <-> area conversion as one pure function that keeps every attribute, binding and override. Tests: `area_text_wraps_inside_a_circle_container`, `auto_size_members_resize_rectangular_containers`, `overset_is_reported_not_dropped`, `point_area_conversion_keeps_every_attribute`, `hostile_containers_are_bounded`.
2. Folio record: `StudioAreaText` + `area_texts` member (validation: story is `text_kind = area`, container layer is a vector layer, closed path, bounds; absent = byte-identical record sets). Tests: `area_text_records_round_trip_and_refuse_hostile_bindings`, `vector_records_without_area_texts_are_byte_identical`, package `every_folio_reference_property_is_classified` (green without an externals edit, or see paths).
3. Live drawing (composite): `path_texts` and `area_texts` lower to glyph outline geometry placed by type and filled with the story's paint rows inside the vector layer's render plan; harness-view path_text.rs only if the viewer still skips them. Tests: `path_text_records_draw_live_and_follow_path_edits`, `area_text_records_draw_inside_their_container`, `editing_the_story_redraws_without_a_bake`.
4. Create Outlines (type + composite): general `create_outlines` for point, area and path text -> standard vector geometry (one closed path per glyph contour, story paints carried), returned as a proposal the host commits as one step. Tests: `create_outlines_of_point_and_area_text_matches_glyph_outlines`, `outlined_text_renders_like_the_live_text`.

</topic>

<topic id="tests" status="active" summary="Test shape, rules 4a-4e, bundled lane-check">

## Test shape

- Engine tests with closed-form geometry (band intersections, glyph advances from the checked-in test fonts), render comparisons of live text vs outlined text through composite on the same document.
- NOT_PROVEN (`NOT_PROVEN:` lines): equality with Illustrator area-type line placement, overflow handling and outline contour order (no Illustrator oracle case for type); window tools Area Type / Create Outlines (later GUI lane).

## Round-parity rules (common 4c-4e, stated for this lane)

- 4c: `lane-check.sh` now runs the round's step-0 policy first (machine paths V4-03, frozen duplicates V4-05); exit 71/72 means the round would refuse: fix before pushing; no drive-letter or user-profile literals in tests or sources; `--step0-only` for a quick scan.
- 4d: if you add or change doc examples in type, composite or folio, also run `cargo test --doc -p <crate>` through studio-env.
- 4e: type shaping tests may need `harfrust`: `.config/round-features.toml` has `hsk-studio-type = [["harfrust"]]` and `hsk-studio-composite = [["motion"]]`; a new `#![cfg(feature = ...)]` test file is proof only if such a unit enables its feature; otherwise note it for the steward.

## Bundled lane-check (D: while R40 holds C:; rule 4a + 4b)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-ai-area-type-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-ai-area-type-1 --disk d -- cargo nextest run -p hsk-studio-type -p hsk-studio-composite -p hsk-studio-folio -p hsk-studio-package --features hsk-studio-type/harfrust --no-fail-fast
```

- Rule 4a: type, composite, folio (+ harness-view and harness with `--features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch` if slice 3 touches path_text.rs). Rule 4b: a Folio record member is added -> package suite included. Loop cap: two failed runs on the same failure, then stop and report verbatim. If the lead moves builds back to C:, use `--disk ssd` with `CARGO_BUILD_JOBS=6` (os error 1455 at default jobs).

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, gaps, conflicts, deadline">

## Edges and dependencies

- None: composite -> type, nib, folio declared; folio -> accord only (unchanged). No crates.io dependency.

## UNVERIFIED (D5)

- Illustrator area-type band rule for concave containers, inset defaults, first-baseline options, outline contour order and winding.

## Spec gaps

- STU-TYP-120 bounds are UNKNOWN (table); STU-TYP-116 does not state how auto-size applies to non-rectangular containers (declared refusal); STU-TYP-002 does not state the band rule for concave shapes (declared: widest interval).

## Risks and conflicts

- Folio is shared with SMB-ID-CJK-KINSOKU-1: this lane edits only `vector.rs` and `vector_path_text.rs`; that lane owns `lib.rs`, its new file and package `externals.rs` (see paths for the classifier fallback).
- Composite carries READY-lane hunks (roto, effect stack, fill layers): new file + additive hunks only; V4-14: the lead pins the round candidate for READY lanes or lists these commits.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
