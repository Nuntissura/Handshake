---
file_id: LANE-SMB-R46-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-31T05:54:02Z
wp: WP-KERNEL-STUDIO
lane: SMB-R46-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r46-followups-1.json
crates: [hsk-studio-render-gpu, hsk-studio-observe, hsk-studio-harness (cli only), hsk-studio-interop-fig-base, hsk-studio-harness-view (design auto-layout walk bound only), hsk-studio-composite (tests only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree, HOST INTEGRATION)
---

<topic id="target" status="active" summary="Close R46 findings F-R46-1..7 and R45 findings F-R45-1..3">

## Target

Close these findings. Read each entry's `finding`, `repair_direction` and `file_line` in `validation/round-R46.json` and `validation/round-R45.json` → `findings_not_failures` first.

1. **F-R46-1 (MEDIUM), render-gpu threshold tie.** The 2^-20 boundary exemption in the threshold parity case rests on a false premise: pigment's Threshold is binary32 end to end, and the WGSL kernel evaluates the same binary32 formula, yet Vulkan returns black where pigment and DX12 return white at the 7 tie pixels.
   - Measure first: print, for one tie pixel, pigment's lum bits, the cut bits, and the GPU's lum bits on Vulkan and DX12 (a debug readback or a one-pixel probe), so the cause (contraction into FMA, evaluation order, or something else) is evidence, not a guess. Record the numbers in the lane notes.
   - Fix the kernel so the tie decides as pigment does on every backend: a contraction-free luminance sum (for example by splitting the products so the compiler cannot fuse them, or an error-free sum), or the integer rule round(255 lum) >= level from the PS-C07 reading applied identically in pigment and the kernel. Choose the smallest change that restores exact parity; if pigment changes, its own tests must stay green and every changed golden is listed.
   - Remove the boundary-band exemption and its NOT_PROVEN line; the tie pixels must match exactly again. Keep the 1e-4-off-cut pin test.
2. **F-R46-2 (LOW), FXC class unexercised.** On Windows, open the two FXC tests explicitly on DX12 (through `asset-local-backends` as a render-gpu dev-dependency on itself, or a test-only port) so a round proves the FXC class; keep the Vulkan branch as an extra row.
3. **F-R46-3 (LOW), authorize refusal untested.** Add a `test-faults` hook that revokes or supersedes the held grant of a live transient backend; assert `execute` and `execute_grade` refuse `Denied(revoked)` / `Lost` with no sink call; add a capture-sink test emitting each `PortEvent` through `report_port`.
4. **F-R46-4 (LOW), flag write before authorize.** In `SharedDevice::render_to_texture` (`plan_sink/shared.rs:499-501`) authorize (or check the grant) before the 4-byte overflow flag write, or move the write after admission and authorize.
5. **F-R46-5 (LOW), grant ledger growth.** Bound `GrantLedger`: classify closed grants by id range (ids are monotone) with a compact tombstone for DoubleRelease vs LateRelease, or prune settled and superseded records past a bounded window; keep an open count instead of the scan in `counters()`.
6. **F-R46-6 (LOW), observe host projection.** Add `RingRecord::host_core_le_bytes() -> [u8; 56]` with `event_code = host_event_code()`, pinned against the DiagEvent table of `wtc-native-editors-v1/src/frontend/handshake_native/diag_ring/schema.rs` @292d7a1c (read-only); state in the DESCRIPTOR that `to_le_bytes` carries the Studio code and `host_core_le_bytes` is the host write.
7. **F-R46-7 (LOW), audio decode budget.** In the harness CLI `export-sequence`, charge frames x channels x 4 of every `--audio` decode against one total budget before the decode (pass the remainder as reel's frame limit), or cap the `--audio` count; use the import limits' max file bytes for audio files.
8. **F-R45-1 (LOW), GRID decode bound.** In `interop-fig-base`, check the 1000-track limit before the quadratic work on track entries, and make the decode cancellable inside one node.
9. **F-R45-2 (LOW), frame-size walk.** The window frame-size walk in harness-view's design auto-layout can take exponential time (`background()` resets the depth counter; work runs before the subtree budget). Fix the bound in the walk logic only: add no egui code (Operator decision D1), touch no rendering call; if the function cannot be bounded without toolkit code, stop and report the exact line.
10. **F-R45-3 (LOW), refusal tests.** The three AE-EFFECT-STACK-1 / AE-EFFECT-FAMILIES-2 refusal tests compare only the code, which the missing-profile refusal now shares; make each assert its own detail.

Out of scope: F-R46-8 (the harness dev-dependency line) is removed by the GUI lane that gives Viewer a port, in one change; do not touch `crates/hsk-studio-harness/Cargo.toml` except as item 2 needs a render-gpu self dev-dependency (render-gpu's own Cargo.toml, not harness).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "central_broker_client", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "ring_visible"}, "control_descriptors": "none", "attribution": "required", "jobs": "bounded_admission_via_job_port", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-r46-followups-1`, base 66c62129)

- render-gpu: `src/kernel.wgsl` (threshold only), `src/client.rs`, `src/client/**`, `src/plan_sink.rs` and `src/plan_sink/shared.rs` (authorize ordering), `src/report.rs`, `tests/gpu_plan.rs`, `tests/gpu_pipeline_fault.rs`, `tests/client_port.rs`, its own `Cargo.toml` (self dev-dependency for item 2).
- pigment: `src/adjust_math.rs` threshold path only, if item 1 chooses the integer rule.
- observe: `src/ring.rs` and the DESCRIPTOR.
- harness: `src/cli.rs` (export-sequence audio budget) and its CLI tests.
- interop-fig-base: the GRID decode module and its tests.
- harness-view: the design auto-layout frame-size walk function only.
- composite: the three refusal tests only.
- Nothing is running in parallel; root inputs and the Master Spec stay untouched.

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Red first per finding; pushed red commits must compile.
- One bundled run on D: (`--disk d`; C: has 16 GiB headroom under the cap and stays reserved for the next round): the whole suites of render-gpu (its round units plus the test feature), pigment if changed, observe, interop-fig-base, composite, harness-view and harness under the round's viewer units, plus clippy over the changed crates' direct dependents (rules 4a and 4e). GPU tests run on Vulkan by default; item 2's DX12 rows run on DX12 explicitly. Name the log per RUN LOGS; lane-check prints the `HEAD=` header.
- Report the tie measurement of item 1 with bit patterns per backend.
- Vendor behaviour stays UNVERIFIED (D5).

</topic>
