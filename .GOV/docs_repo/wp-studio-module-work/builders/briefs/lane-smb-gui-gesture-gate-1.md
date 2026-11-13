---
file_id: LANE-SMB-GUI-GESTURE-GATE-1
file_kind: lane_brief
updated_at: 2026-11-13T17:08:44Z
wp: WP-KERNEL-STUDIO
lane: SMB-GUI-GESTURE-GATE-1
lane_file: builders/lanes/hsk-studio-gui-gesture-gate-1.json
crates: native-host, kumihimo-ui
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The seven STU-SHL-109 arbitration tests exist and pass in the GUI frame; the host clock feeds GuiFrame::advance; the gesture step carries the host run's actor">

## Target

Pointer gesture arbitration in the GUI frame with the STU-SHL-109 acceptance tests #1 to #7, the host clock feeding GuiFrame::advance, and the host run's actor reaching the GUI viewer (hsk-studio-native-host [document])

Why this lane: ledger PRIORITY candidate 2026-11-13T08:33:54Z (F-R60-43, validation/round-R60.json attempt_3.findings_leads), verbatim: "PRIORITY R60-a3 lead: SMB-GUI-SPINE-1 spine gate tests 1 to 7 (STU-SHL-109 #1-#7) do not exist in any unit; only #8 (AC-BEHAV-05 byte-identical reselect, native-host tests/gui_frame.rs:293) exists, so GATE_CHECK cannot pass at b57c82d1; write the 7 tests before a GATE_CHECK (F-R60-43, round-R60.json attempt_3.findings_leads)".
Folded in because the gesture needs them: candidate 2026-11-13T09:27:50Z, verbatim: "tip clock: window.rs does not feed GuiFrame::advance, so dwell tooltips appear only through a host clock or GuiFrame::show_tip" (the wheel idle window of STU-SHL-107 needs the same clock); candidate 2026-11-13T15:54:30Z, verbatim: "native-host document scene (document.rs:49 Viewer::new) and gui scene (GuiFrame::open via open_viewer): the Viewer runs on the fixed asset-local-test-host context, so the caller of a host run (HostRun actor) does not reach the viewer; named V4-06 site of SMB-FIX-HOST-TERMINAL-FRAME-1" (the gesture's one history step is a new mutating op and must carry the caller's actor, HOST INTEGRATION (4)).
Lead blocker decision spine_gate (ledger direction.blocker_decisions_20261107.spine_gate): "Panel fan-out opens on a fresh independent agent running the 8 named tests at the tip, recorded as GATE_CHECK, never PASS_Vn. Builder-green is not acceptance." This lane writes #1 to #7 and makes them pass; it never claims the GATE_CHECK.

Facts are READ by the wave-next-2 planner at origin 89f27330 (read-only, not run; `git diff a77c4ee4 89f27330` touches only harness-view src/view/menu_tree.rs). Treat them as claims and check them.
- Spec: STU-SHL-100..108 (rules 1 to 8, module 14 :32298-32314) and STU-SHL-109 (:32316, the seven tests); STU-MDL-106 dock state (:32369); STU-SHL-185 one numeric widget (:33232); STU-SHL-235 tooltip anti-annoyance (:33452, rule 5: no script step feeds the dwell clock). MTs MT-34631..MT-34640 carry STU-SHL-100..109.
- `native-host/src/gui/mod.rs:882` GuiFrame::apply(HostEvent): PointerMoved `:901`, Left press `:912`, Left release `:934`, Wheel `:962`/`:967`, Key `:978`, Modifiers `:979`. No value field, no gesture owner and no studio.gesture.active node exist (git grep gesture in native-host src: none).
- `gui/mod.rs:1193` dock_state() is studio.dock.state, drawn unpainted by `gui/chrome.rs:506-508` observe(). Spine gate test 8: `tests/gui_frame.rs:291-293` select_deselect_reselect_tree_is_byte_identical (unit features:hsk-studio-native-host+document).
- `gui/mod.rs:2532` GuiFrame::advance(ms) and `:2595` show_tip. `src/window.rs` times frames with its own Instant (`:271`, `:349`, `:420-443`) and its own Phase advance (`:499`); it never calls GuiFrame::advance.
- `src/document.rs:45-50` open_viewer builds `Viewer::new(Prefer::Cpu)`; `gui/mod.rs:583-585` GuiFrame::open calls it. harness-view has `Viewer::with_context(prefer, gpu_options, RunContext)` (`view/mod.rs:344`). The host run's actor: `src/diag.rs:51` HostRun, `src/cli.rs:43` actor.
- A layer value commit exists: catalog id `layer.set_opacity` (harness-core `action/commands/layer.rs:8`; harness-view `menu_tree.rs:970` palette_only MenuPathPending, not a menu leaf); `Viewer::set_opacity` (`view/edit.rs:639-642`) is one undoable chronicle edit per call through `Run::set_opacity(layer, percent, None)`.
- Controls already model gesture transactions: hsk-studio-controls `store.rs:58` GesturePhase, `:85` Interaction::gesture, `:125` gesture_active (tests gesture_panel_report.rs, step_effect.rs:355-366 abort).
- kumihimo-ui `widgets.rs:134` slider: label plus a 72 x 24 ScrubValue field plus rail (design-system.json:2349, STU-SHL-185).

Red test (must fail first):
`native-host/tests/gui_gesture_gate.rs` (new, `#![cfg(feature = "document")]`), one test per STU-SHL-109 item, driving GuiFrame through HostEvent steps only (no winit, no OS cursor):
1. press in a value field, move across a group tab strip and a dock splitter, release: studio.dock.state byte-identical before and after; studio.gesture.active reads `scrub` with that field's author_id at every sample;
2. press, drag past the window edge, release: the value kept changing; the frame's cursor request is hidden during the drag and restored at the press point on release; no warp request during the drag;
3. press, drag, Escape: the value equals the pre-press value and zero history entries were written;
4. 500 pointer samples: exactly one history entry;
5. five wheel notches inside the idle window, then GuiFrame::advance past 400 ms: exactly one history entry;
6. accessibility SetValue on the same field: one history entry and the identical clamped result as the drag;
7. every timed gesture the frame offers (at least the hover-dwell tooltip) has an untimed equivalent reachable from the menu and the palette; the test enumerates the frame's timed gestures, not a hand list.
Plus: a window run feeds GuiFrame::advance (a dwell tip appears without show_tip), and the gesture's history step carries the host run's actor, not asset-local-test-host. They fail at the base (no field, no arbiter).

Fix contract:
- FIRST: send the lead the shape before coding: where the arbiter lives (native-host gui, or kumihimo-ui only if it must be shared), the first ScrubValue field (proposal: the selected layer's opacity in the Layers panel, committed once per gesture as catalog id layer.set_opacity), how the in-gesture value is shown without a history entry, how the host clock reaches GuiFrame::advance, and how GuiFrame::open takes the caller's context (Viewer::with_context). Report it as a blocker message and wait.
- Rule 1 claim at press: while a widget owns the pointer, no other widget gets hover, drop, tooltip, auto-scroll, dock resize or pan (STU-SHL-101, -104). Panel and tab drags start from chrome only (STU-SHL-102).
- One history entry per gesture, named by its catalog id (common ruling 10); Escape and a refused commit write none; SetValue takes the identical clamp and commit path (STU-SHL-107).
- Modifiers per STU-SHL-106: Shift x10, Ctrl /10, Shift+Ctrl x100, Alt never a magnitude; sampled continuously; accumulation in value space.
- The screen edge is a typed cursor request (hide; show at the press point) the host applies; no OS cursor warp (STU-SHL-104).
- studio.gesture.active is a semantic tree node, value `<author_id> <kind>` with kind from {scrub, tab-drag, panel-drag, splitter, pan, none} (STU-SHL-108).
- No catalog command is added (layer.set_opacity exists). No new crate edge. No new egui, eframe, winit, wgpu, naga or WGSL line; winit stays where it is (event.rs, window.rs).
- 3 to 6 commits; slices: actor context, host clock, arbiter plus field, tests 1 to 7.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-r53-followups-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-native-host --features document --test gui_gesture_gate` (add `--test gui_frame --test gui_menu --test gui_chrome` when GuiFrame::open or apply changes; `-p kumihimo-ui` only if touched). Bundled: `lane-check.sh --owner smb-r53-followups-1 --disk d -- cargo nextest run -p hsk-studio-native-host --features document` plus the default-features native-host suite. GPU tests wait for the GPU test lock.
MENU COVERAGE: this lane adds or renames no catalog command; it runs native-host [document] gui_menu (MENU-INV-1) at landing anyway because it changes GuiFrame, and says so in its reports.

Out of scope: docking and panel drags beyond the rule-2 refusal, IME, pen, the command palette (STU-SHL-041..044), scrollbar drag and keyboard row focus (candidate 2026-11-07T17:49:57Z), the F6 region cycle (candidate 2026-11-07T13:35:06Z), menu_tree.rs edits, any harness-view edit without asking the lead (name file, line and why; view/mod.rs and view/commands.rs belong to SMB-FIX-LAYER-RECORD-PRUNE-1), any Cargo.lock change without asking.
Named out of scope: F-R60-32 (shader hash and validation asserts), F-R60-28 (kumihimo-ui shaping refusal counters), the GATE_CHECK itself.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PROTOTYPE-PLAY-CANCEL-1, 2 SMB-PR-TIMELINE-FIXES-1, 3 SMB-MOTION-IMPLIED-VALUES-1, 4 SMB-GUI-GESTURE-GATE-1; the lead may interleave SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 4 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): STUDIO-R60 attempt 4 runs on the D: round slot and holds the D: lock (class round, FIFO priority) for many hours. Your lock-needing steps queue behind it: that is expected. Start the lock-needing step in the background and wait to be notified. Never work around the lock, never poll in a loop, and never run cargo outside a hold. Read code and edit while you wait. ONE HOLD PER PHASE (wave-next-2, 2026-11-13): the four lanes share the D: lock with each other and with the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Lanes land one at a time on the lead's release.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: the gesture's one commit goes through the Viewer's run (one step, its existing frame); the frame receipt counts gestures, aborts and refusals; codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the gesture's history step carries the host run's actor (GuiFrame::open from the caller's context, Viewer::with_context); never asset-local-test-host or a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r53-followups-1`, branch `lane/smb-gui-gesture-gate-1`, base 89f27330)

hsk-studio-native-host/src/gui/mod.rs (apply, the value field, open); a new hsk-studio-native-host/src/gui/gesture.rs (placeholder name); src/gui/chrome.rs (studio.gesture.active node); src/window.rs (host clock to GuiFrame::advance, cursor requests); src/document.rs (open_viewer with the caller's context); src/event.rs (only a cursor request type, if needed); kumihimo-ui/src/widgets.rs (only with the lead's approval); hsk-studio-native-host/tests/gui_gesture_gate.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-r53-followups-1 --disk d`): step 0; native-host suite, default features and [document] (round unit features:hsk-studio-native-host+document); kumihimo-ui suite if touched; gui_menu MENU-INV-1 at landing; GPU tests wait for the GPU test lock; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-r53-followups-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
