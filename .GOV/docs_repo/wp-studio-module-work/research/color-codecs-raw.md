---
file_id: WP-STUDIO-MODULE-WORK-RESEARCH-COLOR-CODECS-RAW
file_kind: research_note
updated_at: 2026-10-11T01:30:00Z
wp: WP-KERNEL-STUDIO
lane: color-codecs-raw (GAP/DELTA note; deep color = engine-color.md, deep video = engine-video.md, not repeated here)
---

Prose research surface, not authority. Method: spec + prep corpus read first (bounded greps); crate facts from crates.io API (2026-10-11), upstream READMEs, and moxcms-0.9.1 source in the local cargo registry. Anything not so checked is labelled UNVERIFIED. Adobe behaviour: public specs + black-box observation only (ACR/Lightroom exports as reference renders), no decompilation. Studio cargo builds/tests/`cargo tree`/`cargo deny` run on D: only (tool installs on C: allowed).

<topic id="prep-already-decided" status="active" summary="What spec and prep corpus already fix, per crate">

## 1. Already decided (do not re-research)

Spec = `.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`; prep = `.GOV/reference/studio_app_feature_research/repass-2026-10-09/*.json` (RP), `.GOV/docs_repo/studio-collaboration-2026-10-09/modular-authoring/*/preparation.json` (MA), `.GOV/task_packets/WP-KERNEL-STUDIO/STUDIO_MODULES.json` (SM).

| Crate | Decided (source) |
|---|---|
| prism | `ColorEngine: Send+Sync` owns ICC/OCIO/LUT in Studio code; no platform CMM, no system LittleCMS "in any build profile, including tests"; implement ICC or vendor a pure-Rust CMS behind the trait (STU-COL-140/141, L10141-10156). Intents encoded 0..3, BPC an independent bool (STU-COL-135/136). Matrix+curve preferred over baked LUT; shaper mandatory for log/linear/HDR; grid 33/65, float storage; interpolation rule in cache key (STU-COL-142/353-355). Materialisation scalar and bit-identical, application = tolerance only (STU-COL-143/350-352). OCIO v1+v2.x semantics, roles, ACES defaults, config provenance, ACES version read from config never compiled in (STU-COL-150-157, 310-317). CIEDE2000 gate, 34 Sharma pairs, expected values independent of engine under test (STU-COL-320-326). RP engineering.json ENG-REUSE-STACK: moxcms "candidate, not source qualified"; RP photocraft-root-drill PHOTO-REUSE-002: PhotoCraft's own pure CMS only as comparison, independent ICC corpus required. Prism today: moxcms `=0.9.1`, features `["extended_range"]` only, BPC rejected, `MAX_PROFILE_BYTES=65536`, LUT/OCIO/softproof/gamut/BPC listed pending (prism `DESCRIPTOR`). |
| pigment | Tile format `rgb_f32le` + separate straight-alpha f32 plane; masks `coverage_u8`/`coverage_u16le`; packed/premultiplied/native-endian need explicit adapters (STU-CON-028, L22502). PhotoCraft pattern: sparse COW tiles, zstd CAS (RP PHOTO-REUSE-003/005, adapt not copy). |
| develop | SM: owns "RAW decode/develop recipes". First slice = decoded CFA u16le in, bilinear_reflect_native_v1, scalar + SSE2 normalisation only, f32 out; container decode explicitly pending (STU-CON-042-046, L22574-22608). Pinned reference leaf: LightCraft commit 88ea1a80, MIT/Apache-2.0 provenance, bilinear + Hermite spline. RP light-effect LE-REUSE-L01/L02: raw/tiff/meta/codecs/heif leaves = selective adaptation; LE-L02: preview-only/profile-approximation paths must be flagged, fixtures vs Lightroom/ACR renders. RP photocraft PHOTO-F-007: CR3/RAF recognised but rejected. Camera profile model: 2 matrices + forward matrices + hue/sat/look LUT (STU-COL-218/219). |
| reel / reel-native | SM: pure media contracts in `reel`; native codecs opt-in in `reel-native`, never reachable from raster/type default closure. STU-VID-051: codecs are an adapter boundary with capability descriptors, absent adapter = `MEDIA_CODEC_UNAVAILABLE`, which codecs ship is a licensing decision. MA media-team `reel`/`reel_native`: independent decoder oracle, FFI confined to provider crate. RP filmcraft FILM-MOD-01/03: pick codec/container packages independently; ffmpeg/ffprobe oracle missing = NO_PROOF. |
| score / score-device | SM: score pure DSP, device I/O optional; MA media-team `score`, `score_device`: preallocated callback, no alloc/locks. RP filmcraft: symphonia MPL-2.0 audio bootstrap; pure leaves audio-dsp/ogg/opus/matroska/mxf have no normal deps (FILM-MOD-02). |
| render-gpu | SM: wgpu backend, API types hidden. RP photocraft-gpu: wgpu 30, Rgba32Float accumulation with Rgba16Float downgrade; RP light-effect LE-L08: GPU path declines non-U8, CPU/GPU depth paths differ. |
| (image codecs) | STU-IO-006 lists png/jpeg/gif/webp/heic/exr/jxl/tiff/bmp/qoi rows and "codec availability surfaced as capability receipt" (L21313-21330). LightCraft workspace pins (RP light-effect-root-drill L180-270): image 0.25, png 0.18, tiff 0.11, image-webp 0.2, zune-jpeg 0.5, jpeg-encoder 0.7, jxl-oxide 0.12, zune-jpegxl 0.5, ravif 0.13, moxcms 0.9 `lut`, wgpu 30, heic-rs `=0.1.1`. |

</topic>

<topic id="remaining-gaps" status="active" summary="Verified gaps for dependency choices">

## 2. Remaining gaps (verified unless marked)

**G1 prism / moxcms 0.9.1** (BSD-3-Clause OR Apache-2.0, MSRV 1.89, pure Rust, published 2026-09-15, 95M downloads; source read in local registry):
- ICC v2.0-v4.4 accepted; v5/iccMAX explicitly rejected (profile.rs L114/L145). No iccMAX engine found for Rust or lcms2 (lcms2 claim UNVERIFIED) -> iccMAX = unsupported receipt + opaque-preserve.
- `TransformOptions` has NO BPC field (`// pub black_point_compensation`, transform.rs L112). BPC runs implicitly inside the LUT path (`detect_black_point`, lut_transforms.rs L363-371), not at all in matrix-shaper. STU-COL-136 (independent BPC bool) cannot be met by a flag: Prism must own BPC (XYZ black scaling) or vendor/patch moxcms (STU-ARC-018: pinned hash + patch provenance).
- Prism enables only `extended_range`; LUT-based profiles (CMYK, Lab, press, scanner, `mAB`) need feature `lut` (default feature in moxcms; LightCraft pins `["lut"]`). Turn off avx*/sse*/neon* features to keep the materialisation path scalar (STU-COL-351). Raise/replace the 64 KiB profile cap before CMYK (real CMYK profile sizes UNVERIFIED).
- Supported: CMYK, Lab, Gray, any 3/4-colour, up to 16 inks; 8/10/12/16-bit/f32/f64 transforms; CICP; tetrahedral/trilinear selectable; builtins sRGB, Adobe RGB, Display P3 (+PQ), BT.2020 (+PQ/HLG), ACES2065-1 and ACEScg linear (defaults.rs). No ProPhoto builtin found (build from ROMM primaries/TRC; UNVERIFIED). No per-pixel gamut-clip report API found (STU-COL-138): Prism computes it (e.g. unclamped f32 round-trip compare).
- `image` 0.25.10 depends non-optionally on moxcms `^0.8.0`: any crate in the closure using `image` yields two moxcms versions (0.8.x + `=0.9.1`) and two ICC engines. Prefer per-format crates, no `image`.
- Alternatives: lcms2 6.2.0 / lcms2-sys 4.0.7 (MIT, C, bundles LittleCMS 2.19.1 with `static`) = banned as runtime dep by STU-COL-140; qcms 0.3.0 (MIT, last release 2024-01-09); kolor 0.1.9 (2023, game colour only); palette 0.7.7 and color 0.3.3 (colour math, no ICC); colorutils-rs 0.8.0 (same author as moxcms, conversions). Recommendation: keep moxcms + Prism-owned BPC/gamut/OCIO; moxcms is also what jxl-oxide offers as pure-Rust CMS feature.

**G2 prism / OCIO** (spec requires OCIO v1+v2.x): upstream OCIO is C++ (BSD-3, v2.6.0 released 2026-10-01 per GitHub API). `ocio` crate 0.1.0 (2026-10-04, MIT AND BSD-3-Clause, pure-Rust port, f32 RGBA CPU ops, config/fileformats/builtins/baker, GPU shader text only; follows OCIO main 93878f9 = between 2.5.0 and 2.6.0; 325 downloads, WIP) is the first real candidate; maturity/conformance UNVERIFIED. `opencolorio` 0.1.0 (2020) is a placeholder. Decision needed: vendor `ocio` as pinned leaf behind ColorEngine (cheapest) vs own subset (YAML via `yaml-rust2` 0.13.0 or `saphyr` 0.1.0; `serde_yaml_ng` 0.10.0 older). Guard RP light-effect LE-E08: unsupported transform must be a typed error, never identity; cache by closure hash (STU-COL-314). Latest published ACES config release: OpenColorIO-Config-ACES v4.0.0 (2025-10-02, GitHub API).

**G3 accuracy oracle (STU-COL-325)**: prep names no engine-independent ICC oracle. Proposed layers: (a) f64 analytic matrix/TRC oracle from published primaries (sRGB IEC 61966-2-1, Display P3, BT.2020, ROMM); (b) the 34 Sharma pairs for the metric; (c) a second CMM for goldens: LittleCMS 2.19.1 via lcms2-sys `static` inside an out-of-workspace golden generator writing frozen JSON stamped "lcms2 6.2.0 / LittleCMS 2.19.1" (lead/spec reading of STU-COL-140 needed: bundled-static is not "system-installed" but is C); (d) PhotoCraft's pattern of moxcms as dev-dependency cross-check for any in-house CMS code (RP photocraft-cms); (e) ACR/Lightroom exported renders as black-box reference for develop/camera-look (LE-L02).

**G4 develop / RAW**: `develop` owns container decode (SM) but no dependency is chosen.
- Rust RAW crates are copyleft: rawler 0.8.0 (dnglab, LGPL-2.1, README "alpha", unstable API, ~857 camera rows in SUPPORTED_CAMERAS.md counted by line, decode + DNG write), rawloader 0.37.2 (LGPL-2.1, 22 format entries; outputs CFA, black/white, WB multipliers, camera->XYZ matrix), imagepipe 0.5.1 (LGPL-3.0-only), dng 1.6.0 (AGPL-3.0). LibRaw wrappers: rsraw 0.1.1 (MIT wrapper over C LibRaw; LibRaw licence UNVERIFIED), libraw-rs 0.0.4 (2021). Recommend: no copyleft crate in default closure; use rawler as black-box camera-coverage/DNG cross-check outside the closure; adapt the pinned MIT/Apache LightCraft raw leaf (RP LE-REUSE-L01) per STU-ARC-018; write DNG/TIFF-EP reader from the public DNG 1.7.1.0 spec (Sept 2023; SDK 1.7.1 build 2611, licence UNVERIFIED) for ColorMatrix1/2, ForwardMatrix, AsShotNeutral, opcodes, BaselineExposure.
- Demosaic: only `bayer` 0.1.5 (2017) found as a crate; AHD appears in LightCraft preview path; RCD/DCB/AMaZE permissive Rust implementations UNVERIFIED (do not copy GPL darktable/RawTherapee code; implement clean-room from papers). Algorithm stays a receipted parameter (STU-RAW-003).
- Lens: lensfun 0.7.0 (pure-Rust port, LGPL-3.0-or-later, 1,640 A/B cases vs C++ per README, bundled DB ~574 KB gz). DB licence UNVERIFIED. STU-RAW-009 wants Studio-native lens assets: implement ptlens/poly3/poly5/TCA/vignetting math natively; import lensfun XML as runtime data; do not link the LGPL crate.
- EXIF/metadata: kamadak-exif 0.6.1 (BSD-2, pure); avoid nom-exif 3.8.0 ("non-standard" licence).
- SIMD: scalar oracle first, then `core::arch` dispatch per STU-CON-043; `half` 2.7.1 for f16 storage.

**G5 still-image codecs (owner gap + choices)**: no module in SM owns png/jpeg/tiff/exr/webp/jxl/avif/heic decode-encode (pigment = tiles/samples, interop-psd = PSD). Lead/spec decision needed; module name NOT invented here. Verified choices (pure Rust unless noted):
- png 0.18.1; tiff 0.11.3 (BigTIFF, CMYK 8-64 bit, IEEE f32/f64; decode LZW/Deflate/PackBits/Fax4/JPEG/ZSTD, encode LZW/Deflate/PackBits only; `ExtraSamples` unsupported per README); gif 0.14.2; qoi 0.4.1; bmp crate stale (2019).
- exr 1.74.2 (BSD-3, no unsafe, PIZ/ZIP/PXR24/B44; README status text says DWA unsupported while checklist ticks dwaa/dwab: UNVERIFIED; deep data not supported).
- JPEG: zune-jpeg 0.5.15 decode (MIT/Apache/Zlib); jpeg-encoder 0.7.1 encode (licence `(MIT OR Apache-2.0) AND IJG`: keep IJG notice); jpeg-decoder 0.3.2 older.
- WebP: image-webp 0.2.4 decode lossy+lossless, encode lossless only.
- JXL: decode jxl-oxide 0.12.6 or official `jxl` 0.7.4 (libjxl/jxl-rs, BSD-3); encode: zune-jpegxl 0.5.2 lossless only (8/16-bit); lossy encode has no permissive pure-Rust crate: jxl-encoder 0.3.1 = AGPL-3.0-only OR commercial, jpegxl-rs 0.16.0 = GPL-3.0-or-later over libjxl C++. Lossy JXL = defer or optional native provider.
- AVIF: encode ravif 0.13.0 (BSD-3) + rav1e 0.8.1 (BSD-2, last release 2025-06-16; nasm optional); decode rav1d 1.1.0 (BSD-2, last release 2025-05-07; c2rust-style port with unsafe + optional nasm asm, README) + avif-parse 2.1.0 (MPL-2.0); avif-decode 3.0.0 and aom-decode 1.0.0 declare MSRV 1.98 > workspace 1.97.1 (incompatible); dav1d 0.11.1 = C.
- HEIC: heic-rs 0.1.1 (MIT/Apache, "no C, no unsafe", 2026-09-12, 19.6k downloads; LightCraft wrapper pins `=0.1.1`) vs heic 0.1.6 (AGPL OR commercial) vs libheif-rs 3.0.0 (C). HEVC patent exposure UNVERIFIED -> off by default, capability receipt.
- Sample types: u8/u16 native, f16 via half, f32; resize/convert SIMD: fast_image_resize 6.1.0, pic-scale 0.7.12, yuvutils-rs 0.8.3.
- Native tile/package compression (not owned by these crates): lz4_flex 0.14.0 (pure, no unsafe default), miniz_oxide 0.9.1 / zlib-rs 0.6.8 (pure), ruzstd 0.9.0 (decoder; encoder completeness UNVERIFIED), zstd 0.14.2 (C libzstd binding).

**G6 reel / score**:
- Audio decode: symphonia 0.6.1 (MPL-2.0, 100% safe, pure). Defaults royalty-free: FLAC, Vorbis, PCM, ADPCM; MKV/WebM, OGG, WAV. Opt-in: AAC-LC, MP3, ALAC, ISO/MP4, AIFF, CAF. README status "-" (not implemented): Opus, HE-AAC, WavPack. Opus gap: libopus via `opus` 0.4.0 (C) or FilmCraft opus leaf (RP FILM-MOD-02). hound 3.5.1 (Apache-2.0, 2023) WAV encode; claxon/lewton stale (2020/2021); cpal 0.18.2 (Apache-2.0) only for score-device; rubato 5.0.1 (MIT/Apache) resampling; dasp 0.11.0 stale (2020); fundsp 0.23.0 optional.
- Video: pure Rust = AV1 only (rav1e/rav1d above). H.264/HEVC/ProRes/DNxHR: no pure-Rust codec verified (ProRes/DNxHR crate search not done: UNVERIFIED). openh264 0.9.8 (BSD-2 wrapper, C++ build; binary-vs-source patent coverage UNVERIFIED); ffmpeg-next 9.0.0 / ffmpeg-sys-next 9.0.1 (WTFPL wrappers; linked libav* are LGPL or GPL depending on build); rsmpeg 0.18.0 (MIT); gstreamer 0.25.4 (MIT/Apache, needs GStreamer install, violates CX-503S as prerequisite). Host has ffmpeg 8.1.1 full build on PATH: usable only as external oracle outside the closure.
- Mux/demux: symphonia (demux MP4/MKV/OGG); mux: muxide 0.2.5 (pure MP4 muxer, maturity UNVERIFIED), mp4 0.14.0 (MIT, 2023), matroska-demuxer 0.8.1 (demux only), webm 2.2.1 (MPL-2.0, libwebm C++), re_mp4 0.5.1 (read), ogg 0.9.2.
- HW decode: no codec API found in the wgpu 30 changelog (its video-frame import entries are web-backend only; absence elsewhere UNVERIFIED). Candidates: gpu-video 0.4.0 (MIT, 2026-05-12, Vulkan Video + wgpu; vk-video 0.3.1 deprecated/renamed) and `windows` 0.62.2 (MIT/Apache) for Media Foundation, both as `reel-native` provider features only.

**G7 render-gpu / raster**: wgpu 30.0.1 (2026-08-22, MIT/Apache, MSRV 1.87; compute core; changelog shows TEXTURE_FORMAT_16BIT_NORM and f16/subgroup work). vello 0.11.0 (2026-10-02, MSRV 1.89) depends on `wgpu ^30.0.0` (optional): pin wgpu 30 to avoid duplicate wgpu trees. vello_cpu 0.3.0 (MSRV 1.89, SIMD/multithread) and tiny-skia 0.12.0 (BSD-3, Skia subset) = CPU 2D reference candidates for render-cpu (conformance vs each other UNVERIFIED). All MSRVs <= 1.97.1.

</topic>

<topic id="recommendations-per-crate" status="active" summary="Dependency table; all pins exact (=x.y.z) per workspace convention">

## 3. Per-crate dependency table

| Crate | Recommended deps (default closure first, opt-in after `+`) | Pure Rust? | Risk |
|---|---|---|---|
| prism | moxcms `=0.9.1` (default-features=false, `extended_range`, add `lut` when CMYK/LUT admitted, no avx/sse/neon); sha2; + `ocio` 0.1.0 vendored leaf (decision G2) | Yes | Medium-high: BPC not controllable in moxcms (G1); iccMAX unsupported; `ocio` 1 week old; 64 KiB profile cap blocks CMYK; STU-COL-351 scalar materialisation must be proven against moxcms SIMD features |
| pigment | std only, `half` 2.7.1 (f16); pixel conversion SIMD optional (pic-scale / fast_image_resize / yuvutils-rs) | Yes | Low; premultiplied/planar adapters are Pigment code (STU-CON-028) |
| develop | `half`; own DNG/TIFF-EP reader; adapted LightCraft raw leaf (MIT/Apache, pinned 88ea1a80); kamadak-exif 0.6.1; + lens data import (lensfun XML as data) | Yes | High: no permissive maintained demosaic/RAW crate; rawler/rawloader/imagepipe LGPL, dng AGPL (excluded); CR3/RAF gaps; Adobe look parity needs black-box ACR renders |
| still-image codec owner (undecided, G5) | png `0.18.1`, tiff `0.11.3`, exr `1.74.2`, zune-jpeg `0.5.15`, jpeg-encoder `0.7.1`, image-webp `0.2.4`, jxl-oxide `0.12.6`, gif `0.14.2`, qoi `0.4.1`; + ravif `0.13.0`/rav1e `0.8.1`, rav1d `1.1.0`+avif-parse `2.1.0`, heic-rs `=0.1.1`, zune-jpegxl `0.5.2` | Yes (rav1d has unsafe/asm) | Medium: owner module missing; two moxcms versions if `image` used; AVIF decode and lossy JXL encode weak; HEVC patent exposure for HEIC |
| reel | symphonia `0.6.1` (default royalty-free features; aac/mp3/isomp4 opt-in); hound `3.5.1`; rav1e/rav1d for AV1; muxide `0.2.5` or mp4 `0.14.0` mux (decide after oracle test) | Yes | Medium: no Opus decode in symphonia; no pure H.264/HEVC/ProRes/DNxHR; MPL-2.0 file-level copyleft; mux crates thin |
| reel-native | + ffmpeg-next `9.0.0` or rsmpeg `0.18.0` (user-supplied FFmpeg, not bundled), openh264 `0.9.8`, gpu-video `0.4.0`, windows `0.62.2` (Media Foundation), opus `0.4.0` | No (FFI isolated) | High: FFmpeg LGPL/GPL build selection, H.264/HEVC patents (UNVERIFIED), external install prerequisite vs CX-503S -> opt-in feature, capability receipt |
| score | std; rubato `5.0.1`; own DSP (no dasp); symphonia via reel only | Yes | Low |
| score-device | cpal `0.18.2` | No (OS audio APIs) | Medium: callback rules STU-ARC-013 are Studio code, not cpal |
| render-gpu | wgpu `=30.0.1` (default-features=false; dx12/vulkan/metal/wgsl as LightCraft); vello `0.11.0` only if vector GPU raster wanted | Yes (drivers external) | Medium: GPU/CPU tolerance STU-COL-352; non-U8 declines (LE-L08); vello MSRV 1.89 |
| render-cpu | tiny-skia `0.12.0` and/or vello_cpu `0.3.0` as reference rasteriser | Yes | Low-medium: AA/rounding differs between rasterisers; pick one oracle and record version |

Top actions for lead: (1) decide BPC ownership + `lut` feature + profile cap in prism; (2) decide `ocio` vendor-vs-own; (3) assign an owner module for still-image codecs; (4) record licence policy: no GPL/AGPL/LGPL crate in default closures (rawler, rawloader, imagepipe, dng, jxl-encoder, jpegxl-rs, heic, lensfun excluded); (5) pick the golden-oracle approach in G3.

</topic>

<topic id="sources" status="active" summary="URLs checked">

## 4. Sources
- crates.io API per crate: `https://crates.io/api/v1/crates/<name>` (all versions/licences/MSRV above; fetched 2026-10-11); dependency lists `.../crates/<name>/<ver>/dependencies` (image->moxcms, vello->wgpu, rav1d->libc).
- https://github.com/awxkee/moxcms (README minimal; facts from `moxcms-0.9.1` source in `~/.cargo/registry`), https://github.com/doubleailes/ocio-rs (README, PORTING.md, UPSTREAM.md), https://api.github.com/repos/AcademySoftwareFoundation/OpenColorIO/releases/latest, https://api.github.com/repos/AcademySoftwareFoundation/OpenColorIO-Config-ACES/releases/latest, https://api.github.com/repos/gfx-rs/wgpu/releases/latest
- https://github.com/dnglab/dnglab (+ SUPPORTED_CAMERAS.md), https://github.com/pedrocr/rawloader, https://github.com/pedrocr/imagepipe, https://github.com/vdavid/lensfun-rs, https://github.com/kornelski/rust-lcms2-sys
- https://github.com/pdeljanov/Symphonia, https://github.com/memorysafety/rav1d, https://github.com/johannesvollmer/exrs, https://github.com/image-rs/image-webp, https://github.com/image-rs/image-tiff, https://github.com/tirr-c/jxl-oxide, https://github.com/etemesi254/zune-image (zune-jpegxl, zune-jpeg), https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md, https://github.com/linebender/vello
- DNG spec/SDK listing: https://helpx.adobe.com/au/camera-raw/desktop/dng-and-file-formats/digital-negative.html (search result; SDK licence text not seen)
- Prep corpus and spec paths in topic 1. Not fetched / UNVERIFIED: LibRaw licence text, Lensfun DB licence, openh264 binary licence terms, HEVC/H.264 patent status, ProRes/DNxHR Rust crates, real CMYK profile byte sizes, ocio-rs conformance.

</topic>
