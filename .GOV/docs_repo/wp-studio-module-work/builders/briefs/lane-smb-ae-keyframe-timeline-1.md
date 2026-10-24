---
file_id: BUILDER-BRIEF-LANE-SMB-AE-KEYFRAME-TIMELINE-1
file_kind: builder_brief
updated_at: 2026-10-24T01:17:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-AE-KEYFRAME-TIMELINE-1
lane_file: builders/lanes/hsk-studio-ae-keyframe-timeline-1.json
crates: [hsk-studio-harness-view, hsk-studio-harness (RUN-UI.md row only)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip 1e3ae3d4); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, parity rows, expected gain, model">

## Target

- App: After Effects. GUI pooled is 0 % in every tier (parity-20261023-rollup.json per_app.after_effects; all 8 workflows ENGINE_ONLY, research/audit-20261021/gui-surfaces.json).
- Workflows this lane makes doable in the window: "Layers, keyframes, parenting" and "Graph editor / keyframe assistants" (slices 1-4); "Create/open a composition" (slice 5). Strict UI row AE-09 (keyframe timeline / graph editor) moves toward AGREE.
- Expected gain (rollup formula: judged 1.0, unjudged 0.5 per workflow): AE GUI pooled 0 -> 12.5 % (2 workflows) or 18.8 % (3) while unjudged, 25.0 % / 37.5 % once judged; 108-workflow pool +1.0 to +1.5 points unjudged (+0.9 to +1.4 pp), +2 to +3 points judged. Largest single GUI gain available: the app sits at 0 %.
- Model / effort: opus / high. Reason: largest GUI lane of the wave, a new crate edge, a Folio<->Motion adapter, additive hunks in two shared shell files and a spec debt gate (G1).

</topic>

<topic id="gates" status="active" summary="Dispatch gates the lead must clear first">

## Dispatch gates (lead, before dispatch)

- G1 spec debt (STU-SHL-120/130). Grep `[STU-SHL-130]`: "**[STU-SHL-130] SD-1 — THE LAYER-VERSUS-NODE COMPOSITING FORK. NOT DECIDED. Settle before SHL-P-14.**" and grep `SHL-P-15`: "| SHL-P-15 | 11 | `keyframe-timeline` panel | SHL-P-14, SHL-P-12 | ...". [STU-SHL-120]: "An implementer encountering one of these MUST stop and escalate rather than choosing." This lane adds NO domain model: it is a projection over the existing, round-validated layer records (`hsk_studio_folio::StudioCompositionRecords`, `hsk_studio_motion`), as [STU-MOT-053] requires ("The graph editor is a projection of the property tree, not a separate document"). The lead records in the lane file that the panel does not decide SD-1 (precedent: SMB-TIMELINE-UI-1 built the clip timeline, SHL-P-20, ahead of SHL-P-12/13). If the lead rules SD-1 blocking, the lane is NOT dispatched and the question goes to the Operator.
- G2 new crate edge `hsk-studio-harness-view -> hsk-studio-motion` (lead decision, steward `steward-sync.sh --add-edge harness-view=motion`). Acyclic: motion's allowed edges are accord, folio, nib, type, observe; nothing depends back on harness-view. Needed because `hsk_studio_composite` does not re-export motion (its folio->motion converter `crates/hsk-studio-composite/src/temporal/composition.rs:631 pub(crate) fn property` is private and its file belongs to READY lanes). Without G2 only slice 1 can be built.

</topic>

<topic id="paths" status="active" summary="Owned paths, shared hunks, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-ae-keyframe-timeline-1`)

- NEW `crates/hsk-studio-harness-view/src/view/keyframe_timeline.rs` and `crates/hsk-studio-harness-view/src/view/keyframe_timeline/**` (suggested: `outline.rs`, `graph.rs`, `adapter.rs` folio<->motion, `ops.rs`).
- NEW `crates/hsk-studio-harness-view/tests/viewer_keyframe_timeline.rs` (own fixtures; do not import helpers from `viewer_timeline.rs`).
- `crates/hsk-studio-harness-view/Cargo.toml`: one line `hsk-studio-motion = { path = "../hsk-studio-motion", version = "=0.1.0", default-features = false, optional = true }` and `"dep:hsk-studio-motion"` added to the `viewer` feature list (after G2).
- Additive hunks ONLY in shared shell files (rebase conflicts with READY lanes otherwise):
  - `src/view/shell.rs`: `Panel::KeyframeTimeline` variant; `Panel::ALL` 24 -> 25 (insert after `Panel::Timeline`); arms in `key` ("keyframe_timeline"), `label` ("Keyframe Timeline", registry row of STU-SHL-079), `key_code`/`shortcut_text`/`modifiers` (Ctrl+Shift+F4: F2-F12, Shift+F2-F12 and Ctrl+Shift+F2/F3 are taken; Studio choice, no spec binding verified), visibility arm beside `Panel::Timeline => self.timeline.is_some()` (~:734), availability arm beside `:767` ("the open document has no composition") and the cm availability arm beside `:793` (`ReasonCode::NoSuchContainer`, container `container:composition`), open arm beside `:1273`.
  - `src/view/mod.rs`: `pub mod keyframe_timeline;` beside the timeline module line (~:75), one `Viewer` field beside the timeline field (~:223), one draw call beside `if self.shell.visible(shell::Panel::Timeline)` (~:985), one `state_json` key `keyframe_timeline`.
  - `crates/hsk-studio-harness/RUN-UI.md`: one panel-table row and one usage paragraph (manual obligation; no other edit).

## Do NOT touch

- `src/view/timeline.rs`, `src/view/timeline_ui.rs`, `src/view/timeline/**`, `tests/viewer_timeline*.rs` (lane SMB-TIMELINE-TRIM-1 owns them tonight); `src/view/hosted.rs` (READY PANELS-1); `src/view/file_dialog.rs` (READY SHELL-2); `src/view/timeline_export/**` (READY RENDER-QUEUE-2); any file of `hsk-studio-folio`, `hsk-studio-composite`, `hsk-studio-motion`, `hsk-studio-session`, `hsk-studio-controls`, `hsk-studio-pulse`, `hsk-studio-prism`, `hsk-studio-package`; `crates/hsk-studio-harness/tests/viewer_shell.rs` and `viewer_panels.rs` unless a test there fails because of your panel (then: minimal additive hunk, recorded in lane notes, reported to the lead).

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (resolve `.GOV/spec/SPEC_CURRENT.md` -> v02.222 `spec-modules/14-studio-creative-suite.md`; grep, never read the module)

- `[STU-SHL-091]`: "`timeline` (clips on tracks) and `keyframe-timeline` (property rows with keyframe tracks) are separate panels." ... "Both panels read ONE document time model: frame rate, duration, work area, playhead and time display base. A playhead moved in either moves in both and in every viewer and meter." ... "minimum height 300px; property-tree column 320px default and 220px minimum, below which it degrades to a name-only tree with the value column suppressed; time column never below 400px, below which the panel shows a HORIZONTAL SCROLL rather than compressing the time ruler".
- `[STU-SHL-079]`: "`keyframe-timeline` (bottom, `document_type_specific`)"; registry row (grep `| Keyframe Timeline |`): "| Keyframe Timeline | `keyframe-timeline` | `document_type_specific` | Time & Results Dock | composition, sequence |".
- `[STU-MOT-038]`: "The normative keyframe operation set. Each is a typed command and one reversible history step." Rows used: "Add or delete at current time | Per property, toggling. Adding captures the property's current evaluated value, so adding a keyframe never changes the render."; "Enable animation on a property | Converts `static` to `keyframed` by creating the first keyframe at the current time."; "Disable animation | Converts back to `static`, discarding keyframes"; "Go to next / previous keyframe"; "Shift selected keyframes by one frame / by ten frames, earlier or later"; "Drag to move in time | Snaps to the playhead, to other keyframes, and to layer in/out points."; "Copy, paste, and REVERSE paste"; "Set interpolation | Per side, per selection, from the [STU-MOT-031] enumeration."; "Edit interpolation numerically" / "Edit velocity numerically" (-> [STU-MOT-055]); "Select all keyframes on a property, layer, or composition".
- `[STU-MOT-037]`: "The normative ease assistant set is three operations" (Ease, Ease In, Ease Out) and "A fourth operation toggles `hold`".
- `[STU-MOT-039]`: "A keyframe operation on a multi-component property applies per component unless the property is unseparated".
- `[STU-MOT-050]`: "Studio's timeline has two display modes over the same data -- the LAYER BAR view and the GRAPH EDITOR -- toggled by one command, and every keyframe operation of [STU-MOT-038] works in both."
- `[STU-MOT-051]`: "Value graph. The vertical axis is the property's value." / "Speed graph. The vertical axis is the rate of change." / "A multi-component property shows one curve per component, individually selectable, and a colour per component that is stable across the session."
- `[STU-MOT-052]`: "choose which properties are graphed ...; auto-fit the vertical axis to the visible curves, or fit-all, or set the range manually; box-select and lasso-select keyframes; drag keyframes in two dimensions (time and value simultaneously) with axis constraint modifiers; drag tangent handles with an option to break or maintain continuity; snap to frame boundaries with a toggle; show or hide the reference graph of non-selected properties; show the audio waveform behind the graph; and apply the ease assistants of [STU-MOT-037] to the selection."
- `[STU-MOT-053]` (projection, no graph-only data) and `[STU-MOT-055]` ("A keyframe INTERPOLATION record exposes ... A keyframe VELOCITY record exposes, per component: incoming speed, incoming influence, outgoing speed, outgoing influence, and a continuity toggle").
- Build-plan row (grep `SHL-P-17`): "Graph editor as a MODE of `keyframe-timeline` | SHL-P-15 | temporal interpolation only; the spatial half waits on [STU-SHL-132]". The spatial (motion path) half is OUT of scope.
- `[STU-MOT-004]` (slice 5): "The normative composition settings record" (width/height 4..30000 default 720/480; frame_rate 1..999 default 29.97; "`duration` ... Required at creation; no default declared").
- `[STU-SHL-008]`: "No vendor product name is a Studio menu title, submenu, leaf, panel, region, tool, family, Layout Preset, Task Scope, command id or manual anchor."

</topic>

<topic id="predecessors" status="active" summary="What exists, gaps, patterns">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-timeline-ui-1.json` notes: panel registration hunks, F-TL-1 (toolbar separators took the window height; fixed 62ad523) and F-TL-2 ("first-frame panel sizing differs from steady state ... headless pointer tests must settle the layout before aiming"); test helpers `frame`, `settle`, `drag`, `ak_find`, `ak_click` in `crates/hsk-studio-harness-view/tests/viewer_timeline.rs:168-248` (copy the pattern, not the file).
- `builders/lanes/hsk-studio-ae-motion-tools.json` (graph editor MODEL already built): `crates/hsk-studio-motion/src/graph.rs` (`graph`, `speed_at`, `path_speed_at`, `fit_range`, `velocity_record`, `set_velocity`, `separate_dimensions`, `join_dimensions`), `src/assistants.rs` (`ease`, `toggle_hold`, `set_interpolation`, auto/continuous bezier, roving), `src/property.rs` (`Property::set_keyframes`, `sample`).
- `builders/lanes/hsk-studio-harness-shell-2.json`, `hsk-studio-harness-panels-1.json`: menu built from `Panel::ALL` (shell.rs:595, :874, :1361, :1428), so a new variant appears in WORKSPACE > Panels and in the HELP/shortcut reference without a controls edit.
- Session API: `crates/hsk-studio-session/src/motion.rs:37 composition_records`, `:54 set_composition_records` (validated by Folio on every set). Check whether document entries take part in EDIT > Undo (`session.rs:2189 set_document_entry`; precedent: the waveform editor's `waveform_edit` document entry shows up as an undo label, `crates/hsk-studio-harness/tests/audio_waveform.rs:142`, so follow the route `view/waveform.rs` uses); if not, keep a bounded panel undo stack of record snapshots (one entry per command, wired to Ctrl+Z / Ctrl+Shift+Z only while the panel is active) and record the choice in lane notes; never edit hsk-studio-session.
- Folio records used read/write through pub API only: `StudioCompositionRecords`, `StudioComposition`, `StudioCompositionLayer {layer_id, timing, parent, streams, ...}`, `StudioPropertyStream {property, static_value, keyframes}`, `StudioCompositionKeyframe {time, value, in_interp, out_interp, in_tangent, out_tangent}`, `StudioLayerProperty` (anchor/position/scale/rotation/opacity).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; push per green bundled run)

1. Panel and layer-bar view: WORKSPACE > Panels > Keyframe Timeline (Ctrl+Shift+F4); composition picker over the session's composition records; property tree column (layers -> the five transform streams) and time column with ruler, layer in/out bars, keyframe diamonds per stream; playhead shared with the program monitor scheduler when one is open (read/write via the existing Viewer field, not via timeline.rs), else a document-local playhead; STU-SHL-091 sizing (min 300 px, tree 320/220 px, time column >= 400 px else horizontal scroll); stable widget names `keyframe_timeline.*`; `state_json()["keyframe_timeline"]` lists composition, layers, streams, keys (time, value, interps), playhead. Tests: `keyframe_timeline_opens_from_the_workspace_menu_and_shortcut`, `accesskit_click_opens_the_keyframe_timeline_and_keys_are_nodes`, `keyframes_sit_at_the_pixels_of_their_ticks`, `narrow_panel_scrolls_instead_of_compressing_the_ruler`, `unavailable_without_compositions_names_the_reason`, `hostile_composition_records_lay_out_bounded`.
2. Keyframe operations (STU-MOT-038/039) as typed commands `Viewer::keyframe_command(KeyframeCommand)`, each one undo step: add/delete at current time (value = evaluated value, so the render is unchanged), enable/disable animation, select scopes, next/previous keyframe, shift 1/10 frames, drag in time with snapping (playhead, other keys, layer in/out), copy/paste/reverse paste, parent pick (layer `parent` field, cycle refused by Folio validation). Tests: `adding_a_key_keeps_the_evaluated_value`, `shift_and_drag_move_keys_and_undo_restores`, `reverse_paste_inverts_key_order`, `dragged_keys_snap_to_playhead_keys_and_layer_edges` (settle before aiming, F-TL-2), `parenting_a_layer_is_one_step_and_cycles_are_refused`.
3. Graph editor MODE (STU-MOT-050/051/052, temporal only): one toggle command switches layer-bar <-> graph over the same selection; value graph and speed graph from `hsk_studio_motion::graph`; per-component curves with stable colours; auto-fit / fit-all / manual range (`fit_range`); box select; 2D key drag with axis constraint; tangent handle drag with break/maintain continuity; frame-snap toggle; reference graph show/hide. Audio waveform behind the graph: NOT in this lane (declare gap). Tests: `graph_toggle_keeps_selection_and_operations`, `value_and_speed_curves_match_motion_samples`, `dragging_a_tangent_writes_the_velocity_record`, `box_select_and_axis_constrained_drag`.
4. Assistants and numeric records: Ease / Ease In / Ease Out / toggle hold (`assistants::ease`, `toggle_hold`), set interpolation per side, interpolation record and velocity record dialogs (`velocity_record`/`set_velocity`), all reachable as typed commands. Tests: `ease_assistants_write_the_spec_default_influence`, `velocity_dialog_round_trips_through_the_typed_record`, `hold_toggle_steps_the_curve`.
5. Create composition (STU-MOT-004 subset stored by Folio: name, width, height, pixel aspect, frame rate, duration, background): New Composition command in the panel, bounds from the STU-MOT-004 table, refusals typed. Tests: `new_composition_is_validated_and_selectable`, `composition_settings_out_of_bounds_are_refused`.

Manual: RUN-UI.md row + paragraph per slice (operator and model layer: command keys, typed I/O, undo semantics, failure codes).

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Headless egui + AccessKit exactly like `viewer_timeline.rs` (no `egui_kittest` on origin: 0 hits; adding it would be a dep request, not needed). Settle the layout (repeat frames until the panel rect is stable, at most 6) BEFORE any pointer event (F-TL-2). Assert through `state_json()` and the Folio records read back from the session, never through pixels alone.
- NOT_PROVEN (print `NOT_PROVEN:` lines, V4-03): real desktop window launch; on-screen composition preview at the playhead (the panel shows evaluated values, not frames); audio waveform behind the graph; After Effects behaviour of drag snapping priorities, curve colours and the Ctrl+Shift+F4 binding (UNVERIFIED, D5 oracle; the AE oracle covers evaluation, not UI).

## Bundled lane-check (one run per push; rule 4a = whole suites, so no `--tests` filter on the push run)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-ae-keyframe-timeline-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-ae-keyframe-timeline-1 --disk d -- cargo nextest run -p hsk-studio-harness-view -p hsk-studio-harness --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: harness-view (changed) and harness (RUN-UI.md plus its viewer_shell/viewer_panels suites iterate `Panel::ALL`). Rule 4b: no Folio record field is added or renamed -> package suite not required; say so in the lane file. Disk: D: (`--disk d`; R39 holds C:). Diagnostic re-runs of named tests only after a failure: `--tests <name,...>` on the same command; loop cap two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified behaviour, conflicts, deadline">

## Edges and dependencies

- New edge: `harness-view -> motion` (G2). No crates.io dependency.

## UNVERIFIED (D5 oracle session)

- AE timeline UI: keyframe snapping priorities, reverse-paste timing anchor, graph colours, Easy Ease influence (spec default 1/6 is used, AE 33.33 % UNVERIFIED per `assistants.rs:8`), speed-graph magnitude for unseparated properties (`graph.rs:10`).

## Risks and conflicts

- READY lanes SMB-HARNESS-SHELL-2, SMB-HARNESS-PANELS-1 and SMB-TIMELINE-UI-1 all edited `shell.rs`/`mod.rs`. If R39 returns a finding in those files, its remediation lands first; rebase, keep your hunks additive, re-run once. Never reorder or reformat existing arms.
- SMB-TIMELINE-TRIM-1 runs in parallel in `timeline.rs`: neither lane edits the other's files; the shared playhead is read through the program-monitor Viewer field only.
- The folio<->motion adapter duplicates the shape of composite's private converter (cite `composite/src/temporal/composition.rs:631`); keep it total and tested (`adapter_round_trips_every_interpolation_and_tangent`) so the two cannot drift silently; propose making composite's converter public in a later lane (not tonight: READY files).
- Undo: if session document entries are not undoable, a panel-local stack is a declared gap against [STU-MOT-038] "one reversible history step" at document level.
- D: is a spinning disk: the first harness-view desktop build may exceed 30 minutes.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes. A push still needs its green bundled run (common rule (3), no placeholder pins): while the run is queued or running at a deadline, commit locally and write a heartbeat (`phase`, local shas, run status, `date -u`) into the lane file instead of pushing an unverified commit.

</topic>
