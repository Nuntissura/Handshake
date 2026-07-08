---
file_id: studio-app-feature-research-layer-graph-vertical-slice
topic_id: SFR-LAYER-GRAPH-SLICE
title: "Non-Destructive Layer Graph Vertical Slice"
status: draft
target_parity_id: "parity.layer_graph.core"
summary: "First implementation-ready vertical slice contract for shared StudioLayerGraph behavior across Photoshop, Affinity, and InDesign-style workflows."
sources: 6
updated_at: "2026-07-05"
---

## [SFR-LAYER-GRAPH-SLICE] Non-Destructive Layer Graph Vertical Slice

### [SFR-LAYER-GRAPH-SLICE.scope] Scope

```yaml
vertical_slice:
  slice_id: "studio.layer_graph.slice.v0"
  parity_id: "parity.layer_graph.core"
  objective: "Prove a shared non-destructive layer graph can represent core Photoshop, Affinity, and InDesign layer behaviors before broader Studio implementation."
  base_scope:
    - "Create layer nodes for raster, vector, text, adjustment, fill, group, placed asset, and generated asset kinds."
    - "Reorder, group, ungroup, show/hide, lock/unlock, rename, and delete layer nodes."
    - "Attach masks and live filter/adjustment operation nodes without mutating source pixels."
    - "Represent linked/embedded placed assets with dependency records."
    - "Export a visible composite or selected layer/artboard range with manifest."
  non_goals:
    - "Full UI implementation."
    - "Full filter catalog."
    - "Full InDesign page layout engine."
    - "Provider-backed generation beyond mocked receipts."
```

### [SFR-LAYER-GRAPH-SLICE.commands] Command Set

```yaml
commands:
  - command_id: "studio.layer_graph.create_layer.v0"
    required_for: ["photoshop.layer.stack", "affinity_photo.live_filters_adjustments", "indesign.layers"]
    acceptance: "Creates stable layer_id, records EventLedger event, updates layer_tree_snapshot, supports undo/redo."
  - command_id: "studio.layer_graph.reorder_layer.v0"
    required_for: ["photoshop.layer.stack", "indesign.layers"]
    acceptance: "Moves layer without visual hash drift except where stacking intentionally changes composite."
  - command_id: "studio.layer_graph.group_layers.v0"
    required_for: ["photoshop.layer.stack", "affinity_designer.pixel_persona_raster_tools"]
    acceptance: "Grouping preserves visual order and emits reversible grouping receipt."
  - command_id: "studio.layer_graph.ungroup_layers.v0"
    required_for: ["photoshop.layer.stack", "indesign.layers"]
    acceptance: "Ungroup restores children to parent order and removes group node with undo support."
  - command_id: "studio.layer_graph.set_layer_attributes.v0"
    required_for: ["photoshop.layer.stack", "affinity_photo.saved_layer_states", "indesign.layers"]
    acceptance: "Applies name, visibility, opacity, lock, blend mode, and tag changes with attribute diff receipt."
  - command_id: "studio.mask.attach_layer_mask.v0"
    required_for: ["photoshop.mask.layer_mask", "affinity_photo.compound_masks"]
    acceptance: "Attaches mask graph node and updates composite without destructive pixel mutation."
  - command_id: "studio.raster.apply_live_filter.v0"
    required_for: ["photoshop.filter.smart_filters", "affinity_photo.live_filters_adjustments"]
    acceptance: "Adds operation node, invalidates render cache, and preserves editable parameters."
  - command_id: "studio.layout.place_linked_asset.v0"
    required_for: ["photoshop.layer.smart_objects", "affinity_publisher.linked_file_layer_visibility_override", "indesign.links_panel"]
    acceptance: "Creates placed asset node, dependency link, missing-link diagnostic, and export manifest entry."
  - command_id: "studio.export.render_recipe.v0"
    required_for: ["photoshop.export.layers_artboards", "indesign.print_pdf_export", "affinity_designer.export_persona_slices"]
    acceptance: "Renders selected visible graph to artifact_ref with output hash and dependency manifest."
```

### [SFR-LAYER-GRAPH-SLICE.state-model] State Model

```yaml
state_model:
  document:
    fields: ["document_id", "color_profile_ref", "layer_graph_root_id", "asset_graph_ref", "style_registry_ref", "operation_stack_ref"]
  layer_node:
    required_fields: ["layer_id", "kind", "parent_id", "order_key", "name", "visible", "locked", "opacity", "blend_mode", "source_ref", "mask_refs", "operation_refs", "style_refs"]
    layer_kinds: ["raster", "vector", "text", "adjustment", "fill", "group", "placed_asset", "generated_asset", "layout_frame"]
  mask_node:
    required_fields: ["mask_id", "mask_kind", "source_ref", "combine_mode", "invert", "coverage_stats"]
  operation_node:
    required_fields: ["operation_id", "operation_kind", "parameter_schema_id", "parameters", "target_ref", "enabled", "mask_ref"]
  dependency_record:
    required_fields: ["dependency_id", "asset_ref", "link_mode", "status", "last_known_hash", "missing_link_policy"]
  eventledger:
    event_families:
      - "studio.layer_graph.layer_created"
      - "studio.layer_graph.layer_reordered"
      - "studio.layer_graph.layers_grouped"
      - "studio.layer_graph.layer_attributes_changed"
      - "studio.mask.attached"
      - "studio.raster.live_filter_applied"
      - "studio.layout.asset_placed"
      - "studio.export.rendered"
```

### [SFR-LAYER-GRAPH-SLICE.fixtures] Verification Fixtures

```yaml
fixtures:
  - fixture_id: "lg-fixture-001-basic-stack"
    purpose: "Create raster, text, vector, adjustment, and group layers; verify stable ordering and undo/redo."
    source_apps_covered: ["Photoshop", "Affinity Photo", "InDesign"]
    expected_checks: ["layer_count", "order_keys", "event_replay_hash", "undo_redo_roundtrip"]
  - fixture_id: "lg-fixture-002-mask-filter"
    purpose: "Attach a layer mask and live filter to a raster layer, then render composite preview."
    source_apps_covered: ["Photoshop", "Affinity Photo"]
    expected_checks: ["mask_coverage_stats", "operation_parameter_roundtrip", "visual_hash"]
  - fixture_id: "lg-fixture-003-placed-link"
    purpose: "Place linked asset, break link, run diagnostics, restore link, export manifest."
    source_apps_covered: ["Photoshop Smart Objects", "Affinity linked file layers", "InDesign Links panel"]
    expected_checks: ["missing_link_detected", "link_restored", "export_manifest_dependency"]
  - fixture_id: "lg-fixture-004-locked-denial"
    purpose: "Attempt mutation on locked layer and verify no state mutation occurs."
    source_apps_covered: ["Photoshop", "Affinity", "InDesign"]
    expected_checks: ["capability_denial", "before_after_hash_equal", "error_receipt"]
  - fixture_id: "lg-fixture-005-parallel-reorder"
    purpose: "Apply concurrent reorder operations and verify deterministic CRDT merge/order."
    source_apps_covered: ["Handshake model workflow"]
    expected_checks: ["merge_determinism", "conflict_trace", "attributed_receipts"]
```

### [SFR-LAYER-GRAPH-SLICE.failures] Risks, Failure Scenarios, And Hardening

```yaml
risks:
  - risk_id: "lg-risk-001-destructive-edit-leak"
    scenario: "A live filter or mask accidentally writes into source raster pixels."
    remediation: "Separate immutable source_ref from operation_node outputs; enforce before/after source hash check."
    verification: "Source asset hash unchanged after mask/filter commands."
  - risk_id: "lg-risk-002-visual-order-drift"
    scenario: "Grouping or reordering changes composite unexpectedly."
    remediation: "Record visual-order proof and run preview hash checks for no-op grouping."
    verification: "Visual hash unchanged after group/ungroup when stacking semantics are equivalent."
  - risk_id: "lg-risk-003-crdt-order-conflict"
    scenario: "Parallel agents reorder the same layer set and produce unstable order."
    remediation: "Use deterministic order keys, actor attribution, and conflict trace receipts."
    verification: "Parallel reorder fixture converges across replay orders."
  - risk_id: "lg-risk-004-linked-asset-data-loss"
    scenario: "Missing or moved linked asset silently exports stale content."
    remediation: "Dependency records carry last_known_hash and export gate emits missing-link blocker unless policy permits fallback."
    verification: "Broken-link fixture blocks export or emits explicit degraded export receipt."
  - risk_id: "lg-risk-005-provider-generated-layer-ambiguity"
    scenario: "Generated layer lacks prompt/provider provenance."
    remediation: "Generated asset layer kind requires model receipt or mock receipt."
    verification: "Provider mock fixture fails if generated layer has no receipt."
```

### [SFR-LAYER-GRAPH-SLICE.acceptance] Acceptance Gates

```yaml
acceptance_gates:
  - gate_id: "lg-gate-001-contract-complete"
    requirement: "All commands in this slice have command contracts with provider posture, inputs, outputs, state mutations, undo semantics, diagnostics, model receipt, and verification."
  - gate_id: "lg-gate-002-state-replay"
    requirement: "EventLedger replay reconstructs the same layer graph hash after every fixture."
  - gate_id: "lg-gate-003-undo-redo"
    requirement: "Every mutating command in the slice passes undo/redo roundtrip."
  - gate_id: "lg-gate-004-visual-proof"
    requirement: "Render previews prove expected visual changes and no unintended changes."
  - gate_id: "lg-gate-005-model-usability"
    requirement: "Each command emits a model-readable receipt and diagnostic snapshot sufficient for a no-context model to continue safely."
```

### [SFR-LAYER-GRAPH-SLICE.roi] High-ROI Additions

```yaml
high_roi_additions:
  - item: "Add layer_tree_snapshot diagnostics from day one."
    why_high_roi: "Models can inspect state without screen-reading the GUI."
    gap_closed: "Improves parallel-agent workflow and reduces UI confusion."
    reuse: "StudioLayerGraph and EventLedger receipt model."
    validation: "Snapshot emitted after every mutating layer command."
  - item: "Make every fixture replayable from receipts."
    why_high_roi: "Turns debugging into deterministic replay instead of manual reproduction."
    gap_closed: "Reduces future rework and data-loss risk."
    reuse: "EventLedger and Flight Recorder concepts."
    validation: "Replay hash equals live hash."
  - item: "Include linked asset diagnostics in the first slice."
    why_high_roi: "Smart Objects, Affinity linked files, and InDesign links are all high-value parity points."
    gap_closed: "Prevents export/package failures later."
    reuse: "ArtifactStore dependency manifests."
    validation: "Missing-link fixture blocks or records degraded export."
```

### [SFR-LAYER-GRAPH-SLICE.sources] Sources

```yaml
sources:
  - { id: LG-S01, path: "05-studio-primitive-map.md", note: "StudioLayerGraph primitive and build-order basis." }
  - { id: LG-S02, path: "10-studio-command-contracts.md", note: "Command schema and seed commands used by this slice." }
  - { id: LG-S03, path: "12-cross-app-parity-matrix.md", note: "parity.layer_graph.core mapping." }
  - { id: LG-S04, path: "01-photoshop-feature-map.md", note: "Photoshop layer, mask, smart object, and export references." }
  - { id: LG-S05, path: "02-affinity-suite-feature-map.md", note: "Affinity layer, linked file, and live filter references." }
  - { id: LG-S06, path: "03-indesign-feature-map.md", note: "InDesign layer, links, object style, and export references." }
```
