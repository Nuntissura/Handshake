---
file_id: LANE-SMB-OBSERVE-DIAG-SINK-1
file_kind: lane_brief
updated_at: 2026-11-07T05:20:57Z
wp: WP-KERNEL-STUDIO
lane: SMB-OBSERVE-DIAG-SINK-1
lane_file: builders/lanes/hsk-studio-observe-diag-sink-1.json
crates: observe, harness-core, harness
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="What ran survives the process: a default durable sink, emit wired to the ring port, a panic record and a health check">

## Target

Work order item 5 in `S/HANDOFF.md`, diagnostics part. Evidence: `S/research/truth-check-20261105-hbr-usage.json` `rules[0]`, `rules[6]`, `rules[8]`, `rules[38]`, `rules[41]`. Today every shipped path has no sink, so nothing survives the process. `observe/src/ring.rs:6` says `Observe::emit*` is not wired to its port; `RingProducerPort` is at `ring.rs:698`; `emit_inner` at `observe/src/lib.rs:924`. `harness-core/src/harness.rs:130` `FrameSink` keeps only class and byte length. The host binary has no panic hook, and R56's composer abort left no record.

`observe` feeds 38 crates, so a change there rebuilds most of the workspace and `--per-sha` holds the disk for a long time. **Keep this lane to about 4 commits**, and do the `observe` part first.

1. **Red first.** After a process exits, a file in `--diag-dir` shows what ran, in order, with codes and counters.
2. **Wire `Observe::emit*` to `RingProducerPort`.** The adapter onto `DiagRingWriter` stays Handshake's (MT-36190): build Studio's side only. With no port and no sink, emitting costs what it costs today: measure it and state the numbers.
3. **Default durable JSONL sink for the standalone host.** Bounded by size, with a stated rotation or stop rule, and a write failure never fails the command. Ids, codes, counters and digests only, never document contents or user paths beyond what frames already carry. Read `spec-modules/15-*` GUI-DESKTOP rules and the HBR diagnostics rows for the required fields; use their names.
4. **`FrameSink`** passes frames to the sink rather than keeping class and length only.
5. **Panic record.** A panic hook in the CLI and the desktop binary writes a typed record to the diag dir and keeps the existing exit behaviour. Say what a stack overflow abort, which no hook sees, leaves behind.
6. **Health check.** `studio-harness health` prints JSON and exits 0 or a typed non-zero. Add it to `manual.rs`. Coordinate with SMB-HARNESS-DRIVE-1, which adds commands in `cli.rs`: keep your edit small.
7. **Sweep (V4-06).** List every discard sink: at least `harness-view/src/view/brush_mix.rs:34-39`, `composite/src/temporal/composition/effects.rs:209-214`, and `Quiet` sinks. Fixed or left with a reason. Changing engine paths is not this lane's job unless it is one line.
8. **MTs.** Claim a row only if you read it and this proves it. Do not edit MT files.

Hostile input (V4-01/02): a full disk, a read-only dir, a dir removed while running, a very long detail. None fails the command or grows without bound.

Out of scope: Flight Recorder and Palmistry bindings (Handshake's), UI heartbeat, drawing, no new dependency without asking the lead, no root input.

PUSH HOLD: STUDIO-R59 is judging c4d06841 on C:. Build on D: only. Push nothing until the lead tells you R59 is recorded; when you are ready to push, message the lead with the local tip and your test counts. One lane pushes at a time.

Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005). The folder name and `--owner` are the old lane's; your branch, log names and records are this lane's.

No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D6, D7, D8), unless this brief names it. No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "this lane builds it: internal_diagnostics producer through the ring port, adapter stays Handshake's (MT-36190)", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not_applicable: records carry the frames' existing attribution", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r53-followups-1`, branch `lane/smb-observe-diag-sink-1`, base c4d06841)

`crates/hsk-studio-observe/src`, `crates/hsk-studio-harness-core/src/harness.rs` FrameSink and `manual.rs`, `crates/hsk-studio-harness/src/cli.rs` and `src/bin/studio-harness.rs` hook and `health` only, their tests. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-r53-followups-1 --disk d`): step 0; observe, harness-core and harness suites; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-r53-followups-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
