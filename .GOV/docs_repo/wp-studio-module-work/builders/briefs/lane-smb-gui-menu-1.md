---
file_id: LANE-SMB-GUI-MENU-1
file_kind: lane_brief
updated_at: 2026-11-13T05:13:41Z
wp: WP-KERNEL-STUDIO
lane: SMB-GUI-MENU-1
lane_file: builders/lanes/hsk-studio-gui-menu-1.json
crates: native-host, kumihimo-ui, harness-view, controls
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree, ONE HOLD PER PHASE, CARGO DISCIPLINE (Operator 2026-11-13))
---

<topic id="target" status="active" summary="B6 in the Vulkan window: 19 titles open menus from one descriptor source, greyed items show reasons, remedies run, items dispatch typed commands, tooltips L0 and L1">

## Target

Build-order step B6 in the Vulkan window: the 19 menu titles open real menus derived from one descriptor source, greyed items show their availability reason, a remedy runs, items dispatch typed commands, tooltips L0 and L1

Why this lane: the spec-coverage audit of 2026-11-11 (`S/research/spec-coverage-20261111-rollup.json` and the group file named below) judged every requirement against origin 87f331f9. Rows named here as MISSING or PARTIAL are your acceptance: a row moves when a model can run it through a typed command (JSONL loop of `harness/src/cli/drive.rs`) and the result reaches save and export.

Facts are READ by the wave planner at origin b57c82d1 with git grep, read-only, not run, unless marked 'planner READ at 0a733db8' (research/width-plan-20261108-*.json, older pin: re-check). Treat them as claims and check them.

Group file: research/spec-coverage-20261111-g7-shell-manual-misc.json. Rows: STU-SHL-010..044 (menu: 7 MISSING 013, 014, 039, 041-044; 23 PARTIAL), STU-SHL-046..055 (availability predicate: MISSING 048, 049, 054, 055), STU-SHL-230..236 (tooltip system: MISSING 230, 231, 233, 235, 236), STU-SHL-240..247 (UiDescriptor: MISSING 242, 245-247). Design: gui-design/build-order.json step B6 'Descriptors and menu', done_when 'All 19 titles open, greyed items show a reason, and a remedy runs'; B6 components menu_popup, menu_item, tooltip_l0, tooltip_l1, context_menu (build-order.json; shapes in gui-design/components.json). Ledger candidates: 2026-11-07T17:49:57Z (menus), 2026-11-07T13:35:06Z (availability fields in the semantic tree), 2026-11-07T12:31:03Z (widget ids not seeded in the UiCatalog), 2026-11-07T17:49:57Z (Viewer::run_command does not accept layer.set_visible).

Facts:
- native-host draws the 19 titles from `gui/preset.rs:13` (`MENU: [&str; 19]`) in `gui/chrome.rs:61-76`; every title is static: `chrome.rs:19` NIY = "not_implemented_yet", and a press on a disabled control answers it (`gui/mod.rs:326`).
- The menu model exists egui-free in `hsk-studio-controls/src/menu.rs`: `MenuBar::new` :367, `walk` :372, `find` :389, `resolve_remedies` :397, `audit` :432, `agent_json` :483, `manual_topics` :502; `Availability` :155 with `inapplicable(code, text, remedy_command_id)` :169 and `reason_line` :193. `menu_bar_ui` at :638 is egui: do not call it.
- The only populated MenuBar is the egui test shell's 7 titles (`harness-view/src/view/shell.rs:1071`): read it, do not edit it.
- `harness-core/src/action/ui_seed.rs:19` `ui_seed()` returns the UiCatalog seeded by SMB-COMMAND-SURFACE-1; RUN_COMMANDS is the typed command table.
- kumihimo-ui has `MenuTitle` (`widgets.rs:287`) and `menu_title_row` (:357); no popup, menu item or tooltip widget exists (git grep).
Depends on: SMB-GUI-FRAME-1 and SMB-VK-UI-PASS-1 (READY, on origin, not yet judged). One GUI line of work before B9 (build-order `parallel_from`): no other GUI lane runs beside this one. Host binary runs take the GPU lock; run Vulkan tests with VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation and go READY only validation-clean (direction.lead_rulings_20261108.vk_ui_pass_1).

How to work:
1. Read the code first. Use the spec's names and cite clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`). Use only names already in the spec, MTs or code.
2. Red test and focused test per item. Typed commands with typed parameters and typed refusals; each mutating command is one named undo step, carries the caller's attribution and survives save and reopen.
3. Commands register in the existing catalog: your domain file under `harness-core/src/action/commands/` (the per-domain `ROWS` tables that `commands.rs` concatenates into `RUN_COMMANDS`), or a `viewer.<method>` row.
4. V4-06: a short list of sibling sites, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. About 6 commits. Width over polish. State what you cut.
MTs: read those your slice proves and name them in your report; the lead records them. Do not edit MT files.

Out of scope: ScrubValue and core controls (B7), Tool Rail (B8), dock host (B9), panels (B10+), canvas overlays; no egui, eframe, wgpu, naga or WGSL line; no edit to the egui shell.

R60 attempt 3 takes the D: lock first. Until it releases, read the code, write red tests and commit locally; your first hold queues behind the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16).

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: menu open, item run and refusal outcomes as codes and counters in the receipt's gui.frame member; commands keep their own Run observe frames", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-entry-references-1`, branch `lane/smb-gui-menu-1`, base b57c82d1)

hsk-studio-native-host/src/gui/chrome.rs; hsk-studio-native-host/src/gui/mod.rs; hsk-studio-native-host/src/gui/preset.rs; hsk-studio-native-host/tests/gui_menu.rs (new); kumihimo-ui/src/widgets.rs (additive); hsk-studio-harness-view/src/view/menu_tree.rs (new, egui-free; name is a placeholder)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-entry-references-1 --disk d`): step 0; native-host [document] tests and kumihimo-ui suite; harness-view [viewer] suite only if you edit its src; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-entry-references-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
