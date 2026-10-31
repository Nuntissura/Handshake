---
file_id: LANE-SMB-PDF-PRESETS-1
file_kind: lane_brief
updated_at: 2026-10-31T11:29:59Z
wp: WP-KERNEL-STUDIO
lane: SMB-PDF-PRESETS-1
lane_file: builders/lanes/hsk-studio-pdf-presets-1.json
crates: [hsk-studio-press (PDF export preset contract and its resolution onto the existing export options)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="STU-IO-125..133: complete, typed PDF export presets with exact domains, numeric contracts, standards and font rules, localised descriptions and provenance">

## Target

The PDF export preset contract is absent at 4f69b03b: press-write's export module lists "presets" as not in its slice (`crates/hsk-studio-press-write/src/export.rs:18-20`); grep `preset|downsampl` over press*, layout and interop-layout src finds nothing else. Engine-only under D6.

**Spec** (`spec-modules/14-studio-creative-suite.md`): STU-IO-125 :21001 (a preset is COMPLETE; no ambient default), STU-IO-126 :21003 (ten groups; every setting declares its group), STU-IO-127 :21005 (image family, applied to colour, greyscale and monochrome), STU-IO-128 :21007-21026 (14 enumerations; single-value rows carry `domain_completeness: partial`), STU-IO-129 :21028-21043 (seven separate numeric fields; `unknown` stays unknown; never turn an observed value into a bound), STU-IO-130 :21045, STU-IO-131 :21047, STU-IO-132 :21049, STU-IO-133 :21051.

**MTs closed** (index status PENDING, write path `crates/hsk-studio-press/**`, host block all `none`): MT-33385, MT-33386, MT-33387, MT-33388, MT-33389, MT-33390, MT-33391, MT-33392, MT-33393, MT-33394, MT-33395.

**Base:** press is a facade (`src/lib.rs` only; re-exports press-write `ExportOptions` in the block at :28-35; `outcome_for` :52, `report` :70, `DESCRIPTOR` :86). press-write `ExportOptions` (`src/export.rs:253-270`: `version` :255, `output_intent` :256, `subset_fonts` :260, `pdfx4` :264, `pdfa2b` :269; Default :281); `VERSIONS` = 1.4, 1.5, 1.6, 1.7, 2.0 (:78); PDF/X-4 needs a GtsPdfx intent, version 1.6 or later and a TrimBox (:261-264); fonts are written embedded only, as Type0/CIDFontType2 (:10-13), with typed refusals such as `font_cff_not_embedded` (:188).

### Slices, one local commit each, red tests first

1. **Preset record** (MT-33385..33387): `PdfPreset {id, name, descriptions, provenance, settings}` with the ten STU-IO-126 groups and the STU-IO-127 family per image group (eleven settings, the codec family, two quality dictionaries). Completeness: a preset missing any setting refuses `preset_incomplete` naming the setting; nothing resolves from an ambient default.
2. **Domains and numeric contracts** (MT-33388..33391):
   - the 14 enumerations exactly as tabled, no member added or renamed;
   - `binding` adds `Right` labelled `studio_declared`; `cannot_embed_font_policy` `Error` and `OK` are `studio_declared`;
   - single-value rows carry `domain_completeness: partial` (:21026);
   - numeric settings store hard_min, hard_max, soft_min, soft_max, default, unit and precision separately, `unknown` as `None`; a value outside hard bounds refuses, outside soft bounds warns typed.
3. **Standards and fonts** (MT-33392, MT-33393):
   - asserting a standard check with an empty `output_intent_profile` is a validation error (:21045);
   - the font settings of :21047 are typed;
   - every font the exporter cannot embed produces a receipt naming the font and the policy applied (STU-IO-012). press-write cannot write a non-embedded font today, so under every policy the receipt records the refusal. Output without that font is a declared gap, never invented.
4. **Descriptions and provenance** (MT-33394, MT-33395): `descriptions` is a locale-keyed map rendered through a declared fallback chain (requested locale, then its language, then a declared default locale), never one flattened string; `preset_source` is `shipped`, `operator` or `imported` (+ origin); a shipped preset is read-only; editing one yields an operator preset with a recorded parent.
5. **Resolution onto export:** `PdfPreset` -> press-write `ExportOptions` plus a receipt that lists every setting as `applied` or `not_applied` with a code (no silent green: downsampling, recompression, security and other settings the exporter does not implement are `not_applied`).
   - **Conflict to surface, not resolve:** the `compatibility_level` domain is 1.3, 1.4, 1.5 (:21011), but the writer starts at 1.4 (:78) and writes PDF/X-4 only at 1.6+ (:261-264). Level 1.3 refuses typed; a standards preset whose level the writer cannot honour refuses typed. Report it in the notes for the spec owner.
- Hostile input (V4-01/02): bounded list lengths (always-embed and never-embed lists, locale map) checked before allocation; typed refusals only. Vendor preset values and Acrobat behaviour are UNVERIFIED (D5).

### Not in this lane
- Stock presets STU-LAY-159 (layout MT-30841/MT-30842): layout has no press edge, so their placement is a lead decision.
- Executing image downsampling and recompression (no resampler in press).
- Preset file import or export (`imported` provenance is typed only).
- PDF/X conformance claims: they stay `not_proven` (SMB-PDFX-ORACLE is an Operator item).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-IO-125", "STU-IO-126", "STU-IO-127", "STU-IO-128", "STU-IO-129", "STU-IO-130", "STU-IO-131", "STU-IO-132", "STU-IO-133", "STU-IO-012", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045"], "refactor_exposure": "none", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"attribution": "editing a shipped preset creates an operator preset; persisting it is a later host or Folio step, so no mutating command is added here", "diagnostics": "refusals and receipts carry codes; terminal outcomes through press report (src/lib.rs:70)"}, "recorded_at": "2026-10-31T11:29:59Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pdf-presets-1`, branch `lane/smb-pdf-presets-1`, base 4f69b03b)

- press: new `src/preset.rs` (submodules allowed under `src/preset/`), `src/lib.rs` (mod, pub use, `DESCRIPTOR` api entries), new `tests/pdf_presets.rs`.
- No press-read or press-write edit, no `Cargo.toml` change, no root input.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold): step 0; clippy over press and its direct dependents (harness-core, harness-view, harness, interop-fig-native, interop-fig, interop-layout by manifest); nextest WHOLE press suite (default unit; press has no round feature unit); `cargo test --doc -p hsk-studio-press` if doc examples are added (4d). Name the log per RUN LOGS.
- Push in any order with SMB-AE-PARTICLES-2 and SMB-ID-BOOKLET-1, before SMB-R47-FOLLOWUPS-1 and SMB-SETTINGS-SCHEMA-1. Do NOT start the bundled run until the lead names the disk and the base.
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
