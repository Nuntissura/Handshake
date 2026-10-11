---
file_id: BUILDER-BRIEF-LANE-REEL-ENG-2
file_kind: builder_brief
updated_at: 2026-10-11T12:55:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-reel
lane: SMB-REEL-ENG-2
depends_on: [reel @ e8b464b (video route, isobmff), lane SMB-REEL-ENG-1 (Export guard, fiel/clap raw fields) landed first because both edit isobmff.rs/video.rs]
author_role: researcher (no product code written; no cargo run; an independent from-spec Python decoder was written and run ONLY as research evidence, see topic cross-validation)
---

<topic id="finding" status="active" summary="Nothing exists for ProRes or DNx; integration points that do exist">

## Finding: the codec is entirely missing; the plumbing it plugs into is present
- `grep -ril "prores|apch|icpf|dnxh|vc-3"` over `crates/`, `tools/`, `.config/`, root `Cargo.toml` of `wt-studio-assets/handshake-studio-assets` (HEAD e8b464b): 0 matches. No ProRes/DNx code, test, fixture or descriptor exists.
- Present and reusable (crates/hsk-studio-reel):
  - `VideoCodec` is `#[non_exhaustive]` with one member `Av1` (`src/video.rs:50-62`). A decoder is a `VideoDecodeSession` (`send(packet, tag, cancel, out)`, `finish`, `:137-151`) producing `DecodedPicture { tag, format: VideoFormat, film_grain_applied, planes }` (`:90-98`). `Forward::accept` (`:321-369`) already refuses a picture whose planes disagree with its format, a picture larger than the container-declared size (`within_declared_size` `:307`), a mid-segment format change, and a packed frame above `max_chunk_bytes`, all BEFORE packing. The receipt carries `decoder_id`/`decoder_version` from `DecoderIdentity` (`:507-508`).
  - Routing today: `choose()` (`:221-230`) picks the in-process decoder only for AV1 + `SourceTrust::Trusted` + feature `av1`; `IsobmffProvider::deliver` routes only `av01` (`src/isobmff.rs:1717-1739`); `parse_visual_children` keeps `colr`/`pasp`/`av1C` (`:687-732`), so a ProRes sample entry (`apco apcs apcn apch ap4h ap4x`) currently ends in `CodecUnavailable { codec: "apch" }` (`:1749-1751`), which is the right typed answer until this lane lands.
  - Output model: `VideoFormat { width, height, bit_depth, chroma: Mono|Yuv420|Yuv422|Yuv444, color: ColorInfo }`, planes tight, 2 bytes little-endian per sample above 8 bits (`src/model.rs:288-366`). There is NO alpha plane and NO field-order member.
  - Only `hsk-studio-reel-native` depends on reel, and it constructs `VideoFormat`/`ColorInfo` and matches `ChromaFormat` exhaustively (`reel-native/src/wire.rs:13,390-393,482-485,510`). Any change to those public types therefore needs a reel-native wire change in the same sequence (one owner per crate).
  - `Limits::max_chunk_bytes` default is 8 MiB (`src/model.rs:95-105`) and bounds BOTH the compressed packet (`read_sample` `isobmff.rs:1532`) and the packed decoded frame (`video.rs:346`). 1080p 4:2:2 10-bit is 8,294,400 bytes (fits), 1080p 4:4:4:4 12-bit is 16,588,800 (does not): callers raise the limit; the decoder must still check before allocating.
- Test infrastructure to reuse: `tests/common/mov.rs` synthetic MOV builder, `tests/v4_alloc_budget.rs` counting allocator (`#[global_allocator]` at `:46`), `bounded_case` wrapper, `tests/video_route.rs` stub decoder `Fake`.

</topic>

<topic id="purpose" status="active" summary="What this lane delivers">

## Purpose
Pure-Rust, `forbid(unsafe_code)`, deterministic decoder for ProRes frames written from SMPTE RDD 36:2022, plugged into reel behind cargo feature `prores` (default OFF) and routed from MOV/MP4 sample entries `apco apcs apcn apch` (slice 1), with decoder id + version in every receipt. DNxHD/HR (ST 2019-1) is the second half of the ledger scope and is BLOCKED on spec access (Operator item O4); it is not specified here.

Why default OFF: Apple states ProRes is licensed to select companies and that unauthorized implementations "may lead to decoding errors, performance degradation, incompatibility, and instability" (see anchors). Whether a clean-room decoder may ship is an Operator decision (O1). The feature flag is the control point, the same pattern as AV1.

</topic>

<topic id="cross-validation" status="active" summary="Spec reading verified end to end by an independent implementation (research evidence)">

## Cross-validation done for this brief (so the builder starts from a verified reading)
An independent decoder was written in Python from RDD 36 text only (no ffmpeg source read) and run against ffmpeg 8.1.1 decodes of 16 generated fixtures (6 final, listed below, plus 10 earlier variants). Script: `engine-video-oracle/prores_spec_xcheck.py` (sha256 `5bfedc4fdbc3edfa93676a5ce3a7225c36f515221303566935e3b3d0480c14e9`, ~200 lines, under the artifact root on D:; it is research tooling, not product code and not to be translated line by line; the builder writes Rust from the PDF).

| Fixture (final set, `engine-video-oracle/cmd-check/final/`) | b | samples | differ by 0 | differ by 1 | max |
|---|---|---|---|---|---|
| prores-422hq-40x24-p (apch, progressive) | 10 | 7680 | 7530 | 150 | 1 |
| prores-422proxy-40x24-mbs1 (apco, 1-MB slices) | 10 | 7680 | 7509 | 171 | 1 |
| prores-422lt-40x24-tff (apcs, interlace_mode 1) | 10 | 7680 | 7497 | 183 | 1 |
| prores-422std-40x24-bff (apcn, interlace_mode 2) | 10 | 7680 | 7548 | 132 | 1 |
| prores-4444a16-40x24 (ap4h, 16-bit alpha) | 12 | 15360 (Y,Cb,Cr,A) | 13840 | 1520 | 1 |
| prores-4444a8-40x24 (ap4h, 8-bit alpha) | 12 | 15360 | 13815 | 1545 | 1 |
The ten earlier variants (72x40 testsrc2 and geq content: HQ, proxy, TFF, BFF, 4444 with 8/16-bit alpha, plus first 40x24 versions) gave max 1 except where a decoded value saturates at the external decoder's clamp floor (max 4 at b = 10 and b = 12, only at samples where the reference is exactly 4; see fact 2).

What this proves: header/picture/slice parsing, slice-size derivation (sizes [2,1] etc.), Golomb-Rice/exp-Golomb combination codes with the Table 9-11 codebook rules and initial states, inverse slice and block scan (both patterns), dequantization, the 8x8 IDCT with C(0) = 1/sqrt(2), the sample formula, macroblock block order for 4:2:2 and 4:4:4, interlaced picture placement, and scanned alpha with both code tables. Four facts the text alone does not make obvious:
1. The PDF's default text extraction drops the radical glyph (C(0) prints as "1 2") and scrambles the row order of Tables 10-11. `pdftotext -raw` recovers the tables cleanly. The 1/sqrt(2) reading is confirmed numerically, not just textually.
2. External decoder OBSERVED to clamp decoded samples at a floor of 4 for both 10-bit and 12-bit output (ref 4 where the spec default clamp gives 0). RDD 36 7.5.1 permits either clamp bound pair; Studio uses the spec default `[0, 2^b-1]`.
3. ffmpeg's ProRes encoder emits an alpha wrap-around as a small non-escape difference (+1 from 65535). The decoder must apply the alpha mask ALWAYS (RDD 36 7.1.2 note says doing so "has no effect" on conformant streams); otherwise alpha becomes 65536.
4. Alpha 16-bit to 12-bit conversion: the spec says round; ffmpeg appears to truncate (>>4). They differ by at most 1 LSB.

Oracle power: flipping each single bit of the slice data of the HQ fixture frame 0 (6,512 flips) gave 2,534 decode errors, 2,376 outputs differing by more than 1 LSB, 1,106 differing by exactly 1 and 496 identical (padding bits): 75% of flips are caught by "error or more than 1 LSB". This is why the tolerance test below also carries a mutation control.

</topic>

<topic id="spec-anchors" status="active" summary="Sources actually opened 2026-10-11">

## Spec anchors (opened this session)
- SMPTE RDD 36:2022, Apple ProRes Bitstream Syntax and Decoding Process: https://pub.smpte.org/pub/rdd36/rdd36-2022.pdf (39 pages, 412,730 bytes, sha256 `7bcef3a4981c3db6f44614f9db460b2344a7051c817202fa495167b17c87cf8a`, HTTP Last-Modified 2025-07-21). It is a Registered Disclosure Document, "NOT a Standard", and sends IP questions to the proponent (ProRes Program Office, Apple). Read order: 4 frame structure (p.6); 5.1-5.3 syntax tables (pp.9-15); 6.1-6.4 semantics (pp.15-24); 7.1 entropy decoding incl. Tables 8-14 (pp.25-31); 7.2 scanning, Figures 3-5 (pp.31-33); 7.3 quantization, Table 15 (pp.33-34); 7.4 IDCT (p.35); 7.5 sample generation (pp.35-37); Annex A IDCT qualification (pp.38-39).
- Directory https://pub.smpte.org/pub/rdd36/ lists one archive `rdd36-20220909-pub.zip` (422,320 bytes by HTTP HEAD; NOT downloaded). The size matches the PDF plus ~10 KB, so it very probably holds no sample bitstreams or reference software; the RDD text says a reference implementation and sample bitstreams were contributed, but no public location was found. UNVERIFIED.
- Apple: "Apple ProRes and ProRes RAW Authorized Products" https://support.apple.com/en-us/118584 (opened, dated 2026-08-09) and the Apple ProRes white paper, April 2022, https://www.apple.com/final-cut-pro/docs/Apple_ProRes.pdf (opened, p.4 "Authorized Apple ProRes Implementations"; licensing questions to ProRes@apple.com). Terms of any licence were not found. See O1.
- QuickTime File Format pages (opened): colour parameter atom https://developer.apple.com/documentation/quicktime-file-format/color_parameter_atom.md ; video sample description https://developer.apple.com/documentation/quicktime-file-format/video_sample_description.md (its codec table does not list ProRes FourCCs); `fiel` in https://developer.apple.com/library/archive/technotes/tn2162/_index.html .
- FourCC to profile: `apco` Proxy, `apcs` LT, `apcn` Standard, `apch` HQ, `ap4h` 4444, `ap4x` 4444 XQ. Source is third-party documentation (docs.rs oxideav-prores search result), NOT Apple. OBSERVED in ffmpeg-written files: apch (HQ), apco (Proxy), apcs (LT), apcn (Standard), ap4h (4444). `ap4x` not observed. `aprn`/`aprh` are ProRes RAW (third-party docs: no public bitstream spec) and must be refused.
- ffmpeg licence https://ffmpeg.org/legal.html (opened): LGPL v2.1+ base, GPL parts; the installed build is `--enable-gpl --enable-version3`; the project says it has "never read patents". Hence the licence rule below.
- Master spec `14-studio-creative-suite.md`: STU-VID-051 (l.24559; clause 1 says "Codec implementation is NOT reimplemented natively; it is reached through a declared adapter"), STU-VID-084 (l.25469: which codecs ship = named operator decision), STU-ARC-012 (l.219), STU-ARC-015 (l.225), STU-ARC-021 (l.251). Research `research/engine-video.md` topics `specs-and-open-engines`, `improvements` (item 1: deterministic decode contract; ProRes/DNx IDCT not normative, pin one reference).

</topic>

<topic id="slices" status="active" summary="Minimal first slice and later slices">

## Slices
**Slice 1 (minimal vertical slice): progressive 4:2:2 YCbCr, profiles Proxy/LT/Standard/HQ, 10-bit output.**
1. Module `src/prores/` behind `prores = []` (default off), `#![forbid(unsafe_code)]` inside the module even though the crate allows `deny` under `av1`: `mod.rs` (public surface, session), `header.rs` (frame, picture and slice headers; semantics from 6.1-6.3), `bits.rs` (MSB-first reader, checked), `entropy.rs` (Golomb combination codes, Tables 9-11, `endOfData` rule), `scan.rs` (Figures 4/5 as `const` tables), `idct.rs` (reference IDCT), `output.rs` (sample formula, placement into the cropped output).
2. `VideoCodec::ProRes` (enum is `#[non_exhaustive]`, additive). `choose()` gets a built-in arm under `#[cfg(feature = "prores")]` that does not depend on `SourceTrust` (pure safe Rust with the hostile suite below; if the lead rejects trust-independence, route ProRes through an external worker exactly as AV1 and say so here).
3. `IsobmffProvider::deliver`: sample entries `apco apcs apcn apch` -> `VideoCodec::ProRes`; `ap4h`/`ap4x` -> `Unsupported { what: "prores-4444-slice-3" }` until slice 3; `aprn`/`aprh` -> `Unsupported { what: "prores-raw" }`. Feature off -> unchanged `CodecUnavailable { codec: "apch" }`.
4. Frame -> `DecodedPicture { format: VideoFormat { bit_depth: 10, chroma: Yuv422, color: ColorInfo { .., full_range: false, origin } } }`. Colour mapping: header codes 0 and 2 mean unspecified (H.273 value 2); otherwise copy primaries/transfer/matrix with `origin: SequenceHeader`; when unspecified use the container `colr` from `VideoStreamSpec.color` with `origin: Container`; else `Untagged`. Range is limited (7.5.1 levels 16..235 scaled by 2^(b-8)); the bitstream has no range flag.
5. Enumerability (STU-VID-051 clause 2): the codec is a video route, not a provider id, so mirror `av1_descriptor` (`src/av1_config.rs:9-21`): `pub fn prores_descriptor(decoder_version: &'static str, can_decode: bool) -> CodecDescriptor` (`id: "prores"`, `container: "isobmff-mov-mp4"`, `bit_depths: &[10, 12]`, `chroma: Some("422")` in slice 1, `can_encode: false`), and mention `prores` in `DESCRIPTOR` with `operator_decision_ref` "START_LOG O1/O3 pending". `Registry::standard()` is unchanged.
6. Bit depth: RDD 36 has NO bit-depth field. Studio picks `b` per profile family (10 for 4:2:2 profiles, 12 for 4444 profiles) as `assumed(rule)`; `ProResOptions::output_bit_depth` may override (10, 12 or 16). Recorded here as a documented assumption, not a spec fact.

**Slice 2: interlaced 4:2:2.** Two pictures per frame, scan pattern of Figure 5 when `interlace_mode != 0`, `picture_vertical_size` rule of 6.2, first picture temporally first, rows interleaved into the frame (7.5.3). No model change: the output is a full interleaved frame; field order stays in the frame header and the container `fiel` (lane ENG-1). Fixtures: `prores-422lt-40x24-tff`, `prores-422std-40x24-bff`; assert `fiel` = (2,9) and (2,14) in the container test.

**Slice 3: 4:4:4 and alpha (needs a coordinated model change).** Alpha needs a fourth plane and `ChromaFormat`/`VideoFormat` have none; interlaced output also lacks a field-order member. Options for the lead (decide before the slice): additive `#[non_exhaustive]` newtype `FrameExtras { alpha_plane: bool, field_order: Option<..> }` carried in the receipt, or new `ChromaFormat` variants. Either touches `reel-native/src/wire.rs`; schedule as one sequenced change with the reel-native owner. Decoder rules already verified: 4:4:4 chroma block order is column-major (0 2 / 1 3) unlike luma (0 1 / 2 3); alpha always masked; alpha to depth `b` by the spec's rounding formula.

**Slice 4: performance and fixed-point IDCT.** Slice-parallel decode (`std::thread::scope`), a fixed-point or f32 IDCT that must pass the Annex A procedure against the f64 reference; any change that can alter one output bit bumps `PRORES_DECODER_VERSION`.

**Slice 5: DNxHD/HR** after O4.

</topic>

<topic id="api" status="active" summary="Public API sketch and typed errors">

## API sketch (names are proposals; keep inside `hsk_studio_reel`, re-export from `lib.rs`)
```rust
// src/prores/mod.rs, feature "prores"
pub const PRORES_DECODER_ID: &str = "prores-rdd36";
pub const PRORES_DECODER_VERSION: &str = "1";   // bump whenever any output bit can change
pub enum ProResProfile { Proxy, Lt, Standard, Hq, P4444, P4444Xq }
impl ProResProfile { pub fn from_fourcc(code: [u8; 4]) -> Option<Self>; pub fn default_bit_depth(self) -> u8 }
pub enum ClampRange { Full /* 0..=2^b-1, spec default */, VideoLegal }
pub struct ProResOptions { pub output_bit_depth: Option<u8>, pub clamp: ClampRange }
pub struct FrameHeader { /* every field of 5.1.1 incl. both 64-byte matrices; reserved bits ignored (6.1.1) */ }
pub fn parse_frame_header(frame: &[u8], declared: (u32, u32), limits: &Limits) -> Result<FrameHeader, ReelError>;
pub struct ProResSession { /* implements VideoDecodeSession; one packet = one 'icpf' frame incl. its 4-byte size */ }
pub fn open_session(spec: &VideoStreamSpec<'_>, profile: ProResProfile, opts: ProResOptions, limits: &Limits)
    -> Result<ProResSession, ReelError>;
```
Errors (no new variants; stable `what` strings): `Corrupt { "prores-frame-identifier" | "prores-frame-size" | "prores-header-size" | "prores-picture-size" | "prores-slice-table" | "prores-slice-size" | "prores-quantization-index" | "prores-entropy" | "prores-version-constraint" }`; `Unsupported { "prores-bitstream-version" (>= 2, 6.1.1 says abort) | "prores-chroma-format" | "prores-interlace-mode" | "prores-raw" }`; `LimitExceeded { "prores-frame-bytes" | "prores-slices" | "prores-golomb" }`; size mismatch against the container-declared size -> `Corrupt { "prores-size-mismatch" }`. Reserved header bits are ignored; a version-0 frame that declares 4:4:4 or alpha violates 6.1.1 and is `Corrupt { "prores-version-constraint" }` (strict, decided here).

</topic>

<topic id="hostile" status="active" summary="V4-01/02 bounds">

## Hostile-input bounds
- `#![deny(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]` on `header.rs`, `bits.rs`, `entropy.rs`, `output.rs`; `idct.rs`/`scan.rs` index fixed `[[_; 8]; 8]` arrays by loop counters and may `allow(indexing_slicing)` with a one-line reason plus an exhaustive test. No panics by construction: the host may build with `panic = "abort"`.
- Before ANY allocation: header width/height <= container-declared size (0 = undeclared -> cap 16384 per side), computed output bytes (checked) <= `limits.max_chunk_bytes`, slice table bytes (`2 * height_in_mb * slices_per_row`) <= bytes remaining in the picture, every `coded_size_of_slice` <= remaining bytes, running sum of slice sizes <= `picture_size - header - table` (aggregate budget, V4-02). Declared counts never size a `Vec` without byte backing.
- Write straight into the cropped output; no padded intermediate plane. Per-slice scratch is bounded by 8 macroblocks x 8 blocks x 64 values.
- Entropy: Golomb prefix capped (> 31 zero bits -> `LimitExceeded "prores-golomb"`); `run` and level accumulation checked against `numBlocks * 64` (overflow -> `Corrupt "prores-entropy"`); dequantization in `i64`, saturated, with a per-session counter of clamped coefficients; quantization_index outside 1..=224 -> `Corrupt` (reserved per 6.3.1). Alpha (slice 3): run <= 2048 by construction, total <= `numAlphaValues`.
- Cancellation: `CancellationToken` checked per slice.
- Tests named in V4 style: `prores_dims_u16_max` (header 65535 x 65535 against declared 0 and against a small declared size -> typed error with peak allocation below one frame) and `prores_aggregate_over_budget` (slice sizes whose sum exceeds the picture; `picture_size` larger than the packet; slice table larger than the picture).

</topic>

<topic id="determinism" status="active" summary="Determinism contract and tolerance rule">

## Determinism contract
1. Exact stages (no tolerance): header, entropy decode, inverse scan, dequantization are integer arithmetic. Bit-exact by construction.
2. IDCT is NOT normative: RDD 36 7.4 accepts any implementation that passes Annex A. Studio's normative reference is `idct_f64_reference`: separable 8x8 f64, row pass then column pass, summation in ascending index order, plain `a * b + c` (never `mul_add`), cosine values from a `const` 8x8 table with 17 significant digits (a test asserts `|table - f64::cos| <= 1e-15`; `f64::cos` is never called at runtime because libm results can differ by host), C(0) = 1/sqrt(2). Sample generation: `s = clamp(floor(2^b * (v + 256) / 512 + 0.5))`, ties up (3.6 allows either tie rule; this pins it), clamp `[0, 2^b - 1]` by default. Result: output is bit-identical on every IEEE-754 binary64 host, independent of thread count. Studio digests (SHA-256 of each packed frame) are pinned only AFTER the oracle test is green (a pin taken earlier would freeze a bug); they prove cross-host determinism, not correctness.
3. Tolerance rule against any OTHER decoder (ffmpeg, Apple, Adobe): `TOL(b) = ceil(0.30 * 2^b / 512)` LSB = 1 at b = 8 and 10, 3 at b = 12. Derivation: Annex A criterion (1) bounds peak IDCT error at 0.15 in the 9-bit signed domain (one unit of v is 2^b/512 LSB); two conforming implementations are therefore at most 0.30 apart before rounding. This is a derived bound, UNVERIFIED as universal for arbitrary data. Measured: max 1 LSB at both b = 10 and b = 12 on all fixtures, so the test asserts `<= TOL(b)` AND ratchets `observed_max <= 1`; exceeding 1 at b = 10 needs a diagnostics note, not a quiet widening.
4. Clamp-bound exclusion: for real-world oracle media, exclude samples whose reference lies within 8 of 0 or 2^b - 1 and print the excluded count (the external decoder floors at 4, Studio at 0). The committed fixtures keep decoded values in [80, 941] by construction (geq clip), so no exclusion occurs there.
5. Alpha tolerance 1 LSB (rounding vs truncation); alpha is always masked.
6. Annex A exact qualification (10,000-block data sets, IEEE 1180 random generator) is only required when a non-reference IDCT exists (slice 4). The generator in IEEE Std 1180-1990 was not opened: NOT_PROVEN until then.

</topic>

<topic id="tests" status="active" summary="At most 6 focused tests, fixtures, exact commands">

## Tests (at most 6; `bounded_case`; fixtures committed per `tests/fixtures/README.md` convention; no test runs ffmpeg)
1. `prores_header_and_slice_table_exact`: parse frame 0 of the HQ fixture: `frame_header_size 148`, `bitstream_version 0`, encoder id `apl0`, 40x24, `chroma_format 2`, `interlace_mode 0`, both matrices loaded (luma values within 4..7), `picture_header_size 8`, `picture_size 830`, `log2_desired_slice_size_in_mb 3`, `width_in_mb 3`, `height_in_mb 2`, `slice_size_in_mb [2, 1]`, 4 slices, frame 0 length 986 (frame sizes 986, 994, 987, 998). Proxy fixture: `log2 0`, 6 slices, picture size 236, frame 0 length 392. Values come from a hexdump of the generated files, not from the decoder under test.
2. `prores_entropy_and_flat_blocks_hand_derived`: a test-only bit writer (in `tests/common/prores.rs`, written from the codebook rules, so it can only prove self-consistency) builds slices of known coefficients: exact coefficient arrays after decode. Hand-derived pixels: 16x16 frame, one macroblock, qIndex 1 (qScale 1), default matrix W = 4: luma DC quantized value 64 gives F = 64*4*1/8 = 32, v = F/8 = 4, `s = floor(1024*(4+256)/512 + 0.5) = 520`; chroma DC 0 gives 512; DC 1 gives 512 (v = 0.0625), DC 16 gives 514. Also derived: rewriting the HQ header to "no matrices loaded" must decode identically to the same header with an explicit all-4 luma matrix and all-4 chroma matrix (tests the default-matrix and chroma-falls-back-to-luma rules; header rewrite is done in the test).
3. `prores_decode_matches_external_oracle_within_tolerance`: for each committed fixture + reference, max |delta| over all planes and frames `<= TOL(b)` and `<= 1` ratchet, and the number of differing samples is printed. Mutation control in the same test: single-bit flips of slice data (fixed list chosen at authoring time) must yield either a typed error or a max delta above TOL for at least 60% of 256 sampled flips (measured 75% on the Python reference), so the oracle cannot go green on a decoder that ignores coefficients.
4. `prores_hostile_frames`: `prores_dims_u16_max`, `prores_aggregate_over_budget`, every single-bit flip of one small frame and every truncation length -> `Ok` or typed `Err`, never a panic, peak allocation <= one output frame + 64 KiB (reuse `v4_alloc_budget` tracker), `bitstream_version = 2`, `frame_identifier` wrong, `quantization_index = 0` and `225`, Golomb overlong prefix.
5. `prores_container_route_and_receipts`: MOV with `apch` through `Registry::decode_segment` (`OutputMode::Decoded`): 4 `VideoFrame` chunks, `pts` exactly `n * 10_160_640_000` ticks at 25 fps (ticks per frame from `research/engine-video.md`), `receipt.decoder_id == "prores-rdd36"`, `decoder_version == "1"`, `receipt.video.bit_depth == 10`, `chroma == Yuv422`, `full_range == false`; feature off -> `CodecUnavailable { codec: "apch" }`; `aprn` -> `Unsupported "prores-raw"`; `Purpose::Export` + unaligned start -> `NotExact` (needs ENG-1); `SourceTrust::Untrusted` still decodes (trust-independent route).
6. `prores_interlaced_and_alpha` (slices 2-3; `#[ignore = "slice 3"]` for alpha until the model change, never a silent pass, V4-03): TFF/BFF fixtures match the oracle; container `fiel` is (2,9) / (2,14).

**Fixtures** (authored once under `D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/engine-video-oracle/`; the dispatch's path without "Handshake Worktrees" does not exist; a verified copy of the final files is in `cmd-check/final/`). All commands below were run and re-checked with the Python cross-check; ffmpeg 8.1.1 gyan.dev full build; output byte-reproducible (regeneration gave identical sha256 with `-fflags +bitexact -flags:v +bitexact`). Source, all 4 frames, 25 fps, 40x24 (width_in_mb 3, height_in_mb 2: both dimensions need padding discarded):
`G422="color=c=gray:size=40x24:rate=25,format=yuv422p10le,geq=lum='clip(80+X*9+Y*5+N*37+8*sin(X*Y/9+N),64,940)':cb='clip(512+2*X-3*N,64,960)':cr='clip(512-2*Y+5*N,64,960)'"`
- HQ progressive: `ffmpeg -y -f lavfi -i "$G422" -frames:v 4 -c:v prores_ks -profile:v 3 -pix_fmt yuv422p10le -mbs_per_slice 8 -fflags +bitexact -vendor apl0 -flags:v +bitexact prores-422hq-40x24-p.mov` (4690 B, sha256 `f2404325141539ae35bac5795a07275dbf355a7f17953e205ce3cf7108793e75`)
- Proxy, 1-MB slices: `... -profile:v 0 -mbs_per_slice 1 ... prores-422proxy-40x24-mbs1.mov` (2298 B, `26ce094602ee7dade16a49ce7fa8ddeb4438be8ddbddd4a1a56f806a40594df1`)
- LT interlaced TFF: `ffmpeg -y -f lavfi -i "$G422" -vf setfield=mode=tff -frames:v 4 -c:v prores_ks -profile:v 1 -pix_fmt yuv422p10le -mbs_per_slice 4 -fflags +bitexact -vendor apl0 -flags:v +bitexact+ildct prores-422lt-40x24-tff.mov` (3576 B, `657be21dbc17633c4c8ff30bf0f2ff60525934f5e8f69eaa755c18ea8bf480ac`). NOTE: a separate `-flags +ildct` before `-flags:v +bitexact` is overridden and silently yields a progressive file; put both in the one `-flags:v` value as shown.
- Standard interlaced BFF: same with `setfield=mode=bff -profile:v 2` -> `prores-422std-40x24-bff.mov` (4108 B, `08577df402babe2902a714d83658f93e996ddfde4a71ea5b1ac5a13a7e920382`)
- 4444 + alpha (slice 3): source `G444A` = the same expression on `format=yuva444p10le` plus `:a='clip(3*X*Y+97*N,0,1023)'`; `-profile:v 4 -pix_fmt yuva444p10le -alpha_bits 16|8 -mbs_per_slice 2` -> `prores-4444a16-40x24.mov` (11944 B, `9f27d078b3cb9d2f1e1bf45d7a171b9d0417049f8c06ade771e5b349ca6df40b`), `prores-4444a8-40x24.mov` (8133 B, `5799373f00f57315a9acfae0c2d405f46d97a5e8d61f609bb2e6b3ca3a4fc9c9`).
- References (external-decoder output, committed as data): `ffmpeg -y -i FILE.mov -f rawvideo FILE.ref.yuv` (native pixel format: 10-bit 4:2:2 planar little endian, 15,360 B each; 4444: `yuva444p12le`, four planes, 30,720 B each). Also `ffmpeg -y -i FILE.mov -c:v copy -f rawvideo FILE.pk` concatenates the raw frames (each starts with its `frame_size`), useful for the hexdump in test 1. Total committed size is about 140 KB, each file below the 200 KB convention.
- Provenance: `tests/fixtures/prores/provenance.json` with the generator lines, ffmpeg version string, sha256 of every file, and the line "external decoder used as black-box oracle only".
- Frame header cross-checks that were run against real bytes: header offsets and bit widths of 5.1.1 (frame_header_size 148 = 20 + two 64-byte matrices at offset 8; chroma_format = top 2 bits of byte 20; interlace_mode = bits 3-2; `alpha_channel_type` = low nibble of byte 25), TFF header byte `0x84`, BFF `0x88`, 4444 `0xC0`, version 1 for 4444.

Cargo form: the studio form from `builders/WORKSPACE_RULES.md`, `-p hsk-studio-reel --features prores --tests` for check/clippy/test, and the default build (feature off) must still compile and pass.

</topic>

<topic id="deps" status="active" summary="Allowed dependencies and licence rule">

## Allowed dependencies
None new. The only `Cargo.toml` change is the feature line `prores = []`. `sha2 =0.10.9` (already a dependency, force-soft; licence `MIT OR Apache-2.0` per the local registry copy) is used by tests for frame digests. Rejected: `oxideav-prores 0.1.1` (MIT, pure Rust, floating-point IDCT; depends on `oxideav-core`, clean-room provenance not stated, found via docs.rs only, not read); Craft `prores` crate (MIT OR Apache-2.0, case study only); ffmpeg `libavcodec` (LGPL/GPL).

LICENCE RULE: do not read, copy or translate ffmpeg source (`proresdec*.c`, `prores_ks`, `proresenc*`), the Craft `prores` crate, `oxideav-prores`, or any other ProRes implementation. Implementation sources are RDD 36 and this brief only; ffmpeg binaries are external black-box tools whose OUTPUT is data. Record that in `src/prores/SOURCE_PROVENANCE.json` (RDD 36 sha256 above, the Apple pages, "no third-party codec source consulted"). Each table and rule in the code cites its RDD 36 section.

</topic>

<topic id="redteam" status="active" summary="Risks, failure scenarios, minimum controls">

## Red team (controls are enforced by acceptance criteria)
| Risk / plausible failure | Minimum control |
|---|---|
| R1 licensing or trademark exposure from shipping an "unauthorized implementation" (Apple wording) | feature `prores` default off, no encoder in scope, descriptor carries the pending decision, Operator items O1/O3; naming uses FourCCs and `prores` descriptively |
| R2 clean-room contamination (builder consults ffmpeg or another decoder) | provenance record + section-cited code; brief forbids those sources; reviewer checks `SOURCE_PROVENANCE.json` |
| R3 symmetric misreading (test encoder and decoder share the same wrong reading) | external-oracle test 3 uses bytes written by ffmpeg's independent encoder; test 2 is labelled self-consistency only |
| R4 tolerance hides a real bug | mutation control in test 3 + the `observed_max <= 1` ratchet |
| R5 DoS or panic from hostile frames | topic hostile; tests 4; no `unwrap/expect/panic/indexing` in parsing modules; counting allocator |
| R6 alpha wrap quirk or clamp floor produce 1-4 LSB or 65536-alpha surprises on real files | always-mask rule, clamp bounds recorded, exclusion band for real oracle media |
| R7 bit depth assumed wrongly (header has none) | documented `assumed(rule)`, override option, 12-bit only for 4444 until an Adobe/Apple-written file says otherwise (D5) |
| R8 public type change breaks reel-native | slice 3 gated on a coordinated plan; slices 1-2 add no public field to `VideoFormat`/`ChromaFormat`/`SegmentReceipt` |
| R9 f64 IDCT too slow for 1080p+ | explicit non-goal for slice 1; slice 4 (parallel + fixed point) with Annex A; no performance claim in docs without a measurement (V4-07) |
| R10 FourCC and frame header disagree (sample entry `apch`, frame says 4:4:4 or alpha) | frame header wins for decoding; FourCC only selects the default bit depth; test 5 includes a mismatch case |
| R11 odd-height interlaced frames | follows 6.2 text; fixtures only cover even heights: stays UNVERIFIED, listed below |

</topic>

<topic id="acceptance" status="active" summary="Acceptance criteria">

## Acceptance
1. Slice 1 delivered: tests 1-5 green with `--features prores`; whole reel suite green with and without the feature and with `av1`; `cargo check -p hsk-studio-reel-native --features av1,av1-worker --tests` still compiles (guards R8).
2. No `unsafe` in `src/prores/**`; the default build contains no ProRes code path; no new dependency.
3. Hostile suite from topic hostile present and green; no `unwrap/expect/panic/indexing` lint exceptions outside `idct.rs`/`scan.rs`.
4. Determinism: two decodes of the same bytes identical; oracle comparison within `TOL(b)` and ratchet 1; Studio digests pinned only after that; `PRORES_DECODER_VERSION` documented as the bump rule.
5. `SOURCE_PROVENANCE.json`, fixtures, `provenance.json` committed; descriptor/`DESCRIPTOR` updated; READY_FOR_VALIDATION only.
6. "Decoder id and version in every frame receipt": satisfied through `SegmentReceipt.decoder_id/decoder_version` (one receipt per segment, every frame belongs to one segment). If the lead requires a per-`Chunk` field, that is a public-type change (UNVERIFIED requirement reading, lead decision).

</topic>

<topic id="unverified" status="active" summary="UNVERIFIED and Operator items">

## UNVERIFIED
- Apple's licence terms, whether a decoder written from RDD 36 needs a licence, patent coverage, and rules for using the "ProRes" name. Only Apple's public statements were read.
- Contents of `rdd36-20220909-pub.zip` and any public sample bitstreams or reference implementation (not downloaded).
- FourCC to profile mapping per Apple (observed on ffmpeg-written files only; `ap4x` unobserved).
- Behaviour on files written by Apple or Adobe encoders: only ffmpeg's encoder (versions 0 and 1) was exercised; D5 session tier.
- Decoder clamp bounds used by Apple/Adobe; profile-to-bit-depth convention.
- ProRes RAW has no public bitstream spec (third-party crate documentation claim).
- Odd picture heights in interlaced frames; widths or heights above the fixtures (only 40x24 and 72x40 were run).
- IEEE 1180-1990 random number generator (Annex A); `TOL(b)` as a universal bound.
- Performance of the f64 IDCT (not measured).
- STU-VID-051 clause 1 vs Operator instruction (see O2).

## Operator items (not decided here)
- O1 Apple ProRes licensing and trademark: Apple says ProRes is licensed to select companies and warns about unauthorized implementations (https://support.apple.com/en-us/118584 ; white paper p.4). RDD 36 itself says IP questions go to Apple (ProRes@apple.com). Decide whether Studio may ship a clean-room decoder, under which naming, and whether to contact Apple. Until then: feature default off.
- O2 Spec conflict: STU-VID-051 clause 1 says codecs are "NOT reimplemented natively" (reached through adapters); the Operator's 2026-10-11 instruction is to rewrite concepts in Rust. The decoder keeps the adapter shape (descriptor, typed unavailability) but contradicts the sentence; spec enrichment is Operator-only (CX-105).
- O3 STU-VID-084: which codecs ship is a named operator decision; covers ProRes and DNx.
- O4 DNx (ST 2019-1): `pub.smpte.org` lists `st2019-1-20160518-pub.zip` (2,299,023 bytes by HTTP HEAD, last modified 2024-11-21; also an amendment page 2022-09-28) but whether the archive holds the full text, and its terms, are UNVERIFIED; GlobalSpec lists the 2014 and 2016 editions as "Buy Now" with no price seen. Approve (or not) downloading that one file into a fresh empty folder on D: for inspection; no download was made.
- O5 FYI: committed fixtures are synthetic bitstreams written by an LGPL/GPL tool from synthetic input; no third-party media. Flagged for awareness only.

</topic>
