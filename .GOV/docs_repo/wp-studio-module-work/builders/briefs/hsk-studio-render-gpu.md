---
file_id: BUILDER-BRIEF-HSK-STUDIO-RENDER-GPU
file_kind: builder_brief
updated_at: 2026-10-11T05:40:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-render-gpu
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-RENDER_GPU` / key `render_gpu` / crate `hsk-studio-render-gpu`; foundation MT-35546 (test target `gpu_plan`, cases `gpu_readback_known_reference`, `device_loss_and_unavailable_are_explicit`, `residency_eviction_budget_cancel`, `stale_revision_sink_refused`); write scope `handshake-studio-assets/crates/hsk-studio-render-gpu/**`. Files per MT: `src/device.rs`, `src/residency.rs`, `src/plan_sink.rs`, `tests/gpu_plan.rs`.
- Responsibility (verbatim): "wgpu backend/device lifecycle, residency and budgets; GPU API types hidden from document/contracts."
- Foundation capability: "Execute real bounded GPU plan/readback at supported backend; explicit unavailable/device-loss result, never skipped PASS."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-prism`, `hsk-studio-pigment`, `hsk-studio-nib`, `hsk-studio-type`, `hsk-studio-observe`. NO `folio`, NO `render-cpu`, NO `composite` edge. Slice uses accord, pigment, observe.
- Forbidden: handshake_core/native/SurrealDB; wgpu/WGSL types in ANY public signature (private leaf, STU-ARC-002; `handshake_core` never gains GPU deps); implicit CPU fallback (explicit `Unavailable`); skipped-as-pass tests.
- Conventions: edition 2024, rust-version 1.97.1 (wgpu 27.0.1 MSRV 1.88 ok), path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `render-gpu-consumer` (`--descriptor`), per-test deadline 60 s per prep. Root `Cargo.toml` steward-owned; the wgpu pin needs a dep-request.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-ARC-002 (l.~189): GPU/CPU render providers are independently selectable crates behind typed `Send + Sync` ports; no GPU dep in `handshake_core`; no mandatory all-engine facade. STU-FX-036/037 (l.19634): `EFFECT_GPU_UNAVAILABLE` is a determinate state, never a silent empty result; output byte-stable on a fixed backend and within tolerance across backends (promotion equivalence, 14.24 l.22678). STU-COL-163: compositor receives buffers already in working/blending space.
- STU-ARC-008/012/013 (l.~203-220): bounded admission of CPU/GPU memory, byte-ownership leases, release on every exit, cancel/deadline observed between bounded chunks, device allocations counted; STU-ARC-015: render receipt binds revision, renderer/version, profile, frame/region; unsupported fallback never proves the result; STU-ARC-016: unavailable device = typed failure, optional early-return test paths are NOT_PROVEN.
- MT-35546 acceptance (verified): "Execute real bounded GPU plan/readback at supported backend; explicit unavailable/device-loss result, never skipped PASS ... backend-qualified real device proof, no skip-as-pass or implicit fallback"; independent oracle = analytic/original-produced output; "CPU-vs-GPU equality alone cannot prove both correct".
- Later scope: full WGSL effect catalogue (14.9 GPU rows `Req`), tile paging/mip, f16 accumulation, multi-GPU, surface/window presentation (host), `_mt_plan_09_effects.json`, `_mt_plan_04_raster.json` (MT-017 RasterEngine).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (Rust + one WGSL compute shader, ~700-1000 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[7]` (MT-35546; "Bounded selected-backend upload/plan execution/readback with tile residency and byte leases; retire device generations safely, never label missing device PASS"; provider "Pinned Photocraft wgpu30 source is inspected candidate; exact backend Cargo features and device oracle remain unresolved. GPU types private to leaf"); Craft reference (pinned cache `.../photocraft-175603a6.../crates/gpu/src/{lib.rs 1929 LOC, plan.rs, compose.wgsl, health.rs, bounds.rs}`; MIT OR Apache-2.0): `Compositor::{set_resident_budget,set_memory_budget,working_set,fits_budget,render_to_vec}`, `DeviceHealth/Fault`, `half` f16 helpers - read for structure only; its comment "Same major version as eframe's wgpu so the app's device can be shared" is the reason for the version decision below. `.GOV/reference/studio_app_feature_research/19-studio-local-first-rust-posture.md`.
VERSION CONTRADICTION (flag to lead): Craft pins wgpu 30 (its eframe 0.36); the HOST lock `handshake_main/src/frontend/handshake_native/Cargo.lock` has eframe/egui-wgpu 0.33.3 -> `wgpu 27.0.1`, accesskit 0.21.1 (verified). To keep a later host device-sharing adapter possible, the slice pins `wgpu =27.0.1` (crates.io: MIT OR Apache-2.0, MSRV 1.88, created 2025-10-02, not yanked). Wgpu 30 would be a separately scoped upgrade (STU-ARC-018).
Pixel contract (pigment, verified): `rgb_f32le` colour plane + SEPARATE straight `f32le` alpha plane, `linear_light`, stride multiple of 4; `pigment::{Rect, Grid}`.
1. `device.rs`: `pub struct GpuContext` (private wgpu fields), `pub struct DeviceOptions{backend:BackendChoice::{Auto,Vulkan,Dx12,Metal,None}, max_buffer_bytes:u64}`, `pub fn acquire(opts:&DeviceOptions, cancel:&CancellationToken) -> Result<GpuContext, GpuError>` headless (no surface/window, no foreground); no adapter / `BackendChoice::None` -> `GpuError::Unavailable(Reason::NoAdapter)` (REAL typed outcome, never a panic or skip); `GpuContext::info() -> DeviceInfo{backend:&'static str, adapter_kind:Discrete|Integrated|Cpu|Unknown, generation:u64}`; device loss (callback or poll error) latches `GpuError::DeviceLost{generation}` for the next call and increments `generation` on re-acquire (stale generation handles refused).
2. `residency.rs` (pure Rust, testable WITHOUT a GPU): `Residency{budget_bytes:u64, used:u64}` with LRU over `TileKey{source:String, bounds:Rect}`; `reserve(key, bytes)->Result<ReservationId, GpuError::BudgetExceeded{requested,budget}>` evicting least-recently-used unpinned entries first; single entry > budget -> `BudgetExceeded`; `release` on drop/error/cancel; counters `peak_used`, `evictions`.
3. `plan_sink.rs`: `pub struct GpuPlan{revision:u64, extent:Rect, profile_sha256:[u8;32], layers:Vec<GpuLayer{rect:Rect, colour:Vec<u8>, alpha:Vec<u8>, blend:Blend::{Normal,Multiply,Screen}, opacity:f32}>}`; `pub fn execute(ctx:&mut GpuContext, plan:&GpuPlan, expected_revision:u64, res:&mut Residency, cancel:&CancellationToken, sink:&mut dyn PixelSink) -> Result<GpuReceipt, GpuError>`: validates stale revision BEFORE any upload, uploads each layer into a storage buffer (`array<vec4<f32>>`, no texture-format limits), one WGSL compute pass per layer using the SAME W3C straight-alpha formula as render-cpu (`ao=as+ab*(1-as); C=((1-as)*ab*Cb+(1-ab)*as*Cs+as*ab*B)/ao`), maps a staging buffer, splits into the canonical colour/alpha planes and hands `Rect`+bytes to `sink` in chunks (cancel checked between layers/chunks; sink error aborts and releases residency). `GpuReceipt{backend, adapter_kind, generation, revision, profile_sha256, extent, bytes_uploaded, bytes_read_back, peak_resident_bytes, evictions, renderer:"hsk-studio-render-gpu/0.1"}`.
4. Failure semantics: `GpuError::{Unavailable(Reason), DeviceLost{generation}, StaleRevision, BudgetExceeded{..}, UnsupportedBlend(u16), InvalidInput, Canceled, SinkRejected}`; after any error the input `GpuPlan` is untouched and residency `used` returns to the pre-call value (assert in tests).
5. Observability: `DESCRIPTOR` + `report(...)`: `Unavailable|DeviceLost`->`Failure(Unavailable)`, `BudgetExceeded|UnsupportedBlend`->`Failure(Unsupported)`, `StaleRevision|InvalidInput`->`Failure(Validation)`, `Canceled`->`Canceled`; example `render-gpu-consumer --descriptor | --probe` (prints backend/adapter kind or the `Unavailable` reason).
Hand-computed oracle (same numbers as render-cpu): backdrop (0.2,0.4,0.6,1.0) + source (1,0,0,1) at opacity 0.5: Normal -> (0.6,0.2,0.3,1.0); Multiply -> (0.2,0.2,0.3,1.0); tolerance 1e-6 per channel on f32 (GPU `mix`/fma may differ in the last ulp - record the observed max error in the receipt/test output; do NOT assert bit-equality across backends).
Oracle note: not one of the Adobe-behaviour crates; GPU equality is judged against hand-computed values and (via the validator, not a crate edge) against render-cpu output; Adobe-referenced parity is owned by render-cpu/composite.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- `wgpu =27.0.1`, `default-features=false`, features `["std","wgsl","vulkan","dx12","metal"]` (names confirmed on docs.rs features page for 27.0.1; DEFAULT set also enables `gles`,`webgpu`,`web`,`parking_lot` - exclude them; `static-dxc`/`spirv`/`glsl`/`naga-ir` stay off). Pure-Rust API; native GPU drivers via platform FFI (allowed as a separately selected optional provider leaf, STU-ARC-016). Exact 27.0.1 API names (`request_adapter` result type, `device.poll`, device-lost callback, buffer-mapping calls) are UNVERIFIED here - read `docs.rs/wgpu/27.0.1` before coding.
- `pollster 1` (or a 20-line `std` `block_on` using `Waker::noop`/thread park) for async adapter/device request - prefer the std helper to avoid a crate; `bytemuck =1.25.2` (already pinned by type; for `cast_slice` of f32 uploads, Pod alignment) optional; `half 2` only if f16 accumulation is added later.
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-render-gpu --tests` (then `clippy`, `test --test gpu_plan`). wgpu pulls a large dependency closure: this crate is a heavy build - build it once, in the shared warm target, never `--workspace`.

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/gpu_plan.rs">

## Tests (<=5, deadline 60 s; MT case names)
1. `gpu_readback_known_reference`: needs a real adapter; runs the oracle above and asserts per-channel error <= 1e-6; if `acquire` returns `Unavailable` this test FAILS with the text `NOT_PROVEN: no adapter` (never early-return Ok, never `#[ignore]` silently) - the validator host must have a supported backend.
2. `device_loss_and_unavailable_are_explicit`: hardware-independent: `BackendChoice::None` -> `Unavailable(NoAdapter)`; an injected lost-device flag (test hook behind a `#[cfg(test)]` method that latches the same internal state as the real callback) -> next `execute` returns `DeviceLost{generation}` and a re-acquire bumps `generation`.
3. `residency_eviction_budget_cancel`: pure `Residency`: LRU eviction order, single oversize -> `BudgetExceeded`, `used` returns to baseline after cancel/sink error; counters exact.
4. `stale_revision_sink_refused`: wrong `expected_revision` -> `StaleRevision`, sink never called, no upload counter movement; sink returning error mid-read-back -> `SinkRejected` and residency released.
5. `no_wgpu_types_in_public_api`: compile-time/text guard - the crate's public items (via `cargo doc`-free grep of `src/lib.rs` re-exports in the test) mention no `wgpu::` path (STU-ARC-002 isolation).

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No window/surface/swapchain/egui integration (host owns presentation), no sharing of the host device (adapter later), no f16/HDR accumulation, no effect catalogue/filters/blur, no tile paging beyond the byte-budget LRU, no multi-GPU, no CPU fallback inside this crate, no wgpu 30 upgrade, no plan lowering (composite) or Folio edge.
- Report READY_FOR_VALIDATION only; never self-PASS. Record the actual adapter/backend used in the lane file (backend-qualified proof).

</topic>
