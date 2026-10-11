---
file_id: BUILDER-BRIEF-HSK-STUDIO-PACKAGE
file_kind: builder_brief
updated_at: 2026-10-11T03:10:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-package
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-PACKAGE` / key `package` / crate `hsk-studio-package`; write scope `handshake-studio-assets/crates/hsk-studio-package/**` only.
- Responsibility (verbatim): "Native document container serialization, manifest/hash references, atomic recovery and bounded loading. Stream asset bytes through CKC/PRIM-ArtifactService provider ports; no private catalog, preview cache, metadata store, watcher or second CAS under STU-ASSET-004/009/010. Standalone fixtures supply scoped in-memory resolver inputs rather than kernel imports."
- Foundation capability: "Save/reopen actual mixed graph/bytes through bounded granted resolver with checksums and hostile input rejection."
- Allowed deps (STUDIO_MODULES depends_on == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-observe`. Forbidden: handshake_core/handshake_native/SurrealDB (ENG-DAG-FORBID-001), second DB/catalog/CAS (ENG-DAG-FORBID-003, shared_owners.catalog), any edge not in `allowed` (no prism/pigment/type here: asset bytes are opaque to package).
- Conventions (copy from existing crates): edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-accord", version="=0.1.0", default-features=false }`, exact `=` pins for third-party, `pub const DESCRIPTOR: &str` JSON, example `package-consumer` with `--descriptor`, tests in `tests/foundation_contract.rs` with a 30 s `bounded_case` wrapper (see folio tests). Root `Cargo.toml` is steward-owned; dep needs go to `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-IO-001 (l.21275): `.handshake` = versioned portable snapshot/container; bounded streaming ZIP64 carrying typed manifest, Studio graph, content-hash asset references, optional granted packed payloads, previews, preservation records; transport only, never a second database/authority; copy-first crash-safe publication; serialized grants never confer authority; unknown semantics kept with explicit loss receipts.
- STU-IO-187 (l.231): ZIP64 with bounded index/entry/count/compressed/inflated/aggregate limits; duplicate-name/key rejection; traversal/cycle/overflow checks; canonical known-field serialization; explicit unknown-version behaviour; unknown records preserved verbatim in quarantine, never executed; save = validate complete temp artifact then atomic replace, retain last-good, never delete destination before rename.
- STU-IO-190 (l.238): `.handshake` is not a grant/live DB; default import allocates fresh IDs (slice: report only, do not remap); divergent state is an import proposal, not an overwrite.
- STU-IO-004/005: loss receipt separates preserved-opaque vs semantically imported. STU-ARC-007/016: foundation = narrow real bounded capability + real consumer + failure path; no hollow registries. STU-ARC-012/020: bounded allocation before decode; release on every exit.
- Later scope: STU-IO-188 (sanitized delivery vs preservation export policies), STU-IO-190 identity remap/reopen reconcile, STU-ASSET-004/005/009/010/012 (CKC placed-asset links), STU-ARC-009 mirror provenance. MT plan domain: `_mt_plan_1213_raw_interop.json` (MT-441..445, MT-467 package/collect), `_mt_plan_00_foundation.json` (MT-002).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, ~700-1000 LOC: `src/{lib,limits,manifest,write,read,save}.rs`)
Reuse (verified by grep): `hsk_studio_folio::{Snapshot (encoded_bytes()), Inspection::{Editable,ReadOnly}, inspect_bytes(&[u8], Budget, &CancellationToken, &dyn Resolver), Budget, Resolver, Unresolved, TileRef, ContentDigest{algorithm,digest}, Diagnostic, Code}`; `hsk_studio_accord::{CancellationToken (cancel/check), DomainId}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode::{Validation,Loss,Unsupported,Unavailable}, SinkPort}`.
1. Bounded limits + hostile ZIP preflight: `pub struct Limits{max_entries:u32, max_entry_bytes:u64, max_total_inflated:u64, max_ratio:u32, max_name_len:u16}` (+`Default`); `pub fn preflight(bytes:&[u8], &Limits, &CancellationToken) -> Result<Vec<EntryMeta>, PackageError>` rejecting: entries > max, any declared/actual size over limit, ratio bomb, duplicate names (case-sensitive and case-folded), names where `enclosed_name()` is `None` (traversal/absolute/`..`), `has_overlapping_files()`, and unsupported compression. Never allocates more than the declared cap before checking it; read via `.take(max+1)`.
2. Manifest + writer: `pub struct Manifest{container_version:u32, document_id:String, revision:u64, entries:Vec<ManifestEntry>}`, `ManifestEntry{name:String, kind:EntryKind, sha256_hex:String, len:u64}`, `enum EntryKind{Document,Asset,Preview,Opaque}`; manifest serialized canonically (field order fixed, entries sorted by name, `deny_unknown_fields` on read). `pub fn write_package(doc:&Snapshot, assets:&dyn AssetSource, limits:&Limits, cancel:&CancellationToken, out:&mut impl Write) -> Result<WriteReceipt, PackageError>` writes `manifest.json`, `document.json` (= `doc.encoded_bytes()` verbatim), `assets/<sha256hex>` for each `TileRef.content_digest` reachable from `doc.document().layers[*].payload` (`Payload::Raster{tiles}`); `trait AssetSource{fn read(&self, digest:&ContentDigest, max:u64)->Result<Vec<u8>, AssetError>}` (granted in-memory resolver for standalone fixtures; `AssetError::{Absent,Unauthorized,TooLarge}`). Missing asset = typed error, never silent skip. `WriteReceipt{entries, bytes_written, doc_revision}`.
3. Reader: `pub fn read_package(bytes:&[u8], limits:&Limits, folio_budget:folio::Budget, resolver:&dyn folio::Resolver, cancel:&CancellationToken) -> Result<Opened, PackageError>`; `Opened{inspection:folio::Inspection, assets:BTreeMap<String,Vec<u8>>, quarantined:Vec<Quarantined{name:String, bytes:Vec<u8>, sha256_hex:String}>, loss:Vec<LossNote{name:String, reason:LossReason}>}`. Verifies every manifest hash/len against actual bytes (`HashMismatch`), rejects unknown `container_version` with `UnsupportedVersion` (keep bytes as `ReadOnly`-style outcome, no decode), routes unknown entries to `quarantined` verbatim with a `LossNote` (STU-IO-187), parses `document.json` through `folio::inspect_bytes` (so folio duplicate-field/unknown-schema logic is reused, not copied).
4. Atomic save: `pub fn save_atomic(target:&Path, package:&[u8], limits:&Limits, cancel:&CancellationToken) -> Result<(), PackageError>`: write `<target>.tmp-<n>` in the same dir, `sync_all`, re-run `preflight` + hash check on the temp bytes read back, then `std::fs::rename` over target (std replaces on Windows). On any failure remove only the temp file and leave the existing target byte-identical (last-good retained). Never delete target first.
5. Observability + consumer: `pub const DESCRIPTOR: &str` JSON; `pub fn report(result:&Result<..,PackageError>, cancel, obs:&mut Observe, sink:&mut impl SinkPort)` mapping Ok->`Outcome::Success`, `Canceled`->`Outcome::Canceled`, hostile/hash/version -> `Failure(Validation)`, absent/unauthorized asset -> `Failure(Unavailable)`, quarantined content -> still Success but caller reads `loss` (use `Failure(Loss)` only if policy `loss_is_error`). Example `package-consumer`: `--descriptor`, `--write DOC.json OUT.handshake`, `--read IN.handshake`.
Oracle for roundtrip: `read(write(x)).inspection` Editable and `encoded_bytes()` byte-equal to the input snapshot; asset bytes byte-equal; a second write of the opened result is byte-identical (deterministic container: fixed entry order, fixed timestamps, no extra fields).

</topic>

<topic id="deps" status="active" summary="third-party deps (verified 2026-10-11)">

## Deps (all pure Rust; builder files a dep-request per crate, versions are exact pins)
- `zip` MIT, repo zip-rs/zip2, 295M downloads. Latest 9.0.1 published 2026-10-10 and 9.0.0 on 2026-10-09 (too fresh, major bump) -> recommend `=8.6.0` (2026-04-25). Use `default-features=false, features=["deflate-flate2"]` (feature name confirmed on docs.rs for 8.6.0; flate2 default backend `miniz_oxide` is pure Rust; do NOT enable zlib/zlib-ng/zstd/bzip2/lzma/aes). Reader hostile-input surface confirmed: `ZipArchive::new`, `len`, `by_index_raw`, `file_names`, `has_overlapping_files`; duplicate-name rejection is NOT documented -> package must check itself (UNVERIFIED zip behaviour). Whether `Stored` works with `default-features=false` and no deflate feature: UNVERIFIED (docs silent) - keep `deflate-flate2`.
- `sha2 =0.10.9` `default-features=false, features=["force-soft"]` (same pin as nib/type, avoids a third sha2 lock entry; prism uses 0.11.0). pure-rust yes.
- `serde =1.0.229` (`std`,`derive`) and `serde_json =1.0.151` (`std`) as pinned in folio/chronicle; pure-rust yes.
- Rejected: own ZIP64 writer/reader (more code, same bounds checks needed), `rc-zip` (not evaluated, UNVERIFIED), tar/7z (spec says ZIP64).
- Build/test (cargo form mandatory): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-package --tests` (then `clippy`, `test`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/foundation_contract.rs">

## Tests (<=5, `bounded_case`)
Foundation MT-35547 proof target (verified in the MT json): `cargo test -p hsk-studio-package --no-default-features --test foundation_contract` (per-test timeout 120 s in prep `shared-team/preparation.json` package row); MT also lists negative cases to cover when cheap: same-ID-different-content collision and revoked link (deferred to the resolver port, see non-goals), plus "twice-roundtrip edited container". Prep refs: shared-team `representative_authored_field_proposals` module `package`; `.GOV/reference/studio_app_feature_research/19-studio-local-first-rust-posture.md` (no new interchange format invented beyond the spec's `.handshake`).
1. `mixed_graph_roundtrip_with_assets`: use folio's `mixed()`-style doc JSON (copy the helper from `crates/hsk-studio-folio/tests/foundation_contract.rs`) plus one tile asset; write->read; Editable, document bytes equal, asset bytes equal, second write byte-identical.
2. `hostile_zip_rejected`: hand-built zips: `../evil` name, absolute path, duplicate name, case-fold duplicate, entry declaring > `max_entry_bytes`, deflate bomb over `max_ratio`, > `max_entries`; each yields a distinct `PackageError::code()`; no panic, bounded memory.
3. `manifest_integrity_and_version`: flipped asset byte -> `HashMismatch`; manifest `len` lie -> reject; `container_version: 2` -> `UnsupportedVersion` without decoding; missing asset in source -> `AssetError::Absent` surfaced, no partial output.
4. `unknown_record_quarantined_verbatim`: extra `opaque/x.bin` entry survives read as `quarantined` with identical bytes and a `LossNote`; never interpreted.
5. `atomic_save_retains_last_good_and_cancel`: corrupt temp (inject failing validation) leaves target byte-identical and no temp left; success replaces; pre-cancelled token -> `Canceled`, nothing written.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No ArtifactService/CKC/EventLedger wiring (host-side, later), no catalog/preview cache/watcher, no packed payload grants, no sanitized-delivery policy (STU-IO-188), no ID remap on import (STU-IO-190), no previews/thumbnails/WAL, no encryption, no streaming from `Read` (slice is `&[u8]`/`Write`; streaming ZIP64 + `Seek`-less sink is the next capability), no >4 GiB tests (ZIP64 record parsing only unit-tested by header), no GUI.
- Package is NOT a vendor-format reader: PSD/AI/IDML/FIG belong to interop-* crates.
- Report READY_FOR_VALIDATION only; never self-PASS (CX-PROOF-002).

</topic>
