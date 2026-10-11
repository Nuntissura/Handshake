---
file_id: WP-STUDIO-MODULE-WORK-RESEARCH-ENGINE-VIDEO
file_kind: research_note
updated_at: 2026-10-11
wp: WP-KERNEL-STUDIO
author_role: ENGINE_RESEARCHER_VIDEO (Sonnet)
---

# Video codec / media engine teardown (Premiere / Media Encoder / After Effects class)

Prose surface (context, not authority). Legend: [L] local file evidence (path given), [V] fetched/searched URL, [U] UNVERIFIED/inferred, [S] static read of Craft source cache (not built, not run). Method: file NAMES, PE version-info strings (Explorer-level metadata), preset XML, public docs. No decompilation, disassembly or strings-dumping of any binary. No Adobe app launched.

<topic id="prep-already-covers" status="active" version="1" summary="What prep corpus already has; this note adds only the delta" updated_at="2026-10-11">

Already in prep (do not redo; paths under kernel root):
- `.GOV/reference/studio_app_feature_research/repass-2026-10-09/filmcraft-root-drill.json`: FILM-ROOT-04 reader/codec factory/GOP cache (process-global registry, first-Some-wins, approximate previous-frame fallback, decoder-pool count thresholds not byte budgets), -05 offline/proxy/relink, -06 render loses errors to `None`, -08 stepped export (512 MiB in-flight, audio cursor lost on Pending retry, non-atomic output), -09 hardware adapters + hybrid software replay, FILM-MOD-02/03 (zero-dep leaves: bitstream, matroska, mxf, ogg, opus; codecs facade has no per-format features), licence line (MIT OR Apache-2.0; Symphonia MPL-2.0), adoption controls FILM-ADOPT-04 (typed codec/render loss). Its own proof_state lists "all individual codec reconstruction/entropy kernels" as NOT covered -> delta below (section rust-state-and-gaps).
- `.GOV/reference/studio_app_feature_research/repass-2026-10-09/engineering.json` (l.692 reel = decode/encode/proxy adapters, native bindings opt-in never in raster/type closure; l.1535 SIMD/codec/GPU need cfg + runtime dispatch + scalar reference; l.1641 watchdog must not let UI heartbeat hide a dead GPU/codec worker).
- `.GOV/docs_repo/studio-collaboration-2026-10-09/modular-authoring/media-team/preparation.json` module_proposals: pulse MT-35540 (l.12498), reel MT-35542 (l.12617: no global factory, requested vs actual PTS, proxy/original/draft disposition explicit, oracle = independently demuxed segment), reel_native MT-35561 (l.12856: opt-in pinned native, cancel/isolation, independent decoder validates output).
- `.../backend-redteam/pdf-audio-root-drill.json` (Symphonia in audio chain), `.../adobe-original-team/media-original.json:611` (verify final codec/metadata/color/audio/duration independently).
- Sibling notes: `research/adobe-behavior.md` (drop-frame label algorithm, test vector, l.52/67), `research/craft-case-study.md`, `research/existing-api-digest.md`. `research/color-codecs-raw.md` was NOT present when this was written (color science / RAW crate picks deliberately not duplicated here).
- Crate state [L]: `wt-studio-assets/handshake-studio-assets/crates/hsk-studio-{pulse,reel,reel-native,interop-media}/src/lib.rs` are 2-5 line scaffolds.
- Not in prep, added here: Adobe vendor stack evidence, per-codec Rust maturity, public spec list, hardware-API Rust state, patent posture, improvements, black-box plan, concrete Reel/ReelNative recommendations.

</topic>

<topic id="adobe-installed-codec-vendors" status="active" version="1" summary="Licensed vendors visible from file names and PE version-info" updated_at="2026-10-11">

Roots [L]: `C:/Program Files/Adobe/Adobe Premiere Pro 2026` (PPro), `.../Adobe Media Encoder 2026` (AME), `.../Adobe After Effects 2026/Support Files` (AE). PPro and AE share one DLL set in the app dir; AME shares plug-ins (`PlugIns/Common/ImporterFFMPEG.prm, ImporterMPEG.prm, ImporterFastMPEG.prm, ImporterMXF.prm, ExporterMPEG2/MPEG4/MXF.prm`).

| Vendor (evidence) | Files | Reading |
|---|---|---|
| MainConcept GmbH (VersionInfo "MainConcept(r) AVC/H.264 Encoder, (c) 2026") | `mc_dec_avc/aac/dv100/mp2v/mp4v/mpa`, `mc_enc_avc/avcsr/aac/dv100/mp2sr/mp2v/mp4v/mpa`, `mc_demux_mp2/mp4/mxf`, `mc_mux_mp2/mp4`, `mc_mfimport`, `mc_trans_video_colorspace` ("Universal Color Converter") | Licensed AVC/AAC/MPEG-1/2/4/DV100 codecs + MP4/MXF/PS demux-mux. No `mc_*hevc*` file name: HEVC path vendor UNVERIFIED [U] |
| MOG Technologies (VersionInfo "mSDK") | `MOG_Framework_1.1.12.dll`, `MSDK_Pro_1.1.12.dll` | Purpose UNVERIFIED (likely MXF/pro-format SDK) [U] |
| FFmpeg (no VersionInfo) | `libavcodec.dll`, `libavformat.dll`, `libavutil.dll`, AME `ImporterFFMPEG.prm` | Long-tail importer. Version/licence build UNVERIFIED [U] (LGPL unless `--enable-gpl`/`nonfree`, [V] ffmpeg.org/legal.html) |
| GoPro/CineForm Inc. | `CFHDDecoder64.dll`, `CFHDEncoder64.dll` ("(c) 2001-2014 GoPro") | CineForm codec; SDK later open-sourced ([V] gopro/cineform-sdk; MIT/Apache per Wikipedia/Fstoppers, LICENSE file not read) |
| Avid Technology | `DNxSDK-vs2019.dll` ("DNx Codec", 2025) | DNxHD/HR via vendor SDK |
| Apple ProRes | `ProResOpt.dll` (VersionInfo "Adobe DVA"), `ProResRAW.dll`, `ProResRawSourceSettings.aex`; presets "Apple ProRes 422 Proxy..4444 XQ with alpha, HLG/PQ" | Provenance of ProRes enc/dec (Apple library vs Adobe RDD 36 implementation) UNVERIFIED [U] |
| RED | `REDDecoder-x64/REDR3D-x64/REDCuda-x64/REDOpenCL-x64.dll`, `REDSourceSettings.aex` | R3D SDK (CPU/CUDA/OpenCL) |
| ARRI / Codex | `ArriImageSdk.9.dll`, `arriimagesdk_plugins/ArriImageSdkTransforms{Cpu,CpuFma,Cuda,OpenCl}_module.9.0.1.dll`, `jpegxs-dll.dll`, `codexhdedecoder.dll` (X2X Limited "Codex HDE SDK v5.2.0"), `ArriRaw*SourceSettings.aex` | ARRIRAW + ARRI Image SDK, JPEG XS, Codex HDE |
| Sony | `SonyRawDev.dll` (Raw Development Kit), `SMDK-VC140-x64-4_26_0.dll` (MXF Development Kit), `SonyRawMXFSourceSettings.aex` | X-OCN/RAW + XAVC/MXF |
| Canon | `crxdec.dll` ("CANON INC. 2018"), `CanonRawSourceSettings.aex`, `CanonLUTs/` | Cinema RAW Light (CRX) |
| Panasonic | `AVCIntraEncoder.dll` | AVC-Intra encode |
| Kakadu | `kdu_vs85R.dll`, `kdu_as85R.dll`, + Adobe `JP2KLib.dll` | JPEG 2000 (commercial) |
| Other | `libmp3lame.dll`, `libmpg123.dll`, `LibLTCWrapper.dll` (Adobe wrapper, LTC timecode), Intel IPP/MKL/OpenVINO, `onnxruntime.dll`, `DirectML.dll`, `opencv_world4110.dll`, `halide_default_windows_x64.dll`, CUDA `cudart64_*` | DSP/ML/GPU runtime |

- Not seen by name: x264/x265, dav1d/libaom/SVT-AV1, Dolby, BRAW. Dolby Vision exists as export UI: HEVC `.epr` has `ADBEDolby_1..23` params [L] (implementation vendor UNVERIFIED [U]).
- No third-party-notice text found under the three install dirs (only ACES/OCIO `LICENSE.md` and CEP `node_modules` licences). Adobe's online notices NOT_INSPECTED.
- Host architecture [L]: importer/exporter plug-ins (`.prm`, `.aex`), Adobe "DVA" framework; export presets are XML `.epr` under `Premiere Pro 2026/MediaIO/systempresets/<ExporterClassID-hex>_<FileType-hex>/` named by FourCC pair, e.g. `4D584620_504D5846`='MXF '+'PMXF' (ProRes MXF, 8 presets), `3F3F3F3F_4D6F6F56`='????'+'MooV' (MOV, 35), `4E49434B_48323634`='NICK'+'H264' (43), `4A454646_48455643`='JEFF'+'HEVC' (10 incl. HLG/PQ/2020), `4d584620_444d5846`='MXF '+'DMXF' (152), `58444341_4d584658`='XDCA'+'MXFX' (117), `50324D58_4D584620`='P2MX'+'MXF ' (23), AS-10/AS-11, DPX, EXR, GIF, DCP.
- Export knob inventory (ParamIdentifier, from `.epr`) [L]: `ADBEVideoCodec`, `ADBEVideoBitDepth`, `ADBEExportColorSpace`, `ADBE_H264_ColorPrimaries`, `ADBE_HEVC_ColorPrimaries`, `ADBE_VIDEO_HDR10`, `ADBEMPEGVideoHDR10MasterDisplayPrimaries/LuminanceMax/Min/MaxContentLightLevel/MaxFrameAvgLightLevel`, `ADBEVideoGraphicsWhiteLuminance`, `ADBERenderDeepColor`, `ADBEForceTCTimebase`, `ADBEReorderFrames`, `ADBEKeyframeEvery`, `ADBE_HEVC_Tier`, `ADBEMPEGVideoEncoderNameParam`, `ADBEMPEGVideoEncodingPerformanceParam`, `ADBEVideoFieldType`, `ADBEDNxHDAlphaType`.
- Takeaway: Adobe is a federation of licensed vendor codecs and camera SDKs behind one plug-in host. The engine value is the host (time, graph, cache, GPU, color), not the codecs. Studio should copy that split: stable provider traits, swappable leaves.

</topic>

<topic id="engine-anatomy" status="active" version="1" summary="Pro NLE media layer anatomy" updated_at="2026-10-11">

1. Demux/mux. ISOBMFF/MOV/MP4 (ISO/IEC 14496-12): sample tables, `elst` edit lists (empty edits, media_time offset), `ctts` v0/v1 composition offsets (negative for B-frame start), `sidx`/fMP4, `tmcd` timecode track, `colr` nclx, `pasp`/`clap`/`fiel`. MXF (SMPTE ST 377-1, OP1a/OPAtom, essence mapping ST 379-2, ProRes RDD 44, DNx ST 2019-4): index tables, descriptors carry color/field/timecode. Matroska (RFC 9559, EBML RFC 8794): Cues, default timebase 1 ms. Output sample index = (DTS, PTS, keyframe, byte range, size).
2. Decode. Parse -> reconstruct -> decode-order frames -> DPB reorder to PTS -> conformance-window crop -> planar u8/u16 + color tag. Intra-only codecs (ProRes, DNxHR, CineForm, APV, AVC-Intra): every frame is a sync sample, frame-parallel, trivially exact seek. Inter codecs (H.264/HEVC/AV1/VP9): GOP, open-GOP/CRA leading pictures (RASL undecodable after random access), AV1 temporal units + `show_existing_frame`, VP9 superframes/hidden frames.
3. Seek accuracy. Nearest earlier sync sample -> decode and discard to target PTS -> apply edit list + composition offset. Must return requested vs actual PTS and a disposition (exact / approximate / draft / proxy / offline) (prep MT-35542). VFR means PTS are not multiples of a nominal frame duration; never derive time from frame index. Known-fragile area even in mature engines: dav1d/ffmpeg seek bugs under frame-delay + threads [V] code.videolan.org dav1d issues 369/389, ffmpeg trac 9665.
4. Time. Pulse = integer ticks, 254016000000/s. Divides exactly: 24000, 25, 30000, 60000, 44100, 48000, 88200, 96000, 90000, 600, 1000, 1e6 and 12800 units/s. Does NOT divide 1e7 (Media Foundation 100 ns units = 25401.6 ticks) -> conversion needs a stated rounding rule + receipt. ticks/frame: 24=10584000000, 25=10160640000, 30=8467200000, 50=5080320000, 60=4233600000, 24000/1001=10594584000, 30000/1001=8475667200, 60000/1001=4237833600 (computed). The 254016000000 rate is Adobe's [V] community.adobe.com (all API time in ticks), Media Encoder UXP TickTime doc developer.adobe.com/media-encoder/uxp/media-encoder-api/tick-time/. Drop-frame is a display label only (SMPTE ST 12-1); vector in `research/adobe-behavior.md:52` (frame 1800 @29.97 -> 00:01:00;02).
5. Color metadata. ITU-T H.273 code points (primaries, transfer, matrix, full/limited range) live in H.264/HEVC VUI, AV1 `color_config`, `colr`, Matroska Colour, MXF descriptors, ProRes frame header. HDR: ST 2086 mastering display + MaxCLL/MaxFALL, PQ (ST 2084), HLG. Engine must carry `tagged | assumed(rule)` and never silently guess 601 vs 709 or limited vs full. Adobe exposes all of these as export params (see vendors topic).
6. Hardware. Decode: Media Foundation MFTs + D3D11/DXVA, NVDEC (`nvcuvid`), oneVPL/QSV, AMF, Vulkan Video. Encode: NVENC, AMF, QSV, MF. H.264/HEVC/AV1/VP9 decode output is normative (bit-exact per spec), so conformant HW and SW decoders should match; encoders are not reproducible across vendors/drivers. Capability probing per device/profile/bit-depth is mandatory (a probe reporting "callable" is not fidelity proof, FILM-ROOT-09).
7. Frame pools. Pooled planar buffers keyed (format, w, h, bit depth), refcounted, bounded by BYTES not counts (prep: Craft pools bound counts only), plus GPU texture pool; admission budget spans decoder refs, CPU/GPU workspaces, audio, prefetch.
8. Audio. PCM/BWF/iXML channel maps, AAC-LC/HE-AAC, MP3, Opus, FLAC, ALAC. Sample-accurate priming/encoder delay (`edts`/iTunSMPB, Opus pre-skip), resample, layout mapping.
9. Proxy. Low-res intra (ProRes Proxy, DNxHR LB, all-intra H.264) with identical timebase, timecode, channel layout; per-source proxy/original state; export always reads original (prep FILM-ROOT-05/08).
10. Smart render. Pass-through of untouched GOP-aligned segments when codec/profile/SPS/color/rate match (intra codecs: frame-granular copy; AVC/HEVC: closed GOP + re-encode edge GOPs). Cache keys must be content-aware: (graph-node hash, source identity + revision, tick range, pixel format, color pipeline version, decoder id + version).
11. Caching. Playhead frame LRU, GOP cache, thumbnails, waveform/peak files, conformed audio, proxies, render cache (Adobe "media cache" internals UNVERIFIED [U]).

</topic>

<topic id="specs-and-open-engines" status="active" version="1" summary="Public specs and open reference engines per codec/container" updated_at="2026-10-11">

| Format | Public spec | Open engines (read for understanding; licences not marked [V] are from memory = UNVERIFIED) |
|---|---|---|
| H.264 | ITU-T H.264 / ISO 14496-10 | ffmpeg libavcodec (LGPL core), openh264 (BSD source; Cisco binary carries the patent grant, [V] docs.rs/openh264), x264 (GPL, never copy), JM ref |
| HEVC | ITU-T H.265 / ISO 23008-2 | libavcodec, libde265, x265 (GPL/commercial), HM ref; VideoToolbox/NVENC HW |
| AV1 | AOM AV1 Bitstream & Decoding Process Spec [V] aomediacodec.github.io/av1-spec (fetched copy is an internal draft, mod. 2023-05-25) | dav1d (BSD-2), rav1d (BSD-2, Rust port), libaom, SVT-AV1, rav1e |
| VP9 | WebM VP9 Bitstream & Decoding Process Spec v0.6 | libvpx (BSD) |
| ProRes | SMPTE RDD 36:2022 (public PDF [V] pub.smpte.org/pub/rdd36/rdd36-2022.pdf; a Registered Disclosure Document, not a standard), RDD 44 (MXF) | ffmpeg `prores`/`prores_ks`/`prores_aw` |
| DNxHD/HR (VC-3) | SMPTE ST 2019-1:2016 + Amd 1:2023, ST 2019-4 (MXF) | ffmpeg `dnxhd` |
| CineForm | open SDK [V] github.com/gopro/cineform-sdk | libcineform fork |
| APV | IETF RFC 9924 (cited by Craft crate [S]; not fetched) | Craft `apv` |
| JPEG 2000 / JPEG XS | ISO 15444 / ISO 21122 | OpenJPEG (BSD-2) [U], Kakadu (commercial) |
| Containers | ISO 14496-12/-14, QuickTime File Format, SMPTE ST 377-1/379-2, Matroska RFC 9559 [V], EBML RFC 8794, ISO 13818-1 (TS) | ffmpeg libavformat, GPAC (LGPL) [U] |
| Color/time | ITU-T H.273 (07/24 in force) [V] itu.int/rec/T-REC-H.273, SMPTE ST 12-1, ST 2084/2086, ARIB STD-B67 | - |
| Audio | ISO 14496-3 (AAC), Opus RFC 6716 [U], WAV/BWF (EBU Tech 3285) [U] | Symphonia, libopus |

Engine-reading priority (most instructive): ffmpeg `libavcodec` + `libavformat` (seek/index/edit-list behavior), dav1d (threading + film grain), Craft crate READMEs (below), GStreamer for pipeline/caps negotiation ideas. Licence rule: MIT/Apache/BSD/MPL may be linked; LGPL only as dynamic native leaf; GPL (x264, x265, GPL-enabled ffmpeg) never copied or linked.

</topic>

<topic id="rust-state-and-gaps" status="active" version="1" summary="Rust crate maturity, hardware APIs, patents" updated_at="2026-10-11">

Craft pure-Rust codecs [S] (`gov_runtime/tools/studio-craft-source-cache/filmcraft-7b6c134472287db4367fca80bcb498240ba36567/crates/*/README.md`, MIT OR Apache-2.0, "clean-room from spec; ffmpeg/dav1d only as external test oracle"; NOT built or run by us; numbers are README claims):
- `h264` 8,985 LOC: progressive 8-bit 4:2:0 Baseline/Main/High, CAVLC+CABAC, B-frames, bit-exact vs ffmpeg on its fixtures; interlaced (PAFF/MBAFF), High10, 4:2:2/4:4:4, lossless return `Unsupported`. ~105-122 fps 1080p single thread (M4 Pro). Has a `draft` flag (skips deblocking) = non-reference output.
- `hevc` 6,963: Main/Main10 4:2:0, bit-exact vs ffmpeg on fixtures; RExt/4:2:2/SCC/MV-HEVC unsupported. `av1` 19,548: all spec stages marked done and bit-exact vs libdav1d on tested vectors (aom conformance subset). `vp9` 7,474: profiles 0-3. `mpeg2v`, `apv`, `h264enc` (Constrained Baseline/Main/High, CRF/ABR/2-pass), `aac`, `ac3`, `opus`.
- `prores` 2,014: decode all 422/4444/XQ + alpha, interlaced; encoder progressive, flat matrices; output within +-1 LSB of ffmpeg (not bit-exact IDCT). `dnx` 4,376: all VC-3 CIDs decoded (some untested vs oracle); within +-2 LSB of ffmpeg; README documents 3 places ffmpeg deviates from ST 2019-1 text and measured them by black-box probes (same method as our oracle plan).
- Containers: `isobmff` 4,360, `matroska` 3,982, `mxf` 4,220, `mpegts` 2,103, `ogg`. `platform` (only crate with `unsafe`): Media Foundation MFT + D3D11 via `windows` 0.62, NVENC FFI (`platform/src/nvenc/ffi.rs`), VA-API via libloading, VideoToolbox; hybrid HW-fail -> SW replay.
- Pro gaps: no 10-bit 4:2:2 intra H.264 (XAVC-I / AVC-Intra), no interlaced H.264, no HEVC 4:2:2 10-bit, no Dolby Vision, no MXF OP1a muxer proof, no AAC encoder, MPEG-2 encode.

Other Rust state [V]:
- rav1d: BSD-2, C-API drop-in for dav1d, x86/x86_64/aarch64 stable, ~5% slower than dav1d (May 2025 bounty post), passes dav1d tests, no releases listed; shares dav1d asm. rav1e 0.8.1: encoder, slower than SVT-AV1, upstream activity UNVERIFIED. `openh264` crate: `source` feature compiles Cisco source (no Cisco patent coverage); `libloading` loads Cisco's binary.
- FFmpeg bindings: `ffmpeg-next` (claims FFmpeg 3.4-8.0, third-party mirror) and `rsmpeg` 0.18.0+ffmpeg.8.0 (docs conflict on 6/7 vs 8). `gstreamer-rs`: MIT/Apache bindings; GStreamer is LGPL-2.1+; `gstreamer` 0.25.3 has `v1_28` feature (Fedora packaging).
- Audio: Symphonia 0.5.x/0.6.1 (decode + demux, no encoders; FLAC/Vorbis default, AAC-LC/MP3/ALAC by feature; MPL-2.0 per prep).
- MP4: `re_mp4` (MIT, parser only), `mp4` (alfg, read+write), Mozilla `mp4parse` (track metadata), `mp4box` 0.12 (edit lists + fMP4, young, Rust 1.88+), `iso-bmff` (early, "not production"). Matroska: `mkv-element`, `mkvparser`, `webm-iterable`, `matroska` (metadata only). Rust MXF: only Craft `mxf` found [U].
- Hardware from Rust: `windows` crate `Win32_Media_MediaFoundation` + D3D11 (official, used by Craft); `nvidia-video-codec-sdk` 0.3.1 (Sep 2023, SDK 12.1.14, stale); `shiguredo_amf` (AMF enc+dec H.264/HEVC/AV1); `vpl-sys` 0.1.3 ("not recommended for production"); Vulkan Video: `gpu-video` 0.4.0 (MIT, ex `vk-video`: H.264 enc+dec, HEVC enc only, AV1 none, outputs `wgpu::Texture`), ash Vulkan Video bindings experimental/semver-exempt. A third-party MF zero-copy NV12 decoder found no D3D11-aware HW MFT on RTX 4090 or UHD 770 test machines -> software fallback is mandatory [V] docs.rs mediaway-decoder benchmarks.
- Patent posture (operator-owned; engineering control = feature-gated providers): H.264 Via LA pool expiry NOT confirmed [U]. HEVC: Access Advance now administers the former Via LA HEVC/VVC pool (Dec 2025), 29,000+ patents; Avanci Video covers HEVC/VVC/VP9/AV1/DASH; Access Advance VDP also lists VP9/AV1 [V] businesswire.com Jan 2026 release, streamingmedia.com landscape article. AV1 is royalty-free by AOMedia intent but third-party pools exist. A clean-room Rust decoder removes copyright, not patent, exposure. ffmpeg.org/legal.html: authors do not track patents.

</topic>

<topic id="improvements" status="active" version="1" summary="Where Studio can beat Adobe" updated_at="2026-10-11">

1. Deterministic decode contract. Every frame carries decoder id+version. Delivery uses a pinned conformant software decoder (H.264/HEVC/AV1/VP9 are bit-exact normative, so SW==HW must hold and can be hash-tested); HW is preview unless parity-proven. ProRes/DNx IDCT is not normative (Craft deviates +-1..2 LSB from ffmpeg), so pin one reference implementation. Export forbids `draft`/approximate frames (typed disposition, prep FILM-ADOPT-04). Validation: per-frame hash manifest on synthetic fixtures.
2. Index-first frame-accurate seek. Build the full sample/keyframe/edit-list index at ingest (cheap for MP4/MXF tables), persist it, return requested vs actual PTS, "exact or error" in export mode. Fixtures: VFR, negative ctts, empty edits, open-GOP HEVC, fMP4.
3. Open codecs first-class. Default closure = pure Rust (AV1, VP9, Opus, FLAC, PCM, ProRes, DNx, image sequences/EXR); proprietary is an opt-in provider, not the base.
4. GPU zero-copy to wgpu. Vulkan Video decode (`gpu-video` style) into `wgpu::Texture`, NV12->RGB in shader with color metadata uniforms; MF/D3D11 -> shared handle path is UNVERIFIED for wgpu [U] and needs a spike with SW-copy fallback.
5. Crash isolation. Native codecs run in a worker subprocess (`reel-native` host) with framed IPC + shared-memory frame ring, deadline + heartbeat counters, kill/restart + bounded replay from the last sync sample (Craft hybrid-replay idea). Vendor SDK DLLs sit in the Adobe app directory (names above) [L]; whether Adobe isolates them out-of-process is UNVERIFIED [U]. Pure-Rust leaves additionally get cargo-fuzz on demux/parse.
6. Atomic, verified export. Stage -> mux -> independent re-decode (own decoder + ffprobe) of frame/sample counts, PTS, color tags -> atomic publish; quarantine cancelled files (fixes FILM-ROOT-08 gaps).
7. Content-aware source identity (sampled head/tail/size hash + header), not path keys (prep FILM-ROOT-04/05).
8. Open, versioned preset schema with an `.epr` importer for the knob inventory above; headless CLI/JSON control + receipts for parallel agents (Adobe apps are GUI-bound; GLOBAL-BUILD-PARALLEL).
9. Explicit color-assumption receipts: `tagged | assumed(rule)` on every decoded frame and export.

</topic>

<topic id="black-box-plan" status="active" version="1" summary="Oracle export + comparison plan; needs Operator approval" updated_at="2026-10-11">

Gate: running any Adobe app opens foreground windows -> needs Operator approval for ONE session (CX-SAFE-002). Candidate lower-footprint entry points [L]: `AfterEffects/Support Files/aerender.exe` (CLI renderer), `Premiere/PProHeadless.exe` (role UNVERIFIED [U]); AME `AMEWebService.exe` role UNVERIFIED. Do not launch without approval. No binary inspection at any step (lead decision, START_LOG `decisions`).
1. Fixtures (synthetic, open): generate with installed `ffmpeg 8.1.1` (`C:/Users/Ilja Smets/AppData/Local/Microsoft/WinGet/Packages/Gyan.FFmpeg_.../ffmpeg-8.1.1-full_build/bin/ffmpeg.exe`, [L] found on PATH): `testsrc2`, `smptehdbars`, gray/color ramps (limited vs full range), 8/10-bit 4:2:0/4:2:2, 23.976/25/29.97 DF/59.94, VFR, interlaced TFF/BFF, alpha disc, click-track + sweep audio, with embedded `tmcd` and explicit bt709/bt2020/PQ/HLG tags, plus UNTAGGED twins. Output dir under `Handshake_Artifacts/WP-KERNEL-STUDIO/engine-video-oracle/`.
2. Export matrix with existing AME presets [L] (record `PresetID` + AME build): ProRes 422 HQ/4444 XQ with alpha (MOV and MXF), one DNxHD/HR MXF preset per family (folder `4d584620_444d5846`), H.264 "00 - Match Source - High bitrate", HEVC "02 - Match Source - PQ/HLG/2020", XDCAM, DPX/EXR sequence. Vary the deep-color switch (`ADBERenderDeepColor`) and `ADBEExportColorSpace` per preset.
3. Measure (read-only tools): `ffprobe -show_streams -show_format -show_packets -of json` for codec profile/level, pix_fmt, primaries/transfer/matrix/range, field_order, time_base, r/avg_frame_rate, start_time, GOP/B-frame structure, audio delay; own ISOBMFF/MXF parser for `elst`, `ctts`, `colr`, `tmcd`, `mdcv`/`clli`, MXF descriptors; decode with ffmpeg AND a second decoder (Craft-derived/own) and compare max-abs/PSNR/histograms to isolate decoder vs encoder deviation.
4. Questions to answer: effective RGB<->YCbCr matrix/range/rounding for tagged vs untagged sources; HDR tag writing; timecode + drop-frame round trip; VFR conform (dup/drop rule); field order + deinterlace; alpha premult in 4444; audio priming, resample, loudness; moov position/interleave; bitrate-control vs file size; "Match Source" resolution/rate rules.
5. Evidence discipline: results are hypotheses until re-run independently (GLOBAL-CLOSURE-040); keep the Craft method (probe streams with single coefficients, fit constants) for codec-level deviations.
6. Seek/playback behavior in Premiere is not measurable without GUI automation; out of scope for this pass.

</topic>

<topic id="recommendations-for-reel-and-reel-native" status="active" version="1" summary="Concrete crate-set and shape recommendations" updated_at="2026-10-11">

Default pure-Rust set (`hsk-studio-reel`, no C toolchain, no `unsafe` in default closure beyond audited leaf crates):
- Containers: own thin ISOBMFF/MOV index+edit-list+`tmcd` reader (reference: Craft `isobmff`, MIT/Apache; `mp4box` is young), Matroska via `mkv-element` or own, MXF reader per ST 377-1 (reference: Craft `mxf`), WAV/BWF/AIFF.
- Video decode: ProRes (RDD 36) and DNxHD/HR (ST 2019-1) first (intra, exact seek, proxy-grade, small, pure-Rust feasible); then AV1 (`rav1d`, enable `asm` feature only as opt-in, or spec-derived clean-room), VP9, H.264 progressive 8-bit and HEVC Main/Main10 (Craft-scope, extend to High10/4:2:2/interlaced as roadmap).
- Audio: Symphonia (MPL-2.0 file-level copyleft acceptable as dependency) for AAC-LC/MP3/FLAC/ALAC/Vorbis decode, own PCM/Opus path.
- Encode: ProRes + DNxHR (intra, small), `rav1e` AV1, atomic mux to MOV/MP4/MXF/WAV.
- Shape (matches prep MT-35542): explicit provider list injected (no global registry); `MediaSource{probe,index}`, `VideoDecoder::decode_segment(range)->frames`, `Frame{planes,format,bit_depth,color:ColorTag{prim,transfer,matrix,range,origin:Tagged|Assumed},pts_ticks,duration,field,disposition}`; byte-budgeted pool; cancel tokens; Pulse tick conversions with receipt for non-divisible timebases (100 ns MF).

Native provider set (`hsk-studio-reel-native`, opt-in, pinned, subprocess-isolated):
- Windows: Media Foundation MFT + D3D11 (`windows` crate) for H.264/HEVC/AV1 HW decode and H.264/HEVC encode, probe first, SW fallback; NVENC/NVDEC by direct FFI with `libloading` (the `nvidia-video-codec-sdk` crate is stale); AMF (`shiguredo_amf`) and oneVPL (`vpl-sys`) only after a probe spike; Vulkan Video (`gpu-video`/`ash`) as cross-vendor GPU-resident path.
- Long tail: FFmpeg via `rsmpeg`/`ffmpeg-next`, dynamic LGPL build only (no `--enable-gpl/nonfree`), version pinned in receipts.
- Excluded from this pass: commercial vendor SDKs (MainConcept, RED, ARRI, Sony, Canon) -> separate licence/RAW decision; GStreamer (heavy runtime, per-plugin licence matrix).
- Worker protocol: framed IPC + shared-memory ring, heartbeat counters, deadline, kill + replay from sync sample, typed `Unavailable` where bounded termination is unproved (MT-35561).

Proof (minimal, per Operator): synthetic ffmpeg fixtures; AV1 bit-exact vs dav1d; H.264/HEVC bit-exact vs ffmpeg; ProRes/DNx within documented LSB tolerance; independent demux of every export. Milestone order: M0 Pulse tick math + Reel traits + ISOBMFF index; M1 ProRes/DNx decode; M2 AV1/H.264/HEVC decode; M3 encoders + mux; M4 reel-native MF/NVENC; M5 subprocess isolation.

</topic>

<topic id="sources" status="active" version="1" summary="URLs and local paths" updated_at="2026-10-11">

Local: Adobe roots above; `PPro/MediaIO/systempresets/**.epr`; Craft cache `D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/filmcraft-7b6c134472287db4367fca80bcb498240ba36567/` (`Cargo.toml`, `NOTICE`, `crates/{h264,hevc,av1,vp9,prores,dnx,apv,mpeg2v,h264enc,platform,codecs,isobmff,mxf,matroska,time}`); prep files in `prep-already-covers`.
Web (searched/fetched 2026-10-11): https://ffmpeg.org/legal.html ; https://docs.rs/openh264/ ; https://www.memorysafety.org/blog/rav1d-perf-bounty/ ; https://github.com/memorysafety/rav1d ; https://docs.rs/crate/rav1e/latest ; https://docs.rs/crate/rsmpeg/0.18.0 ; https://docs.rs/gpu-video/latest/gpu_video/ ; https://github.com/viliamvadocz/nvidia-video-codec-sdk ; https://docs.rs/shiguredo_amf ; https://docs.rs/vpl-sys ; https://docs.rs/crate/mediaway-decoder/0.1.3/source/docs/windows/benchmarks.md ; https://docs.rs/crate/symphonia/latest ; https://lib.rs/crates/re_mp4 ; https://docs.rs/crate/mp4box/0.12.1 ; https://docs.rs/mkv-element ; https://pub.smpte.org/pub/rdd36/rdd36-2022.pdf ; https://aomediacodec.github.io/av1-spec/ ; https://www.rfc-editor.org/rfc/rfc9559.html ; https://www.itu.int/rec/T-REC-H.273/en ; https://github.com/gopro/cineform-sdk ; https://developer.adobe.com/media-encoder/uxp/media-encoder-api/tick-time/ ; https://community.adobe.com/t5/premiere-pro-discussions/inconsistency-in-audio-durations-frames-clip-posistioning-relative-to-the-30-frames/td-p/14304491 ; https://gstreamer.pages.freedesktop.org/gstreamer-rs/stable/latest/docs/ ; https://www.businesswire.com/news/home/20260126799824/en/Access-Advance-Extends-HEVC-Advance-Rate-Increase-Deadline ; https://www.streamingmedia.com/Articles/Editorial/Featured-Articles/Decoding-the-Landscape-Recent-Developments-in-Video-Codec-Licensing-163913.aspx ; https://code.videolan.org/videolan/dav1d/-/issues/369.
NOT_INSPECTED: Adobe online third-party notices; GoPro CineForm LICENSE file; Via LA AVC/Sisvel pool pages; ST 2019-1 and ISO/ITU spec texts (paywalled or large); Apple ProRes licensing; wgpu external-memory import status.

</topic>
