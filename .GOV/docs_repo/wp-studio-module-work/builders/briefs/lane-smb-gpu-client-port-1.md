---
file_id: LANE-SMB-GPU-CLIENT-PORT-1
file_kind: lane_brief
updated_at: 2026-10-31T00:07:10Z
wp: WP-KERNEL-STUDIO
lane: SMB-GPU-CLIENT-PORT-1
lane_file: builders/lanes/hsk-studio-gpu-client-port-1.json
crates: [hsk-studio-render-gpu, hsk-studio-harness (one dev-dependency feature line in Cargo.toml)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="RF-03 phase 0: typed GPU client port in render-gpu plus a transient in-process adapter; asset side of MT-36248">

## Target

This is RF-03 phase 0 of `research/integration-refactor-inventory-20261030.json`. It is the asset side of MT-36248: read `.GOV/task_packets/WP-KERNEL-STUDIO/MT-36248.json`, the `closure_unit` AC-HOST-001..006 and the `host_integration` block.

Today render-gpu has two device owners:
- `GpuContext::acquire` (`src/device.rs:210-280`) creates its own instance and device. On Windows `Auto` tries DX12 before Vulkan (`:241`).
- `SharedDevice` (`src/plan_sink/shared.rs:393-417`, feature `shared-device`) adopts eframe's device.

The residency budget is a per-backend 256 MiB (`DeviceOptions::default`, `residency_budget_bytes`, `src/plan_sink.rs:1382`).

The target is that render-gpu becomes a typed client of the one Handshake GPU owner (MT-203, NOT_STARTED). Until that owner exists, a transient in-process adapter stands in for asset-local proof. Its results are evidence, never host acceptance.

Read first: `src/device.rs`, `src/residency.rs`, `src/plan_sink/shared.rs`, `src/plan_sink.rs:1282-1560` (`State` and `GpuBackend`), `src/error.rs`, and the `lib.rs` `FAULT_HOOKS_ARE_NOT_PUBLIC` compile-check pattern.

The pigment `RenderBackend` trait and `impl RenderBackend for GpuBackend` stay untouched. The wgpu pin stays at 27.0.1 (D2, AC-HOST-006).

Slices, one local commit each, red tests first:

1. **Client port**: a new `src/client.rs`. No wgpu, ash or Vulkan type appears in any public signature.
   - **`GpuClientPort: Send + Sync`:**
     - `admit(&self, &AdmissionRequest, &CancellationToken) -> Result<GpuGrant, ClientRefusal>`
     - `settle(&self, &GpuGrant, Settlement) -> Result<SettleReceipt, SettleError>`
     - `rebind(&self, &GpuGrant, &CancellationToken) -> Result<GpuGrant, ClientRefusal>` (device-loss rebind through the owner; generation + 1).
   - **`AdmissionRequest`** carries the attribution on every allocation (GUI-DESKTOP-029, AC-HOST-002):
     - `operation`, a closed `GpuOperation` (compute, upload, readback, display);
     - `module: ModuleRef` (opaque u32), `client: ClientRef` (opaque u64), `device: DeviceRef` (opaque u64, or any);
     - `requested_bytes`.
   - **`GpuGrant`** is not `Clone`. It holds:
     - `GrantId` (opaque u64) and `generation`;
     - `budget_bytes`, the residency budget the backend uses;
     - `device: DeviceRef`;
     - a plain `backend` key for the receipt;
     - `issuer: GrantIssuer { TransientAssetLocal, Host }`.
   - **Accounting** (GPU-QUOTA-015: observed, estimated and unknown stay distinct):
     - `ByteQuantity { Measured(u64), Estimated(u64), Unknown }`;
     - `Settlement { retained, peak, evictions, outcome: SettleOutcome { Completed, Canceled, Failed, Lost } }`, taken from the `Residency` counters.
     - Residency bytes are measured charges. Native driver bytes are `Unknown`.
   - **Outcomes:**
     - `ClientRefusal { Unavailable(Reason), Lost { generation }, Denied(DenialCode { BudgetExceeded, QuotaUnavailable, Revoked }), Canceled }`
     - `SettleError { DoubleRelease, LateRelease, StaleGrant, InFlight }`
     - Map them onto the existing `GpuError` at the `GpuBackend` boundary.
   - **Settle-once:**
     - A second settle of a grant is `DoubleRelease`.
     - A settle of a grant from a generation before a rebind is `LateRelease` or `StaleGrant`.
     - A stale grant never authorises a new submission (GUI-DESKTOP-036).
     - Cancellation, a dropped client or a pane close never settles work that is still in flight (GUI-DESKTOP-033/034, AC-HOST-003).
2. **Adapters.**
   - **`TransientAdapter`, asset-local:**
     - Implements the port with the current acquire code moved behind it. The wgpu device and queue travel inside the grant as a crate-private lease.
     - Candidate order: Windows Vulkan, then DX12 (`device.rs:241` flips); Apple Metal; elsewhere Vulkan.
     - The backend actually used is in `DeviceInfo` and the receipt. A DX12 or GL result never proves a Vulkan row (GUI-DESKTOP-026).
     - The grant budget comes from `TransientOptions.residency_budget_bytes`: default 256 MiB, labelled `studio_declared`, asset-local.
     - Its `DESCRIPTOR` entry says `asset_local: true` and that it is excluded from host acceptance (AC-HOST-005).
   - **`UnavailablePort`:** always `Unavailable(no_adapter)`. It is the wgpu-free kill switch that replaces `BackendChoice::None` in tests.
   - **`SharedDevice`** becomes an adapter behind the port:
     - `SharedDevice::new(&wgpu::Device, &wgpu::Queue, AdapterInfo, DeviceOptions)` keeps its signature, so `harness-view` `view/display.rs:30` compiles unchanged.
     - Internally it builds an external-device adapter and an `issuer: Host`-shaped grant (budget from `options` until a host supplies one).
     - Loss detection stays the host's, as today.
     - This is the one place where a wgpu type crosses the boundary, behind the existing `shared-device` feature. No new wgpu-naming public item is added (rule 1).
3. **`GpuBackend` on the port.**
   - `GpuBackend::with_grant(port: Arc<dyn GpuClientPort>, request: AdmissionRequest, options: DeviceOptions) -> Self` keeps the Ready / Lost / Unavailable / Failed states.
   - The residency budget is `grant.budget_bytes`, not `options.residency_budget_bytes`.
   - `reacquire` becomes `port.rebind`.
   - An explicit `release(self) -> Result<SettleReceipt, SettleError>` settles once with measured `Residency` numbers. Drop without release records the settlement as abandoned, with a counter; it never leaks silently.
   - `GpuBackend::new(DeviceOptions)` stays as the asset-local compatibility constructor, equal to `with_grant(TransientAdapter)`. `harness-view` `view/mod.rs:744` still calls it. Mark it asset-local in the docs, but add no `#[deprecated]`, because clippy with deny-warnings in harness-view would fail.
   - The `test-faults` constructors (`with_shader_source`, `with_injected_loss`) keep working through the transient adapter.
4. **The public API sheds the device owners.**
   - `GpuContext::acquire` and `acquire_next` become `pub(crate)`, and `GpuContext` leaves the `pub use`. Grep found no public signature outside render-gpu's own tests that needs it.
   - `BackendChoice` becomes crate-private. It stays test-only through a non-default feature, proposed name `asset-local-backends`. Its `DESCRIPTOR` says asset-local, and no product build enables it. That feature also gates the `DeviceOptions.backend` field.
   - Test-only users of the feature:
     - `tests/gpu_plan.rs:377` and `tests/text_render.rs:46` (`HSK_STUDIO_TEST_GPU_BACKEND` selection);
     - `examples/render-gpu-consumer.rs:15-53`;
     - `hsk-studio-harness` `tests/viewer_headless.rs:92`, through ONE added dev-dependency line in `hsk-studio-harness/Cargo.toml` that enables the feature.
   - Tests that only need "no device" move to `UnavailablePort`. Nine render-gpu test files use `BackendChoice::None` at 3cf0f149; grep them.
   - The feature must be on in every round unit that runs these tests (rule 4e). Use a self dev-dependency or `required-features`, and check `.config/round-features.toml` (render-gpu units: `test-faults`, `shared-device`, both). If a steward unit is needed, say so in the notes before READY.
   - `harness-view` needs no edit: it names `DeviceOptions` and `GpuBackend` only, and `BackendChoice` only in a doc comment (`view/mod.rs:288`).
5. **Diagnostics and `DESCRIPTOR`.**
   - Every terminal port outcome (unavailable, lost, denied, budget exceeded, double or late release, rebound) is emitted through the existing `report` → observe path as closed codes and counters, with no text (rule 5). The ring codes come later: SMB-OBSERVE-RING-PORT-1 reserves them and pushes after this lane.
   - Rewrite the `DESCRIPTOR` `device` section: client port, transient asset-local adapter, Vulkan first, `BackendChoice` test-only, `SharedDevice` as an adapter. Run every test that pins `DESCRIPTOR` keys.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

## host_integration

```json
{
  "schema_id": "hsk.studio_mt_host_integration@1",
  "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030",
  "spec_version": "v02.225",
  "gui_host": "none",
  "gpu": "central_broker_client",
  "diagnostics": {
    "internal_diagnostics": "producer_through_observe",
    "flight_recorder": "none",
    "palmistry": "ring_visible"
  },
  "control_descriptors": "none",
  "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing",
  "jobs": "none",
  "settings": "none",
  "spec_refs": ["GUI-DESKTOP-025", "GUI-DESKTOP-026", "GUI-DESKTOP-028", "GUI-DESKTOP-029", "GUI-DESKTOP-030", "GUI-DESKTOP-032", "GUI-DESKTOP-033", "GUI-DESKTOP-034", "GUI-DESKTOP-036", "GUI-DESKTOP-038", "GPU-QUOTA-010", "GPU-QUOTA-013", "GPU-QUOTA-014", "GPU-QUOTA-015", "NATIVE-RENDER-BOOT-007", "STU-ARC-002", "STU-ARC-018"],
  "refactor_exposure": "high",
  "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief",
  "notes": {
    "gpu": "render-gpu is a typed client; the transient adapter is asset-local evidence only (MT-36248 AC-HOST-005); the real owner, issuer and quota are MT-203 work",
    "palmistry": "every GPU failure is a closed observe code so the host ring and Palmistry can show it in phase 1; no ring exists in phase 0",
    "flight_recorder": "MT-36248's business_event_ids row stays with the host MT and kernel-bindings; this lane emits no business event",
    "jobs": "admission runs through the GPU client port, not the session JobPort; this lane adds no queue, worker or job"
  },
  "recorded_at": "2026-10-31T00:07:10Z"
}
```

</topic>

<topic id="paths" status="active" summary="Owned paths, do-not-touch and the GUI follow-up list">

## Owned paths (lane worktree `wt-studio-lanes/smb-gpu-client-port-1`, branch `lane/smb-gpu-client-port-1`, base 3cf0f149)

- render-gpu:
  - new `src/client.rs` (and an adapter module if you split it);
  - `src/device.rs`, `src/plan_sink.rs` (the `GpuBackend` construction, state and release paths only), `src/plan_sink/shared.rs`, `src/residency.rs` (budget input only), `src/lib.rs`, `src/plan_sink/refusal_keys.rs` (its `BackendChoice::None` uses), `src/error.rs` (additive);
  - `Cargo.toml` (the feature);
  - `examples/render-gpu-consumer.rs`;
  - tests: every file that names `BackendChoice` or `GpuContext`, plus new `tests/client_port.rs`.
- harness: ONE dev-dependency line in `Cargo.toml` that enables the test-only feature. No source edit.
- Do not touch:
  - every harness-view file. The display path (`view/display.rs`, `view/render.rs`) is GUI code;
  - pigment's `RenderBackend` trait and `render_plan`;
  - composite;
  - observe (SMB-OBSERVE-RING-PORT-1);
  - every crate SMB-R44-FOLLOWUPS-1 owns (pigment, prism, pulse, composite, layout, interop-vector-svg, interop-fig-native, harness `src/cli.rs`, harness-core);
  - the wgpu, ash and gpu-allocator pins;
  - root inputs and the Master Spec.

## Call sites the GUI lane must change later (listed, not done here)

- `harness-view` `src/view/mod.rs`:
  - `:209-210`: `Viewer` holds `Option<GpuBackend>` and `DeviceOptions`;
  - `:285-290`: `with_gpu_options`;
  - `:744`: `GpuBackend::new(self.gpu_options)`.
  - These must take a host port and request a grant attributed to harness-view's module and client.
- `harness-view` `src/view/render.rs:198-261` and `:318-360`: the GPU, then render-cpu fallback chain becomes a typed, receipted choice. There is no silent CPU fallback (AC-HOST-004, GUI-DESKTOP-038).
- `harness-view` `src/view/display.rs:11, 22, 30`: `SharedDevice::new` on eframe's `RenderState` device changes to a host-granted device. The eframe device ownership ends with D1/D7.
- `harness-view` `src/view/info.rs:683` and `src/view/proof.rs:995`: the shared-device NOT_PROVEN notes are restated against the grant.
- harness `src/bin/studio-harness.rs`: eframe `Renderer::Wgpu` owns a window device, which is the asset-local test host only (SC-2).
- harness `tests/viewer_headless.rs:92`: `BackendChoice::None` changes to `UnavailablePort` through a `Viewer` API that takes a port. Then the dev-dependency feature line goes.

</topic>

<topic id="proof" status="active" summary="Proof, closure and run order">

## Proof (5 or fewer focused tests per slice)

- **Port** (`tests/client_port.rs`, no adapter needed):
  - admission carries the module, client and device;
  - settle once; a second settle is `DoubleRelease`;
  - a settle after a rebind is `LateRelease` or `StaleGrant`;
  - a canceled admission is `Canceled` with no residency charge.
- **Adapters:**
  - the candidate order is a pure function tested per target (Windows `[Vulkan, Dx12]`);
  - `UnavailablePort` gives `Unavailable(no_adapter)` with no sink call;
  - the backend's residency budget equals `grant.budget_bytes`;
  - a live run admits, executes a small plan with `HSK_STUDIO_TEST_GPU_BACKEND` unset (Vulkan first) and settles with `Measured` retained and peak equal to the residency counters;
  - the `require_gpu = true` policy in `tests/gpu-proof.toml` holds: no adapter fails NOT_PROVEN and never passes.
- **Loss** (`test-faults`): `simulate_device_loss` gives `Lost { generation }`; `rebind` gives generation + 1; the old grant is refused for submission and settle.
- **Public API** (`compile_fail` doctests, the `lib.rs` pattern):
  - without the feature, `hsk_studio_render_gpu::BackendChoice` and `GpuContext::acquire` do not resolve;
  - in a default build, no public item names a wgpu type (`SharedDevice` stays behind `shared-device`).
- **SharedDevice:** the existing `shared_device`, `gpu_submit_fault` and `gpu_extended_envelope` shared-device tests stay green. A new case shows its backend holds a grant whose budget comes from `options`.
- **Vulkan proof:** run the GPU tests once with `HSK_STUDIO_TEST_GPU_BACKEND=vulkan`. Record the `DeviceInfo` text actually used in the notes; a DX12 result proves no Vulkan row.

## Closure and run timing

- **Dependents.** render-gpu's direct dependents are harness-view and harness only. Its dependencies (prism, pigment, observe, accord) are changed by SMB-R44-FOLLOWUPS-1 and SMB-OBSERVE-RING-PORT-1, and both push after this lane.
- **The bundled run (rules 4a, 4b, 4d and 4e):**
  - step 0 first;
  - clippy over render-gpu, harness-view and harness;
  - the WHOLE render-gpu suite in all four units (default, `test-faults`, `shared-device`, `shared-device` + `test-faults`) with the test-only feature on, plus its doctests;
  - the harness-view suite (`["viewer"]`, `["desktop", "audio-batch"]`) and the harness suite (the units that build `viewer_headless`: `["viewer"]`, `["desktop", "viewer"]`), because render-gpu's public API changes and they use it (rule 4b).
- **GPU test lock:** a test run selecting render-gpu waits for the machine-wide GPU test lock, and rounds go first. Expect that wait during R45; never bypass it.
- **Disk:** run on D: (`--disk d`) unless the lead names C:. R45 holds C:.
- **Wait for the lead:** do NOT start the bundled run until the lead messages you with the disk and the base.
- **Logs:** name the log per RUN LOGS.
- **Push order:** this lane pushes FIRST of the three, unless the lead orders otherwise. render-gpu is upstream of harness-view and harness only. Then SMB-R44-FOLLOWUPS-1, then SMB-OBSERVE-RING-PORT-1. Rebase first, and re-run if the rebase touched render-gpu or its dependencies.
- **Round risk:** rounds unset `HSK_*`, so after this push the round's GPU proofs run on Vulkan first on Windows instead of DX12. Name this in the READY note, so the lead can check the validator host's Vulkan driver and the tolerances, which were measured on DX12.
- Set READY_FOR_VALIDATION per slice on a pushed commit (workflow-v3 keys). Never claim PASS. MT-36248's HOST-CASES check stays PLANNED; this lane is asset-local evidence for AC-HOST-001..006, never host acceptance.

</topic>
