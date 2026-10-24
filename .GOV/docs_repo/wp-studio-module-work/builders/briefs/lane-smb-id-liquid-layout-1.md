---
file_id: BUILDER-BRIEF-LANE-SMB-ID-LIQUID-LAYOUT-1
file_kind: builder_brief
updated_at: 2026-10-24T06:10:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-LIQUID-LAYOUT-1
lane_file: builders/lanes/hsk-studio-id-liquid-layout-1.json
crates: [hsk-studio-folio, hsk-studio-layout, hsk-studio-package (tests/externals.rs only)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip e3fc44b5); no product code written
sequencing: "SMB-FIG-EDIT-2 pushes its Folio/package slice first; this lane edits folio src/lib.rs and package tests/externals.rs only after that commit is on origin"
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="selection" status="active" summary="Why this lane, compared with the candidates">

## Selection (parity gain per lane, rollup formula: MISSING -> BUILT_NOT_VALIDATED = +0.5 row, PARTIAL -> BUILT = +0.25 row)

- Chosen: InDesign ID-23 "Liquid / adjust / alternate layouts", MISSING, rank 2 of the InDesign gaps (parity-20261023-adobe-print.json; audit `parity-indesign.json` ID-23: "No layout-rule engine, no alternate-layout records"). Fully spec-backed (STU-LAY-009/010/011/111/112, STU-LAY-019). Expected: +1.6 pp InDesign engine (31 rows), +3.2 pp validated; 9-app pool +0.16 pp. Falls to +0.8 pp (PARTIAL) if alternate layouts ship without linked-story update (slice 4).
- Illustrator AI-28 area type (STU-TYP-114): the row is PARTIAL with three named missing parts (area type, a live path-text layer, general Create Outlines). Area type alone leaves it PARTIAL: +0 pp. All three together: PARTIAL -> BUILT = +0.25 x 1/32 = +0.8 pp Illustrator. Lower than ID-23 and needs three capabilities.
- InDesign ID-11 vertical/CJK, first slice (kinsoku STU-TYP-137 + mojikumi STU-TYP-138 tables in the composer): MISSING -> PARTIAL = +0.8 pp InDesign; BUILT needs vertical composition and tate-chu-yoko too (much larger). Lower per lane.
- Lightroom catalog (LR-27, PARTIAL, deficit 0.75; STU-IO-189, STU-ASSET-019): PARTIAL -> BUILT = +0.7 pp per row, but it needs a real catalog store (large; SurrealDB authority questions). Lower per lane.
- Photoshop PS-30 histogram/info/sampler (MISSING, STU-RAS-049/163): +1.4 pp if judged BUILT, but the clause is a "feedback surface" (GUI); an engine-only lane risks PARTIAL (+0.7 pp). Rejected tonight because the GUI half needs `view/mod.rs` (FIG-EDIT-1's).
- No higher-ranked engine row has both a clause and a free crate tonight (PS-28 done; PR-10 done; LR-21 done; PS-31 actions is larger; PS-33 is outside current scope per STU-RAS-045).

</topic>

<topic id="target" status="active" summary="App, row, gain, model">

## Target

- App: InDesign, row ID-23 (also Affinity Publisher layout rows where they cite liquid/alternate layouts: NOT_INSPECTED).
- Model / effort: opus / high (spec enumeration table STU-LAY-111, the adjust-layout contract STU-LAY-112 with a bleed exception, a new Folio record set + rule 4b, layout recomposition through the existing reflow).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-liquid-layout-1`)

- Folio: NEW `crates/hsk-studio-folio/src/layout_rules.rs` (record set `StudioLayoutRuleRecords`, schema `hsk.studio.layout_rules@1`: per-page liquid rule, per-object pins and resize constraints for `object_based`, alternate layouts as sections `alternate_layout` + `alternate_layout_length`, linked-story records), `validate_layout_rule_records`, `layout_rule_records_schema`; NEW `tests/layout_rule_records.rs`; `src/lib.rs` `mod layout_rules;` / `pub use layout_rules::*;` and the DESCRIPTOR text for the new set. Pattern: `src/layout_grids.rs` ("Like the other layout record sets it is stored beside the document (`StudioDocument` gains no fields)"). No field on `StudioPageSpread`, page or frame structs (struct literals across crates).
- Layout: NEW `crates/hsk-studio-layout/src/liquid.rs`, `src/adjust_layout.rs`, `src/alternate.rs` (+ `src/lib.rs` `mod`/`pub use` lines), NEW `tests/liquid_layout.rs`, `tests/adjust_layout.rs`, `tests/alternate_layout.rs`. Reuse `threaded_reflow.rs`, `geometry.rs`, `grids.rs` (liquid guides are `GuideKind::Liquid` in folio `layout_grids.rs:89`), `folio_input.rs`; call, do not fork.
- Package: `crates/hsk-studio-package/tests/externals.rs` one schema line `("layout_rules", serde_json::to_value(folio::layout_rule_records_schema()).unwrap()),` plus any classification it demands (rule 4b).

## Do NOT touch

- SMB-FIG-EDIT-1 files (harness-view `view/design*`, `view/mod.rs`, `shell.rs`, `file_dialog.rs`, `vector.rs`, RUN-UI.md) and SMB-LR-HEALING-2 files (harness-view `view/develop*`, interop-lrcat, develop): this lane has no harness-view hunk at all (window surface = a later lane).
- Existing Folio files (`layout.rs`, `layout_grids.rs`, `parentpages.rs` ...: read only), `hsk-studio-interop-layout/**` (IDML import of liquid rules is a follow-up), session, root files.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-LAY-009]`: "Studio MUST implement responsive layout adaptation when page size, orientation, margins or bleed change. Two mechanisms coexist: per-page liquid rules applied continuously as geometry changes, and the on-demand adjust-layout operation."
- `[STU-LAY-111]` (enumeration table, one microtask, members are acceptance criteria): "`liquid.off` | No liquid adaptation; objects keep absolute geometry." / "`liquid.scale` | All page content scales proportionally, preserving relative positions." / "`liquid.recenter` | Content keeps original size and re-centres on the resized page." / "`liquid.object_based` | Per-object pins to page edges plus resize constraints give mixed fixed/relative behaviour." / "`liquid.guide_based` | Liquid guides slice the page; objects a guide crosses stretch while text reflows and images resize without distortion." / "`liquid.use_parent` | The page inherits whatever liquid rule its parent page defines." / "`liquid.preserve_existing` | Leave the page's existing rule untouched. Used when a rule is applied across a range and some pages must be skipped." Guides: "The captured guide object carries a `guide_type` enumeration `{guide_type.ruler, guide_type.liquid}` ... plus a `guide_zone` (the snap radius)."
- `[STU-LAY-112]`: "The captured adjust-layout operation takes a PROPERTY BAG naming only the values that change, and an optional page scope. Its permissible keys are exactly: `width`, `height`, `bleedInside`, `bleedTop`, `bleedOutside`, `bleedBottom`, `leftMargin`, `topMargin`, `rightMargin`, `bottomMargin`. Studio MUST accept a partial bag ... and MUST accept each value either as a number in `pt` or as a measurement string carrying any suffix from [STU-LAY-101]. Adjust layout is available at document scope, at a page collection, and at a single page. ... when the operation is scoped to individual pages rather than the whole document, BLEED changes have no effect."
- `[STU-LAY-010]`: "Adjust layout MUST recompute object positions and sizes when page size, margins or bleed change, with options to adjust font size (with a minimum and maximum limit), include locked content and move ruler guides. Adjust layout is a discrete, undoable command distinct from the continuous liquid rules. The font-size adjustment's minimum and maximum bounds, unit and default were NOT recovered by the capture and are a declared SPEC GAP; the three booleans ARE required and MUST be exposed."
- `[STU-LAY-011]`: "Alternate layouts MUST be supported: multiple named page-size/orientation variants coexist inside one `StudioDocument`, displayed side by side in the page-navigation surface, with stories linked ([STU-LAY-019]) back to the source layout so edits can propagate." ... "an alternate layout is stored AS A SECTION, not as a separate document. Studio MUST use that model. ... an alternate-layout name MUST NOT be blank, and it MUST be unique and MUST NOT end with `*` or `:`." Flex/container mode: "layout consumes `StudioAutoLayout` and MUST NOT fork a parallel container model" (out of this lane; declared).
- `[STU-LAY-019]`: "Linked stories (place-and-link a child copy of a story) MUST show update state and support auto-update or warn-on-parent-change ... The captured linked-story option object and linked-page-item option object are separate; Studio MUST carry both".

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- Lane files `builders/lanes/hsk-studio-layout-typeset.json`, `hsk-studio-layout-typeset-harden.json`, and the SMB-ID-PREPRESS lane file (layout grids/guides records; find it with `ls builders/lanes | grep -i prepress`).
- Code: folio `src/layout.rs:15-25` (declared not-stored page fields incl. `applied_alternate_layout`, `layout_rule`), `src/layout_grids.rs` (record-set pattern, `GuideKind::Liquid`), `src/parentpages.rs` (parent pages: `use_parent` resolution); layout `src/threaded_reflow.rs:378 reflow_flow`, `src/geometry.rs`, `src/grids.rs`, `src/parent_adapter.rs`, `src/folio_input.rs:130-139` (unsupported compositions).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Folio record set: liquid rule per page (seven members, `use_parent` resolved against parent pages, `preserve_existing` only as an apply-range directive, never stored as a page's effective rule: declared reading), object pins (left/top/right/bottom) and resize constraints (width/height fixed|relative) for `object_based`, alternate-layout sections (name rules: non-blank, unique, not ending `*` or `:`; `alternate_layout_length` = page count, validated), linked-story and linked-page-item option objects (separate), typed refusals, Budget nodes, absent = byte-identical documents; externals schema line. Tests: `liquid_rules_validate_seven_members`, `alternate_layout_names_follow_the_captured_rules`, `linked_story_and_page_item_options_are_separate`, `absent_layout_rules_keep_documents_byte_identical`, package `every_folio_reference_property_is_classified`.
2. Liquid engine (`apply_liquid(document, records, page, new_page_geometry) -> proposal`): off, scale (uniform about the page origin, text frames scaled and reflowed), recenter, object_based (pins + constraints), guide_based (objects crossed by a liquid guide stretch along that axis, text reflows through `threaded_reflow`, images resize without distortion: fit kept), use_parent; one proposal per resize, validated by Folio. Tests: `scale_keeps_relative_positions`, `recenter_keeps_size`, `object_pins_hold_edges`, `guide_crossed_frames_stretch_and_text_reflows`, `use_parent_follows_the_parent_rule`.
3. Adjust layout (STU-LAY-010/112): partial property bag, `pt` numbers or measurement strings (STU-LAY-101 suffixes through the existing unit parser), scopes document/page collection/page, BLEED ignored for page scope (named in the receipt), the three booleans (`adjust_font_size`, `include_locked`, `move_ruler_guides`); font-size min/max are caller-supplied with no default (SPEC GAP declared; omitted bounds with `adjust_font_size` = typed refusal). Tests: `partial_bag_changes_only_named_values`, `measurement_strings_parse_every_suffix`, `page_scope_ignores_bleed`, `locked_content_moves_only_when_included`, `font_size_adjust_needs_explicit_bounds`.
4. Alternate layouts: `create_alternate_layout(source section, name, page size/orientation, liquid rule)` duplicates the section's pages as a new section, applies the liquid rule, and links each copied story back with the linked-story options (update state `up_to_date | out_of_date`, auto-update or warn); `update_linked_stories` propagates source edits (or reports out-of-date). Tests: `alternate_layout_is_a_section_with_its_page_count`, `linked_story_goes_out_of_date_and_updates`, `warn_on_parent_change_does_not_overwrite`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Engine tests with exact geometry in `pt` (closed-form scale/recenter/pin results) and reflow results compared with the existing composer on the same story; hostile record sets bounded (V4-01/02).
- NOT_PROVEN (`NOT_PROVEN:` lines): InDesign liquid behaviour numerics (oracle has no liquid case: `parity-indesign.json` ID-23 "NO_ORACLE_CASE"); page-navigation side-by-side display and the window commands (no harness-view hunk in this lane); IDML import/export of rules.

## Bundled lane-check (C: SSD, rule 4a + 4b)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-id-liquid-layout-1/handshake-studio-assets" && CARGO_BUILD_JOBS=6 bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-id-liquid-layout-1 --disk ssd -- cargo nextest run -p hsk-studio-folio -p hsk-studio-layout -p hsk-studio-package --no-fail-fast
```

- Infra (FIG-EDIT-1 finding): C: builds hit os error 1455 (paging file too small) at default parallelism; prefix your own lane-check and cargo runs on C: with `CARGO_BUILD_JOBS=6` (builders only, never rounds).
- Rule 4a: folio, layout, package. Rule 4b: new Folio record set -> package suite included. Layout's public API is additive; its dependents (components, interop-layout, harness-view, harness) do not pin the new names, so their suites are not required (state it); clippy covers them. Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, gaps, conflicts, deadline">

## Edges and dependencies

- None: layout -> folio is declared; package tests already depend on folio. No crates.io dependency.

## UNVERIFIED (D5)

- InDesign liquid-rule numerics (scale anchor, guide-based stretch distribution, image fit under stretch), adjust-layout font scaling, alternate-layout default naming and linked-story update UI.

## Spec gaps (Operator decides Master Spec edits)

- STU-LAY-010 font-size adjustment bounds, unit and default (declared SPEC GAP in the clause); STU-LAY-111 does not state the anchor of `liquid.scale` or how guide-based stretch splits between several guides; STU-LAY-011's flex/container mode depends on `StudioAutoLayout` parameters "not recovered on this surface" (out of this lane).

## Risks and conflicts

- Folio `lib.rs` and package `tests/externals.rs` carry hunks from READY lanes (SOFT-PROOF-2, MULTICAM-1/2, fill layers, roto): additive lines only; V4-14: the lead pins their round candidate or lists these commits.
- SEQUENCING with SMB-FIG-EDIT-2 (lead added it after this brief): both lanes edit folio `src/lib.rs` and package `tests/externals.rs`. FIG-EDIT-2 pushes its Folio/package slice FIRST; until that sha is on origin this lane may write its NEW files (`folio/src/layout_rules.rs`, layout `liquid.rs` ...) but must not edit those two shared files; then it fetches, rebases onto that sha and adds its lines. Dispatching this lane after FIG-EDIT-2's first push is the simplest way to keep the rule. Layout itself is touched by no other lane.
- Text reflow under stretch can overflow: report overset per frame (layout output), never silently drop text.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
