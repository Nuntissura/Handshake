---
file_id: studio-app-feature-research-primitive-map
topic_id: SFR-PRIMITIVES
title: "Studio Primitive and Engine Module Map"
status: draft
summary: "Cross-app mapping from vendor feature domains to Handshake-native Studio primitives, Rust engine modules, state authority, diagnostics, and model-facing tool surfaces."
sources: 7
updated_at: "2026-07-05"
---

## [SFR-PRIMITIVES] Studio Primitive and Engine Module Map

### [SFR-PRIMITIVES.mapping] Primitive Map

```yaml
primitive_mappings:
  - primitive_domain: raster
    studio_primitive: StudioRasterPipeline
    engine_module: studio_raster
    state_authority: "ArtifactStore source asset + EventLedger edit receipts + non-destructive operation stack"
    model_tool_surface: "studio.raster.apply_operation"
    diagnostics: [operation_preview, before_after_hashes, pixel_bounds, failure_receipt]
    app_references: [photoshop, affinity_photo, affinity_designer]
  - primitive_domain: raw
    studio_primitive: StudioRawDevelopRecipe
    engine_module: studio_raw
    state_authority: "Immutable raw asset ref + develop recipe + rendered proxy artifact"
    model_tool_surface: "studio.raw.update_recipe"
    diagnostics: [raw_metadata, recipe_diff, proxy_render_receipt, color_profile_trace]
    app_references: [photoshop_camera_raw, affinity_photo]
  - primitive_domain: layer
    studio_primitive: StudioLayerGraph
    engine_module: studio_layer_graph
    state_authority: "CRDT layer graph draft promoted through EventLedger; raster/vector/layout layers share one graph contract"
    model_tool_surface: "studio.layer_graph.mutate"
    diagnostics: [layer_tree_snapshot, blend_mode_trace, visibility_state, undo_redo_receipt]
    app_references: [photoshop, affinity_photo, affinity_designer, affinity_publisher, indesign, illustrator, figma]
  - primitive_domain: selection
    studio_primitive: StudioSelectionSet
    engine_module: studio_selection
    state_authority: "Ephemeral selection draft with optional promotion to mask/path/annotation artifact"
    model_tool_surface: "studio.selection.create_or_refine"
    diagnostics: [selection_bounds, confidence_map, source_layer_ref, refinement_receipt]
    app_references: [photoshop, affinity_photo, affinity_designer, illustrator]
  - primitive_domain: mask
    studio_primitive: StudioMaskGraph
    engine_module: studio_mask
    state_authority: "Mask node graph attached to layer graph; supports raster, vector, luminosity/color, and compound masks"
    model_tool_surface: "studio.mask.compose"
    diagnostics: [mask_preview, source_selection_ref, coverage_stats, destructive_apply_warning]
    app_references: [photoshop, affinity_photo]
  - primitive_domain: color
    studio_primitive: StudioColorPipeline
    engine_module: studio_color
    state_authority: "Document color profile + swatch/style assets + color transform receipts"
    model_tool_surface: "studio.color.transform"
    diagnostics: [profile_trace, gamut_warning, before_after_histogram, ocio_config_ref]
    app_references: [photoshop, affinity_photo, indesign, illustrator, figma]
  - primitive_domain: vector
    studio_primitive: StudioVectorPathGraph
    engine_module: studio_vector
    state_authority: "CRDT path graph with fill/stroke/style nodes and exportable vector artifacts"
    model_tool_surface: "studio.vector.path_mutate"
    diagnostics: [path_topology, boolean_operation_trace, snapping_trace, export_preview]
    app_references: [photoshop, affinity_designer, affinity_publisher, illustrator, figma_draw]
  - primitive_domain: typography
    studio_primitive: StudioTextRunAndStory
    engine_module: studio_typography
    state_authority: "Story/text-run graph + style refs + font dependency records"
    model_tool_surface: "studio.typography.edit_story"
    diagnostics: [font_resolution, overset_text, shaping_trace, accessibility_text_trace]
    app_references: [photoshop, affinity_designer, affinity_publisher, indesign, illustrator, figma]
  - primitive_domain: page_layout
    studio_primitive: StudioPageSpread
    engine_module: studio_layout
    state_authority: "Page/spread/frame graph with linked assets, stories, and style refs"
    model_tool_surface: "studio.layout.mutate_document"
    diagnostics: [overset_text, missing_links, layout_reflow_trace, page_preview]
    app_references: [affinity_publisher, indesign, illustrator, figma_design, figjam, figma_slides, figma_sites]
  - primitive_domain: style_system
    studio_primitive: StudioStyleRegistry
    engine_module: studio_styles
    state_authority: "Reusable style definitions versioned by document/workspace and referenced by layers, text, objects, tables, and export recipes"
    model_tool_surface: "studio.styles.upsert"
    diagnostics: [style_dependency_graph, override_report, unused_style_report]
    app_references: [affinity_publisher, indesign, photoshop, illustrator, figma]
  - primitive_domain: tables
    studio_primitive: StudioTableFrame
    engine_module: studio_tables
    state_authority: "Table data/layout model embedded in stories or layout frames"
    model_tool_surface: "studio.table.mutate"
    diagnostics: [cell_overflow, style_resolution, import_diff]
    app_references: [affinity_publisher, indesign]
  - primitive_domain: export
    studio_primitive: StudioExportRecipe
    engine_module: studio_export
    state_authority: "Export recipe + artifact manifest + EventLedger receipt"
    model_tool_surface: "studio.export.render"
    diagnostics: [export_manifest, format_options, output_hashes, failed_asset_refs]
    app_references: [photoshop, affinity_photo, affinity_designer, affinity_publisher, indesign, illustrator, figma]
  - primitive_domain: file_io
    studio_primitive: StudioFileIO
    engine_module: studio_import_export
    state_authority: "Format adapter registry + fixture expectations + import/export receipts + unsupported-feature diagnostics"
    model_tool_surface: "studio.file_io.convert_or_place"
    diagnostics: [format_probe, compatibility_receipt, round_trip_report, unsupported_feature_report]
    app_references: [illustrator, figma, photoshop, affinity_photo, affinity_designer, affinity_publisher, indesign]
  - primitive_domain: prepress
    studio_primitive: StudioPreflightProfile
    engine_module: studio_prepress
    state_authority: "Preflight profile + check results + package/export receipts"
    model_tool_surface: "studio.prepress.run_check"
    diagnostics: [missing_fonts, missing_links, ink_report, accessibility_report, package_manifest]
    app_references: [indesign, affinity_publisher, illustrator]
  - primitive_domain: automation
    studio_primitive: StudioActionGraph
    engine_module: studio_automation
    state_authority: "Typed action graph, not opaque macro text; execution emits receipts per step"
    model_tool_surface: "studio.automation.run_action_graph"
    diagnostics: [step_trace, rollback_plan, batch_item_receipts, capability_denials]
    app_references: [photoshop, affinity_photo, indesign, illustrator, figma_dev_mode, figma_plugins]
  - primitive_domain: collaboration
    studio_primitive: StudioCollaborationSession
    engine_module: studio_collaboration
    state_authority: "CRDT session + EventLedger attribution + share/review adapter records"
    model_tool_surface: "studio.collab.apply_review_or_edit"
    diagnostics: [actor_site_ids, conflict_resolution_trace, review_thread_receipts]
    app_references: [photoshop, indesign, affinity_publisher, illustrator_projects, figma_multiplayer, figjam]
  - primitive_domain: interactive
    studio_primitive: StudioInteractiveDocumentSurface
    engine_module: studio_interaction
    state_authority: "Interaction graph, timeline/keyframe state, prototype navigation, presentation state, and export receipts"
    model_tool_surface: "studio.interaction.mutate"
    diagnostics: [flow_graph, timeline_trace, trigger_condition_report, interactive_export_receipt]
    app_references: [figma_prototyping, figma_motion, figma_slides, figma_sites, indesign_interactive]
  - primitive_domain: ai
    studio_primitive: StudioModelToolContract
    engine_module: studio_model_tools
    state_authority: "Provider/local model request receipt + prompt/input refs + output artifact manifest"
    model_tool_surface: "studio.ai.invoke_tool"
    diagnostics: [prompt_ref, model_id, provider_posture, seed_or_variation_refs, content_credentials]
    app_references: [photoshop, indesign, affinity_photo, illustrator, figma_make, figma_ai]
```

### [SFR-PRIMITIVES.command-contract] Command Contract Shape

```yaml
studio_command_contract:
  required_fields:
    - command_id
    - studio_primitive
    - engine_module
    - input_refs
    - typed_parameters
    - dry_run_supported
    - capability_requirements
    - undo_redo_semantics
    - eventledger_event_family
    - diagnostic_outputs
    - model_visible_summary
  rule: "A vendor feature is not implementation-ready until it maps to this command contract or is explicitly deferred/omitted."
```

### [SFR-PRIMITIVES.build-order] Suggested Build Order

```yaml
build_order:
  - slice: "S1-layered-raster-core"
    base_scope: "StudioDocument + StudioLayerGraph + StudioRasterPipeline + basic import/export."
    high_roi_additions: [undo_redo_receipts, layer_tree_diagnostics, proxy_render_cache]
    reuses: [ArtifactStore, EventLedger, Flight Recorder, CRDT draft state]
    closes_gaps: "Unblocks Photoshop/Affinity Photo layer, mask, adjustment, filter, and export rows."
    risks: [performance, destructive_edit_leakage, color_profile_drift]
    verification: [render_golden_images, event_replay, undo_redo_roundtrip]
  - slice: "S2-selection-mask-color"
    base_scope: "SelectionSet + MaskGraph + ColorPipeline with non-destructive attachment to layer graph."
    high_roi_additions: [mask_preview_diagnostics, coverage_stats, provider_ready_ai_selection_hook]
    reuses: [Media Annotation geometry, Photo Studio mask concepts, visual diagnostics]
    closes_gaps: "Turns selections/masks into reusable primitives for both raster editing and AI tools."
    risks: [mask_precision_loss, hidden_destructive_apply, model_selection_mismatch]
    verification: [mask_boolean_tests, pixel_coverage_tests, model_visible_snapshot]
  - slice: "S3-vector-and-typography"
    base_scope: "VectorPathGraph + TextRunAndStory + style references."
    high_roi_additions: [text_on_path, vector_boolean_trace, font_dependency_diagnostics]
    reuses: [shared style registry, export pipeline]
    closes_gaps: "Unblocks Affinity Designer, Photoshop vector/text, Illustrator vector/type, Figma Draw/vector networks, and Publisher/InDesign text primitives."
    risks: [font_shaping_complexity, path_boolean_robustness, CJK_gap]
    verification: [font_resolution_tests, vector_topology_tests, PDF/SVG_export_checks]
  - slice: "S4-layout-prepress"
    base_scope: "PageSpread + LayoutFrame + TableFrame + PreflightProfile + PDF/package export."
    high_roi_additions: [overset_text_detector, missing_link_report, accessibility_report]
    reuses: [ArtifactStore manifests, export recipes, Flight Recorder evidence]
    closes_gaps: "Unblocks InDesign/Affinity Publisher workflows plus Illustrator artboards and Figma frames/boards/slides/sites."
    risks: [layout_reflow_instability, prepress_false_pass, accessibility_tag_drift]
    verification: [PDF_preflight_tests, package_manifest_tests, accessibility_tree_checks]
  - slice: "S5-automation-model-tools"
    base_scope: "ActionGraph + ModelToolContract over existing primitives."
    high_roi_additions: [dry_run, rollback_plan, batch_receipts, provider_posture_tags]
    reuses: [Unified Tool Surface Contract, capability gates, EventLedger]
    closes_gaps: "Makes the system usable by parallel model agents instead of only human UI flows."
    risks: [unsafe_batch_mutation, provider_lock_in, poor_attribution]
    verification: [dry_run_diff_tests, capability_denial_tests, receipt_replay_tests]
  - slice: "S6-interaction-collaboration-file-compatibility"
    base_scope: "StudioFileIO + StudioInteractiveDocumentSurface + StudioCollaborationSession over existing document primitives."
    high_roi_additions: [round_trip_receipts, unsupported_feature_diagnostics, local_crdt_sessions, prototype_flow_snapshots]
    reuses: [ArtifactStore manifests, EventLedger attribution, CRDT draft state, export recipes]
    closes_gaps: "Unblocks Illustrator/Figma file compatibility, Figma-like prototyping/motion/slides/sites, and local-first collaboration parity."
    risks: [proprietary_schema_overclaim, timeline_export_drift, collaboration_conflict_loss]
    verification: [fixture_round_trip_tests, interaction_flow_replay, local_collaboration_conflict_tests]
```

### [SFR-PRIMITIVES.sources] Sources

```yaml
sources:
  - id: PRIM-S01
    url: ".GOV/spec/HANDSHAKE_PRODUCT_REFERENCE.md"
    note: "Studio boundary and shared-primitives framing."
  - id: PRIM-S02
    url: ".GOV/spec/master-spec-v02.197/spec-modules/10-product-surfaces.md"
    note: "Photo Studio and product-surface references."
  - id: PRIM-S03
    url: ".GOV/spec/master-spec-v02.197/spec-modules/06-mechanical-integrations.md"
    note: "EventLedger, Flight Recorder, Studio, and engine integration references."
  - id: PRIM-S04
    path: "19-studio-local-first-rust-posture.md"
    note: "Local-first, no-cloud-required, Rust-forward Studio posture."
  - id: PRIM-S05
    path: "20-illustrator-feature-map.md"
    note: "Illustrator feature families used to extend primitive app references."
  - id: PRIM-S06
    path: "21-figma-feature-map.md"
    note: "Figma-family feature families used to extend primitive app references."
  - id: PRIM-S07
    path: "27-illustrator-figma-parity-matrix.md"
    note: "Illustrator/Figma parity lanes mapped to Studio primitives."
```
