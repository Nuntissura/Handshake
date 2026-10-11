---
file_id: BUILDER-BRIEF-LANE-REEL-NATIVE-ENG-1
file_kind: builder_brief
updated_at: 2026-10-11T13:05:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-reel-native
lane: SMB-REEL-NATIVE-ENG-1
depends_on: [reel-native @ e8b464b (AV1 worker, lane SMB-REEL-NATIVE-AV1-WORKER READY_FOR_VALIDATION, SAFE_TO_JUDGE at R8), reel prerequisites P1 and P2 below (a reel-owner lane), Operator items O1 and O3]
sequencing: start only after SMB-REEL-NATIVE-AV1-WORKER has a verdict (this lane edits its validated host.rs, wire.rs, worker.rs; V4-14/V4-15) and after prerequisite P1 landed in reel; P2 is needed from slice 3 on. One builder per crate.
author_role: researcher (no product code written; no cargo run; no Windows API was executed)
---

<topic id="finding" status="active" summary="What reel-native already provides and what is missing for Media Foundation">

## Finding (reel-native HEAD e8b464b, files read in full or by symbol)
**Exists (do not redo):**
- Isolated AV1 worker pipeline, the pattern to extend: host `WorkerAv1Provider` implements reel's `ExternalVideoDecoder` (`src/host.rs:903-1071`) with a pool (`Pool` `:502`) of child processes, RAII `Permit` for `max_workers` (`:349`), bounded respawn (`admit_spawn` `:547`), HELLO handshake that checks decoder id, protocol version and a stable decoder version (`accept_hello` `:539`), per-request `batch_deadline` and per-segment `segment_deadline` on the monotonic clock, cancel = kill + reap (HELLO wait polled every 5 ms), output cap derived from DECLARED picture dimensions, empty environment, null stderr, `CREATE_NO_WINDOW` (`:89-91`, `:452-458`), only own children are ever killed (`terminate` `:393`), counters (`Av1WorkerStats` `:263`). Typed faults map to reel's `DecoderFault::{Crashed, Deadline, Protocol}` (`:223-242`).
- Wire protocol `src/wire.rs`: length-prefixed, versioned (`PROTOCOL_VERSION 1`), kinds HELLO/OPEN/PACKET/FINISH/OPENED/FRAME/BATCH_END/ERROR (`:25-32`), caps checked before allocation (`MAX_REQUEST_FRAME 32 MiB`, `MAX_FRAMES_PER_BATCH 64`), planes tight as `VideoFormat` documents, errors cross as an index into a fixed 20-entry AV1-only table `WHATS` (`:36-57`).
- Worker `src/worker.rs` (`worker_main` `:212`): panic hook exits 70, dedicated stdin reader thread, 32 MiB decode-thread stack, fault injection only under `test-faults`.
- Tests: `tests/av1_worker.rs` (7 tests: bit-exact vs in-process and dav1d, hostile aborting mutations, supervision matrix, cancel during HELLO, discovery, `hostile_declared_dims_u32_max_are_refused_before_any_spawn`, routed untrusted decode) and `tests/worker_fault_env.rs`.
- Crate shape: `#![forbid(unsafe_code)]`, default build empty, features `av1`, `av1-worker`, `test-faults`; allowed edges accord, reel, observe only (`Cargo.toml`, root `allowed`).

**Missing (this lane's gap):**
- Everything Media Foundation: no `windows` dependency, no MFT enumeration, no H.264 path, no D3D11/DXVA, no NV12 handling. `grep -iE "mftransform|mfplat|IMFTransform|mediafoundation|nv12|heartbeat|shared.?memory|replay"` over `src`, `tests` and `Cargo.toml`: 0 matches; the only `windows` hits are `std::os::windows::process::CommandExt` for `CREATE_NO_WINDOW` (`src/host.rs:453`, `tests/worker_fault_env.rs:40`), i.e. std, not the `windows` crate.
- Heartbeat: none (a hung worker is only caught by the batch/segment deadline).
- Replay from the last sync sample: none (a crash returns `decoder_crashed` to the caller).
- Shared-memory frame ring: none (frames cross stdout pipes, copied).
- Codec-neutral supervision: types and strings are AV1-named (`Av1WorkerOptions`, `WORKER_DECODER_ID = "rav1d"` `:82`, `WHATS`), the wire has no codec field, `OpenRequest.config` is "av1C".
- Hardware vs software is not visible in receipts (`DecoderIdentity.version` is a worker-reported string, interned with caps `:311-330`).

**Reel prerequisites (assign to the reel owner under new lane ids, V4-15; reel is a different crate):**
- P1 `SMB-REEL-H264-ROUTE`: `VideoCodec::H264` (enum is `#[non_exhaustive]`), capture the `avcC` payload in `parse_visual_children` (`reel/src/isobmff.rs:687-732` keeps `colr/pasp/av1C` only), route sample entries `avc1`/`avc3` in `IsobmffProvider::deliver` (`:1717-1739`), and a pure helper module `h264_config.rs` in the style of `av1_config.rs`: parse `avcC` (profile, level, NAL length size, SPS/PPS lists; hostile bounds) and convert one length-prefixed sample into Annex B with SPS/PPS prepended on sync samples. The `avcC` layout lives in ISO/IEC 14496-15: NOT opened, availability UNVERIFIED (O6). The MS decoder needs Annex B with start codes and skips bytes until it finds a valid SPS and PPS (Learn H.264 decoder page).
- P2 `SMB-REEL-PACKET-INFO` (slice 3 only): additive default method on `VideoDecodeSession`, `send_packet(&mut self, packet: PacketInfo<'_>, cancel, out)` with `PacketInfo { bytes, tag, pts_ticks: i64, sync: bool, preroll: bool }`, default body calling `send(bytes, tag, ..)`. Needed for reorder-safe timestamps and for replay (the session must know where the last sync sample is; today `send` sees only bytes and a tag, `reel/src/video.rs:137-151`).

</topic>

<topic id="purpose" status="active" summary="What this lane delivers">

## Purpose
A second isolated native provider next to AV1: H.264 decode through Windows Media Foundation in a worker process, hardware (D3D11/DXVA) when verified, software path always available, decoder identity and path visible in receipts, with the missing supervision features (heartbeat, bounded replay) and a measured decision on a shared-memory ring. Decode only. Encode (hardware MFT encoders are asynchronous, `MFT_ENUM_FLAG_HARDWARE`), HEVC, VP9, AV1-via-MF are later lanes. Windows only; other platforms are `CodecUnavailable`.

Shipping H.264 is a patent/licensing decision for the Operator (STU-VID-084, O1); the feature stays default OFF.

</topic>

<topic id="spec-anchors" status="active" summary="Microsoft Learn and other sources actually opened 2026-10-11">

## Spec anchors (opened this session)
- MFTEnumEx https://learn.microsoft.com/en-us/windows/win32/api/mfapi/nf-mfapi-mftenumex : `MFTEnumEx(guidCategory, Flags, pInputType, pOutputType, pppMFTActivate, pnumMFTActivate)`; flags `MFT_ENUM_FLAG_SYNCMFT | ASYNCMFT | HARDWARE | FIELDOFUSE | LOCALMFT | TRANSCODE_ONLY | SORTANDFILTER`; every MFT falls in exactly one of sync / async / hardware; zero flags = SYNCMFT|LOCALMFT|SORTANDFILTER; hardware MFTs always process asynchronously; the caller releases each `IMFActivate` and `CoTaskMemFree`s the array; zero matches still returns `S_OK`, so the count must be checked.
- Hardware MFTs https://learn.microsoft.com/en-us/windows/win32/medfound/hardware-mfts : must be asynchronous and support dynamic format change; identified by `MFT_ENUM_HARDWARE_URL_Attribute`.
- Asynchronous MFTs https://learn.microsoft.com/en-us/windows/win32/medfound/asynchronous-mfts : `MF_TRANSFORM_ASYNC`, `MF_TRANSFORM_ASYNC_UNLOCK`, events `METransformNeedInput/HaveOutput/DrainComplete/Marker` via `IMFMediaEventGenerator`, `IMFShutdown`; a call before unlock returns `MF_E_TRANSFORM_ASYNC_LOCKED`.
- Basic MFT processing model https://learn.microsoft.com/en-us/windows/win32/medfound/basic-mft-processing-model : call order `NOTIFY_BEGIN_STREAMING`, `ProcessInput`, `ProcessOutput` (`MF_E_TRANSFORM_NEED_MORE_INPUT`, `MF_E_TRANSFORM_STREAM_CHANGE`), `NOTIFY_END_OF_STREAM`, `COMMAND_DRAIN` then `ProcessOutput` until NEED_MORE_INPUT; flush discards input; decoders cannot ignore discontinuities.
- IMFTransform::ProcessOutput https://learn.microsoft.com/en-us/windows/win32/api/mftransform/nf-mftransform-imftransform-processoutput : one sample per stream per call, `MFT_OUTPUT_STREAM_PROVIDES_SAMPLES`, format-change flag, sample attributes copied by the MFT only if `MFPKEY_EXATTRIBUTE_SUPPORTED`. The page says NOTHING about output timestamps of a reordering decoder (see UNVERIFIED).
- H.264 video decoder https://learn.microsoft.com/en-us/windows/win32/medfound/h-264-video-decoder : `CLSID_CMSH264DecoderMFT`; Baseline/Main/High up to level 5.1; 4:2:0 or monochrome only; minimum 48 x 48, maximum 4096 x 2304 (DXVA guaranteed to 1920 x 1088); Annex B input with start codes; outputs I420, IYUV, NV12, YUY2, YV12; attributes `CODECAPI_AVDecVideoAcceleration_H264`, `CODECAPI_AVLowLatencyMode`, `CODECAPI_AVDecNumWorkerThreads`, `MF_SA_D3D_AWARE`, `MF_SA_MINIMUM_OUTPUT_SAMPLE_COUNT`; no Film Grain; DXVA only for Main-compatible streams.
- Direct3D 11 in Media Foundation https://learn.microsoft.com/en-us/windows/win32/medfound/supporting-direct3d-11-video-decoding-in-media-foundation : the application creates the D3D11 device, shares it via `IMFDXGIDeviceManager` (`MFCreateDXGIDeviceManager`) with `MFT_MESSAGE_SET_D3D_MANAGER`; a decoder that cannot use it returns `MF_E_UNSUPPORTED_D3D_TYPE` from `SetInputType/SetOutputType`, the client then sends the message with NULL and renegotiates, after which the MFT decodes in software; the decoder allocates output textures (texture arrays, `D3D11_BIND_DECODER`); `IMFDXGIDeviceManager::TestDevice` per frame, device change = rebuild.
- IMFDXGIBuffer https://learn.microsoft.com/en-us/windows/win32/api/mfobjects/nn-mfobjects-imfdxgibuffer (`GetResource`, `GetSubresourceIndex`), Uncompressed video buffers https://learn.microsoft.com/en-us/windows/win32/medfound/uncompressed-video-buffers (prefer the Direct3D surface, then `IMF2DBuffer::Lock2D` which returns stride that may exceed the row width and may be negative, then `IMFMediaBuffer::Lock`), Image stride https://learn.microsoft.com/en-us/windows/win32/medfound/image-stride (YUV must be top-down; Direct3D surfaces must be top-down).
- MFStartup https://learn.microsoft.com/en-us/windows/win32/api/mfapi/nf-mfapi-mfstartup : `MFStartup(MF_VERSION, flags)`; `E_NOTIMPL` when the media components are not present (KB2703761), `MF_E_DISABLED_IN_SAFEMODE`; call `MFShutdown` once per startup; not from work-queue threads or static constructors. This is the documented basis for a typed "unavailable" answer (the claim that Windows N editions lack Media Foundation comes from the Craft README only: UNVERIFIED against Microsoft).
- Field implementation (case study, MIT OR Apache-2.0, read-only; do not copy): FilmCraft `crates/platform` README at `gov_runtime/tools/studio-craft-source-cache/filmcraft-7b6c134472287db4367fca80bcb498240ba36567/crates/platform/README.md`. Observations used as hypotheses only: MFT driven per access unit with Annex B and SPS/PPS in front of the first sample; D3D-aware MFT returns NV12 textures read back through ONE staging-texture copy; "a picture that is not a Direct3D 11 texture fails the stream" and the MS software decode is deliberately not used by them; they load `mfplat.dll` at run time so N editions still start. This lane differs in one point: the software MFT IS the mandatory fallback because Studio has no software H.264 decoder yet (ledger scope).
- Master spec `14-studio-creative-suite.md`: STU-VID-051 (l.24559; adapter boundary; NOTE clause 1 "NOT reimplemented natively"), STU-VID-084 (l.25469, named operator decision), STU-ARC-002 (l.187), STU-ARC-012 (l.219), STU-ARC-015 (l.225), STU-ARC-021 (l.251), STU-PAR-005 / STU-QUIET-004 (owner-tagged hidden processes). Research `research/engine-video.md` topics `engine-anatomy` item 6, `improvements` items 1 and 5, `recommendations-for-reel-and-reel-native`. Reel-native prep MT-35561.

</topic>

<topic id="slices" status="active" summary="Minimal first slice and later slices">

## Slices
**Slice 0 (supervisor generalization, before any MF code).** Make the supervisor codec-neutral IN PLACE, never a second copy (V4-05, D18): introduce an internal `WorkerSpec { program_name, expected_decoder_id, codec_tag }`, keep the public AV1 surface (`WorkerAv1Provider`, `Av1WorkerOptions`, `Av1Error`, `Av1WorkerFault`, `Av1WorkerStats`) unchanged so validated tests stay green; add a `codec: u8` to OPEN (1 = av1, 2 = h264) and widen the `WHATS` table to per-codec ranges. Acceptance: every `av1_worker.rs` test passes unchanged. The lead's D18 decision (hub crate `hsk-studio-worker`) supersedes this if taken first.

**Slice 1 (minimal vertical slice): H.264 software decode through the Microsoft decoder MFT in a worker.**
1. Features: `mf = ["dep:hsk-studio-accord", "dep:hsk-studio-reel"]` (host, pure Rust, `forbid(unsafe_code)`; same two optional path dependencies the `av1` feature already uses), `mf-worker = ["mf", "dep:windows"]` (the worker binary `reel-native-mf-worker`, declared with `required-features = ["mf-worker"]` like the AV1 one; if the P1 lane puts H.264 behind a reel cargo feature, forward it here the way `av1-worker` forwards `hsk-studio-reel/av1`). Default build unchanged and empty. The crate's `lib.rs` uses reel's pattern (`cfg_attr(not(feature = "mf-worker"), forbid(unsafe_code))`, `cfg_attr(feature = "mf-worker", deny(unsafe_code))`, `reel/src/lib.rs:31-32`) with `#![allow(unsafe_code)]` ONLY in `src/mf/ffi.rs`; every unsafe block has a `// SAFETY:` line.
2. Worker: `CoInitializeEx` (multithreaded), `MFStartup(MF_VERSION, MFSTARTUP_LITE)` (`E_NOTIMPL`/safe-mode -> typed `CodecUnavailable`), enumerate with `MFTEnumEx(MFT_CATEGORY_VIDEO_DECODER, SYNCMFT | ASYNCMFT | HARDWARE | SORTANDFILTER, {MFMediaType_Video, MFVideoFormat_H264}, NULL)`, record per MFT name (`MFT_FRIENDLY_NAME_Attribute`), CLSID, hardware, asynchronous (`MF_TRANSFORM_ASYNC`), D3D awareness (`MF_SA_D3D11_AWARE`). Slice 1 uses the first SYNC, non-hardware decoder (expected `CLSID_CMSH264DecoderMFT`, UNVERIFIED on this host) WITHOUT a D3D manager: input type `MFMediaType_Video`/`MFVideoFormat_H264` with frame size and rate, output `MFVideoFormat_NV12`, `NOTIFY_BEGIN_STREAMING` + `NOTIFY_START_OF_STREAM`, `ProcessInput` per Annex B access unit (sample time = tag, see spike), drain on FINISH. Pull until `MF_E_TRANSFORM_NEED_MORE_INPUT`; handle `MF_E_TRANSFORM_STREAM_CHANGE` once (new output type), twice in one segment = `Unsupported "mf-format-change-mid-segment"`.
3. Output: read the sample through `IMF2DBuffer::Lock2D`, validate stride (> 0, >= width, plane arithmetic checked), crop to the frame size the output type states, deinterleave NV12 into tight planar Y/U/V (`ChromaFormat::Yuv420`, 8-bit) exactly as `VideoFormat` documents. Colour: do NOT read MF colour attributes (names and semantics not opened); `ColorInfo` from the container `colr` (`origin: Container`) else `Untagged`.
4. Host: `WorkerMfProvider` implements `ExternalVideoDecoder` (`supports(H264)`), returns `DecoderIdentity { id: "mf-h264", version }` where `version` is an interned short string `"<friendly-name>|<clsid>|dxva=0"` learned from HELLO (same interning bounds as `host.rs:311-330`); `probe_decoders()` returns the enumerated list via a new PROBE request.
5. Pre-spawn refusals in the host from the declared stream (`VideoStreamSpec`) and `avcC`: width/height outside 48..=4096 x 48..=2304, profile other than Baseline/Main/High (profile_idc 66/77/100), chroma other than 4:2:0, interlaced -> typed `Unsupported` BEFORE any worker starts (limits quoted from the Learn page).
6. Spike S1 (a test, not product behaviour): feed the B-frame fixture with sample time = decode index and then with the true PTS (100 ns units via accord `Timescale`, `Rounding::NearestHalfUp`; `hsk-studio-pulse` already proved `100 ns = 127008/5 ticks`, lane SMB-PULSE-ENG-1 PASS) and RECORD output order and output times. Product behaviour is chosen from the recorded result: (A) outputs carry the input time -> map by time; (B) outputs are re-timed/sorted -> needs P2 and a PTS table; until P2, a stream with `max_num_reorder_frames > 0` is `Unsupported "mf-reorder-needs-packet-timing"`. No silent guess.

**Slice 2: hardware path (D3D11/DXVA) with software fallback.** Worker creates a D3D11 device with `D3D11_CREATE_DEVICE_VIDEO_SUPPORT`, multithread protection, `MFCreateDXGIDeviceManager` + `ResetDevice`, `MFT_MESSAGE_SET_D3D_MANAGER`; output is a D3D11 texture (NV12), read back through ONE staging copy using `IMFDXGIBuffer::GetResource/GetSubresourceIndex`; `MF_E_UNSUPPORTED_D3D_TYPE` -> send NULL manager, renegotiate, `dxva=0`. `TestDevice` per frame; device lost -> software path for the rest of the segment, receipt says so. Parity gate: DXVA and software outputs must be byte-identical on the fixtures (H.264 decode is normative). A mismatch makes the hardware path preview-only: `Purpose::Export` (reel ENG-1) is routed to software unless a parity receipt for that device/profile exists.

**Slice 3: heartbeat and bounded replay.** Worker thread emits `HEARTBEAT { counter }` while a request is in flight; the counter is advanced by the decode loop at every `ProcessInput/ProcessOutput`, NOT by an independent timer thread (a live timer thread must not hide a hung decode, `repass-2026-10-09/engineering.json` l.1641). Host kills a worker whose counter did not advance for `heartbeat_timeout` (default 2 s) in addition to the batch/segment deadlines. Replay: after `Crashed`/`Deadline` the session re-opens a worker once per segment, re-sends retained packets from the last sync sample (needs P2; retained bytes capped, default 64 MiB, beyond which no replay and the typed error stands) and drops pictures whose tag was already delivered. Counters `replays`, `replay_failed` in stats.

**Slice 4 (measurement gated): shared-memory frame ring.** First measure stdio pipe throughput with an echo worker for 1080p and 2160p NV12 frames (3.1 MB / 12.4 MB). Only if pipes cannot sustain the target (decision threshold written down before measuring) design a ring with Win32 file mapping (`unsafe`, `ffi.rs` only), bounded slots, per-slot generation counters. Not designed here because no measurement exists.

**Later lanes:** MF H.264 hardware ENCODE (async MFT, `ICodecAPI`, Microsoft H.264 Encoder MFT as software fallback), HEVC/VP9/AV1 decode via Store extensions (UNVERIFIED), Job Object memory cap (motion-js uses `windows-sys` JobObjects; consolidation is D18), VideoToolbox/VA-API.

</topic>

<topic id="api" status="active" summary="Public API sketch and typed errors">

## API sketch
```rust
// feature "mf": host side, forbid(unsafe_code)
pub const MF_H264_PROVIDER_ID: &str = "mf-h264";
pub enum MfPath { Auto, SoftwareOnly, HardwareOnly }              // slice 2 for HardwareOnly
pub struct MfWorkerOptions { /* program, max_workers, deadlines, respawn bounds as Av1WorkerOptions via the shared WorkerSpec core */ pub path: MfPath }
pub struct MfDecoderInfo { pub name: String, pub clsid: [u8; 16], pub hardware: bool, pub asynchronous: bool, pub d3d11_aware: bool }
pub struct WorkerMfProvider { /* pool shared with the AV1 supervisor core */ }
impl WorkerMfProvider { pub fn new(options: MfWorkerOptions) -> Result<Self, MfConfigError>; pub fn probe_decoders(&self, cancel: &CancellationToken) -> Result<Vec<MfDecoderInfo>, MfError>; pub fn stats(&self) -> WorkerStats }
impl ExternalVideoDecoder for WorkerMfProvider { /* supports(VideoCodec::H264) */ }
// feature "mf-worker": src/mf/{mod.rs (worker_main_mf), ffi.rs (only unsafe), nv12.rs (pure, stride/crop/deinterleave, unit-testable on any OS)}
```
Faults reuse the existing stable codes (`decoder_crashed`, `decoder_deadline`, `worker_protocol`, `decoder_output_too_large`, `worker_spawn_failed`, `worker_respawn_limited`, `worker_pool_busy`, `worker_version_mismatch`) and add `mf_unavailable` (MFStartup `E_NOTIMPL`/safe mode -> `ReelError::CodecUnavailable { codec: "h264" }`), `mf_no_decoder`, `mf_unsupported_stream` (-> `Unsupported`), `mf_device_lost` (slice 2). `Av1Error`/`Av1WorkerFault` stay as they are; the MF types are siblings, mapped to the same `DecoderFault`.

</topic>

<topic id="hostile" status="active" summary="V4-01/02 bounds">

## Hostile-input bounds
- Trust boundary is the worker: the MS decoder is a native parser of untrusted H.264. Process isolation + deadlines + heartbeat are the controls; no in-process fallback.
- Host before spawn (checked arithmetic, `#![deny(clippy::arithmetic_side_effects)]` on every new host module): declared dims (u32 from the container) bounded to the MS decoder limits above AND `width*height*3/2` (NV12) <= `limits.max_chunk_bytes`; `dims_u32_max`-style test: `u32::MAX` declared dims -> `LimitExceeded` before any spawn (mirror of `tests/av1_worker.rs:582`); `avcC` hostile sizes are P1's tests.
- Aggregate budget (V4-02): in-flight bytes across the pool <= `max_workers * (reply_cap + MAX_PACKET_BYTES)`; test `aggregate_over_budget`: the (max_workers + 1)-th concurrent call is refused immediately (`worker_pool_busy`) without allocating a frame buffer.
- Worker output (hostile COM data): `cbSize`/sample length checked against the output type before any copy; stride must be > 0 and >= width; chroma plane offset arithmetic checked (`nv12.rs` is pure, fuzz-style tested on any OS); reply cap derived from declared dims as for AV1; more than `MAX_FRAMES_PER_BATCH` frames = protocol fault.
- COM/MF failure paths never `unwrap`; all `HRESULT` failures become typed errors; a worker that returns garbage is killed.
- Heartbeat is untrusted input: counter must be strictly monotonic and at most one frame per interval; violation = protocol fault + kill.
- Quiet operation (GLOBAL-BUILD-QUIET): no window, no swap chain, `CREATE_NO_WINDOW`, no foreground focus, no process this host did not spawn is ever stopped.

</topic>

<topic id="determinism" status="active" summary="Determinism contract and tolerance rule">

## Determinism contract
H.264 decoding is normative: for a conformant stream every conformant decoder produces identical pictures (research `engine-video.md` improvements item 1). Therefore: NO tolerance for H.264. Software MFT output must equal the external oracle (ffmpeg decode, per-frame SHA-256) byte for byte; hardware (DXVA) output must equal the software output byte for byte. A mismatch is a finding that is reported with diff statistics, never absorbed by loosening a test, and the offending path is demoted to preview-only. Reordering: output order must equal container presentation order. Determinism of the supervisor is not claimed (process timing). Receipts name the path: decoder id `mf-h264`, version string with MFT name, CLSID and `dxva=0|1`.

</topic>

<topic id="tests" status="active" summary="At most 6 focused tests, fixtures, exact commands">

## Tests (at most 6; `bounded_case`; Windows tests carry `#[cfg(windows)]`; anything that needs hardware or a Store decoder prints `NOT_PROVEN:` and is `#[ignore = "..."]`, V4-03; every child is ours and verified reaped)
1. `mf_probe_lists_decoders_or_reports_not_proven`: worker PROBE returns >= 1 H.264 decoder with name/CLSID/flags, or the test prints `NOT_PROVEN: no H.264 decoder MFT` and asserts the typed `CodecUnavailable`. Output of the run (names, CLSIDs, hardware flags) is printed so the lead can record the host's real inventory.
2. `h264_baseline_software_decode_exact_vs_oracle`: fixture `h264-baseline-128x72-24f.mp4` via `Registry::with_video_decoder` + `decode_segment_routed` (Untrusted): 24 `VideoFrame` chunks, 13,824 bytes each (128*72*3/2), per-frame SHA-256 equal to the pinned ffmpeg `framehash` list (frame 0 starts `38510c300e4699f7`, frame 23 `4ca53778ddaf6096`), PTS exactly `n * 8_475_667_200` ticks, receipt `decoder_id "mf-h264"`, `complete`, `video.chroma Yuv420`, `bit_depth 8`.
3. `h264_high_bf2_reorder_spike_and_policy`: fixture `h264-high-bf2-128x72-24f.mp4` (decode-order PTS in 1/30000: `0,3003,1001,2002,6006,4004,5005,9009,7007,8008,11011,10010,12012,15015,13013,14014,18018,16016,17017,21021,19019,20020,23023,22022`; SPS `max_num_reorder_frames = 2`, frame cropping bottom offset 4 (72 rows inside 80 coded), High profile; frame 0 hash starts `79c95135ab274e5c`, frame 23 `daf77e6cf97b4cb9`). Records the spike result and asserts either exact presentation-order equality with the oracle (strategy A/B) or the typed `Unsupported "mf-reorder-needs-packet-timing"` plus a printed `NOT_PROVEN:` line. Never silently green.
4. `mf_worker_fault_matrix`: crash on open/packet, hang mid-decode, stalled HELLO, oversize reply, garbage reply, cancel mid-segment (the `test-faults` mechanism of the AV1 worker generalized): typed `DecoderFault`, `Child::try_wait` shows no live child, permits released, pool stays usable; plus a mutation sweep over the fixture's access units (byte flips, truncation) that must end as `Ok` (concealed output) or typed error within the deadline with the host alive (analogue of `av1_worker.rs:261`). Also asserts the supervisor core exists once: `grep` fingerprint of the spawn/kill/deadline code finds it in one module (V4-05).
5. `mf_declared_limits_and_unsupported_streams`: `u32::MAX` dims, 47 x 47, 4097 x 2305, interlaced flag, and the 4:2:2 10-bit High 4:2:2 fixture (`h264-high422-10bit-128x72-12f.mp4`, profile_idc 122) -> typed before any spawn; `aggregate_over_budget`.
6. `mf_dxva_parity_with_software` (slice 2; `#[ignore = "needs D3D11 video device"]`, prints `NOT_PROVEN:` when absent): DXVA output equals software output on both fixtures; receipt `dxva=1`; forced `MF_E_UNSUPPORTED_D3D_TYPE`-style fallback test via an option that withholds the D3D manager; `Purpose::Export` picks software unless parity proven.

**Fixtures** (authored once under `D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/engine-video-oracle/`; the dispatch path without "Handshake Worktrees" does not exist; verified copies in `cmd-check/final/`; commit the three small MP4s plus `framehash` lists under `crates/hsk-studio-reel-native/tests/fixtures/h264/` with `provenance.json`; ffmpeg is never a test dependency). Height must be >= 48 because the MS decoder minimum is 48 x 48, which is why 64 x 36 is NOT used here. All commands were run; re-generation of the high fixture reproduced the same sha256.
- Common: `X="threads=1:open-gop=0:scenecut=0:no-info=1"`, `BX="-fflags +bitexact -flags:v +bitexact"`, source `testsrc2=size=128x72:rate=30000/1001`, 24 frames.
- Baseline, no B-frames: `ffmpeg -y -f lavfi -i "testsrc2=size=128x72:rate=30000/1001" -frames:v 24 -c:v libx264 -preset veryfast -profile:v baseline -pix_fmt yuv420p -bf 0 -g 12 -x264-params "$X" $BX h264-baseline-128x72-24f.mp4` (16,799 B, sha256 `f58e0962b4abe2022be5f8b92b5143ad1caf8ec486e2457663d5f0309fa1e63c`; Constrained Baseline, level 1.0 as written by x264, keyframes at 0 and 12)
- High with B-frames: `... -profile:v high -bf 2 ... -movflags +negative_cts_offsets h264-high-bf2-128x72-24f.mp4` (14,690 B, `6a3f74164c45bce441a764a95edf969b658422ba499e4480de1f3d126e7b5466`)
- Unsupported chroma/depth: `ffmpeg -y -f lavfi -i "testsrc2=size=128x72:rate=30000/1001" -frames:v 12 -c:v libx264 -preset veryfast -profile:v high422 -pix_fmt yuv422p10le -g 12 -x264-params "$X" $BX h264-high422-10bit-128x72-12f.mp4` (8,371 B, `fee69bc1a70cbae6dabdbc083d8d5b00b9df6c7b18911c2794dac733173aca4e`)
- Oracle (per-frame exactness, external tool, data only): `ffmpeg -y -i FILE.mp4 -f framehash -hash sha256 -` (verified equal to `sha256` of the raw I420 frame: `ffmpeg -i FILE -f rawvideo -pix_fmt yuv420p -`, first frame of the high fixture `79c95135...d23a`); timing oracle `ffprobe -v error -select_streams v:0 -show_entries packet=pts,flags -of csv=p=0 FILE`; SPS facts via `ffmpeg -v trace -i FILE -c:v copy -bsf:v trace_headers -frames:v 1 -f null -` (`pic_height_in_map_units_minus1 = 4`, `frame_crop_bottom_offset = 4`, `max_num_reorder_frames = 2`).

Cargo form: the studio form from `builders/WORKSPACE_RULES.md`; `-p hsk-studio-reel-native --features mf-worker,test-faults --tests` (the worker binary builds only with `mf-worker`); Windows only. Default features: `cargo check -p hsk-studio-reel-native --tests` must still pass and contain no MF code. The AV1 feature sets stay green (`--features av1`, `--features av1-worker,test-faults`).

</topic>

<topic id="deps" status="active" summary="Allowed dependencies with licences and the steward steps">

## Allowed dependencies
- `windows =0.62.2`, `default-features = false`, optional, enabled only by `mf-worker`. Licence `MIT OR Apache-2.0`, `rust-version 1.82`, read from the local registry copy `~/.cargo/registry/src/index.crates.io-*/windows-0.62.2/Cargo.toml` (read-only on C:). Version 0.62.2 (2025-10-06 per docs.rs) is ALREADY locked in the workspace through `cpal 0.18.2` (score-device); `Cargo.lock` also holds `windows 0.58.0` (gpu-allocator, wgpu-hal). Features, slice 1: `Win32_Foundation`, `Win32_Media_MediaFoundation`, `Win32_System_Com`; slice 2 adds `Win32_Graphics_Direct3D11`, `Win32_Graphics_Dxgi`, `Win32_Graphics_Dxgi_Common` (these names exist in 0.62.2's feature table; the full transitive feature closure, e.g. `Win32_System_Variant`, is found by `cargo check` and must be listed exactly, "features always present" per WORKSPACE_RULES). Symbols confirmed present in the local bindings: `MFTEnumEx`, `IMFTransform`, `MFStartup`, `MFCreateDXGIDeviceManager`, `IMFDXGIDeviceManager`, `MFT_MESSAGE_COMMAND_DRAIN`, `MFT_ENUM_FLAG_HARDWARE`, `MF_E_TRANSFORM_NEED_MORE_INPUT`, `MFCreateSample`, `MFCreateMemoryBuffer`, `MFVideoFormat_H264`, `MFVideoFormat_NV12`, `CODECAPI_AVLowLatencyMode`, `IMFMediaEventGenerator`, `METransformNeedInput`.
- Steward steps (not the builder): add `("hsk-studio-reel-native", ["windows"])` to the probe `APPROVED` table via the registry metadata and pin the transitive closure; the label (`pure-rust` vs `native-ffi`) for the `windows` family is a steward/lead call because the `-sys` name rule does not match it (how the probe treats it is UNVERIFIED). The `[target.'cfg(windows)'.dependencies]` form is avoided on purpose (the probe's TOML subset support for target tables is UNVERIFIED); the dependency is an ordinary optional one and the MF modules are `#[cfg(windows)]` with a `compile_error!` on other targets when `mf-worker` is on.
- Rejected: `windows-sys` (approved for motion-js only), `libloading` (run-time loading of `mfplat.dll` is not needed: a missing DLL makes the worker fail to start, which is a typed `worker_spawn_failed` -> `CodecUnavailable`), `ffmpeg-next`/`rsmpeg` (probe-banned/system prerequisite), `gpu-video`/Vulkan Video, `nvidia-video-codec-sdk` (stale), Craft `platform` code (case study only).
- Licence rule: ffmpeg/ffprobe (LGPL/GPL, https://ffmpeg.org/legal.html opened; the installed build is GPL-enabled) are external black-box fixture generators and oracles only; no ffmpeg source is read, copied or translated. The Media Foundation decoders are Microsoft OS components used through public COM interfaces; no Microsoft binary is bundled or downloaded.

</topic>

<topic id="redteam" status="active" summary="Risks, failure scenarios, minimum controls">

## Red team (controls are enforced by acceptance criteria)
| Risk / plausible failure | Minimum control |
|---|---|
| R1 a third supervisor copy appears (V4-05, D18) | slice 0 generalizes in place; test 4 asserts the supervisor core is defined once; AV1 tests unchanged |
| R2 MS decoder returns pictures with times that are re-sorted, tag mapping silently wrong, frames attached to the wrong packet | spike S1 recorded before behaviour is chosen; test 3 compares every frame hash in presentation order; `Unsupported` until P2 if unresolved |
| R3 hardware path differs from software (driver bug, colour conversion) | byte-exact parity gate (test 6); mismatch demotes hardware to preview-only; Export prefers software |
| R4 GPU driver hang/TDR or MFT deadlock | batch/segment deadlines, heartbeat tied to decode progress (slice 3), kill only own child, bounded respawn |
| R5 `unsafe` creep | `unsafe` only in `src/mf/ffi.rs`; `forbid(unsafe_code)` remains in the default build (a test compiles the default build and greps `unsafe` outside `ffi.rs`); each block has a SAFETY line |
| R6 hostile H.264 or hostile COM output (negative/huge stride, short sample, wrong format) | `nv12.rs` pure + checked; reply caps; refusal tests |
| R7 N edition / Media Foundation absent or Safe Mode | `MFStartup` failure -> typed `CodecUnavailable`; documented basis is the Learn page, N-edition claim stays UNVERIFIED |
| R8 licensing: shipping H.264 decode (OS component) | default-off feature, descriptor carries `operator_decision_ref`, O1 |
| R9 reorder/replay duplicates frames or loses frames after a crash | tag-based dedupe, receipt `actual.units` equals requested or typed error; replay only from a sync sample |
| R10 frame reply throughput through pipes limits 4K | slice 4 is measurement-gated; no ring built on speculation |
| R11 validated AV1 code regresses while generalizing | slice 0 first, AV1 suites must pass untouched; re-include the AV1 lanes in the next round (CX-VAL-009) |
| R12 privacy/quietness: device names or windows | names are device model strings only; no window creation; `CREATE_NO_WINDOW` |

</topic>

<topic id="acceptance" status="active" summary="Acceptance criteria">

## Acceptance
1. Slice 0: all existing `reel-native` tests pass unchanged; one supervisor core.
2. Slice 1: tests 1, 2, 3, 4 (software subset), 5 green on the Windows validation host; test 3 either exact or the documented typed `Unsupported` + `NOT_PROVEN:`; default build still empty; `windows` only behind `mf-worker`; `unsafe` only in `ffi.rs`.
3. Software output equals the external oracle per frame (no tolerance); decoder identity string names MFT, CLSID and `dxva`.
4. Hostile suite present: declared dims, aggregate budget, stride/plane checks, mutation sweep, fault matrix.
5. Later slices accepted separately; READY_FOR_VALIDATION only, the independent validator issues verdicts; any `NOT_PROVEN` is counted, never PASS.
6. No process stopped that this host did not spawn; no foreground window; all outputs under the artifact root.

</topic>

<topic id="unverified" status="active" summary="UNVERIFIED and Operator items">

## UNVERIFIED
- Which H.264 decoder MFTs exist on the validation host and whether `CLSID_CMSH264DecoderMFT` is first (no MF call was run). Whether any HARDWARE-flag decoder MFT exists (NVDEC is normally reached through DXVA inside the MS decoder, not as a separate hardware MFT; Learn says hardware MFTs are asynchronous).
- Output timestamp semantics of the MS H.264 decoder for reordered streams (Learn is silent); whether output equals ffmpeg byte for byte (expected, unmeasured).
- DXVA readback details: array-slice handling via `GetSubresourceIndex` (method listed, usage not read), `MF_SA_D3D11_BINDFLAGS`/staging requirements.
- Windows N edition behaviour (Craft README only), HEVC/VP9/AV1 decoders needing Store extensions (Craft README only), GPU TDR recovery behaviour.
- `avcC` layout (ISO/IEC 14496-15, not opened), ISO/IEC 14496-10 text (not needed for MF but needed to check SPS bounds in P1).
- MF colour attribute names and semantics (not opened; slice 1 avoids them).
- Pipe throughput vs 4K needs; that the probe accepts target-specific tables; how the probe labels the `windows` family.
- `MFTEnumEx` registry switches (OEM-only) can hide hardware codecs per Learn; effect on this host unknown.

## Operator items (not decided here)
- O1 H.264 licensing/patent posture for relying on OS decoders (STU-VID-084). FFmpeg states it has never read patents (https://ffmpeg.org/legal.html); the research note records pool uncertainty (`engine-video.md` rust-state-and-gaps). Decide before enabling `mf` by default anywhere.
- O2 D18 supervision consolidation (hub crate vs in-crate generalization) and D14 packaging of a second worker executable: this lane assumes in-crate generalization (slice 0) unless decided otherwise.
- O3 STU-VID-051 clause 1 ("not reimplemented natively") is respected here (OS decoders behind an adapter) but conflicts with the pure-Rust direction elsewhere; spec enrichment is Operator-only.
- O4 Validation-host GPU proof (RTX 3090 per START_LOG) is required for slice 2 acceptance; no GPU-less pass may be reported as parity.
- O5 Oracle tier: comparing Studio against what Premiere/Media Encoder decode for the same H.264 would need the D5 foreground session; not part of this lane.
- O6 ISO/IEC 14496-15 availability for prerequisite P1: if paywalled, the Operator decides on purchase/download (no download was made).

</topic>
