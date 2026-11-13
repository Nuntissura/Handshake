---
file_id: LANE-SMB-FIX-MENU-CATALOG-COVERAGE-1
file_kind: lane_brief
updated_at: 2026-11-13T15:44:59Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-MENU-CATALOG-COVERAGE-1
lane_file: builders/lanes/hsk-studio-fix-menu-catalog-coverage-1.json
crates: harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Trunk repair: every catalog command has a menu leaf or a typed palette_only reason again (MENU-INV-1), with the invariant test unchanged">

## Target

Trunk repair: hsk-studio-native-host [document] tests/gui_menu.rs every_catalog_command_has_a_leaf_or_a_palette_reason_and_every_domain_a_home fails on origin/codex/handshake-studio-assets with '79 catalog commands have neither a leaf nor a palette_only reason (STU-SHL-013)'.

Why this lane: SMB-GUI-MENU-1 (landed 580f44b2) added the invariant MENU-INV-1 (STU-SHL-013..021). Lanes that landed after it added catalog commands without menu entries, and their landing runs did not run native-host's gui_menu (lead 2026-11-13).

Facts read by the builder at origin 86ee4e57 (claims, check them):
- `crates/hsk-studio-native-host/tests/gui_menu.rs:683` the invariant test; on failure it prints one `PALETTE ("<command>", Unreached::...)` line per missing command with a verb heuristic.
- `crates/hsk-studio-harness-view/src/view/menu_tree.rs:521` LEAVES (14 designed leaves that run catalog commands through `run_catalog`, `CATALOG_COMMANDS` :669), `:540` ALIASES, `:770` PALETTE_ONLY (typed reasons `Unreached::MenuPathPending` / `Unreached::NoMenuHome`, split by verb as SMB-GUI-MENU-1 did: reads, records, entries, submits, catalog and state queries are no_menu_home), `:1356` DOMAIN_HOMES (MENU-INV-3), `:3401` reached_commands.

Work:
1. TRUNK CHECK FIRST, in one D: hold (`lane-check --owner smb-r53-followups-1 --disk d`): the whole native-host [document] suite, the harness-view [viewer] suite and the harness [viewer] suite at the origin tip with a fresh example; list EVERY failing test (trunk-red from cross-lane integration); keep the log.
2. Red proof: the existing MENU-INV-1 test, already red at the tip (that log). Fix in menu_tree.rs: each of the 79 commands gets a real menu leaf where the designed tree (STU-SHL-023..037) has an obvious home, else a palette_only entry with the correct reason (no_menu_home, or menu_path_pending split by verb as SMB-GUI-MENU-1 did). Prefer real leaves where the spec gives a home. Do NOT weaken the invariant test. Any domain new to the catalog gets a DOMAIN_HOMES row (MENU-INV-3).
3. Other trunk-red tests found in step 1 are NOT fixed here: report them with test names and failure lines; the lead decides.
4. Focused gui_menu run, then ONE bundled lane-check: native-host [document] whole suite and harness-view [viewer] suite. Report BUNDLED GREEN with the trunk-check failure list; wait for RELEASED before landing.

Out of scope: gui_menu.rs (the invariant stays as is), any other crate, any other failing test, menu paths for the existing 544 menu_path_pending commands (next-lane candidate of SMB-GUI-MENU-1).

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none new: menu tree data only; menu open, run and refusal outcomes keep GUI-MENU-1's frame.menu codes", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required where a new leaf runs a mutating command: the leaf runs through the existing run_catalog / run_menu_leaf path with the caller's context; never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r53-followups-1`, branch `lane/smb-fix-menu-catalog-coverage-1`, base 86ee4e57)

crates/hsk-studio-harness-view/src/view/menu_tree.rs only

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-r53-followups-1 --disk d`): step 0; trunk check at the origin tip: native-host [document] whole suite, harness-view [viewer] suite, harness [viewer] suite (fresh example); after the fix: focused gui_menu, then native-host [document] whole suite and harness-view [viewer] suite in one bundled run; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-r53-followups-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
