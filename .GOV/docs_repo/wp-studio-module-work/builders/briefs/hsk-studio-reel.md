---
file_id: BUILDER-BRIEF-HSK-STUDIO-REEL
file_kind: builder_brief
updated_at: 2026-10-11T02:30:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-reel
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-REEL` / key `reel` / crate `hsk-studio-reel`; foundation MT `MT-35542` (read `execution.implementation_steps` 1-4 and `proof.checks`). Write scope `crates/hsk-studio-reel/**` only.
- Responsibility (verbatim): "Decode/encode media/proxy-provider adapters; native codec bindings opt-in, never reachable from raster/type default closure."
- Foundation capability: "Decode/encode one real bounded media segment through selected provider with proxy/original receipt; no full-buffer video load."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-observe` ONLY (no folio, no pulse; timestamps are `accord::Ticks`).
- Forbidden: handshake_core/native/SurrealDB; FFI/`-sys`/ffmpeg*/gstreamer/GPU/`image` crate (probe policy `pure_registry_name`: `image`, `wgpu`, `ffmpeg-sys-next`, any `*-sys` rejected); native codec bindings (go in `hsk-studio-reel-native`); process-global mutable codec factory registry (filmcraft anti-pattern, see prep).
- Conventions (verified in existing crates): edition 2024, rust-version 1.97.1, single-line inline-table `Cargo.toml` (see `builders/WORKSPACE_RULES.md`), `[lib] path="src/lib.rs"`, `[[example]] name="reel-consumer"`, `pub const DESCRIPTOR`, tests in `tests/foundation_contract.rs` with `bounded_case` wrapper. Crates.io deps only via `builders/dep-requests.jsonl`.
- Prep refs (reuse, do not re-derive): `docs_repo/studio-collaboration-2026-10-09/modular-authoring/media-team/preparation.json` -> `module_proposals.reel` (steps/oracle/negatives; open question "selected pure provider and streaming sink signatures unresolved"); `.GOV/reference/studio_app_feature_research/repass-2026-10-09/filmcraft-root-drill.json` roots FILM-ROOT-04 (readers/codec factories/GOP cache), 05 (offline media/proxy/relink), 08 (export staging), 09 (hardware fallback) and `modularity_findings` FILM-MOD-01..03. FilmCraft reference source (MIT OR Apache-2.0, read-only case study): `D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/filmcraft-7b6c134472287db4367fca80bcb498240ba36567/crates/{media,codecs,matroska,ogg,bitstream}`; if code is adapted, add `SOURCE_PROVENANCE.json` like nib/pigment.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-VID-051 (l.24559): codec/container support is a declared, ENUMERABLE adapter inventory with a typed capability descriptor (containers, codecs, bit depths, chroma subsamplings, colour transfer functions); absent adapter => determinate `MEDIA_CODEC_UNAVAILABLE` naming the codec, never a silent empty render; no adapter pulls GPU into handshake_core. Which codecs ship = licensing decision (gap STU-VID-084, needs named operator decision) - therefore slice 1 ships only license-free raw providers.
- STU-VID-050 (l.24505): unsupported item => determinate `MEDIA_IMPORT_UNSUPPORTED` naming the detected type. STU-VID-038 (l.23653): proxies and offline media are first-class states; offline relinks by content hash; proxy creation/reconnect are queued background ops. STU-VID-052/053: per-media-item camera settings; conform/peak/media cache are explicit background ops.
- STU-ARC-012 (l.219): bound pending callers BEFORE large allocation/decode; byte leases span worker results; every cancel/error/timeout frees or transfers reservations. STU-ARC-015 (l.225): receipts bind original/proxy resolution and renderer; unsupported fallback or earlier-revision screenshot proves nothing; typed loss/failure for every error (optional early-return test paths are NOT_PROVEN). STU-ARC-021 (l.251): receipts report exact requested vs actual sample/frame counts - no silent clamp/truncation. STU-ARC-013: no decode in an audio callback.
- Later ranges: STU-VID-060-064 export recipes/queue (14.13 owns format writers), STU-IO-* import/export (see interop briefs), STU-VID-080..086 gaps. MT corpus: media import/export feature MTs in the STU-VID block (UNVERIFIED range ~MT-33685..33847).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust; real bytes, bounded streaming; ~900-1200 LOC)
Reuse (verified): `hsk_studio_accord::{Ticks (new/value/checked_frames/from_seconds), TICKS_PER_SECOND, FrameRate (from_import/ticks_per_frame), FrameRateNormalization, BROADCAST_TICKS_PER_FRAME, CancellationToken, DomainId, ActorContext}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. Decision: first providers are RAW and license-free so no codec-licensing decision (VID-084) is needed: **Y4M** (YUV4MPEG2, 8-bit `C420*`/`C444`/`Cmono`, plain `FRAME\n`) video and **WAV PCM** (s16le/f32le) audio. Compressed codecs are wave 2 (see deps).
1. Ports + enumerable registry (instance-owned, NO global): `pub struct CodecDescriptor{id:&'static str, kind:MediaKind, container:&'static str, bit_depth:u8, chroma:Option<&'static str>, transfer:Option<&'static str>, can_decode:bool, can_encode:bool}`; `pub trait MediaProvider: Send + Sync { fn descriptor(&self)->CodecDescriptor; fn decode_segment(&self, src:&mut dyn ReadSeek, req:&SegmentRequest, limits:&Limits, cancel:&CancellationToken, sink:&mut dyn ChunkSink)->Result<SegmentReceipt, ReelError>; fn encode_segment(&self, spec:&EncodeSpec, source:&mut dyn ChunkSource, out:&mut dyn std::io::Write, limits:&Limits, cancel:&CancellationToken)->Result<SegmentReceipt, ReelError>; }`; `pub trait ChunkSink{ fn accept(&mut self, chunk:Chunk<'_>)->Result<(), ReelError>; }`; `Registry::{new, register(Box<dyn MediaProvider>), descriptors()->Vec<CodecDescriptor>, resolve(&SegmentRequest)->Result<&dyn MediaProvider, ReelError>}` (resolve miss => `ReelError::CodecUnavailable{codec}` whose `code()` == `"MEDIA_CODEC_UNAVAILABLE"`; unknown container => `"MEDIA_IMPORT_UNSUPPORTED"`). `ReelError::{CodecUnavailable, ImportUnsupported, Corrupt, Truncated{delivered}, LimitExceeded, Canceled, StaleGrant, Unsupported{what}}`.
2. Y4M decode of a bounded segment: `Y4mProvider`; `SegmentRequest{codec:&str, start_pts:Ticks, frame_count:u64, grant:Grant, prefer:Original|Proxy}`; parse header (<= `limits.max_probe_bytes`, checked W/H/F/A/C tags; rate mapped through `FrameRate::from_import(ticks_per_frame)`, non-broadcast rate => `Unsupported`), compute fixed frame stride with checked arithmetic, `Seek` to frame `start_pts / ticks_per_frame` (must be frame-aligned else `Unsupported`), then read ONE frame at a time into a reused buffer (<= `max_chunk_bytes`), call `sink.accept(Chunk{pts, bytes})`, poll `cancel` between frames, never hold the whole file/segment (peak memory = one frame + header). Truncated tail => deliver complete frames then `Err(Truncated{delivered})` (receipt carried in error) - never success with fewer frames.
3. Y4M encode of a segment (staged, not published on failure): `encode_segment` pulls frames from `ChunkSource`, writes header + `FRAME\n` + bytes to `out`, returns `SegmentReceipt{complete:bool, frames_written, bytes_written}`; cancel/limit mid-segment => `complete:false` and the caller must treat the output as a non-published partial (receipt says so). WAV: `WavProvider` decodes a sample range `[start, start+count)` of PCM in <= `max_chunk_bytes` slices (reuse exact divisor conversion `TICKS_PER_SECOND / rate` -> ticks per sample: 48000 Hz = 5_292_000), `Unsupported` for non-PCM/extensible-unknown, truncated `data` chunk => `Truncated`.
4. Receipts + proxy/original: `SegmentReceipt{requested:Span, actual:Span, disposition:Exact|Approximate|Partial, source:Original|Proxy, original_id:ContentId, proxy_id:Option<ContentId>, codec:&'static str, chunks:u32, bytes_read:u64, complete:bool}` with `Span{start_pts:Ticks, end_pts:Ticks, frames_or_samples:u64}`; `Grant{content_id:ContentId, revision:u64, expected_len:u64}` verified before the first read (`StaleGrant` on length/revision mismatch; optional sha256 check is deferred, see deps). Proxy policy: `prefer=Proxy` with a proxy source whose stream layout differs (e.g. fewer audio tracks / different dims) => original is used and disposition says so; full-resolution export requests refuse proxy fallback (`StrictOriginalRequired`).
5. Observability: `DESCRIPTOR` JSON (`registry` enumeration included) + `report(result, cancel, &mut Observe, &mut impl SinkPort)` (Ok complete -> Success; Truncated -> Failure(Loss); CodecUnavailable/ImportUnsupported -> Failure(Unsupported); Canceled -> Canceled); example `reel-consumer` (`--descriptor`, `--list-codecs`, `--decode FILE.y4m --start-frame N --frames N`, `--encode-roundtrip`) generating its own fixture; no windows, no files outside TEMP.
Oracle (black-box vs Adobe, later; public specs + observation only, no decompilation): for compressed wave-2 providers compare Studio decoded frame count, PTS list (ticks), pixel values (within codec tolerance), range (full/limited), pixel-aspect ratio, start timecode/edit-list offset and audio sample counts against frames/audio exported by Adobe Media Encoder/Premiere (uncompressed or PNG sequence) from the same clip (H.264/HEVC/ProRes/DNxHR fixtures); proxy-vs-original disposition and offline-relink behaviour against Premiere's proxy attach on the same media. Slice-1 Y4M/WAV oracle is pure integer: per-frame SHA of a generated test pattern and exact byte/sample counts.

</topic>

<topic id="deps" status="active" summary="third-party deps (crates.io API checked 2026-10-11 unless cited)">

## Deps
- Slice 1: none required (std `Read/Seek/Write`). Optional dep-request: `sha2 0.10.9` `default-features=false, features=["force-soft"]` (already locked in the workspace for nib/type; pure-rust; steward must add `hsk-studio-reel` to the probe `approved` list) for `Grant` content-hash verification.
- Wave-2 compressed providers (each its own feature, default OFF; pure-Rust first): demux `matroska-demuxer 0.8.1` (Zlib OR MIT OR Apache-2.0, zero deps, updated 2026-08, pure-rust yes); AV1 decode `rav1d 1.1.0` (BSD-2-Clause, Rust port of dav1d; deps libc/raw-cpuid/zerocopy...; **default features enable hand-written `asm` -> use `default-features=false` + `bitdepth_8`**; pure-rust only with asm off; slower), AV1 encode `rav1e 0.8.1` (BSD-2-Clause; default features `asm`(nasm/cc), `binaries` -> `default-features=false`; large dep set); `mp4 0.14.0` (MIT, pure demux/mux, last release 2023-08 - stale); audio via `symphonia 0.6.1` (**MPL-2.0**, pure Rust, updated 2026-08, per-codec features; filmcraft and soundcraft both use it) - needs license sign-off; `ogg 0.9.2`/`lewton 0.10.2` (stale 2021). Alternative per prep (FILM-MOD-02): FilmCraft leaf crates with ZERO normal deps - `matroska` (4.5k LOC), `ogg`, `opus`, `bitstream` (0.4k), `mxf`, `cfb` - adaptable with `SOURCE_PROVENANCE.json` (MIT OR Apache-2.0, keep NOTICE); `h264`/`hevc`/`av1`/`vp9` there depend on `bitstream`, `thiserror`, `rayon`. Which codecs ship = operator decision (STU-VID-084) -> wave 2 only after that decision.
- Rejected here: ffmpeg*/gstreamer/openh264 (C/FFI -> reel-native), `image` crate (probe-banned).
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-reel --tests` (then clippy, focused test).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`; fixtures generated in-test into the OS TEMP set by studio-env.sh and removed after, CX-GIT-003)
1. `y4m_segment_decode_exact`: generate 10-frame 64x36 `C420` pattern at 25 fps (10_160_640_000 ticks/frame); decode frames 3..6 -> 3 chunks, each `pts == n*10_160_640_000`, bytes equal the generator's independent per-frame hash, `peak_alloc <= one frame`, receipt `Exact`, `bytes_read` far below file size.
2. `truncated_and_corrupt`: cut the file one frame early -> `Truncated{delivered}` with the delivered chunks intact (never `Ok`); bad `FRAME` marker / zero dims / oversize header -> `Corrupt`/`LimitExceeded` before allocating the frame buffer.
3. `codec_registry_enumerates_and_fails_loudly`: `descriptors()` lists exactly the registered providers; unknown codec id -> code `"MEDIA_CODEC_UNAVAILABLE"` naming it; unsupported chroma/depth (e.g. `C420p10`) -> `Unsupported`; two registries are independent (no global state).
4. `encode_roundtrip_and_cancel`: decode->encode->decode equals original bytes; cancel after frame 2 mid-encode -> `complete == false`, `frames_written == 2`; WAV 48 kHz s16 range decode returns exact sample count and `5_292_000 ticks/sample` timestamps.
5. `proxy_original_receipt`: `prefer=Proxy` with matching layout -> `source=Proxy`; mismatching layout -> `Original` + disposition note; strict-original request refuses proxy; stale grant (length/revision mismatch) -> `StaleGrant` before any read; `report()` emits one terminal frame per outcome.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No H.264/HEVC/AV1/ProRes/AAC/MP3 etc. (wave 2 after VID-084 decision), no container muxing beyond Y4M/WAV, no GOP cache/seek index, no hardware accel/FFI/OS codecs (reel-native MT-35561), no export queue/recipes (VID-060-064), no colour management/tone-mapping (prism/pigment), no relink/fingerprint search (later; FILM-ROOT-05 documents sampled-identity pitfalls), no GUI, no Pulse/Folio coupling, no global factory registration.
- Wave: 1 (accord + observe only; sha2 optional). Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
