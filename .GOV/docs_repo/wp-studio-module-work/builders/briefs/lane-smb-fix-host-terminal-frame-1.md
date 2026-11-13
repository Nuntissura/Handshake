---
file_id: LANE-SMB-FIX-HOST-TERMINAL-FRAME-1
file_kind: lane_brief
updated_at: 2026-11-13T14:09:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-HOST-TERMINAL-FRAME-1
lane_file: builders/lanes/hsk-studio-fix-host-terminal-frame-1.json
crates: native-host
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Every studio-vk run path sends the host's terminal observe frame through the caller's sink (F-R60-22)">

## Target

Every studio-vk run path sends the host's terminal observe frame through the caller's sink (F-R60-22, hsk-studio-native-host)

Why this lane: R60 attempt 3 finding F-R60-22 (validation/round-R60.json attempt_3.findings_leads[0]; MEDIUM, HOST INTEGRATION (5) gap; reviewer R60-a3-C-8 rated HIGH; attempt 1 R60-H-3; ledger PRIORITY candidate 2026-11-13T08:33:54Z). It touches the READY lanes SMB-VK-HOST-1, SMB-GUI-FRAME-1, SMB-GUI-SPINE-1 and SMB-VK-UI-PASS-1, whose host_integration blocks say 'producer_through_observe' and 'the host keeps its terminal frame'.

Facts are READ by the fix-wave planner at origin c8fca900 (read-only, not run). R60 file:line are at b57c82d1; `git diff b57c82d1 c8fca900` touches none of the files named below unless stated. Treat them as claims and check them.
- `native-host/src/diag.rs:24-36` emit_terminal(actor, resource, correlation_id, result, cancel, sink) and `:8-21` outcome(); its only caller is `tests/cli_and_observe.rs:90`.
- `src/bin/studio-vk.rs:6-27` calls offscreen::run (`src/offscreen.rs:43`) and window::run (`src/window.rs:280`), prints the receipt or exits with the code; no run path emits.
- `git diff b57c82d1 c8fca900` changes native-host gui/, event.rs, lib.rs and tests (SMB-GUI-MENU-1), not diag.rs, offscreen.rs, window.rs or bin/studio-vk.rs.
- HOST INTEGRATION (4) and (5): the actor, resource and correlation come from the caller (never default_actor()); frames carry codes and counters only.

Red test (must fail first):
A test that runs each run path through the host entry with a collecting sink and counts its frames: exactly one Terminal frame per run; Success for a good offscreen clear scene; Failure with outcome()'s code for a usage refusal (window --quiet without --frames, no GPU needed) and for a Vulkan refusal where one can be forced. It fails at the base.

Fix contract:
- Before coding, send the lead the shape: how the library entries take the caller's context and sink, and how studio-vk gets its actor, session and sink (for example caller-supplied values as the harness CLI's --actor and --session, harness/src/cli.rs:315-316) and where its frames go.
- One emission per run on every exit path, early usage refusals and the document and gui scenes included.
- No new crate edge (steward-owned) and no new winit, egui or wgpu line.
- 2 to 4 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: this defect only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Out of scope: RF-01, render-gpu, panels and drawing, any Cargo.lock change without asking the lead, and the other R60 findings named here.
Named out of scope: F-R60-32 (shader hash and validation asserts), F-R60-43 (spine gate tests), GUI behaviour.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-FIX-DIAG-SINK-ABORT-1, 2 SMB-FIX-NEGATIVE-SPACE-1, 3 SMB-FIX-HOST-TERMINAL-FRAME-1, 4 SMB-FIX-ANIMATION-VALUES-1, 5 SMB-FIX-CAPTURE-ADMISSION-1, 6 SMB-FIX-EXPORT-ADMISSION-1, 7 SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 3 of 7.

NO CARGO PHASE 1 WAIT (fix wave 2026-11-13): the validation round is stopped (C: is at the grant limit), so no round holds or queues for the lock; the fix lanes share the D: lock only with each other. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16), in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per host run (device open, window run or offscreen capture), codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the host run's actor, resource and correlation come from the caller; never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-entry-references-1`, branch `lane/smb-fix-host-terminal-frame-1`, base c8fca900)

hsk-studio-native-host/src/diag.rs; hsk-studio-native-host/src/offscreen.rs and src/window.rs (run entries only); hsk-studio-native-host/src/bin/studio-vk.rs; hsk-studio-native-host/src/cli.rs (only if the lead approves a flag); hsk-studio-native-host/tests/fix_host_terminal_frame.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-entry-references-1 --disk d`): step 0; native-host suite, default features and [document] (round unit features:hsk-studio-native-host+document); GPU tests wait for the GPU test lock; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-entry-references-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
