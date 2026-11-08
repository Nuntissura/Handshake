---
file_id: LANE-SMB-TYPE-INDENT-SPACING-1
file_kind: lane_brief
updated_at: 2026-11-08T06:00:13Z
wp: WP-KERNEL-STUDIO
lane: SMB-TYPE-INDENT-SPACING-1
lane_file: builders/lanes/hsk-studio-type-indent-spacing-1.json
crates: c4d06841
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Paragraph indents and spacing (STU-TYP-185) stored on the existing composition settings, composed by every layout composer, imported from IDML and editable through the existing style command, with no new composition token.">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/rulings-20261108-width-lanes.json, entry `SMB-TYPE-INDENT-SPACING-1`: read its `rulings`, `stale_facts` and `overlap_controls` in full before you code. They replace this scope's open-question defaults and correct its facts, which were written at c4d06841; origin has moved 102 commits to cede5958. Where a stale fact and the scope text below disagree, the stale_facts entry wins, and you re-read the code at your base.

- Dispatch: No in-flight lane shares a file (READ: in-flight diffs list no layout, interop-layout, folio typestyle or harness-view layout_ui path). Code now, bundled run after R60, first of the four to push.
- Tree: wt-studio-lanes/smb-r54-followups-1 (C:, target C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-team-dev/target/lanes/smb-r54-followups-1). Its old branch lane/smb-project-bin-1 at 3f2c64a5 is backed up on GitHub as lane/smb-project-bin-1 (pushed by the lead 2026-11-08). Confirm with `git ls-remote origin refs/heads/lane/smb-project-bin-1` before `git switch -c lane/smb-type-indent-spacing-1 cede5958`.
- **No cargo on C: until the lead says R60 has released C:.** A validation round holds C:, and one builder hold stalls it. Until then, write code and tests without building. Read the code with git grep, and stop at REVIEW READY with an unbuilt tip; the pre-build review needs no build. After release: cargo check and focused tests, then the bundled run.
- Landing order (HANDOFF workflow item 16): HANDOFF item 16: lead releases one push at a time. Proposed order of these four after the in-flight lanes they depend on: SMB-TYPE-INDENT-SPACING-1 (no in-flight overlap) first; SMB-PARSER-HOSTILE-1 (no in-flight overlap; safety track per blocker_decisions scoped_lanes, so it yields its slot to any feature lane ready at the same time); SMB-AI-LIVE-EFFECTS-1 after SMB-ARTBOARD-EXPORT-1; SMB-SCORE-EFFECTS-6 after SMB-DRIVE-TYPED-2.

Ruling headlines, details in the file:
- Q1 Fields on StudioCompositionSettings, or a new record named after the spec's paragraph-attribute record? -> On StudioCompositionSettings (default).
- Q2 No composition token, since these are new inputs and every existing output is proven unchanged? -> No token (default), with the stop rule kept: any changed existing row stops the lane.
- Q3 Should the paragraph_flows repair (threaded_reflow.rs:666-670 after a merge) ride in this lane if proven red? -> Yes (default): same helper, same class, one test.
- Q4 Layout placement carries no composition_version; separate lane? -> Separate later lane (default); note it against MT-30898 AC-001.
- Q5 Add indents, space at column top, last-line rule and same-style gap to the vendor oracle list? -> Yes, as D5 rows for InDesign; they do not block.
- New: the existing typed edit now has a drive row and a decoder that lists every StylePatch field. -> Extend both in this lane: the patch shape text of `layout.style.edit_style` (harness-core/src/action/commands/layout.rs:30) and `style_patch` (harness-view/src/view/commands/layout.rs:32-42), with a `composition` key decoded as Folio StudioCompositionSettings (null clears, absent keeps). One drive test edits a style's indent and reads the composed lines.
- New: IDML export now runs from the host (export_folio_longdoc), and the scope left 'does IDML export note a loss for composition records' NOT_INSPECTED. -> Read folio_export.rs first. If it writes no StudioCompositionSettings field and notes no loss, add one typed loss when any paragraph style or paragraph carries settings (all fields, not only the new ones), with one test. Writing the fields to IDML stays a next slice.
- Dispatch: blocker_decisions scoped_lanes says this lane waits; backlog ranks it inside G-TYPE-ATTRIBUTES rank 40, step 24 only, 'after SMB-LAYOUT-PERSIST-1'. -> Dispatch now.

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-TYP-185 (:8823), STU-TYP-108 (:7889, :7895), STU-TYP-105/106 (:7861, :7877), STU-TYP-201 (:9064), STU-TYP-130 (:8096), STU-TYP-187 (:8870). Read MT-30971, MT-30972, MT-30999, MT-30898. Evidence: `research/truth-check-20261105-mt-vector-type-layout-interop.json` top_missing rank 1, family T10.

The gap, in plain words: a paragraph cannot say it is indented or has space around it. Layout states the convention outright: `threaded_reflow.rs:11` "paragraphs have no space before or after", and `compose_flow` gives every line the whole slot. Folio declares the gap (`typestyle.rs:30-32`, `story.rs:14`). InDesign writes these values into every IDML package (root style of all three `oracle-corpus/adobe-session/indesign/ID-03__*.idml`: `LeftIndent`, `RightIndent`, `FirstLineIndent`, `LastLineIndent`, `SpaceBefore`, `SpaceAfter`, all 0 there), and the importer drops them behind one loss code (`idml_composition.rs:37-38`, `:363-364`). So any real document with an indent breaks its lines at the wrong measure, today, in IDML reflow and PDF export. Lists (hanging indent), tabs (positions from the left indent) and drop caps (per-line narrowing) all stand on this.

What already exists, so this is small. Reuse it; do not build beside it:
- `StudioCompositionSettings` (`typestyle.rs:206-255`) already travels per paragraph style (`style_composition`, `:356`) and per paragraph (`paragraph_composition`, `:359`), with `overlaid` (`:262`), `is_empty` (`:258`), `validate` (`:292`), unknown fields refused (`:205`), and absent fields not encoded.
- `layout/styles.rs:131-151` and `:247-258` already resolve STU-TYP-201 steps 3 and 4 and hand the result to `compose_flow` in `ParagraphStyling.composition` (`:174`); `typeset.rs:278` maps it to `ParagraphParams` (`:241`).
- Every composer takes a per-line width: `threaded_reflow.rs:381`, `breaker.rs:209`, `:334`, `adobe_composer.rs:119`, `:523`, `cjk.rs:769`. The paragraph composer already handles a width that changes by line (`adobe_composer.rs:536-539`, `:780-782`, `flat_from`), and `tests/composer_depth_diff.rs` already covers two measures.
- `Slot` holds `x_start_pt` and `width_pt` (`threaded_reflow.rs:213-214`); alignment, justification and the CJK path all read those (`:1264`, `:1276`, `:1326-1327`, `:1353-1355`, `:1373`). `Cursor.extra` already moves baselines (`:193`, `:273`), a column change already resets it (`:344`, `:349`), and Split already skips space at the top of a column (`:1103-1105`).
- All other consumers go through `compose` (`:729`): table cells, notes, running text, story tables, liquid. `pdf_export.rs:1054` and `anchors.rs:283` read `Line.x_start_pt`, so they follow.

### Items, red first

1. **Store (Folio).** Add to `StudioCompositionSettings`, under the spec names: `left_indent`, `right_indent`, `first_line_indent`, `last_line_indent`, `space_before`, `space_after` as `Option<Length>`, and `space_between_same_style` as `Option<bool>`. Extend `overlaid`. `validate`: unit must be `pt` (STU-TYP-108 :7895 stores points; follow `layoutflow.rs:865` `finite_length(.., GeometryUnit::Pt, ..)` with signed values allowed), finite, nothing else.

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `left_indent` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0 | points | UNKNOWN |
| `right_indent` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0 | points | UNKNOWN |
| `first_line_indent` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0 | points | UNKNOWN |
| `last_line_indent` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0 | points | UNKNOWN |
| `space_before` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0 | points | UNKNOWN |
| `space_after` | UNKNOWN | UNKNOWN | UNKNOWN | UNKNOWN | 0 | points | UNKNOWN |

   No clamp and no invented bound, including negatives (STU-TYP-106 :7877; a guessed number is a specification defect). `last_line_indent` must never merge into `right_indent` (:8836). Hold the table as data in Folio, in the shape of `composite/src/simulations.rs:414` `ParamRow` with every UNKNOWN as `None` and hard kept apart from soft, so MT-30972 has something to read. Name it yourself and list it as Studio-minted. Rule 4b: run the package suite; `package/tests/externals.rs:404` and `mask_tiles.rs:346` read the schema.
2. **Indents in composition (layout).** `ParagraphParams` gets the resolved values (engine default 0, STU-TYP-201 step 1). In `compose_flow`, narrow each slot where it is collected: Single `:1048-1051`, Span `:1093-1096`, Split `:1120` and `:1146-1150`. Rule: x moves by `left_indent` (plus `first_line_indent` on the paragraph's first line); width loses both of those and `right_indent`. With wrap obstacles the indents measure from the free gap `next_slot` returns (`:354`), a Studio reading, UNVERIFIED (D5). A slot the indents leave with no width is skipped like a baseline with no gap (`:361`, `:371`), still as the first line if it was; if nothing remains the result is the existing overset (`:1158-1166`) or `BudgetExceeded` (`:311`). When all indents are zero, execute no float operation on the slot, so bits cannot move.
3. **`last_line_indent`.** The last line is known only after breaking. Break; narrow `slots[L]` for the set stage (alignment and justification then read it). If the last line no longer fits, break once more with the width at index L narrowed; the second result stands, and if its last line still does not fit it is `overfull`. Bounded: one re-break, only when this indent is positive and the line overflows. A negative value widens the last line at the set stage only. Studio reading, UNVERIFIED (D5). Measure: this can raise the paragraph composer's state count (`flat_from` moves late), so run 600, 1800 and 5000 words, debug and release, with and without it, per composer, and record the numbers. If the refusal point moves materially, stop and send them.
4. **Space (layout).** Hold `space_after` as a pending value and add it with the next paragraph's `space_before` to `cursor.extra`, so one gap can be judged whole. A paragraph at `cursor.line == 0` takes none, as Split does at `:1103`. A push past the column bottom needs no code: `next_slot` resets `extra`. Anchored `pre` (`:1022-1039`) stays after the attribute space. `space_between_same_style`: when this paragraph's effective flag is true and it shares a non-null `style_ref` with the one before, the gap is zero. `styles::apply` knows both: add a computed bool to `ParagraphStyling`. Span and Split: `max(flow value, attribute value)`; the spec calls the span fields `span_column_min_space_before/after` (:8873), and code holds them at `threaded_reflow.rs:682`. Studio reading, UNVERIFIED (D5). A break sentinel replaces the cursor (`:1432-1439`) and drops the pending value; pin that.
5. **Places that assume a line owns its column or recompute y (V4-06, see sweep). Red by running each.**
   - `balance_band` rebuilds every baseline with `extra: 0.0` (`:764`), so balancing would silently erase paragraph space. Carry the gap in `Meta` (`:235`) and add it per column, except at a column's first line; keep the existing `return false` when it no longer fits (`:772`).
   - `autosize.rs:195-196` sizes width from `width_pt` plus insets, and `:185-192` checks against that room. With indents the final composition at that width would wrap. Have `Composition` (`:716`) report each line's indent total, and count it in both.
6. **Stale byte key (V4-06).** `replace_story_text` moves paragraph starts, `shift_flow_records` (`textflow.rs:151`) moves only flow records, and nothing moves `paragraph_composition`. After an edit before an overridden paragraph, `styles.rs:222-229` refuses `InvalidInput`, or the override lands on another paragraph that now has that start. Red by running through `LayoutEditor`. Add a shift for the type-style records beside `shift_flow_records`: map with `map_pos`, take the edited story, drop an override naming no paragraph of it, and keep the first of two that collapse together (`textflow.rs:9`: first touched wins). Call it at `layout_ui.rs:1067` and `layout_ui/find.rs:101`. Same class, read not run: `paragraph_flows` collapsing into a merged paragraph makes `threaded_reflow.rs:666-670` refuse. Prove it; if red, fix it in the same commit shape.
7. **IDML import (interop-layout).** In `settings_of` (`idml_composition.rs:167`) map `LeftIndent`, `RightIndent`, `FirstLineIndent`, `LastLineIndent`, `SpaceBefore`, `SpaceAfter` through `Reader::number` (`:135`, signed, finite) into `Length` pt; add them to `COMPOSITION_ATTRIBUTES` (`:71`) and update the module docs (`:37-38`). Names are verified on the three InDesign-written packages; only the value 0 is seen there, so semantics stay UNVERIFIED. `SpaceBefore="4"` in those files is on `TableStyle`, not a paragraph; do not read it. No attribute for `space_between_same_style` appears in that root style: leave it unmapped and say so. End-to-end proof: `reflow_import_typeset` over an IDML fixture with values set gives narrowed lines and gaps, and PDF export places the glyphs at the moved x (existing helpers in `interop-layout/tests/layout_flow_pdf.rs`).
8. **Typed edit.** `StyleEdit::EditStyle` (`layout_ui/styles.rs:46`, `:145`) gains a composition field on `StylePatch` (`:32`): `None` unchanged, `Some(None)` clears the style's entry, `Some(Some(settings))` sets it. Non-finite or wrong-unit values refuse through Folio (`:110`), and a character style refuses `WrongStyleScope`. It is the existing command, one undo step (`layout_ui.rs:16`), so no new id. No GUI line: the panel at `:344` stays as is. Proof in `harness/tests/viewer_layout_styles.rs`: edit, compose, undo, redo.
9. **Composition token (STU-TYP-130).** Do not add a token and do not touch `folio/src/composition_token.rs`. Reading: the clause bumps for changed semantics, and these fields are new inputs that leave every existing document's output unchanged. Layout takes no token at all today (no `COMPOSITION_VERSION` in `layout/src`; the token lane's `composing_sites` covers the Viewer text path only). Proof obligations: `tests/composer_depth_diff.rs` passes with its table untouched (never re-bless it), `d5_id_oracle.rs` and every composer row pass unchanged, including with real IDML zeros now stored, and a test pins that settings holding only zero indents give bit-identical lines to no settings. **If any existing row changes, stop and send me the case: that is a token decision, not a fix.** `mt_30898_folios_token_list_is_types_constants_in_order` stays green. Record the reading in `notes.declared_readings` and `notes.spec_proposals`.
10. **MTs.** Read MT-30971 and MT-30972 first; claim only what this proves, as partly met. Their proof commands name `-p hsk-studio-type --test mt_30971/72` and their write paths name Type only, which cannot hold this proof: do not edit MT files, report it in the notes as SMB-TYPE-AUTHORED-TOKEN-1 did for MT-30898. The `gui_obligation` of both stays open.

### V4-06 failure-class sweep

List every site with fixed or not applicable and a reason.
- **Class A, a consumer that treats a line as filling its column.** Regex `width_pt|available_pt|x_start_pt|column_x\(|inner_x\(` over `crates/hsk-studio-layout/src`, `crates/hsk-studio-interop-layout/src`, `crates/hsk-studio-harness-view/src/layout_ui*`. Known: `autosize.rs:195`, `footnotes.rs:231`, `:255`, `endnotes.rs:294`, `table.rs:457`, `:1020`, `anchors.rs:283`, `pdf_export.rs:1054`, `layout_ui.rs:702`.
- **Class B, a path that recomputes a baseline and loses `extra`.** Regex `baseline_of\(|extra: 0\.0|baseline_y_pt =`. Known: `:764`.
- **Class C, a story byte key not following an edit.** Regex `replace_story_text\(|shift_flow_records\(|paragraph_start`. Known callers: `layout_ui.rs:1065`, `find.rs:97`, `xref.rs:465` (`write_cross_reference`, takes only flow records), `find_change.rs:401`, `data_merge.rs:344`. `conditional_view` (`textflow.rs:273`) and `remap_records` (`:225`) show no caller in any `src`: state it, then decide.
- **Class D, a settings record that a path never reads.** `paragraph_params` callers, and `styling = None` paths such as `endnotes.rs:284`: an unstyled note has no indents, which is correct; say so.

### Hostile input (V4-01/02)

Values are finite in Folio, but sums are not: `x0 + indents` and accumulated `extra` over up to 65,536 paragraphs can reach infinity, and `+inf + -inf` is NaN, which passes the `baseline + descent > bottom` test at `:339` and yields a NaN slot. After every addition check finiteness and refuse `InvalidGeometry`. Tests: `f64::MAX` in each field through a struct literal, alternating `+MAX` and `-MAX` space, IDML attributes `1e308`, `-1e308`, `NaN`, `inf`, `1e999` (typed loss `composition_value_invalid`). An indent wider than the frame on every composer gives a typed answer, no abort and no hang. No new allocation beyond existing per-paragraph vectors; `MAX_PARAGRAPH_COMPOSITION` (`typestyle.rs:189`) is unchanged. Vendor parity UNVERIFIED (D5); do not re-bless any parity row.

### Out of scope

Tabs (STU-TYP-197), lists (198), drop caps (196), keep options (186), rules, shading, borders; the character attribute record and OpenType features on `StoryRange` (155-159, 170): `text.rs:573` keeps `features: &[]`; ruby, kenten, warichu, vertical; `redefine_style_from_selection` and override reporting (202); document text default (STU-TYP-201 step 2); variable binding (204); `Viewer` live text (`view/text.rs`, `composite` vector text), which does not compose through layout; IDML export of these fields (`folio_export.rs` and `idml_encode.rs` hold no composition record today; whether a loss is noted is NOT_INSPECTED, report it); a local-override command or any new controls Registry or RUN_COMMANDS row (next slice, after the command contract); the mixed-leading question in `baseline_of` (`:273` multiplies earlier lines by the current paragraph's height; read, not run: report it, do not fix it, and write the space tests at one leading); any GUI; no egui, eframe, winit, wgpu, naga or WGSL line; no GPU code; no `Cargo.toml` change; no Master Spec edit.

## Host integration

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: layout returns typed refusals, overset and counters, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none: the existing EditStyle command gains a field, no new control", "attribution": "unchanged: composition commits nothing; the LayoutEditor edit stays in its caller-held undo history; no default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

## Owned paths (lane worktree `wt-studio-lanes/smb-r54-followups-1`, branch `lane/smb-type-indent-spacing-1`, base c4d06841 or the origin tip at dispatch)

- folio: `src/typestyle.rs`, DESCRIPTOR, `tests/type_style_composition.rs`.
- layout: `src/threaded_reflow.rs`, `src/typeset.rs`, `src/styles.rs`, `src/autosize.rs`, `src/textflow.rs`, `lib.rs` exports and DESCRIPTOR, `tests/`.
- interop-layout: `src/idml_composition.rs`, `src/descriptor.rs`, `tests/`.
- harness-view: `src/layout_ui.rs` (the one call), `src/layout_ui/find.rs` (the one call), `src/layout_ui/styles.rs`; no GUI line.
- harness: `tests/viewer_layout_styles.rs`, `tests/layout_ui_fixture/` only if needed.
- package: externals test only (4b).

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-r54-followups-1 --disk ssd`; log names carry the lane id and sha): step 0; clippy over folio, layout, interop-layout, harness-view and their direct dependents; nextest WHOLE suites of folio, layout, interop-layout, package (4b), harness-view and harness with `--features hsk-studio-harness-view/viewer,hsk-studio-harness/viewer`; doctests if added.
- Push when green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-r54-followups-1 --disk ssd --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after the bundled run: commits touching none of your crates: rebase, `--per-sha`, your new tests only, push; touching one of your crates: that crate's suite once, then the same. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

## Not verified by this scoping

- No code was built or run; every claim about current behaviour comes from reading c4d06841. The red tests prove them.
- All InDesign behaviour in this lane is UNVERIFIED (D5). The corpus shows attribute names with value 0 only.
- The mixed-leading reading of `baseline_of` (`threaded_reflow.rs:273`) is UNVERIFIED.
- The effect of `last_line_indent` on composer budgets is not measured.
- Whether IDML export notes a loss for composition records is NOT_INSPECTED.
- The reuse tree's lane status and built crates come from the 2026-11-07 inspection file; its cleanliness and tip were checked live.


</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: layout returns typed refusals, overset and counters; no new Observe code; emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: composition commits nothing; the LayoutEditor style edit stays in its caller-held undo history; no default_actor() or literal actor added", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r54-followups-1`, branch `lane/smb-type-indent-spacing-1`, base cede5958)

['crates/hsk-studio-folio/src/typestyle.rs', 'crates/hsk-studio-folio/tests/type_style_composition.rs', 'crates/hsk-studio-layout/src/threaded_reflow.rs', 'crates/hsk-studio-layout/src/typeset.rs', 'crates/hsk-studio-layout/src/styles.rs', 'crates/hsk-studio-layout/src/autosize.rs', 'crates/hsk-studio-layout/src/textflow.rs', 'crates/hsk-studio-layout/src/lib.rs (exports and DESCRIPTOR)', 'crates/hsk-studio-layout/tests/ (new indent and spacing test file)', 'crates/hsk-studio-interop-layout/src/idml_composition.rs', 'crates/hsk-studio-interop-layout/src/descriptor.rs', 'crates/hsk-studio-interop-layout/tests/idml_composition.rs', 'crates/hsk-studio-harness-view/src/layout_ui.rs (the one call at :1067)', 'crates/hsk-studio-harness-view/src/layout_ui/find.rs (the one call at :101)', 'crates/hsk-studio-harness-view/src/layout_ui/styles.rs', 'crates/hsk-studio-harness/tests/viewer_layout_styles.rs', 'crates/hsk-studio-package/tests/externals.rs (only if rule 4b requires it)']

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-r54-followups-1`): step 0; ['hsk-studio-folio', 'hsk-studio-layout', 'hsk-studio-interop-layout', 'hsk-studio-package', 'hsk-studio-harness-view [viewer]', 'hsk-studio-harness [viewer]']; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-r54-followups-1 --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
