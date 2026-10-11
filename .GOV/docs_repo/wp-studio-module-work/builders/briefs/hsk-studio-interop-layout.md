---
file_id: BUILDER-BRIEF-HSK-STUDIO-INTEROP-LAYOUT
file_kind: builder_brief
updated_at: 2026-10-11T06:40:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-interop-layout
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-INTEROP_LAYOUT` / key `interop_layout` / crate `hsk-studio-interop-layout`; foundation MT-35551 (test target `layout_edit_roundtrip`; cases `idml_threaded_story_style_edit_reopen_twice`, `zip_traversal_duplicate_inflate_xml_entity`, `indd_actual_native_editability_or_blocker`, `rtl_cjk_overset_loss_cancel_stale`; deadline 60 s); write scope `handshake-studio-assets/crates/hsk-studio-interop-layout/**`. Files per MT: `src/idml_decode.rs`, `src/idml_encode.rs`, `src/indd_adapter.rs`, `tests/layout_edit_roundtrip.rs`.
- Responsibility (verbatim): "IDML/INDD/print interchange conversion; no claim INDD readable without actual decoder proof."
- Foundation capability: "Execute actual supported bounded IDML semantic decode/encode with loss and shared layout; missing INDD decoder remains separately blocked full-feature scope."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-layout`, `hsk-studio-type`, `hsk-studio-prism`, `hsk-studio-observe`. The foundation needs the `layout` crate (thread/frame types) -> start after `hsk-studio-layout` lands; steps 1-3 below need only accord/folio/observe and can begin earlier.
- Forbidden: handshake_core/native/SurrealDB; network/file access from XML (entities, external refs); claiming INDD readable (STU-IO-186); a private layout/story store; second shaper (Type owns shaping).
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `interop-layout-consumer` (`--descriptor`), `bounded_case` (60 s). Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-IO-006 rows 24-25 (l.~21330): IDML (NRT interchange) and INDD (NRT native layout binary) with preservation-blob family law STU-IO-008 (Layout: `.idml` / `.indd`); STU-IO-004 five-artifact gate; STU-IO-005 unknown structure kept opaque + receipt; STU-IO-186 (l.211): INDD binary import is NOT satisfied by IDML; per-object/property editable mapping, preserved opaque data, unsupported/loss decisions, round-trip disposition required; missing decoder = blocking scope gap; STU-IO-187 (container safety: bounded index/entry/count/inflated, duplicate-name rejection, traversal checks; unknown parts quarantined verbatim, atomic last-good save).
- STU-LAY-017/018/122 (l.5727-5810): story threading chain (`PreviousTextFrame`/`NextTextFrame`), overset, STU-LAY-001..003 spreads/pages (l.5370+, IDML spread origin differs: spine/centre), STU-LAY-126 text wrap; STU-ARC-008/012 bounded admission; STU-ARC-016/019.
- MT-35551 acceptance (via prep): "Editable layout proposals, missing font/link and per-property loss report, validated edited transport with atomic last-good save"; gap "Native versioned editable decoder, multilingual/a11y/prepress preservation and original fonts must bind exact rows".
- Later scope: swatches/graphics/styles (`Resources/*.xml`), master spreads, tables, footnotes, anchored objects, GREP/nested styles, XML structure, EPUB/PDF-X export (MT-464..466), INDD decode (MT-463), `_mt_plan_1213_raw_interop.json` MT-462/463, `_mt_plan_06_layout.json`.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (~900-1200 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[13]` (MT-35551; "Bounded ZIP/XML IDML story/spread/style mapping through Layout/Type; native INDD separate versioned provider; preserve unknown data with explicit policy"; provider "Designcraft IDML quick-xml/zip manifest inspected; full semantic decoder source and INDD provider NOT_INSPECTED/NOT_PROVEN"; oracle "InDesign-produced fixtures and independent original reopen/render/property comparison; IDML success does not satisfy binary INDD"); Craft reference (pinned cache `.../designcraft-66c7ce7e.../crates/idml/src/{import.rs 2449, export.rs 2465, xml.rs, names.rs, cjk.rs, arabic.rs}`; MIT OR Apache-2.0; clean-room from the public IDML spec; its own TODO list says "Unknown elements are ignored; nothing is preserved opaquely for round-trip yet" = the gap this crate must close); `.GOV/reference/studio_app_feature_research/03-indesign-feature-map.md`, `19-studio-local-first-rust-posture.md`.
IDML package facts (Craft doc comment + web search, public spec; verified 2026-10-11): ZIP; first entry `mimetype` STORED = `application/vnd.adobe.indesign-idml-package`; `META-INF/container.xml` -> root `designmap.xml`; parts `Resources/{Graphic,Fonts,Styles,Preferences}.xml`, `MasterSpreads/`, `Spreads/Spread_<id>.xml` (geometry), `Stories/Story_<id>.xml` (text in `Content`), `XML/`; spread coordinates originate at the binding spine/spread centre, pages placed by `ItemTransform`.
Reuse (verified): `folio::{StudioPageSpread{page_spread_id,name,parent,bounds}, StudioLayer, Payload::Text{story_id}, Payload::Unsupported, validate_document, Budget}`; `layout::{Frame, ShapedParagraph, reflow...}` (from the layout brief, once landed) for thread validity; `accord::{DomainId (SPGS, SLYR, STXT valid), Length/Unit::Points, CancellationToken}`; `observe::*`.
1. Safe package open (`idml_decode.rs`): `pub fn open_idml(bytes:&[u8], &Limits, cancel) -> Result<IdmlPackage, IdmlError>`; checks mimetype is first, stored, exact string; entry count/size/ratio limits, duplicate and case-fold duplicate names, traversal/absolute names, `designmap.xml` present; EVERY part kept as `(name, bytes, sha256)` in `IdmlPackage.parts` (preservation of unknown parts is the default, STU-IO-005). XML reading with `roxmltree` (DTD/entities disallowed -> `XmlEntityDenied`; node limit) - no network, no file access, no entity expansion.
2. Semantic decode (supported subset only): `designmap.xml` -> ordered spread/story part names; `Spread` -> `Page` (name, `GeometricBounds`, `ItemTransform`) and `TextFrame` (`Self`, `ParentStory`, `PreviousTextFrame`, `NextTextFrame`, `ItemTransform`, `PathGeometry` rectangle); `Story` -> paragraphs (`ParagraphStyleRange@AppliedParagraphStyle`) -> character runs (`CharacterStyleRange@AppliedCharacterStyle`) -> `Content` text and `Br`. Output `IdmlImport{spreads:Vec<SpreadModel>, frames:Vec<FrameModel{id, story_id, prev:Option<String>, next:Option<String>, bounds_pt:[f64;4]}>, stories:Vec<StoryModel{id, paragraphs:Vec<ParaModel{style:String, runs:Vec<RunModel{char_style:String, text:String, span:Range<usize> /*byte range of the Content text in its part*/}>}>}>, threads:Vec<Vec<String>>, folio:Option<folio::Snapshot>, loss:Vec<LossEntry>}`; units: IDML points -> `Unit::Points` (no conversion for pt); thread chains built from prev/next, validated acyclic (`CyclicThread`); the Folio doc has one `StudioPageSpread` per Spread and one `kind:"frame"`/`Payload::Text{story_id}` layer per frame (ids minted via an injected `IdMint`, source ids kept in `provenance` map). Everything outside the subset (tables, footnotes, anchored objects, GREP styles, XML structure, swatches, master pages, effects) -> `LossEntry{part, element, kind:Preserved|Unsupported}` and the bytes stay verbatim in `parts`.
3. Edit + encode (`idml_encode.rs`): `pub fn replace_content(pkg:&IdmlPackage, story_part:&str, run_span:Range<usize>, new_text:&str, cancel) -> Result<IdmlPackage, IdmlError>` using the recorded byte span (XML-escape the new text; no XML re-serialisation) so every other byte of every part is identical; `pub fn write_idml(pkg:&IdmlPackage, &Limits) -> Result<Vec<u8>, IdmlError>` deterministic ZIP (mimetype first stored, original part order, fixed timestamps). Oracle: decode -> edit -> encode -> decode twice: text changed, all other parts byte-equal, loss list identical (`idml_threaded_story_style_edit_reopen_twice`); atomic last-good save is delegated to a caller sink exactly as in the press brief (`ArtifactSink`) - do not depend on `package`.
4. Multilingual honesty (`rtl_cjk_overset_loss_cancel_stale`): any `AppliedLanguage`/`CharacterDirection`/`ParagraphDirection`/composer attribute indicating RTL, CJK, vertical, kinsoku, ruby -> `LossEntry{kind:UnsupportedComposition}` and `layout::reflow` is NOT run on that story (Latin proof never certifies them); overset is reported only for supported stories using the layout crate (or `NotComputed` if layout unavailable); stale source hash / pre-cancel -> typed errors.
5. `indd_adapter.rs`: `identify_indd(bytes:&[u8]) -> IndKind::{NotIndd, MasterPageGuid{..}, Unknown}` (header GUID sniff of the InDesign database file - exact magic UNVERIFIED, confirm on an InDesign-saved `.indd`; InDesign is installed on the operator machine) and `import_indd(..) -> Err(IdmlError::NativeEditabilityBlocked{reason})` ALWAYS, returning the original bytes as a quarantined blob (STU-IO-186: IDML never satisfies INDD).
6. Observability: `DESCRIPTOR` + `report(...)`: `NativeEditabilityBlocked` -> `Failure(Unavailable)`; zip/xml hostile + invalid -> `Failure(Validation)`; unsupported composition inside an Ok import -> `Success` + loss list; `Canceled` -> `Canceled`.
Oracle note: not on the operator's Adobe list but the same rule: later compare InDesign-exported IDML fixtures (threaded 2-frame story with paragraph/character styles, RTL and CJK stories) and InDesign's own re-open of our edited IDML (black-box; public IDML spec + observed behaviour only; InDesign scripting is a public observation channel).

</topic>

<topic id="deps" status="active" summary="third-party deps (verified 2026-10-11)">

## Deps
- `zip =8.6.0` (`default-features=false, features=["deflate-flate2"]`; MIT; zip-rs/zip2; 9.0.1 published 2026-10-10 judged too fresh; same pin as the package brief so the lockfile has one zip) - hostile-input surface and the "duplicates not documented as rejected" caveat are in the package brief.
- `roxmltree =0.21.1` (MIT OR Apache-2.0; 2025-10-12; 0.21.0 yanked; `Node::range()` byte spans used for lossless edits - API UNVERIFIED, check docs.rs).
- `sha2 =0.10.9` `default-features=false, features=["force-soft"]` (workspace pin).
- Alternative parser: `quick-xml 0.38` (Craft pin; event-based, byte offsets via `buffer_position`) if roxmltree spans are insufficient - UNVERIFIED current version.
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-interop-layout --tests` (then `clippy`, `test --test layout_edit_roundtrip`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/layout_edit_roundtrip.rs">

## Tests (<=5, MT case names; build a tiny IDML in-test with the zip crate: mimetype + designmap + 1 spread with 2 threaded frames + 1 story with two paragraphs/styles + one unknown part)
1. `idml_threaded_story_style_edit_reopen_twice`: decode (threads `[F1,F2]`, styles, text spans) -> `replace_content` -> encode -> decode twice; new text present, unknown part byte-equal, loss list identical, Folio snapshot validates.
2. `zip_traversal_duplicate_inflate_xml_entity`: `../x` name, duplicate name, mimetype not first/compressed, inflate bomb over ratio, entry-count over limit, `<!DOCTYPE ... <!ENTITY` in a story -> distinct `IdmlError::code()`, no panic, no file/network access.
3. `indd_actual_native_editability_or_blocker`: bytes with the sniffed INDD header (or arbitrary bytes) -> `NativeEditabilityBlocked`/`NotIndd`, original bytes preserved; an IDML package never turns into an "INDD readable" claim.
4. `rtl_cjk_overset_loss_cancel_stale`: story with `CharacterDirection=RightToLeftDirection` and a CJK language -> `UnsupportedComposition` loss and no reflow; Latin story with too little frame space -> overset reported; changed source hash -> `StaleSource`; pre-cancelled -> `Canceled`.
5. `cyclic_thread_and_dangling_story`: `NextTextFrame` loop -> `CyclicThread`; frame whose `ParentStory` is missing -> typed `DanglingStory`, never silently dropped.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No swatches/colour conversion (prism), styles resolution beyond names, master spreads, tables, footnotes, anchors, hyperlinks, GREP/nested styles, text variables, XML structure, effects, linked-graphic resolution, font availability (type), page-item geometry beyond rectangular frames, EPUB/PDF export, INDD decode, Folio write-back of edits (Chronicle patch), GUI/host wiring.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
