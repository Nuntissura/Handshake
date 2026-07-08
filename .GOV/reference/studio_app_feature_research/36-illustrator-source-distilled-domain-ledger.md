---
file_id: "illustrator-source-distilled-domain-ledger"
topic_id: SFR-ILLUSTRATOR-SOURCE-DISTILLED-DOMAINS
title: "Illustrator Source Distilled Domain Ledger"
status: draft
summary: "Online-source-distilled Illustrator feature/tool domains for Studio parity planning."
sources: 13
updated_at: "2026-07-05"
---

## [SFR-ILLUSTRATOR-SOURCE-DISTILLED-DOMAINS] Illustrator Source Distilled Domain Ledger

### [SFR-ILLUSTRATOR-SOURCE-DISTILLED-DOMAINS.policy] Policy

```yaml
policy:
  distillation_status: "online_source_distilled"
  installed_exports_role: "optional enrichment only"
  rebuild_target: "Handshake Studio local-first Rust vector, typography, illustration, export, prepress, and automation tools with Illustrator-compatible file behavior where source-observable"
  naming_rule: "Illustrator remains source/provenance naming only; Studio surfaces use Handshake-native names."
  coverage_rule: "Merge help leaves, toolbar/tool docs, shortcut docs, scripting/developer docs, supported-format docs, release notes, AI/provider docs, and compatibility pages."
```

### [SFR-ILLUSTRATOR-SOURCE-DISTILLED-DOMAINS.domains] Domains

```yaml
domains:
  - id: ail.domain.workspace_canvas
    name: "Workspace, canvas, artboards, navigation, and document setup"
    app_behavior: "Illustrator works through a vector canvas with documents, artboards, large canvas, templates, rulers, guides, grids, snapping, smart guides, zoom/pan/rotate view, panels, preferences, workspaces, cloud/local documents, and recoverable document state."
    tool_and_feature_scope:
      - "New/open document, artboards, large canvas, canvas navigation, hand, zoom, rotate view, rulers, guides, grids, snap, smart guides, preferences, panels, and workspace management."
      - "Selection context, isolation mode, outline/preview modes, GPU/CPU preview, overprint/pixel preview, and document recovery/version surfaces."
    studio_primitive_domains: [workspace, vector, viewport, file_io, diagnostics]
    source_surfaces: [help_leaf, tool_page, shortcut_row, release_delta]
    manual_topic_candidate: "studio.manual.vector.workspace-and-artboards"
    implementation_notes:
      - "Artboards should be first-class vector document regions with export recipes and layout constraints."
      - "Viewport modes must be scriptable and testable for model-driven visual debugging."
  - id: ail.domain.vector_authoring
    name: "Vector drawing, paths, anchors, shapes, brushes, and shape construction"
    app_behavior: "Illustrator authors editable vector geometry with Pen, Curvature, Pencil, Paintbrush, Blob Brush, Shaper, Line, Arc, Spiral, Grid, Rectangle, Ellipse, Polygon, Star, Flare, Shape Builder, Pathfinder, Live Paint, Width, Mesh, and anchor editing."
    tool_and_feature_scope:
      - "Pen, Add/Delete/Convert Anchor Point, Curvature, Pencil, Smooth, Path Eraser, Join, Paintbrush, Blob Brush, Shaper, shape tools, line tools, flare, slice, width, mesh, live paint, shape builder, and pathfinder."
      - "Path offset, simplify, average, join, outline stroke, expand, compound paths, clipping masks, opacity masks, envelope distort, image trace, and vectorization workflows."
    studio_primitive_domains: [vector, brush_engine, geometry, boolean_ops, mask]
    source_surfaces: [help_leaf, tool_page, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.vector.authoring-tools"
    implementation_notes:
      - "Vector authoring should share primitives with Figma Draw and Affinity Designer."
      - "Path operations need deterministic topology tests, especially compound, clipped, and expanded states."
  - id: ail.domain.object_layout_transform
    name: "Object organization, layers, transforms, symbols, appearances, and reusable assets"
    app_behavior: "Illustrator organizes artwork into objects, groups, layers, sublayers, appearances, graphic styles, symbols, patterns, assets, links, align/distribute, transform operations, envelopes, blends, and object-level metadata."
    tool_and_feature_scope:
      - "Selection, Direct Selection, Group Selection, Magic Wand, Lasso, isolation mode, grouping, locking, hiding, layers, symbols, links, graphic styles, appearance stacking, and assets."
      - "Move, rotate, reflect, scale, shear, free transform, transform each, blend, envelope distort, align, distribute, arrange, repeat, patterns, and object export."
    studio_primitive_domains: [vector, layer, asset_pipeline, geometry, style_system]
    source_surfaces: [help_leaf, tool_page, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.vector.objects-and-transforms"
    implementation_notes:
      - "Appearance stacks should be non-destructive graph nodes, not flattened paint attributes."
      - "Symbols/components should map to shared reusable asset primitives across Illustrator, Affinity, and Figma parity."
  - id: ail.domain.paint_color_appearance
    name: "Paint, color, gradients, mesh, swatches, appearance, transparency, and effects"
    app_behavior: "Illustrator controls vector paint through fills, strokes, swatches, global/process/spot colors, gradients, freeform gradients, meshes, patterns, color groups, recolor, transparency, opacity, blend modes, graphic styles, appearances, and effects."
    tool_and_feature_scope:
      - "Fill/stroke, eyedropper, color panel, swatches, gradient, mesh, live paint, color guide, recolor artwork, color themes, patterns, appearance panel, graphic styles, transparency, opacity masks, and blend modes."
      - "Spot/process colors, separations, overprint, global swatches, tints, pattern editing, and style libraries."
    studio_primitive_domains: [color, vector, prepress, style_system, layer]
    source_surfaces: [help_leaf, tool_page, shortcut_row, file_format_matrix]
    manual_topic_candidate: "studio.manual.vector.color-and-appearance"
    implementation_notes:
      - "Color management must support print-grade spot/process behavior and screen export behavior."
      - "Appearance graph and style reuse should be shared with layout and design-system modules."
  - id: ail.domain.typography
    name: "Type tools, fonts, glyphs, text objects, and advanced typography"
    app_behavior: "Illustrator provides point, area, path, vertical, touch, and regional type tools with character/paragraph formatting, glyphs, OpenType, variable fonts, text-to-vector conversion, font activation, missing-font handling, and type layout."
    tool_and_feature_scope:
      - "Type, Area Type, Type on a Path, Vertical Type variants, Touch Type where available, text wrap, threaded/area text, font search, glyphs, character/paragraph panels, styles, OpenType, variable fonts, and create outlines."
      - "Right-to-left, Indic, CJK, MENA tools where source-observable, font substitution, and packaging/export font behavior."
    studio_primitive_domains: [typography, vector, layout, file_io, prepress]
    source_surfaces: [help_leaf, tool_page, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.vector.typography"
    implementation_notes:
      - "Text must remain editable until explicit outline conversion, with receipts for irreversible conversion."
      - "Typography engine should be shared with Figma, Affinity, and InDesign where possible."
  - id: ail.domain.layers_assets_links
    name: "Layers, assets, linked artwork, libraries, variables, and data-driven graphics"
    app_behavior: "Illustrator uses layers, links, embedded assets, libraries, symbols, templates, variables, datasets, graph/data visualization features, and asset export to manage reusable and external content."
    tool_and_feature_scope:
      - "Layers, sublayers, templates, links, embed/relink/update, placed raster/vector/PDF, libraries, symbols, variables, datasets, graphs/charts where documented, asset export, and package."
      - "Missing-link recovery, modified-link detection, asset replacement, and export naming."
    studio_primitive_domains: [asset_pipeline, layer, vector, data_binding, export]
    source_surfaces: [help_leaf, file_format_matrix, scripting_api, provider_or_cloud]
    manual_topic_candidate: "studio.manual.vector.assets-and-data"
    implementation_notes:
      - "Data-driven graphics need deterministic binding contracts and validation for missing fields."
      - "Linked assets should use the same local resolver and receipts as Photoshop and InDesign."
  - id: ail.domain.effects_3d_web
    name: "Effects, filters, 3D/materials, raster interop, SVG, CSS, and web/screen output"
    app_behavior: "Illustrator applies vector and raster effects, 3D/materials, stylization, distort/transform, path effects, rasterization, image trace, SVG/CSS/web export, screen export, and pixel/screen preview workflows."
    tool_and_feature_scope:
      - "Effect menu families, appearance-applied effects, raster effects settings, 3D and materials, extrude/revolve/inflate where documented, shadows/glows/feathers, distort/transform, stylize, warp, and path effects."
      - "Image trace, rasterize, SVG effects, CSS properties, asset export, Export for Screens, web graphics, and pixel preview."
    studio_primitive_domains: [vector, raster, gpu_pipeline, export, web]
    source_surfaces: [help_leaf, tool_page, shortcut_row, file_format_matrix, release_delta]
    manual_topic_candidate: "studio.manual.vector.effects-and-web-export"
    implementation_notes:
      - "Effects should remain editable appearance nodes where source behavior is non-destructive."
      - "SVG/CSS export needs fixture coverage for gradients, masks, blend modes, text, and effects fallbacks."
  - id: ail.domain.automation_data
    name: "Actions, scripts, variables, developer APIs, plugins, and automation"
    app_behavior: "Illustrator supports automation through Actions, scripts, variables/datasets, scripting object model, developer/plugin documentation, batch operations, menu commands, and workflow recording/replay where documented."
    tool_and_feature_scope:
      - "Actions, batch, scripts, startup scripts, JavaScript/ExtendScript DOM, application/document/layer/pageItem/path/text/swatch objects, variables, datasets, graph/data workflows, and plugin/developer hooks."
      - "Scriptable import/export, document generation, asset production, and command execution."
    studio_primitive_domains: [automation, scripting, plugin_api, command_contracts, batch]
    source_surfaces: [help_leaf, scripting_api, shortcut_row, release_delta]
    manual_topic_candidate: "studio.manual.automation.vector-illustration"
    implementation_notes:
      - "Studio should expose vector commands as typed Rust APIs and scriptable commands from the start."
      - "Automation receipts should capture selected objects, document mutations, and export outputs."
  - id: ail.domain.generative_ai
    name: "Generative AI, Firefly vector generation, recolor, and provider-backed creation"
    app_behavior: "Illustrator source surfaces include provider-backed generative vector workflows such as text-to-vector/shape, generative recolor, generated patterns or fills where documented, model/provider selection, variations, prompt controls, and generated editable artwork."
    tool_and_feature_scope:
      - "Prompt-based vector generation, generated shape/fill/pattern/recolor workflows, variants, contextual generation surfaces, provider errors, and generated vector editability."
      - "Local-first fallback strategies and provider metadata for generated artwork."
    studio_primitive_domains: [ai, provider_adapter, vector, color, asset_pipeline]
    source_surfaces: [help_leaf, provider_or_cloud, release_delta]
    manual_topic_candidate: "studio.manual.ai.vector-generation"
    implementation_notes:
      - "Provider behavior must remain optional; generated artwork should become editable local vector state after creation."
      - "Receipts must record prompt, source selection, provider/model, variant, and generated object graph."
  - id: ail.domain.file_io_export_prepress
    name: "File compatibility, import/export, packaging, print, PDF, SVG, and prepress"
    app_behavior: "Illustrator opens, places, saves, exports, packages, prints, and preflights artwork across AI/AIT, PDF, EPS, SVG, PSD, raster formats, DWG/DXF, text, CSS, web/screen exports, print separations, and compatibility options."
    tool_and_feature_scope:
      - "Open/place/import, save, save as, save a copy, export, export for screens, asset export, package, print, PDF presets, EPS/SVG options, raster options, DWG/DXF import/export, and text/CSS-related output."
      - "Color profiles, overprint, spot colors, separations, transparency flattening, font embedding/outlining, linked assets, and compatibility versioning."
    studio_primitive_domains: [file_io, export, pdf, svg, prepress, print]
    source_surfaces: [file_format_matrix, help_leaf, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.file-compatibility.vector-formats"
    implementation_notes:
      - "AI format compatibility needs fixture-based import/export contracts and unsupported-feature diagnostics."
      - "SVG/PDF/EPS output needs exact option receipts and round-trip comparison tests."
  - id: ail.domain.collaboration_cloud_recovery
    name: "Collaboration, cloud assets, comments, libraries, versioning, and recovery"
    app_behavior: "Illustrator includes cloud documents, review/share, comments, libraries, linked provider assets, version/recovery surfaces, and cloud-sync troubleshooting where source-observable."
    tool_and_feature_scope:
      - "Share/review links, comments, cloud document open/save, libraries, asset sync, version history, recovery, and provider/account error states."
      - "Local-first equivalents for review packages, comments, asset registries, and version snapshots."
    studio_primitive_domains: [collaboration, provider_adapter, asset_pipeline, versioning, diagnostics]
    source_surfaces: [help_leaf, provider_or_cloud, release_delta]
    manual_topic_candidate: "studio.manual.collaboration.vector-review"
    implementation_notes:
      - "Provider-backed collaboration should translate to optional adapters over local project state."
      - "Recovery must be deterministic and inspectable through logs, receipts, and state snapshots."
```

### [SFR-ILLUSTRATOR-SOURCE-DISTILLED-DOMAINS.sources] Sources

```yaml
sources:
  - { id: AIL-S01, url: "https://helpx.adobe.com/illustrator/desktop.html", note: "Official Illustrator desktop help source." }
  - { id: AIL-S02, path: "_source_snapshots/adobe-illustrator-desktop-jina.md", note: "Local reader snapshot of AIL-S01 used for help-leaf extraction." }
  - { id: AIL-S03, url: "https://helpx.adobe.com/illustrator/using/tools.html", note: "Illustrator tools source surface." }
  - { id: AIL-S04, path: "_source_snapshots/illustrator-tools-jina.md", note: "Local tools snapshot." }
  - { id: AIL-S05, path: "_source_snapshots/illustrator-toolbar-jina.md", note: "Local toolbar snapshot." }
  - { id: AIL-S06, url: "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html", note: "Illustrator keyboard shortcut source surface." }
  - { id: AIL-S07, path: "_source_snapshots/illustrator-default-keyboard-shortcuts-jina.md", note: "Local shortcut snapshot." }
  - { id: AIL-S08, path: "_source_snapshots/illustrator-supported-file-formats-jina.md", note: "Local supported-file-format snapshot." }
  - { id: AIL-S09, path: "_source_snapshots/illustrator-scripting-jina.md", note: "Local scripting snapshot." }
  - { id: AIL-S10, path: "_source_snapshots/illustrator-developer-jina.md", note: "Local developer snapshot." }
  - { id: AIL-S11, path: "22-illustrator-leaf-index.md", note: "Generated Illustrator help leaf index." }
  - { id: AIL-S12, path: "24-illustrator-feature-use-cards.md", note: "Generated Illustrator Feature Use Cards." }
  - { id: AIL-S13, path: "31-illustrator-expanded-count-ledger.md", note: "Expanded Illustrator online-source count ledger." }
```
