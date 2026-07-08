---
file_id: "figma-source-distilled-domain-ledger"
topic_id: SFR-FIGMA-SOURCE-DISTILLED-DOMAINS
title: "Figma Source Distilled Domain Ledger"
status: draft
summary: "Online-source-distilled Figma Design, FigJam, Draw, Motion, Slides, Sites, Buzz, Make, Dev Mode, API, and collaboration domains for Studio parity planning."
sources: 17
updated_at: "2026-07-05"
---

## [SFR-FIGMA-SOURCE-DISTILLED-DOMAINS] Figma Source Distilled Domain Ledger

### [SFR-FIGMA-SOURCE-DISTILLED-DOMAINS.policy] Policy

```yaml
policy:
  distillation_status: "online_source_distilled"
  installed_exports_role: "not applicable; Figma is primarily documented through online help, developer, export, and release sources"
  rebuild_target: "Handshake Studio local-first Rust collaborative design, whiteboard, prototype, motion, site, slide, asset, dev handoff, and API tooling with Figma-compatible import/export where source-observable"
  naming_rule: "Figma product names remain source/provenance naming only; Studio surfaces use Handshake-native names."
  coverage_rule: "Merge help leaves, category snapshots, import/export docs, developer API docs, MCP/Dev Mode docs, release notes, and provider/AI docs."
```

### [SFR-FIGMA-SOURCE-DISTILLED-DOMAINS.domains] Domains

```yaml
domains:
  - id: fig.domain.figma_design
    name: "Design files, canvas, frames, sections, layers, components, variables, styles, prototypes, and libraries"
    app_behavior: "Figma Design covers UI/product design through files, pages, frames, sections, layers, groups, constraints, auto layout, grids, text, fills, strokes, effects, images, videos, components, instances, variants, variables, styles, libraries, prototypes, comments, branches, version history, and multiplayer collaboration."
    tool_and_feature_scope:
      - "Move/scale, frame, section, slice, shape, pen, pencil/brush where documented, text, resources, comments, components, variants, properties, variables, styles, libraries, auto layout, constraints, grids, prototype interactions, overlays, smart animate, and sharing."
      - "Design systems, team libraries, file organization, branching, comments, multiplayer cursors, version history, permissions, and export."
    studio_primitive_domains: [design_systems, vector, layout, typography, collaboration, prototype]
    source_surfaces: [help_leaf, category_snapshot, import_export, release_delta, api_docs]
    manual_topic_candidate: "studio.manual.design.figma-class-product-design"
    implementation_notes:
      - "Local-first Studio needs CRDT/event-log design state and deterministic exports instead of mandatory cloud sync."
      - "Components, variables, and styles should become shared Studio design-system primitives."
  - id: fig.domain.figjam
    name: "Whiteboarding, diagrams, sticky notes, tables, meetings, voting, and facilitation"
    app_behavior: "FigJam covers collaborative whiteboarding with boards, sticky notes, shapes, connectors, diagrams, sections, tables, stamps, voting, timers, meetings, widgets, templates, comments, and multiplayer facilitation."
    tool_and_feature_scope:
      - "Sticky notes, text, shapes, connectors, arrows, sections, tables, diagramming, stamps, reactions, cursor chat/following where documented, timers, voting, music/facilitation, templates, widgets, comments, and export."
      - "Board organization, brainstorming workflows, meeting artifacts, and collaboration state."
    studio_primitive_domains: [whiteboard, diagramming, collaboration, tables, export]
    source_surfaces: [help_leaf, category_snapshot, release_delta]
    manual_topic_candidate: "studio.manual.whiteboard.figjam-class-workshops"
    implementation_notes:
      - "Whiteboard primitives should reuse vector, text, table, and collaboration engines."
      - "Meeting/facilitation behavior needs local receipts for votes, timers, participants, and board changes."
  - id: fig.domain.draw
    name: "Figma Draw, vector networks, shapes, pen/pencil/brush, shape builder, simplify, and vectorize"
    app_behavior: "Figma Draw extends vector authoring with expressive drawing, brush/pen/pencil behavior, editable vector networks, shape construction, vectorize/simplify workflows, text-to-path, and illustration-oriented editing."
    tool_and_feature_scope:
      - "Pen, pencil, brush, shapes, vector edit mode, bend/control handles, boolean/shape operations, shape builder, simplify, vectorize, text-to-path, fills, strokes, effects, and export."
      - "Illustration workflows that overlap with Illustrator, Affinity Designer, and Figma Design vector editing."
    studio_primitive_domains: [vector, brush_engine, geometry, boolean_ops, typography]
    source_surfaces: [help_leaf, category_snapshot, release_delta]
    manual_topic_candidate: "studio.manual.vector.figma-draw-class-authoring"
    implementation_notes:
      - "Draw parity should share the same vector kernel as Illustrator/Affinity parity."
      - "Vector networks need a topology model that can represent Figma-style paths and traditional Bezier paths."
  - id: fig.domain.motion
    name: "Motion, timeline, keyframes, easing, animated prototypes, and video/GIF export"
    app_behavior: "Figma Motion source surfaces describe animation and motion workflows including timelines, keyframes, easing, object/property animation, preview/playback, animated export, and design-to-motion handoff behavior."
    tool_and_feature_scope:
      - "Timeline, keyframes, easing curves, object/layer animation, transitions, smart animate overlap, playback controls, animated export formats, and motion presets where documented."
      - "Interaction with prototype states and design layers."
    studio_primitive_domains: [motion, timeline, prototype, export, vector]
    source_surfaces: [category_snapshot, release_delta, help_leaf]
    manual_topic_candidate: "studio.manual.motion.design-animation"
    implementation_notes:
      - "Motion should compile to a local timeline data model with reproducible previews and exports."
      - "Animated exports require fixtures for frame timing, easing, text rendering, and asset embedding."
  - id: fig.domain.slides
    name: "Slides, decks, presentation design, presenter workflows, and export"
    app_behavior: "Figma Slides provides deck creation, slide layouts, design assets, presentation mode, speaker/presenter workflows, comments/collaboration, templates, and deck export/import where documented."
    tool_and_feature_scope:
      - "Slides, deck files/local copies, slide templates, layout tools, design asset reuse, presenter mode, comments, sharing, and PPTX/PDF/image export or import surfaces where documented."
      - "Collaboration and versioning around presentation artifacts."
    studio_primitive_domains: [presentation, layout, typography, vector, export, collaboration]
    source_surfaces: [category_snapshot, import_export, release_delta]
    manual_topic_candidate: "studio.manual.presentations.figma-slides-class"
    implementation_notes:
      - "Slides can reuse page-layout primitives with deck-specific navigation and presenter state."
      - "PPTX compatibility needs fixture-based round-trip tests."
  - id: fig.domain.sites
    name: "Sites, web publishing, responsive pages, components, domains, and export"
    app_behavior: "Figma Sites covers web page creation and publishing workflows, responsive layouts, design-to-site behavior, components, links, assets, previews, publishing, and hosting/domain/provider surfaces where documented."
    tool_and_feature_scope:
      - "Site files/local copies, pages, sections, responsive layout, interactive links, components, styles, assets, preview, publish, custom domain/provider flows, and export/code handoff where documented."
      - "Local-first site generation and provider-backed publishing split."
    studio_primitive_domains: [web, layout, design_systems, export, provider_adapter]
    source_surfaces: [category_snapshot, import_export, release_delta, provider_or_cloud]
    manual_topic_candidate: "studio.manual.web.figma-sites-class"
    implementation_notes:
      - "Studio should generate local site artifacts first; hosting/publishing remains optional provider behavior."
      - "Responsive constraints should reuse design layout primitives."
  - id: fig.domain.buzz
    name: "Buzz, brand asset production, templates, bulk content, and marketing outputs"
    app_behavior: "Figma Buzz covers brand and marketing asset production through templates, brand libraries, editable assets, bulk creation, export, team collaboration, and provider-backed generation where documented."
    tool_and_feature_scope:
      - "Buzz files/local copies, brand kits, templates, locked/editable regions, batch/bulk content, CSV/data import where documented, asset export, collaboration, and AI/provider assistance."
      - "Repeatable production workflows for social, ad, and brand output."
    studio_primitive_domains: [brand_assets, templates, data_binding, export, collaboration]
    source_surfaces: [category_snapshot, import_export, release_delta, provider_or_cloud]
    manual_topic_candidate: "studio.manual.brand-assets.bulk-production"
    implementation_notes:
      - "Bulk brand output should reuse design-system variables, data binding, and batch export recipes."
      - "Template locks and brand controls need structured validation, not just UI hints."
  - id: fig.domain.make
    name: "Make, AI app generation, code layers, prototypes, and local/provider split"
    app_behavior: "Figma Make describes AI-assisted app/prototype generation, prompt-driven edits, code/layer concepts, app previews, iteration, sharing, and generated behavior surfaces."
    tool_and_feature_scope:
      - "Prompt-to-app or prompt-to-prototype generation, Make files/local copies, generated UI, code layers, preview, iteration, publishing/sharing, and AI/provider states."
      - "Connections between design layers, generated code, and editable app behavior where documented."
    studio_primitive_domains: [ai, app_generation, prototype, code_layers, provider_adapter]
    source_surfaces: [category_snapshot, provider_or_cloud, release_delta, api_docs]
    manual_topic_candidate: "studio.manual.ai.app-generation"
    implementation_notes:
      - "Studio Make-like behavior must clearly separate local code generation from optional provider calls."
      - "Generated code should produce inspectable diffs, build/test receipts, and rollback points."
  - id: fig.domain.dev_mode_api
    name: "Dev Mode, inspect, Code Connect, MCP, REST API, plugins, widgets, and webhooks"
    app_behavior: "Figma developer surfaces include Dev Mode inspect and handoff, measurements, code snippets, design tokens, Code Connect, MCP where documented, REST API, file API, plugins, widgets, webhooks, comments API, variables/libraries APIs, and OAuth/scopes."
    tool_and_feature_scope:
      - "Inspect, measurements, CSS/iOS/Android snippets where documented, design tokens, Code Connect mappings, MCP surfaces, REST file/team/project/comment/library/variable endpoints, plugin API, widget API, webhooks, OAuth, and rate/error behavior."
      - "Developer handoff workflows and automation around design data."
    studio_primitive_domains: [dev_mode, api, plugin_api, automation, design_systems]
    source_surfaces: [api_docs, help_leaf, category_snapshot, release_delta]
    manual_topic_candidate: "studio.manual.dev-mode.design-handoff"
    implementation_notes:
      - "Studio should expose a local API for design inspection and automation from day one."
      - "Figma API compatibility should be adapter-based and fixture-tested for supported schemas."
  - id: fig.domain.collaboration_import_export_local_copies
    name: "Collaboration, comments, permissions, branches, local copies, import/export, and compatibility"
    app_behavior: "Figma source surfaces cover multiplayer collaboration, comments, permissions, sharing, teams/projects, branching, version history, local copies, imports, exports, and compatibility across FIG/JAM/DECK/BUZZ/SITE/MAKE, Sketch, SVG, PDF, PNG, JPG, GIF, video, CSV, and PPTX where documented."
    tool_and_feature_scope:
      - "Comments, mentions, permissions, sharing links, teams/projects/files, version history, branches/merge where documented, local copies, import, export, backups, and recovery."
      - "Static export, design file export, image/PDF/SVG export, Sketch import, Figma-family local-copy formats, CSV/data imports, and deck export where documented."
    studio_primitive_domains: [collaboration, file_io, export, versioning, permissions, diagnostics]
    source_surfaces: [help_leaf, import_export, api_docs, release_delta]
    manual_topic_candidate: "studio.manual.file-compatibility.figma-class"
    implementation_notes:
      - "Local-first collaboration needs CRDT/event-log authority, attribution, recoverability, and deterministic merge receipts."
      - "Proprietary local-copy formats are compatibility targets and need fixture-based import/export diagnostics."
```

### [SFR-FIGMA-SOURCE-DISTILLED-DOMAINS.sources] Sources

```yaml
sources:
  - { id: FIG-S01, url: "https://help.figma.com/hc/en-us", note: "Official Figma help center source." }
  - { id: FIG-S02, path: "_source_snapshots/figma-help-home-jina.md", note: "Local Figma help home snapshot." }
  - { id: FIG-S03, path: "_source_snapshots/figma-design-category-jina.md", note: "Local Figma Design category snapshot." }
  - { id: FIG-S04, path: "_source_snapshots/figma-figjam-category-jina.md", note: "Local FigJam category snapshot." }
  - { id: FIG-S05, path: "_source_snapshots/figma-motion-category-jina.md", note: "Local Motion category snapshot." }
  - { id: FIG-S06, path: "_source_snapshots/figma-slides-category-jina.md", note: "Local Slides category snapshot." }
  - { id: FIG-S07, path: "_source_snapshots/figma-sites-category-jina.md", note: "Local Sites category snapshot." }
  - { id: FIG-S08, path: "_source_snapshots/figma-buzz-category-jina.md", note: "Local Buzz category snapshot." }
  - { id: FIG-S09, path: "_source_snapshots/figma-make-category-jina.md", note: "Local Make category snapshot." }
  - { id: FIG-S10, path: "_source_snapshots/figma-build-category-jina.md", note: "Local Build with Figma category snapshot." }
  - { id: FIG-S11, path: "_source_snapshots/figma-import-export-jina.md", note: "Local Figma import/export snapshot." }
  - { id: FIG-S12, path: "_source_snapshots/figma-imports-jina.md", note: "Local Figma import snapshot." }
  - { id: FIG-S13, path: "_source_snapshots/figma-export-formats-jina.md", note: "Local Figma export formats snapshot." }
  - { id: FIG-S14, path: "_source_snapshots/figma-api-docs-jina.md", note: "Local Figma developer API snapshot." }
  - { id: FIG-S15, path: "23-figma-leaf-index.md", note: "Generated Figma help leaf index and category evidence." }
  - { id: FIG-S16, path: "25-figma-feature-use-cards.md", note: "Generated Figma Feature Use Cards." }
  - { id: FIG-S17, path: "26-illustrator-figma-provider-posture-map.md", note: "Provider posture rows covering Figma." }
```
