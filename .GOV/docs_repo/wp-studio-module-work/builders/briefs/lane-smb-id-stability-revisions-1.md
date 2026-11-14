---
file_id: LANE-SMB-ID-STABILITY-REVISIONS-1
file_kind: lane_brief
updated_at: 2026-11-14T08:35:55Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-STABILITY-REVISIONS-1
lane_file: builders/lanes/hsk-studio-id-stability-revisions-1.json
crates: session
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The session's chronicle revision vector stops growing without bound when layers with fresh ids come and go, so never-reuse layer ids do not end in budget_exceeded; first slice of SMB-LAYER-ID-STABILITY-1">

## Target

The session's chronicle revision vector stays within Budget.revisions when layers with fresh ids are created and deleted, so never-reuse layer ids become possible (hsk-studio-session session.rs)

Why this lane: ledger candidate 2026-11-13T13:38:07Z, verbatim: "SMB-LAYER-ID-STABILITY-1: never-reuse layer ids (industry norm) across all mint sites, with chronicle retired-address pruning (Budget.revisions), DS design states keyed by (layer, instance), stable copy ids with in-place redraw, and no-op sync semantics; cross-lane, owns harness-view layers, text, vector and design mint sites plus chronicle. Findings from this lane (option A at 7b0ca538, reverted): (1) with stable copy ids a sync that changes nothing has no structure write and the run refuses it nothing_to_change, while 13 DS tests (one_undo_step..., undo_back_across_compactions..., redo_after_the_undo..., a_state_refused_after_the_sync..., both measured probes, three_hundred_syncs..., r1_*, detach_keeps_the_frame..., f6_a_stale_override..., a_native_state_whose_children..., a_sync_moves_an_explicit_mode...) expect every sync to be one history step (a no-op sync needs a defined step or success semantics; whether the run accepts identical entry bytes as a step is UNVERIFIED); (2) a_sync_with_no_room_for_its_states_is_refused_whole expects a sync to add states, which stable identity replaces (net 0); (3) r2_reopen_after_more_than_1024_instance_commands_keeps_working fails undo budget_exceeded under never-reuse ids, inferred (not instrumented) from the chronicle revision table (Budget.revisions 8192, hsk-studio-chronicle lib.rs:364-380, check near line 951) growing one address per fresh layer id; (4) the run refuses a delete and a create of one id in one transaction (duplicate_layer), so in-place update is the only stable-id redraw; (5) the delete paths other than delete_layer that leave raster_records rows (vector piece/pathfinder/knife replacements, design copy deletes, text conversions)". SMB-LAYER-ID-STABILITY-1 waited for SMB-FIX-LAYER-RECORD-PRUNE-1, which is READY_FOR_VALIDATION at 198f6dc4 (finding (5) is closed by it). The whole item is cross-lane and too large for one lane; this is its first slice, finding (3): the never-reuse mint was reverted (432a543c, lead decision 2026-11-13) because the revision table grows one address set per fresh layer id. Until this is fixed no mint site can stop reusing ids, and the PS-FILTER id scheme and the other slices (mint sites, design states keyed by (layer, instance), no-op sync semantics) stay blocked.

Facts are READ by the wave-next-3 planner at origin 5655345b (read-only, not run). Treat them as claims and check them.
- `hsk-studio-chronicle/src/lib.rs`: `Budget.revisions` (:368), default 8192 (:378); `prepare` refuses `Code::BudgetExceeded` when `revisions.len() > b.revisions` (:951); `seed_revisions(document)` (:459) lists the addresses of the document's nodes. Chronicle is stateless: the caller passes the vector.
- `hsk-studio-session/src/session.rs`: `Session.revisions: Vec<RevisionEntry>` (:501). `publish` (:2271-2302) only pushes (`Slot::New` :2280; the structural seed of new nodes :2296-2302); its doc (:2266-2270) says "Entries of deleted layers are kept: an undo that re-creates the layer finds its revisions where its inverse expects them" and names a test `layer_structure_edits_commit_undo_and_keep_revisions` in tests/structure.rs, which git grep does not find in crates at 5655345b (the name is stale or the test is gone: check). The vector is rebuilt from the document only at :792 (open) and :2642 (divergence resync). `revisions()` accessor :3490. `propose` matches an update to its slot and checks `previous_revision` (:2190-2215).
- So in one session every created layer id adds its addresses for good; with reused ids (today's mints, harness-view view/layers.rs:52 `mint_layer_id`) a deleted layer's entries are found again, with never-reuse ids they are not. The cause of the r2 failure in finding (3) was inferred, not instrumented: prove it first.
- harness tests/viewer_design_components.rs:576 `r2_reopen_after_more_than_1024_instance_commands_keeps_working` is the test that failed under never-reuse ids (read only; harness-view mint sites are NOT in this slice).

Red tests (must fail first):
crates/hsk-studio-session/tests/revision_retirement.rs (new): (1) a session creates and deletes raster layers with a FRESH id each cycle (ids through the session API, no harness-view mint) until the created addresses pass Budget.revisions (use a small test budget, not 8192 real cycles): the next ordinary edit of a live layer must still commit, and at the base it refuses budget_exceeded; (2) undo across the retirement: undoing the delete of the newest deleted layer re-creates it and its later edit quotes a true revision (the property the kept entries protect today); (3) redo after that undo; (4) a retired address is never matched to a new layer that takes the same id later with a wrong revision (stale previous_revision refuses typed); (5) save and reopen after many cycles: the vector equals seed_revisions of the document. Test (1) fails at the base; (2) to (5) pin what must stay true.

Fix contract:
- FIRST: send the lead the shape before coding and wait: WHEN an address retires (the layer is absent from the document AND no undo or redo step the session or the run still holds can re-create it; who knows that: the session, or the caller through a typed call), or whether instead the budget counts only the addresses of live nodes plus those a patch names. Name the undo/redo evidence for your choice (which inverse reads which revision).
- The vector's size is bounded by the live document plus the reachable history, not by the number of ids ever created.
- No change to what an undo of a delete needs; no revision is ever reused for a different node with a stale value.
- If the retirement needs a call from harness-core (history eviction), name the call and STOP for the lead: harness-core harness.rs belongs to SMB-LAYER-RECORD-PRUNE-2 in this wave.
- 1 to 3 commits after the red commit.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red tests above, committed compiling and run red at that COMMITTED tip. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix, committed, and the focused tests run at that committed tip. Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip; do not run on uncommitted work and commit afterwards.
4. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
5. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
6. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13 and 2026-11-14 "avoid large cargo builds, reuse targets"): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-pr-edit-footprint-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-session --test revision_retirement --test session_contract` (add `-p hsk-studio-chronicle --test foundation_contract --test structure` only if chronicle changes). Bundled: the hsk-studio-session suite (and the hsk-studio-chronicle suite if touched); `cargo check --tests` of the direct dependents comes from lane-check's per-sha step. Do NOT run the harness or harness-view suites for this slice unless the lead asks (the public API of session should not change; if it does, name the dependents that pin it).
MENU COVERAGE: this lane adds or renames no catalog command; say so in the reports.

Out of scope: every mint site (harness-view layers, text, vector, design; harness-core page mint), the PS-FILTER layer id scheme, design states keyed by (layer, instance), stable copy ids and in-place redraw, no-op sync semantics, the 13 DS tests of finding (1), layout_ui id mint, harness-core harness.rs and batch.rs (SMB-LAYER-RECORD-PRUNE-2), any Folio record change, any Cargo.lock change without asking.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PALETTE-MODEL-1, 2 SMB-MENU-EFFECT-BOARD-LEAVES-1, 3 SMB-ID-STABILITY-REVISIONS-1, 4 SMB-LAYER-RECORD-PRUNE-2. This lane lands 3 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): round STUDIO-R60 attempt 4 runs on the D: round slot at candidate 198f6dc4 and holds the D: lock with priority (class round); every lane hold queues behind it, which is expected: start the hold in the background and wait to be notified, never poll in a loop, never run cargo outside a hold, never work around the lock. ONE HOLD PER PHASE (wave-next-3, 2026-11-14): the four lanes share the D: lock with each other and with the three active wave-next-2 lanes (SMB-PR-TIMELINE-FIXES-1, SMB-MOTION-IMPLIED-VALUES-1, SMB-GUI-GESTURE-GATE-1); run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Run the focused hold on COMMITTED tips: commit the red tests, run them red, commit the fix, run focused at that tip. Lanes land one at a time on the lead's release, in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none new: the existing typed refusal (budget_exceeded) stops firing for retired addresses; if a retirement is reported, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not applicable: no new mutating op (revision bookkeeping of existing steps)", "jobs": "none", "settings": "none", "refactor_exposure": "medium (session is under every crate that edits a document; the change is internal bookkeeping)"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-id-stability-revisions-1`, base 5655345b)

hsk-studio-session/src/session.rs (the revision vector: publish, propose slot matching, a retirement step); hsk-studio-session/tests/revision_retirement.rs (new); hsk-studio-chronicle/src/lib.rs only with the lead's approval (budget rule)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-edit-footprint-1 --disk d`): step 0; hsk-studio-session suite (and hsk-studio-chronicle if touched); list the test binaries with REVIEW READY; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-edit-footprint-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
