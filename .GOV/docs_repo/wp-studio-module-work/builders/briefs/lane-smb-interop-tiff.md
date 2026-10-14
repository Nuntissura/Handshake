---
file_id: BUILDER-BRIEF-LANE-SMB-INTEROP-TIFF
file_kind: builder_brief
updated_at: 2026-10-14T15:50:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-tiff (NEW crate; module-map change, needs lead/Operator decision F1 before the steward scaffolds it)
lane: SMB-INTEROP-TIFF
depends_on: [decision F1, steward scaffold + registry pin tiff =0.11.3, lane SMB-CONVERT-NATIVE CN-1 for the Folio mapping (slice 2 only)]
author_role: format-coverage scoping researcher (SMB-FORMAT-SCOPE); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 45a3792; tiff 0.11.3 source read in the local cargo registry
---

<topic id="finding" status="active" summary="What exists for TIFF today">

## Finding (verified in code at 45a3792)
- No crate imports or exports general TIFF. The only TIFF reader is `crates/hsk-studio-develop/src/tiff.rs`, which is a private module (`mod tiff;`, `src/lib.rs:23`) used only by the DNG decoder (`src/dng.rs:18`).
- That reader handles classic TIFF IFDs and SubIFDs (tag 330, depth 3). It refuses BigTIFF (`tiff.rs:59`). It returns raw tag values and strip/tile byte ranges (`region`, `tiff.rs:256`). It has no decompression. The DNG path accepts only Compression 1 and 7 (`dng.rs:406-414`).
- `tiff.rs:101` and `tiff.rs:109` compute `base + 2 + i * 12` without checked arithmetic. dng.rs denies `clippy::arithmetic_side_effects`, but tiff.rs does not. Both lines are bounded by `MAX_ENTRIES_PER_IFD`, but a new owner must not inherit this.
- The lock has no `tiff`, `weezl` or `quick-error`. It already has `flate2 1.1.10`, `miniz_oxide 0.9.1`, `zune-jpeg 0.5.15` and `half 2.7.1` (Cargo.lock).
- What the spec asks for:
  - Row 3 of the STU-IO-006 matrix: `.tif/.tiff` with direction I·EP·X·RT, "Multi-layer (where present), alpha, high bit-depth, embedded ICC preserved; per-format compression options exposed" (spec v02.216 module 14, l.21319).
  - Export domains (STU-IO-136, l.21061-21068): TIFF compression `None`, `LZW`, `ZIP`; bit depth 8/10/16/32.

</topic>

<topic id="ownership" status="active" summary="CX-CAP-001 owner proposal for TIFF container parsing (decision F1)">

## Ownership proposal (decision F1; module-map change, Operator/lead)
**Recommended:** a new crate `hsk-studio-interop-tiff`. It is FOLIO-FREE: its Studio edges are `accord` and `observe` only. Its only crates.io dependency is `tiff` (image-rs, MIT).
- It is the single Studio owner of TIFF container parsing, decoding and encoding: IFD chain, SubIFDs, BigTIFF, strips and tiles, decompression, predictors, sample formats, ICC (tag 34675), ExtraSamples, multi-page, and raw access to Photoshop tags 37724 (ImageSourceData) and 34377 (ImageResources).
- Folio-free matters because develop can then consume it. Develop's IFD walk moves onto this crate in slice 3, and `develop/src/tiff.rs` is deleted.
- Develop must not gain a Folio path: STU-CON-045 says "Develop MUST NOT ... introduce a forbidden direct Folio edge" (l.22600). A Folio-mapping interop crate below develop would bring Folio in transitively.
- Mapping a decoded TIFF into a `StudioDocument` is NOT done here. The convert flow does it (SMB-CONVERT-NATIVE, raster proposal CN-1), so the tile/TileRef builder has one owner.
- Layer records inside tag 37724 are parsed by interop-psd, which owns PSD layer-record parsing, through a new public entry point. This crate hands over the raw block bytes and their byte order. No interop-to-interop edge is needed.

Rejected options:
- **(b) Move `develop/src/tiff.rs` into a Folio-mapping interop crate and add a develop->interop edge.** Develop would inherit Folio (STU-CON-045), and an engine would sit above an interop crate.
- **(c) Keep TIFF inside develop and expose it as public.** Develop's responsibility is "RAW decode/develop recipes" (STUDIO_MODULES), not the general raster interchange of row 3.
- **(d) Put it in reel as the interim still-image codec owner (START_LOG decision 01:50Z, D6).** Develop -> reel would pull the media crate into the RAW closure, and D6 is still open. If the Operator decides D6 differently, this crate is the TIFF part of that answer.

Until slice 3 lands, develop's private reader is recorded CX-CAP-001 debt: it is private and has no other consumer, and nobody new may consume it.

</topic>

<topic id="scope" status="active" summary="Minimal slices">

## Slice 1 (this lane, minimal): TIFF codec API over `&[u8]` (no Folio)
1. **Decode.** `decode(bytes, &Limits, &CancellationToken) -> Result<TiffFile, TiffError>`.
   - Uses `tiff::decoder::Decoder` over `std::io::Cursor`. Set `tiff::decoder::Limits` from this crate's `Limits`; never use `Limits::unlimited()`.
   - `TiffFile { pages: Vec<TiffPage> }`: every IFD in the top chain, bounded count.
   - `TiffPage` carries: width, height, photometric, bits per sample, sample format (uint/int/float), samples per pixel, planar flag, `alpha: None|Associated|Unassociated` (ExtraSamples, tag 338), ICC bytes (tag 34675), resolution, orientation (tag 274; recorded, not applied), compression id, and the raw bytes of tags 37724 and 34377 with the file byte order.
   - `TiffPage::pixels()` returns interleaved samples (`U8 | U16 | U32 | F32 | F64`) in file sample order. Gray, gray+alpha, RGB, RGBA, CMYK, CMYKA and YCbCr-in-JPEG come through as the crate decodes them.
2. **Typed refusals** (stable snake_case codes, never defaulted):
   - CIELab (PI 8) and ITU Lab (PI 10). The `tiff` crate refuses both (`decoder/image.rs:498-514`). PI 9 ICCLab is supported.
   - Palette (PI 3). The crate refuses it; slice 2 expands it from the ColorMap tag.
   - LogLuv, Fax3/Fax4 bilevel (feature `fax` stays off in slice 1), ZSTD and WebP (C or off features), and heterogeneous bit depths.
   - Every unsupported compression id refuses as `compression_unsupported` with the id.
3. **Encode.** `encode(&EncodeRequest) -> Result<Vec<u8>, TiffError>`.
   - The request carries width, height, `U8 | U16 | F32` interleaved samples, colour `Gray | GrayA | Rgb | Rgba | Cmyk`, alpha kind, ICC bytes, compression `None | Lzw | Deflate | PackBits`, and an optional horizontal predictor (integer samples only; the crate refuses predictor 3 on encode).
   - `Deflate` is the ZIP member of STU-IO-136. Alpha is declared through `ImageEncoder::extra_samples` (`encoder/mod.rs:718`), ICC through `write_tag(Tag::Unknown(34675))`, and BigTIFF automatically above 4 GiB or by request.
   - 10-bit output from STU-IO-136 is a typed refusal in slice 1.
4. **Round trip.** `encode(decode(x))` keeps samples, ICC bytes and alpha kind exactly for every supported layout (the bounded fidelity envelope of STU-IO-002/004c).
5. **Rules.** `#![forbid(unsafe_code)]` and `#![deny(clippy::arithmetic_side_effects)]` in every module (V4-01). Checked declared-size versus remaining-bytes before any allocation (V4-02). One terminal Observe frame per decode/encode. A `DESCRIPTOR` string lists formats, compressions, refusals and limits.

## Slice 2 (follow-up, same crate)
- Palette expansion and CIELab PI 8 through the crate's raw chunk API (`Decoder::read_chunk_bytes`, `decoder/mod.rs:1266`) plus own unpacking. It is UNVERIFIED whether `read_chunk_bytes` works for PI 8 before `colortype()` refuses: probe that first.
- `ImageSourceData` handoff: validate the signature `Adobe Photoshop Document Data Block\0` and the 4-byte key that follows it (`8BIM`/`MIB8`/`8B64`/`46B8` = BE32/LE32/BE64/LE64; psdtags BSD-3 behaviour reference, https://raw.githubusercontent.com/cgohlke/psdtags/master/psdtags/psdtags.py, `TiffImageSourceData.read`). Return `(byte_order, size_width, block_bytes)` for interop-psd.
- Layered export (writing 37724) is the mirror of that and belongs to interop-psd's writer side.
- The Folio mapping happens through SMB-CONVERT-NATIVE.

## Slice 3 (develop consolidation; executed by the develop owner, sequenced after slice 1 PASS)
- Develop gains the edge `develop -> interop-tiff`.
- `dng.rs` reads IFDs and tags through this crate's IFD API (it needs SubIFD walk, raw tag bytes and byte order) and keeps its own `ljpeg.rs`.
- `src/tiff.rs` is deleted.
- Gate: `tests/develop_dng.rs` and the ignored `tests/real_dng.rs` stay green on the three corpus DNGs before the deletion commit.

</topic>

<topic id="deps" status="active" summary="Dependency and edge requests (steward)">

## Requests
- **New crate and root member:** `hsk-studio-interop-tiff`. Allowed edges: `accord` and `observe` (default for every crate). Decision F1 is needed first. Record the module-map drift the same way as D9/D16.
- **Builder `Cargo.toml`:** `tiff = { version = "=0.11.3", default-features = false, features = ["deflate", "lzw", "jpeg"] }`.
  - Facts read from the local registry source:
    - tiff 0.11.3: MIT, rust-version 1.85, published before 2026-04-17 (Fedora EPEL package date).
    - Default features are `deflate`, `fax`, `jpeg` and `lzw`. `zstd` (C) and `webp` are off.
    - `unsafe` appears only in `bytecast.rs:23,27`.
  - New lock packages:
    - `weezl 0.1.12` (MIT OR Apache-2.0)
    - `quick-error 2.0.1` (MIT/Apache-2.0)
  - Reused lock packages:
    - `flate2 1.1.10`, default `rust_backend` = miniz_oxide
    - `zune-jpeg 0.5.15` (MIT OR Apache-2.0 OR Zlib)
    - `half 2.7.1`
  - `fax 0.2.7` (MIT) is not pulled while the `fax` feature is off.
  - Every licence is in the `license_gate.py` OK set.
- **dep-requests.jsonl line:** the steward appends it on scaffold. This brief does not edit `builders/dep-requests.jsonl`, which has uncommitted changes from another writer.
- **Slice 3 edge:** `develop -> interop-tiff` (`steward-sync.sh --add-edge hsk-studio-develop=hsk-studio-interop-tiff`). It needs the F1 decision.

</topic>

<topic id="spec-basis" status="active" summary="Clauses">

## spec_basis
- **STU-IO-006 row 3** (l.21319): ".tif/.tiff ... I·EP·X·RT | Multi-layer (where present), alpha, high bit-depth, embedded ICC preserved; per-format compression options exposed".
- **STU-IO-136** (l.21061-21068): "TIFF compression | None, LZW, ZIP"; "Bit depth | 8, 10, 16, 32".
- **STU-IO-004** (l.21294): the five-artifact gate. Each must exist before anything is called parity-complete:
  - (a) scope: the descriptor;
  - (b) fixtures: the corpus below;
  - (c) round-trip envelope: test 4;
  - (d) receipts: typed refusals;
  - (e) recovery: truncation and hostile-size tests.
- **STU-RAW-003 provenance paragraph** (l.2096-2100): TIFF appears "only as interop provenance". User-facing command and manual names must name the capability, not the format owner. No canonical command id is verified, so naming is a lead decision.
- **Operator instruction 2026-10-14** (START_LOG): "PSD, TIFF and RAW must stay usable: open directly or convert into the native format."

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (slice 1)
At most 6 focused tests. Required tests build their bytes in the test, so they never read machine-local paths (V4-03).
1. **Hand-built bytes.** Hand-assemble a little-endian and a big-endian 3x2 RGB 8-bit strip TIFF IFD in the test (no encoder). `decode` must return the exact samples, `alpha None` and the ICC bytes.
2. **Compressions.** For each of `None`, `Lzw`, `Deflate`, `PackBits`, at 8-bit and 16-bit RGB and Gray, encode then decode with a horizontal predictor on and off. Samples must be byte-identical. Encoding f32 RGB with a predictor gives a typed `predictor_unsupported`.
3. **Alpha and ICC.** An RGBA 16-bit image with Unassociated alpha and a 3144-byte ICC payload (any profile bytes generated in the test) round-trips. On re-read, the ExtraSamples value is 2 and the ICC bytes are equal.
4. **Hostile input.**
   - A declared width x height x spp whose product overflows refuses before allocation (`dims_u32_max`).
   - An IFD entry count of 65535 and an IFD cycle refuse with typed codes.
   - A strip offset past the end refuses with `truncated`.
   - None of these may panic. Run under the workspace per-test timeout.
5. **Typed refusals.** A PI 8 CIELab page, a PI 3 palette page and Compression 50000 (ZSTD) each give their typed refusal. A multi-page file returns all pages, up to the page limit.
6. **Corpus test** (`tests/real_tiff.rs`, `#[ignore]`). Corpus directory: `$STUDIO_TIFF_CORPUS`, otherwise `../../../../Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/tiff`, which is the same relative scheme as develop `tests/real_dng.rs`.
   - Every file listed as supported in `provenance.json` decodes.
   - Every refused file has its expected refusal code.
   - The sha256 of each file is checked before use.

## Proof commands (studio cargo form, WORKSPACE_RULES topic cargo-form)
- `cd "<assets>" && source "<kernel>/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-interop-tiff --tests`
- the same prefix with `cargo clippy -p hsk-studio-interop-tiff --tests`
- the same prefix with `cargo test -p hsk-studio-interop-tiff`
- the corpus run: `cargo nextest run -p hsk-studio-interop-tiff --test real_tiff --run-ignored ignored-only`

Builders submit READY_FOR_VALIDATION and never self-certify (CX-PROOF-002).

</topic>

<topic id="samples" status="active" summary="Downloaded corpus">

## Samples (downloaded 2026-10-14 as test data; never executed, never committed)
- **Location:** `Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/tiff/`, 30 files, 1,743,613 bytes in total including the notices.
- **Source:** image-rs/image-tiff `tests/images` at commit `3302936b08e2260d4c66f40f51170efbb1fded4c`.
- **Licences:**
  - Top-level files: libtiff permission notice, kept as `NOTICE-COPYRIGHT`, plus `NOTICE-README.txt`.
  - `seq__*` files: covered only by the repository MIT LICENSE; there is no per-folder notice.
- **Provenance:** `provenance.json` holds the URL, licence, sha256 computed at download (upstream publishes none) and size of each file.
- **Coverage:**
  - RGB 8/16-bit, big and little endian
  - f32 (`gradient-3c-32b-float.tiff`) and float predictor 3 (`predictor-3-rgb-f32.tif`, `seq__seq-1c-32f-deflate-fpredict-*`)
  - gray 16-bit and gray+alpha
  - RGBA associated and unassociated (`seq__seq-4c-8b-rgba-*`)
  - CMYK 8/16-bit (`cmyk-4c-8b.tiff`, `cmyk-3c-16b.tiff`, `seq__seq-4c-16b-cmyk-*`)
  - LZW, PackBits, Deflate
  - tiled, tiled JPEG RGB and YCbCr
  - planar
  - palette (refusal in slice 1)
  - ZSTD (`int16_zstd.tif`, refusal)
  - LogLuv (refusal)
  - multipage
  - BigTIFF 8-bit and 16-bit
  - 12-bit (heterogeneous-depth probe)
- **Not available (gaps):**
  - Photoshop-written layered TIFF (37724, BE and LE)
  - CIELab PI 8
  - ICC-tagged real files
  - Photoshop 32-bit TIFF

  No public, licence-clear corpus was found (psdtags ships code only, Zenodo record 10473066 is a 39.9 kB code zip). These go to the D5 oracle session: Photoshop "Save As TIFF" with layers, ZIP/RLE layer compression, Mac and IBM byte order, Lab and CMYK with profile. ICC payloads for synthetic tests are under `oracle-corpus/icc/`.

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **R1. Photometric coverage gaps.** The `tiff` crate refuses CIELab (PI 8) and palette, and its README lists `ExtraSamples` as not supported. However, `colortype()` maps alpha ExtraSamples to RGBA/CMYKA (`decoder/image.rs:413-445`), and gray+extra decodes as `Multiband`.
  - Mitigation: typed refusals now; own unpacking over `read_chunk_bytes` in slice 2.
  - UNVERIFIED which PhotometricInterpretation Photoshop writes for Lab TIFF. D5 decides.
- **R2. Third-party panics on hostile input.**
  - Mitigation: decode runs behind this crate's own pre-checks (IFD count, entry count, page count, declared bytes versus input length) before calling the decoder.
  - Add a `catch_unwind` boundary only if a panic is observed. Record it, never hide it.
  - The crate has a fuzz harness (README), but no fuzzing claim is made here.
- **R3. Memory.** Use `tiff::decoder::Limits` (`decoding_buffer_size` 256 MiB by default, `ifd_value_size` 1 MiB) mapped to accord budgets. The 37724 tag can exceed 1 MiB on real layered files, so raise `ifd_value_size` only up to this crate's own declared cap, with a test.
- **R4. Version churn.** Pin `=0.11.3`. Version 0.11.x promises homogeneous bit depths only (CHANGES 0.11.2).
- **R5. Slice 3 (develop consolidation) may change DNG behaviour.** Mitigation: a byte-identical `DngRaw` (`container_sha256`, samples, unapplied list) gate on the three corpus DNGs plus the existing synthetic tests.

</topic>
