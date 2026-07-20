---
file_id: studio-app-feature-research-snapshot-distillation-delta
topic_id: SFR-SNAPSHOT-DISTILL
title: "Snapshot Distillation Delta (2026-07-20, ACTION-A4)"
status: draft
summary: "Feature rows distilled from content ALREADY present verbatim in _source_snapshots/ but never promoted into rows. Zero new external research."
sources: 3
updated_at: "2026-07-20"
---

## [SFR-SNAPSHOT-DISTILL] Snapshot Distillation Delta

### [SFR-SNAPSHOT-DISTILL.method] Method

```yaml
action: "ACTION-A4 (see 61-parity-audit-action-register.md)"
rule: "Promote only content that exists verbatim in a local _source_snapshots/ file. No new web fetches. Each row cites the exact snapshot path + line range."
verification_status_meaning: "VERIFIED = text is present in the cited local snapshot at the cited lines."
scope_note: "Two items the 2026-07-20 audit hoped to distill here turned out NOT to be present in any local snapshot: Illustrator 'Discard White Overprint' and Figma image-import 4096px downscale. Both are reclassified to ACTION-A5 (need a targeted fetch) and are NOT rowed here. Do not treat their absence from this file as coverage."
```

### [SFR-SNAPSHOT-DISTILL.figma-export] Figma Export-Modal Option Rows

```yaml
rows:
  - id: "figma.distill.export.color-profile"
    app: figma
    feature: "Export color profile dropdown"
    app_behavior: "Export settings expose a color-profile dropdown: 'Same as file', 'sRGB', 'Display P3'. Default exports use the file's color profile; the dropdown overrides per export."
    primitive_domain: color
    source_ref: "_source_snapshots/figma-export-formats-jina.md:110-122"
    verification_status: VERIFIED
    closes: "58 SFR-PGAP-FG (color-management export behavior under-rowed / distillation gap)"
  - id: "figma.distill.export.image-resampling"
    app: figma
    feature: "Image resampling (Detailed/Basic)"
    app_behavior: "For JPG/PNG/PDF export, an 'Image resampling' dropdown under Advanced export settings offers Detailed (default, bicubic) and Basic (nearest-neighbor). Detailed for photos/gradients/shadows; Basic for icons/logos/pixel art."
    primitive_domain: export
    source_ref: "_source_snapshots/figma-export-formats-jina.md:138-161"
    verification_status: VERIFIED
    closes: "58 SFR-PGAP-FG (export-modal option depth)"
  - id: "figma.distill.export.ignore-overlapping-layers"
    app: figma
    feature: "Ignore overlapping layers"
    app_behavior: "Enabled by default. When on, only the selected layers are exported; overlapping/intersecting objects are excluded. Slice-specific behavior: with a slice inside a frame/group, only same-container content within the slice bounds exports; disabled exports all content visually within the slice bounds."
    primitive_domain: export
    source_ref: "_source_snapshots/figma-export-formats-jina.md:124-130"
    verification_status: VERIFIED
    closes: "58 SFR-PGAP-FG (export-modal option depth)"
  - id: "figma.distill.export.slice-region"
    app: figma
    feature: "Slice export regions"
    app_behavior: "Slices act as export regions whose contents are determined by the Ignore-overlapping-layers rule and the slice's container. Slices are a manual export-region mechanism distinct from the SliceNode plugin surface."
    primitive_domain: export
    source_ref: "_source_snapshots/figma-export-formats-jina.md:128-130"
    verification_status: VERIFIED
    closes: "58 SFR-PGAP-FG (Slice tool not rowed; only SliceNode plugin row existed)"
  - id: "figma.distill.export.image-quality"
    app: figma
    feature: "Export image quality"
    app_behavior: "JPG and PDF exports expose a quality/size control; JPG defaults to High, PDF to Medium, changeable in the Export section of the right sidebar."
    primitive_domain: export
    source_ref: "_source_snapshots/figma-export-formats-jina.md:132-134"
    verification_status: VERIFIED
  - id: "figma.distill.export.format-option-matrix"
    app: figma
    feature: "Per-format export option availability"
    app_behavior: "Option availability by format: Ignore overlapping layers + Include bounding box available for PNG/JPG/SVG (not PDF); Include 'id' attribute, Outline text, Simplify stroke available for SVG only."
    primitive_domain: export
    source_ref: "_source_snapshots/figma-export-formats-jina.md:102-108"
    verification_status: VERIFIED
  - id: "figma.distill.export.include-bounding-box"
    app: figma
    feature: "Include bounding box (text layers)"
    app_behavior: "Text-layer-only. Enabled: export size follows the text layer bounding box (includes empty space, or trims text beyond it). Disabled: export size follows the text glyphs' own dimensions."
    primitive_domain: export
    source_ref: "_source_snapshots/figma-export-formats-jina.md:163-167"
    verification_status: VERIFIED
  - id: "figma.distill.export.svg-outline-text-simplify-stroke-id"
    app: figma
    feature: "SVG export options: outline text, simplify stroke, include id"
    app_behavior: "SVG-only export options: Outline text (convert text to paths), Simplify stroke (reduce stroke geometry), Include 'id' attribute (emit stable node ids into the SVG)."
    primitive_domain: export
    source_ref: "_source_snapshots/figma-export-formats-jina.md:104-108,169"
    verification_status: VERIFIED
```

### [SFR-SNAPSHOT-DISTILL.indesign-cjk-dom] InDesign CJK Composition DOM Rows

```yaml
rows:
  - id: "indesign.distill.dom.composite-font"
    app: indesign
    feature: "Composite Font scripting DOM surface"
    app_behavior: "InDesign UXP DOM exposes CompositeFont, CompositeFonts (collection), CompositeFontEntry, and CompositeFontEntries — the object model behind the CJK Composite Font editor (per-script font mixing: kanji/kana/roman/symbol/half-width with size, baseline, and scaling). Confirms Composite Fonts are a real, scriptable InDesign capability the corpus had only at boilerplate row depth."
    primitive_domain: typography
    source_ref: "_source_snapshots/indesign-uxp-dom-api-jina.md:620-623"
    verification_status: VERIFIED
    closes: "58 XAPP-04 (CJK composite fonts absent as a feature row)"
  - id: "indesign.distill.dom.kashidas-options"
    app: indesign
    feature: "Kashidas (Arabic justification) DOM surface"
    app_behavior: "InDesign UXP DOM exposes KashidasOptions — the object behind Arabic kashida elongation/justification control. Confirms scriptable RTL justification depth."
    primitive_domain: typography
    source_ref: "_source_snapshots/indesign-uxp-dom-api-jina.md:970"
    verification_status: VERIFIED
    closes: "58 SFR-PGAP-ID (Arabic/Hebrew option depth)"
  - id: "indesign.distill.dom.kinsoku"
    app: indesign
    feature: "Kinsoku (CJK line-break) DOM surface"
    app_behavior: "InDesign UXP DOM exposes KinsokuSet, KinsokuTable(s), KinsokuHangTypes, and KinsokuType — the object model behind CJK line-break/hang rules (no-start/no-end characters, hanging punctuation)."
    primitive_domain: typography
    source_ref: "_source_snapshots/indesign-uxp-dom-api-jina.md:974-978"
    verification_status: VERIFIED
    closes: "58 XAPP-04 (CJK inline typography at boilerplate depth)"
  - id: "indesign.distill.dom.mojikumi"
    app: indesign
    feature: "Mojikumi (CJK spacing) DOM surface"
    app_behavior: "InDesign UXP DOM exposes MojikumiTable(s), MojikumiTableDefaults, and MojikumiUiPreference — the object model behind CJK character-spacing/aki rules between punctuation and glyph classes."
    primitive_domain: typography
    source_ref: "_source_snapshots/indesign-uxp-dom-api-jina.md:1047-1050"
    verification_status: VERIFIED
    closes: "58 XAPP-04 (CJK inline typography at boilerplate depth)"
```

### [SFR-SNAPSHOT-DISTILL.sources] Sources

```yaml
sources:
  - { id: SD-S01, path: "_source_snapshots/figma-export-formats-jina.md", note: "Figma 'Export formats and settings for static designs' local snapshot." }
  - { id: SD-S02, path: "_source_snapshots/indesign-uxp-dom-api-jina.md", note: "InDesign UXP DOM API class index local snapshot." }
  - { id: SD-S03, path: "58-parity-feature-gap-register.md", note: "Gaps these rows partially close." }
```
