---
file_id: LANE-SMB-SESSION-WRITE-SAFETY-1
file_kind: lane_brief
updated_at: 2026-11-07T05:20:57Z
wp: WP-KERNEL-STUDIO
lane: SMB-SESSION-WRITE-SAFETY-1
lane_file: builders/lanes/hsk-studio-session-write-safety-1.json
crates: harness, harness-core, harness-view, session
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Two processes saving one package cannot silently overwrite each other, and atomic writers use unique temp names">

## Target

Work order item 6 in `S/HANDOFF.md`. Evidence: `S/research/truth-check-20261105-hbr-usage.json` `top_gaps[3]`, `rules[9]` (HBR-SWARM-001) and `rules[51]`. Today two models running `edit --save PKG --overwrite` are silent last-writer-wins: `--expect-revision` is optional and checked only against the in-memory session (`harness/src/cli.rs:34`, `:144`, `:1068`), and `write_atomic` (`harness-view/src/view/export.rs:140`) uses a fixed temp name. STU-CON-007 requires parallel-safe commands.

1. **Red first, with real processes.** Two OS processes saving one package, and one process saving over a file another writer changed since it opened. `thread::spawn` used only as a timeout wrapper does not count: that is the gap the audit names.
2. **Unique temp names** in every atomic writer: `export.rs:140`, `view/audio_import.rs:779` `write_atomically`, and any other you find (V4-06: list each, fixed or not applicable with a reason). A leftover temp file from a crash stays harmless and is named in your report. Keep same-directory rename semantics.
3. **Refuse on mismatch at the file boundary.** A save compares the expected on-disk revision or content hash and refuses with a typed code on mismatch. Read the package format and the session first, then send the lead the option you chose if it changes the file format or any existing success path. A narrow race between compare and rename remains possible without OS locking: state it plainly rather than claim it is closed, or close it and say how.
4. **CLI.** `edit` in place requires `--expect-revision`, as `verify` already requires an expectation (`cli.rs:1014`). This changes a success path: update `harness-core/src/manual.rs:23` and the descriptors, and say which tests changed.
5. **The bridge lease.** `top_gaps[3]` also names lease expiry on the bridge. NOT_INSPECTED by the lead: report what it is and whether it belongs here; do not build it unasked.
6. **MTs.** Claim a row only if you read it and this proves it. Do not edit MT files.

Hostile input (V4-01/02): a package path that is a directory, a read-only file, a vanished file, a very long name. Each refuses typed.

Out of scope: any drawing or panel, the process-level drive path (SMB-HARNESS-DRIVE-1 owns new commands in `cli.rs`; keep your `cli.rs` edits small because it rebases on you or you on it), no crate `Cargo.toml` change, no root input.

PUSH HOLD: STUDIO-R59 is judging c4d06841 on C:. Build on D: only. Push nothing until the lead tells you R59 is recorded; when you are ready to push, message the lead with the local tip and your test counts. One lane pushes at a time.

Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005). The folder name and `--owner` are the old lane's; your branch, log names and records are this lane's.

No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D6, D7, D8), unless this brief names it. No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: typed refusal codes, no file contents", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-sequence-session-1`, branch `lane/smb-session-write-safety-1`, base c4d06841)

`crates/hsk-studio-harness/src/cli.rs` save path, `crates/hsk-studio-harness-core/src/manual.rs`, `crates/hsk-studio-harness-view/src/view/export.rs` and other atomic writers, `crates/hsk-studio-session` save path, their tests. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-sequence-session-1 --disk d`): step 0; harness, harness-core, session and harness-view suites; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-sequence-session-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
