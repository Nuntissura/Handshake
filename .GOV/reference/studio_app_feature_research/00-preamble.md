---
file_id: studio-app-feature-research-preamble
topic_id: SFR-PREAMBLE
title: "Studio App Feature Research: Preamble and Approach"
status: draft
summary: "Scope, non-authority stance, source policy, schema, risks, and recommended build approach."
sources: 9
updated_at: "2026-07-05"
---

## [SFR-PREAMBLE] Preamble and Approach

### [SFR-PREAMBLE.purpose] Purpose

```text
This package inventories the feature behavior of Adobe Photoshop, the Affinity V2 suite, Adobe InDesign, Adobe Illustrator, and Figma so Handshake can rebuild equivalent creative capability as native Rust tools inside the built-in Studio module.

Studio is local-first, no-cloud-required, and Rust-forward. The goal is not to clone legacy application menus or ship vendor-named product surfaces. The goal is to identify the underlying primitives Handshake needs: document state, layer graphs, raster transforms, selections, masks, typography, vector paths, vector networks, artboards, frames, layout constraints, pages/spreads, components, styles, variables, color systems, import/export, prepress, automation, collaboration, diagnostics, and model-facing tool contracts.
```

### [SFR-PREAMBLE.naming-and-compatibility] Naming and File Compatibility Policy

```yaml
naming_policy:
  product_rule: "Studio product tools, commands, panels, manuals, and operator-facing workflows must use Handshake-native names, not Photoshop, InDesign, Illustrator, Figma, FigJam, Affinity, Photo, Designer, or Publisher product names."
  allowed_vendor_name_use:
    - "source provenance"
    - "research inventory namespaces"
    - "compatibility documentation"
    - "import/export test fixture labels"
    - "operator migration notes"
  forbidden_vendor_name_use:
    - "shipped Studio tool names"
    - "command names"
    - "panel names"
    - "manual topic names except compatibility notes"
    - "new Handshake product surface names"
  rationale: "The source apps define capability targets and compatibility requirements; they do not define Handshake product naming."
file_format_policy:
  compatibility_rule: "Do not invent a replacement interchange format for this rebuild scope."
  required_compatibility_targets:
    - "PSD and layered Photoshop-style documents where technically practical"
    - "IDML/PDF/package-oriented layout interchange where technically practical"
    - "Affinity document import/export compatibility where technically practical and legally/tooling feasible"
    - "Illustrator AI/AIT, PDF, SVG/SVGZ, EPS/PS, DWG/DXF, PSD, and raster/vector export compatibility where technically practical"
    - "Figma local copy and interchange compatibility for .fig, .jam, .deck, .buzz, .site, .make, Sketch import, SVG, PDF, PNG, JPEG, GIF, TIFF, WebP, video, PPTX, and CSV where technically practical"
    - "Common creative interchange formats such as PDF, SVG, PNG, JPEG, TIFF, WebP, OpenEXR, DWG/DXF, and font/profile/link dependencies where relevant"
  internal_storage_rule: "Use existing Handshake document/project storage or a separately approved internal state model; do not present a new invented file format as the answer to app compatibility."
  implementation_gate: "Every promoted import/export feature needs explicit compatibility scope, fixtures, round-trip expectations, unsupported-feature diagnostics, and recovery behavior."
```

### [SFR-PREAMBLE.scope] Scope

```yaml
apps_in_scope:
  - Adobe Photoshop desktop
  - Adobe Camera Raw as used by Photoshop workflows
  - Affinity Photo 2
  - Affinity Designer 2
  - Affinity Publisher 2
  - Adobe InDesign desktop
  - Adobe Illustrator desktop
  - Figma Design
  - FigJam
  - Figma Draw
  - Figma Motion
  - Figma Slides
  - Figma Sites
  - Figma Buzz
  - Figma Make
  - Figma Dev Mode and developer APIs
not_in_scope_yet:
  - Exhaustive menu-command parity
  - Pixel-perfect UI clone
  - Vendor cloud service replication
  - Vendor product naming for Studio tools
  - Inventing a replacement interchange file format
  - Legal/licensing analysis
  - Product implementation
authority_status: "Reference only; not a Work Packet, Master Spec, or validator authority."
```

### [SFR-PREAMBLE.schema] Feature Record Schema

```yaml
feature_record_schema:
  id: "Stable feature id. Prefix identifies app namespace."
  name: "Human-readable feature name."
  app_behavior: "Observed app behavior to rebuild or reinterpret as a Handshake primitive."
  primitive_domain: "Primary Handshake primitive/domain candidate."
  source_ids: "Source keys from the topic EOF sources block."
  verification_status: "Optional. Use VERIFIED, PARTIAL, or UNVERIFIED."
  notes: "Optional implementation or uncertainty notes."
```

### [SFR-PREAMBLE.approach] Best Approach

```text
Build from primitives upward, not app-by-app sideways.

Phase 1 should normalize feature rows into a shared Studio primitive model. Photoshop and Affinity Photo both need raster layers, masks, selections, adjustments, color pipelines, filters, and export. InDesign and Affinity Publisher both need pages, spreads, text frames, style systems, tables, links, and prepress. Affinity Designer, Illustrator, and Figma Draw all need paths, vector networks, shapes, fills, strokes, artboards/frames, text-on-path, booleans, components/symbols, and export slices.

Phase 2 should define Rust engine boundaries. Good candidates are `studio_document`, `studio_layer_graph`, `studio_raster`, `studio_selection`, `studio_mask`, `studio_color`, `studio_vector`, `studio_layout`, `studio_typography`, `studio_style_registry`, `studio_interaction`, `studio_collaboration`, `studio_prepress`, `studio_import_export`, `studio_extensibility`, and `studio_automation`.

Phase 3 should add model-facing tool contracts. Each primitive needs stable command IDs, typed inputs/outputs, dry-run support, receipts, undo/redo semantics, visual diagnostics, and deterministic replay hooks.

Phase 4 should pick vertical workflow slices rather than broad parity. Example slices: non-destructive photo edit stack; PSD/AFPHOTO-like layered import; page-layout document with text frames/styles/PDF export; vector artboard with path editing and slices; Figma-like frame/components/auto-layout/prototype export; local collaboration/comment/session replay; preflight/package pipeline.
```

### [SFR-PREAMBLE.risks] Risks and Mitigations

```yaml
risks:
  - id: SFR-RISK-001
    risk: "Vendor parity trap: copying every command can produce a fragmented legacy clone."
    failure_scenario: "Studio accumulates separate Photoshop-like, Affinity-like, and InDesign-like subsystems with duplicated layer/state/export logic."
    mitigation: "Use this inventory to extract shared primitives first; require each feature row to map to one shared primitive before implementation."
    verification: "Primitive matrix shows one implementation surface reused by multiple app-reference rows."
  - id: SFR-RISK-002
    risk: "Cloud-only and AI-provider behavior may not be natively reproducible."
    failure_scenario: "Generative Fill, Firefly Boards, cloud documents, Share for Review, or Canva AI behavior is treated as local Rust parity without a provider plan."
    mitigation: "Split local primitives from provider adapters; mark cloud/provider features as adapter-backed or intentionally omitted."
    verification: "Feature rows carry local/provider/omitted posture before build work starts."
  - id: SFR-RISK-003
    risk: "Feature list is category-level and can miss leaf commands."
    failure_scenario: "A work packet claims Photoshop-class parity while missing core subcommands such as channel operations, blend-range controls, PDF import options, or text composition details."
    mitigation: "Run a second pass that explodes official help indexes into command-level rows."
    verification: "Leaf command inventory count is generated from official table-of-contents pages and reconciled to this seed taxonomy."
  - id: SFR-RISK-004
    risk: "Proprietary file compatibility can be overclaimed."
    failure_scenario: "Studio claims AI or Figma local-copy parity without fixtures proving what is preserved, transformed, rasterized, or unsupported."
    mitigation: "Treat proprietary formats as compatibility targets; require import/export fixtures, round-trip receipts, lossy-conversion diagnostics, and recovery behavior before any parity claim."
    verification: "Compatibility tests record preserved features, transformed features, unsupported features, and recovery path for every promoted file-format feature."
```

### [SFR-PREAMBLE.roi] High-ROI Additions

```yaml
high_roi_additions:
  - id: SFR-ROI-001
    addition: "Add `studio_primitive` and `engine_module` fields to every feature row."
    why_high_roi: "Cheap now because every row is already being normalized; prevents app-specific rebuild drift."
    gap_closed: "Links app behavior to Handshake Rust architecture instead of prose intent."
    reuse: "Existing Handshake Studio/Photo Studio, EventLedger, CRDT, diagnostics, and model-tool-contract concepts."
    verification: "Every feature has a non-empty primitive/module mapping or explicit deferred reason."
  - id: SFR-ROI-002
    addition: "Add local-vs-provider posture for AI/cloud/collaboration features."
    why_high_roi: "Prevents future rework around Firefly, Adobe cloud docs, Share for Review, Canva AI, and StudioLink-like workflows."
    gap_closed: "Separates native Rust rebuild scope from adapter-backed external behavior."
    reuse: "Existing Handshake LLM/client-adapter and capability-gate patterns."
    verification: "Provider-dependent rows are tagged before any implementation packet claims local parity."
  - id: SFR-ROI-003
    addition: "Generate a leaf-level command inventory from official help indexes."
    why_high_roi: "Turns a planning map into a mechanically auditable parity ledger."
    gap_closed: "Reduces missed-tool and hidden-subcommand risk."
    reuse: "Current topic/index structure and source-at-EOF convention."
    verification: "Scripted extraction can diff new vendor help pages against stable feature IDs."
  - id: SFR-ROI-004
    addition: "Keep one Feature Use Card shape across Photoshop, Affinity, InDesign, Illustrator, and Figma."
    why_high_roi: "Cheap now because all cards are generated from the same corpus pattern; prevents future manual and command-contract drift."
    gap_closed: "Adds user-purpose and use-context coverage for vector, design-system, prototype, collaboration, and developer-handoff features."
    reuse: "Existing Feature Use Card schema, manual handoff index, provider posture, and primitive maps."
    verification: "Every generated card remains planning_only until source-page/app inspection, command contract, tests, and internal Studio UserManual update."
```

### [SFR-PREAMBLE.sources] Sources

```yaml
sources:
  - id: PRE-S01
    url: "https://helpx.adobe.com/photoshop/desktop.html"
    note: "Photoshop help index used for feature families."
  - id: PRE-S02
    url: "https://helpx.adobe.com/indesign/desktop.html"
    note: "InDesign help index used for feature families."
  - id: PRE-S03
    url: "https://affinity.help/photo2ipad/en-US.lproj/contents.xml"
    note: "Affinity Photo 2 help table of contents used where desktop XML was blocked."
  - id: PRE-S04
    url: "https://affinity.help/designer2ipad/en-US.lproj/contents.xml"
    note: "Affinity Designer 2 help table of contents used where desktop XML was blocked."
  - id: PRE-S05
    url: "https://affinity.help/publisher2ipad/en-US.lproj/contents.xml"
    note: "Affinity Publisher 2 help table of contents used where desktop XML was blocked."
  - id: PRE-S06
    url: "https://helpx.adobe.com/illustrator/desktop.html"
    note: "Illustrator desktop help index used for feature families and leaf coverage."
  - id: PRE-S07
    url: "https://helpx.adobe.com/illustrator/kb/supported-file-formats-illustrator.html"
    note: "Illustrator supported file formats used for compatibility posture."
  - id: PRE-S08
    url: "https://help.figma.com/hc/en-us"
    note: "Figma Help Center used for feature families, category evidence, import/export, and product surfaces."
  - id: PRE-S09
    url: "https://developers.figma.com/"
    note: "Figma developer documentation used for API and extension-surface coverage."
```
