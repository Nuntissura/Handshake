---
file_id: LANE-SMB-DS-COMPONENTS-AUTHOR-1
file_kind: lane_brief
updated_at: 2026-11-08T08:51:44Z
wp: WP-KERNEL-STUDIO
lane: SMB-DS-COMPONENTS-AUTHOR-1
lane_file: builders/lanes/hsk-studio-ds-components-author-1.json
crates: hsk-studio-harness-view (component, variant set, property and instance authoring commands over Folio StudioDesignRecords and the hsk-studio-components redraw)
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A designer or model creates components, variant sets, properties and instances in a native document by typed commands, one undo step each, saved and reopened, and the existing instance commands work on them without a .fig">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/width-plan-20261108-raster-vector-design.json. Every open question below takes the planner's recommended default as a lead ruling. Its code facts were checked at 0a733db8; this lane's base is eb17fad6. Re-read every file:line at your base before you edit it.

- Tree: the planner proposed smb-sentinels-native-1, which another lane took; this lane uses wt-studio-lanes/smb-ai-distort-effects-1 and its target (HSK-ART-005 reuse).
- Where do authored components live: the vector entry design member (widen style.rs:291-300) or a new session entry token? -> The vector entry design member: one StudioDesignRecords per document, already holding auto-layout nodes and validated whole by Folio; a second entry would split one record set. Caps named in style.rs.
- STU-DS-003 says an instance renders the current definition, not a copy. This slice redraws on component.sync_instances, not on every definition edit. -> Accept for this slice: the redraw is the existing copy mechanism, an explicit step keeps one undo step per command, and the inspector reports stale instances by content signature; automatic propagation is slice 2. Record as an SD item that STU-DS-003 is partly met.
- Native instance origins: derived from layer bounds at command time, or stored? -> Derived: the fig receipt stores origin_px because the source gave it; for a native instance the definition bounds and the instance group's position are the same fact, and storing them would need a Folio field. Declared Studio reading.
- Command ids: spec names (component.*, component_set.*, instance.*) beside the existing design.instance.* rows? -> Spec names for every new row (STU-DS-107, STU-DS-162); the existing design.instance.* rows stay as they are; aliases are the catalog's job later (ui_seed.rs).
- Design pages: product-design step 3 asks for pages; this lane and SMB-DOC-NEW-1 use artboards. -> Artboards stand in, recorded UNVERIFIED; a page model for design documents is a separate spec question for the spec owner.

- Process: red first per item, then focused runs on D:. Stop at REVIEW READY for the read-only code-check and red-team review, then run ONE bundled run, then stop at BUNDLED GREEN. Lanes land one at a time on the lead's RELEASED (HANDOFF topic workflow, items 10, 11 and 16).
- Any lane that changes a Viewer command's refusal order also runs every harness process test that drives that command. This is the lesson from the 2026-11-08 trunk break at 35c50d3b.

## Target

Width plan 20261108 rank 1. A designer can only edit components that arrive in a `.fig`: the inspector reads definitions, sets and instances from the import receipt (`harness-view/src/view/design/instance.rs:8-13`, `Receipt` :141, `receipt_design` :153), and no command creates a component, a variant set, a property or an instance (git grep `create_from_selection|create_instance|combine_as_variants|add_property` over harness-view/src and harness-core/src: 0 hits). Folio already holds every record: `StudioComponent` (`folio/src/components.rs:164`), `StudioComponentSet` (:181), `StudioComponentProperty` (:151), `StudioComponentInstance` (:218), `StudioNodeDesign.component_property_references` (`folio/src/records.rs:80`), all in `StudioDesignRecords` (:92) validated by `validate_design_records` (:616); the instance binding record `StudioInstanceState {layer_id, instance_id, component_id, ..., content_signature}` exists (`folio/src/design_state.rs:92-112`) and is written to the session entry `design` (`harness-view/src/view/design/persist.rs:34`). The redraw that turns a definition into instance children exists (`instance.rs:556` `redraw`, `definition_layers` :185). So this lane is host authoring over existing records and engines: scenario product-design steps 18, 24, 25, 26, 27, 28, 29, 30, 32, 36, 46 and vector-brand 25, 26, 82, and steps 43-45 then run on native instances without a `.fig` bridge. All file:line below are at 0a733db8 (origin tip, read with git show / git grep in wt-studio-lanes/smb-pr-sequence-session-1); paths are under `handshake-studio-assets/crates/hsk-studio-`. Spec lines are module `14-studio-creative-suite.md` of v02.229.

Clauses: STU-DS-002 :20320, STU-DS-003 :20324, STU-DS-003a :20326, STU-DS-004 :20328, STU-DS-006 :20338, STU-DS-016 :20366, STU-DS-102 :19912, STU-DS-103 :19923, STU-DS-104 :19936, STU-DS-105 :19938, STU-DS-107 :19964-19966 (command names), STU-DS-108 :19974, STU-DS-109 :19976, STU-DS-110 :19985, STU-DS-111 :19995, STU-DS-112 :19997, STU-DS-113 :19999, STU-DS-115 :20007, STU-DS-120 :20019, STU-DS-162 :20222, STU-DS-163 :20224, STU-VEC-058 :4670, STU-AUT-147 :21905, STU-CON-007 :22896.

1. **Red first, compiling.** New `harness/tests/viewer_design_components.rs` (feature viewer) and a process test through `command --stdin` (`harness/src/cli/drive.rs:275-293`, typed rows run `Viewer::run_command`, `harness-view/src/view/commands.rs:129`). The document is `document.new` profile `design` (`harness-core/src/document_new.rs:51-68`) with shapes from `viewer.draw_shape` and text from `vector.place_text_with_font`. Every new id refuses `unknown_command` at red; prove red by running.
2. **One home for authored design records (decide before code, open question 1).** The vector entry's `design` member (`harness-view/src/view/vector.rs:309`, field :316) already carries authored `StudioDesignRecords` for auto layout (`design/auto_layout.rs:421` `al_records`, :649 `al_commit`), but its decoder refuses components, sets, instances and variables (`view/vector/style.rs:291-300` `validate_entry_styles`, error `vector_entry_design: members`). Default: widen that check to accept components, component sets, instances and `component_property_references` nodes, with named caps, and keep Folio `validate_design_records` as the authority. Commit through the existing paths so each command is ONE undo step: entry-only through `commit_entry` (`view/vector/style.rs:618`); structure plus entry through `Run::restructure_and_entry` (`harness-core/src/harness.rs:1488`).
3. **Merged records.** The inspector reads receipt records plus entry records as one `StudioDesignRecords` (an id in both is refused `design_record_duplicate`, Folio `check_identity_uniqueness` per the components DESCRIPTOR). `instance_record` (`instance.rs:289`) and `instance_view` (:310) find a native instance by its `StudioInstanceState` binding in the `design` entry; origins of native definitions and instances come from their layers' bounds at command time (Studio reading, declared). Rotated or scaled definitions and instances stay refused as today (`instance.rs:20-28`).
4. **Commands** (ids are the spec's, STU-DS-107 :19966 and STU-DS-112 :19997; new domain files so no row file is shared):
   - `component.create_from_selection {layers, name}`: one layer that is a group becomes the definition; several layers are grouped first in the same step (`Run::restructure_and_entry`, new group via `layers.rs:77` `new_layer`, :65 `operation_for`, ids `layers.rs:51` `mint_layer_id`). Record `StudioComponent {SCMP id, layer_id, publishing, variant_properties: null, component_property_definitions: []}`; slash-path names.
   - `component.create_one_per_top_level_node {layers}`: one component per listed top-level layer, one step (STU-DS-002).
   - `component.create_instance {component, x, y, parent?}`: a new group layer, children copied from the definition through the existing `redraw` (`instance.rs:556`), a `StudioComponentInstance` (scale_factor 1, no overrides) and its `StudioInstanceState` (persist.rs `write_design_state` :215), one step. A set id instantiates `default_variant_id`.
   - `component_set.combine_as_variants {components, properties: {name: [values...]}, assignments: {component: {name: value}}}`: a container group holding the members, `StudioComponentSet` with ordered `variant_group_properties` (STU-DS-108), members get `variant_properties`; completeness and duplicates are Folio's refusals (`variant_assignment_incomplete`, `variant_assignment_duplicate`, components DESCRIPTOR), which is step 26. `component_set.set_default_variant {set, component}`.
   - `component.add_property`, `component.edit_property` (rename, default, options in one call; a rename rewrites every instance's `component_properties` key in the same step, STU-DS-112), `component.delete_property`; types exactly STU-DS-110's five, `SLOT` refused as the components crate does.
   - `component.set_property_reference {layer, field, property}`: writes `StudioNodeDesign.component_property_references` (records.rs:80; STU-DS-113). The reference spelling is not parsed by Folio (components.rs:21-22): use the plain property name and record the reading.
   - `component.set_description {component, description, documentation_links}` (STU-DS-004, `StudioPublishing`).
   - `instance.detach {layer}`: the group stays, its instance record and states go, children stay as plain layers; returns the frame id (STU-DS-107). `component.list_instances {component}` (immediate form only, STU-DS-107 :19968) and `instance.get_main_component {layer}`.
   - `component.sync_instances {component}`: redraws every instance of a definition after the definition's layers changed, keeping overrides (one step). Automatic propagation on every definition edit is not this slice (open question 2), so step 32 is reached through this command.
5. **Rows and dispatch.** New `harness-core/src/action/commands/component.rs` (domains `component`, `component_set`, `instance`; one `mod` and one `DOMAINS` line in `action/commands.rs:164-191`); decoders in a new `harness-view/src/view/commands/component.rs`, dispatch arms beside `"design" if design::owns(id)` (`view/commands.rs:157`); Viewer methods in a new egui-free `harness-view/src/view/design/component.rs`. The catalog seeds itself from RUN_COMMANDS (`harness-core/src/action/ui_seed.rs:18-23`). Do not touch `view/commands/vector.rs` (it hosts `run_design` :236 for auto layout, and SMB-AI-LIVE-EFFECTS-1 adds vector commands).
6. **Failure-class sweep (V4-06): every reader that sees design records or instance layers.** List each, fixed or not applicable with a reason: `duplicate_layer` (`view/layers.rs`, copies layers without records: an instance copy becomes a plain group, report it, or create an instance; read it), deleting a definition layer (STU-DS-003a :20326: instances keep rendering with `main_component_id` null), the `.fig` export of native components (`interop-fig-native/src/export.rs`: a typed loss line, NOT_INSPECTED here), SVG and PDF export (draw layers, not applicable), `doc_export.rs` PSD (`psd_layer_kind_not_exported` path, read it), `prune` on open of records naming deleted layers (as `auto_layout.rs:354` and `place.rs:333` do), `state_json` design projection (`instance.rs:802` `design_json`).
7. **Hostile input (V4-01/02).** Typed refusal, bounded: counts of components, members, properties and instances under named caps charged before allocation; names through Folio's label and slash-path rules; a 4096-layer selection; a property rename colliding with an existing name; a combine whose assignments repeat; an instance placed at a non-finite or huge coordinate (`COORDINATE_LIMIT`); a cycle (a definition containing an instance of itself) refused `component_cycle` before the redraw.

Out of scope: variables and bindings (rank 3 lane), library publish and update review, slots (`component.create_slot`), `component.clone`, `component_set.extract_member`, `component.restore_main_from_instance`, exposed nested instances authoring, deferred `list_instances`, any panel, Assets browser or GUI line, `.fig` export of native components beyond a loss line, automatic propagation, scale_factor other than 1, any Folio schema change, any Cargo.toml change, any MT or Master Spec edit. Vendor parity UNVERIFIED (D5).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: every refusal is a CommandRefusal code (view/commands.rs:129 run_command); Folio diagnostics pass through as their codes, as place.rs folio_refusal does; the components crate report() already emits one Observe outcome per evaluation", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: Viewer methods commit through commit_entry and Run::restructure_and_entry, which carry the viewer run context; no default_actor() or literal actor in new code", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-distort-effects-1`, branch `lane/smb-ds-components-author-1`, base eb17fad6)

- harness-core: new `src/action/commands/component.rs`; `src/action/commands.rs` (one `mod` line, one `DOMAINS` line).
- harness-view: new `src/view/design/component.rs` (Viewer methods, egui-free), new `src/view/commands/component.rs` (decoders), `src/view/commands.rs` (dispatch arms near :157), `src/view/design.rs` (`mod` line), `src/view/design/instance.rs` (merged records, native bindings), `src/view/design/persist.rs` (native states), `src/view/vector/style.rs:291-300` (widened member check, caps).
- harness: new `tests/viewer_design_components.rs`; one process test file or a case in an existing drive test file.
- Not touched: `src/view/vector.rs`, `src/view/design/auto_layout.rs`, `src/view/commands/vector.rs` (SMB-AI-LIVE-EFFECTS-1 edits the first two), folio, components, chronicle, session.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-ai-distort-effects-1 --disk d`): step 0; clippy over harness-core, harness-view, harness; nextest WHOLE suites of hsk-studio-harness-core and hsk-studio-harness-view with [viewer]; hsk-studio-harness viewer_design*, viewer_fig*, design and drive tests (confirm the units in .config/round-features.toml, rule 4e); no folio or components suite unless a test proves an engine defect; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-ai-distort-effects-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
