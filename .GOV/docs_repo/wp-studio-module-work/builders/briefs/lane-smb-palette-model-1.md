---
file_id: LANE-SMB-PALETTE-MODEL-1
file_kind: lane_brief
updated_at: 2026-11-14T08:35:55Z
wp: WP-KERNEL-STUDIO
lane: SMB-PALETTE-MODEL-1
lane_file: builders/lanes/hsk-studio-palette-model-1.json
crates: harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The command palette's model: every catalog command and menu leaf as a row with its menu_path breadcrumb, one availability evaluation shared with the menu, and the fuzzy matcher with a once-built index (STU-SHL-041, MENU-INV-6); first slice of the palette lane">

## Target

The command palette's model in the egui-free Viewer state: rows, breadcrumb, shared availability, fuzzy matcher and index (hsk-studio-harness-view view/palette.rs, new)

Why this lane: it is the first slice of the palette lane that unblocks gesture test 7b and the spine GATE_CHECK. Ledger candidate 2026-11-13T09:27:50Z, verbatim: "STU-SHL-041..044: command palette with menu_path breadcrumb and shared fuzzy matcher, shortcut resolution ranks, shipped shortcut sets, Shortcut Conflict Report; only the Undo and Redo chords (PROVISIONAL Studio Default) resolve at render". Ledger candidate 2026-11-13T09:27:50Z, verbatim: "menu paths for the 544 menu_path_pending catalog commands (STU-SHL-013), per-command review of the palette_only split, MENU-INV-6 palette" (only its MENU-INV-6 part). SMB-GUI-GESTURE-GATE-1 lane notes, verbatim: "Test 7b: untimed equivalents reachable from the menu and the palette, with the palette lane (STU-SHL-109 #7, STU-SHL-041..044); the spine GATE_CHECK waits on it". The whole palette (STU-SHL-041..044) is more than one lane: this slice is the model only; the palette surface in native-host src/gui and test 7b follow after SMB-GUI-GESTURE-GATE-1 lands (its files are active now), shortcuts and the conflict report (STU-SHL-042..044) after that.

Facts are READ by the wave-next-3 planner at origin 5655345b (read-only, not run). Treat them as claims and check them.
- Spec: [STU-SHL-041] (:31859) the palette renders each row's menu_path as a breadcrumb; both projections read ONE availability evaluation and show the same reason code; the matcher is subsequence matching with contiguity and word-boundary bonuses; a prefix/trigram index is built once in the same OnceLock that holds the command set; the semantic match predicate stays the definition and the index is its accelerator. MENU-INV-6 (:31542): the palette never invents a command the menu lacks, and the menu never contains a leaf the palette cannot find. [STU-SHL-013] (:31525) palette_only with a typed reason. [STU-SHL-109] (:32316) test #7.
- No fuzzy or subsequence matcher exists in the product tree (git grep -i 'fuzzy|subsequence' over crates at 5655345b: only Make Subsequence names). The spec's "existing symbol palette" is not in this repository (UNVERIFIED where it lives; ask the lead before copying any matcher).
- `harness-view/src/view/menu_tree.rs`: `template()` (:3246) holds the designed tree in a OnceLock; `LEAVES` (:520-535) the catalog leaves; `ALIASES` (:539); `PALETTE_ONLY` (:771) every catalog command no leaf runs, with `Unreached` (:752) MenuPathPending or NoMenuHome; `reached_commands()` (:3487); `Viewer::studio_menu(&MenuContext)` (:3297) answers (MenuBar, MenuState) with each node's Availability; `evaluate` (:462) and `MenuState::of` (:384) are the one availability evaluation; `run_menu_leaf` (:3404) and `run_menu_leaf_in` (:3412) run a leaf; `run_catalog` (:683). menu_tree.rs belongs to SMB-MENU-EFFECT-BOARD-LEAVES-1 in this wave: READ ONLY here.
- The catalog: harness-core `action::ui_seed()` (harness-view view/commands/seed.rs:22) and `action::command(id)` (view/commands.rs:135).
- native-host src/gui/mod.rs:1741 `GuiFrame::menu_json` is the host's menu projection (SMB-GUI-GESTURE-GATE-1's files: read only).

Red tests (must fail first):
crates/hsk-studio-harness-view/tests/viewer_palette.rs (new, `#![cfg(feature = "viewer")]`): (1) MENU-INV-6 both ways: every enabled-or-disabled menu leaf that runs a command has exactly one palette row, and every palette row is a menu leaf or a PALETTE_ONLY command with its typed reason; no row for an id outside the catalog and the tree; (2) a leaf's row carries its menu_path joined as a breadcrumb equal to the tree's path of that node (for example Object > Layer > Duplicate Layer); a palette_only row carries its reason code instead of a path; (3) a row's availability and reason code are byte-equal to the menu node's for the same Viewer state (an empty document, a document with a selection); (4) the matcher: a subsequence matches, a non-subsequence does not, a contiguous hit outranks a scattered one, a word-boundary hit outranks a mid-word one, ties are stable (tree order), the empty query lists everything in tree order; (5) the indexed search answers exactly what the plain predicate answers over the whole row set for a fixed query list (the index is an accelerator); (6) hostile query: longer than the cap, control characters and non-ASCII refuse typed or match nothing without a panic, bounded work. They fail at the base (no palette module).

Fix contract:
- FIRST: send the lead the shape before coding (the row type, where the OnceLock index lives, the query cap, the scoring rule and its tie-break, how a row is run: by its leaf through run_menu_leaf_in, a palette_only command is listed and not run in this slice) and your asks. Wait for the reply.
- New file `view/palette.rs` plus ONE `mod palette;` line (and a `pub use` if needed) in `view/mod.rs`. A crate-root re-export in src/lib.rs is SMB-GUI-GESTURE-GATE-1's file: ask the lead, naming the line.
- The palette reads the menu's availability evaluation; it never evaluates `requires` a second way.
- No new catalog command, no drive row, no menu_tree.rs edit in this slice (a `Viewer` query method and, if the lead approves it, a `state_json` projection). No egui, eframe, winit or wgpu type or line.
- 1 to 3 commits after the red commit.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red tests above, committed compiling and run red at that COMMITTED tip. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix, committed, and the focused tests run at that committed tip. Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip; do not run on uncommitted work and commit afterwards.
4. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
5. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
6. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13 and 2026-11-14 "avoid large cargo builds, reuse targets"): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-ai-distort-effects-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-harness-view --features viewer --test viewer_palette` and `cargo nextest run -p hsk-studio-harness-view --features viewer --lib` when you add unit tests. Bundled: the harness-view [viewer] suite (send the test-binary list with REVIEW READY).
MENU COVERAGE: this lane adds or renames no catalog command; say so in the reports.

Out of scope: the palette surface and keyboard handling in native-host src/gui (after SMB-GUI-GESTURE-GATE-1), test 7b itself, shortcut ranks, shipped sets and the conflict report (STU-SHL-042..044), running palette_only commands with parameters, WORKSPACE > Tools (STU-SHL-039), menu paths for the menu_path_pending commands, harness-view view/menu_tree.rs and view/commands.rs (SMB-MENU-EFFECT-BOARD-LEAVES-1), view/layer_records.rs (SMB-LAYER-RECORD-PRUNE-2), any Cargo.lock change without asking.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PALETTE-MODEL-1, 2 SMB-MENU-EFFECT-BOARD-LEAVES-1, 3 SMB-ID-STABILITY-REVISIONS-1, 4 SMB-LAYER-RECORD-PRUNE-2. This lane lands 1 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): round STUDIO-R60 attempt 4 runs on the D: round slot at candidate 198f6dc4 and holds the D: lock with priority (class round); every lane hold queues behind it, which is expected: start the hold in the background and wait to be notified, never poll in a loop, never run cargo outside a hold, never work around the lock. ONE HOLD PER PHASE (wave-next-3, 2026-11-14): the four lanes share the D: lock with each other and with the three active wave-next-2 lanes (SMB-PR-TIMELINE-FIXES-1, SMB-MOTION-IMPLIED-VALUES-1, SMB-GUI-GESTURE-GATE-1); run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip. Lanes land one at a time on the lead's release, in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none new: a pure query over the catalog and the menu tree; no job, no mutation", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not applicable: no mutating op in this slice (a row is run by its menu leaf, which carries the caller's context already)", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-distort-effects-1`, branch `lane/smb-palette-model-1`, base 5655345b)

hsk-studio-harness-view/src/view/palette.rs (new); hsk-studio-harness-view/src/view/mod.rs (the `mod palette;` line and a re-export only); hsk-studio-harness-view/tests/viewer_palette.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-ai-distort-effects-1 --disk d`): step 0; harness-view [viewer] suite (list the test binaries with REVIEW READY); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-ai-distort-effects-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
