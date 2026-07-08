---
file_id: studio-rust-implementation-backlog
file_kind: source_distilled_implementation_backlog
topic_id: SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG
title: Studio Rust Implementation Backlog
status: draft
updated_at: '2026-07-05'
backlog_item_count: 28
build_slice_count: 5
feature_row_count: 2730
tool_registry_row_count: 1219
compatibility_record_count: 410
---

## [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG] Studio Rust Implementation Backlog

<topic id="backlog-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and policy for the source-distilled Studio Rust implementation backlog.">

### [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG.coverage] Coverage

```yaml
coverage:
  distillation_status: source_distilled_studio_rust_implementation_backlog
  backlog_item_count: 28
  build_slice_count: 5
  feature_row_count: 2730
  tool_registry_row_count: 1219
  compatibility_record_count: 410
  format_family_count: 38
  policy:
    not_product_authority: This is implementation planning input only until promoted into a work packet or spec authority.
    local_first_rule: Every backlog item targets built-in local-first Studio behavior; provider behavior remains optional adapter scope.
    naming_rule: Shipped commands and modules use Handshake-native names; vendor names stay in source refs and compatibility fixtures.
    manual_rule: Every implemented command must update the internal Studio UserManual in the same change.
```

</topic>

<topic id="primitive-backlog" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Build slices and primitive backlog records for future Studio implementation work.">

### [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG.records] Primitive Backlog

```yaml
build_slices:
- slice_id: studio.slice.layered-raster-core.v1
  primitive_domains:
  - layer
  - raster
  - mask
  - selection
  - color
  - raw
  - camera_raw
  purpose: Unblock Photoshop and Affinity Photo class raster editing, non-destructive layers, masks, selections, and raw development.
- slice_id: studio.slice.vector-typography-design.v1
  primitive_domains:
  - vector
  - typography
  - style_system
  - brush_engine
  - geometry
  - design_systems
  purpose: Unblock Illustrator, Affinity Designer, Photoshop vector/type, and Figma Draw/design-system behavior.
- slice_id: studio.slice.layout-prepress-publishing.v1
  primitive_domains:
  - page_layout
  - tables
  - prepress
  - export
  - presentation
  - web
  - brand_assets
  purpose: Unblock InDesign, Affinity Publisher, Figma Slides/Sites/Buzz, and publication export/preflight behavior.
- slice_id: studio.slice.file-compatibility.v1
  primitive_domains:
  - file_io
  - export
  - prepress
  - asset_pipeline
  purpose: Preserve existing creative file-format compatibility through fixtures, adapters, and unsupported-feature diagnostics.
- slice_id: studio.slice.automation-collaboration-ai.v1
  primitive_domains:
  - automation
  - dev_mode
  - ai
  - collaboration
  - interactive
  - motion
  - whiteboard
  purpose: Unblock model/operator workflow, local-first collaboration, Figma-like interaction/motion, and optional provider adapters.
primitive_backlog:
- backlog_id: studio.backlog.ai.v1
  primitive_domain: ai
  mapping_status: existing_primitive_map
  studio_primitive: StudioModelToolContract
  engine_module: studio_model_tools
  model_tool_surface: studio.ai.invoke_tool
  source_apps_present:
  - figma
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 119
    tool_registry_rows: 39
    compatibility_records: 10
    format_refs: 2
  provider_posture_counts:
    local_primitive_candidate: 2
    provider_adapter: 21
    provider_adapter_or_local_model_candidate: 96
  file_format_compatibility_counts:
    export: 2
    import: 7
    not_applicable: 110
  format_refs:
  - format.make
  - format.unspecified
  base_scope: Implement StudioModelToolContract as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - offline fallback
  - optional provider adapter
  - attribution and recovery receipts
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - offline behavior test
  - provider-adapter mock test
- backlog_id: studio.backlog.asset_pipeline.v1
  primitive_domain: asset_pipeline
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioAssetPipeline
  engine_module: studio_asset_pipeline
  model_tool_surface: studio.asset_pipeline.mutate_or_execute
  source_apps_present:
  - illustrator
  - indesign
  source_counts:
    feature_rows: 0
    tool_registry_rows: 44
    compatibility_records: 2
    format_refs: 1
  provider_posture_counts: {}
  file_format_compatibility_counts: {}
  format_refs:
  - format.pdf
  base_scope: Implement StudioAssetPipeline as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.automation.v1
  primitive_domain: automation
  mapping_status: existing_primitive_map
  studio_primitive: StudioActionGraph
  engine_module: studio_automation
  model_tool_surface: studio.automation.run_action_graph
  source_apps_present:
  - affinity
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 0
    tool_registry_rows: 57
    compatibility_records: 1
    format_refs: 1
  provider_posture_counts: {}
  file_format_compatibility_counts: {}
  format_refs:
  - format.raw
  base_scope: Implement StudioActionGraph as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.brand_assets.v1
  primitive_domain: brand_assets
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioBrandAssets
  engine_module: studio_brand_assets
  model_tool_surface: studio.brand_assets.mutate_or_execute
  source_apps_present:
  - figma
  source_counts:
    feature_rows: 1
    tool_registry_rows: 11
    compatibility_records: 1
    format_refs: 2
  provider_posture_counts:
    local_primitive: 1
  file_format_compatibility_counts:
    not_applicable: 1
  format_refs:
  - format.buzz
  - format.csv
  base_scope: Implement StudioBrandAssets as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.brush_engine.v1
  primitive_domain: brush_engine
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioBrushEngine
  engine_module: studio_brush_engine
  model_tool_surface: studio.brush_engine.mutate_or_execute
  source_apps_present:
  - photoshop
  source_counts:
    feature_rows: 0
    tool_registry_rows: 22
    compatibility_records: 0
    format_refs: 0
  provider_posture_counts: {}
  file_format_compatibility_counts: {}
  format_refs: []
  base_scope: Implement StudioBrushEngine as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.camera_raw.v1
  primitive_domain: camera_raw
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioCameraRaw
  engine_module: studio_camera_raw
  model_tool_surface: studio.camera_raw.mutate_or_execute
  source_apps_present:
  - photoshop
  source_counts:
    feature_rows: 0
    tool_registry_rows: 34
    compatibility_records: 2
    format_refs: 3
  provider_posture_counts: {}
  file_format_compatibility_counts: {}
  format_refs:
  - format.dng
  - format.exr_hdr
  - format.raw
  base_scope: Implement StudioCameraRaw as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.collaboration.v1
  primitive_domain: collaboration
  mapping_status: existing_primitive_map
  studio_primitive: StudioCollaborationSession
  engine_module: studio_collaboration
  model_tool_surface: studio.collab.apply_review_or_edit
  source_apps_present:
  - figma
  - illustrator
  - indesign
  source_counts:
    feature_rows: 2
    tool_registry_rows: 42
    compatibility_records: 1
    format_refs: 13
  provider_posture_counts:
    local_first_collaboration_primitive: 2
  file_format_compatibility_counts:
    not_applicable: 2
  format_refs:
  - format.buzz
  - format.csv
  - format.deck
  - format.fig
  - format.gif
  - format.jam
  - format.jpeg
  - format.pdf
  - format.png
  - format.pptx
  - format.site
  - format.sketch
  - format.svg
  base_scope: Implement StudioCollaborationSession as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - offline fallback
  - optional provider adapter
  - attribution and recovery receipts
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - offline behavior test
  - provider-adapter mock test
- backlog_id: studio.backlog.color.v1
  primitive_domain: color
  mapping_status: existing_primitive_map
  studio_primitive: StudioColorPipeline
  engine_module: studio_color
  model_tool_surface: studio.color.transform
  source_apps_present:
  - affinity
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 164
    tool_registry_rows: 72
    compatibility_records: 29
    format_refs: 4
  provider_posture_counts:
    local_first_collaboration_primitive: 1
    local_primitive: 64
    local_primitive_candidate: 92
    optional_integration: 7
  file_format_compatibility_counts:
    export: 26
    import: 1
    not_applicable: 137
  format_refs:
  - format.exr_hdr
  - format.pdf
  - format.raw
  - format.unspecified
  base_scope: Implement StudioColorPipeline as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.design_systems.v1
  primitive_domain: design_systems
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioDesignSystems
  engine_module: studio_design_systems
  model_tool_surface: studio.design_systems.mutate_or_execute
  source_apps_present:
  - figma
  source_counts:
    feature_rows: 165
    tool_registry_rows: 31
    compatibility_records: 0
    format_refs: 0
  provider_posture_counts:
    compatibility_shim: 134
    local_first_collaboration_primitive: 12
    provider_adapter_or_local_model_candidate: 19
  file_format_compatibility_counts:
    not_applicable: 165
  format_refs: []
  base_scope: Implement StudioDesignSystems as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.dev_mode.v1
  primitive_domain: dev_mode
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioDevMode
  engine_module: studio_dev_mode
  model_tool_surface: studio.dev_mode.mutate_or_execute
  source_apps_present:
  - figma
  source_counts:
    feature_rows: 6
    tool_registry_rows: 14
    compatibility_records: 1
    format_refs: 1
  provider_posture_counts:
    compatibility_shim: 4
    provider_adapter_or_local_model_candidate: 2
  file_format_compatibility_counts:
    not_applicable: 6
  format_refs:
  - format.css
  base_scope: Implement StudioDevMode as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.export.v1
  primitive_domain: export
  mapping_status: existing_primitive_map
  studio_primitive: StudioExportRecipe
  engine_module: studio_export
  model_tool_surface: studio.export.render
  source_apps_present:
  - indesign
  - photoshop
  source_counts:
    feature_rows: 139
    tool_registry_rows: 15
    compatibility_records: 139
    format_refs: 9
  provider_posture_counts:
    compatibility_shim: 3
    local_primitive_candidate: 105
    optional_integration: 31
  file_format_compatibility_counts:
    export: 44
    fixture_required: 93
    import: 2
  format_refs:
  - format.eps
  - format.epub
  - format.html
  - format.jpeg
  - format.pdf
  - format.png
  - format.ps
  - format.unspecified
  - format.xml
  base_scope: Implement StudioExportRecipe as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - format fixtures
  - unsupported-feature diagnostics
  - round-trip report
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - import/export fixture set
  - round-trip unsupported-feature report
- backlog_id: studio.backlog.file_io.v1
  primitive_domain: file_io
  mapping_status: existing_primitive_map
  studio_primitive: StudioFileIO
  engine_module: studio_import_export
  model_tool_surface: studio.file_io.convert_or_place
  source_apps_present:
  - affinity
  - figma
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 68
    tool_registry_rows: 99
    compatibility_records: 87
    format_refs: 34
  provider_posture_counts:
    compatibility_shim: 61
    local_first_collaboration_primitive: 2
    local_primitive: 1
    provider_adapter_or_local_model_candidate: 4
  file_format_compatibility_counts:
    export: 21
    fixture_required: 30
    import: 15
    round_trip: 2
  format_refs:
  - format.afdesign
  - format.afphoto
  - format.afpub
  - format.ai
  - format.ait
  - format.buzz
  - format.css
  - format.deck
  - format.dwg
  - format.dxf
  - format.eps
  - format.epub
  - format.exr_hdr
  - format.fig
  - format.gif
  - format.html
  - format.idml
  - format.indd
  - format.jam
  - format.jpeg
  - format.make
  - format.pdf
  - format.png
  - format.psb
  - format.psd
  - format.raw
  - format.site
  - format.sketch
  - format.svg
  - format.tiff
  - format.unspecified
  - format.webp
  - format.xls_excel
  - format.xml
  base_scope: Implement StudioFileIO as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - format fixtures
  - unsupported-feature diagnostics
  - round-trip report
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - import/export fixture set
  - round-trip unsupported-feature report
- backlog_id: studio.backlog.geometry.v1
  primitive_domain: geometry
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioGeometry
  engine_module: studio_geometry
  model_tool_surface: studio.geometry.mutate_or_execute
  source_apps_present:
  - photoshop
  source_counts:
    feature_rows: 0
    tool_registry_rows: 24
    compatibility_records: 0
    format_refs: 0
  provider_posture_counts: {}
  file_format_compatibility_counts: {}
  format_refs: []
  base_scope: Implement StudioGeometry as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.interactive.v1
  primitive_domain: interactive
  mapping_status: existing_primitive_map
  studio_primitive: StudioInteractiveDocumentSurface
  engine_module: studio_interaction
  model_tool_surface: studio.interaction.mutate
  source_apps_present:
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 217
    tool_registry_rows: 16
    compatibility_records: 34
    format_refs: 4
  provider_posture_counts:
    local_primitive: 1
    local_primitive_candidate: 205
    optional_integration: 11
  file_format_compatibility_counts:
    export: 19
    fixture_required: 8
    import: 6
    not_applicable: 184
  format_refs:
  - format.epub
  - format.html
  - format.pdf
  - format.unspecified
  base_scope: Implement StudioInteractiveDocumentSurface as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.layer.v1
  primitive_domain: layer
  mapping_status: existing_primitive_map
  studio_primitive: StudioLayerGraph
  engine_module: studio_layer_graph
  model_tool_surface: studio.layer_graph.mutate
  source_apps_present:
  - affinity
  - indesign
  - photoshop
  source_counts:
    feature_rows: 213
    tool_registry_rows: 26
    compatibility_records: 7
    format_refs: 1
  provider_posture_counts:
    local_primitive_candidate: 212
    optional_integration: 1
  file_format_compatibility_counts:
    export: 5
    import: 2
    not_applicable: 206
  format_refs:
  - format.unspecified
  base_scope: Implement StudioLayerGraph as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - visual regression fixture
  - state snapshot diagnostics
  - performance guard
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - golden render or state fixture
  - undo/redo replay test
- backlog_id: studio.backlog.motion.v1
  primitive_domain: motion
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioMotion
  engine_module: studio_motion
  model_tool_surface: studio.motion.mutate_or_execute
  source_apps_present:
  - figma
  source_counts:
    feature_rows: 1
    tool_registry_rows: 11
    compatibility_records: 1
    format_refs: 1
  provider_posture_counts:
    local_primitive: 1
  file_format_compatibility_counts:
    not_applicable: 1
  format_refs:
  - format.gif
  base_scope: Implement StudioMotion as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.page_layout.v1
  primitive_domain: page_layout
  mapping_status: existing_primitive_map
  studio_primitive: StudioPageSpread
  engine_module: studio_layout
  model_tool_surface: studio.layout.mutate_document
  source_apps_present:
  - affinity
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 659
    tool_registry_rows: 53
    compatibility_records: 46
    format_refs: 3
  provider_posture_counts:
    compatibility_shim: 2
    local_primitive: 28
    local_primitive_candidate: 597
    optional_integration: 29
    provider_adapter: 3
  file_format_compatibility_counts:
    export: 32
    fixture_required: 6
    import: 6
    not_applicable: 614
    round_trip: 1
  format_refs:
  - format.epub
  - format.pdf
  - format.unspecified
  base_scope: Implement StudioPageSpread as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - visual regression fixture
  - state snapshot diagnostics
  - performance guard
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - golden render or state fixture
  - undo/redo replay test
- backlog_id: studio.backlog.presentation.v1
  primitive_domain: presentation
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioPresentation
  engine_module: studio_presentation
  model_tool_surface: studio.presentation.mutate_or_execute
  source_apps_present:
  - figma
  source_counts:
    feature_rows: 0
    tool_registry_rows: 11
    compatibility_records: 1
    format_refs: 3
  provider_posture_counts: {}
  file_format_compatibility_counts: {}
  format_refs:
  - format.deck
  - format.pdf
  - format.pptx
  base_scope: Implement StudioPresentation as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.raster.v1
  primitive_domain: raster
  mapping_status: existing_primitive_map
  studio_primitive: StudioRasterPipeline
  engine_module: studio_raster
  model_tool_surface: studio.raster.apply_operation
  source_apps_present:
  - affinity
  - indesign
  - photoshop
  source_counts:
    feature_rows: 175
    tool_registry_rows: 82
    compatibility_records: 4
    format_refs: 3
  provider_posture_counts:
    local_primitive_candidate: 170
    optional_integration: 4
    provider_adapter: 1
  file_format_compatibility_counts:
    export: 1
    import: 1
    not_applicable: 173
  format_refs:
  - format.exr_hdr
  - format.raw
  - format.unspecified
  base_scope: Implement StudioRasterPipeline as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - visual regression fixture
  - state snapshot diagnostics
  - performance guard
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - golden render or state fixture
  - undo/redo replay test
- backlog_id: studio.backlog.raw.v1
  primitive_domain: raw
  mapping_status: existing_primitive_map
  studio_primitive: StudioRawDevelopRecipe
  engine_module: studio_raw
  model_tool_surface: studio.raw.update_recipe
  source_apps_present:
  - affinity
  source_counts:
    feature_rows: 24
    tool_registry_rows: 0
    compatibility_records: 0
    format_refs: 0
  provider_posture_counts:
    local_primitive_candidate: 24
  file_format_compatibility_counts:
    not_applicable: 24
  format_refs: []
  base_scope: Implement StudioRawDevelopRecipe as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.selection.v1
  primitive_domain: selection
  mapping_status: existing_primitive_map
  studio_primitive: StudioSelectionSet
  engine_module: studio_selection
  model_tool_surface: studio.selection.create_or_refine
  source_apps_present:
  - affinity
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 112
    tool_registry_rows: 26
    compatibility_records: 2
    format_refs: 1
  provider_posture_counts:
    local_primitive: 19
    local_primitive_candidate: 88
    optional_integration: 1
    provider_adapter: 4
  file_format_compatibility_counts:
    export: 1
    import: 1
    not_applicable: 110
  format_refs:
  - format.unspecified
  base_scope: Implement StudioSelectionSet as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.style_system.v1
  primitive_domain: style_system
  mapping_status: existing_primitive_map
  studio_primitive: StudioStyleRegistry
  engine_module: studio_styles
  model_tool_surface: studio.styles.upsert
  source_apps_present:
  - illustrator
  - indesign
  source_counts:
    feature_rows: 17
    tool_registry_rows: 22
    compatibility_records: 0
    format_refs: 0
  provider_posture_counts:
    local_primitive: 17
  file_format_compatibility_counts:
    not_applicable: 17
  format_refs: []
  base_scope: Implement StudioStyleRegistry as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.tables.v1
  primitive_domain: tables
  mapping_status: existing_primitive_map
  studio_primitive: StudioTableFrame
  engine_module: studio_tables
  model_tool_surface: studio.table.mutate
  source_apps_present:
  - indesign
  source_counts:
    feature_rows: 10
    tool_registry_rows: 0
    compatibility_records: 2
    format_refs: 1
  provider_posture_counts:
    local_primitive_candidate: 10
  file_format_compatibility_counts:
    import: 2
    not_applicable: 8
  format_refs:
  - format.unspecified
  base_scope: Implement StudioTableFrame as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.typography.v1
  primitive_domain: typography
  mapping_status: existing_primitive_map
  studio_primitive: StudioTextRunAndStory
  engine_module: studio_typography
  model_tool_surface: studio.typography.edit_story
  source_apps_present:
  - affinity
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 308
    tool_registry_rows: 74
    compatibility_records: 21
    format_refs: 2
  provider_posture_counts:
    local_primitive: 55
    local_primitive_candidate: 228
    optional_integration: 23
    provider_adapter_or_local_model_candidate: 2
  file_format_compatibility_counts:
    export: 6
    fixture_required: 4
    import: 9
    not_applicable: 287
    round_trip: 2
  format_refs:
  - format.epub
  - format.unspecified
  base_scope: Implement StudioTextRunAndStory as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - visual regression fixture
  - state snapshot diagnostics
  - performance guard
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - golden render or state fixture
  - undo/redo replay test
- backlog_id: studio.backlog.vector.v1
  primitive_domain: vector
  mapping_status: existing_primitive_map
  studio_primitive: StudioVectorPathGraph
  engine_module: studio_vector
  model_tool_surface: studio.vector.path_mutate
  source_apps_present:
  - affinity
  - figma
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 319
    tool_registry_rows: 153
    compatibility_records: 18
    format_refs: 3
  provider_posture_counts:
    local_primitive: 178
    local_primitive_candidate: 138
    optional_integration: 3
  file_format_compatibility_counts:
    fixture_required: 5
    import: 11
    not_applicable: 302
    round_trip: 1
  format_refs:
  - format.css
  - format.svg
  - format.unspecified
  base_scope: Implement StudioVectorPathGraph as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - visual regression fixture
  - state snapshot diagnostics
  - performance guard
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
  - golden render or state fixture
  - undo/redo replay test
- backlog_id: studio.backlog.web.v1
  primitive_domain: web
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioWeb
  engine_module: studio_web
  model_tool_surface: studio.web.mutate_or_execute
  source_apps_present:
  - figma
  source_counts:
    feature_rows: 0
    tool_registry_rows: 14
    compatibility_records: 1
    format_refs: 1
  provider_posture_counts: {}
  file_format_compatibility_counts: {}
  format_refs:
  - format.site
  base_scope: Implement StudioWeb as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.whiteboard.v1
  primitive_domain: whiteboard
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioWhiteboard
  engine_module: studio_whiteboard
  model_tool_surface: studio.whiteboard.mutate_or_execute
  source_apps_present:
  - figma
  source_counts:
    feature_rows: 1
    tool_registry_rows: 22
    compatibility_records: 0
    format_refs: 0
  provider_posture_counts:
    local_primitive: 1
  file_format_compatibility_counts:
    not_applicable: 1
  format_refs: []
  base_scope: Implement StudioWhiteboard as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
- backlog_id: studio.backlog.workspace.v1
  primitive_domain: workspace
  mapping_status: derived_candidate_needs_primitive_map_promotion
  studio_primitive: StudioWorkspace
  engine_module: studio_workspace
  model_tool_surface: studio.workspace.mutate_or_execute
  source_apps_present:
  - affinity
  - illustrator
  - indesign
  - photoshop
  source_counts:
    feature_rows: 10
    tool_registry_rows: 205
    compatibility_records: 0
    format_refs: 0
  provider_posture_counts:
    local_primitive: 10
  file_format_compatibility_counts:
    not_applicable: 10
  format_refs: []
  base_scope: Implement StudioWorkspace as a local-first Rust-backed Studio primitive with source-specific behavior variants.
  high_roi_additions:
  - typed Rust command contract
  - model-visible receipt
  - undo/replay proof
  - internal Studio UserManual topic
  - source-app behavior comparison fixture
  reuse:
    primitive_map: 05-studio-primitive-map.md
    command_contract_seed: 10-studio-command-contracts.md
    feature_rows: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
    tool_registry: 45-source-distilled-tool-registry.md
    format_registry: 46-file-format-compatibility-registry.md
  gaps_closed_against_rebuild:
  - groups source-app feature/tool records into one Studio implementation lane
  - keeps vendor provenance separate from shipped Handshake-native naming
  - preserves Affinity rows as source variants rather than Adobe overlap
  - carries manual and fixture promotion obligations forward
  risks:
  - overclaiming parity before exact source-page behavior inspection
  - implementing duplicate primitives instead of shared Studio primitive
  - format compatibility loss without representative fixtures
  - provider/cloud behavior accidentally becoming a local-first dependency
  failure_scenarios:
  - source-app option variant has no Studio state-model equivalent
  - round-trip import/export silently drops unsupported data
  - manual topic is skipped when a command ships
  - model agent lacks enough receipt fields to diagnose failure
  remediations:
  - promote selected rows through typed command contracts before product code
  - require fixtures and unsupported-feature receipts for compatibility features
  - add same-change Studio UserManual entries for implemented commands
  - run local/offline tests for provider-adjacent behavior
  verification_needs:
  - exact source-page or app-behavior inspection before implementation
  - command-contract acceptance criteria
  - receipt schema validation
  - same-change Studio UserManual update
```

</topic>

<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for the generated Studio Rust implementation backlog.">

### [SFR-STUDIO-RUST-IMPLEMENTATION-BACKLOG.sources] Sources

```yaml
sources:
- id: BACKLOG-S01
  path: 05-studio-primitive-map.md
  note: Existing Studio primitive and engine module map.
- id: BACKLOG-S02
  path: 10-studio-command-contracts.md
  note: Command-contract schema and seed contracts.
- id: BACKLOG-S03
  path: 18-feature-use-card-manual-handoff-index.md
  note: Manual handoff obligations.
- id: BACKLOG-S04
  path: 39-photoshop-source-distilled-feature-rows.md through 43-figma-source-distilled-feature-rows.md
  note: Source-distilled feature rows.
- id: BACKLOG-S05
  path: 44-cross-app-overlap-and-affinity-dedupe-map.md
  note: Overlap and Affinity dedupe policy.
- id: BACKLOG-S06
  path: 45-source-distilled-tool-registry.md
  note: Tool and surface registry.
- id: BACKLOG-S07
  path: 46-file-format-compatibility-registry.md
  note: File-format compatibility registry.
```

</topic>
