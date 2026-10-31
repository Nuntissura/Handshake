---
file_id: LANE-SMB-ID-BOOKLET-1
file_kind: lane_brief
updated_at: 2026-10-31T11:29:59Z
wp: WP-KERNEL-STUDIO
lane: SMB-ID-BOOKLET-1
lane_file: builders/lanes/hsk-studio-id-booklet-1.json
crates: [hsk-studio-layout (print preference record, booklet options, imposition plan)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="STU-LAY-156/157: typed print preferences, two print configurations, booklet options and a page imposition plan for folding">

## Target

Layout has no print, booklet or imposition code at 4f69b03b (grep `booklet|imposition|saddle|print_spreads|bitmap_resolution|PrintPref` over all crates: no hit). Engine-only under D6: typed records and a typed imposition plan, no rendering, no GUI.

**Spec** (`spec-modules/14-studio-creative-suite.md`): STU-LAY-059 :7029; STU-LAY-156 :7031-7064 (numeric bounds :7036-7042, eight option groups :7048-7057, presets, print-as-bitmap, PostScript/EPS and captured typed errors :7059-7064); STU-LAY-060 :7066; STU-LAY-157 :7068-7087 (booklet fields :7072-7083, two print configurations :7085-7087).

**MTs** (index status PENDING, `gui_host` none, write path `crates/hsk-studio-layout/**`):
- closed: MT-30820 (printer-preset numeric bounds), MT-30821..MT-30828 (General, Setup, Marks and bleed, Output, Graphics, Colour management, Advanced, Device: AC-001/AC-002 and the headless half of AC-003; the operator-surface tooltip waits for the GUI, D7), MT-30829 (arrange pages for folding), MT-30830 (two print configurations), MT-30831 (booklet option members);
- advanced: MT-30819 (AC-001 bounds and the `bitmap_printing` / `bitmap_resolution` fields only; PostScript, per-page EPS and bitmap printing output stay open);
- not this lane: MT-186 (booklet GUI, `vulkan_shell_client`), STU-LAY-158/159 PDF export and stock presets (MT-30833.., MT-30841/30842).

### Slices, one local commit each, red tests first

1. **Print preference record** (MT-30819 AC-001, MT-30820, MT-30821..30828):
   - typed `PrintPreferences` with the eight groups of :7050-7057;
   - the enumerations exactly as tabled (`print_layers`, `print_page_orientation`, `trapping`, `send_image_data`);
   - the numeric parameters of :7038-7042 with hard_min, hard_max, soft_min, soft_max, default, unit and precision stored separately and `unknown` as `None` (no bound invented);
   - `PrinterPreset {name, preset_type: preset_type.default | preset_type.custom, complete state}` (:7059-7061);
   - typed errors for the captured cases (:7064). Which fields "half-gap greater than the crossover" names is UNVERIFIED: if no field maps, record that in the notes and add no guess. Ink aliasing belongs to press-write's ink manager, not this lane.
2. **Two configurations and booklet options** (MT-30830, MT-30831):
   - a document print configuration and a separate booklet print configuration (:7085-7087);
   - `BookletOptions` with the fields of :7074-7083: `booklet_type` has exactly the five members; `margins_uniform_size` makes the top margin govern all four;
   - numeric fields admit finite values >= 0 as the only `studio_declared` admission, because the spec declares no bounds.
3. **Imposition plan** (MT-30829): `page_range` -> printer spreads for each booklet type:
   - two-up saddle stitch: pad to a multiple of 4, outermost pages pair first;
   - two-up perfect bound: signatures of `signature_size`, each saddle-ordered;
   - two-, three- and four-up consecutive;
   - blank pages follow `print_blank_printer_spreads`;
   - typed refusals: empty or malformed range, invalid signature size, page count over a declared bound;
   - hand-computed expectations for 8, 12 and 16 pages.
4. **Sheet geometry:** per printer spread, page placement rectangles from the page size, `space_between_pages`, `bleed_between_pages`, the margins (`auto_adjust_margins`) and a creep offset per sheet. The rule is declared and documented (linear from the outermost to the innermost sheet); InDesign's creep formula is UNVERIFIED (D5). The output is a typed plan only; rendering imposed sheets to PDF (interop-layout or press) is a later lane.
- Hostile input (V4-01/02): page counts, signature counts and spreads bounded before allocation (`dims_u32_max`- and `aggregate_over_budget`-style tests); terminal outcomes through layout `report` (`src/lib.rs:214`) with codes only.

### Not in this lane
- Folio persistence of print and booklet records (a later folio lane; rule 4b package suite then).
- PostScript, EPS and bitmap printing output, device printing, printer marks rendering, the operator surface.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-LAY-059", "STU-LAY-156", "STU-LAY-060", "STU-LAY-157", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045"], "refactor_exposure": "none", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"attribution": "typed records and a pure imposition plan; no mutating command is added (persistence is a later folio lane)"}, "recorded_at": "2026-10-31T11:29:59Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-id-booklet-1`, branch `lane/smb-id-booklet-1`, base 4f69b03b)

- layout: new `src/print.rs` and `src/booklet.rs`, module lines and `DESCRIPTOR` entries in `src/lib.rs`, new `tests/print_booklet.rs`.
- No folio, type, press or interop-layout edit, no `Cargo.toml` change, no root input.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold): step 0; clippy over layout and its direct dependents (harness-view, harness, interop-layout by manifest); nextest WHOLE layout suite (default unit; layout has no round feature unit); doctests if doc examples are added (4d). Name the log per RUN LOGS.
- Push in any order with SMB-AE-PARTICLES-2 and SMB-PDF-PRESETS-1, before SMB-R47-FOLLOWUPS-1 and SMB-SETTINGS-SCHEMA-1. Do NOT start the bundled run until the lead names the disk and the base.
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
