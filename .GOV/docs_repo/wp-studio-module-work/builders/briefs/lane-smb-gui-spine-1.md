---
file_id: LANE-SMB-GUI-SPINE-1
file_kind: lane_brief
updated_at: 2026-11-07T15:58:42Z
wp: WP-KERNEL-STUDIO
lane: SMB-GUI-SPINE-1
lane_file: builders/lanes/hsk-studio-gui-spine-1.json
crates: hsk-studio-native-host, kumihimo-ui
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="First real panel on a real document: a layered .handshake in the asset-local Vulkan window beside a kumihimo-ui Layers panel; row select and eye visibility run through command ids as named undo steps, the canvas re-renders, and a tree file shows the revision; CPU-painted chrome, no harness-view edit">

## Target

Read first: `direction.blocker_decisions_20261107` in S/ledger.json (binding: O2, O4, O5, O6, O8, window_layer, layout_and_text, rf_01, spine_gate, gpu_contention), S/research/blockers-20261107-red-team.json attacks 10, 11, 12, 17 and 25, S/research/blockers-20261107-vulkan-engineer.json positions on the HostEvent queue and the scripted file, S/builders/briefs/lane-smb-vk-host-1.md with S/research/review-smb-vk-host-1-a7e966f1.json (answer demo_handshake), S/builders/briefs/lane-smb-ui-tree-1.md with S/research/review-smb-ui-tree-1-44ab8e16.json, S/gui-design/build-order.json (B1 to B5, B10), components.json (layer_row, group_header, panel_tab, panel_body, scrollbar, status_bar, viewer, empty_state), layouts.json (frame, presets.raster.right) and acceptance-checks.json. Where the red team and a position differ, the decisions record wins. Every code fact below is at origin 672a1804 (SMB-VK-HOST-1 landed on cf6788d6); P = handshake-studio-assets/crates; spec = K/.GOV/spec/master-spec-v02.229/spec-modules/.

## Gap, in plain words

The Vulkan window shows one still picture, and nothing in it can be clicked.
- The window builds its content once, before the event loop, and uploads it once: P/hsk-studio-native-host/src/window.rs:187 and :269-273; every later frame presents that same pane again (:370). The offscreen path does the same (src/offscreen.rs:36).
- HostEvent carries only resize, scale, focus, occlusion, close, redraw and destroy (src/event.rs:5-17). from_winit drops every pointer, wheel and key event (event.rs:62-76, `_ => return None` at :74).
- The document feature renders a document once and shows its pixels, nothing else (src/document.rs:21-49).
- No .handshake document reaches the window. The repo's demo.handshake is an edit fixture: its document.json (unzipped from P/hsk-studio-harness/tests/fixtures/demo.handshake) has working_space_rgb null, blending_space null and artboards [], and its only raster layer is visible false with a tile of format caller-selected-format. Viewer refuses it in view_snapshot at P/hsk-studio-harness-view/src/view/render.rs:412-415 (no_working_profile); with a profile it would next fail no_artboard (render.rs:377-381). The refusal is pinned by tests/document_window.rs:57-62. The native-host README cites render.rs:391 (README.md:32): that was the line at 7879e7e9; at 672a1804 it is :414.
- kumihimo-ui already has the layer row (P/kumihimo-ui/src/widgets.rs:171-206), the tab row (:107-131), the retained Panel (src/tree.rs:466-546), Interaction (tree.rs:389-451), the hit test (src/hit.rs:7-28), the semantic tree (src/semantic.rs:77-124, JSON at :184) and the CPU painter (src/paint.rs:149). No host builds a panel from a document, and no host feeds it input.

What does reach the window (read in code and tests, not run by me): a document made by SMB-DOC-NEW-1 has a working profile (P/hsk-studio-harness-core/src/document_new.rs:323-329 sets working_space_rgb and blending_space to one SCPF id; the raster profile adds one empty Background layer, :313-319) and renders (P/hsk-studio-harness/tests/document_new.rs:477-490). But Viewer::new_document takes harness-core's NewDocument (harness-view src/view/document_new.rs:24), and native-host cannot name it: harness-core is not on its allow list (root Cargo.toml:318-319). An imported PSD opens, renders, toggles, saves as .handshake and reopens through public Viewer methods only (P/hsk-studio-harness/tests/viewer_headless.rs:33-73). So this lane's fixture is a small layered PSD assembled in the test (pattern P/hsk-studio-harness/tests/common/mod.rs:53-112, independent byte assembly), opened with Viewer::open, saved with Viewer::save as .handshake, and reopened from that .handshake. Do not edit demo.handshake: harness_drive, harness_headless and action_batch use it as the revision-9 edit fixture.

## Goal

In the asset-local Vulkan window: a real .handshake document with three or more visible layers draws in a Viewport region; a Layers panel built with kumihimo-ui stands in a right dock beside it; a click on a row selects that layer; a click on the eye hides or shows it as one named undo step; the canvas re-renders; and the model path (a kumihimo.ui.tree@1 JSON file) shows the new revision. Chrome stays the CPU painter (decision O6), stated in the window title, the status bar and the receipt as `cpu painter, Vulkan UI pass pending` (src/scene.rs:8; HBR-HOST-001: a software path never passes a Vulkan obligation). It is presented by the existing copy and blit (P/hsk-studio-vulkan/src/present.rs:356-365). No harness-view edit. Recorded deviation: this panel starts before STU-SHL-110 steps SHL-P-09 and SHL-P-10 pass (spec 14-studio-creative-suite.md:32387), under the Operator's working-GUI priority (red team attack 17, decision spine_gate).

## Build-order cut (gui-design/build-order.json)

- B2 IN, as the existing CPU primitive pass: the eleven prims (kumihimo-ui src/display.rs:33-61), edge snapping in one function (src/geom.rs:128, Scale::snap) and paint.rs. The panel is wrong without shared edges at fractional scales, and that pass already exists. The Vulkan pipeline version of B2 waits for the next slice.
- B3 IN, reused, not built: hsk-studio-ui-text TypeText over Inter (P/hsk-studio-ui-text/src/lib.rs:34-62; the font files of tests/text_sample.rs:19-27, both SIL OFL 1.1), with kumihimo-ui's Text caches and ellipsis (src/text.rs:102-252). Icons stay the gallery stand-in (src/gallery.rs:37-66, ICON_EYE at :14). The designer's icon set, the fallback chain and the glyph atlas wait.
- B5 PARTIAL: only the bands this panel needs. The Viewport (centre); a right dock 300 wide (tokens size::DOCK_INSPECT_W, src/tokens.rs:486; layouts.json presets.raster.right.w) holding one group: group_header 26 (size::DOCK_GROUP_HEADER_H, tokens.rs:491) with panel_tab Layers, then panel_body; a status bar 24 (size::FRAME_STATUS_BAR_H, tokens.rs:472). Menu bar, tab strip, Context Bar, Tool Rail and the other regions are the next B5 slice. Positions come from computed layout, so those bands can be added later without moving these.

## Ownership and input path

- kumihimo-ui: widgets, layout, display list, painter, hit test, Interaction and the semantic tree (decision O4), plus one additive passive scrollbar widget (components.json scrollbar: lane 10, thumb 6, min_thumb 24, radius 3). No hsk-studio-* dependency (O5).
- hsk-studio-native-host, feature `document`: the frame (regions, one Panel per region, a Resources source for the canvas image and icons, the text port), the Layers view function over Viewer::layers() rows, the author_id to command table, and the GUI loop in window.rs and offscreen.rs. Keep the frame state free of winit and Vulkan types, so it runs and tests on the CPU alone.
- hsk-studio-vulkan: unchanged (panes and presenter). harness-view: linked, not edited.
- Input: winit WindowEvent -> event::from_winit -> HostEvent (new variants) -> App::window_event -> frame router (physical px divided by scale gives logical px) -> Interaction::pointer_move, pointer_down and pointer_up on the panel under the pointer -> the released author_id -> command id -> Viewer -> state_rev + 1 -> Panel::update -> paint -> upload_pane -> Presenter::frame.

## Items, red first

1. RED, one compiling commit whose new tests fail: tests under crates/hsk-studio-native-host/tests (feature document) for items 2 to 8, against an API skeleton that returns a typed refusal. Put the native-host Cargo.toml, root Cargo.toml and Cargo.lock edits in this first commit: lane-check treats a Cargo.lock change as every member (review a7e966f1, cargo_lock), so the whole-workspace per-sha check runs once.
2. Fixture and document: test support assembles a layered PSD with at least three opaque layers of distinct colours at distinct rects plus one hidden layer, opens it with Viewer::open (harness-view mod.rs:523), saves it with Viewer::save (:737) and reopens the .handshake. Assert render_error is None and layers() lists them top-first (mod.rs:621-654). Map an open refusal to the viewer's own code, not `open` (document.rs:23 drops it).
3. Typed input: HostEvent gains pointer move, pointer leave, button press and release, wheel, and key with modifiers. Positions are physical px as i32, so HostEvent stays Copy and PartialEq and WindowArgs keeps its Eq derive (window.rs:30). Map winit 0.30.13 CursorMoved, CursorLeft, MouseInput, MouseWheel, KeyboardInput and ModifiersChanged (registry winit-0.30.13 src/event.rs:242-249, :272, :278, :275, :205, :222). Coalesce pointer moves to one per loop iteration and keep them out of EventLog (event.rs:33-60 keeps 1024). Tests feed a scripted HostEvent list into the frame and into a quiet window run, as the Vulkan engineer's position puts it (a typed queue reachable by a scripted file): never OS input (STU-QUIET-002, spec 14:22597; HBR-QUIET-002).
4. Frame and Layers panel: the frame owns the Viewer, one Text, one Interaction per window (lead F7 ruling, tree.rs:385-388) and one Panel per region. Lay out at the client extent and scale (Scale::new refuses anything outside 0.5..=8, geom.rs:115); paint into one window-size Canvas (paint.rs:21). The Viewport draws Prim::Checker under Prim::Image of Frame.rgba8 (paint.rs:275-307, straight alpha), 1:1 at scale 1, centred and clipped; zoom and pan are B11. Rows are widgets::layer_row with author_id studio.panel.layers.row.<layer-id> (components.json layer_row; STU-MDL-100 grammar, spec 14:32330). Lowercase the layer id: valid_author_id takes [a-z0-9-] only (semantic.rs:55-74), Folio ids are SLYR-<uuid>, and accord DomainId accepts upper-case hex (P/hsk-studio-accord/src/lib.rs:263-275). A lowercase collision is the Builder's typed DuplicateId (tree.rs:337-338), shown as an empty state with its code, never a panic. Hidden: no eye glyph, the name in text.disabled (widgets.rs:199), value hidden (AC-COL-06: never colour alone). Selected: the row's selected state (widgets.rs:179). Group header: tab_row studio.group.layers with tab studio.tab.layers (widgets.rs:109). Status bar studio.status with readouts for the device and backend (AC-BEHAV-07; DeviceInfo from the Owner), the document revision and the chrome statement. Virtualise: build only the rows that fit the body, from a scroll offset (rows are a fixed 32, size::ROW_LAYER_H, tokens.rs:510; the GPUI uniform_list pattern). The list node's value states the first, last and total rows, so the tree never hides rows silently (STU-MDL-108, spec 14:32379). The wheel over the list scrolls it (STU-SHL-105, spec 14:32308), and the passive scrollbar shows when the list overflows.
5. Commands: a click released on `.visibility` runs layer.set_visible {layer, visible}; a click on the row runs viewer.select {layer}; a click on blank list space runs viewer.select with no layer; Ctrl+Z runs history.undo; Ctrl+Shift+Z and Ctrl+Y run history.redo (chords PROVISIONAL, components.json keyboard). These are the drive surface's own mappings onto public Viewer methods (P/hsk-studio-harness/src/cli/drive.rs:186-195, drive_viewer.rs:225-226): Viewer::set_visible (harness-view mod.rs:658-679), Viewer::select (:864-866), Viewer::undo and Viewer::redo (edit.rs:650-659). Viewer::run_command refuses these rows (commands.rs:81 takes Surface::Viewer rows only), so do not route them there. The same table answers an invoke-by-author_id action (STU-MDL-109, spec 14:32381: the command API plus the tree by author_id, no OS input), exposed as a repeatable CLI flag for offscreen runs; an unknown author_id is a typed refusal. After each command: render_if_dirty (mod.rs:776; no GPU backend with Prefer::Cpu, :784); read the selection and revision once from state_json (mod.rs:869-893, selected at :880; there is no getter); bump state_rev; repaint and upload once. A refused command is not fatal: the status bar shows its code. Run names the undo step: label `panel_set visible <layer_id>` with command id layer.panel_set (harness-core src/harness.rs:1196-1219, history at :1911-1918). Assert both.
6. Window GUI mode: a scene value for the frame in cli.rs:97-103 (name it from existing words). On Resized, re-create the pane at the client extent, releasing first (64-pane cap, P/hsk-studio-vulkan/src/owner.rs:783-785). Resized 0x0 skips layout, paint and create_pane: create_pane(0, 0) is InvalidExtent (refusal.rs:102-104) and is fatal in the loop. ScaleFactor relays out. Idle frames rebuild and upload nothing (Panel::builds, tree.rs:482-484; AC-LOOK-04). The Operator's window waits for events (ControlFlow::Wait, registry winit-0.30.13 src/event_loop.rs:156-174; window.rs:282 forces Poll today), and its 30 s default deadline (cli.rs:64, checked at window.rs:361) must not close it early: give frames 0 a long cap. Set a minimum inner size so the Viewport keeps 360 x 240 (components.json viewer.size.min; registry winit-0.30.13 src/window.rs:212 with_min_inner_size). Quiet rules stay as they are (window.rs:253-262 make_quiet, keep_quiet :287-297).
7. Model path and receipts: write the merged semantic tree (one kumihimo.ui.tree@1 document whose roots are the region panels; semantic.rs:30-35, :184) to a tree file with the document revision. The receipt gains a gui member: revision, rows shown and total, builds, uploads, and the last command with its code. capture.rs Json has no raw variant (capture.rs:40-48), so the tree goes in its own file.
8. Proof: an offscreen GUI PNG with no HWND equals the quiet-window swapchain capture hash, with focus_changes 0 and assert_quiet (pattern tests/host_vulkan.rs:35-81). A scripted session: an eye click gives revision + 1, changed canvas pixels inside that layer's rect, and row value hidden; undo restores the earlier frame hash; redo; select, deselect, reselect gives byte-identical tree JSON (AC-BEHAV-05, one of the 8 spine-gate tests). Tree checks from kumihimo-ui src/checks.rs on the built trees: AC-GEOM-02 (:9), AC-TEXT-01 (:34), AC-TEXT-03 (:76), and AC-GEOM-03 edge_gaps (:88) for Viewport|dock and header|body offscreen at scales 1, 1.25, 1.5 and 2. AC-GEOM-01: the region rects cover the window with no overlap. AC-COL-01: chrome pixels outside the Viewport are theme tokens. AC-GEOM-04: row pitch 32. Add a source scan that native-host calls no focus or input API (focus_window, set_cursor_grab, set_cursor_position, SetForegroundWindow, SetFocus, SendInput). Send the lead the PNG paths under Handshake_Artifacts/WP-KERNEL-STUDIO/smb-gui-spine-1/ (a binary run through studio_gpu_run), with a copy of the fixture .handshake there for the Operator.

Commit plan: about 8 (red; input; fixture; frame and panel; commands; window mode; tree and receipts; proof and README). Message the lead with the tip before the bundled run.

## V4-06 sweep: siblings of the same failure class, fixed or named

- Content built once and never refreshed: window.rs:187 and offscreen.rs:36 (fix both for the GUI scene).
- An open refusal flattened to `open`: document.rs:23 (fix).
- Heavy state_json used for a count (document.rs:40) and for the selection (name it; a Viewer getter belongs to a later harness-view lane).
- Stale line cites: README.md:32 and the lane file's blocker say render.rs:391, now :414 (cite the function, not the line).
- Quadratic Viewer::layers(): mod.rs:626-651 runs layers::stack (layers.rs:32-47) and two linear scans for every layer (call it once per revision; name it for harness-view).
- Canvas::new returns None at 0 or above 16384 (paint.rs:17-24), where gallery.rs:137 uses expect (never expect in the host).
- checks::overlapping_controls is O(n squared) (checks.rs:9-23): run it on the visible rows only.

## Hostile input: bounded work and allocation, typed refusals, no panic

- (a) 10,000 layers. Folio refuses a document past Budget.nodes 4096, which counts artboards + spreads + layers + graph operations (P/hsk-studio-folio/src/lib.rs:546 and :631-641, BudgetExceeded). PSD import refuses past max_layers 8192 (P/hsk-studio-interop-psd/src/limits.rs:27). Assert that the host shows that typed refusal. Also run the Layers view on 10,000 plain rows with no Viewer: the nodes built stay under a fixed bound (rows that fit the body, far below MAX_NODES 65,536 at tree.rs:246), no row off screen is shaped, and the build finishes within a stated time.
- (b) A 4096-byte layer name, set with Viewer::rename_layer (layers.rs:230). The row ellipsizes (text.rs:229-252; MAX_LABEL_BYTES 4096 at :99), AC-TEXT-01 stays clean, the accessible label carries the full name (AC-TEXT-02), and shape misses grow by about log2(n) (fix F9). A 1 MiB name cuts at 4096 bytes.
- (c) Zero-size window: Resized 0x0, then restore (item 6). No Canvas::new(0), no create_pane(0, 0), and the next frame hash equals the one before the minimise.
- (d) Device lost: the paint-and-upload step takes `&mut impl GpuClientPort` (owner.rs:103-115), so a fake port that returns Refusal::DeviceLost proves a typed exit 3 (error.rs:47), no Viewer command after it, and no retry loop. A real device loss stays NOT_PROVEN (review a7e966f1, not_checked).
- Also: a scale outside 0.5..=8 is refused or clamped with a code; a pointer outside the client area hits nothing; a scripted event list over a stated cap is refused.

## MT rows to read

Under K/.GOV/task_packets/WP-KERNEL-STUDIO/, read with python; claim none, because this is an asset-local host:
- MT-34514: STU-SHL-078 spine.
- MT-34562: STU-SHL-113 Layers row (panel_id layers, class always_relevant, Inspect Dock).
- MT-34654: STU-SHL-110 chain (the deviation above).
- MT-34640: STU-SHL-109 (the gate's seven tests).
- MT-34646: STU-MDL-105 numeric node fields.
- MT-34496: STU-SHL-061 anti-thrash.
- MT-36250: pane workbench proof; a host MT, so an asset-local half at most (red team attack 2).

## Spec clauses

- 15-native-desktop-gui-vulkan-gpu-ownership.md: GUI-DESKTOP-007 (:36), -009 (:40), -010 (:42), -012 (:46), -020 (:66), -023 (:72), -024 (:74), -041 (:112).
- 14-studio-creative-suite.md: STU-ARC-024 (:259), STU-QUIET-001, -002, -003, -005 (:22595, :22597, :22599, :22603), STU-SHL-078 (:32066), STU-SHL-101 (:32300), STU-SHL-104 (:32306), STU-SHL-105 (:32308), STU-SHL-109 (:32316), STU-MDL-100 (:32330), STU-MDL-101 (:32338), STU-MDL-108 (:32379), STU-MDL-109 (:32381), STU-SHL-110 (:32387).

## host_integration

- gui_host: none (asset-local host).
- gpu: the existing transient owner; no new device.
- diagnostics: producer_through_observe. Each command reaches a Run operation that emits its own observe frame, and the host keeps its terminal frame (src/diag.rs:24).
- attribution: the Viewer's asset-local test context (Viewer::new, harness-view mod.rs:303-305), recorded on each undo step.
- control_descriptors: none (B6).
- refactor_exposure: high.

## Out of scope

- Any harness-view, harness-core, harness, folio or session edit. That includes widening Viewer::run_command and adding Viewer::canvas_event: there is no canvas input in this slice.
- The Vulkan UI pass: shaders, pipelines, descriptors, the SPIR-V tool, the glyph atlas.
- Menus and the rest of B5 and B6; docking, splitters and drag (B9); ScrubValue and gestures (B7, STU-SHL-109 tests 1 to 7).
- Layers rename, reorder, group, lock, opacity and thumbnails.
- Canvas zoom, pan and tools (B11); File Open, Save and New Document in the GUI.
- AccessKit adapter, IME, pen, multiwindow, off-thread render, owner rebind after device loss.
- demo.handshake.
- Any new registry dependency, Taffy, egui, eframe, wgpu, naga or WGSL.
- No Master Spec edit. Never touch WP-KERNEL-012 files or refund-claim-20261021.md.

6 to 10 commits.

LEAD RULINGS (2026-11-07T15:58:41Z):
- Q1: no copied dispatch table. The host calls the Viewer's public methods directly (select, set_visible and the named undo step as the scope describes); widening Viewer::run_command to accept layer.set_visible is a later harness-view slice, because other lanes edit harness-view now.
- Q2: approved. Root Cargo.toml line 319 adds hsk-studio-ui-text to native-host's allow list, plus the matching Cargo.lock dependency-list line (no new package). It keeps the rule that no kumihimo-* crate has an hsk-studio-* dependency.
- Q3: the panel code is a hsk-studio-native-host module (feature document) now; a leaf crate comes at fan-out.
- winit: lines inside hsk-studio-native-host are allowed (direction.blocker_decisions_20261107 window_layer). The general 'no new winit line' text below applies outside that crate.
- Captures: write the offscreen PNG of the final frame to Handshake_Artifacts/WP-KERNEL-STUDIO/smb-gui-spine-1/ and name it in your report; the lead shows it to the Operator.
- The window stays quiet (vk-host rules): no focus steal, no foreground pop, no input hijack.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: every panel command reaches a Run operation that emits its own observe frame (harness-core); the host keeps its terminal frame (native-host src/diag.rs:24) and adds codes and counters to the receipt, never document text", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "the Viewer's run context: Viewer::new on the asset-local test host context (harness-view src/view/mod.rs:303-305), carried by every undo step (Run::history undo_steps, harness-core src/harness.rs:1911-1918); the host mints no actor", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-entry-references-1`, branch `lane/smb-gui-spine-1`, base e85e1440)

crates/hsk-studio-native-host (src, tests, README.md, Cargo.toml: optional hsk-studio-ui-text under feature document), crates/kumihimo-ui/src/widgets.rs and its tests (one additive passive scrollbar widget, no golden change), root Cargo.toml line 319 (allowed list of hsk-studio-native-host gains hsk-studio-ui-text, lead-approved), Cargo.lock dependency list of hsk-studio-native-host only. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-entry-references-1 --disk d`): step 0; hsk-studio-native-host --features document (GPU test group, .config/nextest.toml:31-32), kumihimo-ui, hsk-studio-ui-text (a dependent of kumihimo-ui); harness-view and hsk-studio-vulkan are linked, not edited; one whole-workspace per-sha for the Cargo.lock commit; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-entry-references-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
