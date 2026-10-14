---
file_id: BUILDER-BRIEF-LANE-SMB-CONVERT-NATIVE
file_kind: builder_brief
updated_at: 2026-10-14T16:00:00Z
wp: WP-KERNEL-STUDIO
crate: multi-owner sequence (interop-psd CN-1, package CN-2, harness CN-3, press CN-5); one owner per step, disjoint files
lane: SMB-CONVERT-NATIVE
depends_on: [decision C1 (dispatcher owner), decision C2 (package typed import entries), SMB-INTEROP-TIFF slice 1 for the TIFF entry, SMB-DEVELOP-RAW-VENDOR for proprietary RAW (DNG works today), decisions P1/P2 for PDF]
author_role: format-coverage scoping researcher (SMB-FORMAT-SCOPE); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 45a3792
---

<topic id="finding" status="active" summary="Code evidence that no open/convert-to-native flow exists">

## Finding: importers exist, but none of them reaches the native container (verified at 45a3792)
- **G1. No crate composes an importer with `hsk-studio-package`.**
  - No crate depends on `hsk-studio-interop-psd`. The only grep hits outside it are comments in composite, pigment and prism.
  - Session opens only `.handshake` bytes or paths (`hsk-studio-session/src/session.rs:515` `open_path`, `:538` `open_bytes`; DESCRIPTOR input "A .handshake package file or bytes").
  - Harness offers only the `chronicle.patch` provider (`hsk-studio-harness/src/harness.rs:29`) and has no importer edge (Cargo.toml: accord, folio, session, controls, observe).
- **G2. `ImportProposal` has no path into a package.**
  - `ImportProposal { snapshot, tiles, mask_tiles, extras, layer_map, loss, provenance, raster_records }` is at `interop-psd/src/proposal.rs:305-316`.
  - Package writes only document + digest-keyed assets + previews + opaque entries: `EntryKind { Document, Asset, Preview, Opaque }` (`package/src/manifest.rs:20-25`), and `Extras { quarantined, previews }` (`package/src/write.rs:132-138`).
  - Package and session have no reference to `StudioRasterRecords`, a loss report, import provenance, or the original source bytes (grep of `raster_records|RasterRecords|design_records` over package/session: 0 hits).
  - STU-IO-001 requires preserving "originals and unknown safely retained semantics with explicit loss receipts". STU-IO-005 requires the preservation blob to be "carried with the document".
- **G3. RAW output has no Folio mapping.**
  - `DevelopedImage { width, height, rgb: Vec<f32> }` is at `develop/src/pipeline.rs:1090-1094`.
  - Develop has no Folio edge and must not get one (STU-CON-045, l.22600).
- **G4. The flat-raster -> Folio layer + Pigment tile builder is private to interop-psd** (`TILE_FORMAT = "rgb_f32le"` `proposal.rs:74`, `TileBlob` `:184`, `Builder`). TIFF and RAW would have to duplicate it, which violates CX-CAP-001.
  - Layerless PSD: `proposal.rs` never references `image_data`, `decode_merged` or the merged image. A PSD with only a merged image therefore appears to map to no pixel layer. UNVERIFIED by a test; CN-1 adds one.
- **G5. PDF has no native import or export.**
  - Press has no Folio dependency (`press/Cargo.toml`: accord, observe). `PdfDoc::proposal` (`press/src/pdf_edit.rs:465`) is a press-local object model.
  - Nothing maps PDF pages into a `StudioDocument`, and nothing writes a PDF from a `StudioDocument`. Press writes PDFs only from an opened PDF; `interop-vector/src/ai_adapter.rs` only detects `%PDF-`.

</topic>

<topic id="design" status="active" summary="One cohesive flow and its owners">

## One flow for all entry points (the Operator prefers cohesion and interflowing workflows)
`source bytes -> sniff -> importer -> ImportedDocument -> package::write_package_with -> Session::open_bytes`

- `ImportedDocument` = Folio snapshot + digest-keyed asset bytes + standalone records + loss receipt + provenance + original source bytes as an asset.
- Every Studio sub-app then edits the same native document through the session it already has. Session needs no change, because a converted package opens through `open_bytes`.

| Step | Owner (CX-CAP-001) | Content |
|---|---|---|
| CN-1 | interop-psd | Expose the private proposal `Builder` as a public raster entry, e.g. `raster_proposal(RasterSource{width,height,depth,colour_mode,planes,alpha,icc}, cfg, mint, converter, cancel) -> ImportProposal`. Map a layerless PSD's merged image through it as one raster layer. TIFF pages and developed RAW use the same entry, so the CMYK/Lab/gray ICC display conversion, tiling and tile digests stay single-owner. Long-term lowest-owner candidate: pigment (tiles); recorded, not now. |
| CN-2 | package | Typed import entries (decision C2): standalone records (`hsk.studio.raster_records@1`, later design/vector records), an `import` receipt (loss report + provenance JSON) and the original source as a manifest-tagged asset. All are hash-verified, bounded and re-emitted on save, and none are reported as quarantined. Today the only carrier would be `Opaque`/quarantined, which produces a loss note on every open, so that is wrong. |
| CN-3 | harness (decision C1) | `import` module with features `import-psd`, `import-tiff`, `import-raw` and `import-pdf`, so each importer is an explicit selected provider (the harness responsibility). Sniffing: `8BPS` v1/v2 -> PSD/PSB; `II*\0`/`MM\0*`/BigTIFF -> TIFF, or the DNG route when DNGVersion is present (develop decides via `decode_container`); `CR\x02` at byte 8, `IIRO/IIRS`, `IIU\0`, `FUJIFILMCCD-RAW` and `ftypcrx` -> develop; `%PDF-` -> press. Each importer gives an `ImportedDocument`, which is written to package bytes and opened by `Session::open_bytes`. The CLI gets `--import IN --save OUT.handshake`, and the report lists receipts and NOT_PROVEN items. |
| CN-4 | export back | Native -> PSD: `export_psd` (`export.rs:279`) needs the original `PsdDocument`, which comes from the CN-2 original asset. Native -> TIFF/PDF needs a composited raster: the harness needs a render path, and today its NOT_PROVEN list says "no RenderPort; render engines are not dependencies of the harness" (`harness.rs:32-36`). This is a separate edge decision. |
| CN-5 | press (decisions P1/P2) | PDF page -> raster layer import needs a public page raster from press. Press's hayro renderer is renderer-internal by lead decision 2026-10-11T19:18:40Z ("press must not expose or reuse that conversion as a colour capability"), so P1 decides whether a page raster may be exposed as an import with prism-managed colour. Native -> PDF export (raster pages + ICC OutputIntent first) is decision P2. Until then the PDF entry imports nothing editable: it carries the original PDF as the preserved asset plus page count and media boxes from `PdfDoc::proposal`, with the typed loss `pdf_pages_not_imported`. That is an honest STU-IO-004(d) receipt. |

## Decision C1: owner of the dispatcher
- **(a) harness (recommended).**
  - No new crate.
  - Its stated responsibility is "explicit selected providers".
  - Host adoption is out of scope now (Operator 2026-10-14).
- **(b) A new top-level crate.**
  - It would be a module-map change with an invented name, so the naming rule needs the Operator.
- **(c) session.**
  - Rejected: its DESCRIPTOR says "No built-in all-engine mega dependency".

## Decision C2: how package carries import data
- **(a) New `EntryKind` for typed records + import receipt (recommended).**
- **(b) Reserved-name opaque entries with a non-quarantine flag.**

</topic>

<topic id="deps" status="active" summary="Edge requests (steward; all need lead decision)">

## Edge requests
- **CN-3 harness edges, optional, each behind its feature:**
  - `harness -> package`, so it can call `write_package_with` directly. Today it reaches package only through session.
  - `harness -> interop-psd`
  - `harness -> develop`
  - `harness -> interop-tiff`, after SMB-INTEROP-TIFF is scaffolded
  - `harness -> press` (CN-5)
  - The harness also needs `pigment` and `prism` only if it constructs the `PrismConverter` itself. Prefer interop-psd's existing `PrismConverter` (`interop-psd/src/convert.rs:308`).
- **CN-1 and CN-2:** no new edges. Interop-psd already has folio, pigment and prism, and package has folio.
- **Crates.io:** none.

</topic>

<topic id="spec-basis" status="active" summary="Clauses">

## spec_basis
- **STU-IO-001** (l.21275): "Studio MUST provide a versioned portable Handshake-native document snapshot/container ... preserve originals and unknown safely retained semantics with explicit loss receipts".
- **STU-IO-002** (l.21281): "`StudioImportProfile` MUST define a typed, deterministic decode from a source format into `StudioDocument`/`StudioLayer`/`StudioMask`/... primitives".
- **STU-IO-005** (l.21304): the preservation blob "MUST be re-emitted on export where the target is the same family", and a receipt records preserved-opaque versus imported.
- **STU-IO-006 rows 1-3, 13, 14, 20** (l.21315-21336). Row 20: PDF "I·EP·X | Multi-page import (pages as layers/spreads)".
- **STU-IO-014** (l.21438): every import/export action is "a typed, model-steerable command with a stable identifier". No canonical command id is verified, so the ids are a lead naming decision. Use capability wording per the STU-RAW-003 provenance paragraph (l.2096-2100).
- **Operator instruction 2026-10-14** (START_LOG): "Handshake uses its own file format ... PSD, TIFF and RAW must stay usable: open directly or convert into the native format. PDF must be usable for compatibility with larger corporations." The Operator prefers cohesion and interflowing workflows between the apps.

</topic>

<topic id="acceptance" status="active" summary="Acceptance per step">

## Acceptance (minimal slice = CN-1 + CN-2 + CN-3 with PSD and DNG; TIFF joins when SMB-INTEROP-TIFF slice 1 passes)
- **CN-1 (interop-psd, at most 3 tests):**
  - A synthetic layerless 8-bit RGB PSD (merged image only) maps to one raster layer whose tiles equal the existing per-layer path for the same pixels.
  - `raster_proposal` with 16-bit gray and 32-bit RGB planes gives `rgb_f32le` tiles, and `validate_raster_records` passes.
  - The existing interop-psd tests stay green.
- **CN-2 (package, at most 3 tests):**
  - Write then read keeps records, the import receipt and the original asset byte-identical and hash-verified.
  - Reading adds no loss note for them.
  - An oversized or hash-mismatched record entry gives a typed refusal with no partial state.
- **CN-3 (harness, at most 4 tests):**
  - A PSD built from the interop-psd test helpers imports, writes a package, and opens in a Session. The layer count equals the proposal count, and the receipt is present.
  - DNG: the ignored corpus test imports `samsung-sm-g973u-16bit-uncompressed.dng` from `oracle-corpus/dng` through develop (`decode_container` -> `develop_to_image` with a `PrismConverter` or identity) into one raster layer, using the sha256-checked corpus.
  - A `%PDF-` input gives the original asset plus the typed `pdf_pages_not_imported` receipt.
  - Unknown magic gives a typed `format_unrecognised`. A proprietary RAW gives develop's typed refusal, passed through unchanged.

## Proof commands (per owner crate; studio cargo form)
- Run `cargo check`, `cargo clippy` and `cargo test` with `-p hsk-studio-interop-psd`, `-p hsk-studio-package` and `-p hsk-studio-harness --features import-psd,import-raw`, each using the prefix `cd "<assets>" && source "<kernel>/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" &&`.
- Corpus: `cargo nextest run -p hsk-studio-harness --features import-raw --run-ignored ignored-only`.

</topic>

<topic id="samples" status="active" summary="Samples">

## Samples
- `oracle-corpus/dng/` (3 CC0 DNGs)
- `oracle-corpus/raw-vendor/` (10 CC0 proprietary RAWs; typed refusals until SMB-DEVELOP-RAW-VENDOR)
- `oracle-corpus/tiff/` (30 TIFF variants)
- `oracle-corpus/icc/` (existing profiles)
- No layered-PSD corpus is downloaded yet. The psd-tools fixtures (MIT) are approved in principle by D5, but their licence and size must be stated before the fetch. CN tests use synthetic PSDs from the interop-psd test helpers.

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Scope creep into a facade.** Keep each importer behind its own harness feature, with no default features.
- **Colour.** Every import is profile-explicit (STU-IO-003). RAW output carries the develop output profile id. A TIFF without ICC uses the same "assumed sRGB" rule and loss entry as PSD (`Provenance::assumed_profile`), never a silent default.
- **Memory.** Package asset bytes for large imports: use `write_package_streaming` / `save_atomic_streaming` (package DESCRIPTOR) when an import exceeds an in-memory budget.
- **Identity.** IDs are minted by the host `IdMint`. The harness uses its existing standalone minting; it creates no new prefixes.
- **PDF expectations.** Corporate PDF use is mostly export. CN-5 needs P1/P2 before any promise; until then the descriptor must say "PDF: carried, not imported".

</topic>
