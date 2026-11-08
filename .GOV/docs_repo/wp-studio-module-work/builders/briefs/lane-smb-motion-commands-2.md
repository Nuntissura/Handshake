---
file_id: LANE-SMB-MOTION-COMMANDS-2
file_kind: lane_brief
updated_at: 2026-11-08T08:53:46Z
wp: WP-KERNEL-STUDIO
lane: SMB-MOTION-COMMANDS-2
lane_file: builders/lanes/hsk-studio-motion-commands-2.json
crates: hsk-studio-harness-core (typed motion commands: solid and adjustment layers, keyframe selection, clipboard and ease) over hsk-studio-folio motion records and hsk-studio-composite composition evaluatio
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Solid and adjustment layers are created by typed commands, an adjustment layer's effect stack applies to the composite beneath it, and keyframes can be selected, copied, pasted, reverse-pasted, shifted and eased as named undo steps">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/width-plan-20261108-video-motion-audio.json. Every open question below takes the planner's recommended default as a lead ruling. Its code facts were checked at {'commit': '0a733db8', 'subject': 'harness-view, PLACE-ASSET-1: F4 float sources decode unclipped (lead ruling 2026-11-08)', 'baseline_of_scenarios': 'c4d06841', 'commits_since_baseline': 132, 'read_from': 'wt-studio-lanes/smb-pr-edit-footprint-1/handshake-studio-assets via git grep and git show 0a733db8:./crates/... (working tree at 08b372af, not checked out)', 'spec': 'v02.229, .GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md', 'label': 'READ'}; this lane's base is eb17fad6. Re-read every file:line at your base before you edit it.

- Tree: the planner proposed smb-folio-angle-unbounded-1, which another lane took; this lane uses wt-studio-lanes/smb-pr-sequence-session-1 and its target (HSK-ART-005 reuse).
- Kind adjustment with a Payload::Group (Folio validator change at lib.rs:931) or a composition-layer adjustment flag? -> Group payload: it keeps STU-CMP-015's 'a StudioLayer with kind = adjustment, not a special object'; the reader sweep is the cost and is listed. Fall back to the flag only if the sweep finds a reader that cannot refuse typed.
- Blend modes on an adjustment layer: STU-CMP-015 is silent. -> Normal only in this slice, other modes refused composition_adjustment_blend, because the lerp reading is only established for normal.
- Paste onto a tick that already holds a key: replace or refuse? -> Replace, labelled a Studio reading (D5), because a refusal blocks the common paste-over-a-pose workflow; the step is undoable.
- Selection state lives in Run (harness-core) and is lost on process end. Should it persist with the document? -> No: selection is view state; STU-MOT-038 lists it as commands, not records; the GUI owns its own copy later through the same rows.
- Text kind: add a composition.add_layer kind text that creates the story itself? -> No in this slice: harness-core cannot shape text or reach the Viewer's fonts; the two-row path exists and gets a render test.
- Solid size bound: STU-MOT-005 states none. -> 1..=30000 from STU-MOT-122's resize bound, declared in the descriptor as a Studio reading.

- Process: red first per item, then focused runs on D:. Stop at REVIEW READY for the read-only code-check and red-team review, then run ONE bundled run, then stop at BUNDLED GREEN. Lanes land one at a time on the lead's RELEASED (HANDOFF topic workflow, items 10, 11 and 16).
- Any lane that changes a Viewer command's refusal order also runs every harness process test that drives that command. This is the lesson from the 2026-11-08 trunk break at 35c50d3b.

## Target

Backlog row SMB-MOTION-COMMANDS-1 landed (0a733db8: 2b54bbc8, 912cb86f) and left its declared limits (`harness-core/src/motion_commands.rs:28-31`): no solid, adjustment, text or footage kind (`:993-994`, `layer_kind_unsupported`), no keyframe selection, copy or paste, no expression store. This lane closes the layer-kind and keyframe part that panels and scripts need first. Motion-comp scenario steps 16 (solid), 17 (Gradient Ramp on the solid), 28 (Easy Ease), 44 (adjustment layer with Levels and Tint) become callable; step 19 (text) is proven through rows that exist. All file:line at 0a733db8, paths under `handshake-studio-assets/crates/hsk-studio-`.

Clauses (spec module `14-studio-creative-suite.md`, v02.229): STU-MOT-005 :25959 (ten layer kinds: `solid` :25969, `adjustment` :25975), STU-MOT-006 :25980, STU-CMP-015 :29538 (adjustment effects apply to the composite beneath; masks scope where, opacity scales how much, in/out scope when; adjustment layers nest), STU-MOT-034 :27635, STU-MOT-037 :27680, STU-MOT-038 :27695 (select-all three scopes :27706, deselect :27708, shift selected 1 or 10 frames :27710, copy, paste and REVERSE paste :27712, set interpolation per selection :27713), STU-MOT-039 :27718, STU-MOT-130 :28326, STU-FX-130 :13163.

1. **Red first, compiling.** New `harness/tests/motion_commands_2.rs` (feature `media`, like `harness/tests/motion_commands.rs:617`), parameters as JSON values through `Run::motion_command` (`harness-core/src/harness.rs:1599`), so the red commit compiles and every new row refuses (`row` lookup `motion_commands.rs:62-67` returns none until the rows exist). New `composite/tests/composition_adjustment_effects.rs` with records written as JSON. Prove red by running.

2. **Folio.**
   - `StudioMotionRecords` (`folio/src/motion_records.rs:656-676`) gains `solid_layers: Vec<StudioSolidLayer>`, `#[serde(default, skip_serializing_if = "Vec::is_empty")]` as its siblings (`:659-675`), added to `new` (`:678-689`), so existing record sets stay byte-identical. `StudioSolidLayer {layer_id, width, height, colour}`: no such type exists (`git grep 'StudioSolidLayer'` empty, READ). Validation beside the shape-layer checks: the layer is in a composition and its document layer is an empty raster layer (the shape precedent, `harness-core/src/motion_commands.rs:1023-1027`), one record per layer, not also a shape or text-animator layer (`procedural_layer_twice`, composite `temporal/motion_records.rs:388-390`), width and height 1..=30000 (the STU-MOT-122 resize bound, :28283-28284; the solid size bound is not stated by STU-MOT-005, so this is a Studio reading), finite colour components 0..=1. Colour type: the one shape fills use (NOT_INSPECTED name; read `StudioShapeItem` fill before choosing).
   - Adjustment kind: Folio's document validator accepts `kind = "adjustment"` only with a `PrimitiveReference` to `ADJUSTMENT_SCHEMA` (`folio/src/lib.rs:931-942`), and the raster records then require a pigment adjustment record for it (`folio/src/raster.rs:1812-1828`). An After Effects adjustment layer has no pigment adjustment of its own. Accept `kind = "adjustment"` with `Payload::Group` as the composition adjustment layer of STU-CMP-015 (ValueType::Image like the pigment one). The raster checks test the `PrimitiveReference` payload (`raster.rs:1762-1764`, `:1819-1820`), so they do not demand a record for it; prove that with a test. See open question 1 for the alternative.

3. **Composite.**
   - Solid: `procedural_layers` (`composite/src/temporal/motion_records.rs:372-399`) gains `ProceduralLayer::Solid`, lowered by building a one-rectangle shape group with a solid fill and calling the same `shape_layer_content` (`:420`), so a solid draws through the shape path, not a second rasteriser. Test: a solid equals byte for byte the equivalent rectangle shape layer.
   - Adjustment effects (STU-CMP-015): `CompositionTemporalSource` maps document layers at `temporal/composition.rs:930-1001`; add an arm for kind `adjustment` with `Payload::Group` BEFORE the `(None, Payload::Group) => Content::None` arm (`:975`) and before the static arm (`:952-954`, whose pattern includes `Payload::Group`). Without effects it is `Content::None` (renders nothing, as STU-CMP-015 'no content of its own'). `read_effects` (`temporal/composition/effects.rs:240-252`) refuses the stack for `Content::None | Content::Adjustment(_)`; keep that refusal for nulls and pigment adjustment layers, lift it for this kind. New `SceneContent` variant (`composite/src/composition.rs:114-168`, `#[non_exhaustive]`), handled in the root loop beside `SceneContent::Adjustment` (`composition.rs:959-997`): render everything beneath the layer into one straight RGBA f32 surface B with the backend the scene already uses (the nested-content render `content_surface` used by `effected_surface`, `composition/effect_stack.rs:846-875`), run the entries over B in stack order with the existing CPU kernels to E, and write L = B + k * (E - B) with k = layer opacity * mask coverage (masks fold as layer-space coverage, `effects.rs:16-21`). L replaces the layers beneath in the root plan as one tile, as `SceneContent::Effected` places its surface (`composition.rs:1134-1146`). Nested adjustment layers then compose by construction (the lower result is part of B). Charge B and E with `charges.surface` as `effect_stack.rs:873-878` does.
   - Typed refusals, each with a test: blend mode other than normal `composition_adjustment_blend` (open question 2); 3D adjustment layer (`COMPOSITION_3D_CONTENT`, as `temporal/composition.rs:1080-1081`); a track matte on it (`ADJUSTMENT_MATTE`, as `composition.rs:960-964`); the document-graph path (already refuses any stack, `temporal/composition.rs:1255`); inside a collapsed precomposition; a pigment adjustment layer (PrimitiveReference) with a stack stays `COMPOSITION_EFFECT_CONTENT`.
   - Reader sweep (V4-06) for a group-payload adjustment layer: every reader that tests kind `adjustment` - `composite/src/lower_extensions.rs:146-150`, `folio/src/raster.rs:1677`, `:1700`, `:1762`, `:1819`, `harness-core/src/harness.rs:124`, `harness-view/src/view/info.rs:311`, `harness-view/src/view/render.rs:582` - and the static document render of a raster document holding one (outside a composition it must refuse typed, never draw as an empty group without a word). List each site fixed or not applicable with a reason.

4. **Commands (harness-core `motion_commands.rs`, rows in `action/commands/motion.rs:37-90`).**
   - `composition.add_layer` (`layer_add`, `:980-1061`): kind `solid` (empty raster document layer through `create_layer` `:276-297`, plus the `StudioSolidLayer`; width and height default to the composition's, `colour` required) and kind `adjustment` (document layer kind `adjustment`, payload group: `create_layer` builds only `group` or `raster` today, `:281-288`, so give it the kind). New row `composition.set_solid` (colour, width, height; one named step). `layer_duplicate` (`:1085-1105`) copies a solid's record with the layer, and an adjustment layer duplicates like a group. `remove_layer_records` (`:1063`) drops the solid record.
   - `footage` and `guide` stay `layer_kind_unsupported` (footage is the next motion lane). Text: no `text` kind; the typed path is `vector.place_text_with_font` (`harness-core/src/action/commands/vector.rs:34`, `Viewer::place_text_with_font` `harness-view/src/view/text.rs:698`) then `composition.add_layer kind=existing` (`motion_commands.rs:989-992`). Add a `media` test that renders the composition through `session_render_queue_jobs` and finds the text: today no test does (UNVERIFIED that it renders). If it refuses, report the code and stop there for text.
   - Keyframe selection and clipboard: transient state in `Run`, beside `web: web_commands::WebHost` (`harness-core/src/harness.rs:459`, built at `:256`; struct `:426`): selected keys as `(layer_id, property address, tick)` and one clipboard. Not a document edit, no history step, not saved. A selected key that no longer exists after an edit, undo or redo is dropped when read and counted. Rows: `keyframe.select` (scope `property|layer|composition`, mode `replace|add|remove`, optional `times`), `keyframe.deselect_all`, `keyframe.selection` (read), `keyframe.copy` (selection, or explicit `layer_id`, `property`, `times`), `keyframe.paste` (target `layer_id`, optional `property`, `time`, `reverse`), `keyframe.shift_selected` (`frames`, any non-zero integer in composition frames; the 1 and 10 steps are the GUI's), `keyframe.ease` (`side both|in|out`) and `keyframe.toggle_hold` over the selection. Paste places the earliest copied key at `time` and keeps relative order (Adobe help, see research basis), copies value, interpolation and tangents, is one named step, refuses `clipboard_empty`, `paste_property_mismatch` (value dimension differs), and replaces a key at an identical tick (open question 3). Reverse paste maps t to (t_first + t_last - t) relative to the anchor and swaps each key's in and out interpolation and tangents (Studio reading, D5).
   - Ease and toggle hold mirror `hsk_studio_motion::assistants::ease` and `toggle_hold` (`motion/src/assistants.rs:64-100`: bezier, speed 0, influence 0.16666666666 per STU-MOT-034) because harness-core has no motion edge (`motion_commands.rs:23-25` states this and pins its own mirror with a `media` test); pin these the same way against motion.
   - Every new row `recordable: false, mutating: true` like its siblings (`motion.rs:83-89`); `keyframe.select*`, `keyframe.selection`, `keyframe.copy` are non-mutating. The drive loop reaches motion rows without a new dispatch arm (`harness/src/cli/drive.rs:159`); confirm and add the catalog rows the drive key-equality test reads.

5. **Descriptors and manual.** Module docs `motion_commands.rs:1-31` (declared limits updated), folio and composite DESCRIPTOR (`composite/src/lib.rs:198`), the new refusal codes. No Master Spec edit.

6. **MTs.** Read MT-33852 and MT-33853 (STU-MOT-005), MT-34075 (STU-CMP-015), MT-33884 and MT-33885 (STU-MOT-038), MT-33883 (STU-MOT-037). Their acceptance lists are empty or per-row; claim a row only if read and proven here. Do not edit MT files.

Hostile input (V4-01/02): solid size 0, above the bound or non-finite colour; a selection over a declared key cap (proposed 65536 keys) refused before allocation; a clipboard over the same cap; paste whose last key passes `i64` ticks refused `tick_overflow` (the existing code); an adjustment layer over a 30000 x 30000 composition refused by the surface charge, not by allocation; 64 nested adjustment layers each with a full stack charged against the scene's budget; cancel during B or E returns promptly.

Vendor parity UNVERIFIED (D5): the lerp reading, paste collision, reverse tangent swap and the 0.1667 ease influence (After Effects F9 uses 33.33 percent, `motion/src/assistants.rs:6-9`). Do not bless a parity row.

Out of scope: footage, guide and text kinds as new kinds; expressions on property streams (G6); Sequence Layers and presets; spatial tangents; any panel, egui, Vulkan or GUI line; render queue run path (SMB-DELIVERY-QUEUE-1); any `Cargo.toml` change; Master Spec and MT edits.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: composite returns per-layer typed refusals (codes only) through TemporalError::Unsupported; motion commands keep their typed error and bounded target in the op record (motion_commands.rs:7-12); keyframe.selection reports the dropped-key count", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: composite and folio commit nothing; every mutating row commits through motion_commands::commit (:405-460) with the caller's CommandContext; selection and clipboard rows make no history step and no business event; no default_actor() or literal actor in new code", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-sequence-session-1`, branch `lane/smb-motion-commands-2`, base eb17fad6)

- folio: `src/motion_records.rs` (member, type, validation), `src/lib.rs:931-942` (adjustment with group payload), `src/raster.rs` only if the sweep proves a defect, DESCRIPTOR, new test file.
- composite: `src/temporal/motion_records.rs` (Solid), `src/temporal/composition.rs:930-1001` (adjustment arm), `src/temporal/composition/effects.rs:240-252`, `src/composition.rs` (variant and root-loop case), `src/composition/effect_stack.rs` (shared surface helper only if needed), `src/lower_extensions.rs:146-150` if the sweep needs it, `src/lib.rs:198`, new `tests/composition_adjustment_effects.rs`.
- harness-core: `src/motion_commands.rs`, `src/action/commands/motion.rs`, `src/harness.rs` (one `Run` field and its constructor), `src/harness.rs:124` sweep site.
- harness-view: `src/view/info.rs:311`, `src/view/render.rs:582` sweep sites only.
- harness: new `tests/motion_commands_2.rs`; catalog test rows if the drive key-equality test requires them.
- package: `tests/externals.rs` classification row for `solid_layers[].layer_id` if the externals test asks for it (rule 4b).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-sequence-session-1 --disk d`): step 0; clippy over folio, composite, harness-core and their direct dependents; nextest WHOLE suites of hsk-studio-folio, hsk-studio-composite (with feature motion), hsk-studio-harness-core, hsk-studio-session, hsk-studio-package (rule 4b, folio record change); hsk-studio-harness tests motion_commands*, motion_records*, render_queue*, composition_export, harness_drive with features media (rule 4e: confirm the unit in .config/round-features.toml); doctests if added; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-sequence-session-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
