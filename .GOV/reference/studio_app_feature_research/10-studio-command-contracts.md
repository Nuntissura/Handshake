---
file_id: studio-app-feature-research-command-contracts
topic_id: SFR-COMMAND-CONTRACTS
title: "Studio Command Contract Seed"
status: draft
summary: "Reusable Studio command-contract schema and seed command set for promoting Photoshop, Affinity, and InDesign help leaves into Handshake-native Rust tool contracts."
sources: 6
updated_at: "2026-07-05"
---

## [SFR-COMMAND-CONTRACTS] Studio Command Contract Seed

### [SFR-COMMAND-CONTRACTS.schema] Contract Schema

```yaml
contract_schema:
  schema_id: "studio.command_contract.v0"
  required_fields:
    - command_id
    - command_name
    - studio_primitive
    - engine_module
    - vendor_source_refs
    - naming_posture
    - file_format_compatibility
    - parity_scope
    - provider_posture
    - input_refs
    - typed_parameters
    - output_refs
    - state_mutations
    - undo_redo_semantics
    - eventledger_event_family
    - crdt_scope
    - diagnostics
    - model_receipt
    - failure_modes
    - verification
  provider_posture_enum:
    - local_primitive
    - provider_adapter
    - optional_integration
    - compatibility_shim
    - omit_for_now
  promotion_rule: "A feature leaf is implementation-ready only after it has a command contract or an explicit omit/defer decision."
  naming_rule: "Command IDs and command names must be Handshake-native. Vendor app names are allowed only in vendor_source_refs, compatibility notes, fixtures, and migration documentation."
  file_format_rule: "Import/export commands must target compatibility with existing creative file formats and must not introduce a new replacement interchange format unless a separate product authority explicitly approves it."
```

### [SFR-COMMAND-CONTRACTS.seed] Seed Contracts

```yaml
contracts:
  - command_id: "studio.layer_graph.create_layer.v0"
    command_name: "Create Layer"
    studio_primitive: "StudioLayerGraph"
    engine_module: "studio_layer_graph"
    vendor_source_refs: ["photoshop.layer.stack", "affinity_photo.layer_stack", "indesign.layers"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Create a raster, vector, text, adjustment, fill, group, or placed-file layer node in the active document graph."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "optional_parent_layer_id", "optional_insert_index"]
    typed_parameters: { layer_kind: "enum", name: "string", visibility: "bool", opacity: "0..1", blend_mode: "enum" }
    output_refs: ["layer_id", "layer_tree_snapshot_ref"]
    state_mutations: ["append_or_insert_layer_node", "update_parent_child_order", "record_default_style_refs"]
    undo_redo_semantics: "Undo removes the new node and restores previous child order; redo recreates same stable layer_id if no conflict."
    eventledger_event_family: "studio.layer_graph.layer_created"
    crdt_scope: "document.layer_graph"
    diagnostics: ["layer_tree_snapshot", "ordering_trace", "style_resolution"]
    model_receipt: ["command_id", "layer_id", "parent_layer_id", "insert_index", "before_hash", "after_hash"]
    failure_modes: ["missing_document", "invalid_parent", "unsupported_layer_kind", "crdt_conflict"]
    verification: ["unit_layer_insert", "undo_redo_roundtrip", "event_replay_hash"]
  - command_id: "studio.layer_graph.reorder_layer.v0"
    command_name: "Reorder Layer"
    studio_primitive: "StudioLayerGraph"
    engine_module: "studio_layer_graph"
    vendor_source_refs: ["photoshop.layer.stack", "affinity_photo.saved_layer_states", "indesign.layers"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Move one or more layer nodes within a parent or into another valid parent."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "layer_ids", "target_parent_layer_id"]
    typed_parameters: { insert_index: "integer", preserve_relative_order: "bool" }
    output_refs: ["layer_tree_snapshot_ref"]
    state_mutations: ["remove_from_old_parent", "insert_into_target_parent", "renormalize_order_keys"]
    undo_redo_semantics: "Undo restores previous parent/order tuple for every moved layer."
    eventledger_event_family: "studio.layer_graph.layer_reordered"
    crdt_scope: "document.layer_graph"
    diagnostics: ["before_order", "after_order", "cycle_check"]
    model_receipt: ["moved_layer_ids", "from_parent_ids", "target_parent_layer_id", "insert_index"]
    failure_modes: ["cycle_parenting", "locked_layer", "missing_layer", "invalid_insert_index"]
    verification: ["multi_layer_move", "locked_layer_denial", "crdt_order_merge"]
  - command_id: "studio.layer_graph.group_layers.v0"
    command_name: "Group Layers"
    studio_primitive: "StudioLayerGraph"
    engine_module: "studio_layer_graph"
    vendor_source_refs: ["photoshop.layer.stack", "affinity_designer.pixel_persona_raster_tools", "indesign.layers"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Create a group node and move selected layers into it while preserving visual stacking."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "layer_ids"]
    typed_parameters: { group_name: "string", collapse_in_ui: "bool" }
    output_refs: ["group_layer_id", "layer_tree_snapshot_ref"]
    state_mutations: ["create_group_node", "move_children", "preserve_effective_order"]
    undo_redo_semantics: "Undo ungroups and removes group node; redo recreates group with stable id."
    eventledger_event_family: "studio.layer_graph.layers_grouped"
    crdt_scope: "document.layer_graph"
    diagnostics: ["visual_order_proof", "group_bounds"]
    model_receipt: ["group_layer_id", "child_layer_ids", "before_hash", "after_hash"]
    failure_modes: ["empty_selection", "mixed_locked_layers", "cycle_parenting"]
    verification: ["visual_hash_unchanged_after_group", "undo_redo_roundtrip"]
  - command_id: "studio.mask.attach_layer_mask.v0"
    command_name: "Attach Layer Mask"
    studio_primitive: "StudioMaskGraph"
    engine_module: "studio_mask"
    vendor_source_refs: ["photoshop.mask.layer_mask", "affinity_photo.compound_masks"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Attach raster/vector/selection-derived mask node to a target layer without destructively changing source pixels."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "target_layer_id", "selection_or_mask_source_ref"]
    typed_parameters: { mask_kind: "raster|vector|compound", initial_mode: "reveal|hide|selection", invert: "bool" }
    output_refs: ["mask_node_id", "mask_preview_ref"]
    state_mutations: ["create_mask_node", "attach_to_layer", "update_layer_composite"]
    undo_redo_semantics: "Undo detaches and removes mask node; redo restores same node and attachment."
    eventledger_event_family: "studio.mask.attached"
    crdt_scope: "document.mask_graph"
    diagnostics: ["coverage_stats", "mask_bounds", "preview_hash"]
    model_receipt: ["target_layer_id", "mask_node_id", "coverage_percent"]
    failure_modes: ["missing_layer", "unsupported_mask_source", "locked_layer"]
    verification: ["mask_coverage_test", "composite_render_delta", "undo_redo_roundtrip"]
  - command_id: "studio.selection.create_subject_selection.v0"
    command_name: "Create Subject Selection"
    studio_primitive: "StudioSelectionSet"
    engine_module: "studio_selection"
    vendor_source_refs: ["photoshop.selection.object_subject_people", "affinity_photo.select_subject_ml"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Create an editable selection set around likely subject regions."
    provider_posture: "provider_adapter"
    input_refs: ["document_id", "source_layer_id"]
    typed_parameters: { model_posture: "local|provider", target: "subject|object|person|hair", confidence_threshold: "0..1" }
    output_refs: ["selection_id", "confidence_map_ref"]
    state_mutations: ["create_ephemeral_selection", "optional_persist_selection_artifact"]
    undo_redo_semantics: "Selection creation is undoable when persisted; ephemeral preview can be discarded without EventLedger mutation."
    eventledger_event_family: "studio.selection.created"
    crdt_scope: "document.selection_overlay"
    diagnostics: ["bounds", "confidence_map", "model_receipt"]
    model_receipt: ["selection_id", "model_id", "threshold", "coverage_percent"]
    failure_modes: ["no_subject_found", "provider_unavailable", "layer_not_rasterizable"]
    verification: ["fixture_subject_mask_iou", "provider_denial_fallback", "persisted_selection_replay"]
  - command_id: "studio.raster.apply_live_filter.v0"
    command_name: "Apply Live Raster Filter"
    studio_primitive: "StudioRasterPipeline"
    engine_module: "studio_raster"
    vendor_source_refs: ["photoshop.filter.smart_filters", "affinity_photo.live_filters_adjustments"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Attach a non-destructive raster operation node to a layer, mask, or smart/placed object."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "target_layer_id"]
    typed_parameters: { filter_kind: "enum", parameters: "object", mask_ref: "optional_ref" }
    output_refs: ["operation_node_id", "preview_ref"]
    state_mutations: ["append_operation_node", "invalidate_render_cache"]
    undo_redo_semantics: "Undo removes operation node; redo restores parameters and render-cache invalidation state."
    eventledger_event_family: "studio.raster.live_filter_applied"
    crdt_scope: "document.operation_stack"
    diagnostics: ["parameter_schema", "preview_hash", "render_time_ms"]
    model_receipt: ["operation_node_id", "filter_kind", "target_layer_id"]
    failure_modes: ["unsupported_filter", "invalid_parameter", "render_failure"]
    verification: ["golden_preview", "parameter_roundtrip", "undo_redo_roundtrip"]
  - command_id: "studio.vector.boolean_path.v0"
    command_name: "Boolean Path Operation"
    studio_primitive: "StudioVectorPathGraph"
    engine_module: "studio_vector"
    vendor_source_refs: ["affinity_designer.shape_builder", "photoshop.vector.pen_paths"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Union, subtract, intersect, divide, or XOR vector path regions while preserving source lineage when requested."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "path_node_ids"]
    typed_parameters: { operation: "union|subtract|intersect|xor|divide", retain_sources: "bool" }
    output_refs: ["path_node_id", "topology_report_ref"]
    state_mutations: ["create_or_replace_path_node", "record_source_lineage"]
    undo_redo_semantics: "Undo restores prior path nodes and removes generated node unless retain_sources already preserved them."
    eventledger_event_family: "studio.vector.boolean_applied"
    crdt_scope: "document.vector_graph"
    diagnostics: ["path_topology", "self_intersection_report", "source_lineage"]
    model_receipt: ["operation", "input_path_node_ids", "output_path_node_id"]
    failure_modes: ["invalid_path", "boolean_kernel_failure", "empty_result"]
    verification: ["topology_golden", "svg_export_check", "undo_redo_roundtrip"]
  - command_id: "studio.typography.edit_story.v0"
    command_name: "Edit Story Text"
    studio_primitive: "StudioTextRunAndStory"
    engine_module: "studio_typography"
    vendor_source_refs: ["photoshop.typography.text_layers", "indesign.threaded_text", "affinity_publisher.footnotes_endnotes_sidenotes"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Edit text in a text layer, frame story, or threaded story with style preservation."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "story_id"]
    typed_parameters: { edit_ops: "array", preserve_styles: "bool", language: "optional_bcp47" }
    output_refs: ["story_snapshot_ref", "layout_reflow_ref"]
    state_mutations: ["apply_text_ops", "update_style_ranges", "trigger_layout_reflow"]
    undo_redo_semantics: "Undo applies inverse text ops and restores style range boundaries."
    eventledger_event_family: "studio.typography.story_edited"
    crdt_scope: "document.story_graph"
    diagnostics: ["overset_text", "font_resolution", "shaping_trace"]
    model_receipt: ["story_id", "edit_op_count", "overset_state"]
    failure_modes: ["invalid_text_range", "missing_font", "locked_story"]
    verification: ["text_op_roundtrip", "font_fallback_trace", "overset_detector"]
  - command_id: "studio.layout.place_linked_asset.v0"
    command_name: "Place Linked Asset"
    studio_primitive: "StudioPageSpread"
    engine_module: "studio_layout"
    vendor_source_refs: ["indesign.import_place_assets", "affinity_publisher.linked_file_layer_visibility_override", "photoshop.layer.smart_objects"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "preserve_existing_linked_asset_formats_and_dependency_metadata"
    parity_scope: "Place an external asset into a frame/layer as linked content with dependency tracking."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "asset_ref", "target_frame_or_layer_id"]
    typed_parameters: { placement_mode: "fit|fill|actual_size|crop", link_mode: "linked|embedded", transform: "matrix" }
    output_refs: ["placed_asset_node_id", "link_record_id"]
    state_mutations: ["create_placed_asset_node", "register_link_dependency", "update_render_cache"]
    undo_redo_semantics: "Undo removes placement and dependency; redo restores link record and transform."
    eventledger_event_family: "studio.layout.asset_placed"
    crdt_scope: "document.asset_graph"
    diagnostics: ["missing_link_status", "asset_metadata", "placement_bounds"]
    model_receipt: ["placed_asset_node_id", "asset_ref", "link_mode"]
    failure_modes: ["missing_asset", "unsupported_format", "broken_link"]
    verification: ["link_manifest_test", "missing_link_preflight", "export_includes_dependency"]
  - command_id: "studio.prepress.run_preflight.v0"
    command_name: "Run Preflight"
    studio_primitive: "StudioPreflightProfile"
    engine_module: "studio_prepress"
    vendor_source_refs: ["indesign.preflight", "affinity_publisher.picture_frames_tables_data_merge"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "validate_existing_print_pdf_package_and_linked_asset_targets"
    parity_scope: "Run deterministic document checks for missing fonts, links, overset text, color, accessibility, and export readiness."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "preflight_profile_id"]
    typed_parameters: { page_range: "optional_range", severity_threshold: "info|warning|error" }
    output_refs: ["preflight_report_ref"]
    state_mutations: ["record_preflight_report"]
    undo_redo_semantics: "No document content mutation; report can be discarded or superseded."
    eventledger_event_family: "studio.prepress.preflight_ran"
    crdt_scope: "document.validation_reports"
    diagnostics: ["missing_fonts", "missing_links", "overset_text", "ink_report", "accessibility_report"]
    model_receipt: ["report_ref", "error_count", "warning_count", "blocking_items"]
    failure_modes: ["missing_profile", "unsupported_check", "stale_document_state"]
    verification: ["fixture_missing_font", "fixture_overset_text", "package_export_gate"]
  - command_id: "studio.export.render_recipe.v0"
    command_name: "Render Export Recipe"
    studio_primitive: "StudioExportRecipe"
    engine_module: "studio_export"
    vendor_source_refs: ["photoshop.export.export_as_quick_export", "indesign.print_pdf_export", "affinity_designer.export_persona_slices"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Render document/layers/artboards/pages/slices to an output artifact with a reproducible manifest."
    provider_posture: "local_primitive"
    input_refs: ["document_id", "export_recipe_id"]
    typed_parameters: { target_format: "enum", range: "optional", color_profile: "optional", include_metadata: "bool" }
    output_refs: ["artifact_ref", "export_manifest_ref"]
    state_mutations: ["record_export_receipt", "write_artifact_manifest"]
    undo_redo_semantics: "Export does not mutate document content; receipt can be superseded."
    eventledger_event_family: "studio.export.rendered"
    crdt_scope: "document.export_receipts"
    diagnostics: ["output_hash", "format_options", "dependency_manifest", "warnings"]
    model_receipt: ["artifact_ref", "format", "page_or_layer_range", "output_hash"]
    failure_modes: ["unsupported_format", "preflight_blocker", "write_failure"]
    verification: ["golden_export_hash", "metadata_policy_test", "preflight_blocking_test"]
  - command_id: "studio.ai.generate_edit.v0"
    command_name: "Generate Image Edit"
    studio_primitive: "StudioModelToolContract"
    engine_module: "studio_model_tools"
    vendor_source_refs: ["photoshop.ai.generative_fill", "indesign.ai_text_to_image"]
    naming_posture: "handshake_native_name_with_vendor_refs_for_provenance_only"
    file_format_compatibility: "not_applicable_runtime_state_command"
    parity_scope: "Generate, fill, expand, harmonize, or otherwise synthesize visual content from prompt/context inputs."
    provider_posture: "provider_adapter"
    input_refs: ["document_id", "target_layer_or_frame_id", "selection_or_mask_ref", "prompt_ref"]
    typed_parameters: { provider_id: "string", model_id: "string", generation_mode: "fill|expand|image|harmonize", seed: "optional_integer", variations: "integer" }
    output_refs: ["generated_asset_refs", "model_receipt_ref"]
    state_mutations: ["create_generated_layer_or_asset", "record_prompt_and_provider_receipt"]
    undo_redo_semantics: "Undo removes generated insertion while preserving provider receipt for audit unless policy purges it."
    eventledger_event_family: "studio.ai.generated_edit"
    crdt_scope: "document.generated_assets"
    diagnostics: ["provider_status", "model_id", "prompt_hash", "variation_refs", "content_credentials"]
    model_receipt: ["provider_id", "model_id", "prompt_hash", "output_asset_refs", "policy_flags"]
    failure_modes: ["provider_unavailable", "quota_denied", "policy_denied", "generation_failed"]
    verification: ["provider_mock_success", "provider_mock_denial", "undo_removes_generated_asset", "receipt_redaction_policy"]
```

### [SFR-COMMAND-CONTRACTS.roi] High-ROI Additions

```yaml
high_roi_additions:
  - item: "Make dry_run mandatory where a command mutates document state."
    why_high_roi: "Lets models preview destructive-looking edits without committing state."
    gap_closed: "Reduces data-loss risk and improves parallel-agent workflow."
    reuse: "EventLedger before/after hashes and diagnostic snapshots."
    validation: "Every mutating command has dry_run output or an explicit no-dry-run exception."
  - item: "Require model_receipt on every command."
    why_high_roi: "Makes model actions observable, attributable, and replayable."
    gap_closed: "Prevents hidden UI-only edits and brittle screen-reading workflows."
    reuse: "Existing EventLedger and Flight Recorder concepts."
    validation: "Receipt schema lint plus replay smoke test."
  - item: "Use provider_posture as an implementation gate."
    why_high_roi: "Prevents cloud and AI features from accidentally becoming assumed local primitives."
    gap_closed: "Closes AI/cloud/collaboration ambiguity."
    reuse: "Provider posture map in SFR-PROVIDER-POSTURE."
    validation: "No ai/collaboration/cloud command ships without posture."
```

### [SFR-COMMAND-CONTRACTS.sources] Sources

```yaml
sources:
  - { id: CC-S01, path: "05-studio-primitive-map.md", note: "Primitive names, engine modules, state authority, diagnostics, and build order." }
  - { id: CC-S02, path: "01-photoshop-feature-map.md", note: "Photoshop category feature references." }
  - { id: CC-S03, path: "02-affinity-suite-feature-map.md", note: "Affinity category feature references." }
  - { id: CC-S04, path: "03-indesign-feature-map.md", note: "InDesign category feature references." }
  - { id: CC-S05, path: "06-photoshop-leaf-index.md", note: "Photoshop official help leaves." }
  - { id: CC-S06, path: "07-indesign-leaf-index.md", note: "InDesign official help leaves." }
```

