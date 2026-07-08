---
file_id: studio-app-feature-research-affinity-suite
topic_id: SFR-AFFINITY
title: "Affinity Suite Feature Map"
status: draft
summary: "Affinity Photo, Designer, and Publisher V2 feature families, including StudioLink and the newer unified Affinity direction."
sources: 9
updated_at: "2026-07-05"
---

## [SFR-AFFINITY] Affinity Suite Feature Map

### [SFR-AFFINITY.source-shape] Source Shape

```yaml
split_decision:
  requested_shape: "Research Affinity for rebuilding Studio."
  inventory_shape: "Use Affinity V2 suite split: Affinity Photo 2, Affinity Designer 2, Affinity Publisher 2."
  current_market_note: "Affinity now also appears as a newer unified photo/vector/layout app under Canva/Affinity branding; that is source context, not a rename of this inventory."
  desktop_help_blocker: "Direct desktop `affinity.help/*2/en-US.lproj/contents.xml` paths returned 403; iPad V2 XML indexes and current Affinity Studio pages were used as official structure sources."
```

### [SFR-AFFINITY.photo] Affinity Photo Features

```yaml
app: "Affinity Photo 2"
features:
  - { id: "affinity_photo.non_destructive_raw_develop", name: "Non-destructive RAW Develop", app_behavior: "Develop RAW files, then revisit develop settings after layer/adjustment edits; embed or link RAW source.", primitive_domain: raw, source_ids: [AF-S01, AF-S02] }
  - { id: "affinity_photo.compound_masks", name: "Compound Masks", app_behavior: "Combine mask layers non-destructively using add/intersect/subtract/XOR.", primitive_domain: mask, source_ids: [AF-S01, AF-S03] }
  - { id: "affinity_photo.live_masks_hue_range", name: "Live Mask: Hue Range", app_behavior: "Auto-updating mask based on selected hue range.", primitive_domain: mask, source_ids: [AF-S01] }
  - { id: "affinity_photo.live_masks_band_pass", name: "Live Mask: Band-Pass", app_behavior: "Auto-updating edge/frequency-focused mask for retouching and effects.", primitive_domain: mask, source_ids: [AF-S01] }
  - { id: "affinity_photo.live_masks_luminosity", name: "Live Mask: Luminosity", app_behavior: "Mask by highlight/shadow/luminosity ranges.", primitive_domain: mask, source_ids: [AF-S01] }
  - { id: "affinity_photo.live_mesh_warp", name: "Live Mesh Warp", app_behavior: "Non-destructive warp for raster/file composites, editable after placement.", primitive_domain: raster, source_ids: [AF-S01, AF-S03] }
  - { id: "affinity_photo.saved_layer_states", name: "Saved Layer States", app_behavior: "Store manual or smart visibility states filtered by layer tag/type/name/lock status.", primitive_domain: layer, source_ids: [AF-S01] }
  - { id: "affinity_photo.select_subject_ml", name: "Select Subject (ML)", app_behavior: "One-click subject selection using ML models.", primitive_domain: selection, source_ids: [AF-S03, AF-S04] }
  - { id: "affinity_photo.object_selection_tool_ml", name: "Object Selection Tool (ML)", app_behavior: "Select objects in image content with ML-assisted pixel selections.", primitive_domain: selection, source_ids: [AF-S03, AF-S04] }
  - { id: "affinity_photo.frequency_separation", name: "Frequency Separation", app_behavior: "Retouch workflow split by frequency layers.", primitive_domain: raster, source_ids: [AF-S03] }
  - { id: "affinity_photo.inpainting_patch_heal", name: "Inpainting / Patch / Healing", app_behavior: "Remove blemishes or image content with inpainting, patching, clone/heal tools.", primitive_domain: raster, source_ids: [AF-S03] }
  - { id: "affinity_photo.live_filters_adjustments", name: "Live Filters and Adjustment Layers", app_behavior: "Non-destructive filters and tonal/color adjustments in layer stack.", primitive_domain: layer, source_ids: [AF-S03] }
  - { id: "affinity_photo.hdr_32bit_ocio", name: "32-bit HDR / OpenEXR / OpenColorIO", app_behavior: "32-bit HDR edit/merge/tone-map workflows with OpenEXR and OCIO support.", primitive_domain: color, source_ids: [AF-S03] }
  - { id: "affinity_photo.image_stacks", name: "Image Stacks", app_behavior: "Stack images for exposure merge, object removal, noise reduction, and creative effects.", primitive_domain: raster, source_ids: [AF-S03] }
  - { id: "affinity_photo.focus_merge", name: "Focus Merge", app_behavior: "Merge focus-bracketed source images.", primitive_domain: raster, source_ids: [AF-S03] }
  - { id: "affinity_photo.panorama_stitching", name: "Panorama Stitching", app_behavior: "Stitch and edit panoramas.", primitive_domain: raster, source_ids: [AF-S03] }
  - { id: "affinity_photo.liquify_persona", name: "Liquify Persona", app_behavior: "Warp pixels with liquify tools, masks, brush and mesh panels.", primitive_domain: raster, source_ids: [AF-S03] }
  - { id: "affinity_photo.macros_batch_jobs", name: "Macros and Batch Jobs", app_behavior: "Record repeated actions and process files in batches.", primitive_domain: automation, source_ids: [AF-S03] }
  - { id: "affinity_photo.jpeg_xl_import_export", name: "JPEG XL Import/Export", app_behavior: "Import/export JPEG XL, including wide-gamut/HDR workflow relevance.", primitive_domain: export, source_ids: [AF-S01] }
```

### [SFR-AFFINITY.designer] Affinity Designer Features

```yaml
app: "Affinity Designer 2"
features:
  - { id: "affinity_designer.vector_warp", name: "Vector Warp", app_behavior: "Non-destructive warp over vector artwork/text with mesh and presets.", primitive_domain: vector, source_ids: [AF-S01] }
  - { id: "affinity_designer.shape_builder", name: "Shape Builder Tool", app_behavior: "Combine/subtract overlapping shape segments by dragging/modifier actions.", primitive_domain: vector, source_ids: [AF-S01] }
  - { id: "affinity_designer.knife_scissor", name: "Knife and Scissor Tools", app_behavior: "Slice shapes/curves/text; split curve nodes or segments.", primitive_domain: vector, source_ids: [AF-S01] }
  - { id: "affinity_designer.measure_area_tools", name: "Measure Tool and Area Tool", app_behavior: "Measure distances, line lengths, areas, perimeters, and segment lengths to scale.", primitive_domain: vector, source_ids: [AF-S01] }
  - { id: "affinity_designer.dwg_dxf_import", name: "DWG/DXF Import", app_behavior: "Import/edit AutoCAD/DXF files while retaining layer structure and scale.", primitive_domain: file_io, source_ids: [AF-S01, AF-S04] }
  - { id: "affinity_designer.xray_view", name: "X-Ray View", app_behavior: "View object/curve makeup for selection inside complex artwork.", primitive_domain: vector, source_ids: [AF-S01] }
  - { id: "affinity_designer.pen_node_curve_editing", name: "Pen / Node / Curve Editing", app_behavior: "Precision curve/node editing and vector drawing.", primitive_domain: vector, source_ids: [AF-S04, AF-S05] }
  - { id: "affinity_designer.vector_flood_fill", name: "Vector Flood Fill", app_behavior: "Flood areas formed by overlapping shapes/curves.", primitive_domain: vector, source_ids: [AF-S06] }
  - { id: "affinity_designer.pixel_persona_raster_tools", name: "Pixel Persona Raster Tools", app_behavior: "Raster tools in Designer context: flood fill, smudge, paint/erase, symmetry, mesh/perspective.", primitive_domain: raster, source_ids: [AF-S04, AF-S07] }
  - { id: "affinity_designer.text_on_path", name: "Text on a Path", app_behavior: "Place/edit text along vector paths.", primitive_domain: typography, source_ids: [AF-S04, AF-S07] }
  - { id: "affinity_designer.export_persona_slices", name: "Export Persona / Slices", app_behavior: "Slice/layer-based export workflow.", primitive_domain: export, source_ids: [AF-S03, AF-S04] }
```

### [SFR-AFFINITY.publisher] Affinity Publisher Features

```yaml
app: "Affinity Publisher 2"
features:
  - { id: "affinity_publisher.books", name: "Books", app_behavior: "Combine Publisher documents as chapters; sync page numbers, TOC, indexes, and styles.", primitive_domain: layout, source_ids: [AF-S01, AF-S05] }
  - { id: "affinity_publisher.footnotes_endnotes_sidenotes", name: "Footnotes / Endnotes / Sidenotes", app_behavior: "Add academic-style notes/references to text.", primitive_domain: typography, source_ids: [AF-S01, AF-S05, AF-S08] }
  - { id: "affinity_publisher.place_autoflow", name: "Place Auto-flow", app_behavior: "Repeat layout automatically until desired images are accommodated; repeat-count variants.", primitive_domain: layout, source_ids: [AF-S01] }
  - { id: "affinity_publisher.linked_file_layer_visibility_override", name: "Linked File Layer Visibility Override", app_behavior: "Toggle layers inside placed PSD/PDF/DWG/DXF/Affinity files while preserving links.", primitive_domain: layer, source_ids: [AF-S01] }
  - { id: "affinity_publisher.dwg_dxf_place", name: "DWG/DXF Place", app_behavior: "Place DXF/DWG files in publications.", primitive_domain: file_io, source_ids: [AF-S01, AF-S05] }
  - { id: "affinity_publisher.style_picker_tool", name: "Style Picker Tool", app_behavior: "Pick style from object/text and apply to other objects/text.", primitive_domain: typography, source_ids: [AF-S01] }
  - { id: "affinity_publisher.smart_master_pages_text_styles", name: "Smart Master Pages and Shared Text Styles", app_behavior: "Reusable page/layout and typography consistency system.", primitive_domain: layout, source_ids: [AF-S05, AF-S09] }
  - { id: "affinity_publisher.studio_link_photo_designer_tools", name: "StudioLink", app_behavior: "Use Photo/Designer editing tools from Publisher workflow.", primitive_domain: studio_link, source_ids: [AF-S01, AF-S05] }
  - { id: "affinity_publisher.picture_frames_tables_data_merge", name: "Picture Frames / Tables / Data Merge", app_behavior: "Layout Studio exposes picture frame, table, and data merge tools.", primitive_domain: layout, source_ids: [AF-S05, AF-S08] }
```

### [SFR-AFFINITY.implementation-notes] Implementation Notes

```text
Affinity's key rebuild lesson is persona/studio crossing: raster, vector, and page-layout tools can operate inside one document model without launching separate applications. That aligns strongly with Handshake's single worksurface direction.

For Handshake, StudioLink should not be copied as an app-switching UX. It should become a shared primitive architecture: the same layer graph, vector path engine, layout frame engine, and export system exposed through task-focused work modes.
```

### [SFR-AFFINITY.gaps] Gaps

```yaml
gaps:
  - id: AF-GAP-001
    detail: "Desktop Affinity help XML remains blocked, but desktop index.html pages were parsed through local Jina Reader snapshots and reconciled against the iPad V2 XML index."
    next_step: "Use 09-affinity-desktop-delta.md for desktop deltas; capture desktop contents.xml later only if exact vendor-TOC fidelity is required."
  - id: AF-GAP-002
    detail: "New unified Affinity app behavior may supersede parts of the V2 suite split."
    next_step: "Add a separate topic if Handshake wants current unified Affinity app parity rather than V2 suite parity."
```

### [SFR-AFFINITY.sources] Sources

```yaml
sources:
  - { id: AF-S01, url: "https://forum.affinity.serif.com/index.php?/topic/170152-affinity-version-2-sets-new-standards-in-creative-software/", note: "Official Affinity V2 launch/release feature list thread." }
  - { id: AF-S02, url: "https://www.affinity.studio/help/raw-raw/", note: "Current Affinity RAW help page; direct browser tool showed unsupported client but search/source metadata confirmed page." }
  - { id: AF-S03, url: "https://affinity.help/photo2ipad/en-US.lproj/contents.xml", note: "Affinity Photo 2 iPad help table of contents used as official feature index." }
  - { id: AF-S04, url: "https://affinity.help/designer2ipad/en-US.lproj/contents.xml", note: "Affinity Designer 2 iPad help table of contents used as official feature index." }
  - { id: AF-S05, url: "https://affinity.help/publisher2ipad/en-US.lproj/contents.xml", note: "Affinity Publisher 2 iPad help table of contents used as official feature index." }
  - { id: AF-S06, url: "https://www.affinity.studio/help/tools-tools-vector-flood-fill/", note: "Current Affinity vector flood fill help page." }
  - { id: AF-S07, url: "https://forum.affinity.serif.com/index.php?/topic/170248-official-affinity-designer-v2-tutorials/", note: "Official Affinity Designer V2 tutorials thread." }
  - { id: AF-S08, url: "https://www.affinity.studio/help/page-layout/", note: "Current Affinity page-layout help page." }
  - { id: AF-S09, url: "https://www.canva.com/newsroom/news/all-new-affinity/", note: "Canva/Affinity current unified app announcement." }
```
