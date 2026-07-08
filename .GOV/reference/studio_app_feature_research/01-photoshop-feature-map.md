---
file_id: studio-app-feature-research-photoshop
topic_id: SFR-PHOTOSHOP
title: "Adobe Photoshop Feature Map"
status: draft
summary: "Photoshop desktop, Camera Raw, generative AI, automation, export, and collaboration feature families."
sources: 16
updated_at: "2026-07-05"
---

## [SFR-PHOTOSHOP] Adobe Photoshop Feature Map

### [SFR-PHOTOSHOP.inventory] Feature Inventory

```yaml
as_of: "2026-07-05"
app: "Adobe Photoshop desktop"
feature_families:
  - category: "Core workspace"
    features:
      - { id: "photoshop.workspace.panels", name: "Dockable panels and workspaces", app_behavior: "Arrange, group, hide/show, save, switch, and restore panels/workspaces.", primitive_domain: automation, source_ids: [PS-S01] }
      - { id: "photoshop.workspace.contextual_task_bar", name: "Contextual Task Bar", app_behavior: "Shows task-specific commands near the active canvas/workflow.", primitive_domain: automation, source_ids: [PS-S01] }
      - { id: "photoshop.history.snapshots", name: "History states and snapshots", app_behavior: "Tracks image states, snapshots, undo/redo, and restoration from previous states.", primitive_domain: raster, source_ids: [PS-S01] }
  - category: "File I/O and documents"
    features:
      - { id: "photoshop.file_io.open_import", name: "Open/import image files", app_behavior: "Open local documents, import assets, and hand off supported formats into Photoshop.", primitive_domain: file_io, source_ids: [PS-S01, PS-S03] }
      - { id: "photoshop.file_io.psd_psb", name: "PSD/PSB layered documents", app_behavior: "Save normal and large layered Photoshop documents.", primitive_domain: file_io, source_ids: [PS-S01, PS-S03] }
      - { id: "photoshop.file_io.cloud_documents", name: "Cloud documents", app_behavior: "Cloud-native Photoshop files with cross-device access, autosave, and full-fidelity layer storage.", primitive_domain: collaboration, source_ids: [PS-S02] }
      - { id: "photoshop.file_io.supported_formats", name: "Supported import/export formats", app_behavior: "Work with supported image, video, audio, and legacy/3D-related file formats.", primitive_domain: file_io, source_ids: [PS-S03] }
  - category: "Layers"
    features:
      - { id: "photoshop.layer.stack", name: "Layer stack", app_behavior: "Create, select, duplicate, rename, delete, reorder, group, link, and sample visible layers.", primitive_domain: layer, source_ids: [PS-S01] }
      - { id: "photoshop.layer.adjustment", name: "Adjustment layers", app_behavior: "Apply non-destructive color/tone changes through editable layers.", primitive_domain: layer, source_ids: [PS-S01] }
      - { id: "photoshop.layer.fill", name: "Fill layers", app_behavior: "Apply editable solid color, gradient, or pattern fills.", primitive_domain: layer, source_ids: [PS-S01] }
      - { id: "photoshop.layer.effects_styles", name: "Layer effects/styles", app_behavior: "Apply, copy, hide/show, scale, manage, and rasterize visual layer effects.", primitive_domain: layer, source_ids: [PS-S01] }
      - { id: "photoshop.layer.smart_objects", name: "Smart Objects", app_behavior: "Embed/link external content, preserve transforms, edit/replace contents, and rasterize when needed.", primitive_domain: layer, source_ids: [PS-S01] }
      - { id: "photoshop.layer.comps_alignment", name: "Layer comps/alignment", app_behavior: "Record layer-state variants and align/auto-align/distribute layers.", primitive_domain: layer, source_ids: [PS-S01] }
  - category: "Selections"
    features:
      - { id: "photoshop.selection.manual_tools", name: "Manual selection tools", app_behavior: "Lasso, polygonal/magnetic lasso, quick selection, selection brush, and move/copy/paste selections.", primitive_domain: selection, source_ids: [PS-S01] }
      - { id: "photoshop.selection.object_subject_people", name: "Object/subject/people selection", app_behavior: "Detect objects, subjects, people, hair, and layer objects for faster isolation.", primitive_domain: selection, source_ids: [PS-S01] }
      - { id: "photoshop.selection.color_based", name: "Color-based selection", app_behavior: "Magic Wand, Color Range, skin-tone presets, and cleanup of color-based selection edges.", primitive_domain: selection, source_ids: [PS-S01] }
      - { id: "photoshop.selection.refine_edges", name: "Refine and modify selections", app_behavior: "Feather, anti-alias, expand/contract, invert, border, soften, defringe, and matte removal.", primitive_domain: selection, source_ids: [PS-S01] }
  - category: "Masks and compositing"
    features:
      - { id: "photoshop.mask.layer_mask", name: "Layer masks", app_behavior: "Hide/reveal parts of layers using selections, painting, transparency, disable/apply/delete, and unlink.", primitive_domain: mask, source_ids: [PS-S01, PS-S04] }
      - { id: "photoshop.mask.detected_object_masks", name: "Object masks", app_behavior: "Create layer masks for all detected objects in a layer.", primitive_domain: mask, source_ids: [PS-S01] }
      - { id: "photoshop.mask.vector_mask", name: "Vector masks", app_behavior: "Mask layers with resolution-independent paths.", primitive_domain: mask, source_ids: [PS-S04] }
      - { id: "photoshop.mask.auto_blend", name: "Auto-Blend Layers", app_behavior: "Blend aligned layers for composites, panoramas, and extended depth of field.", primitive_domain: mask, source_ids: [PS-S01] }
  - category: "Raster transform and geometry"
    features:
      - { id: "photoshop.raster.resize_resample", name: "Image size/resolution/resampling", app_behavior: "Change pixel dimensions, print dimensions, resolution, and resampling method.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.raster.crop_straighten", name: "Crop/straighten/perspective crop", app_behavior: "Crop canvas, straighten tilted photos, and transform perspective while cropping.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.raster.free_transform", name: "Transform/rotate/scale/perspective", app_behavior: "Scale, rotate, flip, duplicate while transforming, set reference point, and apply transforms.", primitive_domain: raster, source_ids: [PS-S01, PS-S05] }
      - { id: "photoshop.raster.content_aware_scale", name: "Content-Aware Scale", app_behavior: "Scale while protecting specified visual content.", primitive_domain: raster, source_ids: [PS-S01] }
  - category: "Repair and retouch"
    features:
      - { id: "photoshop.retouch.remove_tool", name: "Remove tool", app_behavior: "Brush/loop unwanted objects or distractions and have Photoshop fill the area.", primitive_domain: raster, source_ids: [PS-S06] }
      - { id: "photoshop.retouch.content_aware_fill", name: "Content-Aware Fill", app_behavior: "Fill removed areas using sampled surrounding image content with adjustable settings.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.retouch.heal_patch_clone", name: "Healing/Patch/Clone family", app_behavior: "Repair spots, large areas, sampled regions, red eye, and similar imperfections.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.retouch.remove_background", name: "Remove/replace background", app_behavior: "Isolate foreground and remove or generate replacement backgrounds.", primitive_domain: selection, source_ids: [PS-S01] }
  - category: "Color and tone"
    features:
      - { id: "photoshop.color.profiles_modes", name: "Profiles and color modes", app_behavior: "Embed/change profiles and convert RGB, CMYK, grayscale, bitmap, and indexed-color modes.", primitive_domain: color, source_ids: [PS-S01] }
      - { id: "photoshop.color.picker_swatches_spot", name: "Color picker, swatches, spot colors", app_behavior: "Choose foreground/background colors, web-safe/CMYK equivalents, spot colors, and libraries.", primitive_domain: color, source_ids: [PS-S01] }
      - { id: "photoshop.color.adjustments", name: "Hue/Saturation, Colorize, Black & White, Match Color", app_behavior: "Apply global or selective color/tone corrections and color matching.", primitive_domain: color, source_ids: [PS-S01] }
      - { id: "photoshop.color.ocio", name: "OCIO/ACES color management", app_behavior: "Use OCIO/ACES color-management configurations and HDR/profile handling where supported.", primitive_domain: color, source_ids: [PS-S07] }
  - category: "Painting and fills"
    features:
      - { id: "photoshop.paint.brush_engine", name: "Brush tools and presets", app_behavior: "Paint with configurable brush tips, smoothing, presets, brush groups, and imported brush packs.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.paint.fill_stroke", name: "Fill and stroke", app_behavior: "Fill selections/layers/canvas and stroke selections/layers with color.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.paint.patterns", name: "Patterns and Pattern Preview", app_behavior: "Create, preview, and fill with repeatable patterns.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.paint.erasers", name: "Eraser family", app_behavior: "Erase pixels, auto-erase, remove similar pixels, or erase backgrounds to transparency.", primitive_domain: raster, source_ids: [PS-S01] }
  - category: "Vector, shapes, paths, layout"
    features:
      - { id: "photoshop.vector.shapes", name: "Shape layers", app_behavior: "Draw rectangles, circles, polygons, custom shapes, stars, lines, and arrows with fill/stroke.", primitive_domain: vector, source_ids: [PS-S01] }
      - { id: "photoshop.vector.pen_paths", name: "Pen tool and paths", app_behavior: "Draw/edit curves, straight segments, paths, and convert paths/selections/text.", primitive_domain: vector, source_ids: [PS-S01] }
      - { id: "photoshop.vector.content_aware_tracing", name: "Content-Aware Tracing", app_behavior: "Trace image content into paths.", primitive_domain: vector, source_ids: [PS-S01] }
      - { id: "photoshop.layout.artboards_frames", name: "Artboards and frames", app_behavior: "Create multi-canvas documents, add artboards, draw frames, place images, and convert text/shapes to frames.", primitive_domain: vector, source_ids: [PS-S01] }
  - category: "Typography"
    features:
      - { id: "photoshop.typography.text_layers", name: "Text layers", app_behavior: "Add, edit, resize, move, rotate, color, copy/paste, and format paragraph text.", primitive_domain: typography, source_ids: [PS-S01] }
      - { id: "photoshop.typography.fonts_opentype", name: "Fonts/OpenType/variable fonts", app_behavior: "Find/replace fonts, match fonts, apply OpenType features, SVG fonts, glyphs, and variable fonts.", primitive_domain: typography, source_ids: [PS-S01] }
      - { id: "photoshop.typography.text_on_path", name: "Text on paths/shapes", app_behavior: "Place, flip, move, warp/unwarp text along paths or inside shapes.", primitive_domain: typography, source_ids: [PS-S01] }
      - { id: "photoshop.typography.international_text", name: "Unified Text Engine/international scripts", app_behavior: "Create documents using supported international languages and scripts.", primitive_domain: typography, source_ids: [PS-S01] }
  - category: "Filters and effects"
    features:
      - { id: "photoshop.filter.gallery", name: "Filter Gallery and filter application", app_behavior: "Apply filters, use gallery previews, fade/blend filter effects.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.filter.smart_filters", name: "Smart Filters", app_behavior: "Apply editable filter effects to Smart Objects.", primitive_domain: layer, source_ids: [PS-S01] }
      - { id: "photoshop.filter.blur_sharpen", name: "Blur and sharpen", app_behavior: "Lens blur, blur/sharpen tools, Smart Sharpen, Unsharp Mask, and edge-mask sharpening.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.filter.warp_liquify_puppet", name: "Warp/Liquify/Puppet Warp", app_behavior: "Distort image regions, transform warps, freeze/thaw areas, meshes/backdrops, and reconstruct distortions.", primitive_domain: raster, source_ids: [PS-S01] }
      - { id: "photoshop.filter.neural_filters", name: "Neural Filters", app_behavior: "AI-assisted image enhancement filters with categories and output options.", primitive_domain: ai, source_ids: [PS-S01] }
  - category: "Generative AI"
    features:
      - { id: "photoshop.ai.generate_image", name: "Generate Image", app_behavior: "Create images from descriptive text prompts and choose supported AI models.", primitive_domain: ai, source_ids: [PS-S05, PS-S07] }
      - { id: "photoshop.ai.generative_fill", name: "Generative Fill", app_behavior: "Use text prompts and selections to add, remove, or alter image content non-destructively.", primitive_domain: ai, source_ids: [PS-S08] }
      - { id: "photoshop.ai.generative_expand", name: "Generative Expand", app_behavior: "Extend canvas/image content beyond original boundaries.", primitive_domain: ai, source_ids: [PS-S01] }
      - { id: "photoshop.ai.reference_images", name: "Reference images", app_behavior: "Guide generative results with one or more reference images where supported.", primitive_domain: ai, source_ids: [PS-S05] }
      - { id: "photoshop.ai.harmonize", name: "Harmonize", app_behavior: "Blend inserted objects/people into a scene by adjusting lighting, color, and shadows.", primitive_domain: ai, source_ids: [PS-S05] }
      - { id: "photoshop.ai.generative_upscale", name: "Generative Upscale", app_behavior: "Sharpen and upscale images to higher resolutions.", primitive_domain: ai, source_ids: [PS-S05] }
      - { id: "photoshop.ai.on_device_remove_model", name: "On-device Remove model", app_behavior: "Choose local or cloud generative processing for Remove tool where available.", primitive_domain: ai, source_ids: [PS-S05] }
  - category: "Camera Raw"
    features:
      - { id: "photoshop.camera_raw.raw_import", name: "Camera Raw import/process", app_behavior: "Open raw/JPEG/TIFF through Camera Raw, preserve original raw data, and save settings as metadata.", primitive_domain: camera_raw, source_ids: [PS-S11] }
      - { id: "photoshop.camera_raw.basic_adjustments", name: "Basic raw adjustments", app_behavior: "Adjust white balance, tone, saturation, sharpening, noise reduction, lens corrections, and retouching.", primitive_domain: camera_raw, source_ids: [PS-S11] }
      - { id: "photoshop.camera_raw.local_masks", name: "Camera Raw masking", app_behavior: "Create local masks, AI subject masks, add/subtract refinements, and adjust masked regions.", primitive_domain: camera_raw, source_ids: [PS-S10] }
      - { id: "photoshop.camera_raw.vectorscope", name: "Vectorscope", app_behavior: "Visualize hue/saturation distribution and skin-tone indication for color grading.", primitive_domain: camera_raw, source_ids: [PS-S12] }
      - { id: "photoshop.camera_raw.bidirectional_gradient", name: "Bidirectional gradient mask", app_behavior: "Create two-sided gradient masks for local adjustments.", primitive_domain: camera_raw, source_ids: [PS-S12] }
  - category: "Automation and extensibility"
    features:
      - { id: "photoshop.automation.actions", name: "Actions panel", app_behavior: "Record, play, preview, categorize, edit, and manage reusable action sequences.", primitive_domain: automation, source_ids: [PS-S01, PS-S05] }
      - { id: "photoshop.automation.batch_droplets", name: "Batch processing and droplets", app_behavior: "Run actions over batches of files or saved droplets.", primitive_domain: automation, source_ids: [PS-S01] }
      - { id: "photoshop.automation.image_processor", name: "Image Processor", app_behavior: "Convert/process groups of files through scripted batch workflows.", primitive_domain: automation, source_ids: [PS-S01] }
      - { id: "photoshop.automation.data_driven_graphics", name: "Data-driven graphics", app_behavior: "Bind template variables to data sets and export multiple document variants.", primitive_domain: automation, source_ids: [PS-S13] }
      - { id: "photoshop.automation.scripting", name: "Legacy scripting", app_behavior: "Automate Photoshop via platform scripting such as COM/VBScript, AppleScript, and external automation.", primitive_domain: automation, source_ids: [PS-S14] }
      - { id: "photoshop.automation.uxp", name: "UXP scripts/plugins/hybrid plugins", app_behavior: "Modern JavaScript-based scripts/plugins and JS+HTML/CSS plus C++ hybrid plugin extensibility.", primitive_domain: automation, source_ids: [PS-S15] }
  - category: "Export, metadata, collaboration"
    features:
      - { id: "photoshop.export.export_as_quick_export", name: "Export As / Quick Export", app_behavior: "Export work with configurable settings, locations, sizes, and quick-export presets.", primitive_domain: export, source_ids: [PS-S01] }
      - { id: "photoshop.export.layers_artboards", name: "Export layers/artboards", app_behavior: "Export layers as files, artboards as files, and artboards as PDF.", primitive_domain: export, source_ids: [PS-S01] }
      - { id: "photoshop.export.video_animation", name: "Video/image-sequence export", app_behavior: "Export video files, animation frames, and image sequences.", primitive_domain: export, source_ids: [PS-S01] }
      - { id: "photoshop.export.content_credentials", name: "Content Credentials", app_behavior: "Enable edit capture and export/preview Content Credentials metadata.", primitive_domain: export, source_ids: [PS-S16] }
      - { id: "photoshop.collaboration.projects", name: "Projects", app_behavior: "Create projects, add files, share, and collaborate.", primitive_domain: collaboration, source_ids: [PS-S01] }
      - { id: "photoshop.collaboration.firefly_boards", name: "Firefly Boards integration", app_behavior: "Send PSD/PSDC/cloud documents to Firefly Boards and return images to Photoshop.", primitive_domain: collaboration, source_ids: [PS-S05] }
```

### [SFR-PHOTOSHOP.implementation-notes] Implementation Notes

```text
Photoshop parity should start with a shared non-destructive layer graph, because layers, masks, adjustment layers, fill layers, smart objects, smart filters, layer effects, export, selection targets, and AI edits all attach to that graph.

Camera Raw should be modeled as a raw-development recipe over immutable source assets, not as destructive raster edits.

Generative AI features should be modeled as provider-backed tools over selections/masks/layers with receipts, prompt provenance, seed/model metadata where available, and a local/provider posture field.
```

### [SFR-PHOTOSHOP.gaps] Gaps

```yaml
gaps:
  - id: PS-GAP-001
    detail: "Feature rows in this category map are broad families; the generated Photoshop leaf index now enumerates official help-topic leaves, but those leaves are not yet implementation command contracts."
    next_step: "Use 06-photoshop-leaf-index.md to promote selected leaves into Studio command schemas with inputs, outputs, undo semantics, state mutations, diagnostics, and tests."
  - id: PS-GAP-002
    detail: "3D/legacy video features are represented only through supported-format/export families."
    next_step: "Decide whether Studio rebuild scope includes legacy Photoshop 3D/video parity or only modern still-image workflows."
```

### [SFR-PHOTOSHOP.sources] Sources

```yaml
sources:
  - { id: PS-S01, url: "https://helpx.adobe.com/photoshop/desktop.html", note: "Photoshop desktop help index/categories." }
  - { id: PS-S02, url: "https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/common-questions-on-photoshop-cloud-documents.html", note: "Cloud documents behavior." }
  - { id: PS-S03, url: "https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/supported-file-formats-in-photoshop.html", note: "Supported formats." }
  - { id: PS-S04, url: "https://helpx.adobe.com/photoshop/desktop/create-masks/layer-masks/add-layer-masks.html", note: "Layer masks." }
  - { id: PS-S05, url: "https://helpx.adobe.com/photoshop/desktop/whats-new/whats-new-in-adobe-photoshop-on-desktop.html", note: "Current feature summaries, updated Jun 18 2026." }
  - { id: PS-S06, url: "https://helpx.adobe.com/uk/photoshop/desktop/repair-retouch/remove-objects-fill-space/remove-unwanted-objects-and-distractions.html", note: "Remove tool." }
  - { id: PS-S07, url: "https://helpx.adobe.com/photoshop/desktop/whats-new/photoshop-on-desktop-release-notes.html", note: "Photoshop release notes." }
  - { id: PS-S08, url: "https://helpx.adobe.com/photoshop/desktop/create-open-import-images/create-images/edit-images-with-generative-fill.html", note: "Generative Fill." }
  - { id: PS-S09, url: "https://helpx.adobe.com/photoshop/desktop/generative-ai/frequently-asked-questions-about-generative-ai-features.html", note: "Generative AI FAQ." }
  - { id: PS-S10, url: "https://helpx.adobe.com/camera-raw/using/masking.html", note: "Camera Raw masking." }
  - { id: PS-S11, url: "https://helpx.adobe.com/camera-raw/using/introduction-camera-raw.html", note: "Camera Raw import/process." }
  - { id: PS-S12, url: "https://helpx.adobe.com/camera-raw/using/whats-new.html", note: "Camera Raw current features." }
  - { id: PS-S13, url: "https://helpx.adobe.com/photoshop/using/creating-data-driven-graphics.html", note: "Data-driven graphics." }
  - { id: PS-S14, url: "https://helpx.adobe.com/photoshop/using/scripting.html", note: "Photoshop scripting." }
  - { id: PS-S15, url: "https://developer.adobe.com/photoshop/", note: "UXP/SDK extensibility." }
  - { id: PS-S16, url: "https://helpx.adobe.com/photoshop/desktop/save-and-export/metadata-content-credentials/use-content-credentials.html", note: "Content Credentials." }
```
