---
file_id: studio-app-feature-research-affinity-deep-feature-delta
file_kind: deep_feature_delta
topic_id: SFR-AFFINITY-DEEP-DELTA
title: "Affinity Suite Deep Feature Delta"
status: draft
app_key: affinity
updated_at: "2026-07-09"
summary: "Below-TOC-leaf per-persona tool/panel/option inventory for Affinity Photo 2, Designer 2 and Publisher 2 desktop, built from official desktop help pages, local desktop TOC snapshots and 2.x release evidence."
counts:
  total_records: 464
  personas: 14
  photo_tools: 72
  designer_tools: 56
  publisher_tools: 5
  layers_and_adjustments: 108
  selections_and_masks: 12
  color_and_formats: 14
  typography: 20
  publisher_layout: 26
  export_and_formats: 43
  automation_and_integration: 15
  panels_and_workspace: 42
  version_2x_deltas: 13
  post_2x_unified_relaunch: 24
  verified: 464
  unverified: 0
  deepens_existing: 433
  new_surface: 31
---

## [SFR-AFFINITY-DEEP-DELTA] Affinity Suite Deep Feature Delta

Vendor names are research provenance only per the folder naming policy; no Studio surface may reuse them. Rows deepen the help-article leaf corpus in `04-affinity-leaf-index.md` and `09-affinity-desktop-delta.md` down to per-persona tool, sub-tool, per-adjustment, per-filter, per-blend-mode, per-format and per-option granularity. `deepens_existing` rows cite the covering leaf id; `new_surface` rows have no covering help leaf (release-notes-only or posture rows).

### [SFR-AFFINITY-DEEP-DELTA.personas] Personas And StudioLink

```yaml
records:
  - id: "affinity.deep.personas.photo-photo-persona"
    name: "Photo Persona (Affinity Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Default editing persona hosting crop, selection, brush, retouch, erase, warp and vector tools for raster compositing."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.introduction-personas"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S18]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.photo-liquify-persona"
    name: "Liquify Persona (Affinity Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Dedicated mesh-distortion environment for retouch and special warp effects with its own tool and panel set."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.introduction-personas"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S18]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.photo-develop-persona"
    name: "Develop Persona (Affinity Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "In-app raw development environment with full control of image color and tone before committing to the layer stack."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.introduction-personas"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S18]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.photo-tone-mapping-persona"
    name: "Tone Mapping Persona (Affinity Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Dedicated tone-mapping environment intended for 32-bit documents but also enterable from 8/16-bit documents to tone map non-HDR images."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S18, AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.photo-export-persona"
    name: "Export Persona (Affinity Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Slice-based output environment exporting the image, layers or slices to a range of image formats."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.export-persona-exporting-using-export-persona"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S18]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.photo-panorama-persona"
    name: "Panorama stitching mode (Affinity Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Dedicated panorama workspace stitches multiple source images and provides post-stitch editing of seams and transform."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.panoramas-stitching-panoramas"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panorama/panorama_stitching.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.photo-astrophotography-stack-persona"
    name: "Astrophotography Stack Persona (Affinity Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Desktop-only persona for calibrating and stacking astrophotography frames (lights/darks/flats/bias) with its own Files, RAW Options and Stacking Options panels."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.astrophotography-astro_about"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Astrophotography/astro_about.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.designer-designer-persona"
    name: "Designer Persona (Affinity Designer)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Primary vector persona offering vector drawing tools including Contour, Corner, Node, Vector Brush and Pencil tools."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.introduction-personas"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S23]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.designer-pixel-persona"
    name: "Pixel Persona (Affinity Designer)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Raster persona inside the vector app for pixel painting, erasing, pixel selections and retouch; gained perspective and mesh warp live filters in 2.1."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.introduction-personas"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S23, AFD-S26]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.designer-export-persona"
    name: "Export Persona (Affinity Designer)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Slice-based export environment with Slices, Export Options and Export Layers panels."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.export-persona-exporting-using-export-persona"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ExportPersona/exportPersona.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.publisher-publisher-persona"
    name: "Publisher Persona (Affinity Publisher)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Primary DTP persona accessing page layout, text/picture frames, tables, shapes and publishing functions."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.personas-personas"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S23]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.publisher-designer-persona-studiolink"
    name: "Designer Persona inside Publisher (StudioLink)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Embeds the full Designer vector persona in the layout document when Affinity Designer 2 is installed; without it the persona stays unavailable."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.personas-studiolink-to-designer-persona"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Introduction/DesignerPersona.html"
    source_ids: [AFD-S23]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.publisher-photo-persona-studiolink"
    name: "Photo Persona inside Publisher (StudioLink)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Embeds the Photo raster persona (selections, pixel brushes, erasing, retouch) in the layout document when Affinity Photo 2 is installed."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.personas-studiolink-to-photo-persona"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Introduction/PhotoPersona.html"
    source_ids: [AFD-S23]
    verification_status: VERIFIED
  - id: "affinity.deep.personas.studiolink-cross-app-persona-switching"
    name: "StudioLink cross-app persona switching"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Publisher hosts sibling-app personas in-place so one document is edited with three toolsets without app round-trips; requires the sibling apps to be installed and licensed."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.personas-personas"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Introduction/about_Personas.html"
    source_ids: [AFD-S23]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.photo-tools] Photo Persona Toolset, Develop, Liquify, Tone Mapping, Astro

```yaml
records:
  - id: "affinity.deep.photo-tools.photo-view-tool"
    name: "View Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Pans the visible portion of the document in the document view."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-photo-editing-tools-view-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_pan.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-move-tool"
    name: "Move Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Selects, moves, scales, rotates and shears layer content and objects with on-canvas handles."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-photo-editing-tools-move-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_move.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-color-picker-tool"
    name: "Color Picker Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Samples colors from the canvas into the active color; since 2.6 the Color panel picker also applies the sampled color to selected objects."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-photo-editing-tools-color-picker-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_clrpicker.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-style-picker-tool"
    name: "Style Picker Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Samples object attributes/styles from one object and applies them to another."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-photo-editing-tools-style-picker-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_stylePicker.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-measure-tool"
    name: "Measure Tool (Photo)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Measures on-canvas distances and angles in document units; desktop-only tool."
    primitive_domain: diagnostics
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.tools-tools_measure"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_measure.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-crop-tool"
    name: "Crop Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Non-destructively crops and straightens with unconstrained/ratio/absolute modes; 2.1 added crop-to-selection and a Phi (golden ratio) overlay grid."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-photo-editing-tools-crop-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_crop.html"
    source_ids: [AFD-S01, AFD-S26]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-zoom-tool"
    name: "Zoom Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Changes the zoom level of the page/canvas in the document view."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.tools-tools_zoom"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_zoom.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-object-selection-tool-ml"
    name: "Object Selection Tool (ML)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Uses an on-device machine-learning model to select objects on pixel/image/raw layers, with multi-part component selection and optional matting; added in 2.6, runs fully locally."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-selection-tools-object-selection-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_objectSelection.html"
    source_ids: [AFD-S01, AFD-S19, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-selection-brush-tool"
    name: "Selection Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Paints pixel selections that snap to edges, growing or shrinking the selection by stroke."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-selection-tools-smart-selection-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_selectionBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-flood-select-tool"
    name: "Flood Select Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Selects pixels of similar color/tone by tolerance from a clicked sample; 2.6 added modifier-key add/subtract and default antialiasing."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-selection-tools-flood-select-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_floodSelect.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-marquee-selection-tools"
    name: "Marquee Selection Tools (rectangular/elliptical/column/row/freehand)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Geometric and freehand marquee family for pixel selections; 2.6 added draw-from-center, proportional constrain modifiers and a keyed intersection toggle."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-selection-tools-marquee-selection-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_marquee.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-flood-fill-tool"
    name: "Flood Fill Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Fills contiguous pixel regions of similar color with the active color by tolerance."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-fill-tools-flood-fill-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_floodFill.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-gradient-tool"
    name: "Gradient Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies and edits gradient fills on layers, fill layers and masks with on-canvas stop handles."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-fill-tools-gradient-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_gradient.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-paint-brush-tool"
    name: "Paint Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Paints soft/textured antialiased brush strokes with full brush dynamics on pixel layers."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-paint-tools-paint-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_paintBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-color-replacement-brush-tool"
    name: "Color Replacement Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Recolors brushed pixels with the active color while preserving underlying luminosity/texture."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-paint-tools-color-replacement-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_clrReplacementBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-pixel-tool"
    name: "Pixel Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Paints hard-edged, non-antialiased pixel-aligned strokes for pixel-art style editing."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-paint-tools-pixel-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_pixel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-paint-mixer-brush"
    name: "Paint Mixer Brush"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Wet-mixes existing canvas colors with loaded paint for natural-media blending strokes."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-paint-tools-paint-mixer-brush"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_paintMixerBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-erase-brush-tool"
    name: "Erase Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Erases pixels to transparency with brush dynamics; on image layers 2.6 routes erasing through masking."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-erase-tools-erase-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_eraseBrush.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-background-erase-brush-tool"
    name: "Background Erase Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Erases sampled background color under the brush while protecting dissimilar foreground pixels."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-erase-tools-background-erase-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_backgroundEraseBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-flood-erase-tool"
    name: "Flood Erase Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Erases contiguous similar-color pixel regions to transparency by tolerance; desktop-only tool."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.tools-tools_flooderase"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_floodErase.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-dodge-brush-tool"
    name: "Dodge Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Locally lightens pixels by brushing, with tonal range targeting."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-dodge-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_dodgeBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-burn-brush-tool"
    name: "Burn Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Locally darkens pixels by brushing, with tonal range targeting."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-burn-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_burnBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-sponge-brush-tool"
    name: "Sponge Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Locally saturates or desaturates brushed pixels."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-sponge-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_spongeBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-clone-brush-tool"
    name: "Clone Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Paints sampled pixels from a source point or global clone source onto the target, including cross-document sources via the Sources panel."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-clone-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_cloneBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-undo-brush-tool"
    name: "Undo Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Paints areas back to an earlier history/snapshot state (selective undo by brush)."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-undo-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_undoBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-blur-brush-tool"
    name: "Blur Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Locally softens detail by brushing a blur effect."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-blur-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_blurBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-sharpen-brush-tool"
    name: "Sharpen Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Locally increases contrast/detail by brushing a sharpen effect."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-sharpen-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_sharpenBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-median-brush-tool"
    name: "Median Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies a median (noise-reducing, edge-preserving) effect under the brush."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-median-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_medianBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-smudge-brush-tool"
    name: "Smudge Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Drags and smears pixels in the stroke direction."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-smudge-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_smudgeBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-healing-brush-tool"
    name: "Healing Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Clones from a sampled source while blending texture with target color/tone for seamless repairs."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-healing-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_healingBrush.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-patch-tool"
    name: "Patch Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Repairs a drawn region by blending pixels sampled from another region; 2.6 remembers the previously selected target layer."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-patch-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_patch.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-blemish-removal-tool"
    name: "Blemish Removal Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Removes small imperfections with single clicks using automatic sampling."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-blemish-removal-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_blemishRemoval.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-inpainting-brush-tool"
    name: "Inpainting Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Content-aware fills brushed regions from surrounding image data; 2.6 extends inpainting to image/raw layers and remembers the target layer."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-inpainting-brush-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_inpaintingBrush.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-red-eye-removal-tool"
    name: "Red Eye Removal Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Fixes red-eye by clicking or dragging over pupils while preserving eye detail."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-retouch-tools-red-eye-removal-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_redEye.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-mesh-warp-tool"
    name: "Mesh Warp Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Distorts layer content through an editable node/patch mesh with bezier control."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.tools-tools_meshwarp"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_meshWarp.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-perspective-tool"
    name: "Perspective Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies single- or dual-plane perspective correction/distortion to layer content."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.tools-tools_perspective"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_perspective.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-push-forward-tool"
    name: "Liquify Push Forward Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Shifts mesh pixels in the direction of the stroke."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-push-left-tool"
    name: "Liquify Push Left Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Shifts pixels 90 degrees left of the stroke direction, spreading and compressing edges along the stroke."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-twirl-tool"
    name: "Liquify Twirl Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies a clockwise rotational distortion centered under the tool cursor."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-pinch-tool"
    name: "Liquify Pinch Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies a concave spherical distortion under the stroke."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-punch-tool"
    name: "Liquify Punch Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies a convex spherical distortion under the stroke."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-turbulence-tool"
    name: "Liquify Turbulence Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies a crumbling distortion that compacts some mesh lines while expanding others."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-mesh-clone-tool"
    name: "Liquify Mesh Clone Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Paints mesh deformation sampled from one part of the mesh onto another."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-reconstruct-tool"
    name: "Liquify Reconstruct Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Reduces previously applied warp effect where brushed (partial mesh reset)."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-freeze-tool"
    name: "Liquify Freeze Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Masks areas to protect them from warp effects."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-liquify-thaw-tool"
    name: "Liquify Thaw Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Removes the freeze mask so areas can be warped again."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-liquify-tools-liquify-persona-liquify-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    source_ids: [AFD-S09]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-overlay-paint-tool"
    name: "Develop Overlay Paint Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Adds brushed areas to a selected develop overlay so raw adjustments apply regionally."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-raw-tools-develop-persona-raw-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_raw.html"
    source_ids: [AFD-S10]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-overlay-erase-tool"
    name: "Develop Overlay Erase Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Removes brushed areas from a selected develop overlay adjustment."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-raw-tools-develop-persona-raw-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_raw.html"
    source_ids: [AFD-S10]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-overlay-gradient-tool"
    name: "Develop Overlay Gradient Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies linear, elliptical or radial graduated opacity to the selected develop overlay adjustment."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-raw-tools-develop-persona-raw-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_raw.html"
    source_ids: [AFD-S10]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-white-balance-picker-tool"
    name: "Develop White Balance Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Sets white balance automatically from clicked or region-sampled pixels during raw development."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-raw-tools-develop-persona-raw-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_raw.html"
    source_ids: [AFD-S10]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-persona-shared-tools"
    name: "Develop Persona shared tool set (crop, red eye, blemish, view, zoom)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Develop Persona carries persona-scoped Crop, Red Eye Removal, Blemish Removal, View and Zoom tools so raw fixes happen before development."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-raw-tools-develop-persona-raw-tools"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_raw.html"
    source_ids: [AFD-S10]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-basic-exposure-group"
    name: "Develop Basic panel: Exposure group"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Adjusts overall exposure, black point and brightness of the raw image."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-basic-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelBasic.html"
    source_ids: [AFD-S11]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-basic-enhance-group"
    name: "Develop Basic panel: Enhance group"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Adjusts contrast, clarity, saturation and vibrance during raw development."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-basic-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelBasic.html"
    source_ids: [AFD-S11]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-basic-white-balance-group"
    name: "Develop Basic panel: White Balance group"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Removes color casts via temperature/tint control during raw development."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-basic-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelBasic.html"
    source_ids: [AFD-S11]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-basic-shadows-highlights-group"
    name: "Develop Basic panel: Shadows & Highlights group"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Applies tonal recovery to the darkest and lightest raw areas."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-basic-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelBasic.html"
    source_ids: [AFD-S11]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-output-profile"
    name: "Develop output profile selection"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Selects a system-installed ICC output profile for color-managing the developed image."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-basic-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelBasic.html"
    source_ids: [AFD-S11]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-adjustment-presets"
    name: "Develop adjustment presets (add/delete/default)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Per-panel develop presets can be added, deleted or reverted to defaults, with per-adjustment active toggle and reset."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-basic-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelBasic.html"
    source_ids: [AFD-S11]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-develop-tones-panel"
    name: "Develop Tones panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Develop-Persona-only panel grouping three tonal adjustments applied during raw development: Curves, Black & White and Split Toning."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-tones-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelTones.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.photo-tools.photo-develop-details-panel"
    name: "Develop Details panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Develop-Persona-only panel grouping Detail Refinement (edge sharpening), Noise Reduction and Noise Addition applied during raw development."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-details-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelDetails.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.photo-tools.photo-develop-lens-panel"
    name: "Develop Lens panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Develop-Persona-only panel grouping Lens Correction (distortion), Chromatic Aberration Reduction, Defringe, Remove Lens Vignette and Post Crop Vignette, with automatic lens-profile selection plus manual override."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.develop-persona-raw-lens-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelLens.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.photo-tools.photo-develop-focus-panel"
    name: "Develop Focus panel (desktop-only)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Desktop Develop Persona metadata panel reporting capture-time focus settings (Mode, Beam, Circle of Confusion, hyperfocal distance) with a Show AF Regions overlay of camera autofocus zones (Canon CR2); inspection-only, not an adjustment."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.raw-raw_panelfocus"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelFocus.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.photo-tools.photo-develop-snapshots-panel"
    name: "Develop Snapshots panel (desktop-only)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Stores and restores develop-settings snapshots within the Develop Persona for comparing processing variants; snapshots are temporary and deleted on leaving the persona (unlike persistent Photo Persona snapshots)."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.raw-raw_panelsnapshots"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelSnapshots.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.photo-tools.photo-develop-location-panel"
    name: "Develop Location panel (desktop, macOS-only)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Reviews and sets GPS location metadata of the raw image inside the Develop Persona via an interactive map (pin repositioning, address search, current location); help page states it is exclusive to the macOS version."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.raw-raw_panellocation"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelLocation.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.photo-tools.photo-tonemap-tone-compression"
    name: "Tone Mapping: Tone Compression control"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Controls how much of the unbounded HDR tonal range is mapped into displayable range."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-tonemap-local-contrast"
    name: "Tone Mapping: Local Contrast control"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Boosts or reduces clarity/local contrast in the tone-mapped result."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-tonemap-tonal-controls"
    name: "Tone Mapping: exposure/black point/brightness/contrast controls"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Standard tonal sliders adjust exposure, shadows, mid-tones and overall contrast in the tone map."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-tonemap-color-controls"
    name: "Tone Mapping: saturation/vibrance/white balance controls"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Saturation and non-clipping vibrance plus temperature/tint white balance refine tone-mapped color."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-tonemap-shadows-highlights-detail"
    name: "Tone Mapping: shadows/highlights compression and Detail Refinement"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Fine-tunes shadow/highlight compression and applies subtle sharpening to the tone-mapped output."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-tonemap-curves"
    name: "Tone Mapping: Curves control"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Adjusts the tone-mapped tonal range through a curves graph inside the persona."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-tonemap-clamp-to-sdr"
    name: "Tone Mapping: Clamp to SDR"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Ensures the final tone-mapped result stays within displayable SDR bounds."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-tonemap-presets-panel"
    name: "Tone Mapping Presets panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "One-click prebuilt tone-mapping presets with custom preset creation and import."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-tone-mapping-hdr-images"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    source_ids: [AFD-S25]
    verification_status: VERIFIED
  - id: "affinity.deep.photo-tools.photo-bad-pixel-map-tool"
    name: "Bad Pixel Map Tool (Astrophotography)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Marks defective sensor pixels to exclude them from astrophotography stacking."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.tools-tools_badpixelmap"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_badPixelMap.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.designer-tools] Designer Vector And Pixel Persona Toolset

```yaml
records:
  - id: "affinity.deep.designer-tools.suite-pen-tool"
    name: "Pen Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Draws bezier curves node-by-node in all three apps; anchor of the shared vector path model."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-pen-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_pen.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-pen-tool-modes"
    name: "Pen Tool drawing modes (Pen/Smart/Polygon/Line)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Pen Tool context bar switches between bezier Pen, auto-smoothing Smart, straight-segment Polygon and single-segment Line modes."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-pen-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_pen.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.designer-tools.suite-node-tool"
    name: "Node Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Selects and edits curve nodes, control handles and segments; supports multi-node selection and alignment."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-node-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_node.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-point-transform-tool"
    name: "Point Transform Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Transforms objects around a movable origin point directly on canvas, keyed to node positions."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-point-transform-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_pointTransform.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-contour-tool"
    name: "Contour Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Non-destructively offsets (insets/outsets) shape and curve outlines by a contour distance."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-contour-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_contour.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-corner-tool"
    name: "Corner Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Applies live, editable corner types (rounded and variants) to individual curve nodes."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-corner-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_corner.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-pencil-tool"
    name: "Pencil Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Freehand-draws vector curves with smoothing; 2.6 added auto-closing behavior choices, drawing with the current line style, and smoothness control when editing."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-pencil-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_pencil.html"
    source_ids: [AFD-S02, AFD-S21]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-stroke-width-tool"
    name: "Stroke Width Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Edits a curve's pressure/width profile on-document, clicking to add or remove width points; added in 2.5, 2.6 added Reset Profile."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-stroke-width-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_lineWidth.html"
    source_ids: [AFD-S02, AFD-S29, AFD-S21]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-knife-tool"
    name: "Knife Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Cuts curves and shapes apart along freehand or straight cut paths."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-knife-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_knife.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-vector-brush-tool"
    name: "Vector Brush Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Paints raster-textured brush strokes along editable vector spines (image brushes on curves)."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-vector-brush-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_Brush.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-vector-flood-fill-tool"
    name: "Vector Flood Fill Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Flood-fills enclosed vector regions with color by click, generating fill geometry; added in 2.1."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-vector-flood-fill-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_vectorFloodFill.html"
    source_ids: [AFD-S02, AFD-S26]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-transparency-tool"
    name: "Transparency Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies directional opacity gradients to objects on-canvas (Designer and Publisher)."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-transparency-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_transparency.html"
    source_ids: [AFD-S02, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-artboard-tool"
    name: "Artboard Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Creates, sizes and manages multiple artboards in one document with per-artboard export and print."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.tools-tools_artboard"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_artboard.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-place-tool"
    name: "Place Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Places external images/documents into the document as embedded or linked resources with drag-sizing."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.tools-tools_placeimage"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_placeimage.html"
    source_ids: [AFD-S02, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-vector-crop-tool"
    name: "Vector Crop Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Non-destructively crops vector/image objects to a rectangular region without discarding content."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-vector-crop-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_crop.html"
    source_ids: [AFD-S02, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-shape-builder-tool"
    name: "Shape Builder Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Merges and subtracts overlapping shape regions by drag/click gestures to compose new geometry."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-shape-builder-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_shapeBuilder.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-area-tool"
    name: "Area Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Measures enclosed area of shapes/regions in document units (pairs with Measure Tool and drawing scale)."
    primitive_domain: diagnostics
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-design-tools-area-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_area.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-boolean-add"
    name: "Boolean geometry: Add (union)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Unions selected shapes into one object; destructive when applied plainly, live when compounded."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-joining-objects"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/join.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-boolean-subtract"
    name: "Boolean geometry: Subtract"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Removes the top shape's area from those below."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-joining-objects"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/join.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-boolean-intersect"
    name: "Boolean geometry: Intersect"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Keeps only the overlapping area of the selected shapes."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-joining-objects"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/join.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-boolean-divide"
    name: "Boolean geometry: Divide"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Splits overlapping shapes into separate closed objects along intersection boundaries."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-joining-objects"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/join.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-boolean-combine-xor"
    name: "Boolean geometry: Combine (XOR)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Merges shapes while removing overlapping regions (exclusive-or fill result)."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-joining-objects"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/join.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-live-compounds"
    name: "Live compounds (non-destructive booleans)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Creates boolean results as live compound objects whose child shapes and per-child operators stay editable."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-compound-objects"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/compound.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-expand-stroke"
    name: "Expand Stroke"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Converts a stroked line, including pressure-profiled width, into a filled closed shape."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.lines-curves-and-shapes-expanding-strokes"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/CurvesShapes/expandStroke.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-warp-groups"
    name: "Vector warp groups (non-destructive distortion)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Wraps objects in a warp group applying live perspective/quad/mesh distortion while children remain editable."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-warping-objects"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/warp.html"
    source_ids: [AFD-S02, AFD-S27]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-fill-modes-winding-rule"
    name: "Fill modes (winding / even-odd)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Per-curve fill rule switches between non-zero winding and alternate (even-odd) hole filling."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.curvesshapes-fillmode"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/CurvesShapes/fillMode.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-multiple-strokes-and-fills"
    name: "Multiple strokes and fills per object"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "One object carries stacked stroke and fill attribute entries managed in the Appearance panel."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.lines-curves-and-shapes-using-multiple-strokes-and-fills"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/CurvesShapes/multiStrokesAndFills.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-select-by-attribute"
    name: "Select objects by attribute (Select Same/Select Object)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Selects all objects sharing attributes (e.g. fill/stroke/type) across the document; desktop-only surface, also in Publisher."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.objectcontrol-selectbyattribute"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/ObjectControl/selectByAttribute.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-symbols-sync"
    name: "Symbols with per-instance sync control"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Symbol instances share edits; sync can be toggled so selected changes stay detached per instance."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.object-control-symbols"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/SymbolsAssets/symbols.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.designer-tools.suite-constraints"
    name: "Object constraints (responsive anchoring/scaling)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Constraints panel pins object edges and scaling behavior relative to parent/container resize (Designer and Publisher)."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.design-aids-constraints"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/DesignAids/constraints.html"
    source_ids: [AFD-S02, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-pixel-persona-symmetry-painting"
    name: "Symmetry and mirror painting"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Pixel painting supports up to multi-axis symmetry and mirrored strokes (Photo and Designer Pixel Persona); desktop leaf."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.painting-symmetrybrushes"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Painting/symmetryBrushes.html"
    source_ids: [AFD-S01, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.designer-pixel-persona-retouch-set"
    name: "Pixel Persona retouch subset (dodge/burn/smudge/blur/sharpen)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Designer's Pixel Persona ships a reduced retouch brush set compared to Photo (dodge, burn, smudge, blur, sharpen only)."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.painting-retouch"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Painting/retouch.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-rectangle"
    name: "Rectangle Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric rectangle with per-corner live radius and corner-type parameters; available in all three apps."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-rectangle-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_rectangle.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-ellipse"
    name: "Ellipse Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric ellipse/circle shape; constrainable to circle."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-ellipse-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_ellipse.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-rounded-rectangle"
    name: "Rounded Rectangle Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Rectangle variant with adjustable uniform/per-corner rounding parameter."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-rounded-rectangle-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_roundedRectangle.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-triangle"
    name: "Triangle Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric triangle with adjustable apex position."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-triangle-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_triangle.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-diamond"
    name: "Diamond Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric diamond with adjustable midpoint parameter."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-diamond-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_diamond.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-trapezoid"
    name: "Trapezoid Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric trapezoid with adjustable top-edge offsets."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-trapezoid-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_trapezoid.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-polygon"
    name: "Polygon Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric polygon with side count and curvature parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-polygon-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_polygon.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-star"
    name: "Star Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric star with point count and inner/outer radius parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-star-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_star.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-double-star"
    name: "Double Star Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric double star with two independent point sets."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-double-star-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_doublestar.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-square-star"
    name: "Square Star Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric square-pointed star (burst) shape."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-square-star-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_squareStar.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-arrow"
    name: "Arrow Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric arrow with head/tail style and shaft thickness parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-arrow-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_arrow.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-donut"
    name: "Donut Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric ring with hole radius and sweep angle parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-donut-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_dnut.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-pie"
    name: "Pie Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric pie/sector with sweep angle parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-pie-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_pie.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-segment"
    name: "Segment Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric circular segment shape."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-segment-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_segment.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-crescent"
    name: "Crescent Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric crescent/moon shape with curvature parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-crescent-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_crescent.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-cog"
    name: "Cog Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric cog/gear with teeth count, tooth size and hole parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-cog-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_cog.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-cloud"
    name: "Cloud Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric cloud with bump count/curvature parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-cloud-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_cloud.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-callout-rounded-rectangle"
    name: "Callout Rounded Rectangle Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Speech-bubble rectangle with tail position/size parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-callout-rectangle-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_calloutRoundedRectangle.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-callout-ellipse"
    name: "Callout Ellipse Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Speech-bubble ellipse with tail position/size parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-callout-ellipse-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_calloutEllipse.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-tear"
    name: "Tear Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric teardrop with curvature/tail parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-tear-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_tear.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-heart"
    name: "Heart Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric heart with lobe/spread parameters."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-heart-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_heart.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-spiral"
    name: "Spiral Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric spiral supporting linear, decaying, semi-circular, Fibonacci and plotted spiral types; added in 2.3."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-spiral-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_spiral.html"
    source_ids: [AFD-S02, AFD-S30]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-qr-code"
    name: "QR Code Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Generates scannable QR codes as document objects (URL, phone, vCard, FaceTime, WhatsApp actions); added in 2.5 across all three apps."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.tools-shape-tools-qr-code-tool"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_qrCode.html"
    source_ids: [AFD-S02, AFD-S29]
    verification_status: VERIFIED
  - id: "affinity.deep.designer-tools.suite-shape-cat"
    name: "Cat Tool (parametric)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Parametric cat-silhouette novelty shape; desktop-only leaf in all three apps."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.tools-tools_cat"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Tools/tools_cat.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.publisher-tools] Publisher-Specific Toolset

```yaml
records:
  - id: "affinity.deep.publisher-tools.publisher-picture-frame-rectangle-tool"
    name: "Picture Frame Rectangle Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Draws rectangular picture frames that clip and scale placed content per frame fit properties."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.tools-layout-tools-picture-frame-rectangle-tool"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Tools/tools_pictureFrameRectangle.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-tools.publisher-picture-frame-ellipse-tool"
    name: "Picture Frame Ellipse Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Draws elliptical picture frames that clip and scale placed content."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.tools-layout-tools-picture-frame-ellipse-tool"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Tools/tools_pictureFrameEllipse.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-tools.publisher-picture-frame-fit-properties"
    name: "Picture frame content fit/anchor properties"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Frames carry content scaling rules controlling how placed content sizes into the frame: Scale to Max Fit (default, may crop), Scale to Min Fit, Stretch to Fit and None; an anchor point can also be set during precise data-entry frame creation."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.placing-external-content-picture-frames"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Media/pictureFrames.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.publisher-tools.publisher-data-merge-layout-tool"
    name: "Data Merge Layout Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Creates repeating grid layouts (rows/columns/cell size) whose first-cell design replicates to all cells for record-per-cell merges like cards and badges."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.tools-tools_datamergenode"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Tools/tools_dataMergeNode.html"
    source_ids: [AFD-S24]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-tools.publisher-table-tool"
    name: "Table Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Draws and edits tables with row/column manipulation, cell formatting, sorting and reusable table formats."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.tools-text-tools-table-tool"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Tools/tools_tableText.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.layers-and-adjustments] Layer Types, Adjustments, Live Filters, Blend Modes

```yaml
records:
  - id: "affinity.deep.layers-and-adjustments.suite-pixel-layer"
    name: "Pixel layer"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Editable raster layer holding pixel data at document bit depth."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-about-layers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/aboutLayers.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-image-layer"
    name: "Image layer (non-destructive placed container)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Container layer keeping placed images unrasterized at native resolution; 2.6 allowed merge, inpaint, brush, selection-delete-as-mask and duplicate-as-masked operations directly on image layers."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-image-layers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerImage.html"
    source_ids: [AFD-S01, AFD-S20, AFD-S21]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-vector-curve-layer"
    name: "Vector object/curve layer"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Curves, shapes and text live as vector layers in the same stack as raster content in all three apps."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-about-layers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/aboutLayers.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-group-layer"
    name: "Group layer"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Groups nest layers with shared opacity/blend/effects and act as clipping and masking scopes."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layer-operations-grouping"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/LayerOperations/group.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-layer-type"
    name: "Adjustment layer (non-destructive)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Adjustment layers apply color/tonal edits non-destructively to everything below or to clipped parents, with built-in mask."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-adjustment-layers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/adjustmentLayers.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-layer-type"
    name: "Live filter layer (non-destructive)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Filter effects hosted as maskable, re-editable layers instead of destructive pixel filters."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-fill-layer"
    name: "Fill layer"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Whole-canvas solid or gradient fill as a re-editable layer."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-fill-layers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerFill.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-pattern-layer"
    name: "Pattern layer (live repeating tile)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Layer type whose pixel content repeats as a tiled pattern; painting on any tile updates the repeat live."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-pattern-layers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerPattern.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-mask-layer"
    name: "Mask layer (grayscale alpha)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Grayscale masks hide/reveal parent content and can be painted, filled or generated from selections."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-masks"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/LayerMasks.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-compound-mask"
    name: "Compound masks (add/intersect/subtract/xor)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Multiple mask layers combine non-destructively through boolean operators into one compound mask."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-compound-layer-masks"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/compoundMasks.html"
    source_ids: [AFD-S01, AFD-S27]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-layer-masks"
    name: "Live layer masks (parametric masks)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Non-destructive parametric masks generated live from image properties, in three types (Live Hue Range, Live Luminosity Range, Live Band-pass); they update automatically with the underlying image and stay re-editable/reconfigurable at any time."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-layer-masks"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/LiveMasks/liveLayerMasks.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.layers-and-adjustments.suite-layer-tagging"
    name: "Layer tagging"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Layers accept tags (including accessibility-related tags in Publisher) for organization and export semantics."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layer-operations-tagging-layers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/LayerOperations/tagLayers.html"
    source_ids: [AFD-S01, AFD-S30]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-layer-colors"
    name: "Layer color labels"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Color labels mark layers in the Layers panel for organization (Designer/Publisher desktop leaf)."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.layers-layercolours"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Layers/layerColours.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-layer-states"
    name: "Layer states (saved visibility sets and queries)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "States panel saves and recalls layer visibility configurations, including query-based states, for document variations; added in 2.4."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.layeroperations-layerstates"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/LayerOperations/layerStates.html"
    source_ids: [AFD-S01, AFD-S28]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-layer-find"
    name: "Find layers/objects"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Search-based layer/object finding across the document (desktop-only leaf in all three apps)."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.layeroperations-finding"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/LayerOperations/finding.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-brightness-contrast"
    name: "Brightness / Contrast adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Adjusts pixel lightness and the tonal difference between pixels via two sliders."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_brightnesscontrast"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_brightnessContrast.html"
    source_ids: [AFD-S04, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-curves"
    name: "Curves adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Graph-based precise lightness/contrast adjustment with per-channel curves across color models."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_curves"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_curves.html"
    source_ids: [AFD-S04, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-exposure"
    name: "Exposure adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Retrieves lost highlight/shadow detail caused by poor exposure."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_exposure"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_exposure.html"
    source_ids: [AFD-S04, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-levels"
    name: "Levels adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Reassigns black, white and gamma points to redistribute pixel lightness, per channel."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_levels"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_levels.html"
    source_ids: [AFD-S04, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-shadows-highlights"
    name: "Shadows / Highlights adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies tonal adjustment to the darkest and/or lightest areas only."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_shadowshighlights"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_shadowsHighlights.html"
    source_ids: [AFD-S04, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-black-and-white"
    name: "Black and White adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Converts color to grayscale with per-hue contribution sliders."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_blackandwhite"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_blackAndWhite.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-channel-mixer"
    name: "Channel Mixer adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Recomposes output channels from weighted input channel contributions."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_channelmixer"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_channelMixer.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-color-balance"
    name: "Color Balance adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Adjusts color contributions per tonal range (shadows/midtones/highlights)."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_clrbalance"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_clrBalance.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-gradient-map"
    name: "Gradient Map adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Recolors the image by mapping pixel lightness onto a specified gradient."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_gradientmap"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_gradientMap.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-hsl"
    name: "HSL adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Shifts hue, saturation and luminosity globally or per hue range."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_hsl"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_HSL.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-invert"
    name: "Invert adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Reverses color values to a negative image."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_invert"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_invert.html"
    source_ids: [AFD-S06, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-lens-filter"
    name: "Lens Filter adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Emulates a physical lens filter tint with preserve-luminosity control."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_lensfilter"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_lensfilter.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-lut"
    name: "LUT adjustment (load/apply 3D LUTs)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies color grading from loaded lookup tables (matrix-based color remap)."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_3dlut"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_3dLut.html"
    source_ids: [AFD-S06, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-ocio"
    name: "OCIO adjustment (color space transform)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies OpenColorIO source-to-destination color space transforms as an adjustment layer."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_ocio"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_ocio.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-posterize"
    name: "Posterize adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Reduces the image to blocks of solid color by level count."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_posterize"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_posterize.html"
    source_ids: [AFD-S06, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-recolor"
    name: "Recolor adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Converts the image to monochrome tinted by a specified hue/saturation/lightness."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_reclr"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_reclr.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-selective-color"
    name: "Selective Color adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Subtly shifts color by modifying CMYK contributions per RGB/CMYK/lightness channel band."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_selectiveclr"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_selectiveClr.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-soft-proof"
    name: "Soft Proof adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Previews output for a target color space/device as a toggleable adjustment layer, enabling in-stack proofing."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_softproof"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_softProof.html"
    source_ids: [AFD-S06, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-split-toning"
    name: "Split Toning adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Tints highlights and shadows independently with balance control."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_splittoning"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_splitToning.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-threshold"
    name: "Threshold adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Converts to two-tone black/white by lightness threshold."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_threshold"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_threshold.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-vibrance"
    name: "Vibrance adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Adjusts color intensity weighted toward less-saturated pixels to avoid clipping."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_vibrance"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_vibrance.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-adjustment-white-balance"
    name: "White Balance adjustment"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Removes color casts by adjusting light temperature and tint."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.adjustments-adjustment_whitebalance"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Adjustments/adjustment_whiteBalance.html"
    source_ids: [AFD-S05, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-adjustment-normals"
    name: "Normals adjustment (Photo-only)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Adjusts and corrects normal maps used in 3D game-art pipelines."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.adjustments-other-adjustments"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Adjustments/otherAdjustments.html"
    source_ids: [AFD-S06]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-gaussian-blur"
    name: "Live filter: Gaussian Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Non-destructive gaussian softening as a maskable live filter layer."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-box-blur"
    name: "Live filter: Box Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Non-destructive box-average blur live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-median-blur"
    name: "Live filter: Median Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Non-destructive median blur live filter for edge-preserving noise smoothing."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-bilateral-blur"
    name: "Live filter: Bilateral Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Non-destructive edge-aware bilateral blur live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-motion-blur"
    name: "Live filter: Motion Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Directional blur imitating scene motion, as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-radial-blur"
    name: "Live filter: Radial Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Rotational blur around a settable center, as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-lens-blur"
    name: "Live filter: Lens Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Aperture-shaped bokeh blur simulation as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-depth-of-field-blur"
    name: "Live filter: Depth of Field Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Simulated shallow depth of field with in-focus region controls, as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-field-blur"
    name: "Live filter: Field Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Multi-point graduated blur field, as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-diffuse-glow"
    name: "Live filter: Diffuse Glow"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Glow/bloom softening of highlights as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-maximum-blur"
    name: "Live filter: Maximum Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Morphological dilate (maximum) operation as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-minimum-blur"
    name: "Live filter: Minimum Blur"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Morphological erode (minimum) operation as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-clarity"
    name: "Live filter: Clarity"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Local-contrast (midtone) enhancement as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-unsharp-mask"
    name: "Live filter: Unsharp Mask"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Radius/factor/threshold sharpening as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-high-pass"
    name: "Live filter: High Pass"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "High-pass detail extraction (for overlay sharpening workflows) as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-ripple"
    name: "Live filter: Ripple"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Wave/ripple distortion as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-twirl"
    name: "Live filter: Twirl"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Rotational twirl distortion as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-spherical"
    name: "Live filter: Spherical"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Spherical bulge/pinch distortion as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-displace"
    name: "Live filter: Displace"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Displacement-map-driven distortion as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-pinch-punch"
    name: "Live filter: Pinch/Punch"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Concave/convex distortion as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-lens-distortion"
    name: "Live filter: Lens Distortion"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Barrel/pincushion lens distortion correction/creation as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-perspective"
    name: "Live filter: Perspective"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Live perspective plane distortion; 2.6 added a keyboard shortcut for quick application."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-liquify"
    name: "Live filter: Liquify"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Liquify mesh warping hosted as a re-editable live filter layer."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-mesh-warp"
    name: "Live filter: Mesh Warp"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Mesh warp distortion hosted as a re-editable live filter layer."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-denoise"
    name: "Live filter: Denoise"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Luminance/color noise reduction as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-add-noise"
    name: "Live filter: Add Noise"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Adds gaussian/uniform noise/grain as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-diffuse"
    name: "Live filter: Diffuse"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Pixel-scatter diffusion as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-dust-and-scratches"
    name: "Live filter: Dust & Scratches"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Removes small artifacts via radius/tolerance smoothing as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-vignette"
    name: "Live filter: Vignette"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Exposure/hardness/scale-controlled edge vignette as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-defringe"
    name: "Live filter: Defringe"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Removes chromatic fringing along high-contrast edges as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-voronoi"
    name: "Live filter: Voronoi"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Voronoi cell mosaic effect as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-halftone"
    name: "Live filter: Halftone"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Dot/line/circular halftone screening as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-procedural-texture"
    name: "Live filter: Procedural Texture (formula-driven)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "User-authored mathematical formulas generate/transform pixels as a live filter (scriptable pixel math surface)."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-lighting"
    name: "Live filter: Lighting (3D-style lights)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Point/spot/directional scene lighting with ambience and surface properties as a live filter."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.photo-live-filter-shadows-highlights"
    name: "Live filter: Shadows/Highlights"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Tonal-range recovery hosted as a live filter layer."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-live-filters"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    source_ids: [AFD-S07]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-normal"
    name: "Blend mode: Normal"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Fully opaque pixels appear above underlying pixels."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-darken"
    name: "Blend mode: Darken"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Compares layers and retains the darker pixel values."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-multiply"
    name: "Blend mode: Multiply"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Multiplies active-layer pixels with the layers below."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-color-burn"
    name: "Blend mode: Color Burn"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Intensifies contrast and saturates mid-tones to exaggerate color."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-linear-burn"
    name: "Blend mode: Linear Burn"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Darker than Multiply; suited to deep shadow rendering."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-darker-color"
    name: "Blend mode: Darker Color"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Retains only the darker composite color values between layers."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-lighten"
    name: "Blend mode: Lighten"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Retains only the brighter values when comparing layers."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-screen"
    name: "Blend mode: Screen"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Inverts underlying colors and multiplies with the active layer for a lightening effect."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-color-dodge"
    name: "Blend mode: Color Dodge"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Increases luminosity while reducing contrast between layers."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-add"
    name: "Blend mode: Add"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Adds color values of active and underlying layers (linear dodge)."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-lighter-color"
    name: "Blend mode: Lighter Color"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Uses perceived luminosity to keep the lighter composite pixels."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-overlay"
    name: "Blend mode: Overlay"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Darkens dark pixels and lightens light pixels for enhanced contrast."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-soft-light"
    name: "Blend mode: Soft Light"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Diffused contrast effect from comparing luminance and color values."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-hard-light"
    name: "Blend mode: Hard Light"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Dramatic high-contrast complement to Overlay."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-vivid-light"
    name: "Blend mode: Vivid Light"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies Color Burn or Color Dodge depending on layer brightness."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-linear-light"
    name: "Blend mode: Linear Light"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Brighter pixels lighten, darker pixels darken, with strong contrast."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-pin-light"
    name: "Blend mode: Pin Light"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Mixes Darken and Lighten behavior creating distinct boundaries."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-hard-mix"
    name: "Blend mode: Hard Mix"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Highly contrasting posterized result (channel values forced to 0 or 1)."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-difference"
    name: "Blend mode: Difference"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Outputs the absolute difference between the active and underlying layers."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-exclusion"
    name: "Blend mode: Exclusion"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Softer variant of Difference."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-subtract"
    name: "Blend mode: Subtract"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Subtracts active-layer values from those below."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-divide"
    name: "Blend mode: Divide"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Divides underlying pixel values by the active layer's values."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-hue"
    name: "Blend mode: Hue"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Replaces the underlying hue with the active layer's hue."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-saturation"
    name: "Blend mode: Saturation"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Replaces the saturation component while preserving other qualities."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-color"
    name: "Blend mode: Color"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Combines active-layer hue and saturation with underlying brightness."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-luminosity"
    name: "Blend mode: Luminosity"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Combines active-layer brightness with underlying hue and saturation."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-average"
    name: "Blend mode: Average"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Outputs the mean of color and luminance values of the blended layers."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-negation"
    name: "Blend mode: Negation"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Difference-like blend producing more contrasting results."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-reflect"
    name: "Blend mode: Reflect"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Combines Hard Light and Hard Mix behavior preserving darker tones."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-glow"
    name: "Blend mode: Glow"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Opposite of Reflect; preserves lighter tones while enhancing darker ones."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-contrast-negate"
    name: "Blend mode: Contrast Negate"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Inverts pixel values based on underlying layer content."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-mode-erase"
    name: "Blend mode: Erase"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Excludes (punches out) underlying pixels where the active layer has content, working with layer opacity for faded/transparent output."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blending"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    source_ids: [AFD-S08]
    verification_status: VERIFIED
  - id: "affinity.deep.layers-and-adjustments.suite-blend-ranges"
    name: "Blend ranges (per-layer source/underlying tonal curves)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Blend Options constrain a layer's compositing to tonal ranges via editable Source Layer Ranges and Underlying Composition Ranges graphs with draggable/addable nodes, per-channel selection and linear or curved interpolation."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blend-ranges"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendRanges.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.layers-and-adjustments.suite-blend-gamma"
    name: "Blend gamma control"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Blend Options expose a per-layer blend gamma (RGB documents only): 1.0 linear-RGB, 2.2 regular sRGB blending, or any gamma up to 3.0; text layers default to 1.45, other layers to 2.2."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blend-ranges"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendRanges.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.layers-and-adjustments.suite-antialiasing-coverage-map"
    name: "Per-layer antialiasing/coverage map control"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Blend Options allow per-layer antialiasing override (Inherit/Force On/Force Off) plus an interactive Coverage Map chart adjusting the layer's antialiasing ramp for edge rendering control."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layers-layer-blend-ranges"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendRanges.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-AFFINITY-DEEP-DELTA.selections-and-masks] Selections, Refinement, Channels

```yaml
records:
  - id: "affinity.deep.selections-and-masks.photo-selection-combine-modes"
    name: "Selection combine modes (new/add/subtract/intersect)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Selection tools share context-bar combine modes; 2.6 standardized modifier-key add/subtract and a keyed intersection toggle."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-creating-pixel-selections-overview"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_create.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.selections-and-masks.photo-selection-grow-shrink"
    name: "Grow/Shrink selection"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Expands or contracts the pixel selection boundary by a Radius amount (positive values grow, negative values shrink), with a Circular option rounding the selection shape."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-modifying-pixel-selections"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_modify.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.selections-and-masks.photo-selection-feather"
    name: "Feather selection"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Softens the selection edge falloff by radius; for some selection tools feathering is also applied directly from the context toolbar."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-modifying-pixel-selections"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_modify.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.selections-and-masks.photo-selection-smooth"
    name: "Smooth selection"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Smooths jagged selection boundaries by adjusting the curvature of the selection edge (radius-controlled)."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-modifying-pixel-selections"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_modify.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.selections-and-masks.photo-refine-selection-dialog"
    name: "Refine Selection dialog (matte/edge refinement)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Refines selection edges for hair/fine detail with Matte Edges, Border width, Smooth, Feather and Ramp controls, an adjustment brush (Matte/Foreground/Background/Feather modes) and preview modes (Overlay/Black Matte/White Matte/Black & White/Transparent), outputting to Selection, Mask, New Layer or New Layer With Mask."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-refining-pixel-selection-edges"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_refine.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.selections-and-masks.photo-quick-mask-modes"
    name: "Quick Mask (edit selection as layer)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Toggles the selection into a paintable grayscale mask overlay with alternative display modes for precise selection editing."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-edit-selection-as-layer-using-quick-mask"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/editSelectionAsLayer.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.selections-and-masks.photo-tonal-range-selection"
    name: "Select tonal range (shadows/midtones/highlights)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Creates selections from tonal ranges via Select > Tonal Range with Select Shadows, Select Midtones and Select Highlights; all pixels falling in the chosen range join the selection."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-creating-pixel-selections-by-range"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_range.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.selections-and-masks.photo-luminosity-alpha-selection"
    name: "Selection from layer luminosity/alpha content"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Creates selections directly from a layer's luminosity or content/alpha (desktop leaf), enabling luminosity masking workflows."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.selections-selections_fromlayers"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_fromlayers.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.selections-and-masks.photo-select-subject-matting"
    name: "Select Subject (ML) with matting and macro recording"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "One-click on-device ML subject selection with optional matting control; recordable into macros for batch use."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-select-subject-ml"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_selectSubject.html"
    source_ids: [AFD-S19, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.selections-and-masks.photo-save-load-selections"
    name: "Save/load selections (spare channels/files)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Persists selections to spare channels or files and reloads them later (desktop leaf)."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.selections-saveloadselections"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/saveLoadSelections.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.selections-and-masks.photo-channels-panel-operations"
    name: "Channels panel operations (per-channel edit/load/store)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Channels panel toggles per-channel visibility/editability and converts channel content to/from selections, spare channels and masks."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.channels-channelsselectingediting"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Channels/channelsSelectingEditing.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.selections-and-masks.photo-freehand-selection-modes"
    name: "Freehand selection modes (freehand/polygonal/magnetic)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Freehand selection tool switches between freehand drawing, polygonal click-point and magnetic edge-snapping modes."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.selections-creating-pixel-selections-by-drawing"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Selections/selections_freehand.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-AFFINITY-DEEP-DELTA.color-and-formats] Color Depth, Management, Palettes

```yaml
records:
  - id: "affinity.deep.color-and-formats.suite-bit-depth-8-16-32"
    name: "8/16/32-bit document bit depth"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Documents run at 8/16-bit integer or 32-bit float (linear, unbounded HDR) precision."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.getstarted-aboutbitdepth"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/GetStarted/aboutBitDepth.html"
    source_ids: [AFD-S02, AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.suite-color-format-matrix"
    name: "Color formats: RGB, CMYK, LAB, Grayscale"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Documents can use RGB, CMYK, LAB or Gray color models with per-model channel handling."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.color-color-models"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/ClrModels.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.suite-ocio-v2-pipeline"
    name: "OpenColorIO v2 pipeline"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "OCIO configuration drives 32-bit color space transforms and display transforms; OCIO v2 supported since 2.2."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-using-opencolorio"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/ocio.html"
    source_ids: [AFD-S01, AFD-S27]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.suite-icc-profile-management"
    name: "ICC profile assign/convert on documents"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Documents carry ICC working profiles with assign/convert flows and profile embedding on export."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.color-color-management"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/ClrProfiles.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.suite-swatches-palette-types"
    name: "Palette types: document, application, system"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Swatches panel manages palettes scoped to the document, the application, the OS system palette and PANTONE categories, plus import/export of custom palettes as .afpalette or Adobe Swatch Exchange (ASE) files via Panel Preferences."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-swatchespanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/swatchesPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.color-and-formats.suite-pantone-libraries"
    name: "PANTONE swatch libraries"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Built-in PANTONE palette libraries are selectable from the Swatches panel category list without separate installation and are accessible across Affinity documents."
    primitive_domain: color
    dedupe_status: new_surface
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/swatchesPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.color-and-formats.suite-global-colors-propagation"
    name: "Global colors with live propagation"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Document-scoped global color swatches update every object using them when edited."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.color-global-colors"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/globalClr.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.suite-color-picker-dialog-models"
    name: "Color chooser: multi-model sliders/wheel/hex entry"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Color panel and chooser expose the HSL color wheel plus sliders and boxes across RGB, RGB Hex, HSL, CMYK, LAB and Grayscale models with 8-bit, 16-bit or percentage value modes; help page does not document a 32-bit intensity mode here."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.color-selecting-colors"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/selectingClr.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.color-and-formats.suite-color-chords"
    name: "Color chords (harmony palette generation)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Generates harmony-based swatch sets (chords) from a base color; desktop-only leaf in all three apps."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.clr-clrchords"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/clrChords.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.photo-matting"
    name: "Matting (remove composite fringe)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Removes matte fringing around cut-out content; desktop-only leaf, extended by 2.6 ML-selection matting control."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.clr-clrmatting"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/clrMatting.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.suite-gradient-editor-types"
    name: "Gradient editor: linear/elliptical/radial/conical/bitmap fills"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Gradient/bitmap fill editor supports multiple gradient geometries plus image-based bitmap fills with editable stops."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.color-gradients"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/gradientEditor.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.color-and-formats.photo-32bit-preview-panel"
    name: "32-bit Preview panel (display transform/exposure preview)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Controls non-destructive preview exposure and gamma plus display transform choice (ICC Display Transform, Unmanaged linear light, OCIO Display Transform) with EDR/HDR display options for unbounded 32-bit documents; screen presentation only, document values unchanged."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-32bitpanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/32bitPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.color-and-formats.photo-export-custom-3d-lut"
    name: "Export custom adjustments as 3D LUT"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Exports an adjustment-stack color grade as a 3D LUT file for reuse in other tools."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.adjustments-export-lut"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Adjustments/export_3dLut.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.color-and-formats.suite-overprint-attribute"
    name: "Overprint attribute on fills/strokes"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Marks colors/objects to overprint rather than knock out for prepress separations."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.color-overprinting"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Clr/overprint.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.typography] Typography, OpenType, Text Styles

```yaml
records:
  - id: "affinity.deep.typography.suite-artistic-text-tool"
    name: "Artistic Text Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Creates freely scalable display text objects (drag-to-size) in all three apps."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-text-tools-art-text-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_artText.html"
    source_ids: [AFD-S01, AFD-S02, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-frame-text-tool"
    name: "Frame Text Tool"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Creates body-text containers with reflowing text in all three apps."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.tools-text-tools-frame-text-tool"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_frameText.html"
    source_ids: [AFD-S01, AFD-S02, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-path-text-options"
    name: "Text on a path (start/end and baseline controls)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Flows text along any line/curve/shape with green/orange start-end handles restricting flow extent (secondary-path handles when text overflows), per-section baseline distance control, flow-direction control and Reverse Text Path."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.text-text-on-a-path"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Text/pathText.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.typography.suite-opentype-ligatures"
    name: "OpenType toggle: Ligatures"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies available typeface ligatures to the selected text via the Typography panel."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-opentype-alternates"
    name: "OpenType toggle: Alternates"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies available glyph substitutes to the selected text."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-opentype-stylistic-alternates"
    name: "OpenType toggle: Stylistic Alternates"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies font-derived stylistic substitute forms to the selected text."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-opentype-swash"
    name: "OpenType toggle: Swash"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies ornate swash alternates where the font provides them."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-opentype-stylistic-sets"
    name: "OpenType toggle: Stylistic Sets"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies font stylistic set substitutions to the selected text."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-opentype-figure-style"
    name: "OpenType toggle: Figure Style (lining/oldstyle, proportional/tabular)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Selects font-derived numeral styles for the selected text."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-opentype-figure-position"
    name: "OpenType toggle: Figure Position (superscript/subscript forms)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies font-derived superscript/subscript numeral positioning."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-opentype-capitals"
    name: "OpenType toggle: Capitals (small caps, petite caps, casing)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Controls presentation of capital forms including small-caps variants via font features."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-typographypanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    source_ids: [AFD-S17]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-variable-font-axes"
    name: "Variable font axis control and named instances"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Variable fonts expose predefined instances plus per-axis variation sliders; added in 2.5 across the suite."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.text-variable-fonts"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Text/variableFonts.html"
    source_ids: [AFD-S29]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-text-style-types"
    name: "Text style types: paragraph, character, group"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Style system distinguishes paragraph and character styles (with grouping/base relationships) reusable across the document."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.text-text-styles-text-style-types"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Text/textStyles_types.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.suite-text-style-hierarchy"
    name: "Text style inheritance (based-on) and next-style"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Styles inherit from base styles and can define a following-paragraph style for cascading edits."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.text-text-styles-creating-and-managing-text-styles"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Text/textStyles_create.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.typography.publisher-text-frame-options"
    name: "Text frame options (columns, insets, vertical justification, baseline rules)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Text Frame panel controls per-frame columns/gutters (with column rules and balancing), insets, vertical alignment (top/center/bottom/justified), first-baseline Initial Advance, baseline-grid behavior and frame stroke/fill."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.panels-textframepanel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/textFramePanel.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.typography.publisher-baseline-grid-per-frame"
    name: "Baseline grids (document and per-frame)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Baseline Grid manager aligns text lines to document-wide or frame-scoped baseline grids."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.design-aids-baseline-grids"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/DesignAids/baselineGrids.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.typography.publisher-justification-controls"
    name: "Justification/spacing controls (word/letter spacing)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Paragraph panel Justification section exposes Minimum/Desired/Maximum Word Spacing and Minimum/Desired/Maximum Letter Spacing; no glyph scaling control is documented in Publisher 2."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.text-paragraph-level-paragraph-formatting"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Text/paragraphs.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.typography.publisher-hyphenation-language-dictionaries"
    name: "Hyphenation with per-language dictionaries"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Hyphenation rules and spelling operate per assigned text language with installable dictionaries."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.text-editing-hyphenation"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Text/hyphenation.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.typography.publisher-flow-keep-options"
    name: "Paragraph flow/keep options (widows/orphans/keep-with-next)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Paragraph panel Flow options control widow/orphan prevention (Prevent orphaned first lines, Prevent widowed last lines), Keep with next (line count), Keep with previous paragraph, Keep paragraph together and paragraph start position (Anywhere/Next Column/Next Frame/Next Page)."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.text-paragraph-level-paragraph-formatting"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Text/paragraphs.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.typography.suite-regex-find-replace"
    name: "Regular-expression find and replace"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Find and Replace supports regular expressions with format-aware replacement (documented suite-wide regex reference)."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.extras-regex"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Text/find_and_replace.html"
    source_ids: [AFD-S01, AFD-S03]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.publisher-layout] Layout, References, Data Merge, Preflight

```yaml
records:
  - id: "affinity.deep.publisher-layout.publisher-master-multiple-apply"
    name: "Multiple master pages per page"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Pages can receive more than one master page (modifier-drag or Apply Master with Replace Existing unchecked adds a master in addition to those already applied); each applied master appears as its own layer in the Layers panel."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.pages-spreads-and-sections-master-pages-applying-master-pages"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Pages/applyMasterPages.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.publisher-layout.publisher-nested-masters"
    name: "Nested master pages (master applied to master)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "One master page can be applied to another master page (the help page notes that dropping one master onto another applies the former to the latter), enabling master-on-master cascading; documented as a drag-drop behavior rather than an elaborated feature workflow."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.pages-spreads-and-sections-master-pages-creating-master-pages"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Pages/createMasterPages.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.publisher-layout.publisher-multipage-fold-spreads"
    name: "Multiple-page spreads (gatefold/trifold/accordion)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Spreads may contain more than two pages to model foldable formats such as gatefolds, trifolds and accordion folds; added in 2.6."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.pages-spreads-and-sections-multiple-page-spreads"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Pages/multipageSpreads.html"
    source_ids: [AFD-S22]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-section-numbering-formats"
    name: "Sections with per-section page numbering formats"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Section Manager creates named sections that restart page numbering at a chosen number and swap Number style per section, with per-section include-on-export control and multi-section editing."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.pages-spreads-and-sections-adding-sections"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Pages/addingSections.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.publisher-layout.publisher-running-headers"
    name: "Running headers (content-derived headers/footers)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Header/footer fields pull content (e.g. current heading text) per page; added in 2.1."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.pages-spreads-and-sections-page-headers-and-footers"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Pages/numberingPages.html"
    source_ids: [AFD-S26, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-pinning-modes"
    name: "Object pinning (inline and floating anchored objects)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Pins objects inline within text or floating relative to anchors so they travel with reflow; managed via the Pinning panel."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.object-control-pinning-objects"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/ObjectControl/pinning.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-text-wrap-styles"
    name: "Text wrap styles and editable wrap outline"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Objects carry wrap settings — styles None/Jump/Square/Tight/Inside/Edge, side choice (both sides or widest portion), per-side Distance From Text standoffs — with a wrap outline editable via the Node Tool independently of object geometry."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.text-text-frames-text-wrapping"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Text/wrapText.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.publisher-layout.publisher-books-afbook"
    name: "Books (.afbook multi-document binding)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Book files bind chapter documents with shared numbering for page/list/note numbers and combined output."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.advanced-aboutbooks"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/aboutBooks.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-book-chapter-sync"
    name: "Book chapter synchronization (styles/assets from source chapter)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Synchronizes styles and shared attributes across book chapters from a designated source chapter."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.advanced-syncingchapters"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/syncingChapters.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-toc-style-mapping"
    name: "Table of contents with style-based entry collection"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "TOC generation collects entries by chosen paragraph styles, mapping them to TOC entry styles with page numbers/leaders, and refreshes on demand."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.references-table-of-contents"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/toc.html"
    source_ids: [AFD-S03, AFD-S16]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-toc-multiple-instances"
    name: "Multiple tables of contents per document"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "A document can host as many TOCs as required, including secondary section-specific TOCs (e.g. per-chapter), each with independent settings; Update All Tables of Contents refreshes every TOC at once."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.references-table-of-contents"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/toc.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.publisher-layout.publisher-index-topics-and-marks"
    name: "Index with topics, subtopics and inserted index marks"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Index panel manages topic/subtopic trees and cross-reference entries built from index marks inserted in text, generating a formatted index story."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.references-index"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/index.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.publisher-layout.publisher-notes-numbering-options"
    name: "Footnote/sidenote/endnote numbering and placement options"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Notes restart/renumber per scope and anchor to last text line or frame edge, inside or outside margins; added in 2.3."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.references-footnotes-sidenotes-and-endnotes-styling-notes"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/stylingNotes.html"
    source_ids: [AFD-S30, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-fields-custom-variables"
    name: "Fields panel with custom text variables"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Document fields (author, dates, page counts) plus user-defined custom text variables insertable in text; custom variables added in 2.2."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.references-fields"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/fields.html"
    source_ids: [AFD-S27, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-data-merge-manager"
    name: "Data Merge Manager"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Adds/updates/removes external data sources, embeds the source in the document, previews records and generates the merged document."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.advanced-datamerge"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/dataMerge.html"
    source_ids: [AFD-S24]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-data-merge-sources"
    name: "Data merge sources: CSV/TSV, JSON, XLSX, image paths"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Merge accepts plain/CSV/TSV text, JSON (single top-level array of objects) and XLSX spreadsheets, including image fields via absolute/relative paths."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.advanced-datamerge"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/dataMerge.html"
    source_ids: [AFD-S24]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-data-merge-record-filter"
    name: "Data merge record range filtering"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Filter section restricts generation to a min/max record range instead of all records."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.advanced-datamerge"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/dataMerge.html"
    source_ids: [AFD-S24]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-data-merge-qr-generation"
    name: "QR code generation from data merge fields"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Merge fields can emit per-record QR codes; added in 2.6."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.desktop.leaf.advanced-datamerge"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/dataMerge.html"
    source_ids: [AFD-S22, AFD-S24]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-preflight-profiles-live"
    name: "Preflight profiles, live checking and severity thresholds"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Preflight runs live or on-export against editable profiles that set severity levels and warning/error thresholds, with a status-bar indicator (grey/green/yellow/red)."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-preflight"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/preflight.html"
    source_ids: [AFD-S16]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-preflight-text-checks"
    name: "Preflight text checks (overflow, missing fonts/characters, spelling, text patterns, cross-refs)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Flags overflowing text frames/path text, missing fonts and characters, spelling mistakes, text patterns (double spaces, straight quotes, double hyphens) and stale/missing cross-references."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-preflight"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/preflight.html"
    source_ids: [AFD-S16]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-preflight-layout-checks"
    name: "Preflight layout checks (bleed hazard, non-proportional scaling, thin strokes, hidden objects)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Flags objects outside the bleed zone, non-proportional scaling, too-narrow stroke widths, hidden objects and mismatched RGB color spaces."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-preflight"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/preflight.html"
    source_ids: [AFD-S16]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-preflight-resource-checks"
    name: "Preflight resource checks (low DPI, missing/outdated links, PDF passthrough)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Flags low placed-image DPI, missing or outdated linked resources, PDF passthrough compatibility problems and rasterization-forcing effects."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-preflight"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/preflight.html"
    source_ids: [AFD-S16]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-preflight-ink-checks"
    name: "Preflight ink checks (ink density, rich black, CMY-in-gray)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Flags ink density over thresholds for fills/strokes/text, rich black violations, and CMY usage where only grays/spots are expected."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-preflight"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/preflight.html"
    source_ids: [AFD-S16]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-preflight-accessibility-data-checks"
    name: "Preflight accessibility/data checks (alt text, data merge, TOC refresh, anchors, hyperlinks)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Flags missing alt text, stale data-merge sources, out-of-date TOCs, unnamed anchors and invalid hyperlinks; 2.6 added a mismatched-scaling warning for text flows."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-preflight"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/preflight.html"
    source_ids: [AFD-S16, AFD-S22]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-accessible-pdf-reading-order"
    name: "Accessible PDF authoring (tags, reading order, alt text)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Tags and Reading Order panels plus alt-text tagging drive tagged, screen-reader-friendly PDF output; 2.6 added XMP Alt/Extended Description image tags and structure improvements."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-pdf-publishing-accessible-pdfs"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/accessiblePDFs.html"
    source_ids: [AFD-S22, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.publisher-layout.publisher-page-management-dialog"
    name: "Page management dialog (move/copy pages)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Dedicated dialog moves or copies pages within/between positions; added in 2.6."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.pages-spreads-and-sections-arrange-pages"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Pages/arrangePages.html"
    source_ids: [AFD-S22]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.export-and-formats] Export Persona, Format Matrix, PDF And Print

```yaml
records:
  - id: "affinity.deep.export-and-formats.suite-export-persona-slices"
    name: "Export Persona slices (multi-region export)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Slices define independent export regions from drawn areas, layers or artboards, each with its own format presets and scales."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.export-persona-slices-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/ExportPersona/slicesPanel.html"
    source_ids: [AFD-S01, AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-export-persona-continuous-export"
    name: "Continuous (automatic) slice export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Slices can re-export automatically whenever slice content changes via the Slices panel Continuous option ('slices are re-exported automatically if the content within the slices is modified')."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.export-persona-exporting-using-export-persona"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/ExportPersona/exportPersona.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.export-and-formats.suite-export-persona-multi-scale"
    name: "Multi-scale slice export (1x/2x/3x variants)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Each slice's export format carries multiple export sizes with scaling options (1x, 2x, etc.), and filename tokens include a Scale suffix ('@2x' form, excluding 1x) for per-scale naming; documented on the Slices panel page."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.exportpersona-exportoptionspanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/ExportPersona/exportOptionsPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.export-and-formats.suite-export-settings-dialog"
    name: "Export settings dialog (area, resample, metadata, profile embed)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Per-format export settings cover export area (document/selection/slice), size/resampling, metadata inclusion, color profile embedding and format-specific presets."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.exportpersona-exportsettings"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/ExportPersona/exportSettings.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.export-and-formats.suite-format-native-afphoto-afdesign-afpub"
    name: "Native formats: .afphoto/.afdesign/.afpub interchange"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Each app saves its native format and the sibling apps open each other's native files directly (Publisher opens .afdesign/.afphoto)."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S14]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-aftemplate"
    name: "Template format (.aftemplate)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Documents save/open as reusable templates (.aftemplate) across the suite."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S14]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-psd"
    name: "PSD import/export (Smart Objects import; text rasterized on export)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Opens PSD including Smart Objects as editable embedded documents; exports PSD with text rasterized."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.photo-format-psb"
    name: "PSB (large PSD) import"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Opens Photoshop large-document PSB files."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-ai-import"
    name: "AI (Illustrator) import"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports AI files (PDF-compatible stream); multiple artboards arrive as separate layers/pages."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12, AFD-S13]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-freehand-import"
    name: "Adobe Freehand (10/MX) import"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports legacy Freehand 10/MX files; multi-page files are concatenated and text import is unsupported."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-pdf"
    name: "PDF import/export (pages as layers, JBIG2 decode)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports multi-page PDFs keeping each page as a distinct page/layer (Publisher decodes JBIG2); exports PDF (Photo rasterizes text on PDF export)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12, AFD-S14]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-svg"
    name: "SVG import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports and exports SVG; Designer adds a dedicated SVG authoring workflow page (createSVG)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S13]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-eps"
    name: "EPS import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports and exports EPS for legacy vector interchange."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S13]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-dwg-dxf"
    name: "DWG/DXF import and export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports CAD DWG/DXF (Designer/Publisher) with drawing-scale support; DWG/DXF export added in 2.4 (Designer)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.get-started-importing-cad-documents"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/GetStarted/importCAD.html"
    source_ids: [AFD-S13, AFD-S28]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-format-idml-import"
    name: "IDML (InDesign) import"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Opens InDesign IDML documents converting layout, frames and styles; INDD binary is not supported."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.get-started-importing-indesign-documents"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/GetStarted/importInDesign.html"
    source_ids: [AFD-S14]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-format-idml-export-ambiguity"
    name: "IDML posture: import-only (no IDML export)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Verified posture: the Publisher format appendix lists 'Adobe InDesign (IDML only)' under Open/Import only, not under export; its footnote refers to saving IDML from InDesign for import into Publisher, and INDD import is unsupported. Publisher 2 does not export IDML."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S14]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.export-and-formats.publisher-format-docx-rtf-import"
    name: "DOCX/RTF text import"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Flows Microsoft Word DOCX and RTF files into text frames preserving styles."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S14]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-format-xlsx-import"
    name: "XLSX spreadsheet import (tables/data merge)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Imports XLSX spreadsheets for table content and as a data merge source."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S14, AFD-S24]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-png-hdr"
    name: "PNG import/export incl. 32-bit HDR PNG"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "PNG import/export includes 32-bit HDR PNG support (added around 2.4)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12, AFD-S32]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-jpeg"
    name: "JPEG import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Standard JPEG import/export with quality control."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-jpeg-xl"
    name: "JPEG-XL import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports and exports JPEG-XL across the suite (V2 launch feature)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12, AFD-S32]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.photo-format-jpeg2000"
    name: "JPEG 2000 (J2K/JP2) import"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Opens J2K/JP2 files (import only)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12, AFD-S13]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.photo-format-jxr"
    name: "JPEG-XR/JXR (WDP/HDP) import"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Opens JPEG-XR including 10-10-10 packed HDR variants (import only)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-tiff"
    name: "TIFF import/export (incl. 12-bit and legacy layered TIFF import)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "TIFF import handles 12-bit RGB/Grayscale and legacy layered TIFF with CICP data; TIFF export supported."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-gif"
    name: "GIF import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports and exports GIF (static)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-webp"
    name: "WebP import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports and exports WebP."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-heic-depth"
    name: "HEIF/HEIC import with depth maps as layers"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Opens HEIF/HEIC/HIF (import only); iPhone depth maps load as editable layers and Canon 10-bit HDR HEIF is supported."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-tga"
    name: "TGA import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports and exports Targa TGA (game-art pipelines)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12, AFD-S13]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.designer-format-bmp"
    name: "BMP import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Imports and exports Windows BMP."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S13]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-openexr"
    name: "OpenEXR import/export (32-bit multichannel)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports/exports OpenEXR with 32-bit float data and dedicated OpenEXR support page covering channel handling."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.hdr-32-bit-openexr-support"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/openexr.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-format-radiance-hdr"
    name: "Radiance HDR import/export"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Imports and exports Radiance HDR images."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.photo-format-fits"
    name: "FITS astrophotography import"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Opens FITS astronomy frames for the astrophotography stacking pipeline."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.photo-format-raw-dng-proraw"
    name: "RAW/DNG import incl. Apple ProRAW"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Comprehensive camera RAW and DNG import including Apple ProRAW; 2.6 added eight further camera models."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.photo-raw-engine-selection"
    name: "Dual RAW engines (SerifLabs / Apple Core Image RAW)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Raw development can use the in-house SerifLabs engine or Apple Core Image RAW on macOS."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.appendix-supported-file-formats"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    source_ids: [AFD-S12]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-pdf-export-presets"
    name: "PDF export presets (digital, print, export, flatten, press ready)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Named PDF presets cover digital small/high quality, for-print, for-export, flattened and press-ready output."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-pdf-publishing-publishing-pdf-files"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/publishPDFFiles.html"
    source_ids: [AFD-S15]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-pdf-x-variants"
    name: "PDF/X variants (X-1a:2003, X-3:2003, X-4)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Professional print output supports PDF/X-1a:2003, PDF/X-3:2003 and PDF/X-4 presets."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-pdf-publishing-publishing-pdf-files"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/publishPDFFiles.html"
    source_ids: [AFD-S15]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-pdf-version-compatibility"
    name: "PDF version compatibility (1.6/1.7/2.0)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "PDF export targets PDF 1.6 (Acrobat 7), 1.7 (Acrobat 8) and PDF 2.0 (ISO 32000-2)."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-pdf-publishing-publishing-pdf-files"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/publishPDFFiles.html"
    source_ids: [AFD-S15]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-pdf-security"
    name: "PDF passwords and permission restrictions"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Non-PDF/X exports support open and modify/print passwords with independent printing/copying/editing permission restrictions; password-protected PDF create/open arrived in 2.3."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-pdf-publishing-publishing-pdf-files"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/publishPDFFiles.html"
    source_ids: [AFD-S15, AFD-S30]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-pdf-include-options"
    name: "PDF include options (layers, bookmarks, hyperlinks, bleed, printer marks, tagged PDF)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "PDF export optionally embeds layers, bookmarks, hyperlinks, bleed and printer marks, with tagged-PDF support for accessibility."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-pdf-publishing-publishing-pdf-files"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/publishPDFFiles.html"
    source_ids: [AFD-S15]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-pdf-raster-dpi-color"
    name: "PDF rasterization DPI and color conversion controls"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Raster DPI sets effect-rasterization resolution; color conversion targets RGB/CMYK/as-document with optional image-space conversion and font embed/subset control."
    primitive_domain: export
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-pdf-publishing-publishing-pdf-files"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/publishPDFFiles.html"
    source_ids: [AFD-S15]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.publisher-print-models"
    name: "Print dialog layout models (document/booklet/N-up/tiled)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Print dialog offers four layout models — Single, Tiled (large-format posters/banners), N-Up (multiple copies per sheet) and Booklet (fold/staple imposition) — plus flexible page ranges, paper size, two-sided/short-edge binding, duplex or manual odd/even printing and print-to-PDF."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-print"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/print.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.export-and-formats.suite-bleed-settings"
    name: "Document bleed settings"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Documents define bleed areas honored by print/PDF output (Designer and Publisher bleed pages)."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.sharing-setting-bleed"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/bleed.html"
    source_ids: [AFD-S02, AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.export-and-formats.suite-package-output"
    name: "Package output (document + fonts + linked resources)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Creates a folder package bundling the document with used fonts and linked resources; packages reopen and resave (Designer and Publisher)."
    primitive_domain: prepress
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.publishing-and-sharing-packaging-about-packaging"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/aboutPackaging.html"
    source_ids: [AFD-S02, AFD-S03]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.automation-and-integration] Macros, Batch, Plugins, Hardware, Providers

```yaml
records:
  - id: "affinity.deep.automation-and-integration.photo-macros-record-playback"
    name: "Macros (record/replay operation sequences)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Records editing operations into replayable macros with adjustable step parameters; ML selections are recordable since 2.6."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.macros-and-batch-processing-macros"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Macros_Batch/macros.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.photo-macro-panel"
    name: "Macro panel (recorder/step editor)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Panel hosting macro record/stop/play controls and the recorded step list."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-macro-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/macroPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.photo-library-panel"
    name: "Library panel (saved macro categories)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Stores saved macros in categories for one-click playback and import/export."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-librarypanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/libraryPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.photo-batch-jobs"
    name: "Batch jobs (multi-file processing)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Processes an unrestricted number of source files (including raw) asynchronously across processor cores, optionally applying recorded macros, outputting to one or more formats (AFPhoto/JPEG/PNG/TIFF/OpenEXR/WebP/JPEG-XL) with size options; the Batch panel lists and tracks images being processed."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.macros-and-batch-processing-batch-jobs"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Macros_Batch/batchjobs.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.automation-and-integration.photo-photoshop-plugin-support"
    name: "Photoshop plugin (8bf filter) support"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Loads third-party Photoshop-compatible filter plugins from configured plugin folders (desktop-only page)."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.filters-plugins"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Filters/plugins.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.suite-edit-in-other-affinity-apps"
    name: "Edit-in round trips between Affinity apps"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Open-in-sibling-app commands round-trip the current document between Photo, Designer and Publisher without export."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.getstarted-editinotheraffinityapps"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/GetStarted/editInOtherAffinityApps.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.photo-apple-photos-extension"
    name: "Apple Photos editing extension (Apple-only)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Installs as an Apple Photos editing extension on macOS; platform-exclusive integration."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.extras-applephotosextensions"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Extras/applePhotosExtensions.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.photo-windows-photos-integration"
    name: "Windows Photos integration (Windows-only)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Registers with Windows Photos as an external editor; platform-exclusive integration."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.extras-windowsphotosextensions"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Extras/WindowsPhotosExtensions.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.suite-sidecar-surface-input"
    name: "Sidecar, Surface Pen/Dial, pen tablets, trackpads"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Dedicated support pages cover Apple Sidecar (Apple-only), Microsoft Surface Pen/Dial (Windows-only), pressure pen tablets and trackpad gestures."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.extras-sidecar"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Extras/sidecar.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.suite-hardware-acceleration"
    name: "GPU hardware acceleration toggle"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Compute acceleration uses Metal on macOS and OpenCL on Windows (requires Windows 10.0.19042+ and a Direct3D 12 Feature Level 12.0 GPU); toggled off in Settings/Preferences > Performance for troubleshooting when GPU performance underperforms CPU."
    primitive_domain: diagnostics
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.extras-hardwareacceleration"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Extras/hardwareAcceleration.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.automation-and-integration.photo-benchmark"
    name: "Built-in benchmark (CPU/GPU raster and vector scores)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "In-app benchmark measures raster/vector performance for diagnosing hardware acceleration behavior."
    primitive_domain: diagnostics
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.extras-benchmark"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Extras/benchmark.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.suite-ml-models-local-optional"
    name: "ML models: optional download, strictly on-device"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "ML selection models are an optional free download, uninstallable to reclaim space, run entirely on-device with no data collection; require Apple Silicon macOS 13+ or Windows 10/11 x64/Arm64."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.extras-affinity-and-machine-learning-ml"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Extras/machineLearning.html"
    source_ids: [AFD-S19]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.suite-linked-services"
    name: "Linked Services (cloud storage sign-in; provider-dependent)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Signs into external cloud services to browse/place remote content; depends on third-party providers and network availability."
    primitive_domain: collaboration
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.media-linkedservices"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Media/linkedServices.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.suite-stock-panel-providers"
    name: "Stock panel (external stock photo providers; provider-dependent)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Searches third-party stock imagery providers in-app and drags results into documents; depends on external provider APIs and network."
    primitive_domain: collaboration
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-stock-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/stockPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.automation-and-integration.suite-scripting-posture-v2x"
    name: "Scripting posture in V2.x (no shipped scripting API)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Verified posture: Serif's official support article states 'Affinity V2 does not have scripting support', so V2 desktop automation is limited to macros/batch; the article points to AI Automation in the successor Affinity by Canva app instead. Note: developer.affinity.co is an unrelated CRM company's REST API and is not Affinity/Serif scripting documentation; no user scripting SDK is documented on affinity.studio."
    primitive_domain: automation
    dedupe_status: new_surface
    source_url: "https://developer.affinity.co/"
    source_ids: [AFD-S31, AFD-S32]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-AFFINITY-DEEP-DELTA.panels-and-workspace] Studio Panels And Workspace Surfaces

```yaml
records:
  - id: "affinity.deep.panels-and-workspace.photo-adjustment-panel"
    name: "Adjustment panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Lists all adjustment types with presets for one-click non-destructive application."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-adjustments-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/adjustmentsPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-assets-panel"
    name: "Assets panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Stores reusable design elements in categorized subcategories for drag-in reuse; sorting and background options added in 2.1/2.3."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-assets-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/assetsPanel.html"
    source_ids: [AFD-S01, AFD-S26, AFD-S30]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.photo-batch-panel"
    name: "Batch panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Monitors queued/running batch jobs (desktop-only panel)."
    primitive_domain: automation
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-batchpanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/batchPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-brushes-panel"
    name: "Brushes panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Category-organized brush presets with thumbnails and names (2.1) and in-category search (2.6)."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-brushes-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/brushesPanel.html"
    source_ids: [AFD-S01, AFD-S20, AFD-S26]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.photo-channels-panel"
    name: "Channels panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Displays composite/spare channels with per-channel visibility, editability and selection/mask conversion."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-channels-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/channelsPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-character-panel"
    name: "Character panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Full character-level typography controls (font, size, tracking, kerning, baseline, scaling, decorations, language)."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-characterpanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/characterPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-color-panel"
    name: "Color panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Active color selection across models with wheel/slider/box layouts; 2.6 made its picker auto-apply to selected objects."
    primitive_domain: color
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-color-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/clrPanel.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-glyph-browser-panel"
    name: "Glyph Browser panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Browses and inserts any font glyph including unicode search and alternates."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-glyphpanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/glyphPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.photo-histogram-panel"
    name: "Histogram panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Live per-channel tonal distribution display for exposure diagnostics."
    primitive_domain: diagnostics
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-histogrampanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/histogramPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-history-panel"
    name: "History panel (with save-history-in-document)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Undo history with a Position slider for scrubbing between states; documents can optionally save their undo history inside the file for later sessions; Cycle Future preserves abandoned redo branches, plus Undo Brush source selection and an advanced thumbnail/timestamp view."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-history-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/historyPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.panels-and-workspace.photo-info-panel"
    name: "Info panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Live color/position readouts with placeable samplers (desktop-only panel)."
    primitive_domain: diagnostics
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-infopanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/infoPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-layers-panel"
    name: "Layers panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Layer stack management with drop zones, clipping, masks, blend/opacity controls; 2.6 added right-click Clear Mask and Fill Mask."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-layers-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/layersPanel.html"
    source_ids: [AFD-S01, AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-links-panel"
    name: "Links panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Lists linked/embedded resources with status (desktop-only panel in Photo; Resource Manager covers the same across apps)."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-linkspanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/linksPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.photo-metadata-panel"
    name: "Metadata panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Views/edits EXIF/IPTC metadata on the open image."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-metadata-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/metadataPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-navigator-panel"
    name: "Navigator panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Thumbnail-based pan/zoom navigation of the document view."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-navigator-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/navigatorPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-paragraph-panel"
    name: "Paragraph panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Paragraph-level controls: alignment, leading, indents, spacing, tab stops, flow and decorations."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-paragraphpanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/paragraphPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-quick-fx-panel"
    name: "Quick FX panel (layer effects host)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Applies/edits the layer-effect set (3D, bevel/emboss, overlays, glows, shadows, outline, gaussian blur) per layer."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-layer-fx-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/layerFxPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.photo-scope-panel"
    name: "Scope panel (waveform/vectorscope diagnostics)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Video-style scopes including a vectorscope for chroma analysis (desktop-only, with dedicated vectorscope usage page)."
    primitive_domain: diagnostics
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-scopepanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/scopePanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.photo-snapshots-panel"
    name: "Snapshots panel"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Captures named document states restorable later and convertible to new documents; pairs with Undo Brush."
    primitive_domain: document
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-snapshotspanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/snapshotsPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.photo-sources-panel"
    name: "Sources panel (global clone sources)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Stores multiple clone sources, including cross-document sources, for the Clone/Healing tools (desktop-only)."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-sourcespanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/sourcesPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-styles-panel"
    name: "Styles panel (object styles)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Stores and applies reusable object styles (fill/stroke/fx bundles) in categories."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-stylespanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/stylesPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-text-styles-panel"
    name: "Text Styles panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Manages paragraph/character styles with apply/redefine/delete and hierarchy display."
    primitive_domain: typography
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.panels-textstylespanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/textStylesPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-transform-panel"
    name: "Transform panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Numeric x/y/size/rotation/shear entry with anchor-point control and expression-capable fields."
    primitive_domain: layer_graph
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.studio-panels-transform-panel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/transformPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.designer-appearance-panel"
    name: "Appearance panel (multi-fill/stroke stack)"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Manages stacked fills/strokes per object with reorder and per-entry properties."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.studio-panels-appearance-panel"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Panels/appearancePanel.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.designer-isometric-panel"
    name: "Isometric panel"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Sets active axonometric plane and fits/edits objects onto isometric grids (desktop-only panel)."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.desktop.leaf.panels-isometricpanel"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Panels/isometricPanel.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-stroke-panel"
    name: "Stroke panel"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Stroke width, cap/join/miter, alignment, dash and pressure-profile controls (Designer/Publisher)."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.studio-panels-stroke-panel"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Panels/strokePanel.html"
    source_ids: [AFD-S02]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.publisher-anchors-panel"
    name: "Anchors panel"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Manages named anchor targets for hyperlinks, cross-references and TOC destinations."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.studio-panels-anchors-panel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/anchorsPanel.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.publisher-hyperlinks-panel"
    name: "Hyperlinks panel"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Creates/manages document hyperlinks (URL/page/anchor/email) carried into PDF export."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.studio-panels-hyperlinks-panel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/hyperlinksPanel.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.publisher-pages-panel"
    name: "Pages panel"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Thumbnail page/spread management: add, arrange, duplicate, apply masters and navigate."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.studio-panels-pages-panel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/pagesPanel.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.publisher-table-and-formats-panels"
    name: "Table panel and Table Formats panel"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Table panel edits row/column/cell properties; Table Formats panel stores reusable table format presets."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.studio-panels-table-panel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/tablePanel.html"
    source_ids: [AFD-S03]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.publisher-tags-panel"
    name: "Tags panel (alt text / accessibility tags)"
    record_role: "feature_deep_delta"
    source_app: affinity_publisher_2
    app_behavior: "Assigns alt text and tag semantics to images/objects for accessible export; added in 2.3, XMP-based tags in 2.6."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_publisher.leaf.studio-panels-tags-panel"
    source_url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/tagsPanel.html"
    source_ids: [AFD-S03, AFD-S30, AFD-S22]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-toolbar-customization"
    name: "Toolbar customization"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Top toolbar contents are user-customizable per persona (desktop-only page)."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.workspace-customizingtoolbar"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Workspace/customizingToolbar.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-tools-panel-customization"
    name: "Tools panel customization (layout and tool set)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Vertical tools panel supports customized tool sets and column layout per persona."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.workspace-customizingtoolspanel"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Workspace/customizingToolsPanel.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-keyboard-shortcut-editor"
    name: "Keyboard shortcut editor"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Per-persona shortcut reassignment for tools/menus, with import/export; 2.1 added blend-mode shortcuts, 2.2 long-press tool shortcuts."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.workspace-customizing-keyboard-shortcuts"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Workspace/customizingShortcuts.html"
    source_ids: [AFD-S01, AFD-S26, AFD-S27]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-studio-workspace-presets"
    name: "Studio workspace presets"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Panel arrangements save/restore as named studio presets per app (desktop-only page)."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.workspace-customizingworkspace"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Workspace/customizingWorkspace.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-ui-appearance"
    name: "UI appearance (dark/light, UI scaling)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Switches UI theme and appearance settings including background/UI brightness."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.workspace-changing-the-ui-appearance"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Workspace/uiAppearance.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-workspace-window-modes"
    name: "Application/document window modes (separated/floating views)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Documents run tabbed or as separated/floating windows with multi-window view of one document (desktop-only page)."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.workspace-workspacemodes"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Workspace/workspaceModes.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.designer-view-modes-xray-split"
    name: "View modes: vector/pixel preview, retina, X-ray, split view"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Document view switches between Vector, Pixel, Retina pixel, Outline (wireframe), X-ray (reduced-opacity fills) and Hairline (CAD) modes, with Grayscale and Hide Effects (No FX) options and single/split-view comparison arrangements; Grayscale and No FX arrived in 2.2."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.get-started-viewing-options-and-view-modes"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/GetStarted/view.html"
    source_ids: [AFD-S02, AFD-S27]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.panels-and-workspace.suite-assistant"
    name: "Assistant (automatic action policy manager)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Configurable assistant decides automatic behaviors with per-action policies: painting with no layer selected (add new pixel layer and paint / no action), erasing or retouch-brushing on vector layers (pixel mask / rasterize / no action), and adjustment targeting (new adjustment layer vs child layer), with toggleable alerts and an Enable assistant master switch. V2 help page blocked to fetcher (HTTP 403); behavior cross-verified against Serif's archived Assistant Manager help and the V2 help TOC leaf."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.design-aids-assistant"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/DesignAids/assistant.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.panels-and-workspace.suite-isolation-mode"
    name: "Isolation mode (solo editing scope)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Isolates a layer/object for solo viewing/editing while dimming the rest of the document."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.layer-operations-soloing"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/LayerOperations/isolating.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-expressions-field-input"
    name: "Expressions in numeric field input"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Numeric input fields evaluate arithmetic expressions, units and functions (dedicated expressions reference page)."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.workspace-expressions"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Workspace/expressions.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
  - id: "affinity.deep.panels-and-workspace.suite-quick-grids"
    name: "Quick Grids (object grid duplication)"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Drag-duplicates objects into rows/columns/grids in one gesture (desktop-only leaf in all three apps)."
    primitive_domain: layout
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.desktop.leaf.curvesshapes-objectgrids"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/CurvesShapes/objectGrids.html"
    source_ids: [AFD-S01]
    verification_status: VERIFIED
```

### [SFR-AFFINITY-DEEP-DELTA.version-2x-deltas] 2.1-2.6 Release Deltas Not Elsewhere Rowed

```yaml
records:
  - id: "affinity.deep.version-2x-deltas.suite-2-1-balanced-dash-lines"
    name: "2.1: balanced dashed lines and complex dash patterns"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Dashed strokes can auto-rescale the pattern for clean corners and accept more complex dash patterns; added in 2.1."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.lines-curves-and-shapes-dot-dash-line-styles"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/CurvesShapes/dot_dash_lines.html"
    source_ids: [AFD-S26]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-2-1-guides-improvements-close-all"
    name: "2.1: guides editing improvements and File > Close All"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "2.1 improved guide editing/management and added a Close All command."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.design-aids-ruler-and-column-guides"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/DesignAids/guides.html"
    source_ids: [AFD-S26]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.photo-2-1-brush-auto-clean"
    name: "2.1: auto-clean mixer brush after stroke"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Option to automatically clean the mixer brush after every stroke; added in 2.1."
    primitive_domain: raster
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.painting-and-erasing-mixing-paint-colors"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Painting/paintMixing.html"
    source_ids: [AFD-S26]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-2-2-long-press-tool-shortcuts"
    name: "2.2: long-press tool shortcuts"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Holding a tool shortcut key temporarily switches tools, reverting on release; added in 2.2."
    primitive_domain: interactive
    dedupe_status: new_surface
    source_url: "https://affinityspotlight.com/article/say-hello-to-affinity-22/"
    source_ids: [AFD-S27]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.designer-2-2-data-entry-object-creation"
    name: "2.2: object-creation and move/duplicate data entry dialogs"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Dialog-driven numeric object creation plus a move data entry that repositions, rotates and duplicates objects; added in 2.2."
    primitive_domain: vector
    dedupe_status: new_surface
    source_url: "https://affinityspotlight.com/article/say-hello-to-affinity-22/"
    source_ids: [AFD-S27]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-2-3-pixel-grid"
    name: "2.3: pixel grid display"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "A pixel grid becomes visible at high zoom for pixel-precise work; added in 2.3."
    primitive_domain: interactive
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.design-aids-grids"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/DesignAids/grids.html"
    source_ids: [AFD-S30]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-2-4-camera-support-expansion"
    name: "2.4: expanded camera RAW model support"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "2.4 added support for further camera models alongside 32-bit HDR PNG."
    primitive_domain: camera_raw
    dedupe_status: new_surface
    source_url: "https://alternativeto.net/news/2024/2/affinity-2-4-adds-layer-states-dwg-dxf-support-and-more-to-designer-publisher-and-photo/"
    source_ids: [AFD-S28, AFD-S32]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-2-5-arm64-windows-native"
    name: "2.5: native Windows ARM64 builds"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "2.5 shipped native ARM64 support on Windows for the whole suite."
    primitive_domain: diagnostics
    dedupe_status: new_surface
    source_url: "https://alternativeto.net/news/2024/5/affinity-2-5-brings-variable-fonts-qrcode-tool-stroke-width-tool-and-native-arm64-support/"
    source_ids: [AFD-S29]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.photo-2-6-personas-accept-image-layers"
    name: "2.6: Develop/Liquify/Tone Mapping accept image layers as raster input"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "Image layers load into Develop, Liquify and Tone Mapping Personas as raster layers; added in 2.6."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.introduction-new-features-in-v2-6"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/keyFeatures.html?list=newFeatures"
    source_ids: [AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.designer-2-6-set-selection-box"
    name: "2.6: Set Selection Box for rotated objects"
    record_role: "feature_deep_delta"
    source_app: affinity_designer_2
    app_behavior: "Resets the selection bounding box for objects rotated by 90/180/-90 degrees; added in 2.6."
    primitive_domain: vector
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_designer.leaf.introduction-new-features-in-v2-6"
    source_url: "https://affinity.help/designer2/en-US.lproj/pages/Introduction/keyFeatures.html?list=newFeatures"
    source_ids: [AFD-S21]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-2-6-marquee-modifier-standardization"
    name: "2.6: marquee center-draw, constrain and intersection-toggle modifiers"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Marquee selection tools gained draw-from-center, proportional-constrain modifiers and a keyboard intersection toggle across the suite in 2.6."
    primitive_domain: selection_mask
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.introduction-new-features-in-v2-6"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/keyFeatures.html?list=newFeatures"
    source_ids: [AFD-S20, AFD-S21, AFD-S22]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-2-6-updated-serifLabs-raw-engine"
    name: "2.6: updated SerifLabs RAW engine (eight new camera models)"
    record_role: "feature_deep_delta"
    source_app: affinity_photo_2
    app_behavior: "SerifLabs RAW engine update adds Canon, Fujifilm, Google, Leica, Nikon, Panasonic and Sony models in 2.6."
    primitive_domain: camera_raw
    dedupe_status: deepens_existing
    deepens_leaf_id: "affinity_photo.leaf.introduction-new-features-in-v2-6"
    source_url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/keyFeatures.html?list=newFeatures"
    source_ids: [AFD-S20]
    verification_status: VERIFIED
  - id: "affinity.deep.version-2x-deltas.suite-canva-era-posture"
    name: "Canva-era posture: V2 remains local, free since 2.6-era announcement"
    record_role: "feature_deep_delta"
    source_app: affinity_suite
    app_behavior: "Post-acquisition V2 releases added no cloud-required features (2.1-2.6 release evidence; ML runs on-device per help); the successor all-in-one Affinity by Canva app (affinity.studio, free for individuals, Vector/Pixel/Layout studios) is a separate product line outside V2 scope, with optional Canva AI limited to that app for Canva Premium subscribers."
    primitive_domain: document
    dedupe_status: new_surface
    source_url: "https://www.affinity.studio/"
    source_ids: [AFD-S32, AFD-S19]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-AFFINITY-DEEP-DELTA.post-2-6-unified-relaunch] Post-2.6 Unified Relaunch and 2026 Features

Architecture context for Handshake's own "one unified Studio app" goal: the earlier V2 rows above describe the three-app Photo/Designer/Publisher model (V2.6-era). That model was superseded. At the "Creative Freedom" keynote on 30 October 2025, Canva relaunched Affinity as a single unified desktop app (internal version 3.0, branded simply "Affinity" / "Affinity by Canva") that drops the three separate applications and folds their capability into one interface with three switchable studios/modes — Vector (former Designer), Pixel (former Photo), and Layout (former Publisher). The app is free forever (replacing the former ~USD 70-per-app one-time purchase) and introduces a new universal `.af` document that holds vector, raster and layout content in one file. Work is stored locally on device, but a free Canva account and online sign-in are required to run the app (provider posture: online-account-gated; Handshake Studio's equivalent unification stays fully local-first and offline with no account). Core editing, customization and export are entirely free; generative-AI features (Generative Fill, Generative Expand, Generate Image/Vector) are locked behind a Canva Pro/Premium subscription. It shipped on Windows and macOS (iPad planned later). Two free updates followed within scope: version 3.1 (16 March 2026) and version 3.2 (27 April 2026). All rows in this subtopic are release-notes/press-only surfaces with no covering V2 help leaf, so all are `new_surface`.

```yaml
records:
  - id: "affinity.deep.post-2-6.unified-single-app"
    name: "3.0: unified single app replacing the three separate V2 apps"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "Affinity 3.0 (relaunched 30 Oct 2025) drops the separate Photo, Designer and Publisher apps and integrates all three toolsets into one application."
    primitive_domain: document
    dedupe_status: new_surface
    source_url: "https://en.wikipedia.org/wiki/Affinity_(software)"
    source_ids: [AFD-S44, AFD-S45]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.studios-vector-pixel-layout"
    name: "3.0: personas become Vector/Pixel/Layout studios in one interface"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "The former Photo/Designer/Publisher personas become three switchable studios (Pixel, Vector, Layout) accessed via dedicated tabs inside one unified interface."
    primitive_domain: interactive
    dedupe_status: new_surface
    source_url: "https://www.macrumors.com/2025/10/31/canva-relaunches-affinity-free-app/"
    source_ids: [AFD-S45, AFD-S44]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.universal-af-format"
    name: "3.0: universal .af document format"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "A new universal .af file format stores vector, raster and layout content of a project in a single document, replacing the separate .afphoto/.afdesign/.afpub formats."
    primitive_domain: document
    dedupe_status: new_surface
    source_url: "https://en.wikipedia.org/wiki/Affinity_(software)"
    source_ids: [AFD-S44]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.free-forever-pricing"
    name: "3.0: free-forever pricing (no paid license)"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "The unified app is free forever with no purchase; every Pixel/Vector/Layout tool plus customization and export are unrestricted, replacing the former ~USD 70-per-app one-time purchase model."
    primitive_domain: document
    dedupe_status: new_surface
    source_url: "https://www.canva.com/newsroom/news/affinity-free/"
    source_ids: [AFD-S39, AFD-S44]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.canva-account-required"
    name: "3.0: free Canva account and online sign-in required (provider posture)"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "Running the app requires a free Canva account and online sign-in ('Everyone now needs a Canva account to access the software'); this is an online-account-gated provider posture. Handshake Studio's equivalent unification stays fully local-first with no account or sign-in."
    primitive_domain: collaboration
    dedupe_status: new_surface
    source_url: "https://www.macrumors.com/2025/10/31/canva-relaunches-affinity-free-app/"
    source_ids: [AFD-S45]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.local-storage-offline-editing"
    name: "3.0: work stored locally on device"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "Affinity stores project files locally on the device rather than cloud-first, so editing and documents remain local even though initial sign-in is online."
    primitive_domain: document
    dedupe_status: new_surface
    source_url: "https://www.canva.com/newsroom/news/affinity-free/"
    source_ids: [AFD-S39]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.generative-ai-canva-pro"
    name: "3.0: generative-AI features gated behind Canva Pro"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "Generative-AI features (Generative Fill, Generative Expand, Generate Image, Generate Vector) require a paid Canva Pro/Premium subscription; all non-AI editing is free. Provider posture: AI is an optional paid cloud adapter, not core."
    primitive_domain: automation
    dedupe_status: new_surface
    source_url: "https://en.wikipedia.org/wiki/Affinity_(software)"
    source_ids: [AFD-S44, AFD-S39]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.customizable-unified-workspace"
    name: "3.0: cross-studio customizable, savable and shareable workspace"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "The unified UI lets users show/hide panels and mix tools from any studio, then save workspace layouts per task and share those setups with a team or the community."
    primitive_domain: interactive
    dedupe_status: new_surface
    source_url: "https://www.canva.com/newsroom/news/affinity-free/"
    source_ids: [AFD-S39]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.light-ui-theme"
    name: "3.1: customizable Light UI theme"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "3.1 (16 Mar 2026) adds a Light UI theme with adjustable interface brightness alongside the existing dark theme."
    primitive_domain: interactive
    dedupe_status: new_surface
    source_url: "https://www.affinity.studio/blog/affinity-update-march-2026"
    source_ids: [AFD-S40, AFD-S41]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.convert-to-curves"
    name: "3.1: Convert to Curves (pixel selection to editable vector)"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "A Vector-menu Convert to Curves command turns a pixel selection into a fully editable vector curve instantly, without manual tracing."
    primitive_domain: vector
    dedupe_status: new_surface
    source_url: "https://www.affinity.studio/blog/affinity-update-march-2026"
    source_ids: [AFD-S40, AFD-S42]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.live-tone-blend-groups"
    name: "3.1: Live Tone Blend Groups"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "A Live Tone Blend Group is a dynamic group in which placed layers blend with the underlying composition in real time, non-destructively, simplifying still-image compositing."
    primitive_domain: layer_graph
    dedupe_status: new_surface
    source_url: "https://www.affinity.studio/blog/affinity-update-march-2026"
    source_ids: [AFD-S40, AFD-S42]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.tone-brush"
    name: "3.1: non-destructive Tone Brush"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "A non-destructive Tone Brush paints brightness, contrast and color adjustments onto an image with Dodge/Burn, Blend and Inverse Blend modes."
    primitive_domain: raster
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/03/canva-releases-first-major-update-to-its-free-affinity-software/"
    source_ids: [AFD-S42]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.right-click-brush-menu"
    name: "3.1: right-click brush library menu"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "A right-click contextual menu gives instant access to the full brush library while painting."
    primitive_domain: interactive
    dedupe_status: new_surface
    source_url: "https://www.affinity.studio/blog/affinity-update-march-2026"
    source_ids: [AFD-S40]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.document-tab-context-menu"
    name: "3.1: document tab context menu"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "A right-click menu on document tabs exposes color format/size info, close, float-window and other document-management actions."
    primitive_domain: interactive
    dedupe_status: new_surface
    source_url: "https://www.affinity.studio/blog/affinity-update-march-2026"
    source_ids: [AFD-S40]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.develop-tone-curve-choices"
    name: "3.1: Develop Studio tone curve choices"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "The Develop (RAW) studio adds a choice of tone curves — Compressed, Natural, High Contrast and Log — during RAW processing."
    primitive_domain: camera_raw
    dedupe_status: new_surface
    source_url: "https://petapixel.com/2026/03/17/affinitys-first-free-update-adds-new-features-and-camera-raw-profiles/"
    source_ids: [AFD-S41, AFD-S42]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.v31-camera-raw-models"
    name: "3.1: new camera RAW model support"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "3.1 adds RAW support for Canon R6 Mark III, Sony a7 V (lossless compression only), Fujifilm X-T30 III and Sony RX100 VIIA."
    primitive_domain: camera_raw
    dedupe_status: new_surface
    source_url: "https://petapixel.com/2026/03/17/affinitys-first-free-update-adds-new-features-and-camera-raw-profiles/"
    source_ids: [AFD-S41]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.texture-filter"
    name: "3.2: Texture filter"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "3.2 (27 Apr 2026) adds a Texture filter that enhances midtone detail in images."
    primitive_domain: raster
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.vector-blob-erase-brushes"
    name: "3.2: Vector Blob and Vector Erase brushes"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "New Vector Blob and Vector Erase brushes create and remove filled vector shapes directly by brushing, without first drawing outlines."
    primitive_domain: vector
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.v32-raw-mask-types"
    name: "3.2: new RAW mask types"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "RAW processing gains Object Selection, Luminosity, Hue Range and Compound mask types for masked RAW adjustments."
    primitive_domain: selection_mask
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.multi-band-sharpen-fine-detail"
    name: "3.2: Multi Band Sharpen Fine Detail option"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "The Multi Band Sharpen filter adds a Fine Detail sharpening option."
    primitive_domain: raster
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.davinci-resolve-af-import"
    name: "3.2: DaVinci Resolve .af import with real-time sync"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "DaVinci Resolve can import Affinity .af files as titles/overlays with real-time sync back to edits made in Affinity."
    primitive_domain: export
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.capture-one-af-export"
    name: "3.2: Capture One .af export with preserved masks/metadata"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "Affinity can export .af files to Capture One with masks, watermarks and metadata preserved."
    primitive_domain: export
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.custom-image-bullets"
    name: "3.2: custom image bullet points in layout"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "Page-layout tools support using custom images as bullet points, alongside improved OpenType typography support."
    primitive_domain: typography
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "affinity.deep.post-2-6.natural-language-automation-beta"
    name: "3.2: natural-language desktop automation (beta, provider posture)"
    record_role: "feature_deep_delta"
    source_app: affinity_unified
    app_behavior: "A beta feature enables AI-driven task automation inside Affinity via natural-language commands routed through an external AI desktop assistant; provider posture is external-AI-adapter and optional, not a local scripting engine (V2 had no scripting)."
    primitive_domain: automation
    dedupe_status: new_surface
    source_url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    source_ids: [AFD-S43]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-AFFINITY-DEEP-DELTA.sources] Sources

```yaml
sources:
  - id: AFD-S01
    url: "https://affinity.help/photo2/en-US.lproj/index.html"
    note: "Affinity Photo 2 desktop help TOC; local snapshot _source_snapshots/affinity-photo2-desktop-jina.md (fetched 2025-10-30)."
  - id: AFD-S02
    url: "https://affinity.help/designer2/en-US.lproj/index.html"
    note: "Affinity Designer 2 desktop help TOC; local snapshot _source_snapshots/affinity-designer2-desktop-jina.md."
  - id: AFD-S03
    url: "https://affinity.help/publisher2/en-US.lproj/index.html"
    note: "Affinity Publisher 2 desktop help TOC; local snapshot _source_snapshots/affinity-publisher2-desktop-jina.md."
  - id: AFD-S04
    url: "https://affinity.help/photo2/en-US.lproj/pages/Adjustments/tonalAdjustments.html"
    note: "Tonal adjustments list; fetched 2026-07-09."
  - id: AFD-S05
    url: "https://affinity.help/photo2/en-US.lproj/pages/Adjustments/clrAdjustments.html"
    note: "Color adjustments list; fetched 2026-07-09."
  - id: AFD-S06
    url: "https://affinity.help/photo2/en-US.lproj/pages/Adjustments/otherAdjustments.html"
    note: "Other adjustments list (LUT, Invert, Posterize, Soft Proof, Normals); fetched 2026-07-09."
  - id: AFD-S07
    url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/livefilters.html"
    note: "Live filter catalog by category; fetched 2026-07-09."
  - id: AFD-S08
    url: "https://affinity.help/photo2/en-US.lproj/pages/Layers/layerBlendModes.html"
    note: "Blend mode list incl. Erase; fetched 2026-07-09."
  - id: AFD-S09
    url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_liquify.html"
    note: "Liquify tool set; fetched 2026-07-09."
  - id: AFD-S10
    url: "https://affinity.help/photo2/en-US.lproj/pages/Tools/tools_raw.html"
    note: "Develop Persona tool set; fetched 2026-07-09."
  - id: AFD-S11
    url: "https://affinity.help/photo2/en-US.lproj/pages/Raw/raw_panelBasic.html"
    note: "Develop Basic panel control groups; fetched 2026-07-09."
  - id: AFD-S12
    url: "https://affinity.help/photo2/en-US.lproj/pages/Appendix/fileformat.html"
    note: "Photo import/export format matrix; fetched 2026-07-09."
  - id: AFD-S13
    url: "https://affinity.help/designer2/en-US.lproj/pages/Appendix/fileformat.html"
    note: "Designer import/export format matrix; fetched 2026-07-09."
  - id: AFD-S14
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Appendix/fileformat.html"
    note: "Publisher import/export format matrix; fetched 2026-07-09. IDML export-side grouping treated as ambiguous."
  - id: AFD-S15
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/publishPDFFiles.html"
    note: "PDF presets, PDF/X variants, security, include options; fetched 2026-07-09."
  - id: AFD-S16
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Publishing/preflight.html"
    note: "Preflight checks, profiles, live/export modes; fetched 2026-07-09."
  - id: AFD-S17
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/typographyPanel.html"
    note: "Typography panel OpenType feature toggles; fetched 2026-07-09."
  - id: AFD-S18
    url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/about_Personas.html"
    note: "Photo persona descriptions; fetched 2026-07-09."
  - id: AFD-S19
    url: "https://affinity.help/photo2/en-US.lproj/pages/Extras/machineLearning.html"
    note: "ML feature posture: local-only, optional download; fetched 2026-07-09."
  - id: AFD-S20
    url: "https://affinity.help/photo2/en-US.lproj/pages/Introduction/keyFeatures.html?list=newFeatures"
    note: "Photo 2.6 new-features list; fetched 2026-07-09."
  - id: AFD-S21
    url: "https://affinity.help/designer2/en-US.lproj/pages/Introduction/keyFeatures.html?list=newFeatures"
    note: "Designer 2.6 new-features list; fetched 2026-07-09."
  - id: AFD-S22
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Introduction/keyFeatures.html?list=newFeatures"
    note: "Publisher 2.6 new-features list; fetched 2026-07-09."
  - id: AFD-S23
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Introduction/about_Personas.html"
    note: "Publisher personas and StudioLink requirements; fetched 2026-07-09."
  - id: AFD-S24
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Advanced/dataMerge.html"
    note: "Data merge sources, manager, layout tool, filtering; fetched 2026-07-09."
  - id: AFD-S25
    url: "https://affinity.help/photo2/en-US.lproj/pages/HDR/hdr_tonemapping.html"
    note: "Tone Mapping Persona panels and controls; fetched 2026-07-09."
  - id: AFD-S26
    url: "https://www.cgchannel.com/2023/05/serif-ships-affinity-photo-2-1/"
    note: "2.1 release coverage (crop, brushes, guides, vector flood fill, pixel-persona live filters, running headers); search-verified 2026-07-09."
  - id: AFD-S27
    url: "https://affinityspotlight.com/article/say-hello-to-affinity-22/"
    note: "2.2 release coverage (cross-references, custom variables, long-press shortcuts, No FX/greyscale views, OCIO v2, data entry); search-verified 2026-07-09."
  - id: AFD-S28
    url: "https://alternativeto.net/news/2024/2/affinity-2-4-adds-layer-states-dwg-dxf-support-and-more-to-designer-publisher-and-photo/"
    note: "2.4 release coverage (layer states, DWG/DXF export); search-verified 2026-07-09."
  - id: AFD-S29
    url: "https://alternativeto.net/news/2024/5/affinity-2-5-brings-variable-fonts-qrcode-tool-stroke-width-tool-and-native-arm64-support/"
    note: "2.5 release coverage (variable fonts, QR Code Tool, Stroke Width Tool, ARM64); search-verified 2026-07-09."
  - id: AFD-S30
    url: "https://affinityspotlight.com/article/take-control-of-footnotes-sidenotes-and-endnotes-in-affinity-publisher-2/"
    note: "2.3 notes feature plus release coverage (spiral tool, pixel grid, password PDFs, tags panel); search-verified 2026-07-09."
  - id: AFD-S31
    url: "https://developer.affinity.co/"
    note: "CORRECTED 2026-07-09: fetched page is an unrelated CRM company's REST API docs, not Affinity/Serif scripting documentation; V2 scripting posture is instead verified by AFD-S36."
  - id: AFD-S32
    url: "https://www.affinity.studio/help/release-notes/"
    note: "Canva-era Affinity help/release-notes hub used for release-history cross-checks; search-verified 2026-07-09."
  - id: AFD-S33
    url: "https://affinity.help/photo2/en-US.lproj/pages/ExportPersona/slicesPanel.html"
    note: "Slices panel: Continuous auto re-export and multi-scale export sizes with '@2x' Scale suffix token; fetched 2026-07-09. Verifies suite-export-persona-continuous-export and suite-export-persona-multi-scale."
  - id: AFD-S34
    url: "https://affinity.help/publisher2/en-US.lproj/pages/Panels/paragraphPanel.html"
    note: "Paragraph panel: Justification min/desired/max word and letter spacing (no glyph scaling) and Flow options (orphans/widows, keep with next/previous, keep together, start position); fetched 2026-07-09. Verifies publisher-justification-controls and publisher-flow-keep-options."
  - id: AFD-S35
    url: "https://affinity.help/photo2/en-US.lproj/pages/Panels/batchPanel.html"
    note: "Batch panel lists/tracks images processed by batch jobs; fetched 2026-07-09. Verifies photo-batch-jobs."
  - id: AFD-S36
    url: "https://support.serif.com/hc/en-us/articles/10259359235599-Does-Affinity-V2-have-scripting-support"
    note: "Official Serif support answer: 'Affinity V2 does not have scripting support'; points to AI Automation in Affinity by Canva instead; fetched 2026-07-09 via reader proxy. Verifies suite-scripting-posture-v2x."
  - id: AFD-S37
    url: "https://s3-eu-west-1.amazonaws.com/affinity-docs/help/designer/en-US.lproj/pages/DesignAids/AssistantManager.html"
    note: "Serif Assistant Manager help (archived V1 mirror) documenting per-action assistant policies; fetched 2026-07-09 because the V2 assistant.html pages returned HTTP 403 to the fetcher. Cross-verifies suite-assistant."
  - id: AFD-S38
    url: "https://www.affinity.studio/"
    note: "Affinity by Canva homepage: free-for-individuals all-in-one app (Vector/Pixel/Layout), separate from V2 suite, optional Canva AI for Premium subscribers, no scripting mentioned; fetched 2026-07-09 via reader proxy. Verifies suite-canva-era-posture and scripting-posture context."
  - id: AFD-S39
    url: "https://www.canva.com/newsroom/news/affinity-free/"
    note: "Canva newsroom relaunch announcement: free-forever rationale, 'Affinity stores your work locally on your device', no AI training on Affinity content, generative AI within Affinity; fetched 2026-07-09 via reader proxy. Verifies free-forever-pricing, local-storage-offline-editing, customizable-unified-workspace, generative-ai-canva-pro context."
  - id: AFD-S40
    url: "https://www.affinity.studio/blog/affinity-update-march-2026"
    note: "Official Affinity blog, March 2026 (v3.1) update: Light UI, Convert to Curves, Live Tone Blend Groups, right-click brush menu, document tab context menu; released 16 Mar 2026; fetched 2026-07-09. Verifies light-ui-theme, convert-to-curves, live-tone-blend-groups, right-click-brush-menu, document-tab-context-menu."
  - id: AFD-S41
    url: "https://petapixel.com/2026/03/17/affinitys-first-free-update-adds-new-features-and-camera-raw-profiles/"
    note: "PetaPixel coverage of Affinity 3.1: new camera RAW models (Canon R6 Mark III, Sony a7 V lossless-only, Fujifilm X-T30 III, Sony RX100 VIIA) and Develop tone curve choices (Compressed/Natural/High Contrast/Log); fetched 2026-07-09. Verifies v31-camera-raw-models, develop-tone-curve-choices."
  - id: AFD-S42
    url: "https://www.cgchannel.com/2026/03/canva-releases-first-major-update-to-its-free-affinity-software/"
    note: "CG Channel coverage of Affinity 3.1 (first major free update): Tone Brush (Dodge/Burn, Blend, Inverse Blend), selection-to-curves, Live Tone Blend Group, Develop tone-curve choices, version number 3.1; fetched 2026-07-09. Verifies tone-brush and v3.1 version numbering."
  - id: AFD-S43
    url: "https://www.cgchannel.com/2026/04/canva-releases-affinity-3-2/"
    note: "CG Channel coverage of Affinity 3.2 (27 Apr 2026): Texture filter, Vector Blob/Erase brushes, new RAW mask types (Object Selection/Luminosity/Hue Range/Compound), Multi Band Sharpen Fine Detail, DaVinci Resolve .af import, Capture One .af export, custom image bullets, OpenType, natural-language desktop automation beta; fetched 2026-07-09. Verifies all 3.2 post-2-6 rows."
  - id: AFD-S44
    url: "https://en.wikipedia.org/wiki/Affinity_(software)"
    note: "Wikipedia Affinity (software): confirms 30 Oct 2025 'Creative Freedom' keynote relaunch as internal version 3.0, 'Version 3 drops the separate applications and integrates their functionality into a singular application', Vector/Pixel/Layout studios, custom .af file format, Canva AI locked behind Canva Pro; fetched 2026-07-09. Verifies unified-single-app, studios, universal-af-format, generative-ai-canva-pro."
  - id: AFD-S45
    url: "https://www.macrumors.com/2025/10/31/canva-relaunches-affinity-free-app/"
    note: "MacRumors relaunch coverage: 'Everyone now needs a Canva account to access the software, but signing up is free'; unifies vector/photo/layout into one app with Vector/Pixel/Layout tabs; fetched 2026-07-09. Verifies canva-account-required, studios-vector-pixel-layout."
```
