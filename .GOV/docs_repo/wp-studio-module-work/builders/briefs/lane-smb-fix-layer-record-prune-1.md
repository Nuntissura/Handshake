---
file_id: LANE-SMB-FIX-LAYER-RECORD-PRUNE-1
file_kind: lane_brief
updated_at: 2026-11-13T14:09:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-LAYER-RECORD-PRUNE-1
lane_file: builders/lanes/hsk-studio-fix-layer-record-prune-1.json
crates: harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="After any structure step, in the same step, the records of layers absent after the step are removed">

## Target

Central prune of layer-keyed records: after any structure step, in the same step, remove the records of layers absent after the step (harness-view)

Why this lane: lead PRIORITY candidate 2026-11-13T13:38:07Z (ledger direction.next_lane_candidates): vector piece and pathfinder replacements, design copy deletes and other delete paths besides layers.rs delete_layer leave raster_records rows until the next open, so a reused id can inherit them in-session; pairs with SMB-LAYER-ID-STABILITY-1. Common lead ruling 5: a deleted object's records are removed in the same undo step as the object, and undo restores both.
Lead direction for this wave: after any structure step, in the same step, remove the layer-keyed records (raster_records, board and board_node records, text stories bound to removed nodes, vector path records of removed nodes) of layers absent after the step.

Facts are READ by the fix-wave planner at origin c8fca900 (read-only, not run). R60 file:line are at b57c82d1; `git diff b57c82d1 c8fca900` touches none of the files named below unless stated. Treat them as claims and check them.
- Today only open and delete_layer prune: `harness-view/src/view/place.rs:401` prune and `:776` drop_absent_records (at open, outside history); `view/board/write.rs:587` board_opened (at open) and `:616` board_records_after_delete; `view/design/component.rs:832` records_after_delete; `view/layers.rs:245` delete_layer composes them into its own step (`:287`).
- Structure steps without that prune: `view/layers.rs:92` structure (`:93`); `view/vector.rs:1646` replace_with_entry (`:1651`), `:2177` replace_with_pieces, `:2187` replace_with_pieces_deleting, `:2662` pathfinder_layers; `view/design/component.rs:468` commit_structure (`:480`), duplicate_layer at `:901` and `:940`; `view/design/instance.rs:1153` native_redraw (`:1211`, `:1214`); `view/place.rs:575` and `:1075`; `view/board/write.rs:177` board_commit (`:195`). Run steps: `harness-core/src/harness.rs:1439` restructure, `:1497` restructure_and_entry, `:1512` restructure_and_entries, `:1618` catalog_step.
- Layer-keyed fields to enumerate (folio): StudioRasterRecords rows (`raster.rs:751` layer_id) and the place entry's placed instances; StudioVectorRecords (`vector.rs:456-535`): clips and masks `source_layer_id` (`:287`, `:307`), mask attachments and boolean groups `layer_id` (`:374`, `:443`), area_texts `container_layer_id`, board_nodes (`board.rs:342`); the text entry's stories bound to board nodes (d41271aa).

Red test (must fail first):
One red test per path family in `harness/tests/fix_layer_record_prune.rs`: a vector piece replacement, a pathfinder, a design copy delete and a board node delete each remove the removed layer's records in the same step; a layer made after it that takes the id inherits nothing in-session; undo restores layer and records together, redo removes both; save and reopen agree. They fail at the base.

Fix contract:
- Before coding, send the lead the design: ONE central function every structure step calls (or a Run hook applied inside the step), the record kinds it prunes, and every call site; ask before editing harness-core harness.rs.
- The open-time drops (place, board, design) stay as the guard for packages written before this lane.
- No new Folio field (if one is needed, rule 4b: the package suite too).
- 3 to 6 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: this defect only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Out of scope: RF-01, render-gpu, panels and drawing, any Cargo.lock change without asking the lead, and the other R60 findings named here.
Named out of scope: SMB-LAYER-ID-STABILITY-1 (id minting), F-R60-36 (Layers > Duplicate of a placed layer drops its raster record: name it as a sibling), layout records (SMB-LAYOUT-DELETE-CASCADE-1).

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-FIX-DIAG-SINK-ABORT-1, 2 SMB-FIX-NEGATIVE-SPACE-1, 3 SMB-FIX-HOST-TERMINAL-FRAME-1, 4 SMB-FIX-ANIMATION-VALUES-1, 5 SMB-FIX-CAPTURE-ADMISSION-1, 6 SMB-FIX-EXPORT-ADMISSION-1, 7 SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 7 of 7.

NO CARGO PHASE 1 WAIT (fix wave 2026-11-13): the validation round is stopped (C: is at the grant limit), so no round holds or queues for the lock; the fix lanes share the D: lock only with each other. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16), in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-sentinels-native-1`, branch `lane/smb-fix-layer-record-prune-1`, base c8fca900)

hsk-studio-harness-view/src/view/layer_records.rs (new; placeholder name); the structure-step call sites in hsk-studio-harness-view/src/view/layers.rs, place.rs, vector.rs, board/write.rs, design/component.rs and design/instance.rs; one mod line in hsk-studio-harness-view/src/view/mod.rs; hsk-studio-harness-core/src/harness.rs (only if the lead approves a Run hook); hsk-studio-harness/tests/fix_layer_record_prune.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-sentinels-native-1 --disk d`): step 0; harness-view [viewer] suite; the harness-core suite only if harness.rs changes; harness [viewer] --test fix_layer_record_prune and the harness test binaries that run structure steps (layers, place, vector pieces and pathfinder, design components and instances, board; list them with git grep and send the list with REVIEW READY); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-sentinels-native-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
