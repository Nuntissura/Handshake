---
file_id: LANE-SMB-LAYER-RECORD-PRUNE-2
file_kind: lane_brief
updated_at: 2026-11-14T08:35:55Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYER-RECORD-PRUNE-2
lane_file: builders/lanes/hsk-studio-layer-record-prune-2.json
crates: harness-core, harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A played action's restructure prunes the records of the layers it removes like every other structure step, and the layer-record reference scan matches ids at exact boundaries">

## Target

A played action's restructure prunes the records of the layers it removes, in the same step, and the reference scan matches ids at exact boundaries (hsk-studio-harness-core harness.rs, batch.rs; hsk-studio-harness-view view/layer_records.rs)

Why this lane: ledger candidate 2026-11-13T17:13:35Z, verbatim: "F2 (SMB-FIX-LAYER-RECORD-PRUNE-1): a harness-core Run hook so Run::play_action's restructure (harness.rs:879; batch.rs:971, :985) prunes layer records like Viewer::structure_step". Ledger candidate 2026-11-13T17:13:35Z, verbatim: "F4 (SMB-FIX-LAYER-RECORD-PRUNE-1): exact-boundary id matching in the layer-record reference scan (today a substring hit keeps the id, which only keeps more)". Both are the accepted follow-ups of SMB-FIX-LAYER-RECORD-PRUNE-1 (lane notes next_lane_candidates_accepted: "lead 2026-11-13: all three on the follow-up list"), which is READY_FOR_VALIDATION at 198f6dc4. F2 is the one V4-06 site that lane named and could not fix: a correctness defect (records of a layer an action deletes outlive it, so a reused id can inherit them in-session).

Facts are READ by the wave-next-3 planner at origin 5655345b (read-only, not run). Treat them as claims and check them.
- `harness-core/src/harness.rs`: `Run::play_action` (:805); its step loop runs `Call::Restructure { op, writes } => self.restructure(&op, &writes)` (:879) with no record prune; rollback on a fault follows (:887-). `restructure` (:1439), `restructure_inner` (:1450), `restructure_and_entry` (:1497), `restructure_and_entries` (:1512), `restructure_and_entry_step` (:1697).
- `harness-core/src/batch.rs`: the automation calls `run.play_action(a, PlayMode::Headless)` at :1014 and :1028 (the candidate's :971 and :985 are these two sites at an older tip). The catalog row `action.play` (harness-core action/commands/automation.rs:6) is PALETTE_ONLY MenuPathPending (menu_tree.rs:772); no harness-view caller of play_action was found by git grep (check how a Viewer reaches it, if at all).
- `harness-view/src/view/layer_records.rs`: `Viewer::before_structure` (:215), `structure_step` (:226-243), `prune_removed` (:245); the prune joins the newest step only when the structure step pushed exactly one, else `prune_skipped:no_step` (lead ruling F6); warnings go to `Viewer::prune_warnings`. `occurring` (:87-130) is the reference scan: "a substring match: an occurrence glued to other id bytes counts"; MAX_SCAN_BYTES (:74); callers at :317, :378, :435, :443, :501.
- harness-core cannot reach `Viewer::structure_step` (harness-view depends on harness-core): the prune lane's notes name "a Run hook for the prune" as the repair.
- Existing tests: harness tests/fix_layer_record_prune.rs (15 tests), layer_records lib tests (5; one at :617), harness-core tests/action_record_replay.rs (:82-195), tests/business_events_run.rs:204.

Red tests (must fail first):
(a) crates/hsk-studio-harness/tests/layer_record_prune_2.rs (new, [viewer]) or an added test in fix_layer_record_prune.rs: an action whose step deletes a layer that holds raster, vector or text records is played on a document; after the play the records of the deleted layer are gone in the same history step (one undo restores layer and records, redo removes both), a bystander's records are byte-identical, and a faulted action's rollback leaves every record as it was. (b) in the same file or the layer_records unit tests: a candidate id that occurs only as a strict part of a longer id (for example LAY-12 inside LAY-123, and a prefix, a suffix and a middle case) is NOT counted as referenced, so its record is pruned; an id at an exact boundary (quoted JSON string, list element, start and end of a part) IS counted; a hostile part (no delimiters, 0 bytes, non-UTF-8) neither panics nor passes the byte budget. They fail at the base (no prune on play; a substring hit keeps the id).

Fix contract:
- FIRST: send the lead the shape before coding and wait: the hook's type and owner (a typed callback or port the Run holds, set by the Viewer; never a harness-view type in harness-core), when it runs inside play_action (per restructure step, before and after, like before_structure and prune_removed), how its entry writes join the action's history step and its rollback, what a headless batch Run without a Viewer does (no hook: today's behavior, named), and for F4 which bytes are id boundaries in each entry format you scan.
- The prune stays ONE implementation (layer_records.rs); the hook calls it. No copy of the prune in harness-core.
- A bound, a cancel or an unprunable entry keeps today's ruling: the step commits and carries prune_skipped:<entry>.
- F4 only narrows what counts as a reference; where a boundary cannot be decided, keep the id (keeping more is safe, dropping a referenced record is not).
- Any line in harness-view view/mod.rs needs the lead (SMB-PALETTE-MODEL-1 adds one line there): name it.
- 1 to 4 commits after the red commit.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red tests above, committed compiling and run red at that COMMITTED tip. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix, committed, and the focused tests run at that committed tip. Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip; do not run on uncommitted work and commit afterwards.
4. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
5. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
6. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13 and 2026-11-14 "avoid large cargo builds, reuse targets"): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-sentinels-native-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-harness --features viewer --test layer_record_prune_2 --test fix_layer_record_prune`, `cargo nextest run -p hsk-studio-harness-core --test action_record_replay --test business_events_run`, and `cargo nextest run -p hsk-studio-harness-view --features viewer --lib` for the layer_records unit tests. Bundled: the harness-core suite, the harness-view [viewer] suite, and the harness [viewer] binaries that play actions or prune records (git grep play_action and structure_step users in crates/hsk-studio-harness/tests; send the list with REVIEW READY).
MENU COVERAGE: this lane adds or renames no catalog command; say so in the reports.

Out of scope: Q2 of the prune lane (carrying clips, masks and text over to pieces), the atomic multi-entry history step (candidates 2026-11-13T07:59:45Z and 11:28:13Z; if the hook cannot join the action's step without it, STOP and report), property_edit undo names (gesture-gate candidate), layer id stability, harness-core motion_commands.rs (SMB-MOTION-IMPLIED-VALUES-1, active), harness-view view/menu_tree.rs and view/commands.rs (SMB-MENU-EFFECT-BOARD-LEAVES-1), view/palette.rs and view/mod.rs (SMB-PALETTE-MODEL-1), hsk-studio-session (SMB-ID-STABILITY-REVISIONS-1), any Cargo.lock change without asking.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PALETTE-MODEL-1, 2 SMB-MENU-EFFECT-BOARD-LEAVES-1, 3 SMB-ID-STABILITY-REVISIONS-1, 4 SMB-LAYER-RECORD-PRUNE-2. This lane lands 4 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): round STUDIO-R60 attempt 4 runs on the D: round slot at candidate 198f6dc4 and holds the D: lock with priority (class round); every lane hold queues behind it, which is expected: start the hold in the background and wait to be notified, never poll in a loop, never run cargo outside a hold, never work around the lock. ONE HOLD PER PHASE (wave-next-3, 2026-11-14): the four lanes share the D: lock with each other and with the three active wave-next-2 lanes (SMB-PR-TIMELINE-FIXES-1, SMB-MOTION-IMPLIED-VALUES-1, SMB-GUI-GESTURE-GATE-1); run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip. Lanes land one at a time on the lead's release, in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: the existing prune warnings (prune_skipped:<entry>) on the played action's result, codes only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the prune's entry writes join the action's step under the action's caller context; no default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium (harness-core Run::play_action is on the batch path; the hook is absent by default)"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-sentinels-native-1`, branch `lane/smb-layer-record-prune-2`, base 5655345b)

hsk-studio-harness-core/src/harness.rs (play_action restructure step and the hook); hsk-studio-harness-core/src/batch.rs (only if the automation must pass the hook); hsk-studio-harness-view/src/view/layer_records.rs (hook body, occurring boundaries); hsk-studio-harness/tests/layer_record_prune_2.rs (new) or tests/fix_layer_record_prune.rs (added tests only)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-sentinels-native-1 --disk d`): step 0; harness-core suite; harness-view [viewer] suite; harness [viewer] action and prune test binaries (list them with REVIEW READY); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-sentinels-native-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
