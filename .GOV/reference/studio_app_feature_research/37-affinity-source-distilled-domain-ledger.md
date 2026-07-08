---
file_id: "affinity-source-distilled-domain-ledger"
topic_id: SFR-AFFINITY-SOURCE-DISTILLED-DOMAINS
title: "Affinity Source Distilled Domain Ledger"
status: draft
summary: "Online-source-distilled Affinity Photo, Designer, and Publisher feature/tool domains for Studio parity planning."
sources: 12
updated_at: "2026-07-05"
---

## [SFR-AFFINITY-SOURCE-DISTILLED-DOMAINS] Affinity Source Distilled Domain Ledger

### [SFR-AFFINITY-SOURCE-DISTILLED-DOMAINS.policy] Policy

```yaml
policy:
  distillation_status: "online_source_distilled"
  installed_exports_role: "not applicable for source distillation; desktop installed inspection remains optional if available"
  rebuild_target: "Handshake Studio local-first Rust raster, vector, and publishing tools with Affinity-compatible behavior and file interchange where source-observable"
  naming_rule: "Affinity product names remain source/provenance naming only; Studio surfaces use Handshake-native names."
  coverage_rule: "Merge Affinity help leaves, desktop delta rows, persona/tool/panel docs, command/workflow docs, and file-format/compatibility docs."
```

### [SFR-AFFINITY-SOURCE-DISTILLED-DOMAINS.domains] Domains

```yaml
domains:
  - id: aff.domain.personas_and_unified_workspaces
    name: "Personas, unified workspace, StudioLink, panels, and shared app shell"
    app_behavior: "Affinity organizes capability through personas and a shared Studio panel model across Photo, Designer, and Publisher, including StudioLink-style cross-app editing, persona switching, tool contexts, panels, assets, resources, preferences, snapping, grids, guides, and document tabs."
    tool_and_feature_scope:
      - "Photo Persona, Develop Persona, Liquify Persona, Tone Mapping Persona, Export Persona, Designer Persona, Pixel Persona, Publisher Persona, and shared Studio panels."
      - "StudioLink cross-persona workflows, panel docking, context toolbar, tool presets, assets, resources, grids/guides, snapping, history, navigator, transform, color, swatches, layers, and effects."
    studio_primitive_domains: [workspace, raster, vector, page_layout, asset_pipeline]
    source_surfaces: [help_leaf, desktop_delta, tool_page, panel_page]
    manual_topic_candidate: "studio.manual.workspace.persona-style-modules"
    implementation_notes:
      - "Studio should use one native workspace with mode-specific tool groups rather than cloning vendor persona names."
      - "Cross-domain editing must preserve shared document state across raster, vector, and publishing modules."
  - id: aff.domain.photo_imaging
    name: "Photo raster editing, raw development, selections, masks, adjustments, live filters, and retouch"
    app_behavior: "Affinity Photo covers raster/photo work: RAW development, layer editing, masks, selections, adjustments, live filters, retouching, inpainting, frequency separation, liquify, tone mapping, HDR, panorama, stacks, focus merge, macros, batch, and export."
    tool_and_feature_scope:
      - "Move, View, Crop, Selection Brush, Flood Select, Marquee, Freehand Selection, Paint Brush, Pixel, Erase, Clone, Healing, Inpainting, Patch, Blemish Removal, Dodge, Burn, Sponge, Blur, Sharpen, Smudge, Gradient, Flood Fill, Mesh Warp, Liquify, and Develop tools."
      - "RAW panels, overlays, adjustments, live filters, masks, channels, blend ranges, frequency separation, HDR merge, panorama, focus merge, stacks, tone mapping, macros, batch jobs, and export slices."
    studio_primitive_domains: [raster, camera_raw, selection, mask, color, layer, brush_engine]
    source_surfaces: [help_leaf, desktop_delta, tool_page, command_workflow]
    manual_topic_candidate: "studio.manual.photo.affinity-class-imaging"
    implementation_notes:
      - "Live filters and adjustments should map to shared non-destructive graph nodes."
      - "Macro/batch behavior should be implemented through Studio command contracts, not a separate recorder-only path."
  - id: aff.domain.vector_design
    name: "Designer vector tools, shapes, curves, pixel persona, constraints, symbols, and export"
    app_behavior: "Affinity Designer covers vector illustration and UI asset work through vector tools, curves, shapes, pens, pencils, brushes, booleans, symbols, constraints, artboards, export persona, slices, raster/pixel persona editing, and asset management."
    tool_and_feature_scope:
      - "Move, Node, Point Transform, Pen, Pencil, Vector Brush, Corner, Contour, Shape Builder, Knife, Fill, Transparency, Gradient, Mesh Gradient, shapes, Artistic Text, Frame Text, Place Image, Artboard, Slice, and Export Persona tools."
      - "Boolean operations, curves, compound objects, symbols, constraints, assets, grids, snapping, vector brushes, pixel persona tools, artboards, and slice/export workflows."
    studio_primitive_domains: [vector, geometry, boolean_ops, brush_engine, design_systems, export]
    source_surfaces: [help_leaf, desktop_delta, tool_page, panel_page]
    manual_topic_candidate: "studio.manual.vector.affinity-class-design"
    implementation_notes:
      - "Designer vector primitives should align with Illustrator and Figma Draw parity."
      - "Export Persona behavior is a strong model for local-first batch asset export recipes."
  - id: aff.domain.publishing_layout
    name: "Publisher pages, spreads, masters, frames, preflight, package, and PDF"
    app_behavior: "Affinity Publisher covers page layout and publishing through documents, pages, spreads, master pages, frames, text flow, placed graphics, styles, tables, TOC/index, preflight, package, print, and PDF/export."
    tool_and_feature_scope:
      - "Move, Node, Frame Text, Artistic Text, Picture Frame, Table, Pen, Pencil, shape tools, Fill, Transparency, Place Image, Vector Crop, Measure, Area, View, and Zoom."
      - "Pages/spreads, master pages, sections, text frames, image frames, linked resources, text wrap, styles, tables, TOC/index, preflight, package, print, and PDF export."
    studio_primitive_domains: [page_layout, master_pages, typography, tables, prepress, export]
    source_surfaces: [help_leaf, desktop_delta, tool_page, command_workflow]
    manual_topic_candidate: "studio.manual.layout.affinity-class-publishing"
    implementation_notes:
      - "Publisher parity should share InDesign-style layout and prepress primitives."
      - "Package/preflight should produce machine-readable reports for model operators."
  - id: aff.domain.typography_and_text
    name: "Typography, text frames, styles, glyphs, OpenType, text flow, and tables"
    app_behavior: "Affinity apps include artistic text, frame text, path text, typography panels, character/paragraph formatting, styles, glyphs, OpenType, spelling, find/replace, text wrapping, linked frames, and tables where applicable."
    tool_and_feature_scope:
      - "Artistic Text, Frame Text, text on path, character and paragraph panels, text styles, glyph browser, OpenType, bullets/numbering, decorations, tabs, text wrap, frame linking, spelling, find/replace, and tables."
      - "Publisher-specific long-document text, Designer text objects, Photo text layers, and conversion/outline behavior."
    studio_primitive_domains: [typography, text_engine, layout, style_system, tables]
    source_surfaces: [help_leaf, desktop_delta, tool_page]
    manual_topic_candidate: "studio.manual.typography.affinity-class-text"
    implementation_notes:
      - "Text engine should use a shared model across raster, vector, and layout documents."
      - "Style and text-flow behavior should be fixture-tested for reflow and export."
  - id: aff.domain.color_prepress_and_design_aids
    name: "Color, swatches, gradients, effects, grids, snapping, resources, and prepress"
    app_behavior: "Affinity provides color and design-aid systems including color panels, swatches, palettes, gradients, transparency, blend modes, effects, styles, assets, resources, grids, guides, snapping, constraints, color management, print, and prepress checks."
    tool_and_feature_scope:
      - "Color, swatches, gradients, transparency, effects, styles, blend ranges, color formats, ICC/profile behavior, grids, guides, snapping, symbols/assets, resources manager, and constraints."
      - "Publisher preflight, bleed, printer marks, separations-related output where documented, and PDF export settings."
    studio_primitive_domains: [color, prepress, style_system, asset_pipeline, layout]
    source_surfaces: [help_leaf, desktop_delta, panel_page, file_format_matrix]
    manual_topic_candidate: "studio.manual.color.affinity-class-design-aids"
    implementation_notes:
      - "Resource management should feed the same local asset resolver used by Adobe-class import/export."
      - "Color behavior needs print and screen fixtures."
  - id: aff.domain.tools_by_app
    name: "Tool inventories by app family"
    app_behavior: "Affinity tool surfaces are app-specific but share a large cross-app tool vocabulary. Studio should preserve the complete tool intent while consolidating shared local implementations."
    tool_and_feature_scope:
      photo_2:
        - "Move, View, Zoom, Crop, Selection Brush, Flood Select, Marquee, Freehand Selection, Paint Brush, Pixel, Erase, Clone, Healing, Inpainting, Patch, Blemish Removal, Red Eye, Dodge, Burn, Sponge, Blur, Sharpen, Smudge, Gradient, Flood Fill, Mesh Warp, Pen, Node, Shape, Text, Place Image, Color Picker, Measure."
      designer_2:
        - "Move, Node, Point Transform, Pen, Pencil, Vector Brush, Corner, Contour, Shape Builder, Knife, Fill, Transparency, Gradient, Mesh Gradient, shapes, Artistic Text, Frame Text, Place Image, Artboard, Slice, Pixel Persona raster tools, and Export Persona slice tools."
      publisher_2:
        - "Move, Node, Frame Text, Artistic Text, Picture Frame, Table, Pen, Pencil, vector shapes, Fill, Transparency, Place Image, Vector Crop, Measure, Area, View, Zoom, and StudioLink persona tools."
    studio_primitive_domains: [workspace, raster, vector, typography, page_layout, export]
    source_surfaces: [help_leaf, desktop_delta, tool_page]
    manual_topic_candidate: "studio.manual.tools.affinity-source-inventory"
    implementation_notes:
      - "These app tool lists should seed the Studio tool registry and deduplicate shared implementations."
      - "Tool names are source evidence; shipped Studio tool names can be native as long as behavior is preserved."
  - id: aff.domain.studio_panels
    name: "Studio panels, inspectors, history, assets, resources, and diagnostics"
    app_behavior: "Affinity Studio panels expose state and workflows for layers, color, swatches, brushes, adjustments, effects, assets, resources, typography, pages, preflight, export, history, channels, macros, navigator, transform, constraints, symbols, and document metadata."
    tool_and_feature_scope:
      - "Photo panels: Layers, Adjustments, Effects, Brushes, Color, Swatches, Channels, History, Navigator, Transform, Macro, Library, Assets, Stock, Histogram, Scope, and related panels."
      - "Designer panels: Layers, Assets, Symbols, Constraints, Appearance, Stroke, Brushes, Color, Swatches, Transform, Export, Navigator, History, and related panels."
      - "Publisher panels: Pages, Text Styles, Table, Preflight, Resources, Layers, Assets, Color, Swatches, Text Wrap, Index, TOC, Navigator, History, and related panels."
    studio_primitive_domains: [workspace, diagnostics, asset_pipeline, style_system, prepress]
    source_surfaces: [help_leaf, desktop_delta, panel_page]
    manual_topic_candidate: "studio.manual.panels.affinity-class-inspectors"
    implementation_notes:
      - "Studio panels should expose the same state through structured diagnostics and model-facing APIs."
      - "Panel state should not be the only authority; canonical state belongs in document models."
  - id: aff.domain.commands_and_workflow_surfaces
    name: "Commands, personas, macros, batch, export, resource management, and recovery"
    app_behavior: "Affinity workflows include command menus, persona workflows, non-destructive edits, macros, batch jobs, export persona, resource management, package/collect behavior where documented, undo/history, snapshots, and recovery."
    tool_and_feature_scope:
      - "Macro recording/playback, batch jobs, export persona slices, resource manager, package/export workflows, snapshots, history, save history with document where supported, and document recovery."
      - "Persona-specific workflows for RAW develop, liquify, tone mapping, pixel editing, vector design, publishing, and export."
    studio_primitive_domains: [automation, command_contracts, batch, export, versioning]
    source_surfaces: [help_leaf, desktop_delta, command_workflow]
    manual_topic_candidate: "studio.manual.automation.affinity-class-workflows"
    implementation_notes:
      - "Batch and macro workflows should compile to Studio command sequences with receipts."
      - "Recovery and history behavior should be deterministic and portable."
  - id: aff.domain.compatibility_and_formats
    name: "Native documents, PSD/PDF/SVG/EPS/AI-compatible import, raster formats, and export"
    app_behavior: "Affinity source surfaces document native documents, PSD interchange, PDF import/export, SVG/EPS/vector interchange, AI/PDF-compatible import behavior, raster formats, RAW formats, package/export workflows, and app-specific compatibility constraints."
    tool_and_feature_scope:
      - "AFPHOTO, AFDESIGN, AFPUB, PSD, PDF, SVG, EPS, AI/PDF-compatible, TIFF, JPEG, PNG, GIF, WebP, EXR/HDR where documented, RAW formats, and publication export formats."
      - "Import/export options, unsupported-feature handling, round-trip expectations, color/profile handling, text/font preservation, and layered data preservation."
    studio_primitive_domains: [file_io, export, pdf, svg, raster, vector, page_layout]
    source_surfaces: [help_leaf, desktop_delta, file_format_matrix]
    manual_topic_candidate: "studio.manual.file-compatibility.affinity-class"
    implementation_notes:
      - "Compatibility must be fixture-driven with explicit preservation and loss reports."
      - "Affinity native files are compatibility targets; Studio should not replace them with a new interchange format."
```

### [SFR-AFFINITY-SOURCE-DISTILLED-DOMAINS.sources] Sources

```yaml
sources:
  - { id: AFF-S01, url: "https://affinity.help/", note: "Official Affinity help source." }
  - { id: AFF-S02, path: "_source_snapshots/affinity-photo2-desktop-jina.md", note: "Local Affinity Photo 2 desktop help snapshot." }
  - { id: AFF-S03, path: "_source_snapshots/affinity-designer2-desktop-jina.md", note: "Local Affinity Designer 2 desktop help snapshot." }
  - { id: AFF-S04, path: "_source_snapshots/affinity-publisher2-desktop-jina.md", note: "Local Affinity Publisher 2 desktop help snapshot." }
  - { id: AFF-S05, path: "_source_snapshots/photo2ipad-contents.xml", note: "Official Affinity Photo 2 iPad help XML snapshot used for leaf expansion." }
  - { id: AFF-S06, path: "_source_snapshots/designer2ipad-contents.xml", note: "Official Affinity Designer 2 iPad help XML snapshot used for leaf expansion." }
  - { id: AFF-S07, path: "_source_snapshots/publisher2ipad-contents.xml", note: "Official Affinity Publisher 2 iPad help XML snapshot used for leaf expansion." }
  - { id: AFF-S08, path: "02-affinity-suite-feature-map.md", note: "Affinity suite feature family map." }
  - { id: AFF-S09, path: "04-affinity-leaf-index.md", note: "Generated Affinity V2 help leaf index." }
  - { id: AFF-S10, path: "09-affinity-desktop-delta.md", note: "Affinity desktop delta rows." }
  - { id: AFF-S11, path: "16-affinity-feature-use-cards.md", note: "Generated Affinity Feature Use Cards." }
  - { id: AFF-S12, path: "12-cross-app-parity-matrix.md", note: "Cross-app parity matrix including Affinity." }
```
