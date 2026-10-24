---
file_id: BUILDER-BRIEF-LANE-SMB-ID-CJK-KINSOKU-1
file_kind: builder_brief
updated_at: 2026-10-24T08:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-CJK-KINSOKU-1
lane_file: builders/lanes/hsk-studio-id-cjk-kinsoku-1.json
crates: [hsk-studio-layout, hsk-studio-folio (new file + src/lib.rs), hsk-studio-package (tests/externals.rs only)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip bfb5843c); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, row, expected gain, model">

## Target

- App: InDesign (CJK typography is also Illustrator provenance; not counted). Row ID-11 "Vertical / CJK typography (tate-chu-yoko, mojikumi)" is MISSING (audit `parity-indesign.json`: "IDML import only detects CJK/vertical stories and marks them unsupported_composition. Folio does not store story_orientation").
- First vertical slice: horizontal CJK composition with kinsoku line-break tables (STU-TYP-137) and mojikumi spacing tables (STU-TYP-138), stored as document resources and used by the `latin_cjk` composer. Vertical text, tate-chu-yoko, ruby/kenten/warichu stay out (later lanes).
- Expected gain: ID-11 MISSING -> PARTIAL (+0.25 row) = +0.8 pp InDesign engine (31 rows); BUILT needs vertical composition too. 9-app pool +0.08 pp.
- Model / effort: opus / high (two spec enumerations taken whole, a new Folio record set + rule 4b, composer integration into the Knuth-Plass item model, data tables from a published standard).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-cjk-kinsoku-1`)

- Layout: NEW `crates/hsk-studio-layout/src/cjk.rs` (and `src/cjk/**`: JLREQ character classes, kinsoku sets, mojikumi base-set tables, the CJK item pass), additive hunks in `src/paragraph.rs` (`build_items` :293 calls the CJK pass when the paragraph's composer binds CJK tables; `break_opportunities` :152 unchanged for non-CJK text), `src/lib.rs` (`mod`/`pub use`), `src/folio_input.rs` (resolve the record set for a story's paragraphs), NEW `tests/cjk_kinsoku.rs`, `tests/cjk_mojikumi.rs`.
- Folio: NEW `crates/hsk-studio-folio/src/cjk_composition.rs` (record set `StudioCjkCompositionRecords`, schema `hsk.studio.cjk_composition@1`: named kinsoku tables, named mojikumi tables, named composers per STU-TYP-136 binding scope/engine/tables, per-paragraph-style bindings with the kinsoku attributes), `validate_cjk_composition_records`, `cjk_composition_records_schema`; NEW `tests/cjk_composition_records.rs`; `src/lib.rs` `mod cjk_composition;` / `pub use cjk_composition::*;` + DESCRIPTOR text. This lane is the only writer of folio `lib.rs` tonight.
- Package: `crates/hsk-studio-package/tests/externals.rs` one schema line `("cjk_composition", serde_json::to_value(folio::cjk_composition_records_schema()).unwrap()),` + the classifications it demands (rule 4b). This lane is the only writer of that file tonight; push this slice first so SMB-AI-AREA-TYPE-1 can rebase onto it if its classifier fallback is needed.

## Do NOT touch

- `hsk-studio-type/**` (SMB-AI-AREA-TYPE-1 owns type tonight: character classes live in layout's `cjk.rs`, read from the text Type already passes to layout), folio `vector.rs` / `vector_path_text.rs` (AI-AREA-TYPE-1), `hsk-studio-composite`, `hsk-studio-pigment`, harness-view (SMB-PS-INFO-HISTOGRAM-1), `hsk-studio-interop-layout` (IDML import of kinsoku/mojikumi is a follow-up; its CJK refusal stays), layout files of READY SMB-ID-LIQUID-LAYOUT-1 (`liquid.rs`, `adjust_layout.rs`, `alternate.rs`: read only).

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-TYP-137]`: "A kinsoku table is a first-class named document resource with FOUR character-class fields, each a character set: `cant_begin_line_chars`, `cant_end_line_chars`, `cant_be_separated_chars`, `hanging_punctuation_chars`. A paragraph references a table by name and carries: `kinsoku_type` (`push_in_first` | `push_out_first` | `push_out_only` | `prioritize_adjustment`), `kinsoku_hang_type` (`none` | `regular` | `force`), `bunri_kinshi` (bool - adds the double period, ellipsis and double hyphen to the active set), `rensuuji` (bool - forbids breaking inside multi-digit numbers), `burasagari_type` (`none` | `standard` | `forced`) and `kinsoku_order` (`push_in` | `push_out_first` | `push_out_only`). `highlight_kinsoku` is a view preference, not a document property."
- `[STU-TYP-138]`: "A mojikumi table is a first-class named document resource carrying a `based_on` base set and an `aki_override_list`. The base sets are a SEVENTEEN-member enumeration and MUST all be shipped: `nothing`, `line_end_all_one_half_em`, ... `traditional_chinese_default`, `simplified_chinese_default`. `use_cid_mojikumi` (bool) selects glyph-CID-based rather than codepoint-based class lookup."
- `[STU-TYP-135]`: "`composition_engine`: `latin_cjk` | `world_ready` | `adornment`. `latin_cjk` is the metric composer covering Latin and CJK." and "All six combinations MUST be representable and MUST compose."
- `[STU-TYP-136]`: "A named composer binds a `composition_scope`, a `composition_engine`, and the CJK composition tables of [STU-TYP-137]-[STU-TYP-139]. ... an unresolvable name is a validation error, never a silent fallback."
- `[STU-TYP-132]`: "ONE engine MUST handle, at minimum: Latin, Cyrillic, Greek, ..." (script coverage, CJK included; grep for the full list).
- `[STU-TYP-140]`: "Composition determinism across the axes." (fixed input closure -> identical output).

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- Lane files `builders/lanes/hsk-studio-layout-typeset.json`, `hsk-studio-layout-typeset-harden.json` (composer, Knuth-Plass, Adobe paragraph composer), the IDML composition lane (find with `ls builders/lanes | grep -i idml`).
- Code: layout `src/paragraph.rs:1-3` (UAX #14 break opportunities, box/glue/penalty items), `:152 break_opportunities`, `:293 build_items`; `src/breaker.rs` (Knuth-Plass, greedy), `src/adobe_composer.rs`; `src/folio_input.rs:130-170`; interop-layout `src/idml_composition.rs:174` (`StudioCompositionEngine::LatinCjk` already mapped); folio `src/typestyle.rs` (paragraph style ids the bindings name), `src/layout_grids.rs` (record-set pattern).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Folio record set (FIRST push): kinsoku tables (four character sets, bounded), mojikumi tables (`based_on` one of the 17 members, `aki_override_list` entries of class pair + aki value in em fractions, bounded; `use_cid_mojikumi`), named composers (scope, engine, table names), bindings from paragraph style ids (with the six kinsoku attributes); unresolvable names refused (STU-TYP-136); absent = byte-identical; externals schema line. Tests: `cjk_records_validate_both_enumerations_whole`, `unresolvable_composer_or_table_name_is_refused`, `absent_cjk_records_keep_documents_byte_identical`, package `every_folio_reference_property_is_classified`.
2. Kinsoku in the item model: CJK characters become break opportunities between each pair except where the active table forbids (`cant_begin`, `cant_end`, `cant_be_separated`; `bunri_kinshi` adds the double period, ellipsis and double hyphen; `rensuuji` keeps digit runs together); `kinsoku_type` decides push-in vs push-out when a forbidden break falls at the line end; `burasagari_type` / `kinsoku_hang_type` let hanging punctuation overhang the measure. Shipped default tables (hard and soft kinsoku) from W3C "Requirements for Japanese Text Layout" (JLREQ) with attribution (W3C document licence: data, no code copied; declared). Tests: `cant_begin_chars_never_start_a_line`, `push_in_and_push_out_choose_differently`, `rensuuji_keeps_digit_runs_together`, `burasagari_hangs_the_period`.
3. Mojikumi spacing: class-pair aki as glue (natural, shrink, stretch) per base set; all 17 base sets shipped as data (JLREQ class-pair rules for the Japanese sets; the two Chinese defaults from the published CLREQ where stated, else declared Studio values), `aki_override_list` applied over the base; `use_cid_mojikumi` refused named when no CID mapping is available (declared). Tests: `all_seventeen_base_sets_are_shipped_and_distinct`, `line_end_half_em_set_compresses_closing_brackets`, `aki_override_replaces_the_base_pair`, `cjk_composition_is_deterministic`.
4. Composer wiring: a paragraph whose composer (STU-TYP-136 resolution order) binds CJK tables composes through slices 2-3 in both scopes (`single_line`, `every_line`); non-CJK paragraphs compose byte-identically to before (regression test over the existing layout fixtures). Tests: `cjk_paragraph_composes_in_both_scopes`, `latin_paragraphs_are_unchanged`.

</topic>

<topic id="tests" status="active" summary="Test shape, rules 4a-4e, bundled lane-check">

## Test shape

- Engine tests on synthetic advances (fixed em boxes), so line breaks and spacing are closed-form; one test through the real shaper with a checked-in CJK-capable test font if one exists in the corpus, else `NOT_PROVEN:` (no machine fonts, V4-03).
- NOT_PROVEN: equality with InDesign's Japanese composer and its shipped kinsoku/mojikumi sets (no InDesign oracle case for CJK); vertical composition (out of slice).

## Round-parity rules (common 4c-4e, stated for this lane)

- 4c: `lane-check.sh` runs the round's step-0 policy first; exit 71/72 = the round would refuse; no drive-letter or user-profile literals; `--step0-only` for a quick scan.
- 4d: doc examples added or changed in layout or folio -> `cargo test --doc -p <crate>` through studio-env.
- 4e: layout and folio tests are default-feature (no unit needed); a new `#![cfg(feature = ...)]` file needs a `.config/round-features.toml` unit or a lane-notes request to the steward.

## Bundled lane-check (D: while R40 holds C:; rule 4a + 4b)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-id-cjk-kinsoku-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-id-cjk-kinsoku-1 --disk d -- cargo nextest run -p hsk-studio-layout -p hsk-studio-folio -p hsk-studio-package --no-fail-fast
```

- Rule 4a: layout, folio, package. Rule 4b: new Folio record set -> package suite included. Layout dependents (components, interop-layout, harness-view, harness) get clippy from lane-check; their suites only if a pinned API changes (it should not). Loop cap: two failed runs on the same failure, then stop and report verbatim. On C: later: `--disk ssd` with `CARGO_BUILD_JOBS=6`.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, gaps, conflicts, deadline">

## Edges and dependencies

- None (layout -> folio, type declared). No crates.io dependency (tables are data in the crate). Licence: W3C JLREQ / CLREQ are published W3C documents; record the source and licence of each table in the module docs and lane notes; no copyleft source.

## UNVERIFIED (D5)

- InDesign's shipped kinsoku sets (hard/soft) and the exact aki values of its 17 mojikumi base sets; push-in/push-out tie rules; hanging-punctuation width.

## Spec gaps (Operator decides Master Spec edits)

- STU-TYP-138 names the 17 base sets but gives no class-pair aki values; STU-TYP-137 gives the table fields but no default contents. The lane ships declared values from JLREQ/CLREQ and lists the gap for the HANDOFF spec-delta list.

## Risks and conflicts

- This lane owns folio `lib.rs` and package `tests/externals.rs` tonight; SMB-AI-AREA-TYPE-1 edits folio `vector.rs` / `vector_path_text.rs` only and rebases onto your externals push if it needs a classification row.
- Layout `lib.rs` carries READY ID-LIQUID-LAYOUT-1 lines: additive only; V4-14: the lead pins the round candidate for READY lanes or lists these commits.
- Non-CJK regression is the main risk: slice 4 must prove Latin paragraphs compose byte-identically.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
