---
file_id: "21-figma-feature-map"
topic_id: SFR-FIGMA
title: "Figma Feature Map"
status: draft
summary: "Figma Design, Draw, FigJam, Motion, Slides, Sites, Buzz, Make, Dev Mode, API, AI, and collaboration feature families for local-first Rust Studio parity."
sources: 6
updated_at: "2026-07-05"
---

## [SFR-FIGMA] Figma Feature Map

### [SFR-FIGMA.inventory] Feature Families

```yaml
feature_families:
  - id: "figma.canvas_layers"
    name: "Canvas, files, pages, layers, frames, groups, sections"
    app_behavior: "Core editable design graph and navigation model."
    primitive_domain: "page_layout"
    studio_surface: "StudioPageSpread"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.vector_draw"
    name: "Vector networks, pen, pencil, brush, shape builder, simplify, vectorize"
    app_behavior: "Illustration and vector authoring parity including Figma Draw."
    primitive_domain: "vector"
    studio_surface: "StudioVectorPathGraph"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.typography"
    name: "Text, fonts, text properties, text styles"
    app_behavior: "Font loading, typography, text styles, text-to-path conversion."
    primitive_domain: "typography"
    studio_surface: "StudioTextRunAndStory"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.visual_styling"
    name: "Fills, gradients, patterns, images, effects, blend modes, color profiles"
    app_behavior: "Visual style stack and color pipeline."
    primitive_domain: "color"
    studio_surface: "StudioColorPipeline"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.auto_layout"
    name: "Auto layout, constraints, responsive sizing, grids"
    app_behavior: "Responsive layout engine and constraints."
    primitive_domain: "page_layout"
    studio_surface: "StudioPageSpread"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.components_systems"
    name: "Components, instances, variants, slots, styles, variables, libraries"
    app_behavior: "Design-system graph and reusable token/component registry."
    primitive_domain: "style_system"
    studio_surface: "StudioStyleRegistry"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.prototyping_motion"
    name: "Prototypes, interactions, smart animate, variables in prototypes, Motion timeline"
    app_behavior: "Runtime interaction/timeline model."
    primitive_domain: "interactive"
    studio_surface: "StudioInteractiveDocumentSurface"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.import_export_formats"
    name: "Import/export, local copies, Sketch import, .fig, SVG/PDF/PNG/JPG/video/animation export"
    app_behavior: "Compatibility and asset IO surface."
    primitive_domain: "file_io"
    studio_surface: "StudioFileIO"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.collaboration"
    name: "Comments, multiplayer, branches, history, sharing, meetings, FigJam sessions"
    app_behavior: "Local-first CRDT collaboration replacement for cloud collaboration."
    primitive_domain: "collaboration"
    studio_surface: "StudioCollaborationSession"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.figjam"
    name: "FigJam whiteboard, sticky notes, tables, mind maps, meetings, imports/exports"
    app_behavior: "Whiteboard and workshop parity."
    primitive_domain: "page_layout"
    studio_surface: "StudioPageSpread"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.dev_mode_api"
    name: "Dev Mode, inspect, Code Connect, MCP, REST, plugin/widget APIs"
    app_behavior: "Developer handoff and extension surfaces."
    primitive_domain: "automation"
    studio_surface: "StudioActionGraph"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.make_ai"
    name: "Make, AI agent, Weave, generative plugins, web/code workflows"
    app_behavior: "Provider/local model adapter and local code-generation sandbox."
    primitive_domain: "ai"
    studio_surface: "StudioModelToolContract"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
  - id: "figma.slides_sites_buzz"
    name: "Slides, Sites, Buzz and adjacent canvas products"
    app_behavior: "Presentation, responsive site, and brand asset production surfaces."
    primitive_domain: "interactive"
    studio_surface: "StudioInteractiveDocumentSurface"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    local_first_posture: "local_rust_core_unless_marked_provider_or_compatibility"
```

### [SFR-FIGMA.implementation-notes] Implementation Notes

```yaml
implementation_notes:
  local_first: "Studio is built-in, local-first, no-cloud-required, and Rust-forward."
  parity_rule: "Vendor names define source behavior and compatibility only; Studio product surfaces use Handshake-native names."
  file_format_rule: "Do not invent a replacement interchange format; implement compatibility adapters, fixtures, diagnostics, and explicit unsupported-feature receipts."
```

### [SFR-FIGMA.sources] Sources

```yaml
sources:
  - { id: FIG-S01, url: "https://help.figma.com/hc/en-us/categories/360002042553-Figma-Design", note: "Official Figma Design category." }
  - { id: FIG-S02, url: "https://help.figma.com/hc/en-us/categories/360002051633-FigJam", note: "Official FigJam category." }
  - { id: FIG-S03, url: "https://help.figma.com/hc/en-us/categories/31304285531543-Figma-Make", note: "Official Figma Make category." }
  - { id: FIG-S04, url: "https://help.figma.com/hc/en-us/categories/41274596092695-Figma-Motion", note: "Official Figma Motion category." }
  - { id: FIG-S05, url: "https://developers.figma.com/", note: "Official Figma developer docs." }
  - { id: FIG-S06, url: "https://www.figma.com/release-notes/", note: "Official Figma release notes." }
```
