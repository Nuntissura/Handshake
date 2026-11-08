---
file_id: LANE-SMB-AI-LIVE-EFFECTS-1
file_kind: lane_brief
updated_at: 2026-11-08T06:00:13Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-LIVE-EFFECTS-1
lane_file: builders/lanes/hsk-studio-ai-live-effects-1.json
crates: c4d06841
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The seven evaluating vector live effects are stored in the document, drawn by render and export, edited by typed commands and bakeable, or refused typed">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/rulings-20261108-width-lanes.json, entry `SMB-AI-LIVE-EFFECTS-1`: read its `rulings`, `stale_facts` and `overlap_controls` in full before you code. They replace this scope's open-question defaults and correct its facts, which were written at c4d06841; origin has moved 102 commits to cede5958. Where a stale fact and the scope text below disagree, the stale_facts entry wins, and you re-read the code at your base.

- Dispatch: Code now on C:; bundled run after R60 releases C:; push after SMB-ARTBOARD-EXPORT-1 lands. Items 1-4 and 6 (nib, folio vector records, composite, Viewer live_filters.rs) touch no in-flight file except folio/src/lib.rs (module line, DESCRIPTOR); item 5's exporter loss lines and the vector/interop.rs, pdf_export.rs sites wait for ARTBOARD-EXPORT-1 to land, then re-read their lines.
- Tree: wt-studio-lanes/smb-pr-nudge-1 (C:, target C:/HandshakeBuild/WP-KERNEL-STUDIO/studio-team-dev/target/lanes/smb-pr-nudge-1). Its old branch lane/smb-doc-export-1 at 6cf52d85 is backed up on GitHub as lane/smb-doc-export-1 (pushed by the lead 2026-11-08). Confirm with `git ls-remote origin refs/heads/lane/smb-doc-export-1` before `git switch -c lane/smb-ai-live-effects-1 cede5958`.
- **No cargo on C: until the lead says R60 has released C:.** A validation round holds C:, and one builder hold stalls it. Until then, write code and tests without building. Read the code with git grep, and stop at REVIEW READY with an unbuilt tip; the pre-build review needs no build. After release: cargo check and focused tests, then the bundled run.
- Landing order (HANDOFF workflow item 16): HANDOFF item 16: lead releases one push at a time. Proposed order of these four after the in-flight lanes they depend on: SMB-TYPE-INDENT-SPACING-1 (no in-flight overlap) first; SMB-PARSER-HOSTILE-1 (no in-flight overlap; safety track per blocker_decisions scoped_lanes, so it yields its slot to any feature lane ready at the same time); SMB-AI-LIVE-EFFECTS-1 after SMB-ARTBOARD-EXPORT-1; SMB-SCORE-EFFECTS-6 after SMB-DRIVE-TYPED-2.

Ruling headlines, details in the file:
- Q1 STU-VEC-023 says effects are carried in the shared StudioEffectStack and STU-VEC-022 forbids a vector-only reimplementation; the slice adds a separate stack  -> Accept the object-scope stack in StudioVectorRecords as the first step, AND make each entry the spec's StudioLiveFilter shape (STU-FX-002 field names, see Q3), so that unification later moves records rather than reshaping them. Record an SD item: STU-VEC-023 object scope partly met; holder placement open.
- Q2 STU-VEC-157 gives default 1.0 for scale (percent) and move; copied literally a fresh transform makes 1 percent copies 1 unit away; nib's identity defaults ar -> Default stands: nib's 100 percent and 0, labelled a Studio reading in the descriptor with the spec cell quoted verbatim beside it; commands store resolved entries; SD item; MT-30558 stays open.
- Q3 Type and member names: StudioLiveFilter from the spec; StudioLiveFilterStack and live_filters composed. Allowed? -> CHANGED: the entry is StudioLiveFilter with the effect key stored as `filter_kind` (not `effect_id`), plus `instance_id`, `enabled`, `version`, typed `params`. Holder StudioLiveFilterStack and member live_filters are allowed, listed as Studio-minted. Values of filter_kind are nib's effect ids; the converter maps filter_kind to EffectRecord::from_parts(effect_id, ..).
- Q4 HOST rule 3 asks a Registry id for every new command; SMB-HARNESS-DRIVE-1 was settling naming, O5 open. -> CHANGED: the command corpus has landed; add typed rows vector.live_filter_apply, _update, _reorder, _toggle, _remove, _bake, _read to harness-core/src/action/commands/vector.rs and their decoders to harness-view/src/view/commands/vector.rs, calling the new Viewer methods; one test through the drive path. control_descriptors stays none (no panel).
- Q5 direction.dispatch pauses harness-view surfaces; item 6 adds window-free Viewer logic only. -> Allowed. Keep item 6 and the Viewer half of item 5; no egui line.
- Q6 In a px document nib's pt reads as px and per_in as per 72 px; non-px documents refuse. -> Accept as stated; same gate as strokes and envelopes (REFUSE_GEOMETRY_UNIT).
- Q7 Is the silent envelope drop on direct fig, SVG and EPS export a finding? -> Fix the loss line here (envelope and live filter) and report it as found, but only after SMB-ARTBOARD-EXPORT-1 lands, re-reading the line numbers.
- Dispatch: blocker_decisions_20261107.scoped_lanes says this lane 'closes few steps and waits'; backlog defers it (rank 43, step 73 only). -> Dispatch now on C:, push after ARTBOARD-EXPORT-1.

## Target

HANDOFF next-work item 7, second bullet. Evidence: `S/research/truth-check-20261105-mt-vector-type-layout-interop.json` `top_missing` rank 2 and family V07. A designer can roughen, twist or zig-zag a path only in a Rust test: `hsk_studio_nib::live_effect` has no caller outside nib (`git grep 'live_effect\|EffectRecord' c4d06841` outside `hsk-studio-nib/` finds none; the composite and folio hits for the words 'live effect' are comments and descriptors). A document cannot hold an effect: `StudioAppearanceRow` is Fill and Stroke only (`folio/src/paint.rs:204`) and `StudioVectorRecords` has no effect member (`folio/src/vector.rs:456`). So 7 PASS_V1 MTs (MT-30545..30550, MT-30559) prove pure functions nobody calls. This lane makes the seven evaluating effects real on a vector object: stored in the document, drawn by render and export, editable by typed commands, bakeable. All file:line below are at c4d06841; paths are under `handshake-studio-assets/crates/hsk-studio-`.

The closest pattern is `StudioEnvelope`: Folio stores (`folio/src/vector_tools.rs:397`, validation `:605-634`), composite evaluates through nib (`composite/src/vector.rs:1479`, `:1608`), Viewer commands in one egui-free file (`harness-view/src/view/vector/envelope.rs:63`, `:124`, `:140`), tests `composite/tests/vector_tools.rs:129` and `harness/tests/viewer_illustrator_tools.rs:167`. Follow it. Edges exist already: `composite/Cargo.toml:18` has nib; folio has no nib edge and gets none (`folio/Cargo.toml`), so Folio validates shape and nib validates parameters.

Clauses to read (spec module `14-studio-creative-suite.md`, v02.229): STU-VEC-022 :4084, STU-VEC-023 :4091, STU-VEC-054 :4424, STU-VEC-100 :2993, STU-VEC-101 :3028, STU-VEC-154 :4176, STU-VEC-155 :4201, STU-VEC-157 :4345, STU-VEC-158 :4374, STU-FX-002 :19395, STU-FX-003 :19397, STU-FX-007 :19413, STU-FX-011 :19421, STU-FX-012c :19433, STU-FX-027 :19720, Distort and Transform table :19737-19747, STU-CON-007 :22896.

1. **Red first, compiling.** New `composite/tests/vector_live_filters.rs`, records written as JSON so the red commit compiles and fails at run time (`deny_unknown_fields`, `folio/src/vector.rs:454`, refuses the new member until it exists). For each of roughen, zig zag, pucker and bloat, twist, tweak, free distort, step repeat: a one-effect document through `prepare_vectors` equals a folio-free spec built from `nib::live_effect::evaluate` plus `prepare_vector_layers` (the method of `composite/tests/vector_tools.rs:88`, `:129`); the same through `expand_vectors`. Prove red by running.

2. **Folio record.** Add one optional member to `StudioVectorRecords`, absent when empty, so existing record sets stay byte-identical (`vector.rs:483-484` shows the form; add to `new`, `:531`). Module beside `vector_tools.rs`, with `#![deny(clippy::arithmetic_side_effects)]` as `width_profile.rs` has. Proposed shape, names from STU-FX-027 and STU-FX-002 (`StudioLiveFilter`; no such type exists in code today, `git grep -E '(struct|enum|type) StudioLiveFilter'` is empty) and from STU-VEC-154 and `StudioEffect` (`raster.rs:716-717`):
   - member `live_filters: Vec<StudioLiveFilterStack>`; stack `{layer_id, filters: Vec<StudioLiveFilter>}`, at most one stack per layer, list order = application order (STU-FX-003).
   - `StudioLiveFilter {instance_id, enabled, effect_id, version, params}`; `params` an ordered list of `{key, type, value}` with `type` exactly `Int | Real | Bool | String` (STU-VEC-154 :4196-4199: typed map, untyped write rejected). Use a serde tagged enum so a wrong type cannot deserialize.
   - Validation inside `validate_tools` (`vector_tools.rs:511`): layer exists and is a Vector payload (`PayloadMismatch`, as `:609`; group target ring and row scope are out of this slice and refuse here), stack and instance ids unique (`DuplicateIdentity`), finite reals (`InvalidField`), key grammar `key()` as smart filters use, string length and control characters as `text_ok` `:414`, `effect_id` non-empty and bounded, every count before use. Unknown `effect_id` or parameter is NOT Folio's refusal: Folio cannot see nib, and a later law set must stay loadable. It refuses at composite.
   - Budgets: member count under `TOOLS_MAX_ENTRIES` (`:44`, loop `:518`, charged to `cx.nodes`), filters per stack and params per filter as named constants. STU-RAS-156 :1748 forbids a per-kind cap below 10 and STU-FX-006b :19407 allows only a performance budget, so the stack cap is a declared engine limit: 64 proposed. Params: the largest table is free distort at 16 rows (`live_effect.rs:449`); 32 proposed.
   - Tests: byte identity without the member, wire round trip with exact reals, each refusal. Add the member's reference-looking properties to `package/tests/externals.rs` (table at `:528` onward): common rule 4b requires the whole package suite.

3. **Transform table before it becomes wire format.** `live_effect.rs:18-19` and the lane notes of SMB-AI-DISTORT-EFFECTS-1 say every step-repeat row is Studio-declared because no recovered record exists. That is wrong: STU-VEC-157 :4345-4372 says the transform-each command and the live transform effect share one parameter set, and gives it. Today `STEP_REPEAT` (`live_effect.rs:462-474`) has all bounds unknown, and stores `random` where the spec names `randomize`. Once documents store these keys, a rename is a migration, so fix it first. Rows to copy exactly:

   | Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
   |---|---|---|---|---|---|---|---|
   | `scale_horizontal`, `scale_vertical` | -4000 | 4000 | 0 | 200 | 1.0 (see open question) | `percent` | 2 |
   | `move_horizontal`, `move_vertical` | -4000 | 4000 | -100 | 100 | 1.0 (see open question) | `document_unit` | 4 |
   | `rotate` | unknown | unknown | unknown | unknown | 0 | `deg` | unknown |
   | `copies` | unknown | unknown | unknown | unknown | 0 | `count` | 0 |

   Plus `reflect_x`, `reflect_y` default false, key `randomize` default false, provenance `Recovered("STU-VEC-157")`. `seed` stays Studio-declared (STU-FX-011). Not in this slice, and said so in the descriptor: `scale_mode`, the `*_absolute` rows, `transform_objects`, `transform_patterns`, `scale_strokes_and_effects`, `scale_corners`; MT-30558 stays open. The other tables (STU-VEC-155, STU-VEC-158) are already in `live_effect.rs:409-505`: do not retype them. Update `vector_live_effects.rs` pins and `nib/src/descriptor.rs:10`, and name each changed pin in your notes.

4. **Composite evaluation.** New module `composite/src/vector_live_filters.rs`, called from `FolioCx::own_geometry` (`vector.rs:1422`), which both `prepare_vectors` (`:2077`) and `expand_vectors` (`:2470`) use, so render, SVG (`harness-view/src/view/vector/interop.rs:433`), PDF (`pdf_export.rs:335`) and separations (`separations.rs:301`) all get it from one place. No GPU or render plan change: a vector layer is drawn on the host into a coverage plane and lowered through existing pigment routes (`vector.rs:2-4`), so render-gpu needs nothing.
   - One converter: `StudioLiveFilter` to `EffectRecord` through `EffectRecord::from_parts` (`live_effect.rs:617`), public, because item 6 uses it too. No second reader of the parameter map.
   - Early return `:1449` also tests for an enabled filter. A stack with only disabled entries takes the unchanged path and must render byte-identically to no stack.
   - Insert before the loop at `:1459`: evaluate enabled filters in list order over the layer's whole path list, each result the next one's sources (`evaluate`, `live_effect.rs:762`; bounds-relative laws read the union of the current sources). The results then feed the existing fill and stroke paths, so strokes stroke the effected path, brushes and markers follow, and the envelope (`:1479`) wraps the effected geometry. Order law: filters, then envelope; Studio reading, parity UNVERIFIED (D5).
   - Step repeat returns more paths than it took (`laws.rs:481`, sources then each copy). `paths`, `networks` and `profiles` are zipped one to one at `:1459`, so width profiles must be cycled by source index, output `k` taking profile `k % n`. Test it: a naive zip silently drops copies.
   - `unit_px` false refuses `REFUSE_GEOMETRY_UNIT`, as `wrap` does (`:1611`). In a px document nib's pt reads as px, and `per_in` as per 72 px (see open question).
   - `FolioCx` has no cancel token (`:1231`). Add one at both constructors (`:2042`, `:2447`). Cancel refuses and returns nothing.
   - Refusal per layer through the existing `Result<_, &'static str>` to `push_refusal` (`:2102`), never a silent draw of the source. Map every `LiveEffectError` (`live_effect.rs:113`, 23 variants) to one `&'static str` in an exhaustive match with no wildcard, so a new variant fails compilation. `convert_to_shape` and `object_mosaic` keep their typed refusals (`:765-766`).

   | Case | Result |
   |---|---|
   | Unknown effect id, other version, unknown key, wrong type, out of hard range | layer refused with nib's code mapped |
   | `parameter_unset` (a table default is unknown) | refused, Studio never invents a default |
   | Network path on the layer (`:1432`) | `vector_effect_network`, next slice |
   | Path text or area text bound to the layer (`:2080`, `vector_text.rs:303`) | `vector_effect_text_bound`, text would follow the raw path |
   | Work budget spent | `vector_effect_work_limit` |

5. **Failure-class sweep (V4-06): every reader that takes vector geometry or records and would silently drop an effect.** List each site, fixed or not applicable with a reason.
   - Raw geometry in composite: boolean operand `vector.rs:1281`, envelope top object `:1646`, clip source `:2118`, mask source `:2226-2227`, and `mask_node` from `:2244` (NOT_INSPECTED beyond its leaf test). A source layer holding an enabled filter refuses `vector_effect_source` rather than using the raw shape. Applying the effect there is a later slice.
   - Direct record exporters that do not go through `expand_vectors`: `interop-fig-native/src/export.rs:392` (it already loses raster effects typed at `:398-399`), `interop-vector-svg/src/svg_export.rs:1137`, `interop-vector-core/src/eps_export.rs:372` (NOT_INSPECTED beyond that line). A filter there is a typed loss: `self.lose` for fig, `note` (`svg_export.rs:213`) for SVG. **Same class, existing:** `git grep -i envelope` finds nothing in those three crates' `src`, so a `StudioEnvelope` is dropped with no loss on these paths today. Add its loss line beside the filter's, and report it.
   - Viewer entry budget list `harness-view/src/view/vector.rs:622` gains the member, and `validate_records` `:678-696` gains a retain like `:681`, so a deleted layer's stack survives for undo.
   - Stored coordinates must follow the layer. Free distort's `src*`/`dst*` are document coordinates. Mesh envelope points are moved at `vector/arrange.rs:199-206` and `design/auto_layout.rs:1061-1067`; do the same for free distort corners at both sites, or a moved layer distorts against a frame left behind. Absolute sizes do not scale (`scale_strokes_and_effects` default false, STU-VEC-157). Read for any third transform site.
   - `duplicate_layer` (`view/layers.rs:123`, `copy_subtree` `:135`): the first 45 lines of `copy_subtree` do not mention vector records, so a copy may lose its envelope and would lose its filters (UNVERIFIED: read it, pin the behaviour with a test, report). SMB-PR-ROUTING-REPAIR-1 edits `layers.rs:51`: ask before editing this file.
   - Laws that must keep `fill_rule`: seen at `laws.rs:118`, `:209`, `:261`, `:433`; zig zag, pucker and step repeat not read. Test all seven.

6. **Typed commands in the Viewer.** New egui-free `harness-view/src/view/vector/live_filters.rs`, shaped like `envelope.rs`, one method per STU-FX-007 verb, each ONE undo step (STU-FX-012c) through `commit_entry` (`vector/style.rs:618`) or, for bake, `commit_paths` (`vector/arrange.rs:213`). These reach the existing `layer.set_tiles_and_entry` and `layer.set_vector_paths_and_entry` rows (`harness-core/src/action.rs:856`, `:858`), so no new `Run` method and no RUN_COMMANDS row.
   - `apply`, `update` (one key, typed, through `EffectRecord::set`), `reorder`, `toggle`, `remove`, `bake`, and a read of a layer's stack with each row's seven numeric fields from `EffectKind::params`, so a model can write a parameter without knowing the effect (STU-VEC-154).
   - `apply` and `update` refuse a bad parameter before commit with nib's code, and store every resolved entry (`EffectRecord::entries`, `live_effect.rs:642`), including defaults, so a stored record does not change if a table default changes later.
   - `bake` (this slice's part of STU-VEC-054 and STU-FX-003): the effected paths become the layer's paths through `AnchorPath::from_owned_path`, as `expand_envelope` does (`envelope.rs:140-175`); style and width of output `k` come from source `k % n`; the stack goes; an envelope stays. Refuse networks and text-bound layers. Never implicit on save or export (STU-VEC-054 :4427-4428). The five expand selectors are not this slice.
   - Tests in new `harness/tests/viewer_vector_live_filters.rs`, `viewer` feature: apply then render differs from source; undo restores bytes; edit the source path and re-render follows (STU-FX-027); reorder two filters changes output deterministically; remove restores the exact prior render (STU-FX-003); same seed gives byte-identical output (STU-FX-011); save, reopen, same render; bake then render equals pre-bake render.
   - No panel, menu item, tool or `PanelAction` arm. No egui line. `state_json` (`view/mod.rs:837`) stays with SMB-HARNESS-DRIVE-1. No CLI or `manual.rs` edit.

7. **Descriptors.** `folio/src/lib.rs:1618`, `composite/src/lib.rs:198`, `nib/src/descriptor.rs:10` name the new member, codes and limits. Fix the wrong comment at `live_effect.rs:18-19`.

8. **MTs.** Read MT-30525, MT-30526, MT-30535, MT-30540, MT-30557, MT-30558, MT-30564, MT-32970, MT-32980..32986, MT-289, MT-291. Their `operator_surface_required` is YES (read on MT-30525, 30526, 30535, 30557, 30558, 30564, 32970, 32982, 32983), so expect partial claims: claim a row only if you read it and this proves it. Their proof commands name test targets that do not exist. Do not edit MT files.

Hostile input (V4-01/02): typed refusal, no abort, bounded allocation. Test: over-count stacks, filters and params; non-finite and huge reals; a very long string; duplicate keys; a stack on a raster or group layer; step repeat with `copies` near `i64::MAX` (refused before allocation, `laws.rs:497`); a chain of step repeats each just under `MAX_OP_ELEMENTS` (`shape_ops.rs:56`, 2^18); 4096 layers each with a full stack, which is why one evaluation's budget is not enough: charge source elements against one aggregate budget per `prepare_vectors` or `expand_vectors` call, as `MAX_MASK_CONTENT_PIXELS` does at `vector.rs:2136`. Measure, then declare the number, and record timings debug and release. A huge `move` must refuse at `COORDINATE_LIMIT` (`nib/src/owned_path.rs:14`) or `REFUSE_VECTOR_OVER_LIMIT`, not allocate a plane. A canceled token during a long stack returns promptly.

Vendor parity stays UNVERIFIED (D5): every law here is the declared Studio reading v1. Do not bless a parity row.

Out of scope: row scope and group or layer target ring (STU-VEC-023 second and third scopes); per-entry mask, blend, opacity and `render_scope` (STU-FX-002); Stylize, scribble and raster-effect settings (STU-VEC-156, STU-VEC-159, STU-FX-030); registering offset, simplify, round corners, geometry and warp as effect kinds; networks as sources; the five expand selectors and the rest of expand-appearance; effect styles; blend, live paint, trace, repeat; AI, SVG or PSD import of live effects; any panel, GUI, menu or Registry control; any egui, eframe, winit, wgpu, naga or WGSL line; any GPU kernel; any `Cargo.toml` change; no root input; no Master Spec edit; no MT edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: composite returns per-layer typed refusals, codes only; the Viewer reports through its existing report path; nib::live_effect::outcome and report (live_effect.rs:804, :817) already map codes to observe for a host that emits", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: composite and folio commit nothing; Viewer commands commit through the existing commit_entry and commit_paths path, which carries the Run context; no default_actor() or literal actor in new code", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-nudge-1`, branch `lane/smb-ai-live-effects-1`, base cede5958)

- nib: `src/live_effect.rs`, `src/live_effect/laws.rs` only if a test proves a defect, `src/descriptor.rs`, `tests/vector_live_effects.rs`.
- folio: new `src/vector_live_filter.rs`, `src/vector.rs`, `src/vector_tools.rs` (`validate_tools` call), `src/lib.rs` (module line, DESCRIPTOR), new test file.
- composite: new `src/vector_live_filters.rs`, `src/vector.rs`, `src/lib.rs` (module line, DESCRIPTOR), new `tests/vector_live_filters.rs`.
- package: `tests/externals.rs` classification rows only.
- harness-view: new `src/view/vector/live_filters.rs`, `src/view/vector.rs` (`mod` line, `:622`, `:678-696`), `src/view/vector/arrange.rs:199-206`, `src/view/design/auto_layout.rs:1061-1067`. Ask before `src/view/layers.rs`.
- harness: new `tests/viewer_vector_live_filters.rs`.
- interop-fig-native: `src/export.rs` loss lines. interop-vector-svg: `src/svg_export.rs` loss lines. interop-vector-core: `src/eps_export.rs` loss lines. Each with one test.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-pr-nudge-1`): step 0; clippy over nib, folio, composite and their direct dependents; nextest WHOLE suites of hsk-studio-nib, hsk-studio-folio, hsk-studio-composite, hsk-studio-package (rule 4b), hsk-studio-interop-vector, hsk-studio-interop-vector-core, hsk-studio-interop-vector-svg, hsk-studio-interop-fig-native; hsk-studio-harness-view with [viewer] and hsk-studio-harness viewer_vector*, viewer_illustrator*, viewer_vector_live_filters tests (rule 4e: confirm the unit in .config/round-features.toml, which lists hsk-studio-harness-view [viewer] at line 44); doctests if added; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-nudge-1 --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
