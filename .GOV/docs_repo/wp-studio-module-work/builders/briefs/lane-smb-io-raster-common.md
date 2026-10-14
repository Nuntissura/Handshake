---
file_id: BUILDER-BRIEF-LANE-SMB-IO-RASTER-COMMON
file_kind: builder_brief
updated_at: 2026-10-14T19:06:53Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-raster (NEW crate; module-map change = decision R-OWN, part of open Operator decision D6) + hsk-studio-harness (step 2, separate builder or same builder after step 1 pushes)
lane: SMB-IO-RASTER-COMMON
owned_paths: [crates/hsk-studio-interop-raster/** (step 1), crates/hsk-studio-harness/src/import/raster.rs + the `SourceFormat`/`sniff`/feature lines in crates/hsk-studio-harness/src/import.rs and Cargo.toml (step 2)]
depends_on: [decision R-OWN (new crate) + steward scaffold, steward pins png =0.18.1, decision J1 only for the JPEG encoder (slice 2)]
author_role: brief writer (parity follow-up 2026-10-14); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 9a2c840
parity_refs: [photoshop interop "JPEG / PNG / WebP / GIF" MISSING, LR-33, lightroom interop "JPEG / PNG / HEIC" MISSING, FG-20, top_gap photoshop#4, lightroom#4, figma#5]
---

<topic id="finding" status="active" summary="What exists today">

## Finding (verified in code and lock)
- No PNG/JPEG/WebP decoder or encoder in any crate. Harness `sniff` knows PSD/PSB, PDF, camera raw and TIFF magic only (`hsk-studio-harness/src/import.rs:62-89`); `SourceFormat` has no raster-interchange member (`:28`).
- The open/convert flow for a flat raster already exists end to end for TIFF, and every piece is reusable:
  - decode bytes -> planes: `hsk-studio-interop-tiff` (`src/decode.rs:12`), a folio-free codec crate (edges accord + observe only; `src/lib.rs:1-10`).
  - planes -> Folio proposal: `hsk_studio_interop_psd::raster_proposal(RasterSource{format, name, width, height, depth 8/16/32, colour_mode, planes (BE), alpha (straight), icc}, ...)` (`hsk-studio-interop-psd/src/proposal.rs:1716`, `:1762`), which also routes the embedded ICC like a PSD profile and records `profile_assumed_srgb` when absent.
  - proposal -> package: `package_proposal(format, importer, bytes, proposal, extra)` (`hsk-studio-harness/src/import.rs:428`): tile assets, records, receipt, original bytes.
  - the TIFF route that wires them: `hsk-studio-harness/src/import/tiff.rs:47-155` (flat path at `:110-123`).
- Lock already has `zune-jpeg 0.5.15` + `zune-core 0.5.3` (MIT OR Apache-2.0 OR Zlib; pulled by tiff's `jpeg` feature), `crc32fast 1.5.2`, `flate2 1.1.10`, `miniz_oxide 0.9.1`, `simd-adler32 0.3.10`, `bitflags 2.13.2`. Not locked: `png`, `fdeflate`, any JPEG encoder, `image-webp`.
- `image` crate is forbidden for every crate by the workspace probe (WORKSPACE_RULES crates-io-deps).

</topic>

<topic id="ownership" status="active" summary="Owner proposal (decision R-OWN, part of D6)">

## Owner (decision R-OWN; module-map change, lead/Operator)
- STUDIO_MODULES has no still-image codec owner; START_LOG 2026-10-11T01:50Z made reel the INTERIM owner pending D6. The F1 precedent (2026-10-14) chose a folio-free codec crate for TIFF instead of reel, because engines (develop) must be able to consume it without pulling Folio or the media crate.
- **Recommended:** new crate `hsk-studio-interop-raster`, edges `accord` + `observe` only (folio-free, like interop-tiff), the single owner of PNG / JPEG / WebP (later GIF/BMP) container decode and encode over `&[u8]`. Mapping into Folio stays with interop-psd `raster_proposal` (one tile builder) and the harness route (one dispatcher), exactly as for TIFF.
- Rejected: (a) reel adapters (develop/harness -> reel would pull the media crate into image import closures, and D6 still open); (b) extending interop-tiff (a TIFF-named crate owning PNG breaks the naming rule and CX-CAP-001 clarity); (c) putting codecs in harness (harness is a consumer, not an engine; develop could not reuse it for LR export).
- The crate name is NOT a verified canonical name (HSK naming rule): it follows the existing `interop-<family>` pattern; the lead confirms or renames before the steward scaffolds.

</topic>

<topic id="spec-basis" status="active" summary="Clauses (spec v02.216 module 14)">

## spec_basis
- STU-IO-006 (l.21313) rows: "| 4 | Portable network graphics | `.png` | Raster | I·X | Flat raster + alpha; 8/16-bit; no layers (rasterized on export) | SOX |" (l.21320); "| 5 | JPEG | `.jpg`/`.jpeg` | Raster | I·X | Flat lossy raster; metadata/ICC preserved; quality options exposed | SOX |" (l.21321); "| 7 | WebP | `.webp` | Raster | I·X | Raster + alpha + animation; lossy/lossless options | SOX |" (l.21323).
- STU-IO-136 (l.21061): "Output format | `JPEG`, `PNG`, `TIFF`, `PSD`, `PSB`, `DNG`, `AVIF`, `JPEG_XL`, `ORIGINAL`"; "Bit depth | `8`, `10`, `16`, `32`".
- STU-IO-004 (l.21294): five-artifact gate (scope descriptor, fixtures, round-trip envelope, receipts, recovery) before anything is called parity-complete.
- STU-IO-002 (l.21281): `StudioImportProfile` "MUST define a typed, deterministic decode from a source format into `StudioDocument`/`StudioLayer`/... primitives".
- Operator instruction 2026-10-14 (START_LOG): formats must "open directly or convert into the native format".

</topic>

<topic id="scope" status="active" summary="Slice 1 (~1-2 h)">

## Step 1 (crate hsk-studio-interop-raster; `#![forbid(unsafe_code)]`, `#![deny(clippy::arithmetic_side_effects)]` in every module)
1. `sniff(bytes) -> Option<RasterFormat>`: PNG `\x89PNG\r\n\x1a\n`, JPEG `FF D8 FF`, WebP `RIFF....WEBP` (WebP is recognised and refused `webp_decoder_unavailable` in slice 1).
2. `decode(bytes, &Limits, &CancellationToken) -> Result<RasterImage, RasterError>`:
   - `RasterImage { width, height, depth: 8 | 16, colour: Gray | GrayA | Rgb | Rgba | Cmyk (JPEG Adobe), planes: interleaved Samples::{U8, U16}, icc: Option<Vec<u8>>, exif_orientation: Option<u16> (recorded, not applied), gamma_chunk: Option<..> (recorded), source_bit_depth }`.
   - PNG via `png::Decoder` with `png::Limits{bytes}` set from our `Limits` (never unlimited); palette and sub-8-bit expanded through the crate's transformations (`EXPAND`), 16-bit kept as 16 (`STRIP_16` never); ICC from `iCCP`; APNG: first frame only + loss `png_animation_not_imported`; interlaced accepted.
   - JPEG via `zune_jpeg::JpegDecoder` with its decoder options max width/height set from `Limits`; ICC from APP2 `ICC_PROFILE` chunks (reassembled, bounded); CMYK/YCCK output kept as CMYK with Adobe inversion recorded; EXIF orientation read from APP1 (bounded, recorded only).
   - Typed refusals (stable snake_case): `dims_over_limit`, `decoded_bytes_over_limit`, `truncated`, `corrupt_png`, `corrupt_jpeg`, `arithmetic_coding_unsupported` (if zune refuses), `webp_decoder_unavailable`, `unknown_format`.
3. `encode_png(&PngEncodeRequest) -> Result<Vec<u8>>`: Gray/GrayA/Rgb/Rgba, 8/16-bit, optional ICC as `iCCP`, compression level enum; deterministic output (fixed filter strategy) so bytes are reproducible.
4. Round trip: `decode(encode_png(x))` returns identical samples, colour type and ICC bytes.
5. `DESCRIPTOR` (formats, directions, limits, refusals, NOT_PROVEN parity) + `report()` one terminal Observe frame per call (same shape as interop-tiff `lib.rs`).

## Step 2 (crate hsk-studio-harness; after step 1 pushes and the steward adds the edge)
- `SourceFormat::{Png, Jpeg}` + `sniff` branches (before the TIFF branch; PNG/JPEG magics do not collide), new optional feature `import-raster = ["import", "dep:hsk-studio-interop-psd", "dep:hsk-studio-prism", "dep:hsk-studio-interop-raster"]`, new `src/import/raster.rs` modelled on `import/tiff.rs:110-155`: `RasterImage` -> `psd::RasterSource{format: "png"|"jpeg", name: "Background", depth, colour_mode, planes (de-interleaved, BE for 16-bit), alpha, icc}` -> `raster_proposal` -> `package_proposal`. EXIF orientation != 1 is a loss `raster_orientation_not_applied` (same rule as TIFF tag 274).

## Non-goals (slice 1)
- JPEG encode (decision J1 below), WebP decode/encode, GIF, HEIC/AVIF/JXL, EXR (separate rows); animation import; metadata (XMP/EXIF) mapping beyond orientation/ICC; colour conversion on export (prism owns it; slice 2 export route).
- The EXPORT route from a native document (render -> sRGB -> PNG) is slice 2: no crate today owns "render a document to a deliverable" (harness has no composite/render-cpu edge; TIFF export has the same gap). Record it for the lead as decision R-EXPORT.

## Slice 2
- JPEG encoder (after J1), WebP via `image-webp` 0.2.4 (MIT OR Apache-2.0, published 2025-08-27; decode + lossless encode), export route decision R-EXPORT (candidate: session or harness feature with composite + render-cpu + prism + interop-raster edges).

</topic>

<topic id="native" status="active" summary="Relation to the native format">

## Native format
- Import produces the same package shape as TIFF: one raster `StudioLayer` with `rgb_f32le` working-space tiles (Folio `TileRef`s), the import receipt (`ImportRole::Receipt`, loss list incl. orientation/animation), and the ORIGINAL bytes (`ImportRole::Original`) so the source survives losslessly. No new Folio record, no new package entry kind.
- Export (slice 2) is a derived deliverable (STU-IO-136 export recipe), never stored back as authority.

</topic>

<topic id="ui" status="active" summary="UI dependency">

## UI
- None. Headless `studio-harness import SOURCE [--save OUT.handshake]` already exists and gains the formats; file dialogs/export panels wait for the GUI host (D11).

</topic>

<topic id="deps" status="active" summary="Edges and crates.io (steward)">

## Requests
- New crate + root member `hsk-studio-interop-raster`, allowed edges `accord`, `observe` (decision R-OWN). Edge `hsk-studio-harness -> hsk-studio-interop-raster` (`steward-sync.sh --add-edge hsk-studio-harness=hsk-studio-interop-raster`).
- `png = { version = "=0.18.1", default-features = false, features = [] }` — MIT OR Apache-2.0, published 2026-02-14 (crates.io API), rust-version 1.73; normal deps `bitflags ^2` (locked 2.13.2), `crc32fast ^1.2` (locked 1.5.2), `fdeflate ^0.3.3` (NEW 0.3.7 in local registry, MIT OR Apache-2.0, dep simd-adler32 locked), `flate2 ^1.0.35` (locked 1.1.10), `miniz_oxide ^0.8` with feature `simd` (NEW second copy 0.8.x next to locked 0.9.1; record as accepted duplicate or wait for a png release on 0.9). `png` has 1 `unsafe` occurrence in src (local registry grep).
- `zune-jpeg = { version = "=0.5.15", default-features = false, features = ["std"] }` and `zune-core =0.5.3` (already locked; local registry Cargo.toml 0.5.15: features `std`, `x86`, `neon`, `log`, `portable_simd`, default `x86`+`neon`+`std`; pin `["std"]` only, so the SIMD paths are compiled out and decoding is scalar).
- **Decision J1 (JPEG encoder, slice 2):** `jpeg-encoder` 0.7.1 (2026-07-27) and 0.7.0/0.6.1 declare `(MIT OR Apache-2.0) AND IJG`; `IJG` is NOT in `license_gate.py` OK set, so the gate fails. 0.6.0 (2023-10-09) declares `MIT OR Apache-2.0` but the code lineage is the same, so relying on the older label is not a clean answer. Options: (a) lead/Operator add `IJG` to the permissive set (IJG terms are permissive with an attribution notice), (b) own baseline JPEG encoder (~400 lines, DCT + Huffman, public ITU T.81), (c) defer JPEG export. Recommend (a) with the NOTICE line; until decided JPEG export refuses `jpeg_encoder_unavailable`.
- dep-requests.jsonl lines: appended by the steward on scaffold (this brief edits no shared ledger).

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (step 1: at most 5 tests; step 2: 2 tests)
1. **PNG round trip** (bytes built in the test): Gray8, GrayA16, Rgb8, Rgba16 with a 3144-byte ICC payload: `decode(encode_png(x))` keeps samples, colour, depth and ICC exactly; two encodes of the same input are byte-identical.
2. **PNG from foreign encoders:** a palette 4-bit and an Adam7-interlaced PNG built by hand-assembled chunks in the test (zlib via `miniz_oxide` dev-use or stored deflate blocks) decode to the expected RGB(A).
3. **JPEG decode:** a baseline 8x8 gray JPEG and a 16x16 4:2:0 colour JPEG embedded as byte arrays in the test (generated once, provenance comment, < 2 KB each) decode to within +/-2 of the stored expected samples; an APP2 ICC split over 2 chunks is reassembled byte-exact.
4. **Hostile (V4-02):** `dims_u32_max` (PNG IHDR 0xFFFFFFFF x 0xFFFFFFFF and JPEG SOF 65535x65535 refuse `dims_over_limit` before allocation), `aggregate_over_budget` (decoded bytes over `Limits.max_decoded_bytes` refuse), truncated IDAT / missing EOI refuse `truncated`/`corrupt_*`, ICC chunk count > 255 or total > `Limits.max_icc_bytes` refuse; nothing panics (per-test timeout of the workspace).
5. **Corpus (`#[ignore]`, V4-03):** `tests/real_raster.rs` over `$STUDIO_RASTER_CORPUS` or `../../../../Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/raster` with `provenance.json` + sha256 check (same scheme as interop-tiff `tests/real_tiff.rs`). The psd-tools corpus already holds 47 PNG files (oracle-corpus/psd-tools, MIT repo-level) usable as PNG decode inputs; JPEG/WebP samples need a licence-clear download (PngSuite is free to use; record licence before fetch).
6. **Harness (step 2):** `import` on a hand-built Rgba8 PNG produces one raster layer whose package re-opens with `ImportRole::Original` bytes equal to the input; a JPEG with EXIF orientation 6 carries `raster_orientation_not_applied` in the receipt.

## Proof commands
- `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-interop-raster --tests`
- same prefix: `cargo clippy -p hsk-studio-interop-raster --tests`; `cargo nextest run -p hsk-studio-interop-raster`
- step 2: same prefix `cargo nextest run -p hsk-studio-harness --features import-raster`

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Third-party decoder panics/allocation on hostile input.** Mitigation: own header pre-checks (IHDR/SOF dims, chunk lengths vs input length) before calling the crates; crate limits always set; `catch_unwind` only if a panic is observed and recorded.
- **Colour:** PNG `gAMA`/`cHRM`/`sRGB` without `iCCP` are recorded, not applied (raster_proposal's sRGB assumption stays explicit); JPEG CMYK without profile becomes a typed unmapped layer (existing interop-psd rule), never a guessed profile.
- **Duplicate miniz_oxide (0.8 + 0.9).** Acceptable pure-Rust duplicate; the steward records it; revisit when png moves to 0.9.
- **zune-jpeg determinism:** with `x86`/`neon` off the decoder is scalar, so output should be host-independent; tests still compare against stored values with +/-2 tolerance because the IDCT is not specified bit-exactly by T.81; byte-exact equality with other decoders is not claimed.

</topic>

<topic id="cohesion" status="active" summary="How it serves several apps">

## Cohesion
- One codec crate serves Photoshop open/save-as, Lightroom import and export (develop can consume it later through a develop -> interop-raster edge because it is folio-free), Figma/Illustrator PNG export of frames/artboards, InDesign placed images, After Effects/Premiere still-image footage, and the convert-to-native flow (harness) with the same `raster_proposal` mapping as TIFF/RAW.

</topic>
