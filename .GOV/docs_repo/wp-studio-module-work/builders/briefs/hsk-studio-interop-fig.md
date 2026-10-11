---
file_id: BUILDER-BRIEF-HSK-STUDIO-INTEROP-FIG
file_kind: builder_brief
updated_at: 2026-10-11T07:00:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-fig
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-INTEROP_FIG` / key `interop_fig` / crate `hsk-studio-interop-fig`; foundation MT-35550 (test target `fig_import`; cases `fig_container_bounded_identity_loss`, `unsupported_version_duplicate_key_bomb_cancel`, `actual_fig_instance_variable_prototype_edit_reopen`, `stale_import_and_sanitized_policy`; deadline 60 s); write scope `handshake-studio-assets/crates/hsk-studio-interop-fig/**`. Files per MT: `src/container.rs`, `src/version_map.rs`, `src/semantic_decode.rs`, `tests/fig_import.rs`.
- Responsibility (verbatim): "Offline .fig proprietary-version decoder and conversion adapter owner; decoder availability NOT_PROVEN. Preserve original/source-qualified IDs and map components/variants/variables/modes/auto-layout/prototypes with loss receipts. SVG/PNG/token/API interchange does not replace required editable .fig import."
- Foundation capability: "Execute real bounded .fig container/identity/opaque preservation/loss validation and reject unsupported versions; editable decoder feature remains explicitly blocked until original-produced semantic/render proof."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-components`, `hsk-studio-type`, `hsk-studio-observe`. Slice uses accord + observe (+ folio only for provenance ids later); `components` is for the later semantic mapping, so the container capability can run in wave 1.
- Forbidden: handshake_core/native/SurrealDB; Figma cloud/API/account access (local-copy only, STU-IO-008/013); claiming editable support from container success; substituting SVG/PNG/tokens (STU-IO-186). Foundation permits ONLY container/identity/opaque/loss/version-reject (prep `interop_fig` row).
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `interop-fig-consumer` (`--descriptor`), `bounded_case` (60 s). Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-IO-008 (l.21372+): Figma family `.fig/.jam/.deck/.buzz/.site/.make` is a LOCAL-copy round-trip (no cloud/account); STU-IO-006 rows 35-40 (NRT); STU-IO-186 (l.211): native editable local FIG remains requested scope, "FIG is not satisfied by SVG or preview extraction", per-object/property mapping, preserved opaque data, unsupported/loss decisions, provenance, round-trip disposition; missing decoders are blocking scope gaps, parser success/self-render/original-byte preservation is NOT editable-parity proof; STU-IO-187 (container safety, duplicate-key rejection, bomb limits, unknown version behaviour, quarantine); STU-IO-004/005 gate + preservation blob.
- Components/variables/modes/auto-layout/prototypes target model: 14.10 (l.19658+), 14.11 (l.20273+); STU-ARC-016 (decoder-missing foundation may expose container/opaque/loss validation only), STU-ARC-019 (opaque imported objects keep explicit semantics).
- Later scope: Kiwi message -> node tree -> Folio/Components mapping, variants/variables/modes/prototypes, images/fonts, `.jam/.deck/.buzz/.site/.make`, writer; `_mt_plan_1213_raw_interop.json` MT-469 (Figma local-copy family), `_mt_plan_1011_designsys_proto.json`.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (~700-1000 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[12]` (MT-35550; "Bounded container/version/identity decoding with opaque preservation; actual offline semantic decoder must map variants/variables/modes/auto-layout/prototypes to Folio/Components"; provider "Decoder NOT_PROVEN/NOT_INSPECTED. Foundation container handling is permitted narrow capability, no declaration of editable conversion support"; oracle "Original Figma-produced local FIG with actual semantic edit/reopen and original rendering/property snapshots; SVG/PNG/token/API substitutes rejected"; gap "Actual native decoder and independent original-app oracle resources block editable feature; scope stays retained"); visual-team finding VIS-PREP-004 (BLOCKING_PARITY: FIG/INDD editable decoder availability unproven); `.GOV/reference/studio_app_feature_research/21-figma-feature-map.md`, `23-figma-leaf-index.md`, `27-illustrator-figma-parity-matrix.md`, `19-studio-local-first-rust-posture.md` (".fig undocumented schema = compatibility target, not internal storage authority"). No local `.fig` fixture was found on the machine (bounded search, 2026-10-11): the lead/operator must supply Figma-authored files; until then fixtures are SYNTHETIC and prove container handling only.
Format facts (public community sources, NOT an official spec - Figma has published none; verified by web search 2026-10-11; confirm against the first real file before relying on any byte): a modern `.fig` is a ZIP (`canvas.fig`, `meta.json`, `thumbnail.png`, `images/<hash>`); `canvas.fig` starts with an 8-byte ASCII prelude (`fig-kiwi` design, `fig-jam.` FigJam, `fig-deck` deck) + little-endian u32 version, then length-prefixed chunks; chunk 0/1 = Kiwi schema compressed with raw deflate, following chunk = Kiwi message (zstd in newer files, detected by magic `28 B5 2F FD`); sources disagree on chunk numbering -> parse generically and record what was seen; legacy files may be the bare `canvas.fig` stream without ZIP. Kiwi (evanw/kiwi) is a self-describing binary schema format similar to protobuf.
Reuse (verified): `accord::{CancellationToken, DomainId}`; `observe::*`; (later) `folio::*`, `components` records. 
1. `container.rs`: `pub fn open_fig(bytes:&[u8], &Limits, cancel) -> Result<FigContainer, FigError>`: detects ZIP (`PK\x03\x04`) vs bare prelude; ZIP limits (entries, per-entry/total inflated, ratio, name length), duplicate/case-fold-duplicate/traversal names (same hostile-input rules as the package brief), then `canvas.fig` prelude (`FigKind::{Design,Jam,Deck,Unknown(8 bytes)}`), version u32, chunk table with checked lengths (a length past EOF/over limit -> `ChunkOverflow`); every ZIP part and every raw chunk kept as `(name_or_index, bytes, sha256)` in `FigContainer.parts` (opaque preservation is the default); `meta.json` parsed only for its top-level keys (bounded) and kept raw.
2. `version_map.rs`: `pub struct VersionPolicy{supported:Vec<(FigKind,u32)>}` (initially built ONLY from versions seen in supplied fixtures; synthetic tests use a test-only entry) and `pub fn check_version(&FigContainer,&VersionPolicy)->Result<(),FigError::UnsupportedVersion{kind,found}>`; unsupported versions return the container as opaque-preserved (no decode attempt, no partial import) - the explicit "reject unsupported versions" requirement.
3. Schema + census (still not editable): decompress the schema chunk (raw deflate, output cap) and parse the Kiwi schema definition table (names/kinds/fields only); if the message chunk decodes within limits, produce `Census{node_type_histogram:BTreeMap<String,u32>, node_count:u32, root_ids:Vec<SourceId>}` with `SourceId{session:u32, local:u32}` formatted `"s:l"` - ids are preserved verbatim, never reminted. `FigError::DecoderUnavailable{stage}` when zstd/kiwi decode is not available in this build (explicit, never a partial census). Output `FigImport{kind, version, parts_sha256, schema_summary:Option<..>, census:Option<Census>, loss:Vec<LossEntry{part, kind:PreservedOpaque|Unsupported, code}>, editable:EditableStatus::NotProven}` - `editable` is a constant `NotProven` until original-produced semantic/render proof exists (`semantic_decode.rs` exports `pub fn decode_semantic(..) -> Err(FigError::EditableDecoderBlocked{reason:"NOT_PROVEN: needs original-produced semantic/render proof"})`).
4. Identity + sanitized policy (`stale_import_and_sanitized_policy`): `FigImport.source_sha256` bound to the input; `pub fn sanitized_summary(&FigImport, &SanitizePolicy)->Vec<PartName>` lists which parts a sanitized export would exclude (thumbnail, images, meta) vs a preservation export keeps all (STU-IO-188); stale `expected_sha256` -> `StaleSource`; pre-cancelled -> `Canceled`; no write path in this slice.
5. Observability: `DESCRIPTOR` + `report(...)`: `UnsupportedVersion|EditableDecoderBlocked|DecoderUnavailable` -> `Failure(Unavailable)` (distinct from validation); hostile/overflow/duplicate/bomb -> `Failure(Validation)`; Ok import -> `Success` with `editable: NotProven` visible in the receipt; `Canceled` -> `Canceled`. Example `interop-fig-consumer --inspect FILE.fig`.
Oracle note: not on the operator's Adobe list; original-app oracle = Figma-authored local files (black-box observation of file structure and Figma's own rendering; public docs/community format notes only; no decompilation).

</topic>

<topic id="deps" status="active" summary="third-party deps (verified 2026-10-11)">

## Deps
- `zip =8.6.0` (`default-features=false, features=["deflate-flate2"]`; same pin as package/interop-layout) and `flate2` (via zip; for raw deflate use `flate2::read::DeflateDecoder` with `rust_backend`/miniz_oxide; flate2 1.1.10 MIT OR Apache-2.0 2026-08-28 verified).
- `kiwi-schema 0.2.1` (evanw/kiwi Rust helper; MIT; last release 2023-09-03; 1.57M downloads; pure Rust; verified on crates.io) for schema parse + generic value decode - or hand-written Kiwi reader if its API is too limited (Kiwi format is small); UNVERIFIED that 0.2.1 decodes arbitrary schema-driven messages without a generated type.
- `ruzstd` for the zstd message chunk (Craft vectorcraft pin `0.9`, `default-features=false, features=["std"]`; currency/licence UNVERIFIED - verify on crates.io before the dep-request; decode-only, pure Rust). Without it the zstd chunk path returns `DecoderUnavailable` (explicit).
- `sha2 =0.10.9` `default-features=false, features=["force-soft"]` (workspace pin). `serde_json` only if `meta.json` key listing needs it (`=1.0.151` pin exists in folio).
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-interop-fig --tests` (then `clippy`, `test --test fig_import`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/fig_import.rs">

## Tests (<=5, MT case names; synthetic fixtures built in-test: ZIP with `canvas.fig` = `fig-kiwi` + version + deflate-raw schema chunk + message chunk, plus `meta.json`, `images/ab12`)
1. `fig_container_bounded_identity_loss`: synthetic container opens; kind/version/parts/sha256 exact; unknown extra part preserved byte-equal with `PreservedOpaque` loss; `editable == NotProven`; census node ids round-trip verbatim as `"s:l"`.
2. `unsupported_version_duplicate_key_bomb_cancel`: version not in policy -> `UnsupportedVersion` with the container returned opaque (no decode); duplicate ZIP name, traversal name, inflate bomb, chunk length past EOF, entry count over limit -> distinct codes; pre-cancelled -> `Canceled`.
3. `actual_fig_instance_variable_prototype_edit_reopen`: with NO real fixture supplied this test asserts `decode_semantic` returns `EditableDecoderBlocked` (NOT_PROVEN, not PASS-by-skip); when the lead drops a Figma-authored file at `tests/fixtures/*.fig` the test must instead assert the census and record the real version in `version_map.rs` (and stay NOT_PROVEN for editability).
4. `stale_import_and_sanitized_policy`: wrong `expected_sha256` -> `StaleSource`; `sanitized_summary` excludes thumbnail/images/meta while the preservation policy keeps all parts.
5. `legacy_bare_and_foreign_kinds`: bare `canvas.fig` (no ZIP), `fig-jam.` and `fig-deck` preludes classified; unknown 8-byte prelude -> `FigKind::Unknown` + `UnsupportedVersion`/`NotFig` (never guessed).

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No node-to-Folio/Components mapping, no variants/variables/modes/auto-layout/prototype semantics, no rendering, no writer/round-trip export, no image decoding, no fonts, no Figma API/cloud/plugins, no `.jam/.deck/.buzz/.site/.make` semantics beyond prelude classification, no claim of editable support.
- Report READY_FOR_VALIDATION only; never self-PASS. Keep `editable: NotProven` visible in every receipt.

</topic>
