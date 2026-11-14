---
file_id: LANE-SMB-MENU-EFFECT-BOARD-LEAVES-1
file_kind: lane_brief
updated_at: 2026-11-14T08:35:55Z
wp: WP-KERNEL-STUDIO
lane: SMB-MENU-EFFECT-BOARD-LEAVES-1
lane_file: builders/lanes/hsk-studio-menu-effect-board-leaves-1.json
crates: harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Two menu leaves run their catalog commands (Disable All Effects -> effect.toggle, New Board -> board.add behind a board-container gate), and a canceled command is recorded as canceled, not command_refused">

## Target

Two designed menu leaves run their catalog commands, and a canceled command's outcome is recorded as canceled (hsk-studio-harness-view view/menu_tree.rs, view/commands.rs)

Why this lane: ledger candidate 2026-11-13T16:46:23Z, verbatim: "EFFECTS > Effect Stack > Disable All Effects -> effect.toggle {layer: selected, enabled: false} (no instance: the whole stack): needs a run_catalog arm through Viewer::run_command and a Rule; the label-to-command mapping is a reading (STU-FX-011a), not spec-given". Ledger candidate 2026-11-13T16:46:23Z, verbatim: "DOCUMENT > Boards > New Board -> board.add (all parameters optional): needs a board-container gate in Profile::of (whiteboard mode derives the board container) so the leaf is available only where board commands run". Ledger candidate 2026-11-14T07:44:47Z, verbatim: "F5 (review 2adb7501 of SMB-PROTOTYPE-PLAY-CANCEL-1): harness-view view/commands.rs:143 run_command records every refusal as command_refused, so a canceled command (prototype.start_session, a no-path capture) is recorded as a validation failure; pass Err("canceled") when the refusal code is canceled (pre-existing)". The first two were held in wave-next-2 because view/commands.rs and the board files sat in SMB-FIX-LAYER-RECORD-PRUNE-1's paths; that lane is READY_FOR_VALIDATION at 198f6dc4 (on origin), so they are unblocked. They make two GUI menu items work.

Facts are READ by the wave-next-3 planner at origin 5655345b (read-only, not run). Treat them as claims and check them.
- Spec: EFFECTS > Effect Stack lists Disable All Effects (:31739); DOCUMENT > Boards lists New Board (:31648); [STU-FX-011a] (:19423); [STU-SHL-013] (:31525) every catalog command has a menu leaf or a typed palette_only reason.
- `harness-view/src/view/menu_tree.rs`: designed nodes `studio.effects.effect_stack.disable_all_effects` (:2659) and `studio.document.boards.new_board` (:1963); `Rule` (:483-502); `LEAVES` (:520-535); `leaf_params` (:613-660); `CATALOG_COMMANDS` (:668-669, a [&str; 10]); `declared` (:672); `run_catalog` (:683-745); `PALETTE_ONLY` rows `("board.add", Unreached::MenuPathPending)` (:797) and `("effect.toggle", Unreached::MenuPathPending)` (:936); `ContainerKind::Board` exists (:75) and `Profile::of` (:303-325) says "Board and site tree containers have no document record here yet (never derived)"; `MenuState::of` (:384).
- Catalog rows: harness-core `action/commands/board.rs:18` board.add (name, width, height, x, y all optional); `action/commands/photo.rs:108` effect.toggle (layer, instance optional, enabled).
- `view/raster_effects.rs:1135-1150` effect.toggle: no instance toggles the whole stack's `enabled`; refusals `effect_instance_unknown` (the layer has no smart filter stack) and nothing_to_change.
- `view/board.rs:334` whiteboard_only refuses `not_whiteboard_document`; `:453` board_ctx reads `self.run.whiteboard_mode()`; `:590` board_add; `view/commands/board.rs:71` decodes board.add.
- `view/commands.rs:134-157` Viewer::run_command: when no op was recorded, `Err(_) => Err("command_refused")` (:144) goes to `Run::command_outcome` (harness-core harness.rs:683).
- Tests that enumerate leaves: harness tests/harness_drive_menu.rs:152 `drive_and_the_native_menu_path_agree_on_every_leaf` (spawns the native host binary); native-host tests/gui_menu.rs:683 MENU-INV-1. Existing command tests: harness tests/board.rs (:102, :121 not_whiteboard_document), tests/viewer_raster_effects.rs (:129-134, :233-234).

Red tests (must fail first):
crates/hsk-studio-harness/tests/menu_effect_board_leaves.rs (new, [viewer]): (1) on a raster layer with a smart filter stack selected, `run_menu_leaf` of studio.effects.effect_stack.disable_all_effects disables the whole stack in ONE undo step named effect.toggle; undo restores it; with no selection the leaf is unavailable with the selection reason; on a layer without a stack the command's own refusal effect_instance_unknown comes back; (2) in a whiteboard document studio.document.boards.new_board is available and adds a board in ONE undo step named board.add; in a non-whiteboard document the leaf is UNAVAILABLE with the container reason (the gate), it does not run into not_whiteboard_document; `Profile::of` holds the board container exactly when whiteboard mode is on, and it survives save and reopen; (3) both commands are leaves, so neither is in PALETTE_ONLY and `reached_commands()` holds both; (4) F5: a command refused `canceled` with no op recorded (a canceled token before prototype.start_session, or a no-path capture) records outcome canceled, and another refusal still records command_refused. They fail at the base (both leaves are NOT_IMPLEMENTED_YET; every refusal is command_refused).

Fix contract:
- Two `Rule` variants (or one reused where it fits), two LEAVES rows with their `requires` (selection for the effect leaf; `container:board` for New Board), two run_catalog arms through the commands' existing Viewer paths (no second implementation of either command), CATALOG_COMMANDS grown, the two PALETTE_ONLY rows removed.
- `Profile::of` derives `ContainerKind::Board` from whiteboard mode (`Run::whiteboard_mode`); a read error keeps the container absent (no panic, no refusal from the profile).
- F5: only the outcome code passed to `command_outcome` changes for a `canceled` refusal; results and other refusals stay byte-identical.
- The label-to-command mapping of Disable All Effects is a reading (STU-FX-011a): record it in the lane notes as a reading, not as spec-given.
- 1 to 3 commits after the red commit.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red tests above, committed compiling and run red at that COMMITTED tip. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix, committed, and the focused tests run at that committed tip. Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip; do not run on uncommitted work and commit afterwards.
4. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
5. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
6. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13 and 2026-11-14 "avoid large cargo builds, reuse targets"): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-pr-entry-references-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-harness --features viewer --test menu_effect_board_leaves --test board --test viewer_raster_effects`. Bundled: the harness-view [viewer] suite; harness [viewer] --test menu_effect_board_leaves --test board --test viewer_raster_effects --test harness_drive_menu; native-host [document] --test gui_menu (send the list with REVIEW READY).
MENU COVERAGE: this lane adds or renames no catalog command, but it moves two commands from PALETTE_ONLY to leaves, so it does BOTH: the menu_tree.rs entries in the lane, and native-host [document] gui_menu (MENU-INV-1) passing at landing. Say so in the reports.

Out of scope: the other Effect Stack and Boards leaves, any dialog, board GUI, a document record for the board container (derive it; do not add a Folio field), menu paths for other menu_path_pending commands, drive.rs menu rows, native-host src/gui/* and src/*.rs (SMB-GUI-GESTURE-GATE-1: read only; if a leaf needs a host change, ask the lead naming file and line), view/palette.rs and view/mod.rs (SMB-PALETTE-MODEL-1), view/layer_records.rs (SMB-LAYER-RECORD-PRUNE-2), any Cargo.lock change without asking.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PALETTE-MODEL-1, 2 SMB-MENU-EFFECT-BOARD-LEAVES-1, 3 SMB-ID-STABILITY-REVISIONS-1, 4 SMB-LAYER-RECORD-PRUNE-2. This lane lands 2 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): round STUDIO-R60 attempt 4 runs on the D: round slot at candidate 198f6dc4 and holds the D: lock with priority (class round); every lane hold queues behind it, which is expected: start the hold in the background and wait to be notified, never poll in a loop, never run cargo outside a hold, never work around the lock. ONE HOLD PER PHASE (wave-next-3, 2026-11-14): the four lanes share the D: lock with each other and with the three active wave-next-2 lanes (SMB-PR-TIMELINE-FIXES-1, SMB-MOTION-IMPLIED-VALUES-1, SMB-GUI-GESTURE-GATE-1); run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip. Lanes land one at a time on the lead's release, in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: the commands' existing settlements; F5 changes one outcome code (canceled) of Run::command_outcome, codes only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: both leaves run existing commands that carry the caller's context; no new mutating op", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-entry-references-1`, branch `lane/smb-menu-effect-board-leaves-1`, base 5655345b)

hsk-studio-harness-view/src/view/menu_tree.rs (Rule, LEAVES, leaf_params, CATALOG_COMMANDS, declared, run_catalog, PALETTE_ONLY, Profile::of); hsk-studio-harness-view/src/view/commands.rs (run_command outcome code only); hsk-studio-harness/tests/menu_effect_board_leaves.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-entry-references-1 --disk d`): step 0; harness-view [viewer] suite; harness [viewer] --test menu_effect_board_leaves --test board --test viewer_raster_effects --test harness_drive_menu; native-host [document] --test gui_menu (list them with REVIEW READY); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-entry-references-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
