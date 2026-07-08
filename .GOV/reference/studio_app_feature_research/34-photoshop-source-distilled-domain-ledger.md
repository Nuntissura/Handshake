---
file_id: "photoshop-source-distilled-domain-ledger"
topic_id: SFR-PHOTOSHOP-SOURCE-DISTILLED-DOMAINS
title: "Photoshop Source Distilled Domain Ledger"
status: draft
summary: "Online-source-distilled Photoshop and Camera Raw feature/tool domains for Studio parity planning."
sources: 13
updated_at: "2026-07-05"
---

## [SFR-PHOTOSHOP-SOURCE-DISTILLED-DOMAINS] Photoshop Source Distilled Domain Ledger

### [SFR-PHOTOSHOP-SOURCE-DISTILLED-DOMAINS.policy] Policy

```yaml
policy:
  distillation_status: "online_source_distilled"
  installed_exports_role: "optional enrichment only"
  rebuild_target: "Handshake Studio local-first Rust tools with Photoshop-compatible file, workflow, and behavior coverage where source-observable"
  naming_rule: "Photoshop remains source/provenance naming only; Studio surfaces use Handshake-native names."
  coverage_rule: "Do not stop at Photoshop help leaves. Merge help TOC, toolbar docs, shortcut docs, scripting/UXP docs, file-format docs, Camera Raw docs, provider docs, and release deltas."
```

### [SFR-PHOTOSHOP-SOURCE-DISTILLED-DOMAINS.domains] Domains

```yaml
domains:
  - id: psd.domain.workspace_and_ui
    name: "Workspace, panels, tools, preferences, and navigation"
    app_behavior: "Photoshop exposes a configurable workspace with document tabs, panel groups, toolbar customization, tool presets, context-sensitive options, history, rulers, guides, grids, snapping, shortcuts, preferences, screen modes, zoom, pan, rotate view, and extension surfaces."
    tool_and_feature_scope:
      - "Move, Artboard, Hand, Rotate View, Zoom, Eyedropper, Measure, Count, Note, and navigation-related tools."
      - "Toolbar customization, tool nesting, tool presets, context options, panel docks, custom workspaces, shortcut sets, and menu command access."
      - "History, snapshots, undo/redo behavior, guides, grids, rulers, snapping, screen modes, and workspace reset."
    studio_primitive_domains: [workspace, command_palette, viewport, diagnostics, automation]
    source_surfaces: [help_leaf, toolbar_page, shortcut_row, scripting_api, uxp_api]
    manual_topic_candidate: "studio.manual.workspace.photoshop-class-navigation"
    implementation_notes:
      - "Model-facing operations need deterministic command receipts for active tool, active document, selection state, and visible panel state."
      - "Toolbar customization should become data-driven tool registry configuration rather than vendor-named UI cloning."
  - id: psd.domain.document_file_io
    name: "Documents, presets, templates, file open/save/export, and metadata"
    app_behavior: "Photoshop creates, opens, places, links, saves, exports, packages, embeds, prints, and annotates documents with color profiles, metadata, presets, recent files, cloud/local documents, and multi-format compatibility."
    tool_and_feature_scope:
      - "New document presets, artboards, canvas size, image size, resolution, bit depth, color mode, duplicate, place embedded, place linked, import, export, save a copy, and print."
      - "PSD/PSB, TIFF, JPEG, PNG, GIF, WebP, PDF, SVG-related export, camera/raw formats, video/image sequence where supported, metadata, XMP, and color profile handling."
      - "Asset export, slices, generator-style asset output, Save for Web lineage, batch export, and format-specific option dialogs."
    studio_primitive_domains: [file_io, export, color, asset_pipeline, metadata]
    source_surfaces: [help_leaf, file_format_matrix, shortcut_row, scripting_api, release_delta]
    manual_topic_candidate: "studio.manual.file-compatibility.photoshop-class-documents"
    implementation_notes:
      - "Compatibility must be fixture-driven, with import/export receipts that record preserved, translated, and unsupported constructs."
      - "Do not invent a replacement interchange format for parity scope; preserve existing creative formats through adapters."
  - id: psd.domain.layers_and_non_destructive_graph
    name: "Layers, masks, groups, Smart Objects, effects, and non-destructive editing"
    app_behavior: "Photoshop centers editing around a non-destructive layer graph with pixel layers, adjustment layers, fill layers, type layers, shape layers, Smart Objects, groups, artboards, masks, clipping, blend modes, opacity, layer styles, layer comps, and linked assets."
    tool_and_feature_scope:
      - "Layer creation, duplication, locking, linking, grouping, merging, flattening, rasterizing, converting to Smart Object, embedded/linked Smart Objects, and layer comps."
      - "Layer masks, vector masks, clipping masks, blend modes, opacity/fill, layer effects, adjustment/fill layers, knockout, advanced blending, and compositing order."
      - "Layer search/filtering, align/distribute layers, auto-align, auto-blend, and artboard/layer export."
    studio_primitive_domains: [layer, mask, vector, raster, export]
    source_surfaces: [help_leaf, shortcut_row, scripting_api, uxp_api]
    manual_topic_candidate: "studio.manual.layer-graph.photoshop-class-nondestructive"
    implementation_notes:
      - "This should map to the shared StudioLayerGraph rather than one-off raster commands."
      - "Smart Object parity needs embedded asset state, linked asset state, transform stack, and edit-in-place receipts."
  - id: psd.domain.selections_and_masks
    name: "Selections, segmentation, masks, channels, and extraction"
    app_behavior: "Photoshop provides manual, assisted, and AI-backed selection workflows that create pixel selections, masks, alpha channels, object/subject/sky selections, color/focus/range selections, Select and Mask refinements, and reusable channels."
    tool_and_feature_scope:
      - "Marquee, Lasso, Polygonal Lasso, Magnetic Lasso, Object Selection, Quick Selection, Magic Wand, Select Subject, Select Sky, Color Range, Focus Area, Grow, Similar, Feather, Modify, Transform Selection, and Save/Load Selection."
      - "Select and Mask workspace, edge detection, radius, refine edge brush, output to selection/mask/new layer, alpha channels, quick mask mode, and channel operations."
    studio_primitive_domains: [selection, mask, ai, raster, diagnostics]
    source_surfaces: [help_leaf, toolbar_page, shortcut_row, provider_or_cloud]
    manual_topic_candidate: "studio.manual.selection.photoshop-class-masking"
    implementation_notes:
      - "AI-backed segmentation must have local fallback or provider posture rows; manual selection remains local-first."
      - "Selection state should be serializable and testable independently from the visible marching-ants UI."
  - id: psd.domain.crop_transform_geometry
    name: "Crop, resize, transforms, warp, perspective, and geometry"
    app_behavior: "Photoshop changes canvas, pixels, layer transforms, perspective, object geometry, crop boundaries, content-aware scaling, warp meshes, puppet warps, perspective warps, and camera/lens geometry."
    tool_and_feature_scope:
      - "Crop, Perspective Crop, Slice, Frame, Content-Aware Scale, Free Transform, Transform Again, Scale, Rotate, Skew, Distort, Perspective, Warp, Puppet Warp, Perspective Warp, Vanishing Point, and lens correction geometry."
      - "Canvas Size, Image Size, rotation, trim, reveal all, align/distribute, and artboard geometry."
    studio_primitive_domains: [geometry, raster, vector, layer, export]
    source_surfaces: [help_leaf, toolbar_page, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.geometry.photoshop-class-transform"
    implementation_notes:
      - "Transforms need editable matrices and non-destructive history when applied to layer graph nodes."
      - "Destructive pixel commits require before/after receipts and recovery guidance."
  - id: psd.domain.retouch_repair_compositing
    name: "Retouch, repair, cleanup, cloning, and compositing"
    app_behavior: "Photoshop supports targeted pixel repair and compositing through healing, cloning, patching, content-aware fill, remove, red-eye, dodge/burn/sponge, blur/sharpen/smudge, object cleanup, sky replacement, and composite blending workflows."
    tool_and_feature_scope:
      - "Remove, Spot Healing Brush, Healing Brush, Patch, Content-Aware Move, Clone Stamp, Pattern Stamp, Red Eye, Dodge, Burn, Sponge, Blur, Sharpen, Smudge, and history brush lineage."
      - "Content-Aware Fill workspace, generate/cleanup variants, sample area controls, fill output, and retouch-on-new-layer workflows."
    studio_primitive_domains: [raster, ai, layer, mask, history]
    source_surfaces: [help_leaf, toolbar_page, provider_or_cloud, shortcut_row]
    manual_topic_candidate: "studio.manual.retouch.photoshop-class-repair"
    implementation_notes:
      - "Local repair tools should be deterministic where possible; provider-backed generative repair must keep source masks, prompts, and variant receipts."
      - "Retouch tools need brush engine integration, sampling state, and non-destructive layer recommendations."
  - id: psd.domain.color_tone_and_color_management
    name: "Color, tone, adjustments, profiles, HDR, and color management"
    app_behavior: "Photoshop edits tone and color through adjustments, color modes, profiles, HDR support, LUTs, swatches, gradients, curves, levels, Camera Raw filters, proofing, soft proof, gamut warnings, separations, and profile conversion."
    tool_and_feature_scope:
      - "Levels, Curves, Exposure, Vibrance, Hue/Saturation, Color Balance, Black and White, Photo Filter, Channel Mixer, Color Lookup, Gradient Map, Selective Color, Shadows/Highlights, HDR toning, and adjustment layers."
      - "RGB, CMYK, Lab, Grayscale, Bitmap, Duotone, Indexed Color, multichannel, bit depth, color settings, assign/convert profile, proof setup, and gamut warnings."
    studio_primitive_domains: [color, layer, raster, prepress, camera_raw]
    source_surfaces: [help_leaf, shortcut_row, file_format_matrix, scripting_api]
    manual_topic_candidate: "studio.manual.color.photoshop-class-adjustments"
    implementation_notes:
      - "Color operations need explicit profile-aware pipelines and regression fixtures across bit depths."
      - "Adjustment layers should share the non-destructive graph with Affinity and layout/prepress engines."
  - id: psd.domain.painting_fills_and_patterns
    name: "Painting, brushes, fills, gradients, patterns, and textures"
    app_behavior: "Photoshop provides brush-based painting, erasing, filling, patterning, gradient creation, mixer brush behavior, symmetry, brush presets, texture/scatter dynamics, color sampling, and pattern libraries."
    tool_and_feature_scope:
      - "Brush, Pencil, Mixer Brush, Color Replacement, Eraser, Background Eraser, Magic Eraser, Gradient, Paint Bucket, pattern fill, content-aware fill, and brush settings."
      - "Brush presets, brush libraries, smoothing, symmetry, pressure/tilt dynamics, textures, scattering, dual brush, wet edges, and tool presets."
    studio_primitive_domains: [brush_engine, raster, color, asset_pipeline]
    source_surfaces: [help_leaf, toolbar_page, shortcut_row]
    manual_topic_candidate: "studio.manual.paint.photoshop-class-brushes"
    implementation_notes:
      - "Brush engine should be shared by raster painting, mask painting, vector brush previews, and retouch tools."
      - "Preset import/export and reproducible stroke receipts are required for model-driven operation."
  - id: psd.domain.vectors_shapes_type_layout
    name: "Vector paths, shapes, type, artboards, and design layout"
    app_behavior: "Photoshop includes vector path and design-layout tools: Pen, Curvature Pen, shape tools, path selection, vector masks, type layers, paragraph/character controls, glyphs, OpenType, artboards, frames, and design asset export."
    tool_and_feature_scope:
      - "Pen, Freeform Pen, Curvature Pen, Add/Delete/Convert Anchor Point, Path Selection, Direct Selection, Rectangle, Ellipse, Polygon, Line, Custom Shape, Frame, Horizontal/Vertical Type, Type Mask, and text warp."
      - "Character and paragraph panels, glyphs, fonts, styles, variable/OpenType controls, path operations, shape properties, and artboard workflows."
    studio_primitive_domains: [vector, typography, layout, layer, export]
    source_surfaces: [help_leaf, toolbar_page, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.vector-type.photoshop-class-design"
    implementation_notes:
      - "Photoshop vector/type behavior should reuse Illustrator/Figma vector and text primitives where semantics overlap."
      - "Shape layers need both vector editability and raster compositing behavior."
  - id: psd.domain.filters_effects_and_ai_filters
    name: "Filters, effects, liquify, neural filters, and procedural image operations"
    app_behavior: "Photoshop applies destructive and non-destructive filters, Smart Filters, filter gallery effects, blur/sharpen/distort/noise/render/stylize/video/other filters, Liquify, Vanishing Point, Camera Raw Filter, Adaptive/Neural filters, and effect previews."
    tool_and_feature_scope:
      - "Smart Filters, Filter Gallery, Blur Gallery, Lens Blur, Liquify, Displace, Distort, Noise, Render, Sharpen, Stylize, Pixelate, Other, and Camera Raw Filter."
      - "Neural Filters, adaptive presets, skin/smart portrait/colorization style filters where documented, preview toggles, masks, and output targets."
    studio_primitive_domains: [raster, layer, ai, gpu_pipeline, diagnostics]
    source_surfaces: [help_leaf, provider_or_cloud, release_delta, shortcut_row]
    manual_topic_candidate: "studio.manual.filters.photoshop-class-effects"
    implementation_notes:
      - "Separate local deterministic filters, GPU shaders, and provider-backed AI filters in the command contract."
      - "Smart Filters should preserve editable parameters in the layer graph."
  - id: psd.domain.generative_ai
    name: "Generative AI, prompt-based editing, and provider-backed variants"
    app_behavior: "Photoshop exposes prompt and context driven generation such as Generative Fill, Generative Expand, background/object generation, remove/distraction cleanup, model/provider selection where documented, variant review, and generated layer outputs."
    tool_and_feature_scope:
      - "Prompt, mask/selection-based generation, contextual task bar, reference and variant workflows, generated layers, prompt history where available, and provider safety/error states."
      - "Offline fallback, provider adapter, credits/rate/error receipts, and local-first alternatives where feasible."
    studio_primitive_domains: [ai, provider_adapter, layer, mask, asset_pipeline]
    source_surfaces: [help_leaf, provider_or_cloud, release_delta]
    manual_topic_candidate: "studio.manual.ai.photoshop-class-generation"
    implementation_notes:
      - "Provider-backed commands must remain optional integrations, not hard dependencies for local Studio."
      - "Receipts must record prompt, mask, source bounds, model/provider, seed/variant if available, and output placement."
  - id: psd.domain.camera_raw_development
    name: "Camera Raw development, profiles, optics, presets, and output"
    app_behavior: "Camera Raw provides raw processing and photo development with profiles, edit panels, tone/color detail, curves, mixer, grading, optics, geometry, effects, calibration, presets, snapshots, workflow options, and batch output."
    tool_and_feature_scope:
      - "Basic, Curve, Detail, Color Mixer, Color Grading, Optics, Geometry, Effects, Calibration, Presets, Snapshots, Crop/Rotate, Healing, Red Eye, and workflow output settings."
      - "Raw defaults, camera/lens profiles, DNG handling, HDR, enhance/super resolution, noise reduction, preview and before/after comparison."
    studio_primitive_domains: [camera_raw, color, raster, file_io, export]
    source_surfaces: [help_leaf, shortcut_row, file_format_matrix, release_delta]
    manual_topic_candidate: "studio.manual.camera-raw.development"
    implementation_notes:
      - "Raw edits should be parameter stacks over immutable source data."
      - "Output settings need deterministic conversion receipts and profile-aware export tests."
  - id: psd.domain.camera_raw_masking_and_scopes
    name: "Camera Raw masking, selection, healing, and local adjustments"
    app_behavior: "Camera Raw includes local edit masks, brush/linear/radial gradients, subject/sky/background/object/people masking, luminance/color/depth range masks, healing, red eye, snapshots, copy/paste settings, and batch synchronization."
    tool_and_feature_scope:
      - "Mask creation, add/subtract/intersect masks, AI masks, range masks, mask overlays, local adjustment sliders, healing samples, and synchronization."
      - "Batch edits, presets with masks where supported, and workflow-safe propagation across images."
    studio_primitive_domains: [camera_raw, selection, mask, ai, batch]
    source_surfaces: [help_leaf, shortcut_row, provider_or_cloud]
    manual_topic_candidate: "studio.manual.camera-raw.local-adjustments"
    implementation_notes:
      - "Mask trees should reuse the Studio selection/mask engine and serialize as editable raw-development state."
      - "AI masks need provider posture and local fallback tracking."
  - id: psd.domain.automation_extensibility
    name: "Actions, scripts, droplets, batch, UXP, plugins, and automation"
    app_behavior: "Photoshop exposes user and developer automation through Actions, Batch, Image Processor, droplets, scripts, Generator/legacy extension concepts, UXP plugins, batchPlay, descriptors, document/action DOM objects, menu commands, and scripting events."
    tool_and_feature_scope:
      - "Record/play actions, action sets, modal controls, batch process, droplets, conditional actions, scripts, script events, image processor, UXP panels/commands, and batchPlay."
      - "Document/layer/channel/path/action descriptors, menu command IDs, plugin UI, permissions, file access, and automation error handling."
    studio_primitive_domains: [automation, scripting, plugin_api, command_contracts, batch]
    source_surfaces: [help_leaf, scripting_api, uxp_api, shortcut_row]
    manual_topic_candidate: "studio.manual.automation.photoshop-class"
    implementation_notes:
      - "Every Studio command should be scriptable and receipt-producing to avoid separate GUI-only behavior."
      - "Photoshop action compatibility can be treated as a translator layer where legally and technically practical."
  - id: psd.domain.export_metadata_collaboration
    name: "Export, review, metadata, libraries, sharing, and collaboration"
    app_behavior: "Photoshop supports export/share workflows, metadata and copyright fields, Creative Cloud/library-linked assets, comments/review, cloud documents, version/history surfaces, and collaboration-adjacent provider behavior."
    tool_and_feature_scope:
      - "Export As, Quick Export, slices/assets, metadata, comments, review links, cloud documents, libraries, linked assets, share, and version/recovery workflows."
      - "Local-first equivalents for review packages, asset libraries, linked resource resolution, and offline recovery."
    studio_primitive_domains: [export, metadata, collaboration, asset_pipeline, provider_adapter]
    source_surfaces: [help_leaf, provider_or_cloud, file_format_matrix, release_delta]
    manual_topic_candidate: "studio.manual.export.photoshop-class-review"
    implementation_notes:
      - "Cloud collaboration should become optional provider adapters plus local project packages and receipts."
      - "Library assets should resolve through local asset registries first, with provider links as optional references."
```

### [SFR-PHOTOSHOP-SOURCE-DISTILLED-DOMAINS.sources] Sources

```yaml
sources:
  - { id: PSD-S01, url: "https://helpx.adobe.com/photoshop/desktop.html", note: "Official Photoshop desktop help source." }
  - { id: PSD-S02, path: "_source_snapshots/adobe-photoshop-desktop-jina.md", note: "Local reader snapshot of PSD-S01 used for help-leaf extraction." }
  - { id: PSD-S03, url: "https://helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html", note: "Photoshop keyboard shortcut source surface." }
  - { id: PSD-S04, path: "_source_snapshots/photoshop-keyboard-shortcuts-jina.md", note: "Local shortcut snapshot." }
  - { id: PSD-S05, url: "https://helpx.adobe.com/photoshop/using/tools.html", note: "Photoshop toolbar and tool-family source surface." }
  - { id: PSD-S06, path: "_source_snapshots/photoshop-customize-toolbar-jina.md", note: "Local toolbar customization snapshot." }
  - { id: PSD-S07, path: "_source_snapshots/photoshop-workspace-overview-jina.md", note: "Local workspace overview snapshot." }
  - { id: PSD-S08, url: "https://developer.adobe.com/photoshop/uxp/", note: "Photoshop UXP developer source." }
  - { id: PSD-S09, path: "_source_snapshots/photoshop-uxp-api-jina.md", note: "Local UXP snapshot." }
  - { id: PSD-S10, path: "_source_snapshots/photoshop-scripting-jina.md", note: "Local Photoshop scripting snapshot." }
  - { id: PSD-S11, path: "06-photoshop-leaf-index.md", note: "Generated Photoshop help leaf index." }
  - { id: PSD-S12, path: "15-photoshop-feature-use-cards.md", note: "Generated Photoshop Feature Use Cards." }
  - { id: PSD-S13, path: "29-photoshop-expanded-count-ledger.md", note: "Expanded Photoshop online-source count ledger." }
```
