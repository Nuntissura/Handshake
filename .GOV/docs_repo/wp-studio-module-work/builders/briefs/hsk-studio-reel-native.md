---
file_id: BUILDER-BRIEF-HSK-STUDIO-REEL-NATIVE
file_kind: builder_brief
updated_at: 2026-10-11T05:00:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-reel-native
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-REEL_NATIVE` / key `reel_native` / crate `hsk-studio-reel-native`; foundation MT `MT-35561` (read `implementation_steps` 1-4; depends_on MT-35542 reel). Write scope `crates/hsk-studio-reel-native/**` only.
- Responsibility (verbatim): "Optional native media codec provider; isolate FFI/toolchain/codec license/version/process boundaries from pure media contracts."
- Foundation capability: "Actual selected codec decodes/encodes bounded segment with errors/cancellation/provenance; no full-media buffering or swallowed failures."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-reel`, `hsk-studio-observe`. Needs the `reel` crate (its `MediaProvider`/`Registry`/`SegmentReceipt`/`ReelError`, see `hsk-studio-reel.md`) => wave 2.
- Forbidden: handshake_core/native/SurrealDB; FFI handles in any public type (MT-35561 step 1: "typed ports contain no FFI handles"); codec code reachable from `reel`/raster/type default closure; `ffmpeg-sys-next` and `image` (probe-banned names); GPU deps; global codec factory registration; swallowing decoder errors into "empty/previous frame" (filmcraft `media_frame` anti-pattern, FILM-ROOT-06); requiring an installed external app at product runtime (CX-503S).
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[features] default = []`, `[lib] path="src/lib.rs"`, `[[example]]`s, `DESCRIPTOR`, `tests/foundation_contract.rs` + `bounded_case`.
- POLICY BLOCKER (lead/steward): any real native codec crate trips the probe policy (`pure_registry_name`: `*-sys` names, plus per-owner `approved` list). `openh264-sys2` ends in `-sys2` so the NAME check passes by accident - do not rely on that; request an explicit scoped exemption for this crate's optional feature.
- Prep refs: `media-team/preparation.json` -> `module_proposals.reel_native` (oracle: independent real container/codec decoder validates encoded saved bytes and segment metadata); `repass-2026-10-09/filmcraft-root-drill.json` FILM-ROOT-04 (codec factory registry, GOP cache, "first Some result returned even when it is an error"), FILM-ROOT-09 (hardware adapters: availability probes show callable support, NOT certified fidelity; FFI/OS/GPU must be selectable leaves behind shared media contracts), `licensed_and_unsafe_boundaries`, FILM-MOD-03 (platform uses OS/FFI deps); `backend-redteam/backpressure-architecture-review.json` (bytes AND slots; permits held until retirement). Reference source (MIT OR Apache-2.0, case study): `D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/filmcraft-7b6c134472287db4367fca80bcb498240ba36567/crates/{platform (14k LOC, Media Foundation/NVENC/VA via `windows`), codecs, h264, h264enc}`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-VID-051 (l.24559): codecs are an ADAPTER boundary with an enumerable typed capability descriptor (containers, codecs, bit depths, chroma, transfer); absent adapter => `MEDIA_CODEC_UNAVAILABLE` naming the codec; no adapter pulls GPU into handshake_core; WHICH codecs ship is a licensing/dependency decision needing a named operator decision (gap STU-VID-084). => this crate cannot ship a codec by default: it is opt-in per feature AND per operator decision (H.264 has patent-licensing implications; record the decision source in `DESCRIPTOR`).
- STU-ARC-002 (l.187): GPU/CPU render/sequence/audio providers are independently selectable crates behind typed `Send + Sync` ports. STU-ARC-012/008: bound producers/bytes/queue/process counts before allocation; every cancel/error/timeout frees reservations; permits until native quiescence. STU-ARC-015: unavailable provider = typed loss/failure; STU-ARC-021: exact requested vs actual counts; STU-ARC-013: no decode on audio callbacks. STU-PAR-005 / STU-QUIET-004: spawned processes owner-tagged, hidden, reclaimed.
- Later: STU-VID-060-064 export recipes (14.13 writers), hardware accel profiles, STU-VID-052 camera/raw settings. MT corpus: native codec feature MTs UNVERIFIED.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (default build pure Rust, `#![forbid(unsafe_code)]`; worker holds the native codec; ~800-1100 LOC)
Reuse (verified): `hsk_studio_accord::{Ticks, FrameRate, CancellationToken, ActorContext}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`; from `hsk-studio-reel` (read its lib once landed - use ITS names, do not redefine): `MediaProvider`, `CodecDescriptor`, `Registry`, `SegmentRequest`, `SegmentReceipt`, `ChunkSink`/`ChunkSource`, `ReelError`, `Limits`, and the Y4M provider (as the independent raw-frame source/sink for tests).
Architecture (same isolation pattern as score-plugin; design carried by `score-plugin` brief: framed stdio protocol, host spawns and owns the worker, kill+reap on timeout/crash, no in-process fallback): the host crate NEVER links the codec; a short-lived WORKER process does.
1. Capability negotiation + provenance: `pub struct NativeProviderIdentity{name, version, license_spdx, abi, toolchain, binary_sha256:Option<[u8;32]>}`; `pub fn negotiate(&self, want:&CodecWant{codec, container, bit_depth, chroma, transfer}) -> Result<NativeCaps, ReelError>` BEFORE any invocation; missing worker/feature/isolation => `ReelError::CodecUnavailable{codec}` (code `"MEDIA_CODEC_UNAVAILABLE"`); identity is copied into every `SegmentReceipt` (provenance). `ReelNative: reel::MediaProvider` so it registers into the pure `Registry` exactly like a Rust provider; its `descriptor()` states `native:true`, license and operator-decision ref.
2. Framed worker protocol + process host (pure Rust, host side): `{magic,version,kind,len}` frames with limits checked BEFORE allocation; kinds Hello/Negotiate/DecodeSegment/EncodeSegment/Chunk/Done/Error/Cancel/Shutdown; `WorkerHost::spawn` with piped stdio (`CREATE_NO_WINDOW` on Windows, null stdin), per-call `Deadline`, `max_workers` + `max_in_flight_bytes` RAII permits released only after the child is reaped; timeout/crash/cancel => kill+reap what we spawned and return `Timeout/Crashed/Canceled` + `QuiescenceReport{killed, reaped, permits_released}`; segments stream chunk-by-chunk to the caller's `ChunkSink` (never buffer the whole media, never hold more than `max_chunk_bytes * queue_depth`); partial output is marked incomplete, never `Ok`.
3. Reference worker `examples/reel-native-worker.rs` with a pure-Rust passthrough "codec" (Y4M-in/Y4M-out) used ONLY to prove protocol, cancellation, crash/hang containment and quiescence without claiming a native codec (labelled in `DESCRIPTOR`).
4. Real native backend candidate in the worker, feature `openh264` (default OFF): `openh264 0.9.8` (BSD-2-Clause wrapper over Cisco OpenH264; updated 2026-08-08; deps `openh264-sys2`, `wide`) - encode a bounded Y4M 4:2:0 segment (e.g. 64x36 x 10 frames) to H.264 Annex-B, then decode it back; failures surface as typed `Corrupt/Truncated/Unsupported`, never previous-frame substitution. Prefer the crate's `libloading` feature (loads the Cisco prebuilt DLL from an operator-APPROVED, SHA-256-pinned path; avoids a C++ toolchain) over the default `source` feature (compiles OpenH264 C++ via `cc`; NASM/MSVC requirements UNVERIFIED). Downloading the Cisco binary needs explicit Operator approval (file, source, size must be stated first); H.264 shipping = STU-VID-084 licensing decision. Alternative single candidate on this Windows host: OS Media Foundation H.264 via the `windows` crate (MIT OR Apache-2.0, 0.62.2) as FilmCraft `platform` does - larger FFI surface, OS-version dependent; not chosen for slice 1.
5. `DESCRIPTOR` JSON (provider identity, isolation model, license, `operator_decision_ref`, `NOT_PROVEN` markers) + `report(...)` into `Observe` (Success/Canceled/Failure(Unavailable|Loss|Unsupported|Validation)) + example `reel-native-consumer` (`--descriptor`, `--negotiate h264`, `--worker PATH --roundtrip FRAMES [--corrupt] [--cancel-after N] [--hang]`).
Oracle (independent, validator-time): `ffprobe`/`ffmpeg` 8.1.1 is installed on this machine (WinGet Gyan.FFmpeg full build; found on PATH) and may serve as the INDEPENDENT decoder/metadata oracle for the encoded bytes (codec name, frame count, dimensions, PTS) and a reference decode compared to the provider's decode within lossy tolerance (PSNR threshold stated up front); it must never be a product prerequisite (CX-503S) and its absence makes the oracle `NOT_PROVEN`, not PASS.

</topic>

<topic id="deps" status="active" summary="third-party deps (crates.io API 2026-10-11)">

## Deps
- Slice 1 (cap. 1-3): none (std only).
- Native candidate (cap. 4, worker-only, feature `openh264`, needs dep-request + scoped probe exemption): `openh264 0.9.8` BSD-2-Clause (updated 2026-08-08, ~0.9M recent dl, MSRV 1.85; features `source` [default], `libloading`) + `openh264-sys2 0.9.8` BSD-2-Clause (deps none; optional `libloading`, `sha2` for DLL hash pinning); **pure-rust: NO** (C++ via source or runtime DLL). Cisco's license grants H.264 patent coverage only for Cisco-distributed binaries - record in `DESCRIPTOR`. `sha2 0.10.9` (`force-soft`, pure, workspace-pinned) for the host-side hash gate.
- Pure-Rust alternatives are NOT this crate: AV1 via `rav1d 1.1.0` (BSD-2; use `default-features=false` to drop hand asm) and `rav1e 0.8.1` (BSD-2; `default-features=false`) belong to the pure `hsk-studio-reel` wave-2 providers (see reel brief); FilmCraft `h264`/`h264enc`/`hevc`/`vp9`/`av1` crates (MIT OR Apache-2.0; deps `bitstream`, `thiserror`, `rayon`) are pure-Rust adaptation candidates for `reel` after the STU-VID-084 decision.
- Rejected: `ffmpeg-next`/`ffmpeg-sys-next` (probe-banned, system FFmpeg prerequisite), `gstreamer*`, `x264`/`x265` bindings (GPL), `libaom`/`libvpx` sys crates.
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-reel-native --tests` (add `--features openh264` only after the steward approves it).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`; fixtures generated in-test via reel's Y4M provider into the studio TEMP; every child is ours and verified reaped)
1. `negotiation_before_invocation`: unsupported codec/depth/chroma or missing worker => `MEDIA_CODEC_UNAVAILABLE` naming the codec, no process spawned; supported => capability set + identity (name/version/license/abi) present in the later receipt.
2. `reference_worker_roundtrip_streams_in_chunks`: passthrough worker returns the segment chunk-by-chunk to a `ChunkSink`; peak buffered bytes `<= max_chunk_bytes * queue_depth`; counts/PTS exact; receipt `complete == true`.
3. `crash_hang_cancel_quiescence`: worker `--crash`, `--hang`, and cancel mid-segment => `Crashed/Timeout/Canceled`, partial output flagged incomplete (never `Ok`), `killed/reaped/permits_released` all true, `Child::try_wait` shows no live child.
4. `protocol_and_limit_rejections`: oversize/short/garbage frames, wrong version, queue/byte budget exhaustion before allocation (`LimitExceeded`), `max_workers=1` second spawn => immediate `Busy/BudgetExceeded`.
5. `openh264_roundtrip_if_available` (feature `openh264`, `#[ignore]` unless approved + DLL present): encode 10 frames, independent `ffprobe` reports codec h264 and 10 frames, decode back within PSNR threshold, corrupted stream => typed `Corrupt`; when the DLL/feature/ffprobe is absent the test reports `NOT_PROVEN` (not pass).

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No claim of native-codec proof from the reference worker; no HEVC/ProRes/AAC/hardware (NVENC/MF/VA) providers; no GOP cache/seek index; no audio codecs; no GPU; no export recipes/queue; no shipping decision (STU-VID-084 operator decision); no in-process fallback; no pixel-format conversion beyond the 4:2:0 test path.
- Wave: 2 (needs `reel`); native feature additionally blocked on probe-policy exemption + Operator approval for any binary download. Report READY_FOR_VALIDATION only for what is proven.

</topic>
