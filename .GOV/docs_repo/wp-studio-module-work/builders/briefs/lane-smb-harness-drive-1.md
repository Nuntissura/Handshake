---
file_id: LANE-SMB-HARNESS-DRIVE-1
file_kind: lane_brief
updated_at: 2026-11-07T05:20:57Z
wp: WP-KERNEL-STUDIO
lane: SMB-HARNESS-DRIVE-1
lane_file: builders/lanes/hsk-studio-harness-drive-1.json
crates: harness, harness-core, harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A model with no Rust test can read state and drive Studio as a process, with no window">

## Target

Work order item 4 in `S/HANDOFF.md`, ranked first by four audits. Evidence: `S/research/truth-check-20261105-hbr-usage.json` `top_gaps[0]` and `rules[21]`, `rules[23]`; `truth-check-20261105-mt-shell-manual-foundation.json`. Today a model without a Rust test can observe or drive nothing beyond 13 document commands. STU-CON-007: every tool or command must be model-invokable, parallel-safe, deterministic and visually verifiable. Read that clause and STU-MDL-006 in the spec first. MT ranges to read: MT-33547..33557, MT-33500..33507, MT-33634..33639, MT-35107..35133.

This is the first slice, not the whole contract. **Send the lead a short plan before writing code:** the command set, how a command is named and typed, and what `state` returns. Reuse what exists: `Viewer::state_json` (`harness-view/src/view/mod.rs:837`), `Run`, `CommandContext`, the typed timeline and document commands, `harness-core/src/manual.rs:17`. Do not invent a second command catalog, and use only names already in the spec or code. Where the control mechanic should live is the Operator's open O5: build nothing that decides it.

1. **Red first, through the real process.** Tests spawn the built binary and speak to it through arguments and stdin. An in-process call does not prove this lane.
2. **Commands in `harness/src/cli.rs:270`:** `state`, `command`, `undo`, and a stdin JSONL loop, over `Run` and the Viewer's non-UI methods. Each prints JSON with the revision and `state_json`, takes `--actor`, `--session`, and `--diag-dir` if SMB-OBSERVE-DIAG-SINK-1 has landed (otherwise name it as the next step). A failure is a typed refusal with a non-zero exit.
3. **No window, ever.** These commands open no window, take no focus and use no OS input (HBR-QUIET). Add a negative test.
4. **The window trap.** In `harness/src/bin/studio-harness.rs` any unknown argument, including `help` or `develop`, is taken as a file and opens a foreground window (the `_ => file =` arm). The manual says `studio-harness help`, and the example and binary share a name. An unknown word must print usage and exit without a window. Change only argument handling there: no new egui, eframe, winit or wgpu line.
5. **Manual.** `manual.rs` gains these commands, generated from one table if that is cheap; a no-context model must be able to use them from `help` alone.
6. **MTs.** Read the ranges above. Claim a row only if you read it and this proves it. Do not edit MT files.

Hostile input (V4-01/02): a malformed JSONL line refuses and the loop continues; a very long line and a very deep document are bounded; a closed stdin ends cleanly.

Out of scope: the full control registry, drawing, panels, RF-01, RF-05, process-level parallel save safety (SMB-SESSION-WRITE-SAFETY-1), no crate `Cargo.toml` change without asking, no root input.

PUSH HOLD: STUDIO-R59 is judging c4d06841 on C:. Build on D: only. Push nothing until the lead tells you R59 is recorded; when you are ready to push, message the lead with the local tip and your test counts. One lane pushes at a time.

Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005). The folder name and `--owner` are the old lane's; your branch, log names and records are this lane's.

No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D6, D7, D8), unless this brief names it. No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: --actor and --session reach the CommandContext of every command; never a default actor on a mutating command", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-harness-drive-1`, base c4d06841)

`crates/hsk-studio-harness/src/cli.rs` and new modules under it, `crates/hsk-studio-harness/src/bin/studio-harness.rs` argument handling only, `crates/hsk-studio-harness-core/src/manual.rs`, `crates/hsk-studio-harness/tests`. Non-draw Viewer methods only if a command needs one. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-edit-footprint-1 --disk d`): step 0; harness and harness-core suites, plus harness-view if touched; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-edit-footprint-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
