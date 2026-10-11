---
file_id: BUILDER-BRIEF-HSK-STUDIO-INTEROP-LRCAT
file_kind: builder_brief
updated_at: 2026-10-11T04:00:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-lrcat
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-INTEROP_LRCAT` / key `interop_lrcat` / crate `hsk-studio-interop-lrcat`; foundation MT `MT-35562` (read `execution.implementation_steps` 1-4: planned files `src/catalog_bytes.rs`, `src/wal_snapshot.rs`, `tests/catalog_decode.rs`; planned test names `real_catalog_wal_fixed_entity_reconciliation`, `overflow_cycle_ungranted_wal_and_changed_source`, `process_enable_smart_rule_unknown_loss`, `cancel_stale_no_source_write_no_sql_closure`). Write scope `crates/hsk-studio-interop-lrcat/**` only.
- Responsibility (verbatim): "Internal bounded read-only Lightroom .lrcat file-byte decoder, fixed Lightroom semantic extraction only; explicitly granted stable main/WAL handles. No engine, query API, writable catalog or DB. Outputs CKC/Studio proposals through shared ports."
- Foundation capability: "Decode a real supported catalog/WAL slice with stable fingerprint and entity/loss reconciliation; reject malformed cycles/overflow/ungranted WAL and leave source bytes unchanged."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-observe` (folio unused in slice 1).
- Forbidden (hard, CX-503R / STU-SDB-002): ANY SQL/database engine or `rusqlite`/`libsqlite3-sys`/`sqlx`/`turso`-class crate, SQL query API, writable catalog, repair, private cache/DB/catalog, treating decoded rows as authority; `-sys` crates; reading the user's real catalogs without explicit Operator grant (privacy - use generated fixtures); edges outside `allowed`. The ONLY exception is bounded read-only decoding of Operator-selected external `.lrcat` bytes (STU-SDB-002).
- Conventions (verified): edition 2024, rust-version 1.97.1, single-line inline `Cargo.toml` tables (`builders/WORKSPACE_RULES.md`), `[lib] path="src/lib.rs"`, `[[example]] name="interop-lrcat-consumer"`, `DESCRIPTOR`, tests in `tests/catalog_decode.rs` (MT-named) with a `bounded_case` wrapper.
- Prep refs: `docs_repo/studio-collaboration-2026-10-09/modular-authoring/autonomous-research-team/research.json` finding `INT-LRCAT-001` (extract only internal bounded page/WAL primitives + fixed field extraction; public entrypoint takes explicitly granted stable main/WAL handles); `backend-redteam/independent-adoption-redteam.json` ADOPT-CHECK-003 ("read-only input does not mean read-only importer authority": the LightCraft endpoint mutates a Session - do not copy that). Reference source (MIT OR Apache-2.0; read as CONCEPT reference only, write your own code from the public SQLite file-format doc https://www.sqlite.org/fileformat.html; MT-35562 says the decoder is "not adopted from any SQL catalog implementation"; record any copied snippet in `SOURCE_PROVENANCE.json`): `D:/Projects/LLM projects/Handshake/Handshake Worktrees/gov_runtime/tools/studio-craft-source-cache/lightcraft-88ea1a804b35709e1fe436e3792adf0428b97daa/crates/engine/src/{lightroom_sqlite.rs (1148 LOC clean-room page reader: header validation, WAL overlay, depth/row/cell/overflow caps, CREATE TABLE tokenizer), lightroom_catalog.rs (1306 LOC: committed-WAL-prefix checksum at 301-337, source-changed-during-read check at 347-366, fixed table list)}`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-IO-189 (l.235): Lightroom catalog/develop/WAL provenance needs versioned per-entity mapping/loss outcomes; unsupported process-version/Enable flags, smart rules, archive entities, plugin/effect graphs MUST be explicitly retained/translated/approximate/unsupported/missing; "successful parser/count is not full conversion"; the `.lrcat` decoder public API is fixed-format semantic extraction from bounded immutable granted bytes, NEVER a generic table/query/database API. STU-SDB-002 (l.36): sole exception to the no-SQLite rule; decoder must not link/invoke a SQL engine, expose SQL queries or connectivity, write/repair the catalog, or create a private DB/cache/catalog; emits typed import proposals + provenance/loss receipts; accepted data enters CKC/Studio via ResourceBroker, sandbox/validation/promotion.
- STU-IO-186/187 (l.211/231): explicit editable mapping and opaque preservation; container safety (bounded, duplicate/path/cycle/overflow checks). STU-IO-012 (l.21418): fail closed, never a silent partial. STU-ARC-012 (l.219): bound producers/bytes before allocation; release reservations on every exit. STU-ARC-015: typed loss/failure.
- Later: STU-IO-006 matrix row for Lightroom catalog (develop mapping into `hsk.studio.raw_develop@1`, accord `SCHEMA_STUDIO_RAW_DEVELOP`), develop/RAW feature MTs (`_mt_plan_1213_raw_interop.json`, domain 12/13), CKC catalog promotion via ports.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only; ~1000-1400 LOC)
Reuse (verified): `hsk_studio_accord::{CancellationToken, DomainId, ActorContext}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. No Folio use yet (proposal types local until the CKC/Studio port exists).
1. Granted immutable byte sources: `pub trait ByteSource{ fn len(&self)->u64; fn read_at(&self, offset:u64, buf:&mut [u8]) -> Result<(), SourceError>; }` + `pub struct Grant{ main: Box<dyn ByteSource>, wal: WalGrant }`, `enum WalGrant{ None, Granted(Box<dyn ByteSource>), PresentButNotGranted }` (the caller states honestly whether a `-wal` sibling exists; `PresentButNotGranted` => `Err(UngrantedWal)` because the catalog could be stale). `SliceSource(Vec<u8>)` for tests. `Limits{max_main_bytes (<= 1<<30), max_wal_bytes, max_pages, max_rows, max_cells, max_payload, max_overflow_pages, max_btree_depth (64), max_columns}`; every limit checked BEFORE allocation.
2. Bounded read-only SQLite page decoder (INTERNAL module `catalog_bytes`, nothing generic is `pub`): validate `SQLite format 3\0` header (page size 512..=65536 power of two or 1=65536, usable size, text encoding 1/2/3, page count vs length), walk table b-trees (interior `0x05`, leaf `0x0d`) iteratively with depth cap and a visited-page set (`Err(Cycle)`), decode varints and records with checked arithmetic, follow overflow chains with a page cap and cycle detection, locate `sqlite_master` and parse `CREATE TABLE` column names with a tiny bounded tokenizer (needed to address columns by name). Payload sizes, rowid ranges, page numbers beyond the file => typed `Overflow/Corrupt`. Never writes, never allocates beyond limits.
3. WAL snapshot (`wal_snapshot`): parse WAL header (magic `0x377f0682/3`, version 3007000, page size must equal main, header checksum, salts), accept ONLY frames up to the last valid commit frame (frame checksum chain; stop at first mismatch/salt change), build page overlay (latest committed version per page). `Fingerprint{main_len, wal_len, committed_wal_len, sha256_main, sha256_wal}` computed from the granted bytes; `extract` takes the fingerprint before and after reading and returns `Err(SourceChanged)` if either differs (stable-handle check; "changed source").
4. Fixed Lightroom semantic extraction (the ONLY public entry): `pub fn extract_catalog(grant:&Grant, limits:&Limits, cancel:&CancellationToken) -> Result<CatalogProposal, LrcatError>` reading exactly these tables by name, each optional-with-loss unless noted (names observed in the LightCraft reference; verify against a Lightroom-generated catalog, UNVERIFIED for current LR versions): `AgLibraryRootFolder`, `AgLibraryFolder`, `AgLibraryFile`, `Adobe_images` (required), `AgLibraryCollection`, `AgLibraryCollectionImage`, `AgLibraryKeyword`, `AgLibraryKeywordImage`, `Adobe_imageDevelopSettings`, `Adobe_AdditionalMetadata`, plus develop-history tables as `Unsupported` losses. `CatalogProposal{fingerprint, schema_version:Option<String>, photos:Vec<PhotoProposal{local_id:i64, root_path, path_from_root, base_name, extension, rating:Option<i64>, capture_time:Option<String>, orientation:Option<String>}>, collections, keywords, develop:Vec<DevelopProposal{local_id, process_version:Option<String>, settings_text:Opaque(Vec<u8>) , decoded:BTreeMap<String,String> (simple scalar fields only)}>, losses:Vec<LossNote{entity, field, disposition: Editable|Translated|Approximate|Opaque|Unsupported|Missing, code}>, counts:EntityCounts}`. Smart-collection rules, process-version/Enable flags that the extractor does not map, archive entities, compressed XMP (inflate not in slice 1) => explicit `Unsupported/Opaque` loss entries - never silently dropped; counts of rows read vs mapped reconcile (`rows_seen == mapped + lost`). No partial publication: any error returns no proposal and the sources are untouched (borrow-only).
5. Observability: `DESCRIPTOR` JSON (public API is fixed extraction only; states "no SQL engine, no query API") + `report(result, cancel, &mut Observe, &mut impl SinkPort)` (Ok -> Success even with losses; `UngrantedWal/SourceChanged` -> Failure(Unavailable); `Corrupt/Overflow/Cycle` -> Failure(Validation); Canceled -> Canceled) and example `interop-lrcat-consumer` (`--descriptor`, `--main FILE [--wal FILE | --wal-present-ungranted]`) printing counts + loss summary; opens files read-only, never writes.
Fixtures: unit tests build tiny valid SQLite images by hand (page header, varint, record, leaf; WAL with valid checksums) like the LightCraft test helpers do; the INDEPENDENT oracle (validator-time) is a catalog produced by installed Lightroom Classic (new small catalog: 3-5 generated images, ratings, one keyword, one collection, one smart collection, one develop edit) compared with a fixed expected entity/develop export (Lightroom's own XMP/metadata output, black-box only; no decompilation) and an independent source byte re-read hash before/after.

</topic>

<topic id="deps" status="active" summary="third-party deps (crates.io API 2026-10-11)">

## Deps
- Slice 1: `sha2 0.10.9` (`default-features=false`, `features=["force-soft"]`; pure-rust; already workspace-locked for nib/type) for fingerprints - dep-request + probe `approved` extension for `hsk-studio-interop-lrcat`. Everything else std.
- Later only (dep-request): inflate for compressed XMP: `miniz_oxide 0.9.1` (MIT OR Zlib OR Apache-2.0, dep adler2, pure-rust) with an output cap; `serde`/`serde_json` (pinned) only if proposals need JSON.
- Rejected: `rusqlite`, `libsqlite3-sys`, `sqlx`, `sqlite`/`turso`/`limbo`-style engines, `limbo`, any crate exposing SQL execution; `zip`/`flate2` default features that drag C backends.
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-interop-lrcat --tests` (then clippy, `cargo test -p hsk-studio-interop-lrcat --test catalog_decode`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests (MT-35562 names first)">

## Tests (<=5, `tests/catalog_decode.rs`, `bounded_case`, hand-built byte fixtures, no real catalog needed for CI)
1. `real_catalog_wal_fixed_entity_reconciliation`: hand-built main image (+ committed WAL overriding one page) with 2 photos, 1 collection, 1 keyword: assert extracted entities equal the fixed expectation, WAL-overlaid value wins, counts reconcile (`seen == mapped + lost`), fingerprint stable across two reads and equals independent SHA-256 of the input bytes.
2. `overflow_cycle_ungranted_wal_and_changed_source`: b-tree interior pointing back to an ancestor => `Cycle`; overflow chain loop/page beyond file => `Overflow/Corrupt`; huge declared payload / over limits rejected before allocation; `WalGrant::PresentButNotGranted` => `UngrantedWal`; a `ByteSource` whose bytes change between fingerprint reads => `SourceChanged`; uncommitted trailing WAL frames ignored.
3. `process_enable_smart_rule_unknown_loss`: develop row with unknown process version, an Enable-style flag, a smart-collection row and an unknown table => `Unsupported/Opaque` loss entries with entity+field, never counted as mapped; unsupported schema version noted.
4. `cancel_stale_no_source_write_no_sql_closure`: pre-canceled token and cancel mid-walk => `Canceled`, no proposal; source bytes byte-identical before/after (hash); a test asserts the crate's resolved dependencies (parse `Cargo.toml` text) contain no SQL/database engine names and that no `pub` item exposes a generic table/query API (compile-visible API list in `DESCRIPTOR`).
5. `report_and_descriptor`: `Vec`-backed `SinkPort`; Success-with-losses, UngrantedWal and Canceled each emit exactly one terminal frame; `DESCRIPTOR` parses and states "fixed extraction only".

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No SQL parsing beyond `CREATE TABLE` column names, no index/page-cache/generic table reader, no write/repair/vacuum, no catalog or cache of decoded rows, no full develop-settings semantics (Lua text stays opaque except simple scalars), no XMP inflate in slice 1, no Folio/CKC promotion (shared ports later), no Lightroom CC cloud catalogs, no use of any user's real catalog without explicit grant.
- Wave: 1 (accord + observe; sha2 dep-request). Report READY_FOR_VALIDATION only; never self-PASS; "parser success is not full conversion" must stay visible in the receipt.

</topic>
