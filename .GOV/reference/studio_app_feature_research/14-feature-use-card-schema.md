---
file_id: studio-app-feature-research-feature-use-card-schema
topic_id: SFR-FEATURE-USE-CARDS
title: "Feature Use Card Schema"
status: draft
summary: "Planning bridge for preserving feature purpose, user intent, workflows, options, mistakes, edge cases, and future internal UserManual handoff requirements."
sources: 9
updated_at: "2026-07-05"
---

## [SFR-FEATURE-USE-CARDS] Feature Use Card Schema

### [SFR-FEATURE-USE-CARDS.policy] Policy

```yaml
policy:
  purpose: "Preserve what each feature is meant to do and how operators/models should use it before the feature becomes a native Studio tool."
  authority_status: "Reference/planning bridge only. The in-product internal UserManual remains the durable product-facing/manual surface once implementation starts."
  handoff_rule: "When a feature use card becomes product code, the implementation work packet must create or update the matching Studio UserManual topic in the same change."
  naming_rule: "Feature Use Cards may cite Photoshop, Affinity, InDesign, Illustrator, and Figma-family products as source applications, but shipped Studio tools, commands, panels, and manual topics must use Handshake-native names."
  file_format_rule: "File compatibility is a rebuild requirement. Import/export work should target existing creative formats and compatibility fixtures instead of inventing a replacement interchange format."
  not_a_replacement_for:
    - "Handshake internal UserManual"
    - "work packet acceptance criteria"
    - "product spec authority"
    - "runtime proof"
  closes_gap: "Prevents building primitives while losing feature intent, usage path, expected outcome, and recovery guidance."
```

### [SFR-FEATURE-USE-CARDS.schema] Schema

```yaml
feature_use_card_schema:
  schema_id: "studio.feature_use_card.v0"
  required_fields:
    - feature_use_card_id
    - feature_name
    - source_apps
    - studio_surface
    - naming_posture
    - file_format_compatibility
    - purpose
    - user_goal
    - when_to_use
    - typical_workflow
    - key_options
    - expected_result
    - common_mistakes
    - edge_cases
    - recovery_steps
    - handshake_tool_design_notes
    - equivalent_features
    - command_contract_refs
    - verification_refs
    - user_manual_handoff
    - source_refs
  user_manual_handoff:
    required_fields:
      - manual_topic_candidate
      - manual_entry_status
      - no_context_model_usage_notes
      - expected_inputs_outputs
      - failure_recovery_notes
      - diagnostic_surfaces
      - proof_required_before_manual_publish
    manual_entry_status_enum:
      - "planning_only"
      - "ready_for_manual_draft"
      - "manual_draft_required_in_work_packet"
      - "manual_updated_in_product"
  naming_posture_enum:
    - "handshake_native_name_with_vendor_source_refs"
    - "compatibility_note_only"
    - "migration_alias_only"
  file_format_compatibility_enum:
    - "not_applicable_runtime_state_command"
    - "must_preserve_existing_format_compatibility"
    - "import_only_compatibility"
    - "export_only_compatibility"
    - "round_trip_compatibility_target"
```

### [SFR-FEATURE-USE-CARDS.seed] Seed Use Cards

```yaml
feature_use_cards:
  - feature_use_card_id: "fuc.layer_graph.create_layer.v0"
    feature_name: "Create Layer"
    source_apps: ["Photoshop", "Affinity Photo/Designer/Publisher", "InDesign"]
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    studio_surface: "StudioLayerGraph"
    purpose: "Add a new editable content or organizational node to a Studio document without flattening or mutating existing content."
    user_goal: "Separate content, effects, text, placed files, or layout elements so they can be edited, reordered, hidden, masked, exported, or automated independently."
    when_to_use:
      - "Starting a new visual element."
      - "Keeping edits non-destructive."
      - "Separating text, raster, vector, adjustment, fill, group, placed asset, or generated content."
      - "Preparing content for selective export or automation."
    typical_workflow:
      - "Open or create a Studio document."
      - "Choose the layer kind."
      - "Set name, parent/group, insert position, visibility, opacity, and blend mode."
      - "Create the layer."
      - "Inspect the layer tree snapshot and receipt."
      - "Add content, mask, operations, or export rules as needed."
    key_options:
      - { name: "layer_kind", meaning: "Determines behavior and allowed child/content/operation types." }
      - { name: "parent_layer_id", meaning: "Places the layer under the document root or inside a group." }
      - { name: "insert_index", meaning: "Controls visual stacking order." }
      - { name: "blend_mode", meaning: "Controls how the layer composites with layers beneath it." }
      - { name: "opacity", meaning: "Controls global layer transparency." }
    expected_result: "A stable layer_id exists in the document layer graph, EventLedger records creation, undo can remove it, redo can restore it, and the layer tree snapshot exposes the new node."
    common_mistakes:
      - "Creating destructive raster edits directly on a source asset instead of adding an editable layer or operation node."
      - "Using a group layer when a mask or clipping relationship is intended."
      - "Leaving generated or placed content without provenance/dependency records."
    edge_cases:
      - "Layer insertion into a locked parent."
      - "Concurrent agents creating layers at the same insert position."
      - "Generated layers require provider/model receipt."
      - "Placed asset layers require dependency records."
    recovery_steps:
      - "Use undo to remove an accidental layer."
      - "Use reorder to fix stacking mistakes."
      - "Use diagnostics to inspect parent, order_key, visibility, lock state, and source_ref."
    handshake_tool_design_notes: "Expose layer creation as a typed command and a UI action. Always show layer kind, name, parent, order, visibility, lock state, source/provenance, and receipt."
    equivalent_features:
      photoshop: ["Layer stack", "Adjustment layers", "Fill layers", "Smart Objects"]
      affinity: ["Layer stack", "Live filters", "Saved Layer States", "Linked file layer visibility"]
      indesign: ["Layers", "Links panel", "Object styles"]
    command_contract_refs: ["studio.layer_graph.create_layer.v0"]
    verification_refs: ["lg-fixture-001-basic-stack", "lg-gate-002-state-replay", "lg-gate-003-undo-redo"]
    user_manual_handoff:
      manual_topic_candidate: "Studio / Layers / Create Layer"
      manual_entry_status: "planning_only"
      no_context_model_usage_notes: "A model should call create_layer only after identifying document_id, layer_kind, intended parent, stacking order, and whether source/provenance records are required."
      expected_inputs_outputs: "Inputs: document_id, optional parent, optional insert index, layer kind, name, visibility, opacity, blend mode. Outputs: layer_id and layer_tree_snapshot_ref."
      failure_recovery_notes: "If parent is missing/locked or layer kind is unsupported, make no state mutation and emit a denial receipt. Operator/model can choose another parent or layer kind."
      diagnostic_surfaces: ["layer_tree_snapshot", "EventLedger receipt", "undo_redo_receipt"]
      proof_required_before_manual_publish: ["unit_layer_insert", "undo_redo_roundtrip", "event_replay_hash"]
    source_refs: ["SFR-COMMAND-CONTRACTS", "SFR-LAYER-GRAPH-SLICE", "SFR-PARITY-MATRIX"]
  - feature_use_card_id: "fuc.layer_graph.attach_mask.v0"
    feature_name: "Attach Layer Mask"
    source_apps: ["Photoshop", "Affinity Photo"]
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    studio_surface: "StudioMaskGraph"
    purpose: "Hide or reveal parts of a layer through an editable mask while preserving the original layer content."
    user_goal: "Composite, isolate, blend, or constrain edits without deleting source pixels."
    when_to_use:
      - "Removing or hiding a background non-destructively."
      - "Constraining an adjustment/filter to part of a layer."
      - "Turning a selection into a reusable editable mask."
      - "Combining masks for complex visibility control."
    typical_workflow:
      - "Create or select target layer."
      - "Create or choose a selection, vector path, luminosity/color range, or existing mask source."
      - "Attach mask in reveal/hide/selection mode."
      - "Inspect mask preview and coverage stats."
      - "Refine mask or combine it with other mask nodes."
    key_options:
      - { name: "mask_kind", meaning: "Raster, vector, or compound mask behavior." }
      - { name: "initial_mode", meaning: "Whether the mask starts as reveal, hide, or selection-derived." }
      - { name: "invert", meaning: "Swaps visible and hidden mask regions." }
      - { name: "combine_mode", meaning: "Add/intersect/subtract/XOR behavior for compound masks." }
    expected_result: "A mask node is attached to the target layer, source pixels remain unchanged, composite preview updates, and coverage diagnostics are available."
    common_mistakes:
      - "Applying/deleting pixels instead of masking."
      - "Inverting the mask unintentionally."
      - "Attaching the mask to the wrong layer or group."
      - "Forgetting that mask visibility affects export output."
    edge_cases:
      - "Mask source resolution differs from target layer."
      - "Vector mask and raster mask combine order changes output."
      - "Locked target layer denies mask attachment."
      - "Nested group masks and child masks interact."
    recovery_steps:
      - "Disable or detach the mask."
      - "Invert mask if visibility is reversed."
      - "Inspect mask coverage stats and target_layer_id."
      - "Undo attachment to restore previous composite."
    handshake_tool_design_notes: "Expose mask attachment as a first-class command with preview, target layer, source selection/path, combine mode, and destructive-apply warnings."
    equivalent_features:
      photoshop: ["Layer masks", "Vector masks", "Object masks"]
      affinity: ["Compound Masks", "Live Masks", "Luminosity masks"]
      indesign: ["Frame clipping and text wrap are adjacent layout behaviors, not direct mask parity."]
    command_contract_refs: ["studio.mask.attach_layer_mask.v0"]
    verification_refs: ["lg-fixture-002-mask-filter", "lg-gate-004-visual-proof"]
    user_manual_handoff:
      manual_topic_candidate: "Studio / Masks / Attach Layer Mask"
      manual_entry_status: "planning_only"
      no_context_model_usage_notes: "A model should confirm target_layer_id and mask source before attachment, then inspect coverage stats and preview hash."
      expected_inputs_outputs: "Inputs: document_id, target_layer_id, selection_or_mask_source_ref, mask kind, initial mode, invert. Outputs: mask_node_id and mask_preview_ref."
      failure_recovery_notes: "On locked/missing layer or unsupported mask source, emit a failure receipt and do not mutate document state."
      diagnostic_surfaces: ["mask_preview", "coverage_stats", "composite_render_delta", "EventLedger receipt"]
      proof_required_before_manual_publish: ["mask_coverage_test", "composite_render_delta", "undo_redo_roundtrip"]
    source_refs: ["SFR-COMMAND-CONTRACTS", "SFR-LAYER-GRAPH-SLICE"]
  - feature_use_card_id: "fuc.layout.place_linked_asset.v0"
    feature_name: "Place Linked Asset"
    source_apps: ["Photoshop", "Affinity Publisher", "InDesign"]
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    studio_surface: "StudioPageSpread / StudioLayerGraph"
    purpose: "Insert external content into a document while retaining a trackable dependency on the source asset."
    user_goal: "Use files such as images, PSD/PDF/DWG/DXF/Affinity documents, or generated assets in a composition without losing link status, update state, or export dependencies."
    when_to_use:
      - "Compositing an external image or design file."
      - "Building a publication with placed graphics."
      - "Keeping source assets updateable."
      - "Preparing package/export workflows that must report missing links."
    typical_workflow:
      - "Choose source asset."
      - "Choose target frame/layer/page."
      - "Set linked or embedded mode."
      - "Set placement fit/fill/crop/actual-size transform."
      - "Create placement."
      - "Inspect dependency record and missing-link diagnostic."
      - "Run export/preflight when needed."
    key_options:
      - { name: "link_mode", meaning: "Linked keeps dependency to external asset; embedded stores content in the document package." }
      - { name: "placement_mode", meaning: "Controls fit/fill/crop/actual-size behavior." }
      - { name: "transform", meaning: "Position, scale, rotation, and crop transform applied to the placed content." }
      - { name: "missing_link_policy", meaning: "Determines whether export blocks or degrades when the source asset is unavailable." }
    expected_result: "A placed asset node and dependency record exist; diagnostics can report link status, metadata, placement bounds, and export manifest inclusion."
    common_mistakes:
      - "Embedding when linked update behavior is required."
      - "Exporting with a missing link without an explicit degraded-export policy."
      - "Moving source files without updating dependency records."
      - "Confusing layer visibility inside a placed file with document-layer visibility."
    edge_cases:
      - "Linked asset hash changed externally."
      - "Source asset has internal layers/pages/artboards."
      - "Asset format is supported for placement but not editable conversion."
      - "Multiple placements share one source asset."
    recovery_steps:
      - "Relink missing asset."
      - "Embed asset if stable external path cannot be guaranteed."
      - "Inspect dependency manifest before export."
      - "Use degraded export only with explicit receipt."
    handshake_tool_design_notes: "Expose placed assets as dependency-aware graph nodes, not opaque pixels. UI and model surfaces must show link status, source hash, placement transform, and export/package impact."
    equivalent_features:
      photoshop: ["Smart Objects", "Linked Smart Objects"]
      affinity: ["Linked file layer visibility override", "DWG/DXF Place"]
      indesign: ["Place/import graphics", "Links panel", "Package for output"]
    command_contract_refs: ["studio.layout.place_linked_asset.v0"]
    verification_refs: ["lg-fixture-003-placed-link", "lg-risk-004-linked-asset-data-loss"]
    user_manual_handoff:
      manual_topic_candidate: "Studio / Assets / Place Linked Asset"
      manual_entry_status: "planning_only"
      no_context_model_usage_notes: "A model should check whether the operator needs linked update behavior or embedded portability before placing the asset."
      expected_inputs_outputs: "Inputs: document_id, asset_ref, target frame/layer, placement mode, link mode, transform. Outputs: placed_asset_node_id and link_record_id."
      failure_recovery_notes: "If source is missing or unsupported, emit no placement mutation; if a linked source later breaks, preflight/export must show the missing dependency."
      diagnostic_surfaces: ["missing_link_status", "asset_metadata", "placement_bounds", "export_manifest"]
      proof_required_before_manual_publish: ["link_manifest_test", "missing_link_preflight", "export_includes_dependency"]
    source_refs: ["SFR-COMMAND-CONTRACTS", "SFR-LAYER-GRAPH-SLICE", "SFR-PARITY-MATRIX"]
```

### [SFR-FEATURE-USE-CARDS.rollout] Rollout

```yaml
rollout:
  immediate_use: "Attach one feature use card to every command contract promoted from the research corpus."
  first_batch: "Use the StudioLayerGraph vertical slice cards before expanding to raster, vector, typography, layout, design systems, interaction/motion, collaboration, export, automation, and AI/provider tools."
  generated_full_coverage_card_count: 2730
  generated_card_files:
    - "15-photoshop-feature-use-cards.md"
    - "16-affinity-feature-use-cards.md"
    - "17-indesign-feature-use-cards.md"
    - "24-illustrator-feature-use-cards.md"
    - "25-figma-feature-use-cards.md"
  later_user_manual_topic: "When the Studio UserManual topic is enforced, convert accepted use cards into product manual entries and mark manual_entry_status accordingly."
  validation:
    - "Each implemented feature has a use card before coding starts."
    - "Each use card has a matching internal UserManual entry before implementation closeout."
    - "The UserManual entry is tested by a no-context/manual operation or inspection path."
```

### [SFR-FEATURE-USE-CARDS.sources] Sources

```yaml
sources:
  - { id: FUC-S01, path: "10-studio-command-contracts.md", note: "Seed command contracts that need use cards before implementation." }
  - { id: FUC-S02, path: "13-layer-graph-vertical-slice.md", note: "Layer graph vertical slice used for seed cards." }
  - { id: FUC-S03, path: "12-cross-app-parity-matrix.md", note: "Cross-app parity lanes and equivalent feature references." }
  - { id: FUC-S04, path: ".GOV/roles/kernel_builder/KERNEL_BUILDER_PROTOCOL.md", note: "Internal UserManual duty for product behavior changes." }
  - { id: FUC-S05, path: ".GOV/roles/wp_validator/WP_VALIDATOR_PROTOCOL.md", note: "Validator UserManual evidence gate." }
  - { id: FUC-S06, path: "19-studio-local-first-rust-posture.md", note: "Local-first Rust-forward Studio posture." }
  - { id: FUC-S07, path: "20-illustrator-feature-map.md", note: "Illustrator source-app family coverage." }
  - { id: FUC-S08, path: "21-figma-feature-map.md", note: "Figma-family source-app coverage." }
  - { id: FUC-S09, path: "18-feature-use-card-manual-handoff-index.md", note: "Generated manual handoff grouping for all app cards." }
```

