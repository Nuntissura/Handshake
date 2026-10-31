---
file_id: LANE-SMB-ID-PDF-EXPORT-PREFS-1
file_kind: lane_brief
updated_at: 2026-10-31T14:38:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-PDF-EXPORT-PREFS-1
lane_file: builders/lanes/hsk-studio-id-pdf-export-prefs-1.json
crates: [hsk-studio-layout (PDF export preference record STU-LAY-158: six panels, relative bounds, read-back fields, security errors)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="STU-LAY-158: the typed PDF export preference record (six panels), partner-relative compression bounds, requested vs effective read-back, typed security errors, and a stock preset reference by id">

## Target

InDesign print/export engine work after SMB-ID-BOOKLET-1. Layout has no PDF export preference record at e9c46302 (grep `pdf_export|acrobat_compatibility|threshold_to_compress` over layout src: no hit; `print.rs` and `booklet.rs` are the precedent). Engine-only under D6: a typed record, validation and read-back, no export, no GUI.

**Spec** (`spec-modules/14-studio-creative-suite.md`): STU-LAY-158 :7093-7136 (numeric table :7098-7108, relative bounds note :7110-7112, panels :7120-7125, effective read-back and password error :7127-7131, tagged PDF :7133-7135); STU-LAY-168 :7582 (relative bound descriptors re-validate when the partner changes); STU-LAY-159 :7137 (compliance tokens).

**MTs** (index status PENDING, `gui_host` none, write path `crates/hsk-studio-layout/**`):
- closed: MT-30834 (nine numeric bound sets), MT-30835 General, MT-30836 Compression, MT-30837 Marks and bleeds, MT-30838 Output, MT-30839 Advanced, MT-30840 Security (AC-001/AC-002 and the headless half of AC-003; the operator-surface tooltip waits for the GUI, D7);
- advanced: MT-30833 (AC-001 relative bounds, AC-002 grouped surface, AC-003 typed password error; AC-004 tagged and accessible PDF output stays open: it needs the exporter);
- not this lane: applying the record to an actual export (interop-layout, which depends on both layout and press), STU-LAY-159 stock presets (lane SMB-PDF-STOCK-PRESETS-1).

**Base at e9c46302:** `src/print.rs`: `NumericBounds` :51 (seven fields, `None` = unknown), `PrintParameter` :65, `PrintError` :235, `PrinterPreset` :699; `src/booklet.rs`: `parse_page_range` :260 (reuse for `page_range`), `MAX_PAGE_RANGE_BYTES` :42; `src/lib.rs`: `mod print` :36, `pub use print::{..}` :71, `report` :227, `DESCRIPTOR` :252.

### Slices, one local commit each, red tests first

1. **Numeric contracts** (MT-30834, MT-30833 AC-001): the nine rows of :7100-7108, every field separate, `unknown` as `None`:
   - absolute rows: sampling dpi 9..2400, `subset_fonts_below` 0..100, tile sizes 128..2048;
   - the three `threshold_to_compress_*` rows: a relative bound type (`1x .. 10x` of the named partner), never an absolute stored maximum. Validated against the partner's live value; changing a sampling dpi re-validates its threshold atomically (refuse and keep the old state, or accept both). Tests: partner up, partner down, threshold-first edit.
   - Unit conflict to report, not resolve: the table states unit `ratio` while the bounds are multiples of a dpi partner. Declare the stored unit in the module docs and the notes; UNVERIFIED (D5).
2. **General and Compression panels** (MT-30835, MT-30836):
   - enumerations exactly as tabled: `acrobat_compatibility` five members; `export_layers` three; `pdf_display_title` two; quality tiers seven; `compression_type` three;
   - `page_range` through `parse_page_range`;
   - `pdf_magnification` is declared ten-member but unnamed, and `standards_compliance`, `interactive_elements_option`, `pdf_page_layout`, sampling method and compression method name no members: bounded tokens with `domain_completeness: undeclared`, never invented members. `standards_compliance` may admit the four STU-LAY-159 compliance tokens as a `partial` domain citing :7137.
3. **Marks and bleeds, Output** (MT-30837, MT-30838):
   - `printer_mark_weight`: the nine fixed members with their units kept (0.125pt .. 0.30mm), each with an exact point value, never merged;
   - four bleeds and `use_document_bleed_with_pdf`;
   - `pdf_color_space` six members;
   - the four `effective_*` read-back fields are a separate read-only type, `None` until an exporter records them through a typed setter; a test proves a request is never echoed into an effective field.
4. **Advanced and Security** (MT-30839, MT-30840, MT-30833 AC-003):
   - `applied_flattener_preset` is a bounded name;
   - reading `open_document_password` or `change_security_password` while `use_security` is false returns the typed error (:7130-7131), never an empty string;
   - passwords are a redacting type: `Debug` and every observe or receipt path print no value (a test greps the Debug output);
   - nine `disallow_*` flags independent.
5. **Preset reference and record** (lead placement for STU-LAY-159):
   - `PdfExportPreferences` holds the six panels and an optional `pdf_export_preset_id` (bounded id string, `studio_declared`), which resolves nothing (press owns the catalogue; layout has no press edge);
   - complete-state copy like `PrinterPreset`; terminal outcomes through layout `report` (`src/lib.rs:227`) with codes only.
- Hostile input (V4-01/02): every text bounded before allocation; typed refusals only. Name test functions after the MT proof recipe filters (`pdf_numeric_relative_bound_metadata`, `pdf_general_document_structure_options`, `pdf_compression_classes_quality_tiles`, `pdf_marks_bleed_nine_weights`, `pdf_output_effective_colour_readback`, `pdf_advanced_subset_omit_flatten`, `pdf_security_password_permission_bits`).

### Not in this lane
- Mapping the record onto press `ExportOptions` or `PdfPreset` (acrobat_4..8 vs 1.3..1.5 is its own conflict), tagged PDF, Folio persistence (rule 4b then).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-LAY-158", "STU-LAY-168", "STU-LAY-159", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045"], "refactor_exposure": "none", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"attribution": "typed records and validation only; no mutating command is added (persistence is a later folio lane)", "diagnostics": "password values never reach observe, receipts or Debug output"}, "recorded_at": "2026-10-31T14:38:14Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-pdf-export-prefs-1`, branch `lane/smb-id-pdf-export-prefs-1`, base e9c46302)

- layout: new `src/pdf_export_prefs.rs` (submodules allowed under `src/pdf_export_prefs/`), module and pub use lines and `DESCRIPTOR` entries in `src/lib.rs`, new `tests/pdf_export_prefs.rs`. `print.rs` and `booklet.rs` are read and reused, not changed (if a shared helper must move, say so in the notes).
- No folio, type, press or interop-layout edit, no `Cargo.toml` change, no root input, no new dependency.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over layout and its direct dependents (harness-view, harness, interop-layout by manifest); nextest WHOLE layout suite (default unit; no round feature unit for layout, 4e); doctests if doc examples are added (4d). Name the log per RUN LOGS.
- Push order: layout's closure (folio, type, nib) holds SMB-FOLIO-CHANNEL-RECORD's folio, so this lane pushes BEFORE that lane; any order with SMB-PDF-STOCK-PRESETS-1 and SMB-AE-WATER-SIMS-1. Do NOT start the bundled run until the lead names the disk and confirms R49 judged SMB-ID-BOOKLET-1 (a FAIL there is remediated first on that lane).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
