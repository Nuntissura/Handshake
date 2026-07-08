---
file_id: "studio-local-first-rust-parity-posture"
topic_id: SFR-STUDIO-LOCAL-FIRST-RUST
status: draft
summary: "Local-first, no-cloud, Rust-forward posture for expanding Studio into Photoshop, Affinity, InDesign, Illustrator, and Figma parity without vendor product naming."
sources: 6
updated_at: "2026-07-05"
---

## [SFR-STUDIO-LOCAL-FIRST-RUST] Studio Local-First Rust Parity Posture

### [SFR-STUDIO-LOCAL-FIRST-RUST.policy] Policy

```yaml
studio_identity:
  module: "Studio"
  product_home: "Handshake"
  built_in: true
  local_first: true
  no_cloud_required: true
  rust_forward: true
  vendor_names_in_product_surface: false
  vendor_names_allowed_for: [source_refs, compatibility_notes, fixtures, migration_docs]
core_rules:
  - "Studio is a built-in local creative module for Handshake, not an external cloud clone."
  - "Core creative behavior must run locally in Rust-native engines wherever technically practical."
  - "Cloud, account, credit, community, and AI-provider behaviors are optional adapters or local-model/provider-neutral abstractions, never core requirements."
  - "File compatibility must target existing creative formats; do not invent a replacement interchange format for this rebuild scope."
  - "Every promoted feature needs a typed command contract, fixtures, receipts, diagnostics, undo/replay, and an internal Studio UserManual topic."
```

### [SFR-STUDIO-LOCAL-FIRST-RUST.engine-map] Rust-Forward Engine Map

```yaml
engine_targets:
  - { engine_module: studio_vector, owns: [illustrator_paths, figma_vector_networks, draw_tools, shape_builder, boolean_geometry, svg_pdf_vector_io] }
  - { engine_module: studio_layout, owns: [figma_frames, auto_layout, artboards, boards, slides, sites, responsive_constraints, page_spreads] }
  - { engine_module: studio_layer_graph, owns: [layers, groups, masks, placed_assets, components, object_order, visibility_locking] }
  - { engine_module: studio_typography, owns: [text_runs, font_resolution, glyphs, type_on_path, text_styles, accessibility_text] }
  - { engine_module: studio_style_registry, owns: [styles, variables, tokens, components, variants, libraries, symbols] }
  - { engine_module: studio_import_export, owns: [ai_ait, pdf, svg_svgz, eps_ps, psd, dwg_dxf, fig_jam_sketch, png_jpg_webp_tiff_gif, pptx, csv] }
  - { engine_module: studio_interaction, owns: [prototype_flows, smart_animate, overlays, motion_timeline, keyframes, slide_presentations] }
  - { engine_module: studio_collaboration, owns: [local_crdt, comments, branches, merge, history, meetings, cursor_chat, voting, attribution] }
  - { engine_module: studio_model_tools, owns: [provider_neutral_ai, local_model_tools, prompt_receipts, generation_provenance, optional_provider_adapters] }
  - { engine_module: studio_extensibility, owns: [plugins, widgets, mcp_server, rest_facade, scripting, local_package_registry] }
```

### [SFR-STUDIO-LOCAL-FIRST-RUST.compatibility] Compatibility Posture

```yaml
compatibility_targets:
  illustrator: [ai, ait, pdf, svg, svgz, eps, ps, dwg, dxf, psd, png, jpg, jpeg, gif, tiff, webp, css, txt]
  figma: [fig, jam, deck, buzz, site, make, sketch, png, jpg, svg, pdf_1_7, gif, tiff, webp, mp4, mov, webm, pptx, csv]
compatibility_rules:
  - "Prefer faithful import/export with explicit unsupported-feature diagnostics over silent lossy conversion."
  - "Round-trip tests must declare what is preserved, transformed, rasterized, ignored, or represented by compatibility shims."
  - "Proprietary local-copy formats with undocumented schemas are compatibility targets, not internal Studio storage authority."
```

### [SFR-STUDIO-LOCAL-FIRST-RUST.sources] Sources

```yaml
sources:
  - { id: LFR-S01, path: "00-preamble.md", note: "Existing Studio app feature research preamble and naming/file compatibility policy." }
  - { id: LFR-S02, path: "05-studio-primitive-map.md", note: "Existing Studio primitive and Rust module map." }
  - { id: LFR-S03, url: "https://helpx.adobe.com/illustrator/desktop.html", note: "Official Illustrator desktop help." }
  - { id: LFR-S04, url: "https://helpx.adobe.com/illustrator/kb/supported-file-formats-illustrator.html", note: "Official Illustrator supported file formats." }
  - { id: LFR-S05, url: "https://help.figma.com/hc/en-us", note: "Official Figma Help Center." }
  - { id: LFR-S06, url: "https://developers.figma.com/", note: "Official Figma developer documentation." }
```
