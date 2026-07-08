---
file_id: "online-source-distilled-feature-ledger"
topic_id: SFR-ONLINE-SOURCE-DISTILLED-LEDGER
title: "Online Source Distilled Feature Ledger"
status: draft
summary: "Unified source-distilled ledger for documenting all online-source-observable features and tools for Photoshop, InDesign, Illustrator, Affinity, and Figma before Studio implementation."
sources: 43
updated_at: "2026-07-05"
---

## [SFR-ONLINE-SOURCE-DISTILLED-LEDGER] Online Source Distilled Feature Ledger

### [SFR-ONLINE-SOURCE-DISTILLED-LEDGER.policy] Policy

```yaml
policy:
  correction: "Online sources are sufficient to distill the feature/tool parity inventory for Studio rebuild planning."
  installed_exports_role: "optional enrichment for exact ids, shortcuts, locale/version context, and hidden installed states"
  source_distillation_rule: "Every feature/tool record should merge evidence from help leaves, tool pages, shortcut rows, scripting/API docs, format matrices, release notes, provider docs, and compatibility pages."
  output_rule: "Do not stop at counts. Preserve feature intent, usage, source URL, Studio primitive, command/manual target, compatibility posture, provider posture, and implementation readiness."
  naming_rule: "Vendor names stay in source/provenance/compatibility references. Studio product names remain Handshake-native."
```

### [SFR-ONLINE-SOURCE-DISTILLED-LEDGER.source-surfaces] Source Surfaces

```yaml
source_surfaces:
  - id: "surface.help_leaf"
    meaning: "Official help article or topic leaf."
    corpus_files:
      - "06-photoshop-leaf-index.md"
      - "07-indesign-leaf-index.md"
      - "09-affinity-desktop-delta.md"
      - "22-illustrator-leaf-index.md"
      - "23-figma-leaf-index.md"
  - id: "surface.feature_use_card"
    meaning: "Generated purpose/use/manual planning card."
    corpus_files:
      - "15-photoshop-feature-use-cards.md"
      - "16-affinity-feature-use-cards.md"
      - "17-indesign-feature-use-cards.md"
      - "24-illustrator-feature-use-cards.md"
      - "25-figma-feature-use-cards.md"
  - id: "surface.tool_page"
    meaning: "Named toolbar/tool/category source."
    source_examples:
      - "Illustrator tools and tool-technique pages"
      - "InDesign toolbox pages"
      - "Photoshop toolbar/customize toolbar pages"
      - "Figma toolbar/Draw/vector editing docs"
      - "Affinity tool/category help pages"
  - id: "surface.shortcut_row"
    meaning: "Keyboard shortcut row that exposes commands, tools, toggles, panels, and workflows."
  - id: "surface.scripting_api"
    meaning: "DOM/API class, property, method, enum, action, plugin/widget endpoint, REST endpoint, or scriptable object."
  - id: "surface.file_format_matrix"
    meaning: "Open/place/import/save/export/package/share/local-copy support row and option surface."
  - id: "surface.release_delta"
    meaning: "What is new, release note, beta, retired, changed, or platform-gated capability."
  - id: "surface.provider_or_cloud"
    meaning: "AI, cloud, community, collaboration, stock/template, account, publishing, or provider-backed behavior."
```

### [SFR-ONLINE-SOURCE-DISTILLED-LEDGER.app-ledgers] App Ledgers

```yaml
app_ledgers:
  photoshop:
    source_status: "online_source_distilled_seed_exists"
    existing_card_count: 441
    expanded_ledger: "29-photoshop-expanded-count-ledger.md"
    primary_domains:
      - "document creation, open/save/export, presets, templates, file formats"
      - "workspace, panels, toolbar, tool presets, preferences, shortcuts, history"
      - "layers, groups, Smart Objects, layer comps, blending, opacity, masks, clipping"
      - "selection tools, Select and Mask, object/color/focus/sky/subject selection"
      - "crop, resize, transform, warp, perspective, content-aware scale"
      - "retouch and repair: Remove, Healing, Patch, Clone, Content-Aware Fill"
      - "painting, brushes, pencil, erasers, gradients, fills, patterns"
      - "text, typography, glyphs, styles, text effects"
      - "shapes, paths, pen tools, vector masks, frame/artboard design"
      - "adjustments, filters, neural filters, Camera Raw, liquify, blur/sharpen/distort"
      - "color management, profiles, swatches, LUTs, HDR, bit depth"
      - "video/timeline/animation and frame export"
      - "automation: actions, scripts, droplets, UXP, batchPlay"
      - "AI/provider features: Generative Fill/Expand, Firefly, AI Assistant where applicable"
      - "Camera Raw: edit panels, masking, enhance, profiles, presets, optics, lens blur"
  indesign:
    source_status: "online_source_distilled_seed_exists"
    existing_card_count: 542
    expanded_ledger: "30-indesign-expanded-count-ledger.md"
    primary_domains:
      - "documents, books, templates, libraries, cloud document management"
      - "pages, spreads, master/parent pages, sections, numbering, alternate layouts"
      - "frames, objects, layout, grids, guides, rulers, alignment, gap/page tools"
      - "text frames, stories, threading, overset, import, find/change"
      - "typography, character/paragraph styles, OpenType, glyphs, variable fonts"
      - "tables, data import, cell/table styles, CSV/text workflows"
      - "graphics/media placement, links, captions, object fitting, text wrap"
      - "color, swatches, gradients, transparency, effects, strokes, prepress"
      - "interactive elements, forms, buttons, hyperlinks, bookmarks, media"
      - "TOC, indexes, footnotes/endnotes, cross-references, accessibility"
      - "print, package, preflight, separations, PDF, EPUB, HTML, images"
      - "collaboration, comments, review, InCopy, copyfit/editorial workflows"
      - "automation: scripts, UXP, DOM, Server, menuActions, panels, toolBoxTools"
      - "AI/provider features: text/image generation and cloud review surfaces where documented"
  illustrator:
    source_status: "online_source_distilled_seed_exists"
    existing_card_count: 515
    expanded_ledger: "31-illustrator-expanded-count-ledger.md"
    primary_domains:
      - "documents, artboards, large canvas, templates, files, cloud/project workflows"
      - "selection tools: Selection, Direct Selection, Group Selection, Magic Wand, Lasso, Artboard"
      - "navigation tools: Hand, Rotate View, Zoom"
      - "paint tools: Gradient, Mesh, Shape Builder, Live Paint, swatches, color"
      - "text tools: Type, Type on Path, Vertical Type, glyphs, fonts, text formatting"
      - "draw tools: Pen, anchors, Curvature, line/shape tools, brush/blob/pencil/shaper/slice"
      - "modify tools: Rotate, Reflect, Scale, Shear, Width, Free Transform, Blend, Eraser, Scissors, Dimension"
      - "objects, groups, layers, symbols, appearances, graphic styles"
      - "paths, compound paths, clipping masks, booleans, Pathfinder, expand, simplify"
      - "patterns, gradients, mesh, recolor, image trace, raster/vector interop"
      - "3D/materials/effects, filters, SVG effects, transparency, blend modes"
      - "automation: actions, scripts, variables, data visualization, developer APIs"
      - "file compatibility: AI/AIT/PDF/SVG/EPS/PS/DWG/DXF/PSD/raster/CSS/text/web/screen exports"
      - "AI/provider features: Firefly vector generation, recolor, shape fills, partner models"
  affinity:
    source_status: "online_source_distilled_seed_exists"
    existing_card_count: 1032
    existing_raw_desktop_leaf_rows: 1035
    primary_domains:
      - "Photo: raster editing, RAW develop, layers, masks, selections, adjustments, live filters"
      - "Photo: retouch, inpainting, frequency separation, liquify, tone mapping, HDR, panorama, stacks"
      - "Designer: vector tools, shapes, curves, pen/pencil/brush, booleans, symbols, constraints"
      - "Designer: artboards, export persona, slices, UI/vector asset export, pixel persona"
      - "Publisher: pages/spreads, master pages, text frames, styles, tables, preflight, package, PDF"
      - "Shared: personas, StudioLink, color, typography, grids/guides, snapping, assets, resources"
      - "File compatibility: PSD, PDF, SVG, EPS, AI/PDF-compatible, Affinity native docs, raster formats"
  figma:
    source_status: "online_source_distilled_seed_exists_partial_category_crawl"
    existing_card_count: 200
    primary_domains:
      - "Design files, pages, frames, sections, layers, groups, canvas navigation"
      - "Figma Draw/vector networks, pen/pencil/brush, shapes, shape builder, simplify, vectorize"
      - "text, fonts, typography, text styles, text-to-path"
      - "fills, strokes, images, videos, gradients, effects, blend modes, color profiles"
      - "auto layout, constraints, responsive design, grids"
      - "components, instances, variants, slots, variables, styles, libraries, design systems"
      - "prototypes, interactions, overlays, smart animate, variables, conditionals"
      - "Motion, timeline, keyframes, easing, animated export"
      - "FigJam boards, sticky notes, tables, diagrams, meetings, voting, sessions"
      - "Slides, Sites, Buzz, Make, AI agent, Weave, code layers, shaders where documented"
      - "Dev Mode, inspect, Code Connect, MCP, REST API, plugins, widgets, webhooks"
      - "collaboration: comments, multiplayer, branches, history, sharing, meetings"
      - "import/export/local copies: FIG/JAM/DECK/BUZZ/SITE/MAKE, Sketch, SVG, PDF, PNG, JPG, GIF, video, CSV, PPTX"
```

### [SFR-ONLINE-SOURCE-DISTILLED-LEDGER.merge-record] Canonical Merge Record

```yaml
canonical_source_distilled_feature_record:
  source_distilled_feature_id: "stable id generated from app, surface, source slug, and normalized feature name"
  source_ids: []
  source_apps: []
  source_surfaces: []
  feature_name: ""
  feature_kind: "tool|command|panel|workflow|api|format|provider|compatibility|release_delta"
  app_behavior: ""
  user_goal: ""
  source_urls: []
  source_files: []
  source_confidence: "online_source_direct|online_source_inferred|cross_surface_merged"
  studio_surface: ""
  primitive_domain: ""
  provider_posture: "local_primitive|provider_adapter|optional_integration|compatibility_shim"
  file_format_compatibility: "not_applicable|import|export|round_trip|fixture_required"
  manual_topic_candidate: ""
  implementation_readiness: "needs_command_contract_promotion"
```

### [SFR-ONLINE-SOURCE-DISTILLED-LEDGER.next] Next Distillation Pass

```yaml
next_distillation_pass:
  id: "online-source-full-feature-ledger-v1"
  action: "Generated one YAML per-feature row ledger per app family from current Feature Use Cards and source-distilled domain ledgers."
  domain_ledger_status: "created in 34-photoshop-source-distilled-domain-ledger.md through 38-figma-source-distilled-domain-ledger.md"
  feature_row_ledger_status: "created in 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md"
  overlap_dedupe_status: "created in 44-cross-app-overlap-and-affinity-dedupe-map.md"
  tool_registry_status: "created in 45-source-distilled-tool-registry.md"
  format_compatibility_status: "created in 46-file-format-compatibility-registry.md"
  implementation_backlog_status: "created in 47-studio-rust-implementation-backlog.md"
  provider_offline_parity_status: "created in 48-provider-offline-parity-registry.md"
  source_coverage_verification_status: "created in 49-source-coverage-verification-matrix.md"
  proprietary_format_fixture_plan_status: "created in 50-proprietary-format-fixture-plan.md"
  generator: "_tools/generate-source-distilled-feature-rows.py"
  outputs:
    - "39-photoshop-source-distilled-feature-rows.md"
    - "40-indesign-source-distilled-feature-rows.md"
    - "41-illustrator-source-distilled-feature-rows.md"
    - "42-affinity-source-distilled-feature-rows.md"
    - "43-figma-source-distilled-feature-rows.md"
    - "44-cross-app-overlap-and-affinity-dedupe-map.md"
    - "45-source-distilled-tool-registry.md"
    - "46-file-format-compatibility-registry.md"
    - "47-studio-rust-implementation-backlog.md"
    - "48-provider-offline-parity-registry.md"
    - "49-source-coverage-verification-matrix.md"
    - "50-proprietary-format-fixture-plan.md"
    - "_tools/generate-source-distilled-feature-rows.py"
    - "_tools/generate-cross-app-dedupe-map.py"
    - "_tools/generate-source-distilled-tool-registry.py"
    - "_tools/generate-file-format-compatibility-registry.py"
    - "_tools/generate-studio-rust-implementation-backlog.py"
    - "_tools/generate-provider-offline-parity-registry.py"
    - "_tools/generate-source-coverage-verification-matrix.py"
    - "_tools/generate-proprietary-format-fixture-plan.py"
  validation:
    - "Domain ledgers must not stop at help leaves; they must preserve tool, shortcut, API, file-format, release, provider, and compatibility source surfaces where available."
    - "Generated feature-row ledgers are card-derived row seeds plus domain-ledger context; promote rows through exact source-page and source-surface inspection before implementation."
    - "Every feature/tool row has at least one source URL or local source snapshot."
    - "Every generated feature row has source_ids and source_refs."
    - "Rows preserve purpose/use/manual handoff fields."
    - "Provider/cloud features are retained and marked, not discarded."
    - "File format compatibility rows include fixture requirements."
    - "Provider/cloud/AI/collaboration rows have local-first parity, optional-adapter, fallback, receipt, and verification posture."
    - "Coverage claims must be checked against the source coverage verification matrix before being treated as complete."
    - "Native/proprietary/local-copy format compatibility claims must be checked against the fixture plan before being treated as complete."
```

### [SFR-ONLINE-SOURCE-DISTILLED-LEDGER.sources] Sources

```yaml
sources:
  - { id: OSD-S01, path: "15-photoshop-feature-use-cards.md", note: "Photoshop generated use cards." }
  - { id: OSD-S02, path: "16-affinity-feature-use-cards.md", note: "Affinity generated use cards." }
  - { id: OSD-S03, path: "17-indesign-feature-use-cards.md", note: "InDesign generated use cards." }
  - { id: OSD-S04, path: "24-illustrator-feature-use-cards.md", note: "Illustrator generated use cards." }
  - { id: OSD-S05, path: "25-figma-feature-use-cards.md", note: "Figma generated use cards." }
  - { id: OSD-S06, path: "28-adobe-count-methodology.md", note: "Corrected online-source distillation method." }
  - { id: OSD-S07, path: "29-photoshop-expanded-count-ledger.md", note: "Photoshop expanded source surfaces." }
  - { id: OSD-S08, path: "30-indesign-expanded-count-ledger.md", note: "InDesign expanded source surfaces." }
  - { id: OSD-S09, path: "31-illustrator-expanded-count-ledger.md", note: "Illustrator expanded source surfaces." }
  - { id: OSD-S10, path: "20-illustrator-feature-map.md", note: "Illustrator feature families." }
  - { id: OSD-S11, path: "21-figma-feature-map.md", note: "Figma feature families." }
  - { id: OSD-S12, path: "02-affinity-suite-feature-map.md", note: "Affinity feature families." }
  - { id: OSD-S13, url: "https://helpx.adobe.com/photoshop/desktop.html", note: "Photoshop online source." }
  - { id: OSD-S14, url: "https://helpx.adobe.com/indesign/desktop.html", note: "InDesign online source." }
  - { id: OSD-S15, url: "https://helpx.adobe.com/illustrator/desktop.html", note: "Illustrator online source." }
  - { id: OSD-S16, url: "https://affinity.help/", note: "Affinity online source." }
  - { id: OSD-S17, url: "https://help.figma.com/hc/en-us", note: "Figma online source." }
  - { id: OSD-S18, url: "https://developers.figma.com/", note: "Figma developer online source." }
  - { id: OSD-S19, path: "34-photoshop-source-distilled-domain-ledger.md", note: "Photoshop online-source-distilled domain ledger." }
  - { id: OSD-S20, path: "35-indesign-source-distilled-domain-ledger.md", note: "InDesign online-source-distilled domain ledger." }
  - { id: OSD-S21, path: "36-illustrator-source-distilled-domain-ledger.md", note: "Illustrator online-source-distilled domain ledger." }
  - { id: OSD-S22, path: "37-affinity-source-distilled-domain-ledger.md", note: "Affinity online-source-distilled domain ledger." }
  - { id: OSD-S23, path: "38-figma-source-distilled-domain-ledger.md", note: "Figma online-source-distilled domain ledger." }
  - { id: OSD-S24, path: "39-photoshop-source-distilled-feature-rows.md", note: "Photoshop source-distilled feature rows." }
  - { id: OSD-S25, path: "40-indesign-source-distilled-feature-rows.md", note: "InDesign source-distilled feature rows." }
  - { id: OSD-S26, path: "41-illustrator-source-distilled-feature-rows.md", note: "Illustrator source-distilled feature rows." }
  - { id: OSD-S27, path: "42-affinity-source-distilled-feature-rows.md", note: "Affinity source-distilled feature rows." }
  - { id: OSD-S28, path: "43-figma-source-distilled-feature-rows.md", note: "Figma source-distilled feature rows." }
  - { id: OSD-S29, path: "_tools/generate-source-distilled-feature-rows.py", note: "Feature row generator." }
  - { id: OSD-S30, path: "44-cross-app-overlap-and-affinity-dedupe-map.md", note: "Cross-app overlap and Affinity dedupe map." }
  - { id: OSD-S31, path: "_tools/generate-cross-app-dedupe-map.py", note: "Overlap and Affinity dedupe generator." }
  - { id: OSD-S32, path: "45-source-distilled-tool-registry.md", note: "Source-distilled cross-app tool registry." }
  - { id: OSD-S33, path: "_tools/generate-source-distilled-tool-registry.py", note: "Tool registry generator." }
  - { id: OSD-S34, path: "46-file-format-compatibility-registry.md", note: "Source-distilled file-format compatibility registry." }
  - { id: OSD-S35, path: "_tools/generate-file-format-compatibility-registry.py", note: "File-format compatibility registry generator." }
  - { id: OSD-S36, path: "47-studio-rust-implementation-backlog.md", note: "Source-distilled Studio Rust implementation backlog." }
  - { id: OSD-S37, path: "_tools/generate-studio-rust-implementation-backlog.py", note: "Implementation backlog generator." }
  - { id: OSD-S38, path: "48-provider-offline-parity-registry.md", note: "Provider/offline parity registry." }
  - { id: OSD-S39, path: "_tools/generate-provider-offline-parity-registry.py", note: "Provider/offline parity registry generator." }
  - { id: OSD-S40, path: "49-source-coverage-verification-matrix.md", note: "Source coverage verification matrix." }
  - { id: OSD-S41, path: "_tools/generate-source-coverage-verification-matrix.py", note: "Source coverage verification matrix generator." }
  - { id: OSD-S42, path: "50-proprietary-format-fixture-plan.md", note: "Proprietary/native format fixture plan." }
  - { id: OSD-S43, path: "_tools/generate-proprietary-format-fixture-plan.py", note: "Proprietary/native format fixture plan generator." }
```
