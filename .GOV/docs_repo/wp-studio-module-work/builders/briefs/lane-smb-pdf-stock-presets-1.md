---
file_id: LANE-SMB-PDF-STOCK-PRESETS-1
file_kind: lane_brief
updated_at: 2026-10-31T14:38:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-PDF-STOCK-PRESETS-1
lane_file: builders/lanes/hsk-studio-pdf-stock-presets-1.json
crates: [hsk-studio-press (stock PDF preset catalogue STU-LAY-159 on the press PdfPreset contract, preset codec)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="STU-LAY-159: the six stock PDF export presets as a press catalogue, with the compliance token kept apart from the name, a nullable end page, a lossless preset codec and the spec gaps surfaced">

## Target

**Placement (lead question, no new module edge):** the stock catalogue lives in press, next to the `PdfPreset` contract it instantiates (`src/preset.rs`, SMB-PDF-PRESETS-1). Layout has no press edge (WORKSPACE_RULES path-deps: layout -> folio, type, nib) and holds no PDF export record yet; the layout PDF export preference record of STU-LAY-158 is lane SMB-ID-PDF-EXPORT-PREFS-1 this wave, which records a preset by its stable id string and resolves nothing. Consumers that resolve ids (interop-layout, harness) already depend on press.

**Spec** (`spec-modules/14-studio-creative-suite.md`): STU-LAY-159 :7137-7175 (six presets, 19 rows, compliance token separate from the name, common defaults, nullable end page); STU-LAY-168 :7582 (one descriptor per stock preset: captured values survive save/load; one per open spec gap); STU-IO-125..133 :21001-21051; STU-SECTION-003 :118.

**MTs:** advances MT-30841 (AC-002, AC-003, AC-004 for all six; AC-001 for three, see gaps) and MT-30842 (codec round trip, name/compliance separation, real export of the writable presets). Neither closes: three presets carry an open spec gap and SD-13 blocks their export. MT-30842's command-descriptor half (`control_descriptors: shared_typed_source`) waits for a command lane; this lane registers no command.

**Base at e9c46302:** `src/preset.rs` (1385 lines): SD-13 note :28-31; `ImageFilter` :196 (DCTEncode, FlateEncode); `END_PAGE` :361 (-1 = last); `SETTINGS` :447 (74 settings); `PresetError` from :493 (`PresetIncomplete` :500); `StandardsSettings` :956 (`asserts_standard` :964); `from_draft` :1048; `edit_copy` :1248; `resolve` :1295. `src/lib.rs`: `mod preset` :47, `pub use preset::{..}` :48, `report` :84, `DESCRIPTOR` :100. press-write `ExportOptions` has no transparency, flattening or overprint field. No consumer of `PdfPreset` outside press (grep at e9c46302).

### Slices, one local commit each, red tests first

1. **Contract additions the stock rows need** (press `SETTINGS`, additive):
   - `standards_compliance.compliance_check`: enum `none`, `PDF/X-1a:2001`, `PDF/X-3:2002`, `PDF/X-4:2007` (STU-LAY-159 row; Partial); any member but `none` asserts a standard (STU-IO-130 output intent rule, SD-13 refusals);
   - `allow_transparency` and `preserve_overprint` (Bool; group basis `studio_declared`); resolution marks both `not_applied` (the writer has no such field);
   - `image_filter` gains `CCITTFaxEncode` citing STU-LAY-159 (common default); it is admitted only on `monochrome_images.filter`, elsewhere typed `filter_not_admitted_for_group`;
   - `PdfPreset::page_range() -> (u32, Option<u32>)`: the end is nullable; the -1 sentinel of STU-IO-129 stays inside the setting only.
   - Update `tests/pdf_presets.rs` drafts and the image_filter domain assertion; nothing else there.
2. **Stock catalogue** (new `src/preset/stock.rs`; MT-30841):
   - six records: stable id (`hsk.studio.pdf_preset.<snake>`), name as the spec column states it, `compliance` token as its own field (AC-003: a test renames a preset and the compliance token is unchanged; nothing parses one from the other);
   - all 19 rows exactly; common defaults (AC-004): JPEG-2000 tile 256 x 256 quality 30, ACS QFactor 0.15 with 1:1 chroma, monochrome CCITT, image memory 1,048,576, start page 1, end page `None`;
   - every setting the spec does not give a stock value carries per-value provenance `studio_declared` (STU-IO-125: the shipped set is Studio's own), listed by a `stock_provenance(id)` receipt; numeric ones use the STU-IO-129 `default` column where it has one;
   - `logical_count() == 6`; the seven locale variants are not materialised (identities not captured: typed gap `locale_variants_not_captured`).
3. **Stock -> `PdfPreset`** (`PresetSource::Shipped`, read-only; `edit_copy` gives an operator preset):
   - High Quality Print, Press Quality, Smallest File Size build complete;
   - the three PDF/X presets refuse typed `stock_output_intent_not_captured`, because STU-IO-130 requires an output-intent profile and the spec captured none; this gap stays open with a test (STU-LAY-168).
4. **Codec** (new `src/preset/codec.rs`; MT-30842): canonical deterministic UTF-8 text, hand-written, no new dependency; `encode` then `decode` reproduces every setting, description, provenance and the compliance token for the three complete presets.
   - Hostile text (V4-08): bounded before allocation; tokenised, escapes decoded first; three adversarial tests (escape, comment/decoy line, case or whitespace);
   - typed refusals: duplicate key, unknown key, missing setting, over-long value.
5. **Real export** (MT-30842 step 1): `resolve` each writable stock preset and export a one-page PDF through press-write; read the header version back with press-read (1.4, 1.4, 1.5).
   - The PDF/X presets stay refused before export: SD-13 `compatibility_level_not_writable` (1.3) and `standard_level_conflict` (X-4).
- Terminal outcomes through press `report` (`src/lib.rs:84`) with codes only. Vendor values (Acrobat behaviour, locale files) are UNVERIFIED (D5).

### Spec conflicts to report, not resolve
- STU-LAY-159 says monochrome filter CCITT; STU-IO-128 lists `image_filter` as DCTEncode, FlateEncode only.
- STU-LAY-159 says the end page is nullable, never -1; STU-IO-129 stores -1.
- `cannot_embed_font_policy`: STU-LAY-159 shows `error` captured; STU-IO-128 calls Error studio-declared. Keep the press label.
- STU-IO-127 has two quality dictionaries; the JPEG-2000 tile and quality have no setting home, so the stock record carries them and the receipt marks them `not_representable`.
- The PDF/X output-intent profiles (STU-IO-130) are not captured.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-LAY-159", "STU-LAY-168", "STU-IO-125", "STU-IO-127", "STU-IO-128", "STU-IO-129", "STU-IO-130", "STU-IO-133", "STU-SECTION-003", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045", "STU-ARC-014", "STU-MDL-002", "GUI-DESKTOP-018"], "refactor_exposure": "none", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"control_descriptors": "MT-30842 declares shared_typed_source; this lane adds no command, so its descriptor half stays open for a command lane", "attribution": "the catalogue is static and shipped presets are read-only; edit_copy is in-memory, persisting it is a later host or Folio step", "diagnostics": "refusals and receipts carry codes; terminal outcomes through press report (src/lib.rs:84)"}, "recorded_at": "2026-10-31T14:38:14Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pdf-stock-presets-1`, branch `lane/smb-pdf-stock-presets-1`, base e9c46302)

- press: `src/preset.rs`, new `src/preset/stock.rs` and `src/preset/codec.rs`, `src/lib.rs` (pub use lines, `DESCRIPTOR` api entries), `tests/pdf_presets.rs`, new `tests/pdf_stock_six_preset_defaults.rs` and `tests/pdf_stock_preset_matrix_roundtrip.rs` (target names from the MT-30841/30842 proof recipes).
- No press-read, press-write, layout or interop-layout edit, no `Cargo.toml` change, no root input, no new dependency.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over press and its direct dependents (harness-core, harness-view, harness, interop-fig-native, interop-fig, interop-layout by manifest); nextest WHOLE press suite (default unit; no round feature unit for press, 4e); `cargo test --doc -p hsk-studio-press` if doc examples are added (4d). Name the log per RUN LOGS.
- Push order: press's closure (accord, observe, press-read, press-write) holds none of this wave's other lanes, so it pushes in any order with SMB-AE-WATER-SIMS-1 and SMB-ID-PDF-EXPORT-PREFS-1. Do NOT start the bundled run until the lead names the disk and confirms R49 judged SMB-PDF-PRESETS-1 (a FAIL there is remediated first on that lane).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
