---
file_id: LANE-SMB-OBSERVE-RING-PORT-1
file_kind: lane_brief
updated_at: 2026-10-31T00:07:10Z
wp: WP-KERNEL-STUDIO
lane: SMB-OBSERVE-RING-PORT-1
lane_file: builders/lanes/hsk-studio-observe-ring-port-1.json
crates: [hsk-studio-observe (additive only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="RF-04 phase 0: a ring-shaped, text-free producer port in observe, alongside SinkPort">

## Target

RF-04 phase 0 of `research/integration-refactor-inventory-20261030.json`, the observe side of MT-35530 HOST-AMEND-001 and MT-36190. Add a ring-shaped producer port next to the existing `SinkPort`.

Observe stays a bounded adapter: "not a recorder or authority" (`src/lib.rs:1`; STU-ARC-010, STU-VAL-002). Observe gets no storage, reader, catalogue or session registry. The ring itself, the reader and Palmistry are Handshake's (phase 1, MT-36233).

Host contract, read-only (Handshake `wtc-native-editors-v1` @292d7a1c):
- `src/frontend/handshake_native/diag_ring/src/schema.rs`:
  - `DiagEvent`, a 56-byte `#[repr(C)]` POD with align 8 and fields `event_code u16 | phase_marker u8 | severity u8 | _reserved [u8;4] (always zero, not content) | thread_id | sequence_id | counter_a | counter_b | metric_micros | timestamp_nanos` (u64 each);
  - `DiagEventCode`, a closed `u16` enum: Heartbeat 0 … StalledOperation 11, Other 65535;
  - `DiagPhase` 0..4 and `DiagSeverity` 0..2.
- `src/frontend/handshake_native/src/diagnostics/recorder.rs`: `record()` takes a `Mutex<VecDeque>` (:113). It is not real-time safe, which is a Handshake prerequisite.
- Do not edit or depend on either file.

Slices, one local commit each, red tests first. Put everything in a new `src/ring.rs` with `pub mod ring;` plus re-exports in `src/lib.rs`.

1. **`RingRecord`**: a fixed-size, text-free event record.
   - **Shape.** `#[repr(C)]`, `Copy`, integer fields only, no implicit padding. Add compile-time `size_of` and `align_of` asserts. Keep `#![forbid(unsafe_code)]` and add no new dependency: no bytemuck. Provide a field-by-field `to_le_bytes() -> [u8; RING_RECORD_BYTES]` for the host adapter and the tests.
   - **Core, the first 56 bytes.** Same order, widths and offsets as `DiagEvent`:
     - `code u16` (a `RingCode`), `phase u8`, `severity u8`;
     - `reserved [u8; 4]`, always zero;
     - `thread u64` (opaque producer thread number), `sequence u64`, `counter_a u64`, `counter_b u64`, `metric_micros u64`, `timestamp_nanos u64`.
   - **Identity tail.** These are the fields `DiagEvent` lacks today (inventory gap: "no module or correlation field in DiagEvent"):
     - `module u32` (`ModuleId`, opaque, assigned by the host registration);
     - `reserved u32`, zero;
     - `correlation u64` (observe's existing correlation id);
     - `session` and `process`, each an opaque 128-bit placeholder stored as `[u64; 2]` so the record stays align 8. `process` is PID plus process birth, filled only by the host.
     - Suggested size: 104 bytes. You may reorder the tail. State the offset table in the module docs and the `DESCRIPTOR`, and pin it in a test.
   - **Construction.** Only from typed parts: `RingCode`, `RingPhase`, `RingSeverity`, and the opaque id newtypes `ModuleId(u32)`, `ThreadRef(u64)`, `CorrelationRef(u64)`, `SessionRef([u64; 2])`, `ProcessRef([u64; 2])`.
     - No `String`, `&str`, `Vec`, slice or `From<&str>` in any public type of the module.
     - Observe never derives an id from text: no hashing of a `DomainId`, account or session string. Low-entropy ids are dictionary-reversible.
     - Placeholders default to zero, meaning "not supplied". Observe never invents a process identity (GUI-DESKTOP-050).
2. **`RingCode` and the mapping.** `RingCode` is a closed `#[repr(u16)]` enum with explicit discriminants.
   - **Codes.** One code per `Outcome`: Progress, Success, the four `Failure(FailureCode)` (Validation, Unavailable, Loss, Unsupported) and Canceled. One code per `DeliveryError`: Saturated, Rejected, Unavailable, Indeterminate.
   - **Reserved codes.** Also reserve the GPU client outcomes that SMB-GPU-CLIENT-PORT-1 types: GpuUnavailable, GpuDeviceLost, GpuAdmissionDenied, GpuBudgetExceeded, GpuDoubleRelease, GpuLateRelease, GpuRebound. Then the GPU follow-up needs no second observe push (observe is upstream of 38 crates). They are declared vocabulary; no producer uses them yet. Say that in the docs.
   - **Discriminants.** Put them in one Studio block, 0x5300..0x53FF, as a PROPOSED allocation, so they never collide with Handshake's codes 0..11 and 65535.
   - **`RingPhase` and `RingSeverity`.** Use the same discriminants as `DiagPhase` and `DiagSeverity`.
   - **Mapping functions.** `ring_of_outcome(Outcome) -> (RingCode, RingPhase, RingSeverity)` and `ring_of_delivery(DeliveryError) -> (...)` are exhaustive matches with no wildcard, so a new variant fails to compile. Declare the phase and severity choices as Studio readings in a docs table:
     - Progress → Tick/Info; Success → End/Info;
     - Failure → End/Warn, or End/Error for Unavailable and Loss;
     - Canceled → End/Warn;
     - Saturated → Degraded/Warn; Rejected, Unavailable and Indeterminate → Degraded/Error.
   - **Host projection.** `DiagEventCode` has no Studio or GUI code at 292d7a1c. Until Handshake allocates them (a prerequisite in the inventory, spec delta D5 "observe-to-ring mapping"), a host projection to today's `DiagEventCode` is `Other` and the code is lost.
     - The docs and `DESCRIPTOR` say so. Never put the code into `counter_a` or `counter_b`; their meaning is per code.
     - Mirror the Handshake discriminants as constants cited by file and commit, for the projection table and its test. It is a contract mirror, not a dependency.
3. **`RingProducerPort`.**
   - **Signature.** `pub trait RingProducerPort: Send { fn try_record(&mut self, record: &RingRecord) -> Result<(), RingRefusal>; }`. `RingRefusal` is a closed enum: `{ Saturated, Unavailable }`.
   - **Contract, in the docs.** Non-blocking, non-allocating, constant bounded work, no lock, no syscall, callable from real-time threads. The trait takes `&mut self`, so one producer handle per thread matches the host's SPSC `DiagRingWriter`. No lock appears in the trait contract.
   - **Real-time audio callbacks never call observe** (rule 6). The port is for the host's preallocated hand-off, not for those callbacks.
   - `SinkPort`, `Observe::emit*` and wire versions 1-5 stay byte-identical. The ring port is not wired into `emit` in this lane; that is RF-04 phase 1.
4. **Constructors from observe state.**
   - `RingRecord::terminal(&Receipt, identity, timing)`: code from the receipt outcome, sequence from the receipt, `counter_a` = delivered bytes.
   - `RingRecord::delivery_failure(DeliveryError, attempted_sequence, identity, timing)`.
   - Document the counter meaning per code in one table.
5. **`DESCRIPTOR`.** Add one `ring_port` key: record layout, code table, projection, contract and the "not a recorder" statement. The JSON must stay valid.

### Public API: additive only

- No new variant on `Outcome`, `FailureCode`, `DeliveryError`, `DeliveryClass` or `Error`. Dependents match on them: chronicle, components, composite and controls grep hits.
- No changed signature, no changed constant (`MAX_FRAME_BYTES`, `MAX_PROGRESS_EVENTS`, `WIRE_VERSION`).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

## host_integration

```json
{
  "schema_id": "hsk.studio_mt_host_integration@1",
  "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030",
  "spec_version": "v02.225",
  "gui_host": "none",
  "gpu": "none",
  "diagnostics": {
    "internal_diagnostics": "producer_through_observe",
    "flight_recorder": "none",
    "palmistry": "none"
  },
  "control_descriptors": "none",
  "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing",
  "jobs": "none",
  "settings": "none",
  "spec_refs": ["GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002"],
  "refactor_exposure": "medium",
  "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief",
  "notes": {
    "internal_diagnostics": "phase 0 producer shape only; the host adapter onto DiagRingWriter and the RT-safe hand-off are Handshake work (MT-36190, MT-36233)",
    "palmistry": "none in phase 0: no ring exists on the Studio side; ring visibility is proven in phase 2 on the Studio pane (MT-36233, MT-36251)",
    "attribution": "session and process are opaque placeholders filled only by the host; observe never invents a process identity (GUI-DESKTOP-050)",
    "exposure": "observe is a direct dependency of 38 crates at 3cf0f149 (the inventory says 35); the change is additive, so no dependent edits"
  },
  "recorded_at": "2026-10-31T00:07:10Z"
}
```

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (`master-spec-v02.225`)

- `[STU-VAL-002]` (14:22682): Studio modules implement Flight Recorder, internal_diagnostics and Palmistry posture "through shared typed diagnostic ports and existing host owners". Asset-only proof exercises the port; native embedding proof is scheduled separately.
- `[STU-ARC-010]` (14:215): observe is a catalogued module root with a declared semantic owner.
- `[GUI-DESKTOP-049]` (15:130): GUI actions, errors and GPU failures are observable through the existing internal_diagnostics producer and its typed mechanical allowlist.
- `[GUI-DESKTOP-050]` (15:132): stable ids, real correlation and session identity, exact PID and process-birth binding. Correlation is not permission or an invented process identity.
- Inventory SC-4: observe's `correlation_id` is a u64 "not a host acceptance identity". The tail carries it as correlation only.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-observe-ring-port-1`, branch `lane/smb-observe-ring-port-1`, base 3cf0f149)

- observe:
  - new `src/ring.rs`;
  - additive `mod` / `pub use` lines and the `DESCRIPTOR` `ring_port` key in `src/lib.rs`;
  - new `tests/ring_port.rs`;
  - `examples/observe-consumer.rs`, only if `--descriptor` output needs no change beyond the key.
- Do not touch:
  - any other crate, including the 38 dependents (no call-site migration in this lane);
  - `tests/foundation_contract.rs` (existing pins stay as they are);
  - Handshake code;
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run order">

## Proof (tests/ring_port.rs, 5 or fewer per slice)

- **Layout.**
  - `size_of`, `align_of` and every field offset equal the table. The first 56 bytes equal the `DiagEvent` offset table, kept as a test constant cited to `schema.rs` @292d7a1c.
  - Reserved bytes are zero for every constructor.
- **Mapping.**
  - Every `Outcome` (all four `FailureCode`) and every `DeliveryError` maps to a distinct `RingCode`.
  - The `ALL` arrays equal the exhaustive-match cases.
  - Discriminants are pinned; every code sits in the Studio block and none equals a Handshake code.
- **No text crosses.**
  - Build `Observation`s with a decoy `private_project_text` and a `DomainId` / `ActorContext` holding decoy strings.
  - Produce a record per outcome and delivery error, serialise each, and assert that no decoy byte sequence (and no 4-byte run of input text) appears.
  - Add `compile_fail` doctests: `RingRecord` built from a `&str`, and `ModuleId::from("x")`.
- **Port contract.**
  - A test-local producer (a fixed array in the test file, never a type in observe) records from a second thread through `&mut`.
  - Saturation is the typed `Saturated`, and a drop counter is visible.
  - Prove no allocation with a counting global allocator in the test binary if the workspace lints allow `unsafe` there. Otherwise say why not in the notes and rely on the type-level argument (`Copy` record, `Copy` result).
- **Regression.** `tests/foundation_contract.rs` stays green and unchanged (wire v1-v5 bytes).

## Closure and run timing

- **Dependents.** Observe is a direct dependency of 38 crates at 3cf0f149:
  - chronicle, components, composite, controls, develop;
  - harness, harness-core, harness-view;
  - interop-fig, interop-fig-base, interop-layout, interop-lrcat, interop-media, interop-psd, interop-raster, interop-tiff, interop-vector, interop-vector-core;
  - kernel-bindings, layout, motion, motion-js, nib, package, pigment, press, prism, pulse, reel, render-cpu, render-gpu;
  - score, score-device, score-plugin, session, type, web, web-core.
- **The bundled run (rules 4a, 4b and 4d):**
  - step 0 first;
  - clippy over observe and all 38 dependents (lane-check selects them; this is close to a whole-workspace clippy);
  - the WHOLE observe suite and observe doctests (the new `compile_fail` cases);
  - the harness-core and harness suites, because harness-core embeds `hsk_studio_observe::DESCRIPTOR` (`src/harness.rs:189`) in its descriptor and manual surface (rule 4b).
  - No other dependent pins the new items, so their test suites are not required.
- **Disk:** run on D: (`--disk d`) unless the lead names C:. R45 holds C:.
- **Wait for the lead:** do NOT start the bundled run until the lead messages you with the disk and the base.
- **Logs:** name the log per RUN LOGS.
- **Push order:** observe is upstream of everything, so push LAST, after SMB-GPU-CLIENT-PORT-1 and SMB-R44-FOLLOWUPS-1. Rebase first. Observe depends only on accord, so re-run only if the rebase touched observe or accord. The push invalidates the closure of every later lane; say so in the READY note.
- Set READY_FOR_VALIDATION on a pushed commit (workflow-v3 keys). Never claim PASS.

## Follow-ups (not this lane)

- **Handshake:**
  - `DiagEvent` module, correlation, session and process fields, or a versioned record;
  - Studio and GUI `DiagEventCode` allocation;
  - an RT-safe preallocated `record()` hand-off;
  - the host adapter `RingProducerPort` → `DiagRingWriter` (MT-36190, MT-36233).
- **Studio:**
  - wire `Observe::emit*` and the worker heartbeat to the ring port (RF-04 phase 1);
  - map render-gpu's client outcomes to the reserved GPU codes (after SMB-GPU-CLIENT-PORT-1).

</topic>
