---
file_id: BUILDER-BRIEF-LANE-SMB-DEVELOP-RAW-VENDOR
file_kind: builder_brief
updated_at: 2026-10-14T15:55:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-develop
lane: SMB-DEVELOP-RAW-VENDOR
depends_on: [decision R1 (licence route), decision R2 (camera colour source; slice 1 does not need it), SMB-INTEROP-TIFF slice 3 optional (slice 1 uses develop's existing private tiff.rs)]
author_role: format-coverage scoping researcher (SMB-FORMAT-SCOPE); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 45a3792
---

<topic id="finding" status="active" summary="What happens to proprietary RAW today">

## Finding (verified in code at 45a3792 plus the magic bytes of the downloaded samples)
- The entry point is `decode_container` (`crates/hsk-studio-develop/src/lib.rs:200-210`).
  - Files with TIFF-family magic go to `dng::decode`; everything else gets `DecoderUnavailable`.
  - `dng::decode` requires a DNGVersion tag and returns `UnsupportedContainer` without one (`dng.rs:350`).
  - Compression must be 1 or 7, otherwise `UnsupportedCompression` (`dng.rs:406-414`).
  - DESCRIPTOR pending lists "proprietary RAW containers (decoder_unavailable)" and "DNG compressions other than none and lossless JPEG (deflate, lossy JPEG)".
- Result per downloaded sample (first 16 bytes read with od):

| Sample | Magic | Today |
|---|---|---|
| CR2 (`canon-eos-350d-raw.cr2`) | `II*\0` + `CR\x02\x00` at byte 8 | `unsupported_container` (TIFF magic, no DNGVersion) |
| NEF D1H / D70s / Z6 | `MM\0*` / `MM\0*` / `II*\0` | `unsupported_container` |
| ARW ILCE-7S | `II*\0` | `unsupported_container` |
| ORF C5060WZ | `IIRS` | `decoder_unavailable` (magic not in the list at `lib.rs:202`) |
| RW2 LX7 | `IIU\0` | `decoder_unavailable` |
| RAF S5000 / X-S10 | `FUJIFILMCCD-RAW ` | `decoder_unavailable` |
| CR3 R6 | `ftypcrx ` (ISOBMFF) | `decoder_unavailable` |

- Reusable inside develop:
  - The TIFF IFD walker (`src/tiff.rs`, private).
  - The lossless-JPEG decoder (`src/ljpeg.rs`), which is the CR2 codec family.
  - CFA patterns, Bayer and the 6x6 X-Trans grid (`pipeline.rs`, DESCRIPTOR `cfa`).
  - The `DngRaw` frame with black/white, active area, linearization and the `unapplied` list (`dng.rs:121-147`).
  - `require_complete` and `solve_colour` (`dng.rs:863-895`), with typed `MissingColourData` and `UnsupportedContribution`.
- Spec gap: STU-CON-042 (l.22574) says MT-35538 "creates no ... container decoder". The DNG reader exists by lead decision (START_LOG decisions 01:50Z), and proprietary decoders extend it. Record this in the staged enrichment batch as "develop owns container decode for row 13/14 formats".

</topic>

<topic id="field" status="active" summary="Field research: crates, licences, decode-vs-develop structure">

## Rust RAW crates (checked 2026-10-14)

| Crate | Version / date | Licence | Coverage | Verdict under the Studio licence policy (START_LOG decisions 01:50Z: copyleft only in optional isolated providers, with Operator approval) |
|---|---|---|---|---|
| rawler (dnglab) | 0.8.0, 2026-08-30 | LGPL-2.1 (Cargo.toml) | broad incl. CR3; DNG writer | Excluded from the closure. It also depends on `image 0.25`, which `tools/workspace-probe.rs` forbids for every crate. Usable only as an EXTERNAL oracle process (dnglab CLI), never linked. |
| rawloader | 0.37.2, 2026-08-15 (previous 2022) | LGPL-2.1 | MRW ARW SRF SR2 RW2 NEF NRW CRW CR2 ORF KDC DCS DCR MEF SRW ERF RAF PEF IIQ MOS 3FR ARI, partial DNG | Excluded (copyleft) |
| quickraw | 0.2.1-alpha.1, 2023 | LGPL-2.1 | partial | Excluded |
| rawshift | 0.1.1, 2026-05-30 | MPL-2.0 | ARW stabilizing; CR2/NEF/RAF incomplete; CR3 metadata only; libwebp-sys/libheif | Excluded (copyleft file-level, C deps, immature) |
| rawkit (Graphite, GSoC 2024) | 0.1.0, 2024-11-03 | MIT OR Apache-2.0 | Sony ARW only (arw1, arw2, uncompressed); camera_data TOML for about 40 Sony bodies (matrix provenance UNVERIFIED) | Permissive. Candidate behaviour reference or dependency for ARW only; deps bitstream-io 4.x, num_enum, thiserror 2, rawkit-proc-macros |
| LibRaw (C++) | n/a | LGPL-2.1 or CDDL-1.0 | broadest | FFI is forbidden in develop (native policy: `-sys` only in designated native providers). Only an isolated provider, which needs an Operator decision. |

Sources:
- https://docs.rs/crate/rawler/latest
- https://raw.githubusercontent.com/dnglab/dnglab/main/rawler/Cargo.toml
- https://docs.rs/crate/rawloader/latest
- https://raw.githubusercontent.com/pedrocr/rawloader/master/Cargo.toml
- https://lib.rs/crates/quickraw
- https://docs.rs/crate/rawshift/0.1.0
- https://docs.rs/crate/rawkit/latest
- https://raw.githubusercontent.com/GraphiteEditor/Graphite/master/libraries/rawkit/Cargo.toml
- https://www.libraw.org/about

## How mature tools split decode from develop
- **RawSpeed (LGPL-2, darktable org).** It is "only ... the first stage decoding". It supplies unmodified raw data, the CFA layout and black levels. It does no colour, white balance or demosaic, and camera definitions are XML data (https://github.com/darktable-org/rawspeed). darktable does all development in its own pipeline.
- **LibRaw.** It extracts raw pixels, processing metadata and previews. Its dcraw-derived RGB conversion is kept only "for compatibility and quick testing"; production rendering is out of scope (https://www.libraw.org/about).
- **RawTherapee 5.11.** It reads raws through LibRaw and falls back to its enhanced dcraw reader (https://rawtherapee.com/downloads/5.11).
- **Studio already matches this split.** Container decoders produce an immutable `DngRaw` frame, and the existing develop pipeline (demosaic, white balance, Prism transform, curves) is untouched. Each vendor decoder is its own module that outputs that frame plus an `unapplied` list. Camera colour comes from a data table, not from code.

## Format facts for slice 1 (CR2)
Public description: libopenraw format notes https://libopenraw.pages.freedesktop.org/formats/cr2 and https://wiki.freedesktop.org/libopenraw/Canon_CR2/, plus https://www.swetake.com/photo/cr2/cr2-2_en.html. Read the documentation pages only; do not read LGPL code.
- **Header.** TIFF header, `CR`, major 2, minor 0, then a u32 offset of the raw IFD ("IFD3"). The sample shows `CR\x02\x00` at byte 8.
- **Raw data.** Lossless JPEG (ITU-T T.81 process 14, SOF3).
- **Slices tag 0xC640 (50752).** Three SHORTs: number of full slices, width of each full slice, width of the last slice.
  - Decoded JPEG rows are reassembled column-slice by column-slice, top to bottom, then left to right.
  - The JPEG frame width counts two components per sample, so the real width is twice the JPEG width (libopenraw).
  - Sources disagree on the first value (1 vs 2). The sensor width check `slices*w1 + w2 == raw width` decides, and a mismatch is a typed refusal.
- **Model differences.** Older bodies such as the 350D, 5D and 30D differ in layout (libopenraw). The downloaded sample is a 350D, which is the hard case first. Add a newer CR2 body if slice 1 hits that variant.
- **Black and white levels.** These come from masked sensor borders or MakerNote sensor info. Slice 1 records black and white as `unapplied` or caller-supplied, unless they are derivable from the masked area, which is UNVERIFIED in the docs.

</topic>

<topic id="decisions" status="active" summary="Operator/lead decisions this lane needs">

## Decisions
- **R1 (licence route; lead recommends a):**
  - (a) Clean-room own decoders in develop from public format documentation, permissive, in the order CR2 -> NEF (uncompressed, then Nikon Huffman) -> ARW (rawkit MIT/Apache as permissive reference or dependency) -> ORF/RW2 -> RAF (incl. X-Trans) -> CR3 (ISOBMFF + CRX wavelet, the largest).
  - (b) Link rawkit for ARW only.
  - (c) dnglab (LGPL) as an external converter process to DNG inside the `hsk-studio-worker` supervision (D18). Shipping it needs Operator approval, but it is allowed now as a TEST ORACLE outside the closure.
  - (d) A LibRaw native provider crate (new FFI owner).
- **R2 (camera colour, needed from slice 2):** proprietary files carry no ColorMatrix.
  - (a) The Operator converts each sample with the free Adobe DNG Converter (Operator-attended, D5-style session), and the ColorMatrix/ForwardMatrix values become a develop data table with provenance. Whether numeric matrices may be reused is UNVERIFIED (not legal advice).
  - (b) rawkit camera_data (MIT/Apache, value provenance UNVERIFIED).
  - (c) Caller-supplied colour only.
  - Slice 1 needs none of these: it refuses `missing_colour_data` at `solve_colour` unless the caller provides a `DngColor`.

</topic>

<topic id="scope" status="active" summary="Minimal slice first">

## Slice 1 (minimal): CR2 CFA decode
1. **Dispatch.** `decode_container` (`lib.rs:200`) routes TIFF-magic files by marker:
   - DNGVersion -> `dng`;
   - `CR\x02` at byte 8 -> new `src/cr2.rs`;
   - otherwise unchanged (`unsupported_container`).
   - Add the magics `IIRO`/`IIRS`/`MMOR` (ORF), `IIU\0` (RW2), `FUJIFILMCCD-RAW` (RAF) and `ftypcrx` (CR3) as RECOGNISED but typed `decoder_unavailable` with the format named in the error detail. Today they are anonymous.
2. **`cr2::decode(bytes) -> DngRaw`.**
   - Walk to the raw IFD from the header offset using the existing `tiff.rs`.
   - Read StripOffsets/StripByteCounts and tag 50752.
   - Decode the SOF3 stream with `ljpeg.rs`, extending it only if CR2 needs features DNG did not (e.g. 2 or 4 components, predictor 1); the extension stays in `ljpeg.rs` (one owner).
   - Reassemble the slices with checked arithmetic.
   - The CFA pattern comes from the camera family. CR2 has no CFAPattern tag; RGGB for the sample body is UNVERIFIED, so the oracle checks it.
   - The result is `DngRaw` with `color` empty and the `unapplied` list naming what was not read (black/white from MakerNote, sensor crop).
   - `container_sha256` is set exactly as for DNG.
3. **No pipeline changes.** `develop_to_image` with a caller-supplied `DngColor` produces an image. Without one, `missing_colour_data` (typed).
4. **Rules.** `#![deny(clippy::arithmetic_side_effects)]` in `cr2.rs` (V4-01). Bounded slices and allocation only after a payload-size check, as in the `dng.rs:462` pattern of review R5 (V4-02).
5. **DESCRIPTOR.** Add CR2 (CFA decode, colour caller-supplied); list the others as recognised and unavailable.

## Slice 2+ (sequenced, one format per lane step)
- NEF uncompressed (D1H sample), then NEF Huffman-compressed (D70s lossy type 1, Z6 12-bit compressed; NEF linearization curve from the MakerNote).
- ARW2 (ILCE-7S).
- ORF, RW2.
- RAF incl. X-Trans (X-S10; the develop X-Trans demosaic already exists).
- CR3 (R6; ISOBMFF box parsing; CRX codec).
- Each step adds its camera colour entries per decision R2.
- Adjacent cheap ROI: DNG Deflate (Compression 8, float DNG) and lossy-JPEG DNG (34892). Both get cheap once SMB-INTEROP-TIFF slice 3 brings flate2/zune-jpeg through the shared TIFF owner.

</topic>

<topic id="deps" status="active" summary="Dependency and edge requests">

## Requests
- **Slice 1:** none. It uses develop's existing modules and `sha2`, with no new crate and no new edge.
- **Optional rawkit (decision R1 b):**
  - `rawkit = { version = "=0.1.0", default-features = false, features = [] }`, MIT OR Apache-2.0.
  - New packages: bitstream-io 4.x, num_enum 0.7, thiserror 2, rawkit-proc-macros 0.1.
  - The steward checks the probe and licence gate first. Note: docs.rs lists a bitstream-io ^2.5.3 requirement for 0.1.0 while the repo master uses 4.9.0. UNVERIFIED which one the published 0.1.0 resolves; read the registry Cargo.toml before pinning.
- **Later:** `develop -> interop-tiff` (SMB-INTEROP-TIFF slice 3).

</topic>

<topic id="spec-basis" status="active" summary="Clauses">

## spec_basis
- **STU-IO-006 row 13** (l.21329): "Camera raw (mosaic) | proprietary raw family (`.cr2`/`.cr3`/`.nef`/`.arw`/`.raf`/…) ... I·EP | Decoded + developed via `StudioRawDevelop` (14.12)".
- **Row 14** (l.21330): `.dng` I·EP·X.
- **STU-RAW-003** (l.2085-2093): "The pipeline MUST accept mosaic raw sensor inputs — both the Bayer and the X-Trans colour-filter-array families ... demosaic them through a native deterministic engine in the owning modular Studio asset crate".
- **STU-IO-004** (l.21294): five-artifact gate.
- **STU-IO-003** (l.21290): "no implicit device color". This is why slice 1 refuses rather than guessing a matrix.
- **Operator instruction 2026-10-14:** "PSD, TIFF and RAW must stay usable: open directly or convert into the native format."
- **Staged enrichment item:** STU-CON-042 "creates no ... container decoder" versus develop's container decoders (see the finding topic).

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (slice 1, at most 5 tests)
1. **Synthetic CR2.** Build in the test a tiny CR2: TIFF header + `CR\x02\x00` + raw IFD with tag 50752 `[2, 8, 8]` over a 24x4 SOF3 stream. Encode it with a test-local lossless-JPEG encoder or a hand-built bit stream. `decode_container` must return a `DngRaw` whose samples equal the source grid after slice reassembly.
2. **Slice mismatch.** Slice widths that do not sum to the raw width give a typed refusal. A slice count that overflows the width gives a typed refusal before allocation.
3. **Magic routing.** An ORF/RW2/RAF/CR3 magic prefix gives `decoder_unavailable` with the format named. A TIFF without DNGVersion and without the CR marker stays `unsupported_container`.
4. **Missing colour.** A decoded CR2 without a supplied colour gives `missing_colour_data` at `solve_colour`/`recipe`, never a default matrix.
5. **Corpus test** (`tests/real_raw.rs`, `#[ignore]`). Corpus `$STUDIO_RAW_CORPUS`, otherwise `../../../../Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/raw-vendor`, sha256-checked against `provenance.json`.
   - The 350D CR2 decodes with the width, height and bit depth from the raw.pixls.us Exif dump: https://raw.pixls.us/getfile.php/1230/exif/IMG_8728.CR2.exif.txt ; copy the values into the test before running.
   - ORACLE: the CFA samples equal the CFA samples of the same file converted to uncompressed DNG by an independent tool and read by develop's own DNG reader. The tool is dnglab `convert` or the Adobe DNG Converter, run by the validator or the Operator outside the closure; record the converter version and sha256 of the produced DNG. If no converter run is approved, the oracle is NOT_PROVEN, and the test still checks dimensions and determinism.

## Proof commands
- `cd "<assets>" && source "<kernel>/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-develop --tests`
- `cargo clippy -p hsk-studio-develop --tests` (same prefix)
- `cargo test -p hsk-studio-develop` (same prefix)
- corpus: `cargo nextest run -p hsk-studio-develop --test real_raw --run-ignored ignored-only`
- regression: `--test real_dng --run-ignored ignored-only` (the three DNGs are now downloaded)

</topic>

<topic id="samples" status="active" summary="Downloaded corpus">

## Samples (downloaded 2026-10-14; CC0-1.0 per the raw.pixls.us listing row; sha256 verified against the listing; never executed, never committed)
- **DNG corpus.** `Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/dng/` holds the three D10 DNGs, 11,142,186 bytes in total:
  - `canon-eos-5d3-mlv-14bit-lossless.dng` (id 2204, sha256 1d77dcc6…b603e)
  - `samsung-sm-g973u-16bit-uncompressed.dng` (id 3839, 6492b7e0…84ab1)
  - `nikon-d80-16bit-lossless-tiled-linearized.dng` (id 1988, 5095ca26…26bb)

  These are the names and path `tests/real_dng.rs` and `tests/real_dng/fetch-corpus.sh` expect. `provenance.json` is a superset of the fetch script's format.
- **Proprietary corpus.** `.../oracle-corpus/raw-vendor/` holds 10 files, 78,245,849 bytes in total, with `provenance.json` (id, camera, mode, URL, licence, sha256, bytes):

| File | raw.pixls.us id | sha256 prefix |
|---|---|---|
| `canon-eos-350d-raw.cr2` | 1230 | 4b47ba27c0090efa |
| `canon-eos-r6.cr3` | 4659 | 74abb0a113d075ad |
| `nikon-d1h-12bit-uncompressed.nef` | 5364 | d110b80a8b10a3a6 |
| `nikon-d70s-12bit-lossy-type1.nef` | 1117 | 6c46f9854033e762 |
| `nikon-z6-12bit-compressed.nef` | 3587 | f0ab08da469ee205 |
| `sony-ilce-7s-14bit-compressed.arw` | 1582 | a35ebb2fbec929da |
| `fujifilm-finepix-s5000.raf` | 2726 | dabd5e74521a6980 |
| `fujifilm-x-s10-14bit-compressed-xtrans.raf` | 4190 | 9419d1408ebf8503 |
| `olympus-c5060wz-12bit.orf` | 1799 | 3fc12bc3f6320cfa |
| `panasonic-dmc-lx7.rw2` | 7010 | 18dfc3877e944020 |

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Clean-room provenance.** Builders must not read LGPL/GPL decoder source (rawloader, rawler, LibRaw, RawSpeed, darktable, RawTherapee). Allowed sources are format documentation, the raw.pixls.us Exif dumps and black-box tool output. Record the sources used in the module header, following the `dng.rs:1-9` pattern.
- **No public colour data with a clear licence** (R2). Mitigation: slice 1 is colour-free with a typed refusal.
- **Variant explosion** (CR2 old/new layouts, Nikon compression types, Sony ARW1/ARW2, Fuji compressed and uncompressed). Mitigation: one variant per step. An unknown variant is a typed refusal naming the variant; it is never guessed.
- **CR3/CRX cost** (wavelet codec, no public spec). Schedule it last, and re-evaluate with the Operator against R1 (c) before building.
- **Hostile input.** The same V4-01/02 controls as DNG apply. The `dng.rs:462` payload-versus-declared-frame check is mandatory for every compressed format.

</topic>
