---
file_id: "indesign-source-distilled-domain-ledger"
topic_id: SFR-INDESIGN-SOURCE-DISTILLED-DOMAINS
title: "InDesign Source Distilled Domain Ledger"
status: draft
summary: "Online-source-distilled InDesign feature/tool domains for Studio parity planning."
sources: 12
updated_at: "2026-07-05"
---

## [SFR-INDESIGN-SOURCE-DISTILLED-DOMAINS] InDesign Source Distilled Domain Ledger

### [SFR-INDESIGN-SOURCE-DISTILLED-DOMAINS.policy] Policy

```yaml
policy:
  distillation_status: "online_source_distilled"
  installed_exports_role: "optional enrichment only"
  rebuild_target: "Handshake Studio local-first Rust page-layout, typography, publishing, prepress, and automation tools with InDesign-compatible import/export behavior where source-observable"
  naming_rule: "InDesign remains source/provenance naming only; Studio surfaces use Handshake-native names."
  coverage_rule: "Merge help leaves, toolbox docs, shortcut docs, scripting/UXP DOM docs, supported-format docs, export/print/preflight docs, and collaboration/review docs."
```

### [SFR-INDESIGN-SOURCE-DISTILLED-DOMAINS.domains] Domains

```yaml
domains:
  - id: idd.domain.workspace_and_toolbox
    name: "Workspace, toolbox, panels, navigation, and preferences"
    app_behavior: "InDesign exposes a document workspace with configurable panels, toolboxes, control/context bars, shortcuts, menus, rulers, guides, grids, zoom/pan, screen modes, presentation preview, and workspace persistence."
    tool_and_feature_scope:
      - "Selection, Direct Selection, Page, Gap, Type, Type on a Path, Line, Pen, Pencil, shape/frame tools, Scissors, Free Transform, Rotate, Scale, Shear, Gradient, Gradient Feather, Eyedropper, Measure, Hand, Zoom, Note, and Content tools."
      - "Panels for pages, layers, links, swatches, styles, text wrap, align, effects, preflight, hyperlinks, bookmarks, index, table of contents, scripts, and review/comment workflows."
      - "Preferences, shortcuts, menus, workspaces, rulers, guides, baseline/document grids, snapping, and measurement units."
    studio_primitive_domains: [workspace, layout, viewport, command_palette, diagnostics]
    source_surfaces: [help_leaf, tool_page, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.workspace.layout-tools"
    implementation_notes:
      - "The layout workspace should expose structured page/object state for parallel model work, not rely on visible screen state."
      - "Toolbox rows should map to a local tool registry shared with vector and publishing modules."
  - id: idd.domain.documents_pages_layout
    name: "Documents, pages, spreads, parent pages, books, and layout systems"
    app_behavior: "InDesign creates publication documents with pages, spreads, parent pages, sections, page numbering, alternate layouts, liquid layout, margins, columns, grids, layers, books, and layout adjustment."
    tool_and_feature_scope:
      - "New document, templates, page size, margins, columns, facing pages, bleed/slug, pages/spreads, parent pages, sections, numbering, shuffle, page tool, alternate layout, liquid layout, layout adjustment, and books."
      - "Layers, object stacking, spread geometry, page transitions where applicable, and long-document organization."
    studio_primitive_domains: [page_layout, master_pages, layout, layer, file_io]
    source_surfaces: [help_leaf, shortcut_row, scripting_api, file_format_matrix]
    manual_topic_candidate: "studio.manual.page-layout.document-structure"
    implementation_notes:
      - "Parent-page inheritance should be explicit graph state with override receipts."
      - "Book workflows require multi-document indexes, shared styles, cross-document numbering, and package/export coordination."
  - id: idd.domain.text_typography_proofing
    name: "Text frames, stories, typography, proofing, and composition"
    app_behavior: "InDesign manages text through stories, linked frames, overset detection, text import, threading, composition engines, glyphs, OpenType, paragraph/character formatting, spellcheck, dictionaries, find/change, and copyfitting."
    tool_and_feature_scope:
      - "Type tool, text frames, threaded text, story editor, overset indicators, text variables, bullets/numbering, tabs, indents, drop caps, keep options, composer settings, hyphenation, justification, glyphs, and OpenType features."
      - "Find/Change text, GREP, glyph, object, color, and query workflows; spelling, autocorrect, dictionaries, notes, tracked editorial changes where documented."
      - "CJK, right-to-left, vertical type, ruby, kinsoku, mojikumi, and regional typography surfaces where source-observable."
    studio_primitive_domains: [typography, text_engine, layout, proofing, search_replace]
    source_surfaces: [help_leaf, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.typography.publication-text"
    implementation_notes:
      - "Text composition must be deterministic and fixture-tested across locale, OpenType, overset, and reflow scenarios."
      - "Find/Change should be command-contract driven with dry-run and receipt outputs."
  - id: idd.domain.styles_tables_references
    name: "Styles, tables, references, indexes, and long-document structure"
    app_behavior: "InDesign provides reusable formatting and long-document structures through paragraph, character, object, table, and cell styles; tables; footnotes; endnotes; cross-references; table of contents; indexes; captions; and variables."
    tool_and_feature_scope:
      - "Style creation, nested styles, GREP styles, style mapping/import, style overrides, based-on relationships, quick apply, and style packaging."
      - "Tables, cells, rows, columns, strokes/fills, table and cell styles, table import, footnotes/endnotes, cross-references, TOC generation, index topics/references, captions, and text variables."
    studio_primitive_domains: [style_system, tables, references, typography, page_layout]
    source_surfaces: [help_leaf, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.styles.layout-publishing"
    implementation_notes:
      - "Styles need stable IDs, dependency graphs, override tracking, and safe update previews."
      - "Generated references must be reproducible commands with stale-reference diagnostics."
  - id: idd.domain.graphics_objects_color
    name: "Graphics, objects, links, color, transparency, and effects"
    app_behavior: "InDesign places and manages images, vectors, frames, objects, links, captions, object fitting, clipping, transformations, text wrap, swatches, gradients, tints, transparency, effects, strokes, fills, and output color."
    tool_and_feature_scope:
      - "Place/import, links, relink, embed, update, missing link handling, captions, object fitting, frame fitting options, clipping paths, alpha channels, text wrap, anchored objects, align/distribute, transform, and object styles."
      - "Swatches, color groups, gradients, mixed inks where applicable, overprint, transparency, effects, blend modes, feathering, drop shadow, and separations-aware color."
    studio_primitive_domains: [asset_pipeline, layout, vector, color, prepress]
    source_surfaces: [help_leaf, shortcut_row, file_format_matrix, scripting_api]
    manual_topic_candidate: "studio.manual.layout.graphics-and-color"
    implementation_notes:
      - "Linked assets need local-first resolution, missing-link blockers, last-known hashes, and package receipts."
      - "Color and transparency must share prepress-compatible rendering with PDF/export engines."
  - id: idd.domain.generative_ai
    name: "Generative AI and provider-backed layout assets"
    app_behavior: "InDesign source surfaces include provider-backed generation and assistance where documented, such as text/image generation, generated image placement, prompt-driven creative assets, review assistance, and cloud-account gated features."
    tool_and_feature_scope:
      - "Prompt-based generation, generated images/assets, variation review, placed generated content, provider errors, and account/cloud constraints."
      - "Local-first fallback records for provider-backed behavior and deterministic placeholder workflows."
    studio_primitive_domains: [ai, provider_adapter, asset_pipeline, layout, metadata]
    source_surfaces: [help_leaf, provider_or_cloud, release_delta]
    manual_topic_candidate: "studio.manual.ai.layout-generation"
    implementation_notes:
      - "AI behavior must be optional adapter behavior in Studio, with local publication tooling unaffected when offline."
      - "Generated assets need provenance metadata and export/package handling."
  - id: idd.domain.interactive_and_accessible_outputs
    name: "Interactive documents, forms, hyperlinks, media, EPUB, and accessibility"
    app_behavior: "InDesign builds interactive and accessible outputs through buttons, forms, hyperlinks, bookmarks, media, page transitions, animations where documented, tagged PDF, alt text, articles, reading order, EPUB, and accessibility checks."
    tool_and_feature_scope:
      - "Hyperlinks, cross-document links, buttons, forms, bookmarks, media placement, interactive PDF export, fixed-layout/reflowable EPUB export, HTML-related output where documented."
      - "Alt text, tags, articles panel, reading order, accessibility metadata, PDF accessibility preparation, and export validation."
    studio_primitive_domains: [interactive, accessibility, pdf, epub, export]
    source_surfaces: [help_leaf, file_format_matrix, scripting_api]
    manual_topic_candidate: "studio.manual.export.accessible-interactive-publications"
    implementation_notes:
      - "Interactive objects should compile into export-target-specific capability maps, because PDF and EPUB support differ."
      - "Accessibility should be validated through structured preflight checks and not only visual review."
  - id: idd.domain.collaboration_cloud_review
    name: "Collaboration, review, comments, InCopy, libraries, and cloud workflows"
    app_behavior: "InDesign supports editorial and review workflows through Share for Review, comments, cloud documents, Creative Cloud Libraries, InCopy assignment workflows, notes, package sharing, and version/recovery surfaces."
    tool_and_feature_scope:
      - "Review links, comments, resolve/comment states, cloud document open/save, library assets, InCopy stories/assignments, editorial notes, managed content, and package handoff."
      - "Local-first replacement surfaces for review packages, comments, attributable changes, and recoverable collaboration state."
    studio_primitive_domains: [collaboration, review, asset_pipeline, provider_adapter, versioning]
    source_surfaces: [help_leaf, provider_or_cloud, release_delta]
    manual_topic_candidate: "studio.manual.collaboration.publication-review"
    implementation_notes:
      - "Studio should separate local review packages from optional provider sync."
      - "Parallel model collaboration needs attributable edit logs and conflict-safe document operations."
  - id: idd.domain.import_export_publish_print
    name: "Import, export, package, print, PDF, preflight, and publishing"
    app_behavior: "InDesign imports content and publishes output through file placing, supported formats, package, preflight, print, separations, PDF presets, EPUB, images, HTML-related outputs, Publish Online/provider-backed publishing, and output diagnostics."
    tool_and_feature_scope:
      - "Place/import text, Word, Excel, XML, images, vector, PDF, and media where supported; package fonts/links; preflight profiles; print booklet; separations/ink manager; PDF/X and print presets."
      - "Export PDF, EPUB, images, HTML, IDML, snippets, templates, package reports, and publish/provider outputs."
    studio_primitive_domains: [file_io, export, pdf, epub, print, prepress, packaging]
    source_surfaces: [help_leaf, file_format_matrix, shortcut_row, scripting_api]
    manual_topic_candidate: "studio.manual.prepress.layout-export"
    implementation_notes:
      - "Every import/export path needs fixtures and receipts for unsupported content, font/link substitution, and color conversion."
      - "Preflight should be a machine-readable validation surface for models and humans."
  - id: idd.domain.automation_extensibility_server
    name: "Scripts, UXP, DOM, Server, menu actions, and extensibility"
    app_behavior: "InDesign exposes automation through ExtendScript/scripting DOM, UXP where documented, menu actions, panels, script labels, events, object model classes, InDesign Server automation, and batch publishing."
    tool_and_feature_scope:
      - "Application/document/page/spread/story/text/object/style/table/link/export/preflight DOM objects, menuActions, scriptUI/UXP surfaces, scripts panel, startup scripts, events, and labels."
      - "Server-side rendering/export workflows, headless or unattended document processing, and scriptable batch operations."
    studio_primitive_domains: [automation, scripting, plugin_api, command_contracts, batch]
    source_surfaces: [scripting_api, uxp_api, help_leaf, shortcut_row]
    manual_topic_candidate: "studio.manual.automation.layout-publishing"
    implementation_notes:
      - "The Studio command layer should be the automation API first, with GUI tools calling the same commands."
      - "Server-style batch workflows are high ROI for local-first publication automation."
```

### [SFR-INDESIGN-SOURCE-DISTILLED-DOMAINS.sources] Sources

```yaml
sources:
  - { id: IDD-S01, url: "https://helpx.adobe.com/indesign/desktop.html", note: "Official InDesign desktop help source." }
  - { id: IDD-S02, path: "_source_snapshots/adobe-indesign-desktop-jina.md", note: "Local reader snapshot of IDD-S01 used for help-leaf extraction." }
  - { id: IDD-S03, url: "https://helpx.adobe.com/indesign/using/toolbox.html", note: "InDesign toolbox source surface." }
  - { id: IDD-S04, path: "_source_snapshots/indesign-tools-jina.md", note: "Local toolbox snapshot." }
  - { id: IDD-S05, url: "https://helpx.adobe.com/indesign/using/default-keyboard-shortcuts.html", note: "InDesign keyboard shortcut source surface." }
  - { id: IDD-S06, path: "_source_snapshots/indesign-keyboard-shortcuts-jina.md", note: "Local shortcut snapshot." }
  - { id: IDD-S07, path: "_source_snapshots/indesign-supported-file-formats-jina.md", note: "Local supported-file-format snapshot." }
  - { id: IDD-S08, path: "_source_snapshots/indesign-scripting-jina.md", note: "Local scripting snapshot." }
  - { id: IDD-S09, path: "_source_snapshots/indesign-uxp-dom-api-jina.md", note: "Local UXP DOM API snapshot." }
  - { id: IDD-S10, path: "07-indesign-leaf-index.md", note: "Generated InDesign help leaf index." }
  - { id: IDD-S11, path: "17-indesign-feature-use-cards.md", note: "Generated InDesign Feature Use Cards." }
  - { id: IDD-S12, path: "30-indesign-expanded-count-ledger.md", note: "Expanded InDesign online-source count ledger." }
```
