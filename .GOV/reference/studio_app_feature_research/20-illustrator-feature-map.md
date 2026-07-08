---
file_id: "20-illustrator-feature-map"
topic_id: SFR-ILLUSTRATOR
title: "Illustrator Feature Map"
status: draft
summary: "Illustrator vector, typography, object, color, AI, import/export, print, and automation feature families for Handshake-native Studio parity."
sources: 4
updated_at: "2026-07-05"
---

## [SFR-ILLUSTRATOR] Illustrator Feature Map

### [SFR-ILLUSTRATOR.inventory] Feature Families

```yaml
feature_families:
  - id: "illustrator.vector_paths"
    name: "Vector paths and drawing tools"
    app_behavior: "Pen, pencil, curvature, anchor point, smooth, erase, cut, simplify, live shapes, and path editing."
    primitive_domain: "vector"
    studio_surface: "StudioVectorPathGraph"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.live_shapes"
    name: "Live shapes and shape construction"
    app_behavior: "Lines, arcs, stars, spirals, polygons, pie shapes, shape builder, shaper, combine shapes."
    primitive_domain: "vector"
    studio_surface: "StudioVectorPathGraph"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.object_arrangement"
    name: "Object selection and arrangement"
    app_behavior: "Selection methods, magic wand, grouping, isolation, move, align, distribute, expand, stack order, transforms."
    primitive_domain: "vector"
    studio_surface: "StudioVectorPathGraph"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.artboards_canvas"
    name: "Artboards, canvas, workspace, and UI"
    app_behavior: "Large canvas, artboards, workspaces, properties/control/context panels, toolbars, preferences, shortcuts."
    primitive_domain: "page_layout"
    studio_surface: "StudioPageSpread"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.color_appearance"
    name: "Color, fills, strokes, gradients, mesh, patterns, and appearance"
    app_behavior: "Fill/stroke models, swatches, gradients, mesh, recolor, blend modes, appearances, graphic styles."
    primitive_domain: "color"
    studio_surface: "StudioColorPipeline"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.typography"
    name: "Typography and glyph-aware editing"
    app_behavior: "Text objects, type on path, fonts, glyph snapping/guides, proofreading/translation/rewrite where AI-backed."
    primitive_domain: "typography"
    studio_surface: "StudioTextRunAndStory"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.layers_symbols_assets"
    name: "Layers, symbols, links, embedded assets"
    app_behavior: "Layer organization, symbols, linked/embedded files, relink all instances, placed files."
    primitive_domain: "layer"
    studio_surface: "StudioGeneralToolSurface"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.import_export_formats"
    name: "Import/export/save/place formats"
    app_behavior: "AI/AIT, PDF, SVG/SVGZ, EPS/PS, DWG/DXF, PSD, raster formats, CSS, save for web/screens."
    primitive_domain: "file_io"
    studio_surface: "StudioFileIO"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.generative_ai"
    name: "Generative and AI-assisted vector workflows"
    app_behavior: "Text to vector graphic, generative recolor/patterns/shape fills, vectorize raster, edit generated artwork, partner models."
    primitive_domain: "ai"
    studio_surface: "StudioModelToolContract"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.print_prepress"
    name: "Print, PDF, package, and prepress output"
    app_behavior: "PDF output, separations, color management, linked asset/package concerns, print-ready vector output."
    primitive_domain: "prepress"
    studio_surface: "StudioPreflightProfile"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.automation_extensions"
    name: "Actions, scripts, variables, plugins, and extensibility"
    app_behavior: "Automation and extension surfaces needed for parity with production Illustrator workflows."
    primitive_domain: "automation"
    studio_surface: "StudioActionGraph"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "illustrator.recovery_diagnostics"
    name: "Recovery, performance, troubleshooting, and damaged files"
    app_behavior: "Crash recovery, safe mode, damaged documents, missing plugins/fonts/printers, performance diagnostics."
    primitive_domain: "workspace"
    studio_surface: "StudioWorkspaceSurface"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
```

### [SFR-ILLUSTRATOR.implementation-notes] Implementation Notes

```yaml
implementation_notes:
  local_first: "Studio is built-in, local-first, no-cloud-required, and Rust-forward."
  parity_rule: "Vendor names define source behavior and compatibility only; Studio product surfaces use Handshake-native names."
  file_format_rule: "Do not invent a replacement interchange format; implement compatibility adapters, fixtures, diagnostics, and explicit unsupported-feature receipts."
```

### [SFR-ILLUSTRATOR.sources] Sources

```yaml
sources:
  - { id: ILL-S01, url: "https://helpx.adobe.com/illustrator/desktop.html", note: "Official Illustrator desktop help." }
  - { id: ILL-S02, url: "https://helpx.adobe.com/illustrator/using/tools-in-illustrator.html", note: "Official Illustrator tools overview." }
  - { id: ILL-S03, url: "https://helpx.adobe.com/illustrator/kb/supported-file-formats-illustrator.html", note: "Official Illustrator supported file formats." }
  - { id: ILL-S04, url: "https://helpx.adobe.com/illustrator/desktop/new-features/release-notes.html", note: "Official Illustrator release notes." }
```
