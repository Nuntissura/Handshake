---
file_id: BUILDER-BRIEF-HSK-STUDIO-SCORE-DEVICE
file_kind: builder_brief
updated_at: 2026-10-11T04:20:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-score-device
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-SCORE_DEVICE` / key `score_device` / crate `hsk-studio-score-device`; foundation MT `MT-35559` (read `implementation_steps` 1-4; depends_on MT-35541 score). Write scope `crates/hsk-studio-score-device/**` only.
- Responsibility (verbatim): "Optional native device I/O provider; all configuration and allocation off realtime callback; preallocated fixed-capacity handoff."
- Foundation capability: "Actual controlled input/output provider opens, transfers bounded blocks and reports device loss, overrun/discontinuity/underrun without callback blocking."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-score`, `hsk-studio-observe`. Needs the `score` crate => wave 2 (after `hsk-studio-score`).
- Forbidden: handshake_core/native/SurrealDB; egui/eframe/wgpu; any device API in `score` itself; plugin hosting (score-plugin); a native audio crate reachable from default features (the default build of this crate MUST be pure Rust); process-global device registry; blocking, allocating, locking, logging or doing I/O inside the audio callback.
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[lib] path="src/lib.rs"`, `[features] default = []`, `[[example]] name="score-device-consumer"`, `DESCRIPTOR`, `tests/foundation_contract.rs` + `bounded_case`.
- POLICY BLOCKER (lead/steward): `tools/workspace-probe.rs` `pure_registry_name` rejects any registry name ending `-sys`/`_sys` (and `image`, `wgpu`, ...), and each owner needs an entry in the probe's `approved` list. `cpal 0.18.2` pulls `alsa`->`alsa-sys` on Linux; because `Cargo.lock` is platform-universal, the lock would contain `alsa-sys` even if only built on Windows. => the native backend needs an explicit, scoped policy exemption for this crate's optional feature (or a Windows-only candidate, see deps). Slice 1 below is deliberately buildable WITHOUT any native crate so work is not blocked.
- Prep refs: `docs_repo/studio-collaboration-2026-10-09/modular-authoring/media-team/preparation.json` -> `module_proposals.score_device` (oracle: independent controlled real device/loopback capture observes channels/sample counts, discontinuity) and `observed_source_findings[0]` (SoundCraft playback callback drains commands and reallocates blocks - violates STU-ARC-013); `backend-redteam/pdf-audio-root-drill.json` root `SOUND-CONTROL-PLAYBACK`/`SOUND-RECORD-DELIVERY` + `ownership_ports`; `repass-2026-10-09/filmcraft-root-drill.json` FILM-ROOT-07 (verified pattern to KEEP: 200 ms lead, 1024-frame blocks, atomic SPSC ring, callback copies/zero-fills and counts underruns; anti-pattern: non-f32 conversion allocating on the callback, eprintln on failure, silent mixer panic). Reference source (MIT OR Apache-2.0, case study): `D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/soundcraft-eac0edd896cd6bdf27c0cdd0f9f9158c8956248a/crates/{audio-io,playback}`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-ARC-013 (l.221): audio callbacks use preallocated bounded handoff and MUST NOT allocate, block on locks, decode, save plugin state, retire foreign graphs or call the locking general recorder; underrun has a defined SAFE output; overrun has a defined discontinuity behaviour; only replaceable uncommitted previews may coalesce (typed superseded outcome); committed recording/export must not silently drop. STU-ARC-021 (l.251): export/bounce receipts report exact requested vs actual sample counts. STU-ARC-008/012 (l.207/219): bounded admission (job count, bytes in flight, decoded samples, device allocations, process count), release on every exit incl. cancel/error/timeout, permits held until native quiescence. STU-ARC-015: typed loss/failure for unavailable device/provider; optional early-return test paths are NOT_PROVEN.
- STU-FX-137 (l.18454): channel label lists (stereo = labels 100,101), sample rate 48000/32000 per sequence. Later: audio-device selection UX (Controls), record/overdub (STU-FX/VID audio), plugin latency compensation, STU-QUIET-001 (no foreground/focus; a device test must not make audible output).
- MT corpus: device feature MTs unknown (UNVERIFIED); foundation only.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (default features = pure Rust, std only, `#![forbid(unsafe_code)]`; ~700-900 LOC)
Reuse (verified): `hsk_studio_accord::{CancellationToken, ActorContext, DomainId}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`; from `hsk-studio-score` (read its brief/lib once landed): `ChannelLayout`, sample-rate/frame types (use its stereo labels 100,101; do not redefine).
1. Provider port + config negotiation (all off-callback): `pub struct StreamConfig{sample_rate_hz:u32, layout:ChannelLayout, block_frames:u32, ring_blocks:u32}` (validated: rate != 0, block/ring bounds, total bytes <= `Limits.max_ring_bytes`); `pub trait DeviceProvider: Send + Sync { fn id(&self)->&'static str; fn enumerate(&self)->Result<Vec<DeviceInfo>, DeviceError>; fn open_output(&self, dev:&DeviceId, cfg:&StreamConfig, grant:&DeviceGrant)->Result<OutputStream, DeviceError>; fn open_input(&self, ..)->Result<InputStream, DeviceError>; }`; `DeviceGrant` is an explicit caller-supplied token (no ambient device access; missing => `Unauthorized`); unsupported device/format => `Unavailable` (never a silent downgrade).
2. Wait-free SPSC ring without `unsafe`: `Vec<AtomicU32>` storing `f32::to_bits`, head/tail `AtomicUsize` with Acquire/Release, capacity fixed and preallocated at open; `Producer::try_push_block(&mut self, &[f32]) -> Result<(), PushError::{Full, WrongLength}>` (control thread) and `Consumer::pop_into(&mut self, out:&mut [f32]) -> usize` (callback thread). `Counters` (all atomics, relaxed): `submitted_frames, consumed_frames, underruns, overruns, discontinuities, callbacks, device_lost`.
3. Callback body as ONE pure function reused by every backend: `pub fn render_callback(c:&mut Consumer, out:&mut [f32], k:&Counters)` = copy available frames, ZERO-FILL the remainder (safe underrun output), bump counters; `pub fn capture_callback(p:&mut Producer, input:&[f32], k:&Counters)` = push what fits, count overrun + discontinuity for the rest. No allocation, no locks, no logging, no formatting - enforce with a test that calls them under a counting `#[global_allocator]` in a dedicated test binary and asserts zero allocations.
4. Real thread-driven software provider (pure Rust proof of the lifecycle, NOT native proof): `ClockedSoftwareDevice` spawns one OS thread that calls the callback at block cadence from an `Instant` schedule, supports `inject_device_loss()`, `stop()`, `close(self) -> CloseReport{permits_released, thread_joined, frames_submitted, frames_consumed, underruns, overruns}`; shutdown performed off the callback thread, `Drop` joins (we own this thread, so stopping it is allowed). Counts are independently verified against the producer's own totals (`submitted == consumed + residual`).
5. Native backend candidate behind feature `cpal-backend` (default OFF, separate leaf): `cpal 0.18.2` (Apache-2.0, updated 2026-08): `Host::default_output_device` -> `supported_output_configs` -> require `f32` else `Unavailable` (or preallocate a scratch buffer at open for `i16`; never allocate in the data callback) -> `build_output_stream(config, data_cb: render_callback..., err_cb: sets device_lost)`; open ONLY after `DeviceGrant`; output is digital silence/pre-rendered test data only (no audible sound; STU-QUIET). Native tests are `#[ignore]` unless `STUDIO_NATIVE_AUDIO=1`, and when no device exists the result is `NOT_PROVEN` (never PASS). `DESCRIPTOR` JSON + `report(...)` into `Observe` (Success/Canceled/Failure(Unavailable|Loss)) + example `score-device-consumer` (`--descriptor`, `--software --blocks N --block-frames F --inject-loss`, `--cpal` when feature on).
Oracle: counters vs independent producer-side totals; wall-clock jitter is reported but not asserted; real device proof (validator, not CI): controlled loopback/capture of the exact submitted blocks (channels, sample counts, discontinuity at capacity+1 offer) - hardware availability UNVERIFIED (no loopback device assumed).

</topic>

<topic id="deps" status="active" summary="third-party deps (crates.io API 2026-10-11)">

## Deps
- Slice 1 (capabilities 1-4): none. Pure-Rust only.
- Native candidate (cap. 5): `cpal 0.18.2` - Apache-2.0, maintained (RustAudio, updated 2026-08-16, ~7.9M recent dl), MSRV 1.85, default features empty; Windows path via `windows`/`windows-core`, Linux `alsa`(-sys), macOS CoreAudio/objc2; **pure-rust: NO** (OS/FFI) and `Cargo.lock` carries `alsa-sys` => needs the probe-policy exemption above. Windows-only alternative: `wasapi 0.25.0` (MIT, updated 2026-10-01; deps log, num-integer, thiserror, windows, windows-core; whether `windows-sys` enters the lock is UNVERIFIED). Both are OS-binding crates, not codecs; they cannot be in default closure. SoundCraft and FilmCraft desktop both use cpal (prep: `pdf-audio-root-drill.json` sources, filmcraft-root-drill `licensed_and_unsafe_boundaries`).
- Rejected: `rodio` (hides callback/allocation behaviour; pulls decoders), `jack`/`pipewire`/`pulseaudio` features, `ringbuf`/`rtrb` for slice 1 (our atomic-ring is ~80 lines and unsafe-free; reconsider `rtrb`, MIT/Apache, only if profiling demands).
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-score-device --tests` (feature build: add `--features cpal-backend` ONLY after the steward approves the dependency).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`; no audible output; no device required for 1-4)
1. `ring_fill_and_overflow_discontinuity`: fill the admitted capacity then offer capacity+1 => `Full`, `overruns` and `discontinuities` increment exactly once; consumed samples equal produced samples in order (independent counter).
2. `empty_ring_underrun_is_safe_silence`: `render_callback` on an empty ring writes only zeros, `underruns` counts, `consumed_frames` unchanged; partial ring => copied frames then zeros.
3. `callback_is_allocation_free_and_lock_free`: dedicated test binary with a counting global allocator: 10_000 callback invocations => 0 allocations; source scan test asserts the callback module contains no `Mutex`, `RwLock`, `Vec::push`, `format!`, `println!`.
4. `software_device_lifecycle_and_loss`: `ClockedSoftwareDevice` streams N blocks, `inject_device_loss()` mid-transfer sets `device_lost`, stop is acknowledged, `close()` reports `thread_joined && permits_released`, frame totals reconcile (`submitted == consumed + residual`).
5. `grant_and_config_rejections`: missing `DeviceGrant` => `Unauthorized` before any resource is created; rate 0, zero block frames, ring over `max_ring_bytes`, unsupported layout => typed errors with no allocation of the ring; `report()` emits one terminal frame per outcome.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No claim of native-device proof from capabilities 1-4 (MT-35559 says simulated rings are insufficient - those only prove the lifecycle/counters), no resampling, no format conversion beyond f32 (and a preallocated i16 scratch), no MIDI, no ASIO/JACK/PipeWire/Pulse, no record-to-file, no plugin hosting, no device-selection UI, no latency auto-calibration, no multi-device aggregate.
- Wave: 2 (needs `score`); native feature additionally blocked on the probe-policy exemption. Report READY_FOR_VALIDATION only for what was proven; native stays `NOT_PROVEN` until an actual device run.

</topic>
