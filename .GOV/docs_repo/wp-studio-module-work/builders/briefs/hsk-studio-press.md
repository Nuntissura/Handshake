---
file_id: BUILDER-BRIEF-HSK-STUDIO-PRESS
file_kind: builder_brief
updated_at: 2026-10-11T06:20:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-press
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-PRESS` / key `press` / crate `hsk-studio-press`; foundation MT-35557 (test target `pdf_delivery`; cases `pdf_object_edit_save_independent_reopen`, `redacted_text_absent_all_reachable_streams`, `separations_overprint_output_intent`, `signed_readonly_cancel_stale_save_fault`; per-test deadline 60 s); write scope `handshake-studio-assets/crates/hsk-studio-press/**`. Files per MT: `src/pdf_edit.rs`, `src/prepress.rs`, `src/sanitized_save.rs`, `tests/pdf_delivery.rs`.
- Responsibility (verbatim): "Bounded PDF object editing, projection proposals, forms/annotations/signature states, prepress/accessibility/redaction/full-save. PDF bytes are provider transport, never separate live document authority."
- Foundation capability: "Read/edit/save one real PDF object through bounded granted ports, re-read exact output and reject signed/read-only or malformed mutation."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-observe`. Slice uses accord + observe; folio only when projecting a PDF page to a Folio proposal (later). Press is the PDF provider for `interop-vector` (trait defined THERE, implemented by a host adapter - no edge).
- Forbidden: handshake_core/native/SurrealDB; executing PDF JavaScript/actions/launch (never); network fetches; a second live document authority or catalog; an all-PDF engine dependency pulled by default (features stay selectable).
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `press-consumer` (`--descriptor`), tests with `bounded_case` (60 s). Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-PDF-001 (l.~240): separate reversible marking vs irreversible apply-redaction proposals; selective sanitize and full-save disposition incl. signed/read-only states; destructive apply goes through normal proposal/promotion; full reachable-object rewrite and exclusion of prior revisions/source-preservation payloads checked INDEPENDENTLY against text, graphics, metadata, attachments and decompressible bytes - exporter self-verification is insufficient. STU-PDF-002: print/export plans keep output-profile hash/intent, spot/process ink, overprint/separation, reading order; artifact-produced status or approximate preview NEVER proves PDF/X, PDF/UA, plates or tagged order.
- STU-IO-006 PDF rows (l.~21330, row 20 PDF, 26 PDF/X, 27 tagged PDF), STU-IO-125..133 (PDF export presets/standards/fonts), 14.6.9 Prepress and Output (l.6933+; 14.6.9.3 PDF and Ink/Separation Output l.7089), STU-IO-187/188 (atomic last-good, preservation vs sanitized policies), STU-ARC-008/012/013/020 (bounded bytes before decode, release on every exit, cancel/outcome-unknown recovery), STU-ARC-016.
- MT-35557 acceptance (via prep): "Editable saved PDF and per-object/signature/loss/preflight report; separate sanitized versus preservation policy"; gap "typed recipe corruption, exact trap/ink bounds and whole native PDF feature-specific independent oracle need binding".
- Later scope: full redaction (glyph/image/vector/annotation removal), forms/annotations CRUD, PAdES signing/validation, tagged PDF a11y, PDF/X-PDF/A conformance, OCR, compare, optimise, page organise; `_mt_plan_1213_raw_interop.json` MT-459/464/465, MT-188 family (`_mt_plan_06_layout.json`/1213).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (~700-1000 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[8]` (MT-35557 / MT-188; "Bounded PDF object mutation/full-save and ink/overprint/preflight output; sanitize entire reachable object/metadata/attachment/history set, atomically retain last-good on fault"; provider "Pdfcraft redact/cos/edit references; Prism math shared. Provider selection cannot implicitly pull every PDF feature"; oracle "Independent PDF parser/render/text extraction and original Acrobat output"); Craft reference (pinned cache `.../pdfcraft-f626db25.../crates/{cos 2983 LOC: lazy tolerant parser, copy-on-write edits, incremental+full save; redact 2589 (sanitize.rs: selects hidden payload classes, forces full rewrite to discard prior revisions); sign; edit}`; MIT OR Apache-2.0; `cos` depends on hayro-syntax + pdfcraft-crypt/filters = heavier than the slice needs); `.GOV/reference/studio_app_feature_research/11-provider-posture-map.md` (OCR/AI = parked adapters), `19-studio-local-first-rust-posture.md`.
Reuse (verified): `accord::{CancellationToken, ActorContext}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. No existing crate has a PDF type; PDF objects stay inside this crate (provider transport).
1. `pdf_edit.rs`: `pub struct PdfDoc` (wraps the parser; private), `PdfDoc::open(bytes:&[u8], &Limits{max_bytes,max_objects,max_depth}, &CancellationToken) -> Result<PdfDoc, PressError>`; `inspect(&self) -> Inspection{version:String, page_count:u32, object_count:u32, encrypted:bool, permissions_allow_modify:Option<bool>, signature_count:u32, has_byte_range_signature:bool, sha256_hex of input}` (signed = AcroForm `/SigFlags` and/or `/Sig` field with `/ByteRange`; encrypted/permissions from `/Encrypt` `/P`). `pub fn edit_entry(&mut self, target:ObjRef, key:&str, value:PdfScalar, expected_source_sha256:&str, cancel) -> Result<EditReceipt, PressError>` for dictionary objects only (allowlisted keys: Info `/Title /Author /Subject`, page `/Rotate`, annotation `/Contents`); refusals: `SignedDocument` (full rewrite would invalidate the signature; incremental policy is later), `ReadOnly`/`PermissionDenied`, `NoSuchObject`, `UnsupportedObject` (stream), `StaleSource` (hash differs), `KeyNotAllowed`, `Malformed`. Never executes actions/JS.
2. Save + independent reopen: `pub fn save_full(&self, &Limits, cancel) -> Result<Vec<u8>, PressError>` and `pub fn save_to(&self, sink:&mut dyn ArtifactSink, cancel) -> Result<SaveReceipt, PressError>` where `trait ArtifactSink{fn begin(&mut self)->Result<(),SinkError>; fn write(&mut self,&[u8])->Result<(),SinkError>; fn commit(&mut self)->Result<(),SinkError>; fn abort(&mut self);}`: abort on any fault, last-good in the sink untouched (in-memory test sink retains last-good). Test oracle for reopen: the PDF re-parsed by the same crate AND scanned by an independent byte-level scanner in the test (find `/Title (New)` in the new bytes; assert untouched objects' serialisations are byte-equal) - note that same-parser reopen alone is NOT independent evidence (record the independent tool used).
3. `sanitized_save.rs`: `pub fn sanitize_full_rewrite(doc:&PdfDoc, policy:&SanitizePolicy{drop_info_metadata, drop_xmp, drop_embedded_files, drop_javascript, drop_prior_revisions:true}) -> Result<(Vec<u8>, SanitizeReport{removed:Vec<(String,u32)>}), PressError>`: writes ONLY objects reachable from the trailer `/Root` after removals (fresh file, no incremental tail, no orphan objects); `redacted_text_absent_all_reachable_streams` proves a marker string placed in metadata, an attachment and an orphaned old object is absent from the output bytes AND from every decoded stream (decode with the parser's filter support). Content-stream text redaction (glyph removal) is NOT in this slice: report `UnsupportedRedactionClass` for text/image/vector marks instead of claiming success (STU-PDF-001).
4. `prepress.rs` facts only (no conformance claims): `read_prepress(doc) -> PrepressFacts{output_intents:Vec<{subtype, condition_identifier, has_dest_profile}>, uses_separation:bool, uses_devicen:bool, overprint_stroke/fill flags from ExtGState (/OP /op /OPM)}` computed by walking resources; document that PDF/X or PDF/UA status is never inferred from these (STU-PDF-002).
5. Observability: `DESCRIPTOR` + `report(...)`: `SignedDocument|ReadOnly|PermissionDenied` -> `Failure(Unsupported)`; `Malformed|StaleSource|KeyNotAllowed|NoSuchObject` -> `Failure(Validation)`; sink fault -> `Failure(Unavailable)`; `Canceled` -> `Canceled`. Example `press-consumer --inspect FILE.pdf | --set-title NEW FILE.pdf OUT.pdf`.
Oracle note: not on the operator's Adobe list, but the PDF rule is the same: later check against Acrobat-saved fixtures (Acrobat DC installed): signed/certified docs, permissions, incremental vs full save, output intents; never treat our own re-read as the oracle.

</topic>

<topic id="deps" status="active" summary="third-party deps (verified 2026-10-11)">

## Deps
- `lopdf =0.45.0` (J-F-Liu; MIT; 2026-09-08; 23.6M downloads; pure Rust). Use `default-features=false` (defaults are `chrono-clock`,`rayon`); required feature names for parse/save/decompress/encryption on 0.45.0 are UNVERIFIED - read docs.rs/lopdf/0.45.0/features. Its incremental-save and object-pruning support is UNVERIFIED; if it cannot give "reachable-only full rewrite", write the reachable set yourself (walk from `/Root`, renumber) rather than relying on a library flag.
- Fallback if lopdf blocks (2 distinct failed attempts): vendor the Craft `pdfcraft-cos` subset with pinned-hash provenance (STU-ARC-018, same `third-party/` convention as `hsk-studio-type`) - heavier (hayro-syntax, crypt, filters).
- Not now: `hayro`/`hayro-syntax` 0.7.x (render/parse, Craft pins), `krilla` 0.8 (PDF creation), `rsa`/X.509 stack (signing), `weezl`. SHA-256 for fingerprints: `sha2 =0.10.9` `default-features=false, features=["force-soft"]` (same pin as nib/type).
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-press --tests` (then `clippy`, `test --test pdf_delivery`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/pdf_delivery.rs">

## Tests (<=5, MT case names; build tiny PDFs in-test from literal bytes - a 1-page PDF with Info, one annotation, optionally a signature dictionary shell)
1. `pdf_object_edit_save_independent_reopen`: edit `/Title`, save, independent byte scan finds the new value, old value gone, all other object serialisations byte-identical, page count unchanged.
2. `redacted_text_absent_all_reachable_streams`: marker in Info, XMP, an embedded file and an unreachable old object; `sanitize_full_rewrite` output contains the marker in NO raw byte range and NO decoded stream; `UnsupportedRedactionClass` for a content-stream text mark.
3. `separations_overprint_output_intent`: PDF with `/OutputIntents`, a `/Separation` colour space and `/OP true`; `read_prepress` reports exactly those facts; a PDF without them reports none; no conformance claim field exists.
4. `signed_readonly_cancel_stale_save_fault`: signed shell -> `SignedDocument`; `/Encrypt` with modify bit clear -> `PermissionDenied`; wrong `expected_source_sha256` -> `StaleSource`; pre-cancelled -> `Canceled`; sink fault on commit -> last-good bytes in the sink unchanged and `abort` called.
5. `hostile_pdf_bounds`: truncated xref, cyclic page tree, object count over limit, > `max_bytes` input -> typed errors, no panic, no unbounded allocation, no action executed.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No rendering, no content-stream editing/glyph redaction, no forms/annotation authoring, no signing/validation, no incremental-save policy, no PDF/X-PDF/A-PDF/UA conformance, no OCR/compare/optimise, no Folio projection yet (gap: define the page-object -> Folio proposal mapping with the lead first), no host/ArtifactService wiring, no GUI.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
